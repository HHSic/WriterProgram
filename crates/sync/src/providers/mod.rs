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

use std::collections::HashMap;

use crate::Result;

mod config;
pub mod dropbox;
pub mod google;
pub mod onedrive;
mod session;

pub use config::{AppId, AppIds, DROPBOX_PORT, Endpoints, Provider, client};
pub use session::{Account, FULL, Session};
pub(crate) use session::{drive_error, space_full};

/// Path of an item inside the folder `root`, for drives that keep files by id
/// and tell only each item's parent: worked out up the chain of parents in
/// `by_id`. None when the item is not in there.
pub(crate) fn path_in<'a, T>(
    item: &'a T,
    root: &str,
    by_id: &HashMap<&str, &'a T>,
    name: impl Fn(&'a T) -> &'a str,
    parent: impl Fn(&'a T) -> Option<&'a str>,
) -> Option<String> {
    let mut parts = vec![name(item).to_string()];
    let mut up = parent(item)?;
    for _ in 0..64 {
        if up == root {
            parts.reverse();
            return Some(parts.join("/"));
        }
        let folder = *by_id.get(up)?;
        parts.push(name(folder).to_string());
        up = parent(folder)?;
    }
    None
}

/// How full a drive is, in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Space {
    pub used: u64,
    pub total: u64,
}

impl Space {
    pub fn left(&self) -> u64 {
        self.total.saturating_sub(self.used)
    }
}

/// How full the drive is, read lightly before a pass. None when the drive
/// does not say: no limit, a refusal (OneDrive's app folder permission may
/// not cover it), or no connection. Never stops a pass.
pub fn space(s: &Session) -> Option<Space> {
    match s.provider {
        Provider::Google => google::space(s),
        Provider::Onedrive => onedrive::space(s),
        Provider::Dropbox => dropbox::space(s),
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
