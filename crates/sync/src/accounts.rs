//! What this device remembers about drives, apart from the secrets: which
//! drives are connected (and as whom), and which projects are kept in step
//! with which drive folder. Both are small JSON files in the app's settings
//! folder, never in a project.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use writer_core::store::{atomic_write, now_iso, read_text};

use crate::Result;
use crate::base::Base;
use crate::engine::{self, Report};
use crate::http::Http;
use crate::oauth::{self, Listener};
use crate::providers::{self, Account, AppId, Endpoints, Provider, Session};
use crate::secrets::Secrets;

/// A drive this device is signed in to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
    pub provider: Provider,
    pub account: Account,
    /// Where the sign-in is kept in the credential store.
    pub key: String,
    pub connected_at: String,
}

/// A project kept in step with a folder on a drive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    pub provider: Provider,
    /// The project's folder on the drive.
    pub folder: String,
    pub linked_at: String,
    #[serde(default)]
    pub synced_at: Option<String>,
    /// Why the last pass stopped, in screen words; none when it went through.
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Registry {
    #[serde(default)]
    pub connections: Vec<Connection>,
    /// By project id.
    #[serde(default)]
    pub links: BTreeMap<String, Link>,
}

impl Registry {
    pub fn load(file: &Path) -> Registry {
        read_text(file)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, file: &Path) -> Result<()> {
        let text = serde_json::to_string_pretty(self).expect("registry serializes");
        Ok(atomic_write(file, text.as_bytes())?)
    }

    pub fn connection(&self, provider: Provider) -> Option<&Connection> {
        self.connections.iter().find(|c| c.provider == provider)
    }
}

/// Where a project's base (what both sides looked like after the last pass)
/// is kept on this device.
pub fn base_file(dir: &Path, project_id: &str) -> PathBuf {
    dir.join(format!("{project_id}.json"))
}

/// Signs in with the drive's page in the browser (`open` shows it), keeps the
/// sign-in, and returns the connection to remember.
pub fn sign_in(
    provider: Provider,
    app: &AppId,
    ends: Endpoints,
    secrets: Arc<dyn Secrets>,
    open: impl FnOnce(&str) -> Result<()>,
    cancel: &AtomicBool,
) -> Result<Connection> {
    let client = providers::client(provider, app, &ends);
    let listener = Listener::bind(&client)?;
    let pkce = oauth::pkce()?;
    let state = oauth::new_state()?;
    open(&oauth::sign_in_url(
        &client,
        &listener.redirect,
        &state,
        &pkce,
    ))?;
    let code = listener.wait(&state, Duration::from_secs(300), cancel)?;
    let tokens = oauth::exchange(&Http::default(), &client, &code, &pkce, &listener.redirect)?;
    let session = Session::new(provider, app, ends, tokens, secrets);
    let account = session.account()?;
    let key = oauth::secret_key(provider.key(), &account.email);
    session.keep(&key)?;
    Ok(Connection {
        provider,
        account,
        key,
        connected_at: now_iso(),
    })
}

/// One pass for a linked project.
pub fn sync_project(
    session: &Session,
    link: &Link,
    root: &Path,
    base_file: &Path,
    lock: &Mutex<()>,
) -> Result<Report> {
    let mut remote = providers::open(session, &link.folder, true)?;
    let mut base = Base::load(base_file);
    let report = engine::sync(root, remote.as_mut(), &mut base, lock);
    // Keep what went through even when the pass stopped half way.
    base.save(base_file)?;
    report
}
