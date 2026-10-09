//! Test the actual UI catalog; adding a choice must retain these constraints.
use writer_core::export::{DocInfo, DocOptions, ExportDoc};
use writer_core::import::{self, CommitSpec, ImportOptions, Pick, SplitRule};
use writer_core::markup::{Block, plain_text};
use writer_core::project::{self, NewProject, ProjectKind};
use writer_core::{doc, format, hwpx};

#[derive(serde::Deserialize)]
struct Group {
    #[serde(default)]
    unicode: bool,
    items: Vec<String>,
}

fn groups() -> Vec<Group> {
    serde_json::from_str(include_str!("../../../app/src/lib/text-emoticons.json")).unwrap()
}

fn catalog() -> Vec<String> {
    groups().into_iter().flat_map(|g| g.items).collect()
}

#[test]
fn basic_choices_stay_in_the_traditional_korean_character_repertoire() {
    // EUC_KR in encoding_rs uses the Windows-949 repertoire. This checks
    // character coverage, not font availability or Hancom's visual rendering.
    let combining = regex::Regex::new(r"\p{M}").unwrap();
    let items: Vec<_> = groups()
        .into_iter()
        .filter(|g| !g.unicode)
        .flat_map(|g| g.items)
        .collect();
    for text in &items {
        assert!(!combining.is_match(text), "combining sequence: {text}");
        assert!(
            text.chars().all(|c| c <= '\u{ffff}' && !c.is_control()),
            "non-text character: {text}"
        );
        let (encoded, _, errors) = encoding_rs::EUC_KR.encode(text);
        assert!(!errors, "outside Korean character repertoire: {text}");
        let (decoded, _, errors) = encoding_rs::EUC_KR.decode(&encoded);
        assert!(!errors);
        assert_eq!(decoded, *text);
    }
}

#[test]
fn unicode_choices_allow_combining_marks_without_hidden_controls_or_emoji_sequences() {
    let hidden = regex::Regex::new(r"[\p{C}\x{FE00}-\x{FE0F}]").unwrap();
    let items = catalog();
    let mut unique = std::collections::HashSet::new();
    for text in &items {
        assert!(
            !text.is_empty() && unique.insert(text),
            "empty or duplicate: {text}"
        );
        assert!(
            !hidden.is_match(text),
            "hidden control or variation selector: {text}"
        );
        assert!(
            text.chars().all(|c| c <= '\u{ffff}'),
            "supplementary character: {text}"
        );
    }
    assert!(
        items
            .iter()
            .any(|t| regex::Regex::new(r"\p{M}").unwrap().is_match(t))
    );
}

#[test]
fn every_offered_face_survives_hwpx_export_and_import() {
    let items = catalog();
    let blocks: Vec<_> = items.iter().map(|text| Block::text(text)).collect();
    let bytes = hwpx::hwpx_bytes(
        &[ExportDoc {
            heading: String::new(),
            blocks,
        }],
        &format::builtin("submission-a4").unwrap(),
        &DocOptions {
            include_titles: false,
            scene_break: "***".into(),
        },
        &DocInfo {
            title: "텍스트 이모티콘".into(),
            author: String::new(),
        },
    )
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("emoticons.hwpx");
    std::fs::write(&path, bytes).unwrap();
    let root = project::create(&NewProject {
        parent: dir.path().to_string_lossy().into_owned(),
        title: "이모티콘 시험".into(),
        kind: ProjectKind::Webnovel,
        per_doc_goal: None,
        count_spaces: true,
        first_chapter: false,
        platform: None,
    })
    .unwrap();
    let opts = ImportOptions {
        rule: SplitRule::File,
        ..Default::default()
    };
    let done = import::commit(
        &root,
        &[path],
        &opts,
        &CommitSpec {
            part_id: None,
            after: None,
            picks: vec![Pick {
                index: 0,
                title: "표정".into(),
            }],
            leave_notes: false,
            status: None,
            page_setup: false,
        },
    )
    .unwrap();
    assert_eq!(done.docs.len(), 1);
    assert_eq!(
        plain_text(&doc::load(&root, &done.docs[0]).unwrap().body),
        items.join("\n")
    );
}
