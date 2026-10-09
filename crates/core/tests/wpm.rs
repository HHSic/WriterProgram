use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use writer_core::doc::{self, SaveGuard};
use writer_core::markup::Block;
use writer_core::project::{self, NewProject, ProjectKind};
use writer_core::wpm;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

fn make_project(parent: &Path) -> PathBuf {
    project::create(&NewProject {
        parent: parent.to_string_lossy().into_owned(),
        title: "달빛 아래".into(),
        kind: ProjectKind::Print,
        per_doc_goal: None,
        count_spaces: true,
        first_chapter: true,
        platform: None,
    })
    .unwrap()
}

fn archive(path: &Path, entries: &[(&str, &[u8])]) {
    let mut zip = ZipWriter::new(fs::File::create(path).unwrap());
    for (name, bytes) in entries {
        zip.start_file(*name, SimpleFileOptions::default()).unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap();
}

const MANIFEST: &[u8] = br#"{"kind":"writerprogram-manuscript","format":1}"#;

#[test]
fn whole_work_round_trips_and_can_be_edited_and_saved_again() {
    let dir = tempfile::tempdir().unwrap();
    let root = make_project(dir.path());
    let id = project::overview(&root).unwrap().parts[0].docs[0]
        .id
        .clone();
    let text = "한글 원고 😊 ❤️ 👩‍💻 가족 👨‍👩‍👧‍👦";
    doc::save_body(
        &root,
        &id,
        vec![Block::text(text)],
        chrono::Duration::minutes(10),
        SaveGuard::default(),
    )
    .unwrap();
    for (name, bytes) in [
        ("cards/person.md", "인물 설정".as_bytes()),
        ("notes/note.md", "나중에 고칠 곳".as_bytes()),
        (".snapshots/doc/kept.md", "이전 원고".as_bytes()),
        (".trash/item/item.json", b"{}".as_slice()),
        (".journal/device/2026-10.jsonl", b"journal\n".as_slice()),
        (".journal/anchors/time.tsr", b"\x00\xff\x10".as_slice()),
        (".exchanges/sent/original.docx", b"\x00\xfe\x10".as_slice()),
    ] {
        let path = root.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    fs::write(root.join("manuscript/.unsaved.tmp"), "temporary").unwrap();
    fs::write(root.join("unrelated.txt"), "not project data").unwrap();
    let dest = dir.path().join("달빛 아래.wpm");
    wpm::save(&root, &dest).unwrap();
    let before = wpm::fingerprint(&dest).unwrap();
    let opened = wpm::unpack(&dest, &dir.path().join("work")).unwrap();
    assert_eq!(
        doc::load(&opened, &id).unwrap().body,
        vec![Block::text(text)]
    );
    assert_eq!(
        fs::read(root.join("project.json")).unwrap(),
        fs::read(opened.join("project.json")).unwrap()
    );
    for name in [
        "cards/person.md",
        "notes/note.md",
        ".snapshots/doc/kept.md",
        ".trash/item/item.json",
        ".journal/device/2026-10.jsonl",
        ".journal/anchors/time.tsr",
        ".exchanges/sent/original.docx",
    ] {
        assert_eq!(
            fs::read(root.join(name)).unwrap(),
            fs::read(opened.join(name)).unwrap()
        );
    }
    assert!(!opened.join("manuscript/.unsaved.tmp").exists());
    assert!(!opened.join("unrelated.txt").exists());
    let again = wpm::unpack(&dest, &dir.path().join("work")).unwrap();
    assert_ne!(opened, again);
    doc::save_body(
        &opened,
        &id,
        vec![Block::text("고친 원고 🥰")],
        chrono::Duration::minutes(10),
        SaveGuard::default(),
    )
    .unwrap();
    wpm::save(&opened, &dest).unwrap();
    assert_ne!(before, wpm::fingerprint(&dest).unwrap());
    let reopened = wpm::unpack(&dest, &dir.path().join("work")).unwrap();
    assert_eq!(
        doc::load(&reopened, &id).unwrap().body,
        vec![Block::text("고친 원고 🥰")]
    );
}

#[test]
fn refuses_unsafe_paths_and_case_collisions_without_writing_outside_work() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("bad.wpm");
    let parent = dir.path().join("work");
    for name in [
        "../escape.md",
        "/escape.md",
        "manuscript/../../escape.md",
        "manuscript/..\\escape.md",
        "manuscript/C:escape.md",
        "manuscript/CON.md",
        "manuscript/file.md.",
        "manuscript/./file.md",
    ] {
        archive(&source, &[("wpm.json", MANIFEST), (name, b"bad")]);
        assert!(wpm::unpack(&source, &parent).is_err(), "{name}");
        assert!(!dir.path().join("escape.md").exists());
    }
    archive(
        &source,
        &[
            ("wpm.json", MANIFEST),
            ("manuscript/A.md", b"one"),
            ("manuscript/a.md", b"two"),
        ],
    );
    assert!(wpm::unpack(&source, &parent).is_err());
    assert!(!parent.exists());
}

#[test]
fn rejects_wrong_or_newer_formats_and_cleans_up_damaged_projects() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("bad.wpm");
    let parent = dir.path().join("work");
    for manifest in [
        MANIFEST,
        b"not JSON",
        br#"{"kind":"other","format":1}"#,
        br#"{"kind":"writerprogram-manuscript","format":2}"#,
    ] {
        archive(&source, &[("wpm.json", manifest), ("project.json", b"{}")]);
        assert!(wpm::unpack(&source, &parent).is_err());
        assert!(!parent.exists() || fs::read_dir(&parent).unwrap().count() == 0);
    }
    archive(&source, &[("project.json", b"{}")]);
    assert!(wpm::unpack(&source, &parent).is_err());
    fs::write(&source, "not zip").unwrap();
    assert!(wpm::unpack(&source, &parent).is_err());
}

#[test]
fn failed_save_keeps_the_previous_package_and_rejects_a_destination_inside_work() {
    let dir = tempfile::tempdir().unwrap();
    let root = make_project(dir.path());
    let file = dir.path().join("work.wpm");
    wpm::save(&root, &file).unwrap();
    let previous = fs::read(&file).unwrap();
    let project_file = fs::read(root.join("project.json")).unwrap();
    fs::write(root.join("project.json"), "damaged project").unwrap();
    assert!(wpm::save(&root, &file).is_err());
    assert_eq!(fs::read(&file).unwrap(), previous);
    fs::write(root.join("project.json"), project_file).unwrap();
    assert!(wpm::save(&root, &root.join("recursive.wpm")).is_err());
    assert!(wpm::save(&root, &dir.path().join("wrong.txt")).is_err());
    assert!(!dir.path().read_dir().unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .ends_with(".tmp")
    }));
}

#[cfg(unix)]
#[test]
fn never_follows_symlinks_in_either_direction() {
    let dir = tempfile::tempdir().unwrap();
    let root = make_project(dir.path());
    let file = dir.path().join("work.wpm");
    std::os::unix::fs::symlink("/etc/passwd", root.join("manuscript/link.md")).unwrap();
    assert!(wpm::save(&root, &file).is_err());
    let mut zip = ZipWriter::new(fs::File::create(&file).unwrap());
    zip.start_file("wpm.json", SimpleFileOptions::default())
        .unwrap();
    zip.write_all(MANIFEST).unwrap();
    zip.add_symlink(
        "manuscript/link.md",
        "../../outside",
        SimpleFileOptions::default(),
    )
    .unwrap();
    zip.finish().unwrap();
    assert!(wpm::unpack(&file, &dir.path().join("work")).is_err());
}
