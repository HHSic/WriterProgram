//! 맞춤법 검사 over a project, with the test dictionary in
//! tests/fixtures/spell (not hunspell-dict-ko, which is never kept here).

use std::path::{Path, PathBuf};

use chrono::Duration;
use writer_core::doc::{self, MetaPatch};
use writer_core::markup::Block;
use writer_core::project::{self, NewDoc, NewProject, ProjectKind};
use writer_core::spell::{self, Known, Speller, words};

const AFF: &str = include_str!("fixtures/spell/ko.aff");
const DIC: &str = include_str!("fixtures/spell/ko.dic");

fn new_project(dir: &Path) -> PathBuf {
    project::create(&NewProject {
        parent: dir.to_string_lossy().into_owned(),
        title: "시험".into(),
        kind: ProjectKind::Webnovel,
        per_doc_goal: None,
        count_spaces: true,
        first_chapter: true,
        platform: None,
    })
    .unwrap()
}

fn write(root: &Path, id: &str, texts: &[&str]) {
    let body: Vec<Block> = texts.iter().map(|t| Block::text(t)).collect();
    doc::save_body(root, id, body, Duration::hours(1), Default::default()).unwrap();
}

#[test]
fn whole_work_lists_issues_by_chapter_and_marks_locked_ones() {
    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path());
    let first = project::overview(&root).unwrap().parts[0].docs[0]
        .id
        .clone();
    let second = project::add_doc(&root, &NewDoc::default()).unwrap();
    write(&root, &first, &["나무는 멱었다.", "서하가 웃었다."]);
    write(&root, &second, &["사람을 멱었다."]);
    doc::update_meta(
        &root,
        &second,
        &MetaPatch {
            locked: Some(true),
            ..Default::default()
        },
    )
    .unwrap();

    let speller = Speller::new(AFF, DIC).unwrap();
    let words_file = words::project_words_file(&root);
    words::add(&words_file, "서하").unwrap();
    let known = Known::new(words::read(&words_file).unwrap());
    let found = spell::check_work(&root, &speller, &known, None).unwrap();
    assert_eq!(found.total, 2);
    assert_eq!(found.docs[0].doc_id, first);
    assert!(!found.docs[0].locked);
    let issue = &found.docs[0].issues[0];
    assert_eq!((issue.block, issue.start, issue.end), (0, 4, 7));
    assert_eq!(issue.text, "멱었다");
    assert_eq!(found.docs[1].doc_id, second);
    assert!(found.docs[1].locked);

    // Only some documents.
    let one = spell::check_work(&root, &speller, &known, Some(vec![second.clone()])).unwrap();
    assert_eq!(one.total, 1);
}
