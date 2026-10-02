//! Tests of telling copies apart and merging copies of `project.json`.

use super::scan::{Found, classify};
use super::*;
use crate::project::{Part, Project};

fn found(file: &str, front: Option<&str>, secs: u64) -> Found {
    Found {
        file: file.into(),
        stem: file.trim_end_matches(".md").into(),
        front_id: front.map(str::to_string),
        modified: Some(std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs)),
    }
}

#[test]
fn names_from_each_sync_program() {
    let id = "k7q2m9x4t1ab";
    let files = [
        found("k7q2m9x4t1ab.md", Some(id), 1),
        found("k7q2m9x4t1ab-DESKTOP-1AB2C3D.md", Some(id), 2),
        found(
            "k7q2m9x4t1ab (홍길동's conflicted copy 2026-09-28).md",
            Some(id),
            3,
        ),
        found("k7q2m9x4t1ab 2.md", Some(id), 4),
        found("k7q2m9x4t1ab (1).md", Some(id), 5),
        found(
            "k7q2m9x4t1ab.sync-conflict-20260928-101500-ABCDEF7.md",
            Some(id),
            6,
        ),
        // Without front matter the name gives it away.
        found("k7q2m9x4t1ab - 복사본.md", None, 7),
        // Hand-made files without front matter are chapters of their own.
        found("loose.md", None, 8),
        found("메모.md", None, 9),
    ];
    let scan = classify(&files);
    assert_eq!(scan.ids, ["k7q2m9x4t1ab", "loose"]);
    assert_eq!(scan.copies.len(), 6);
    assert!(scan.copies.iter().all(|(_, of)| of == id));
    assert_eq!(scan.loose, ["메모.md"]);
    assert!(scan.orphans.is_empty());
}

#[test]
fn newest_copy_takes_the_place_of_a_missing_original() {
    let files = [
        found("abc-LAPTOP.md", Some("abc"), 10),
        found("abc (1).md", Some("abc"), 20),
    ];
    let scan = classify(&files);
    assert_eq!(
        scan.orphans,
        [("abc (1).md".to_string(), "abc".to_string())]
    );
    assert_eq!(
        scan.copies,
        [("abc-LAPTOP.md".to_string(), "abc".to_string())]
    );
}

#[test]
fn device_names() {
    let id = "k7q2m9x4t1ab";
    assert_eq!(
        device_of("k7q2m9x4t1ab-DESKTOP-1AB2C3D", id).as_deref(),
        Some("DESKTOP-1AB2C3D")
    );
    assert_eq!(
        device_of("k7q2m9x4t1ab-DESKTOP-1AB2C3D-2", id).as_deref(),
        Some("DESKTOP-1AB2C3D")
    );
    assert_eq!(
        device_of("k7q2m9x4t1ab (홍길동's conflicted copy 2026-09-28)", id).as_deref(),
        Some("홍길동")
    );
    assert_eq!(device_of("k7q2m9x4t1ab 2", id), None);
    assert_eq!(device_of("k7q2m9x4t1ab (1)", id), None);
}

fn part(id: &str, docs: &[&str]) -> Part {
    Part {
        id: id.into(),
        title: id.to_uppercase(),
        docs: docs.iter().map(|d| d.to_string()).collect(),
    }
}

fn project_with(parts: Vec<Part>, planning: &[&str]) -> Project {
    let mut p: Project = serde_json::from_str(
        r#"{"app":"WriterProgram","format":1,"id":"p","title":"t","kind":"webnovel","goal":{}}"#,
    )
    .unwrap();
    p.parts = parts;
    p.planning = planning.iter().map(|d| d.to_string()).collect();
    p
}

#[test]
fn merging_structure_keeps_both_devices_chapters() {
    // This device added c2 after a2; the other added b1 after a1 and a
    // new part with d1, and a planning document.
    let mut mine = project_with(vec![part("a", &["a1", "a2", "c2"])], &["s"]);
    let theirs = project_with(
        vec![
            part("a", &["a1", "b1", "a2"]),
            part("d", &["d1"]),
            part("e", &[]),
        ],
        &["s", "s2"],
    );
    merge_project(&mut mine, &theirs);
    assert_eq!(
        mine.parts.len(),
        2,
        "an empty part only the copy has stays out"
    );
    assert_eq!(mine.parts[0].docs, ["a1", "b1", "a2", "c2"]);
    assert_eq!(mine.parts[1].id, "d");
    assert_eq!(mine.parts[1].docs, ["d1"]);
    assert_eq!(mine.planning, ["s", "s2"]);

    // Chapters placed differently stay where this device has them.
    let mut mine = project_with(vec![part("a", &["a1"]), part("b", &["a2"])], &[]);
    let theirs = project_with(vec![part("a", &["a1", "a2"]), part("b", &[])], &[]);
    merge_project(&mut mine, &theirs);
    assert_eq!(mine.parts[0].docs, ["a1"]);
    assert_eq!(mine.parts[1].docs, ["a2"]);
}
