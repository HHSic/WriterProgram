//! The drives: how to sign in to each, who is signed in, and each one as a
//! [`Remote`](crate::remote::Remote) for one project folder.
//!
//! Every drive keeps the app's files apart from the writer's other files:
//! Google Drive's `drive.file` lets the app see only the files it made (under
//! a "WriterProgram" folder), OneDrive and Dropbox give it a folder of its own
//! (`앱/WriterProgram`). Inside, each project has a folder named after it.
//!
//! The app needs its own registration with each company to sign in (an app
//! id, and for Google also a desktop client secret, which is not secret in an
//! installed app). Those come from [`AppIds`], read from `drive-apps.json` in
//! the app's settings folder, or built in at compile time.

use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::http::{Body, Http, Reply};
use crate::oauth::{self, Client, Tokens};
use crate::secrets::Secrets;
use crate::{Error, Result};

pub mod dropbox;
pub mod google;
pub mod onedrive;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Google,
    Onedrive,
    Dropbox,
}

impl Provider {
    pub const ALL: [Provider; 3] = [Provider::Google, Provider::Onedrive, Provider::Dropbox];

    pub fn key(self) -> &'static str {
        match self {
            Provider::Google => "google",
            Provider::Onedrive => "onedrive",
            Provider::Dropbox => "dropbox",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Provider::Google => "Google Drive",
            Provider::Onedrive => "OneDrive",
            Provider::Dropbox => "Dropbox",
        }
    }
}

/// The app's registration with one company.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppId {
    pub client_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,
    /// Dropbox wants the exact address registered, port included.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub redirect_port: Option<u16>,
}

/// Registrations for all drives (`drive-apps.json`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppIds {
    #[serde(default)]
    pub google: Option<AppId>,
    #[serde(default)]
    pub onedrive: Option<AppId>,
    #[serde(default)]
    pub dropbox: Option<AppId>,
}

impl AppIds {
    /// The file's registrations, then the ones built into this copy of the app.
    pub fn load(file: &std::path::Path) -> AppIds {
        let mut ids: AppIds = writer_core::store::read_text(file)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        let built_in = |id: Option<&str>, secret: Option<&str>| {
            id.filter(|s| !s.is_empty()).map(|id| AppId {
                client_id: id.into(),
                client_secret: secret.filter(|s| !s.is_empty()).map(str::to_string),
                redirect_port: None,
            })
        };
        ids.google = ids.google.take().or_else(|| {
            built_in(
                option_env!("WRITER_GOOGLE_CLIENT_ID"),
                option_env!("WRITER_GOOGLE_CLIENT_SECRET"),
            )
        });
        ids.onedrive = ids
            .onedrive
            .take()
            .or_else(|| built_in(option_env!("WRITER_ONEDRIVE_CLIENT_ID"), None));
        ids.dropbox = ids.dropbox.take().or_else(|| {
            built_in(option_env!("WRITER_DROPBOX_APP_KEY"), None).map(|mut id| {
                id.redirect_port = Some(DROPBOX_PORT);
                id
            })
        });
        ids
    }

    pub fn get(&self, provider: Provider) -> Option<&AppId> {
        match provider {
            Provider::Google => self.google.as_ref(),
            Provider::Onedrive => self.onedrive.as_ref(),
            Provider::Dropbox => self.dropbox.as_ref(),
        }
        .filter(|id| !id.client_id.trim().is_empty())
    }
}

/// Port registered for Dropbox's return address (`http://localhost:53682/`).
pub const DROPBOX_PORT: u16 = 53682;

/// Where each drive's services are; tests point these at a stand-in server.
#[derive(Debug, Clone)]
pub struct Endpoints {
    pub auth: String,
    pub token: String,
    pub api: String,
    /// Uploads and downloads, where the drive keeps them apart.
    pub content: String,
}

impl Endpoints {
    /// The real services, or a stand-in at the address in the environment
    /// variable `WRITER_<DRIVE>_ENDPOINT` (for trying the app without an
    /// account, see the `stand_in` example).
    pub fn of(provider: Provider) -> Endpoints {
        let var = format!("WRITER_{}_ENDPOINT", provider.key().to_uppercase());
        match std::env::var(var) {
            Ok(base) if !base.trim().is_empty() => {
                Endpoints::at(provider, base.trim_end_matches('/'))
            }
            _ => Endpoints::real(provider),
        }
    }

    /// A stand-in server at `base`, laid out like the real one.
    pub fn at(provider: Provider, base: &str) -> Endpoints {
        let (api, content) = match provider {
            Provider::Google => ("/drive/v3", "/upload/drive/v3"),
            Provider::Onedrive => ("/v1.0", "/v1.0"),
            Provider::Dropbox => ("/2", "/2"),
        };
        Endpoints {
            auth: format!("{base}/auth"),
            token: format!("{base}/token"),
            api: format!("{base}{api}"),
            content: format!("{base}{content}"),
        }
    }

    pub fn real(provider: Provider) -> Endpoints {
        match provider {
            Provider::Google => Endpoints {
                auth: "https://accounts.google.com/o/oauth2/v2/auth".into(),
                token: "https://oauth2.googleapis.com/token".into(),
                api: "https://www.googleapis.com/drive/v3".into(),
                content: "https://www.googleapis.com/upload/drive/v3".into(),
            },
            Provider::Onedrive => Endpoints {
                auth: "https://login.microsoftonline.com/common/oauth2/v2.0/authorize".into(),
                token: "https://login.microsoftonline.com/common/oauth2/v2.0/token".into(),
                api: "https://graph.microsoft.com/v1.0".into(),
                content: "https://graph.microsoft.com/v1.0".into(),
            },
            Provider::Dropbox => Endpoints {
                auth: "https://www.dropbox.com/oauth2/authorize".into(),
                token: "https://api.dropboxapi.com/oauth2/token".into(),
                api: "https://api.dropboxapi.com/2".into(),
                content: "https://content.dropboxapi.com/2".into(),
            },
        }
    }
}

/// How to sign in to `provider` with the app's registration.
pub fn client(provider: Provider, app: &AppId, ends: &Endpoints) -> Client {
    let (scopes, extra, host): (&[&str], &[(&str, &str)], &str) = match provider {
        Provider::Google => (
            &[
                "https://www.googleapis.com/auth/drive.file",
                "openid",
                "email",
                "profile",
            ],
            // A refresh token every time, also when signing in again.
            &[("access_type", "offline"), ("prompt", "consent")],
            "127.0.0.1",
        ),
        Provider::Onedrive => (
            &["Files.ReadWrite.AppFolder", "User.Read", "offline_access"],
            &[("prompt", "select_account")],
            // Microsoft takes any port with the registered http://localhost.
            "localhost",
        ),
        Provider::Dropbox => (
            &[
                "files.content.read",
                "files.content.write",
                "files.metadata.read",
                "account_info.read",
            ],
            &[("token_access_type", "offline")],
            "localhost",
        ),
    };
    Client {
        auth_url: ends.auth.clone(),
        token_url: ends.token.clone(),
        client_id: app.client_id.clone(),
        client_secret: app.client_secret.clone(),
        scopes: scopes.iter().map(|s| s.to_string()).collect(),
        extra: extra
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        redirect_host: host.into(),
        redirect_port: match provider {
            Provider::Dropbox => Some(app.redirect_port.unwrap_or(DROPBOX_PORT)),
            _ => app.redirect_port,
        },
    }
}

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

/// A project folder's name on a drive: its title, made safe for file names.
pub fn folder_name(title: &str) -> String {
    writer_core::store::safe_file_name(title, "새 작품")
}

/// Project folders on a drive (their names).
pub fn projects(s: &Session) -> Result<Vec<String>> {
    match s.provider {
        Provider::Google => google::projects(s),
        Provider::Onedrive => onedrive::projects(s),
        Provider::Dropbox => dropbox::projects(s),
    }
}

/// A project folder on a drive as a remote; made when missing and `create`.
pub fn open<'s>(
    s: &'s Session,
    folder: &str,
    create: bool,
) -> Result<Box<dyn crate::remote::Remote + 's>> {
    Ok(match s.provider {
        Provider::Google => Box::new(google::open(s, folder, create)?),
        Provider::Onedrive => Box::new(onedrive::open(s, folder, create)?),
        Provider::Dropbox => Box::new(dropbox::open(s, folder, create)?),
    })
}
