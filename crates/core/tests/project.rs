use std::fs;
use std::path::{Path, PathBuf};

use chrono::Duration;
use writer_core::count::{Counts, count_blocks};
use writer_core::doc::{self, MetaPatch, Section};
use writer_core::export::{self, ExportItem, TextOptions};
use writer_core::markup::{Block, Body};
use writer_core::project::{self, NewDoc, NewProject, ProjectKind};
use writer_core::{recent, snapshot, trash};

fn new_project(dir: &Path, kind: ProjectKind) -> PathBuf {
    project::create(&NewProject {
        parent: dir.to_string_lossy().into_owned(),
        title: "달빛 서점의 마지막 손님".into(),
        kind,
        per_doc_goal: Some(5000),
        count_spaces: true,
        first_chapter: true,
    })
    .unwrap()
}

fn first_doc(root: &Path) -> String {
    project::overview(root).unwrap().parts[0].docs[0].id.clone()
}

fn body(texts: &[&str]) -> Vec<Block> {
    texts.iter().map(|t| Block::text(t)).collect()
}

#[test]
fn create_and_open() {
    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path(), ProjectKind::Webnovel);
    assert_eq!(root.file_name().unwrap(), "달빛 서점의 마지막 손님");

    let ov = project::open(&root).unwrap();
    assert_eq!(ov.project.kind, ProjectKind::Webnovel);
    assert_eq!(ov.project.goal.per_doc, Some(5000));
    assert_eq!(ov.parts.len(), 1);
    assert_eq!(ov.parts[0].title, "1부");
    assert_eq!(ov.parts[0].docs.len(), 1);
    let planning: Vec<_> = ov.planning.iter().map(|d| d.title.as_str()).collect();
    assert_eq!(planning, ["시놉시스", "작품 소개"]);
    assert_eq!(ov.trash_count, 0);

    // A second project with the same title gets its own folder.
    let again = new_project(dir.path(), ProjectKind::Print);
    assert_eq!(again.file_name().unwrap(), "달빛 서점의 마지막 손님 (2)");
    assert_eq!(project::load(&again).unwrap().scene_break, "*");
}

#[test]
fn not_a_project_folder() {
    let dir = tempfile::tempdir().unwrap();
    let err = project::open(dir.path()).unwrap_err();
    assert!(err.user_message().contains("작품 폴더가 아님"));
}

#[test]
fn saving_keeps_meta_changed_elsewhere() {
    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path(), ProjectKind::Webnovel);
    let id = first_doc(&root);

    doc::update_meta(
        &root,
        &id,
        &MetaPatch {
            title: Some("비에 젖은 손님".into()),
            status: Some("stock".into()),
            target: Some(Some(4000)),
            ..Default::default()
        },
    )
    .unwrap();
    let out = doc::save_body(
        &root,
        &id,
        body(&["셔터를 반쯤 내렸을 때 종이 울렸다."]),
        Duration::minutes(10),
    )
    .unwrap();
    assert_eq!(out.counts.with_spaces, 20);
    assert!(out.snapshot.is_none(), "an empty document needs no record");

    let file = doc::load(&root, &id).unwrap();
    assert_eq!(file.meta.title, "비에 젖은 손님");
    assert_eq!(file.meta.status, "stock");
    assert_eq!(file.meta.target, Some(4000));
    assert_eq!(file.body, body(&["셔터를 반쯤 내렸을 때 종이 울렸다."]));

    // The file on disk is plain text a writer can read.
    let text = fs::read_to_string(root.join("manuscript").join(format!("{id}.md"))).unwrap();
    assert!(text.contains("title: \"비에 젖은 손님\""));
    assert!(text.ends_with("셔터를 반쯤 내렸을 때 종이 울렸다.\n"));

    let err = doc::update_meta(
        &root,
        &id,
        &MetaPatch {
            status: Some("nope".into()),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(err.user_message().contains("알 수 없는 상태"));
}

#[test]
fn automatic_records() {
    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path(), ProjectKind::Webnovel);
    let id = first_doc(&root);
    let every = Duration::zero();

    doc::save_body(&root, &id, body(&["하나"]), every).unwrap();
    let out = doc::save_body(&root, &id, body(&["하나", "둘"]), every).unwrap();
    let rec = out.snapshot.expect("previous text is kept");
    assert_eq!(rec.kind, "auto");
    assert_eq!(rec.counts.with_spaces, 2);

    // Saving the same text again does nothing.
    let out = doc::save_body(&root, &id, body(&["하나", "둘"]), every).unwrap();
    assert!(out.snapshot.is_none());

    // Not due yet with a long interval.
    let out = doc::save_body(&root, &id, body(&["하나", "둘", "셋"]), Duration::hours(1)).unwrap();
    assert!(out.snapshot.is_none());
    assert_eq!(snapshot::list(&root, &id).unwrap().len(), 1);
}

#[test]
fn records_by_hand_and_going_back() {
    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path(), ProjectKind::Print);
    let id = first_doc(&root);
    let hour = Duration::hours(1);

    doc::save_body(&root, &id, body(&["퇴고 전 문장."]), hour).unwrap();
    let current = doc::load(&root, &id).unwrap();
    let kept = snapshot::create(&root, &current, "manual", "  1교 보내기 전 ").unwrap();
    assert_eq!(kept.name, "1교 보내기 전");

    doc::save_body(&root, &id, body(&["완전히 새로 쓴 문장."]), hour).unwrap();
    let before = snapshot::restore(&root, &id, &kept.id).unwrap();
    assert_eq!(before.kind, "before-restore");

    assert_eq!(
        doc::load(&root, &id).unwrap().body,
        body(&["퇴고 전 문장."])
    );
    let list = snapshot::list(&root, &id).unwrap();
    let kinds: Vec<_> = list.iter().map(|r| r.kind.as_str()).collect();
    assert_eq!(kinds, ["before-restore", "manual"]);
    assert_eq!(
        snapshot::load(&root, &id, &list[0].id).unwrap().body,
        body(&["완전히 새로 쓴 문장."])
    );

    assert!(snapshot::load(&root, &id, "../project").is_err());
}

#[test]
fn structure_editing() {
    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path(), ProjectKind::Webnovel);
    let first = first_doc(&root);
    let part1 = project::overview(&root).unwrap().parts[0].id.clone();

    let third = project::add_doc(&root, &NewDoc::default()).unwrap();
    let second = project::add_doc(
        &root,
        &NewDoc {
            after: Some(first.clone()),
            title: "끼워 넣은 회차".into(),
            ..Default::default()
        },
    )
    .unwrap();
    let ids = |root: &Path| -> Vec<Vec<String>> {
        project::overview(root)
            .unwrap()
            .parts
            .iter()
            .map(|p| p.docs.iter().map(|d| d.id.clone()).collect())
            .collect()
    };
    assert_eq!(
        ids(&root),
        vec![vec![first.clone(), second.clone(), third.clone()]]
    );

    let part2 = project::add_part(&root, "").unwrap();
    assert_eq!(project::overview(&root).unwrap().parts[1].title, "2부");
    project::rename_part(&root, &part2, "2부 · 비밀 서가").unwrap();
    project::move_doc(&root, &third, Some(&part2), 0).unwrap();
    project::move_doc(&root, &first, Some(&part1), 1).unwrap();
    assert_eq!(
        ids(&root),
        vec![vec![second.clone(), first.clone()], vec![third.clone()]]
    );

    assert!(project::remove_part(&root, &part2).is_err());
    project::move_doc(&root, &third, Some(&part1), 99).unwrap();
    project::remove_part(&root, &part2).unwrap();
    assert_eq!(ids(&root), vec![vec![second, first, third]]);

    let planning = project::add_doc(
        &root,
        &NewDoc {
            section: Some(Section::Planning),
            title: "플롯 구상".into(),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(project::move_doc(&root, &planning, Some(&part1), 0).is_err());
    project::move_doc(&root, &planning, None, 0).unwrap();
    assert_eq!(
        project::overview(&root).unwrap().planning[0].title,
        "플롯 구상"
    );
}

#[test]
fn trash_and_restore() {
    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path(), ProjectKind::Webnovel);
    let first = first_doc(&root);
    let second = project::add_doc(&root, &NewDoc::default()).unwrap();
    doc::save_body(&root, &first, body(&["지울 회차"]), Duration::hours(1)).unwrap();

    let item = trash::trash_doc(&root, &first).unwrap();
    assert_eq!(item.chars, 5);
    let ov = project::overview(&root).unwrap();
    assert_eq!(ov.parts[0].docs.len(), 1);
    assert_eq!(ov.trash_count, 1);
    assert!(doc::load(&root, &first).is_err());

    trash::restore(&root, &item.id).unwrap();
    let ov = project::overview(&root).unwrap();
    let order: Vec<_> = ov.parts[0].docs.iter().map(|d| d.id.clone()).collect();
    assert_eq!(order, [first.clone(), second.clone()]);
    assert_eq!(ov.trash_count, 0);

    // Deleting for good also removes the document's records.
    let current = doc::load(&root, &second).unwrap();
    snapshot::create(&root, &current, "manual", "").unwrap();
    let item = trash::trash_doc(&root, &second).unwrap();
    trash::delete(&root, &item.id).unwrap();
    assert!(trash::list(&root).unwrap().is_empty());
    assert!(!root.join(".snapshots").join(&second).exists());
}

#[test]
fn open_repairs_structure_from_files() {
    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path(), ProjectKind::Webnovel);
    let first = first_doc(&root);

    // A chapter file arrives by folder sync; another one disappears.
    let manuscript = root.join("manuscript");
    fs::write(
        manuscript.join("synced123.md"),
        "---\nid: \"synced123\"\ntitle: \"다른 기기에서 쓴 회차\"\n---\n\n본문\n",
    )
    .unwrap();
    fs::remove_file(manuscript.join(format!("{first}.md"))).unwrap();
    // A file without front matter is read as a plain chapter.
    fs::write(manuscript.join("loose.md"), "그냥 쓴 글\r\n").unwrap();

    let ov = project::open(&root).unwrap();
    let titles: Vec<_> = ov.parts[0].docs.iter().map(|d| d.title.as_str()).collect();
    assert_eq!(titles.len(), 2);
    assert!(titles.contains(&"다른 기기에서 쓴 회차"));
    assert_eq!(
        ov.total.with_spaces,
        "본문".chars().count() as u32 + "그냥 쓴 글".chars().count() as u32
    );
}

#[test]
fn text_export() {
    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path(), ProjectKind::Webnovel);
    let first = first_doc(&root);
    let second = project::add_doc(&root, &NewDoc::default()).unwrap();
    doc::save_body(
        &root,
        &first,
        vec![
            Block::text("첫 화"),
            Block::SceneBreak {},
            Block::text("장면 둘"),
        ],
        Duration::hours(1),
    )
    .unwrap();
    doc::save_body(&root, &second, body(&["둘째 화"]), Duration::hours(1)).unwrap();

    let items = vec![
        ExportItem {
            doc_id: first.clone(),
            heading: "1화 시작".into(),
            file_name: "달빛_001".into(),
        },
        ExportItem {
            doc_id: second.clone(),
            heading: "2화".into(),
            file_name: "달빛_002".into(),
        },
    ];
    let opts = TextOptions {
        include_titles: true,
        blank_line_between: true,
        scene_break: "◆".into(),
    };
    let text = export::items_text(&root, &items, &opts).unwrap();
    assert_eq!(
        text,
        "1화 시작\n\n첫 화\n\n◆\n\n장면 둘\n\n\n2화\n\n둘째 화"
    );

    let out = dir.path().join("out");
    fs::create_dir_all(&out).unwrap();
    let files = export::export_txt(&root, &items, &opts, &out, true).unwrap();
    assert_eq!(files.len(), 2);
    let bytes = fs::read(&files[0]).unwrap();
    assert!(
        bytes.starts_with(b"\xEF\xBB\xBF1\xED\x99\x94"),
        "BOM then text"
    );
    assert!(String::from_utf8_lossy(&bytes).contains("첫 화\r\n\r\n◆"));

    let single = dir.path().join("전체.txt");
    export::export_txt(&root, &items, &opts, &single, false).unwrap();
    assert!(single.is_file());
}

#[test]
fn recent_list() {
    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path(), ProjectKind::Webnovel);
    let file = dir.path().join("settings").join("recent.json");
    assert!(recent::list(&file).is_empty());

    let ov = project::open(&root).unwrap();
    recent::touch(&file, &ov).unwrap();
    recent::touch(&file, &ov).unwrap();
    let items = recent::list(&file);
    assert_eq!(items.len(), 1);
    assert!(items[0].exists);
    assert_eq!(items[0].title, "달빛 서점의 마지막 손님");

    fs::rename(&root, dir.path().join("moved")).unwrap();
    assert!(!recent::list(&file)[0].exists);
    recent::remove(&file, &items[0].path).unwrap();
    assert!(recent::list(&file).is_empty());
}

/// Same cases as app/src/editor/counts.test.ts.
#[test]
fn counts_fixture() {
    #[derive(serde::Deserialize)]
    struct Case {
        name: String,
        body: Body,
        counts: Counts,
    }
    let text = include_str!("fixtures/counts.json");
    let cases: Vec<Case> = serde_json::from_str(text).unwrap();
    for case in cases {
        assert_eq!(
            count_blocks(&case.body.content),
            case.counts,
            "{}",
            case.name
        );
    }
}
