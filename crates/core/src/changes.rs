//! What a change to a file in the project folder means for the screen. The
//! app watches the folder while a project is open; changes it did not make
//! itself (another device's edits brought by a sync program, or files changed
//! by hand) are described here and passed on.

use std::path::{Component, Path};

use serde::Serialize;

use crate::cards::CARDS_DIR;
use crate::doc::{self, body_rev};
use crate::notes::NOTES_DIR;
use crate::project::{MANUSCRIPT_DIR, PLANNING_DIR, PROJECT_FILE};
use crate::snapshot::SNAPSHOT_DIR;
use crate::trash::TRASH_DIR;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Change {
    /// `project.json`: structure and settings.
    Project,
    /// A copy of `project.json` left by a sync program (copies/).
    ProjectCopy,
    /// A manuscript or planning file (or a copy of one, whose `id` is then
    /// its file name). `rev` is the fingerprint of the text now in it; none
    /// when the file is gone.
    #[serde(rename_all = "camelCase")]
    Doc {
        id: String,
        rev: Option<String>,
    },
    Card {
        id: String,
    },
    Note {
        id: String,
    },
    /// Records (기록) of a document.
    #[serde(rename_all = "camelCase")]
    Records {
        doc_id: String,
    },
    Trash,
}

/// Describes a change to `rel` (a path inside the project folder) whose
/// content is now `bytes` (none when the file is gone). None for files the
/// screen does not care about, such as temporary files of a save.
pub fn describe(rel: &Path, bytes: Option<&[u8]>) -> Option<Change> {
    let parts: Vec<String> = rel
        .components()
        .filter_map(|c| match c {
            Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();
    let name = parts.last()?;
    // Temporary files of saves and files hidden by the system.
    if name.starts_with('.') || name.starts_with('~') {
        return None;
    }
    let stem = || name.strip_suffix(".md").map(str::to_string);
    match parts.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        [PROJECT_FILE] => Some(Change::Project),
        [file] if file.starts_with("project") && file.ends_with(".json") => {
            Some(Change::ProjectCopy)
        }
        [dir, _] if dir == MANUSCRIPT_DIR || dir == PLANNING_DIR => {
            let id = stem()?;
            let rev = match bytes {
                None => None,
                // A file still being written is not text yet; the next change
                // brings it complete.
                Some(bytes) => {
                    let text = std::str::from_utf8(bytes).ok()?;
                    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
                    let file = doc::parse_doc(&text.replace("\r\n", "\n"), &id);
                    Some(body_rev(&file.body))
                }
            };
            Some(Change::Doc { id, rev })
        }
        [CARDS_DIR, _] => Some(Change::Card { id: stem()? }),
        [NOTES_DIR, _] => Some(Change::Note { id: stem()? }),
        [SNAPSHOT_DIR, doc_id, ..] => Some(Change::Records {
            doc_id: doc_id.to_string(),
        }),
        [SNAPSHOT_DIR] => None,
        [TRASH_DIR, ..] => Some(Change::Trash),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markup::Block;

    #[test]
    fn describes_project_files() {
        let p = |s: &str| std::path::PathBuf::from(s);
        assert_eq!(
            describe(&p("project.json"), Some(b"{}")),
            Some(Change::Project)
        );
        assert_eq!(
            describe(&p("project-DESKTOP-1.json"), Some(b"{}")),
            Some(Change::ProjectCopy)
        );
        let text = "---\nid: \"abc\"\ntitle: \"t\"\n---\n\n본문\n";
        let rev = body_rev(&[Block::text("본문")]);
        assert_eq!(
            describe(&p("manuscript/abc.md"), Some(text.as_bytes())),
            Some(Change::Doc {
                id: "abc".into(),
                rev: Some(rev)
            })
        );
        assert_eq!(
            describe(&p("planning/abc.md"), None),
            Some(Change::Doc {
                id: "abc".into(),
                rev: None
            })
        );
        // A half-written multi-byte character: wait for the rest.
        assert_eq!(describe(&p("manuscript/abc.md"), Some(&[0xea, 0xb0])), None);
        assert_eq!(describe(&p("manuscript/.abc.md.x1.tmp"), Some(b"")), None);
        assert_eq!(
            describe(&p("cards/c1.md"), Some(b"")),
            Some(Change::Card { id: "c1".into() })
        );
        assert_eq!(
            describe(&p(".snapshots/abc/20260928-101500-000.auto.md"), Some(b"")),
            Some(Change::Records {
                doc_id: "abc".into()
            })
        );
        assert_eq!(
            describe(&p(".trash/x/item.json"), None),
            Some(Change::Trash)
        );
        assert_eq!(describe(&p("exports/a.txt"), Some(b"")), None);
        // The creation journal is not something the screen shows.
        assert_eq!(describe(&p(".journal/k7q2m9x4t1ab.jsonl"), Some(b"")), None);
    }
}
