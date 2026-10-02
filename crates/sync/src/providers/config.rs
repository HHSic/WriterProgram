//! The drives and the app's registration with each: which ones there are,
//! the app ids, where each drive's services are, and how to sign in.

use serde::{Deserialize, Serialize};

use crate::oauth::Client;

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
