//! The creation journal as the app uses it: saves, records and imports add
//! lines while it is on, nothing while it is off, and no manuscript text ever
//! goes in. One test only: the journal's device is set for the whole process.

use std::fs;

use chrono::Duration;
use writer_core::doc::{self, SaveGuard};
use writer_core::import::{self, CommitSpec, ImportOptions, Pick};
use writer_core::journal;
use writer_core::markup::Block;
use writer_core::project::{self, NewProject, ProjectKind};
use writer_core::snapshot;

const DEVICE: &str = "k7q2m9x4t1ab";
const SECRET: &str = "달빛이 서점 유리문에 번지던 밤";
const IMPORTED: &str = "가져온 원고의 첫 문장은 비밀이다";

fn kinds(text: &str) -> Vec<String> {
    text.lines()
        .map(|l| {
            let v: serde_json::Value = serde_json::from_str(l).unwrap();
            v["kind"].as_str().unwrap().to_string()
        })
        .collect()
}

#[test]
fn saves_records_and_imports_are_journaled_without_text() {
    let dir = tempfile::tempdir().unwrap();
    let root = project::create(&NewProject {
        parent: dir.path().to_string_lossy().into_owned(),
        title: "일지 시험".into(),
        kind: ProjectKind::Webnovel,
        per_doc_goal: None,
        count_spaces: true,
        first_chapter: true,
    })
    .unwrap();
    let id = project::overview(&root).unwrap().parts[0].docs[0]
        .id
        .clone();
    let save = |texts: &[&str]| {
        doc::save_body(
            &root,
            &id,
            texts.iter().map(|t| Block::text(t)).collect(),
            Duration::hours(1),
            SaveGuard::default(),
        )
        .unwrap()
    };
    let journal_file = root.join(".journal").join(format!("{DEVICE}.jsonl"));

    // Off: nothing is written.
    journal::set_device(None);
    save(&["꺼져 있을 때 쓴 글"]);
    assert!(!root.join(".journal").exists());

    journal::set_device(Some(DEVICE.into()));
    save(&[SECRET]);
    save(&[SECRET, "둘째 문단."]);
    // The same text again changes nothing on disk, so no line.
    save(&[SECRET, "둘째 문단."]);
    let current = doc::load(&root, &id).unwrap();
    snapshot::create(&root, &current, "manual", "보관 이름").unwrap();

    let source = dir.path().join("초고.txt");
    fs::write(&source, format!("제1화\n{IMPORTED}\n\n제2화\n둘째 화.\n")).unwrap();
    let opts = ImportOptions::default();
    let preview = import::preview(std::slice::from_ref(&source), &opts);
    let spec = CommitSpec {
        part_id: None,
        after: None,
        picks: preview
            .chapters
            .iter()
            .map(|c| Pick {
                index: c.index,
                title: c.title.clone(),
            })
            .collect(),
        leave_notes: false,
        status: None,
        page_setup: false,
    };
    let done = import::commit(&root, std::slice::from_ref(&source), &opts, &spec).unwrap();

    let text = fs::read_to_string(&journal_file).unwrap();
    // The first save also kept an automatic record of the text before it.
    assert_eq!(
        kinds(&text),
        ["snapshot", "save", "save", "snapshot", "import"]
    );
    let lines: Vec<serde_json::Value> = text
        .lines()
        .skip(1)
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let first = SECRET.chars().count() as u64;
    assert_eq!(lines[0]["chars"], first);
    // Added and removed are how much longer or shorter the text got.
    let before = "꺼져 있을 때 쓴 글".chars().count() as u64;
    assert_eq!(lines[0]["added"], first - before);
    assert_eq!(lines[1]["added"], "둘째 문단.".chars().count() as u64);
    assert_eq!(lines[1]["removed"], 0);
    let written = writer_core::markup::write_body(&current.body);
    assert_eq!(
        lines[1]["body"].as_str().unwrap(),
        journal::fingerprint(written.as_bytes())
    );
    assert_eq!(lines[2]["snapshotKind"], "manual");
    assert_eq!(lines[3]["file"], "초고.txt");
    assert_eq!(
        lines[3]["fileHash"].as_str().unwrap(),
        journal::fingerprint(&fs::read(&source).unwrap())
    );
    assert_eq!(lines[3]["docs"].as_array().unwrap().len(), done.docs.len());

    // No manuscript text, record name or folder anywhere in the journal.
    for words in [SECRET, "둘째 문단", IMPORTED, "보관 이름", "꺼져 있을 때"] {
        assert!(!text.contains(words), "{words} leaked into the journal");
    }
    assert!(!text.contains(&*dir.path().to_string_lossy()));

    assert!(journal::verify(&root).unwrap().ok);
    let sum = journal::summary(&root, Some(DEVICE)).unwrap();
    assert_eq!((sum.saves, sum.imports, sum.this_device), (2, 1, 5));

    // Off again: nothing more.
    journal::set_device(None);
    save(&["다시 꺼짐"]);
    assert_eq!(fs::read_to_string(&journal_file).unwrap(), text);
}
