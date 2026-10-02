//! Two devices keeping one project in step through a stand-in drive (a plain
//! folder): new files, edits, removals, a chapter changed on both, and the
//! structure changed on both.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use writer_core::copies;
use writer_core::doc::{self, SaveGuard};
use writer_core::markup::Block;
use writer_core::project::{self, NewDoc, NewProject, ProjectKind};
use writer_core::trash;
use writer_sync::base::Base;
use writer_sync::engine::{Report, sync};
use writer_sync::folder::FolderRemote;

struct Device {
    root: PathBuf,
    base: Base,
}

impl Device {
    fn sync(&mut self, drive: &Path) -> Report {
        let lock = Mutex::new(());
        sync(
            &self.root,
            &mut FolderRemote::new(drive),
            &mut self.base,
            &lock,
        )
        .unwrap()
    }
}

fn body(texts: &[&str]) -> Vec<Block> {
    texts.iter().map(|t| Block::text(t)).collect()
}

fn write(root: &Path, id: &str, texts: &[&str]) {
    doc::save_body(
        root,
        id,
        body(texts),
        chrono::Duration::hours(1),
        SaveGuard::default(),
    )
    .unwrap();
}

fn text(root: &Path, id: &str) -> Vec<Block> {
    doc::load(root, id).unwrap().body
}

/// Every file of a project, by path, for comparing two devices.
fn files(root: &Path) -> Vec<(String, Vec<u8>)> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) {
        for entry in fs::read_dir(dir).unwrap().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                let rel = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                out.push((rel, fs::read(&path).unwrap()));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}

fn setup() -> (tempfile::TempDir, PathBuf, Device, Device) {
    let dir = tempfile::tempdir().unwrap();
    let drive = dir.path().join("drive");
    fs::create_dir_all(&drive).unwrap();
    let a = project::create(&NewProject {
        parent: dir.path().join("a").to_string_lossy().into_owned(),
        title: "달빛 서점".into(),
        kind: ProjectKind::Webnovel,
        per_doc_goal: None,
        count_spaces: true,
        first_chapter: true,
    })
    .unwrap();
    let b = dir.path().join("b").join("달빛 서점");
    fs::create_dir_all(&b).unwrap();
    (
        dir,
        drive,
        Device {
            root: a,
            base: Base::default(),
        },
        Device {
            root: b,
            base: Base::default(),
        },
    )
}

#[test]
fn a_second_device_gets_everything_and_edits_travel_both_ways() {
    let (_dir, drive, mut a, mut b) = setup();
    let first = project::overview(&a.root).unwrap().parts[0].docs[0]
        .id
        .clone();
    write(&a.root, &first, &["노트북에서 쓴 첫 문단."]);

    let up = a.sync(&drive);
    assert!(up.uploaded.contains(&"project.json".to_string()));
    let down = b.sync(&drive);
    assert!(!down.downloaded.is_empty());
    assert_eq!(files(&a.root), files(&b.root));
    assert_eq!(project::open(&b.root).unwrap().parts[0].docs[0].id, first);

    // Nothing changed: nothing to do.
    for device in [&mut a, &mut b] {
        let quiet = device.sync(&drive);
        assert!(
            quiet.uploaded.is_empty() && quiet.downloaded.is_empty(),
            "{quiet:?}"
        );
    }

    // B edits and adds a chapter; A gets both.
    write(&b.root, &first, &["휴대폰에서 고친 첫 문단."]);
    let second = project::add_doc(&b.root, &NewDoc::default()).unwrap();
    b.sync(&drive);
    let report = a.sync(&drive);
    assert!(
        report
            .downloaded
            .iter()
            .any(|p| p.ends_with(&format!("{first}.md")))
    );
    assert_eq!(text(&a.root, &first), body(&["휴대폰에서 고친 첫 문단."]));
    assert_eq!(
        project::overview(&a.root).unwrap().parts[0].docs[1].id,
        second
    );
    assert_eq!(files(&a.root), files(&b.root));
}

#[test]
fn a_chapter_changed_on_both_devices_is_kept_twice() {
    let (_dir, drive, mut a, mut b) = setup();
    let id = project::overview(&a.root).unwrap().parts[0].docs[0]
        .id
        .clone();
    a.sync(&drive);
    b.sync(&drive);

    write(&a.root, &id, &["노트북의 글."]);
    write(&b.root, &id, &["휴대폰의 글."]);
    a.sync(&drive);
    let report = b.sync(&drive);
    assert_eq!(report.copies.len(), 1, "{report:?}");
    // B keeps its text; the other device's becomes a copy next to it.
    assert_eq!(text(&b.root, &id), body(&["휴대폰의 글."]));
    let found = copies::list(&b.root).unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].of, id);
    assert_eq!(
        copies::load(&b.root, found[0].section, &found[0].file)
            .unwrap()
            .body,
        body(&["노트북의 글."])
    );

    // A catches up: the same two versions there.
    a.sync(&drive);
    assert_eq!(text(&a.root, &id), body(&["휴대폰의 글."]));
    assert_eq!(copies::list(&a.root).unwrap().len(), 1);
    assert_eq!(files(&a.root), files(&b.root));
}

#[test]
fn structure_changed_on_both_devices_is_merged() {
    let (_dir, drive, mut a, mut b) = setup();
    a.sync(&drive);
    b.sync(&drive);

    let from_a = project::add_doc(&a.root, &NewDoc::default()).unwrap();
    let part = project::add_part(&b.root, "2부").unwrap();
    let from_b = project::add_doc(
        &b.root,
        &NewDoc {
            part_id: Some(part.clone()),
            ..Default::default()
        },
    )
    .unwrap();
    a.sync(&drive);
    let report = b.sync(&drive);
    assert!(report.merged);
    a.sync(&drive);

    for root in [&a.root, &b.root] {
        let ov = project::overview(root).unwrap();
        assert_eq!(ov.parts.len(), 2);
        assert!(ov.parts[0].docs.iter().any(|d| d.id == from_a));
        assert_eq!(ov.parts[1].docs[0].id, from_b);
        assert!(copies::list(root).unwrap().is_empty());
    }
    assert_eq!(files(&a.root), files(&b.root));
}

#[test]
fn trashing_travels_and_a_change_beats_a_removal() {
    let (_dir, drive, mut a, mut b) = setup();
    let first = project::overview(&a.root).unwrap().parts[0].docs[0]
        .id
        .clone();
    let second = project::add_doc(&a.root, &NewDoc::default()).unwrap();
    a.sync(&drive);
    b.sync(&drive);

    // A sends a chapter to the trash: B's copy of it goes there too.
    trash::trash_doc(&a.root, &second).unwrap();
    a.sync(&drive);
    let report = b.sync(&drive);
    assert!(
        report
            .removed_here
            .iter()
            .any(|p| p.ends_with(&format!("{second}.md")))
    );
    assert_eq!(project::overview(&b.root).unwrap().parts[0].docs.len(), 1);
    assert_eq!(trash::list(&b.root).unwrap().len(), 1);

    // A removes a file B has just changed: B's change stays.
    fs::remove_file(a.root.join("manuscript").join(format!("{first}.md"))).unwrap();
    write(&b.root, &first, &["지워지기 전에 고친 글."]);
    a.sync(&drive);
    b.sync(&drive);
    a.sync(&drive);
    assert_eq!(text(&a.root, &first), body(&["지워지기 전에 고친 글."]));
    assert_eq!(files(&a.root), files(&b.root));
}

#[test]
fn the_base_survives_a_restart() {
    let (dir, drive, mut a, _) = setup();
    a.sync(&drive);
    let file = dir.path().join("base.json");
    a.base.save(&file).unwrap();
    let again = Base::load(&file);
    assert_eq!(again, a.base);
    // With the base back, a pass finds nothing to do.
    let mut a2 = Device {
        root: a.root.clone(),
        base: again,
    };
    let report = a2.sync(&drive);
    assert!(
        report.uploaded.is_empty() && report.downloaded.is_empty(),
        "{report:?}"
    );
    // Temporary files of a save never travel.
    fs::write(a.root.join("manuscript").join(".x.md.123.tmp"), "half").unwrap();
    assert!(a2.sync(&drive).uploaded.is_empty());
}

#[test]
fn each_device_journal_travels_without_copies() {
    use writer_core::journal::{self, Entry, Paste};
    let (_dir, drive, mut a, mut b) = setup();
    let paste = |chars| {
        Entry::Paste(Paste {
            doc: "d1".into(),
            chars,
            outside: true,
        })
    };
    journal::append(&a.root, "devaaaaaaaaa", &paste(100)).unwrap();
    a.sync(&drive);
    b.sync(&drive);
    // Both write at the same time, each to its own file.
    journal::append(&a.root, "devaaaaaaaaa", &paste(200)).unwrap();
    journal::append(&b.root, "devbbbbbbbbb", &paste(300)).unwrap();
    let up = b.sync(&drive);
    assert!(
        up.uploaded
            .contains(&".journal/devbbbbbbbbb.jsonl".to_string())
    );
    let report = a.sync(&drive);
    assert!(report.copies.is_empty(), "{report:?}");
    b.sync(&drive);
    assert_eq!(files(&a.root), files(&b.root));
    let check = journal::verify(&b.root).unwrap();
    assert!(check.ok);
    assert_eq!(check.files.len(), 2);
}
