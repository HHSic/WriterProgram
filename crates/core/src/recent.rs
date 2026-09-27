//! Recently opened projects, kept in the app's settings folder.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::Result;
use crate::project::{Overview, PROJECT_FILE, ProjectKind};
use crate::store::{atomic_write, now_iso, read_text};

const MAX_ITEMS: usize = 30;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentItem {
    pub path: String,
    pub title: String,
    pub kind: ProjectKind,
    #[serde(default)]
    pub chars: u32,
    pub opened_at: String,
    /// Whether the folder is still there. Worked out when listing.
    #[serde(default, skip_deserializing)]
    pub exists: bool,
}

fn same_path(a: &str, b: &str) -> bool {
    if cfg!(windows) {
        a.eq_ignore_ascii_case(b)
    } else {
        a == b
    }
}

fn read(file: &Path) -> Vec<RecentItem> {
    read_text(file)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn write(file: &Path, items: &[RecentItem]) -> Result<()> {
    let text = serde_json::to_string_pretty(items).expect("items serialize");
    atomic_write(file, text.as_bytes())
}

pub fn list(file: &Path) -> Vec<RecentItem> {
    let mut items = read(file);
    for item in &mut items {
        item.exists = Path::new(&item.path).join(PROJECT_FILE).is_file();
    }
    items
}

/// Puts a project at the top of the list with its latest title and length.
pub fn touch(file: &Path, overview: &Overview) -> Result<()> {
    let mut items = read(file);
    items.retain(|i| !same_path(&i.path, &overview.root));
    items.insert(
        0,
        RecentItem {
            path: overview.root.clone(),
            title: overview.project.title.clone(),
            kind: overview.project.kind,
            chars: overview.total.with_spaces,
            opened_at: now_iso(),
            exists: true,
        },
    );
    items.truncate(MAX_ITEMS);
    write(file, &items)
}

/// Takes a project off the list. Its files are not touched.
pub fn remove(file: &Path, path: &str) -> Result<()> {
    let mut items = read(file);
    items.retain(|i| !same_path(&i.path, path));
    write(file, &items)
}
