//! Core of WriterProgram: the project folder format, manuscript files, counts,
//! records (snapshots), setting cards, notes (메모), trash, manuscript format and
//! export (txt, docx, HWPX).
//!
//! Nothing here depends on the UI or on Tauri, so the whole storage layer can be
//! tested with plain `cargo test` and reused on mobile. The on-disk format is
//! described in `docs/architecture.md`.

pub mod cards;
pub mod changes;
pub mod copies;
pub mod count;
pub mod doc;
pub mod docx;
pub mod error;
pub mod export;
pub mod format;
pub mod hwpx;
pub mod import;
pub mod indent;
pub mod layout;
pub mod markup;
pub mod notes;
pub mod places;
pub mod project;
pub mod recent;
pub mod search;
pub mod snapshot;
pub mod store;
pub mod trash;
mod xml;

pub use error::{Error, Result};
