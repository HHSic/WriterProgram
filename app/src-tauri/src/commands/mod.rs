//! Commands the screen calls, one file per area. Each one is a thin wrapper
//! over `writer_core`; errors come back as a short reason in screen words.
//! The drives' commands are in `crate::drives`, the in-app browser's in
//! `crate::browser`.

pub mod cards;
pub mod doc;
pub mod journal;
pub mod exchange;
pub mod notes;
pub mod output;
pub mod project;
pub mod search;
pub mod sync_folder;
