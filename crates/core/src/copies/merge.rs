//! What is sorted out when a project opens: copies whose original is gone,
//! files dropped in by hand, and copies of `project.json` merged into it.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;

use super::{SECTIONS, rename, scan};
use crate::cards;
use crate::doc::{self, Section};
use crate::project::{self, PROJECT_FILE, Part, Project};
use crate::snapshot::SNAPSHOT_DIR;
use crate::store::{modified_iso, read_text, stable_id, stamp};
use crate::{Error, Result};

/// Lets copies whose original is gone take its place, and takes in manuscript
/// and planning files whose names are not ids. True when anything changed.
pub fn reconcile(root: &Path) -> Result<bool> {
    let mut changed = false;
    for section in SECTIONS {
        let found = scan(root, section)?;
        let dir = root.join(section.dir());
        for (file, of) in &found.orphans {
            let target = dir.join(doc::file_name(of));
            if !target.exists() {
                rename(&dir.join(file), &target)?;
                changed = true;
            }
        }
        if !matches!(section, Section::Manuscript | Section::Planning) {
            continue;
        }
        for file in &found.loose {
            let stem = file.strip_suffix(".md").unwrap_or(file);
            let id = stable_id(&format!("{}/{stem}", section.dir()));
            let target = dir.join(doc::file_name(&id));
            if target.exists() || doc::locate(root, &id).is_ok() {
                // Taken in already (on this or another device): the file is
                // left as it is and shows up as a copy of that one.
                continue;
            }
            let source = dir.join(file);
            let mut d = doc::parse_doc(&read_text(&source)?, &id);
            d.meta.id = id.clone();
            if d.meta.title.trim().is_empty() {
                d.meta.title = stem.to_string();
            }
            if d.meta.created.is_empty() {
                d.meta.created = modified_iso(&source).unwrap_or_default();
            }
            doc::write_doc_file(&target, &d)?;
            fs::remove_file(&source).map_err(|e| Error::io(&source, e))?;
            changed = true;
        }
    }
    Ok(changed)
}

/// Copies of `project.json` in the project folder: `project-DESKTOP-1.json`,
/// `project (1).json` and so on, that belong to this project.
fn project_copies(root: &Path, id: Option<&str>) -> Vec<(PathBuf, Project)> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut out: Vec<(PathBuf, Project)> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            name != PROJECT_FILE
                && name.starts_with("project")
                && name.ends_with(".json")
                && e.path().is_file()
        })
        .filter_map(|e| {
            let project: Project = serde_json::from_str(&read_text(&e.path()).ok()?).ok()?;
            (project.app == project::APP_NAME && id.is_none_or(|id| project.id == id))
                .then(|| (e.path(), project))
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// When `project.json` itself is missing but a copy of it is there, the copy
/// takes its place. True when that happened.
pub fn restore_project_file(root: &Path) -> Result<bool> {
    if root.join(PROJECT_FILE).exists() {
        return Ok(false);
    }
    let Some((path, _)) = project_copies(root, None).into_iter().next() else {
        return Ok(false);
    };
    rename(&path, &root.join(PROJECT_FILE))?;
    Ok(true)
}

/// Merges copies of `project.json` into it: parts and documents only the copy
/// has are added where the copy had them; everything else stays as it is.
/// Each merged copy is kept under `.snapshots/project/`. True when any copy
/// was merged.
pub fn merge_project_copies(root: &Path) -> Result<bool> {
    let mut current = project::load(root)?;
    let copies = project_copies(root, Some(&current.id));
    if copies.is_empty() {
        return Ok(false);
    }
    for (_, copy) in &copies {
        merge_project(&mut current, copy);
    }
    project::save(root, &current)?;
    let keep = root.join(SNAPSHOT_DIR).join("project");
    fs::create_dir_all(&keep).map_err(|e| Error::io(&keep, e))?;
    for (path, _) in &copies {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let target = keep.join(format!("{}-{name}", stamp(Utc::now())));
        rename(path, &target)?;
    }
    Ok(true)
}

/// Adds to `into` what only `other` has. `into` wins wherever both have
/// something: titles, order, settings.
pub fn merge_project(into: &mut Project, other: &Project) {
    let mut placed: HashSet<String> = project::doc_ids(into).into_iter().collect();

    for (pi, part) in other.parts.iter().enumerate() {
        match into.parts.iter().position(|p| p.id == part.id) {
            Some(i) => {
                for (di, doc) in part.docs.iter().enumerate() {
                    if placed.contains(doc) {
                        continue;
                    }
                    // Right after the chapter it followed in the copy.
                    let at = part.docs[..di]
                        .iter()
                        .rev()
                        .find_map(|prev| into.parts[i].docs.iter().position(|d| d == prev))
                        .map_or(0, |p| p + 1);
                    into.parts[i].docs.insert(at, doc.clone());
                    placed.insert(doc.clone());
                }
            }
            // A part only the copy has comes along when it brings chapters.
            None => {
                let docs: Vec<String> = part
                    .docs
                    .iter()
                    .filter(|d| placed.insert((*d).clone()))
                    .cloned()
                    .collect();
                if docs.is_empty() {
                    continue;
                }
                let at = other.parts[..pi]
                    .iter()
                    .rev()
                    .find_map(|prev| into.parts.iter().position(|p| p.id == prev.id))
                    .map_or(0, |p| p + 1);
                into.parts.insert(
                    at,
                    Part {
                        id: part.id.clone(),
                        title: part.title.clone(),
                        docs,
                    },
                );
            }
        }
    }

    for (di, doc) in other.planning.iter().enumerate() {
        if placed.contains(doc) {
            continue;
        }
        let at = other.planning[..di]
            .iter()
            .rev()
            .find_map(|prev| into.planning.iter().position(|d| d == prev))
            .map_or(0, |p| p + 1);
        into.planning.insert(at, doc.clone());
        placed.insert(doc.clone());
    }

    if let Some(types) = &other.card_types {
        let mine = into.card_types.get_or_insert_with(cards::default_types);
        for t in types {
            if !mine.iter().any(|m| m.id == t.id) {
                mine.push(t.clone());
            }
        }
    }
    for (key, value) in &other.extra {
        into.extra
            .entry(key.clone())
            .or_insert_with(|| value.clone());
    }
}
