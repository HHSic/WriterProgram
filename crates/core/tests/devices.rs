//! Writing on two devices: saves that meet another device's edits, copies
//! left by sync programs, structure arriving before files, moving a project.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::Duration;
use writer_core::cards;
use writer_core::copies::{self, Resolve};
use writer_core::doc::{self, SaveGuard, Section};
use writer_core::markup::Block;
use writer_core::project::{self, NewDoc, NewProject, ProjectKind};
use writer_core::{snapshot, trash};

fn new_project(dir: &Path) -> PathBuf {
    project::create(&NewProject {
        parent: dir.to_string_lossy().into_owned(),
        title: "달빛 서점".into(),
        kind: ProjectKind::Webnovel,
        per_doc_goal: None,
        count_spaces: true,
        first_chapter: true,
        platform: None,
    })
    .unwrap()
}

fn body(texts: &[&str]) -> Vec<Block> {
    texts.iter().map(|t| Block::text(t)).collect()
}

fn first_doc(root: &Path) -> String {
    project::overview(root).unwrap().parts[0].docs[0].id.clone()
}

fn chapter_path(root: &Path, id: &str) -> PathBuf {
    root.join("manuscript").join(format!("{id}.md"))
}

fn save(
    root: &Path,
    id: &str,
    texts: &[&str],
    base: Option<&str>,
    force: bool,
) -> doc::SaveOutcome {
    doc::save_body(
        root,
        id,
        body(texts),
        Duration::hours(1),
        SaveGuard { base, force },
    )
    .unwrap()
}

/// Another device's save, as a sync program would bring it in.
fn other_device_writes(root: &Path, id: &str, texts: &[&str]) {
    let mut file = doc::load(root, id).unwrap();
    file.body = body(texts);
    fs::write(chapter_path(root, id), doc::write_doc(&file)).unwrap();
}

#[test]
fn a_save_never_covers_another_devices_text() {
    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path());
    let id = first_doc(&root);

    let first = save(&root, &id, &["처음 문장."], None, false);
    assert!(!first.conflict);
    let base = doc::load(&root, &id).unwrap().rev();
    assert_eq!(first.rev, base);

    // Unchanged on disk: saves go through and move the base along.
    let out = save(&root, &id, &["처음 문장.", "둘째."], Some(&base), false);
    assert!(!out.conflict);
    let base = out.rev;

    other_device_writes(&root, &id, &["휴대폰에서 고친 문장."]);
    let out = save(&root, &id, &["이 PC에서 고친 문장."], Some(&base), false);
    assert!(out.conflict);
    let kept = out.snapshot.expect("this device's text is kept");
    assert_eq!(kept.kind, "this-device");
    assert_eq!(
        doc::load(&root, &id).unwrap().body,
        body(&["휴대폰에서 고친 문장."]),
        "the other device's text stays on disk"
    );
    assert_eq!(out.rev, doc::load(&root, &id).unwrap().rev());
    let mine = snapshot::load(&root, &id, &kept.id).unwrap();
    assert_eq!(mine.body, body(&["이 PC에서 고친 문장."]));

    // Keeping this device's text: the other device's goes to the records.
    let out = save(&root, &id, &["이 PC에서 고친 문장."], Some(&base), true);
    assert!(!out.conflict);
    assert_eq!(out.snapshot.unwrap().kind, "other-device");
    assert_eq!(
        doc::load(&root, &id).unwrap().body,
        body(&["이 PC에서 고친 문장."])
    );

    // Both devices ending with the same text is no conflict.
    other_device_writes(&root, &id, &["같은 문장."]);
    let out = save(&root, &id, &["같은 문장."], Some(&base), false);
    assert!(!out.conflict);

    // Before loading another device's text, the editor's is kept once.
    let rec = doc::keep_record(&root, &id, body(&["불러오기 전"]), "before-reload").unwrap();
    assert_eq!(rec.unwrap().kind, "before-reload");
    let again = doc::keep_record(&root, &id, body(&["불러오기 전"]), "before-reload").unwrap();
    assert!(again.is_none());
}

#[test]
fn chapter_copies_are_listed_apart_and_can_be_picked() {
    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path());
    let id = first_doc(&root);
    save(&root, &id, &["이 PC의 글."], None, false);

    let make_copy = |name: &str, text: &str| {
        let mut file = doc::load(&root, &id).unwrap();
        file.meta.title = "비에 젖은 손님".into();
        file.body = body(&[text]);
        fs::write(root.join("manuscript").join(name), doc::write_doc(&file)).unwrap();
    };
    make_copy(&format!("{id}-DESKTOP-1AB2C3D.md"), "노트북의 글.");

    let ov = project::open(&root).unwrap();
    assert_eq!(
        ov.parts[0].docs.len(),
        1,
        "the copy is not a chapter of its own"
    );
    assert_eq!(ov.copies.len(), 1);
    let copy = &ov.copies[0];
    assert_eq!(copy.of, id);
    assert_eq!(copy.section, Section::Manuscript);
    assert_eq!(copy.device.as_deref(), Some("DESKTOP-1AB2C3D"));
    assert_eq!(copy.title, "비에 젖은 손님");
    assert_eq!(
        copies::load(&root, Section::Manuscript, &copy.file)
            .unwrap()
            .body,
        body(&["노트북의 글."])
    );

    // Discard: into the trash, and back again as the same copy.
    copies::resolve(&root, Section::Manuscript, &copy.file, Resolve::Discard).unwrap();
    assert!(copies::list(&root).unwrap().is_empty());
    let items = trash::list(&root).unwrap();
    assert_eq!(items[0].file.as_deref(), Some(copy.file.as_str()));
    trash::restore(&root, &items[0].id).unwrap();
    assert_eq!(copies::list(&root).unwrap().len(), 1);
    assert_eq!(project::overview(&root).unwrap().parts[0].docs.len(), 1);

    // Take: the copy's text and title become the chapter's; the chapter's
    // own text is kept as a record.
    copies::resolve(&root, Section::Manuscript, &copy.file, Resolve::Take).unwrap();
    let file = doc::load(&root, &id).unwrap();
    assert_eq!(file.body, body(&["노트북의 글."]));
    assert_eq!(file.meta.title, "비에 젖은 손님");
    assert_eq!(file.meta.id, id);
    let records = snapshot::list(&root, &id).unwrap();
    assert_eq!(records[0].kind, "before-copy");
    assert!(copies::list(&root).unwrap().is_empty());

    // Keep both: the copy becomes a chapter right after the original.
    make_copy(&format!("{id} (1).md"), "구글 드라이브의 글.");
    project::add_doc(&root, &NewDoc::default()).unwrap();
    let copy = copies::list(&root).unwrap().remove(0);
    copies::resolve(&root, Section::Manuscript, &copy.file, Resolve::KeepBoth).unwrap();
    let ov = project::overview(&root).unwrap();
    let titles: Vec<_> = ov.parts[0].docs.iter().map(|d| d.title.as_str()).collect();
    assert_eq!(titles, ["비에 젖은 손님", "비에 젖은 손님 (사본)", ""]);
    assert!(ov.copies.is_empty());
    assert_ne!(ov.parts[0].docs[1].id, id);
}

#[test]
fn card_copies_swap_through_the_trash() {
    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path());
    let card = cards::create(&root, "person", "윤서하").unwrap();
    let mut other = card.clone();
    other.description = "다른 기기에서 쓴 설명".into();
    fs::write(
        root.join("cards").join(format!("{} 2.md", card.id)),
        cards::write_card(&other),
    )
    .unwrap();

    assert_eq!(cards::list(&root).unwrap().len(), 1);
    let copy = copies::list(&root).unwrap().remove(0);
    assert_eq!(copy.section, Section::Cards);
    copies::resolve(&root, Section::Cards, &copy.file, Resolve::Take).unwrap();
    assert_eq!(
        cards::load(&root, &card.id).unwrap().description,
        "다른 기기에서 쓴 설명"
    );
    // The card as it was waits in the trash, and comes back as a copy.
    let item = trash::list(&root).unwrap().remove(0);
    trash::restore(&root, &item.id).unwrap();
    let back = copies::list(&root).unwrap();
    assert_eq!(back.len(), 1);
    assert_eq!(back[0].of, card.id);
    // Deleting a copy for good leaves the original alone.
    copies::resolve(&root, Section::Cards, &back[0].file, Resolve::Discard).unwrap();
    let item = trash::list(&root).unwrap().remove(0);
    trash::delete(&root, &item.id).unwrap();
    assert_eq!(cards::list(&root).unwrap().len(), 1);
}

#[test]
fn structure_copies_are_merged_when_the_project_opens() {
    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path());
    let first = first_doc(&root);

    // The other device added a part with a chapter; this one a chapter.
    let mut theirs = project::load(&root).unwrap();
    let new_chapter = "ab12cd34ef56";
    fs::write(
        chapter_path(&root, new_chapter),
        "---\nid: \"ab12cd34ef56\"\ntitle: \"2부 첫 화\"\n---\n\n본문\n",
    )
    .unwrap();
    theirs.parts.push(project::Part {
        id: "part2".into(),
        title: "2부".into(),
        docs: vec![new_chapter.into()],
    });
    fs::write(
        root.join("project-DESKTOP-1AB2C3D.json"),
        serde_json::to_string_pretty(&theirs).unwrap(),
    )
    .unwrap();
    project::add_doc(&root, &NewDoc::default()).unwrap();

    let ov = project::open(&root).unwrap();
    let parts: Vec<_> = ov
        .parts
        .iter()
        .map(|p| (p.title.as_str(), p.docs.len()))
        .collect();
    assert_eq!(parts, [("1부", 2), ("2부", 1)]);
    assert_eq!(ov.parts[0].docs[0].id, first);
    assert!(!root.join("project-DESKTOP-1AB2C3D.json").exists());
    let kept = fs::read_dir(root.join(".snapshots").join("project"))
        .unwrap()
        .count();
    assert_eq!(kept, 1, "the copy is kept aside");
}

#[test]
fn structure_that_arrives_before_its_files() {
    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path());
    let a = first_doc(&root);
    let b = project::add_doc(&root, &NewDoc::default()).unwrap();
    let c = project::add_doc(&root, &NewDoc::default()).unwrap();

    // The file of b has not arrived yet.
    let held = fs::read(chapter_path(&root, &b)).unwrap();
    fs::remove_file(chapter_path(&root, &b)).unwrap();
    let ov = project::open(&root).unwrap();
    let ids: Vec<_> = ov.parts[0].docs.iter().map(|d| d.id.clone()).collect();
    assert_eq!(ids, [a.clone(), c.clone()]);
    assert_eq!(
        project::load(&root).unwrap().parts[0].docs.len(),
        3,
        "its place is kept"
    );

    // Moving c to the end of what the tree shows (index 1 after taking it out).
    let part = ov.parts[0].id.clone();
    project::move_doc(&root, &c, Some(&part), 1).unwrap();
    let shown: Vec<_> = project::overview(&root).unwrap().parts[0]
        .docs
        .iter()
        .map(|d| d.id.clone())
        .collect();
    assert_eq!(shown, [a.clone(), c.clone()]);
    project::move_doc(&root, &c, Some(&part), 0).unwrap();
    let shown: Vec<_> = project::overview(&root).unwrap().parts[0]
        .docs
        .iter()
        .map(|d| d.id.clone())
        .collect();
    assert_eq!(shown, [c.clone(), a.clone()]);

    // When it arrives, it shows up in its place.
    fs::write(chapter_path(&root, &b), held).unwrap();
    let shown: Vec<_> = project::overview(&root).unwrap().parts[0]
        .docs
        .iter()
        .map(|d| d.id.clone())
        .collect();
    assert_eq!(shown.len(), 3);
    assert!(shown.contains(&b));
}

#[test]
fn stray_files_and_lone_copies_are_taken_in() {
    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path());
    let first = first_doc(&root);

    // A note dropped in by hand, and a chapter whose only file is a copy.
    fs::write(root.join("manuscript").join("메모.md"), "손으로 넣은 글\n").unwrap();
    let text = fs::read_to_string(chapter_path(&root, &first)).unwrap();
    fs::remove_file(chapter_path(&root, &first)).unwrap();
    fs::write(root.join("manuscript").join(format!("{first} 2.md")), text).unwrap();

    let ov = project::open(&root).unwrap();
    let titles: Vec<_> = ov.parts[0].docs.iter().map(|d| d.title.as_str()).collect();
    assert_eq!(titles, ["", "메모"]);
    assert!(
        chapter_path(&root, &first).is_file(),
        "the copy took the chapter's place"
    );
    assert!(ov.copies.is_empty());
    let adopted = &ov.parts[0].docs[1].id;
    assert!(copies::is_id(adopted));
    assert!(!root.join("manuscript").join("메모.md").exists());
    // Opening again changes nothing.
    let again = project::open(&root).unwrap();
    assert_eq!(again.parts[0].docs.len(), 2);
}

#[test]
fn moving_a_project() {
    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path());
    let id = first_doc(&root);
    save(&root, &id, &["옮겨도 그대로."], None, false);

    let err = project::relocate(&root, &root.join("manuscript")).unwrap_err();
    assert!(err.user_message().contains("안으로는"));
    let err = project::relocate(&root, dir.path()).unwrap_err();
    assert!(err.user_message().contains("이미"));

    let dest = dir
        .path()
        .join("OneDrive")
        .join("문서")
        .join("WriterProgram");
    let moved = project::relocate(&root, &dest).unwrap();
    assert!(!moved.left_behind);
    assert!(!root.exists());
    let new_root = PathBuf::from(&moved.root);
    assert_eq!(new_root, dest.join("달빛 서점"));
    assert_eq!(
        doc::load(&new_root, &id).unwrap().body,
        body(&["옮겨도 그대로."])
    );
}
