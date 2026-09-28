//! What this device and the drive looked like after the last pass, file by
//! file. A file whose fingerprint here or version there differs from it was
//! changed on that side since. Kept per device (in the app's settings folder,
//! not in the project), so it never travels to the drive.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use writer_core::store::{atomic_write, read_text};

use crate::Result;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Base {
    #[serde(default)]
    pub files: BTreeMap<String, Known>,
    /// When the last pass finished (RFC 3339).
    #[serde(default)]
    pub synced_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Known {
    /// Fingerprint of the content on this device (`writer_core::store::rev_of`).
    pub local: String,
    /// The drive's version.
    pub remote: String,
    /// Size and modification time of the file here when last read, to skip
    /// reading files that did not change.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stamp: Option<(u64, i64)>,
}

impl Base {
    /// Reads a saved base; a missing or unreadable one starts empty (the
    /// next pass then treats every file as new on both sides, which is safe).
    pub fn load(file: &Path) -> Base {
        read_text(file)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, file: &Path) -> Result<()> {
        let text = serde_json::to_string_pretty(self).expect("base serializes");
        Ok(atomic_write(file, text.as_bytes())?)
    }
}
