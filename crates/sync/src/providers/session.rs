//! A signed-in drive: who is signed in, the access token kept fresh, and
//! requests sent with it.

use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use super::{AppId, Endpoints, Provider, client, dropbox, google, onedrive};
use crate::http::{Body, Http, Reply};
use crate::oauth::{self, Client, Tokens};
use crate::secrets::Secrets;
use crate::{Error, Result};

/// Who is signed in, to show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub name: String,
    pub email: String,
}

/// A signed-in drive: its access token, renewed from the stored refresh
/// token when it runs out.
pub struct Session {
    pub provider: Provider,
    pub ends: Endpoints,
    pub(crate) http: Http,
    client: Client,
    secrets: Arc<dyn Secrets>,
    /// Where the refresh token is kept; none until `keep`.
    secret_key: Mutex<Option<String>>,
    tokens: Mutex<Tokens>,
}

impl Session {
    /// Right after signing in, before anything is kept.
    pub fn new(
        provider: Provider,
        app: &AppId,
        ends: Endpoints,
        tokens: Tokens,
        secrets: Arc<dyn Secrets>,
    ) -> Session {
        Session {
            provider,
            client: client(provider, app, &ends),
            ends,
            http: Http::default(),
            secrets,
            secret_key: Mutex::new(None),
            tokens: Mutex::new(tokens),
        }
    }

    /// Keeps the sign-in in the credential store under `key`.
    pub fn keep(&self, key: &str) -> Result<()> {
        let tokens = self.tokens.lock().unwrap_or_else(|p| p.into_inner());
        let refresh = tokens.refresh.as_ref().ok_or_else(|| {
            Error::Drive("드라이브가 다시 로그인할 수 있는 열쇠를 주지 않음".into())
        })?;
        self.secrets.set(key, refresh)?;
        *self.secret_key.lock().unwrap_or_else(|p| p.into_inner()) = Some(key.to_string());
        Ok(())
    }

    /// From a stored sign-in; `SignedOut` when there is none.
    pub fn resume(
        provider: Provider,
        app: &AppId,
        ends: Endpoints,
        secrets: Arc<dyn Secrets>,
        secret_key: String,
    ) -> Result<Session> {
        let refresh = secrets.get(&secret_key)?.ok_or(Error::SignedOut)?;
        Ok(Session {
            provider,
            client: client(provider, app, &ends),
            ends,
            http: Http::default(),
            secrets,
            secret_key: Mutex::new(Some(secret_key)),
            tokens: Mutex::new(Tokens {
                access: String::new(),
                refresh: Some(refresh),
                expires_at: 0,
            }),
        })
    }

    fn access(&self, renew: bool) -> Result<String> {
        let mut tokens = self.tokens.lock().unwrap_or_else(|p| p.into_inner());
        let expired = tokens.expires_at - 60 <= chrono::Utc::now().timestamp();
        if renew || expired || tokens.access.is_empty() {
            let refresh = tokens.refresh.clone().ok_or(Error::SignedOut)?;
            let fresh = oauth::refresh(&self.http, &self.client, &refresh)?;
            let key = self
                .secret_key
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .clone();
            if fresh.refresh.as_deref() != Some(refresh.as_str())
                && let (Some(new_refresh), Some(key)) = (&fresh.refresh, key)
            {
                self.secrets.set(&key, new_refresh)?;
            }
            *tokens = fresh;
        }
        Ok(tokens.access.clone())
    }

    /// Sends a request with the access token, renewing it once if refused.
    pub(crate) fn call<'b>(
        &self,
        method: &str,
        url: &str,
        headers: &[(&str, String)],
        body: &dyn Fn() -> Body<'b>,
    ) -> Result<Reply> {
        let mut renewed = false;
        loop {
            let token = self.access(renewed)?;
            let mut all: Vec<(&str, String)> = vec![("Authorization", format!("Bearer {token}"))];
            all.extend(headers.iter().cloned());
            let reply = self.http.send(method, url, &all, body())?;
            if reply.status == 401 && !renewed {
                renewed = true;
                continue;
            }
            if reply.status == 401 {
                return Err(Error::SignedOut);
            }
            return Ok(reply);
        }
    }

    /// Forgets the sign-in on this device.
    pub fn forget(&self) -> Result<()> {
        match self
            .secret_key
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .take()
        {
            Some(key) => self.secrets.delete(&key),
            None => Ok(()),
        }
    }

    pub fn account(&self) -> Result<Account> {
        match self.provider {
            Provider::Google => google::account(self),
            Provider::Onedrive => onedrive::account(self),
            Provider::Dropbox => dropbox::account(self),
        }
    }
}

/// What the writer reads when the drive has no room left, the same for every
/// drive.
pub const FULL: &str = "드라이브 공간이 가득 참";

/// Google's reasons for a full drive (`error.errors[].reason`, or
/// `error.details[].reason` in its newer answers), compared without case or
/// underscores.
const GOOGLE_FULL: [&str; 2] = ["storagequotaexceeded", "quotaexceeded"];
/// Google's reasons for "too many requests", which it answers with a 403.
const GOOGLE_BUSY: [&str; 2] = ["ratelimitexceeded", "userratelimitexceeded"];

/// The reasons in a Google error answer, lowercased without underscores.
fn google_reasons(reply: &Reply) -> Vec<String> {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&reply.body) else {
        return Vec::new();
    };
    let error = &value["error"];
    ["errors", "details"]
        .iter()
        .filter_map(|list| error[list].as_array())
        .flatten()
        .filter_map(|e| e["reason"].as_str())
        .map(|r| r.replace('_', "").to_lowercase())
        .collect()
}

/// Dropbox's `error_summary`, e.g. `path/insufficient_space/..`.
fn dropbox_summary(reply: &Reply) -> String {
    serde_json::from_slice::<serde_json::Value>(&reply.body)
        .ok()
        .and_then(|v| v["error_summary"].as_str().map(str::to_string))
        .unwrap_or_default()
}

/// The drive says it has no room left: OneDrive's 507, Google's 403 with a
/// storage quota reason, Dropbox's 409 with `insufficient_space`.
pub(crate) fn space_full(reply: &Reply) -> bool {
    match reply.status {
        507 => true,
        403 => google_reasons(reply)
            .iter()
            .any(|r| GOOGLE_FULL.contains(&r.as_str())),
        409 => dropbox_summary(reply).contains("insufficient_space"),
        _ => false,
    }
}

/// A drive's error answer, for the screen.
pub(crate) fn drive_error(what: &str, reply: &Reply) -> Error {
    if space_full(reply) {
        // The pass stops here; files not yet sent stay on this device and
        // go up in a later pass, once there is room.
        return Error::Drive(format!(
            "{what} · {FULL}. 원고는 이 기기에 그대로 있고, 공간이 생기면 다시 올립니다."
        ));
    }
    match reply.status {
        401 => Error::SignedOut,
        403 if google_reasons(reply)
            .iter()
            .any(|r| GOOGLE_BUSY.contains(&r.as_str())) =>
        {
            Error::Offline(format!("{what} · 드라이브가 잠시 바쁨"))
        }
        403 => Error::Drive(format!(
            "{what} · 드라이브가 허락하지 않음 ({})",
            reply.text()
        )),
        404 => Error::Drive(format!("{what} · 드라이브에 없음")),
        429 | 503 => Error::Offline(format!("{what} · 드라이브가 잠시 바쁨")),
        _ => Error::Drive(format!("{what} · {} {}", reply.status, reply.text())),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn reply(status: u16, body: serde_json::Value) -> Reply {
        Reply {
            status,
            body: serde_json::to_vec(&body).unwrap(),
        }
    }

    #[test]
    fn a_full_drive_reads_the_same_on_every_drive() {
        let answers = [
            reply(
                403,
                json!({ "error": { "code": 403, "errors": [{ "domain": "usageLimits", "reason": "storageQuotaExceeded" }] } }),
            ),
            reply(
                403,
                json!({ "error": { "status": "PERMISSION_DENIED", "details": [{ "reason": "STORAGE_QUOTA_EXCEEDED" }] } }),
            ),
            reply(
                409,
                json!({ "error_summary": "path/insufficient_space/..", "error": { ".tag": "path" } }),
            ),
            reply(507, json!({ "error": { "code": "quotaLimitReached" } })),
        ];
        for r in &answers {
            let message = drive_error("드라이브에 올리지 못함", r).user_message();
            assert!(message.contains(FULL), "{message}");
        }
    }

    #[test]
    fn other_refusals_keep_their_words() {
        let refused = reply(
            403,
            json!({ "error": { "errors": [{ "reason": "insufficientFilePermissions" }] } }),
        );
        assert!(
            drive_error("x", &refused)
                .user_message()
                .contains("허락하지 않음")
        );
        let busy = reply(
            403,
            json!({ "error": { "errors": [{ "reason": "userRateLimitExceeded" }] } }),
        );
        assert!(matches!(drive_error("x", &busy), Error::Offline(_)));
        let conflict = reply(409, json!({ "error_summary": "path/conflict/file/.." }));
        assert!(!space_full(&conflict));
    }
}
