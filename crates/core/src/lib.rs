//! Core of WriterProgram: the project folder format, manuscript files, counts,
//! records (snapshots), trash and plain-text export.
//!
//! Nothing here depends on the UI or on Tauri, so the whole storage layer can be
//! tested with plain `cargo test` and reused on mobile. The on-disk format is
//! described in `docs/architecture.md`.

pub mod count;
pub mod doc;
pub mod error;
pub mod export;
pub mod markup;
pub mod project;
pub mod recent;
pub mod snapshot;
pub mod store;
pub mod trash;

pub use error::{Error, Result};
