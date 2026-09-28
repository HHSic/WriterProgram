//! Keeping a project in step with the writer's own Google Drive, OneDrive or
//! Dropbox, with no server of ours in between (docs/business-model.md, 무료
//! 동기화). This is what phones use; on a computer the drive's own program
//! usually does the job (writer_core::places, copies).
//!
//! - [`remote::Remote`]: what the engine needs from a drive: list, get, put
//!   and remove files of one project folder, each put conditional on the
//!   file's version so two devices never silently overwrite each other.
//! - [`engine::sync`]: one pass comparing this device's files, the drive's
//!   files and what both looked like after the last pass ([`base::Base`]).
//!   A file changed on both sides is kept twice: this device's version stays,
//!   the drive's becomes a copy next to it, which the app lists like the
//!   copies other sync programs leave. `project.json` is merged instead.
//! - [`folder::FolderRemote`]: a plain folder acting as a drive, for tests and
//!   for trying things without an account (흉내 드라이브).
//! - [`oauth`]: signing in with the drive's own page (PKCE, the answer comes
//!   back to a short-lived listener on this computer), and keeping the sign-in
//!   fresh. Only the refresh token is stored, in the system's credential store
//!   ([`secrets`]).
//! - [`providers`]: Google Drive (`drive.file`: only files the app made),
//!   OneDrive (the app's own folder) and Dropbox (the app's own folder).

pub mod accounts;
pub mod base;
pub mod engine;
pub mod error;
pub mod folder;
pub mod http;
pub mod oauth;
pub mod providers;
pub mod remote;
pub mod secrets;

pub use error::{Error, Result};
