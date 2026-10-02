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

/// A drive's error answer, for the screen.
pub(crate) fn drive_error(what: &str, reply: &Reply) -> Error {
    match reply.status {
        401 => Error::SignedOut,
        403 => Error::Drive(format!(
            "{what} · 드라이브가 허락하지 않음 ({})",
            reply.text()
        )),
        404 => Error::Drive(format!("{what} · 드라이브에 없음")),
        429 | 503 => Error::Offline(format!("{what} · 드라이브가 잠시 바쁨")),
        507 => Error::Drive(format!("{what} · 드라이브 공간이 가득 참")),
        _ => Error::Drive(format!("{what} · {} {}", reply.status, reply.text())),
    }
}
