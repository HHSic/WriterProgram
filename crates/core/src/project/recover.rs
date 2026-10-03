//! When `project.json` is damaged (docs/safety-design.md S5): what can bring
//! it back, and doing it. The chapters are files of their own, so only the
//! structure (parts, order, settings) is at stake.
//!
//! Three ways, best first: a copy another device left through a sync program
//! (copies/), the newest daily backup (backup.rs), or rebuilding from the
//! files. Whichever is picked, the damaged file is kept as
//! `project.json.damaged-<stamp>`, and chapters the chosen structure does not
//! list are added at the end of the last part (`repair`), as when files
//! arrive by folder sync.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::Duration;

use chrono::Utc;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Map;

use super::overview::{repair, sort_by_created};
use super::{
    APP_NAME, FORMAT_VERSION, Goal, PROJECT_FILE, Project, ProjectKind, latest_backup, load,
    new_part, save,
};
use crate::cards::{self, CardType};
use crate::copies;
use crate::doc::Section;
use crate::format;
use crate::store::{modified_iso, new_id, now_iso, rename_retry, stamp};
use crate::{Error, Result};

/// One way back, as the recovery screen shows it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Choice {
    /// The backup's day (`2026-10-02`) or the copy's file name.
    pub name: String,
    /// When the copy was last written; none for a backup.
    pub modified: Option<String>,
    pub parts: usize,
    pub chapters: usize,
}

impl Choice {
    fn of(name: String, modified: Option<String>, project: &Project) -> Self {
        Choice {
            name,
            modified,
            parts: project.parts.len(),
            chapters: project.parts.iter().map(|p| p.docs.len()).sum(),
        }
    }
}

/// What can bring a damaged `project.json` back.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Recovery {
    /// The newest copy another device left (`project (1).json` and so on).
    pub copy: Option<Choice>,
    /// The newest daily backup that reads.
    pub backup: Option<Choice>,
    /// Chapter files in `manuscript/`: what rebuilding would list.
    pub chapter_files: usize,
}

/// How to bring it back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Way {
    Copy,
    Backup,
    Rebuild,
}

fn newest_copy(root: &Path) -> Option<(PathBuf, Project)> {
    copies::project_copies(root, None)
        .into_iter()
        .max_by_key(|(path, _)| fs::metadata(path).and_then(|m| m.modified()).ok())
}

/// What can bring the project back when its `project.json` is damaged;
/// none when it is not damaged.
pub fn recovery(root: &Path) -> Result<Option<Recovery>> {
    match load(root) {
        Err(Error::ProjectDamaged { .. }) => {}
        Ok(_) => return Ok(None),
        Err(e) => return Err(e),
    }
    let copy = newest_copy(root).map(|(path, project)| {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        Choice::of(name, modified_iso(&path), &project)
    });
    let backup = latest_backup(root)
        .map(|b| Choice::of(b.day.format("%Y-%m-%d").to_string(), None, &b.project));
    Ok(Some(Recovery {
        copy,
        backup,
        chapter_files: copies::scan(root, Section::Manuscript)?.ids.len(),
    }))
}

/// Brings a damaged `project.json` back the chosen way. The damaged file is
/// kept as `project.json.damaged-<stamp>`; nothing else is removed.
pub fn recover(root: &Path, way: Way) -> Result<()> {
    let path = root.join(PROJECT_FILE);
    if !matches!(load(root), Err(Error::ProjectDamaged { .. })) {
        return Err(Error::Invalid(
            "작품 구조 파일(project.json)에 고칠 곳이 없습니다".into(),
        ));
    }
    // Files dropped in by hand or left by sync programs first, so the
    // structure is worked out from the files as they will stay.
    copies::reconcile(root)?;
    let damaged = fs::read(&path).map_err(|e| Error::io(&path, e))?;
    let mut project = match way {
        Way::Copy => {
            newest_copy(root)
                .ok_or_else(|| Error::NotFound("다른 기기 사본이 없습니다".into()))?
                .1
        }
        Way::Backup => {
            latest_backup(root)
                .ok_or_else(|| Error::NotFound("되돌릴 백업이 없습니다".into()))?
                .project
        }
        Way::Rebuild => rebuild(root, &String::from_utf8_lossy(&damaged))?,
    };

    let kept = root.join(format!("{PROJECT_FILE}.damaged-{}", stamp(Utc::now())));
    rename_retry(&path, &kept, 6, Duration::from_millis(40)).map_err(|e| Error::io(&path, e))?;
    if way == Way::Copy {
        // The copy becomes project.json, as when project.json is missing.
        if let Some((copy, _)) = newest_copy(root) {
            rename_retry(&copy, &path, 6, Duration::from_millis(40))
                .map_err(|e| Error::io(&copy, e))?;
        }
    }
    repair(root, &mut project)?;
    save(root, &project)
}

/// A first string value of `key` in what is left of a damaged file.
fn salvaged(text: &str, key: &str) -> Option<String> {
    static VALUE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#""(\w+)"\s*:\s*"((?:[^"\\]|\\.)*)""#).unwrap());
    VALUE
        .captures_iter(text)
        .find(|c| &c[1] == key)
        .and_then(|c| serde_json::from_str::<String>(&format!("\"{}\"", &c[2])).ok())
        .filter(|v| !v.trim().is_empty())
}

/// A structure made from the files: every chapter by creation time in one
/// part, planning documents likewise, kinds of setting cards gathered from
/// the cards, settings at their defaults. Title, kind and id are read from
/// what is left of the damaged file when they can be.
fn rebuild(root: &Path, damaged: &str) -> Result<Project> {
    // Only what comes before the parts: an "id" further on is a part's.
    let head = damaged.find("\"parts\"").map_or(damaged, |i| &damaged[..i]);
    let kind = match salvaged(head, "kind").as_deref() {
        Some("print") => ProjectKind::Print,
        _ => ProjectKind::Webnovel,
    };
    let title = salvaged(head, "title").unwrap_or_else(|| {
        root.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "되살린 작품".into())
    });
    let id = salvaged(head, "id")
        .filter(|id| copies::is_id(id))
        .unwrap_or_else(new_id);

    let mut chapters = copies::scan(root, Section::Manuscript)?.ids;
    sort_by_created(root, Section::Manuscript, &mut chapters);
    let mut planning = copies::scan(root, Section::Planning)?.ids;
    sort_by_created(root, Section::Planning, &mut planning);
    let mut part = new_part("1부");
    part.docs = chapters;

    let mut card_types = cards::default_types();
    for card in cards::load_all(root)? {
        if !card_types.iter().any(|t| t.id == card.card_type) {
            card_types.push(CardType {
                id: card.card_type.clone(),
                name: format!("되살린 분류 {}", card_types.len() - 2),
                fields: Vec::new(),
            });
        }
    }

    let manuscript_format = format::default_for(kind);
    Ok(Project {
        app: APP_NAME.into(),
        format: FORMAT_VERSION,
        id,
        title,
        kind,
        pen_name: salvaged(head, "penName").unwrap_or_default(),
        created: salvaged(head, "created").unwrap_or_else(now_iso),
        goal: Goal {
            per_doc: None,
            count_spaces: true,
            daily: None,
        },
        scene_break: match kind {
            ProjectKind::Webnovel => "◆".into(),
            ProjectKind::Print => "*".into(),
        },
        preset: manuscript_format.preset.clone(),
        manuscript_format: Some(manuscript_format),
        card_types: Some(card_types),
        parts: vec![part],
        planning,
        keep_daily: false,
        extra: Map::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::super::{BACKUP_DIR, NewProject, add_doc, add_part, create, open};
    use super::*;
    use crate::project::NewDoc;

    /// A project with two parts and three chapters, saved (so backed up).
    fn sample() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = create(&NewProject {
            parent: dir.path().to_string_lossy().into_owned(),
            title: "달빛 서점".into(),
            kind: ProjectKind::Print,
            per_doc_goal: None,
            count_spaces: true,
            first_chapter: true,
        })
        .unwrap();
        let part = add_part(&root, "2부").unwrap();
        for title in ["둘째", "셋째"] {
            // Creation times apart, for the order a rebuild gives.
            std::thread::sleep(Duration::from_millis(5));
            add_doc(
                &root,
                &NewDoc {
                    section: Some(Section::Manuscript),
                    part_id: Some(part.clone()),
                    after: None,
                    title: title.into(),
                },
            )
            .unwrap();
        }
        (dir, root)
    }

    fn chapters(root: &Path) -> usize {
        open(root).unwrap().parts.iter().map(|p| p.docs.len()).sum()
    }

    fn damaged_kept(root: &Path) -> usize {
        fs::read_dir(root)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("project.json.damaged-")
            })
            .count()
    }

    const DAMAGE: [&[u8]; 3] = [b"", b"[1, 2, 3]", b"{\"app\": \"WriterProgram\", \"format\": 1, \"id\": \"k7q2m9x4t1ab\", \"title\": \"\xeb\x8b\xac"];

    #[test]
    fn damaged_project_files_are_reported_in_screen_words() {
        let (_dir, root) = sample();
        fs::write(root.join(PROJECT_FILE), b"{\"app\": ").unwrap();
        let e = open(&root).unwrap_err();
        assert!(matches!(e, Error::ProjectDamaged { .. }));
        assert_eq!(e.user_message(), crate::error::PROJECT_DAMAGED);
        assert!(!e.user_message().contains("expected"));
    }

    #[test]
    fn restoring_the_backup_brings_every_chapter_back() {
        for (i, damage) in DAMAGE.iter().enumerate() {
            let (_dir, root) = sample();
            let whole = fs::read(root.join(PROJECT_FILE)).unwrap();
            // A chapter made after the last backup: the backup does not list it.
            let backups = root.join(BACKUP_DIR);
            let before: Vec<_> = fs::read_dir(&backups).unwrap().collect();
            assert_eq!(before.len(), 1, "one backup a day");
            let saved = fs::read(before[0].as_ref().unwrap().path()).unwrap();
            assert_eq!(saved, whole);
            add_doc(
                &root,
                &NewDoc {
                    section: Some(Section::Manuscript),
                    part_id: None,
                    after: None,
                    title: "넷째".into(),
                },
            )
            .unwrap();
            fs::write(before[0].as_ref().unwrap().path(), &saved).unwrap();
            fs::write(root.join(PROJECT_FILE), damage).unwrap();

            let r = recovery(&root).unwrap().expect("damaged");
            assert_eq!(r.copy, None);
            let b = r.backup.expect("backup");
            assert_eq!((b.parts, b.chapters), (2, 3), "case {i}");
            assert_eq!(r.chapter_files, 4);

            recover(&root, Way::Backup).unwrap();
            assert_eq!(damaged_kept(&root), 1);
            assert_eq!(recovery(&root).unwrap(), None);
            let ov = open(&root).unwrap();
            assert_eq!(ov.parts.len(), 2);
            assert_eq!(chapters(&root), 4, "case {i}");
            assert_eq!(ov.project.title, "달빛 서점");
        }
    }

    #[test]
    fn rebuilding_without_a_backup_lists_every_chapter() {
        for (i, damage) in DAMAGE.iter().enumerate() {
            let (_dir, root) = sample();
            let id = load(&root).unwrap().id;
            fs::remove_dir_all(root.join(BACKUP_DIR)).unwrap();
            fs::write(root.join(PROJECT_FILE), damage).unwrap();
            let r = recovery(&root).unwrap().expect("damaged");
            assert_eq!((r.copy.clone(), r.backup.clone()), (None, None));
            assert!(recover(&root, Way::Backup).is_err());

            recover(&root, Way::Rebuild).unwrap();
            assert_eq!(damaged_kept(&root), 1);
            let ov = open(&root).unwrap();
            assert_eq!(ov.parts.len(), 1);
            assert_eq!(chapters(&root), 3, "case {i}");
            assert_eq!(ov.planning.len(), 2);
            let titles: Vec<_> = ov.parts[0].docs.iter().map(|d| d.title.as_str()).collect();
            assert_eq!(titles, ["", "둘째", "셋째"], "by creation time");
            if i == 2 {
                // What was left of the file: the id; the title was cut short.
                assert_eq!(ov.project.id, "k7q2m9x4t1ab");
            } else {
                assert_ne!(ov.project.id, id);
            }
        }
    }

    #[test]
    fn a_copy_from_another_device_comes_first() {
        let (_dir, root) = sample();
        let whole = fs::read(root.join(PROJECT_FILE)).unwrap();
        fs::write(root.join("project (1).json"), &whole).unwrap();
        fs::write(root.join(PROJECT_FILE), b"{").unwrap();
        let r = recovery(&root).unwrap().expect("damaged");
        let c = r.copy.expect("copy");
        assert_eq!(c.name, "project (1).json");
        assert_eq!((c.parts, c.chapters), (2, 3));
        recover(&root, Way::Copy).unwrap();
        assert!(!root.join("project (1).json").exists());
        assert_eq!(damaged_kept(&root), 1);
        assert_eq!(chapters(&root), 3);
    }

    #[test]
    fn a_whole_project_file_is_not_recovered() {
        let (_dir, root) = sample();
        assert_eq!(recovery(&root).unwrap(), None);
        assert!(recover(&root, Way::Rebuild).is_err());
        assert_eq!(damaged_kept(&root), 0);
    }
}
