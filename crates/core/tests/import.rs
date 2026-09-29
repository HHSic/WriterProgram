use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use writer_core::doc;
use writer_core::import::{self, CommitSpec, ImportOptions, LineMode, Pick, SplitRule};
use writer_core::markup::{Block, Inline, Mark, plain_text};
use writer_core::notes;
use writer_core::project::{self, NewProject, ProjectKind};
use zip::write::SimpleFileOptions;

fn new_project(dir: &Path) -> PathBuf {
    project::create(&NewProject {
        parent: dir.to_string_lossy().into_owned(),
        title: "가져오기 시험".into(),
        kind: ProjectKind::Webnovel,
        per_doc_goal: None,
        count_spaces: true,
        first_chapter: false,
    })
    .unwrap()
}

const NS: &str = r#"xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main""#;

fn p(text: &str) -> String {
    format!(r#"<w:p><w:r><w:t xml:space="preserve">{text}</w:t></w:r></w:p>"#)
}

fn heading(text: &str) -> String {
    format!(r#"<w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>{text}</w:t></w:r></w:p>"#)
}

fn docx(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    let file = fs::File::create(&path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let opts = SimpleFileOptions::default();
    zip.start_file("word/document.xml", opts).unwrap();
    write!(zip, r#"<?xml version="1.0" encoding="UTF-8"?><w:document {NS}><w:body>{body}</w:body></w:document>"#).unwrap();
    zip.start_file("word/styles.xml", opts).unwrap();
    write!(zip, r#"<?xml version="1.0" encoding="UTF-8"?><w:styles {NS}><w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/></w:style></w:styles>"#).unwrap();
    zip.finish().unwrap();
    path
}

#[test]
fn docx_headings_marks_and_what_is_left_out() {
    let dir = tempfile::tempdir().unwrap();
    let body = [
        heading("1화 문 닫는 시간"),
        p("　　서하는 문을 닫았다."),
        r#"<w:p><w:r><w:rPr><w:b/></w:rPr><w:t>굵게</w:t></w:r><w:r><w:t>&amp; 보통</w:t></w:r></w:p>"#.into(),
        "<w:tbl><w:tr><w:tc>".to_string() + &p("표 안의 글") + "</w:tc></w:tr></w:tbl>",
        r#"<w:p><w:r><w:drawing><w:inline/></w:drawing></w:r></w:p>"#.into(),
        p("***"),
        p("장면 뒤 글."),
        r#"<w:p><w:r><w:t>각주 있음</w:t></w:r><w:r><w:footnoteReference w:id="2"/></w:r></w:p>"#.into(),
        heading("2화 빗소리가 들리는 밤"),
        p("둘째 회차."),
    ]
    .concat();
    let path = docx(dir.path(), "a.docx", &body);
    let preview = import::preview(std::slice::from_ref(&path), &ImportOptions::default());
    assert!(
        preview.files[0].error.is_none(),
        "{:?}",
        preview.files[0].error
    );
    assert_eq!(preview.files[0].rule, "제목 서식");
    let titles: Vec<_> = preview.chapters.iter().map(|c| c.title.as_str()).collect();
    assert_eq!(titles, ["문 닫는 시간", "빗소리가 들리는 밤"]);
    assert_eq!(preview.skipped.tables, 1);
    assert_eq!(preview.skipped.images, 1);
    assert_eq!(preview.skipped.footnotes, 1);
    assert_eq!(preview.chapters[0].skipped.tables, 1);
    assert_eq!(preview.chapters[1].skipped.tables, 0);

    let root = new_project(dir.path());
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
        leave_notes: true,
        status: None,
    };
    let done = import::commit(&root, &[path], &ImportOptions::default(), &spec).unwrap();
    assert_eq!(done.docs.len(), 2);
    assert_eq!(
        done.notes, 2,
        "one at the table and picture, one at the footnote"
    );
    let overview = project::overview(&root).unwrap();
    let ids: Vec<_> = overview
        .parts
        .iter()
        .flat_map(|p| p.docs.iter().map(|d| d.id.clone()))
        .collect();
    assert_eq!(ids, done.docs);

    let first = doc::load(&root, &done.docs[0]).unwrap();
    assert_eq!(first.meta.title, "문 닫는 시간");
    let text = plain_text(&first.body);
    assert!(text.starts_with("서하는 문을 닫았다."), "{text}");
    assert!(!text.contains("표 안의 글"));
    assert!(first.body.iter().any(|b| matches!(b, Block::SceneBreak {})));
    let bold = first.body.iter().any(|b| matches!(b, Block::Paragraph { content, .. }
        if content.iter().any(|i| matches!(i, Inline::Text { text, marks } if text == "굵게" && marks.contains(&Mark::Bold {})))));
    assert!(bold);
    assert!(text.contains("& 보통"));
    let all = notes::list(&root).unwrap();
    assert_eq!(all.len(), 2);
    assert!(
        all.iter()
            .any(|n| n.text.contains("표 1개") && n.text.contains("그림 1개"))
    );
}

#[test]
fn txt_is_cut_at_episode_lines() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("연재.txt");
    fs::write(
        &path,
        "프롤로그 글\n\n제1화 문 닫는 시간\n서하는 문을 닫았다.\n\n제2화. 빗소리\n둘째.\n",
    )
    .unwrap();
    let preview = import::preview(&[path], &ImportOptions::default());
    assert_eq!(preview.files[0].rule, "제N화");
    let titles: Vec<_> = preview.chapters.iter().map(|c| c.title.as_str()).collect();
    assert_eq!(titles, ["연재", "문 닫는 시간", "빗소리"]);
    assert_eq!(preview.chapters[1].paragraphs, 1);
}

#[test]
fn one_file_one_chapter_and_bad_pattern() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("한 편.txt");
    fs::write(&path, "가\n나\n").unwrap();
    let opts = ImportOptions::default();
    let preview = import::preview(std::slice::from_ref(&path), &opts);
    assert_eq!(preview.chapters.len(), 1);
    assert_eq!(preview.chapters[0].title, "한 편");

    let bad = ImportOptions {
        rule: SplitRule::Regex,
        pattern: "(".into(),
        ..Default::default()
    };
    assert!(
        import::preview(std::slice::from_ref(&path), &bad).files[0]
            .error
            .is_some()
    );

    let custom = ImportOptions {
        rule: SplitRule::Regex,
        pattern: "^나$".into(),
        line_mode: LineMode::Line,
        ..Default::default()
    };
    let preview = import::preview(&[path], &custom);
    assert_eq!(preview.chapters.len(), 2);
}

#[test]
fn unsupported_files_say_why() {
    let dir = tempfile::tempdir().unwrap();
    let hwp = dir.path().join("a.hwp");
    let bad = dir.path().join("b.docx");
    fs::write(&hwp, b"x").unwrap();
    fs::write(&bad, b"not a zip").unwrap();
    let preview = import::preview(&[hwp, bad], &ImportOptions::default());
    assert!(
        preview.files[0]
            .error
            .as_deref()
            .unwrap()
            .contains("docx나 txt")
    );
    assert!(preview.files[1].error.is_some());
    assert!(preview.chapters.is_empty());
}

#[test]
fn commit_after_a_chapter_and_nothing_left_behind_on_error() {
    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path());
    let a = project::add_doc(
        &root,
        &project::NewDoc {
            title: "기존".into(),
            ..Default::default()
        },
    )
    .unwrap();
    let path = dir.path().join("x.txt");
    fs::write(&path, "1화 하나\n글\n2화 둘\n글\n").unwrap();
    let opts = ImportOptions::default();
    let picks = vec![
        Pick {
            index: 0,
            title: "하나".into(),
        },
        Pick {
            index: 1,
            title: "둘".into(),
        },
    ];
    let spec = CommitSpec {
        part_id: None,
        after: Some(a.clone()),
        picks,
        leave_notes: false,
        status: Some("stock".into()),
    };
    let done = import::commit(&root, std::slice::from_ref(&path), &opts, &spec).unwrap();
    let overview = project::overview(&root).unwrap();
    let ids: Vec<_> = overview.parts[0]
        .docs
        .iter()
        .map(|d| d.id.clone())
        .collect();
    assert_eq!(ids[0], a);
    assert_eq!(&ids[1..], &done.docs[..]);
    assert_eq!(
        doc::load(&root, &done.docs[0]).unwrap().meta.status,
        "stock"
    );

    let stale = CommitSpec {
        part_id: None,
        after: None,
        picks: vec![Pick {
            index: 9,
            title: "?".into(),
        }],
        leave_notes: false,
        status: None,
    };
    assert!(import::commit(&root, &[path], &opts, &stale).is_err());
    let files = fs::read_dir(root.join("manuscript")).unwrap().count();
    assert_eq!(files, 3);
}

#[test]
fn our_own_docx_export_comes_back_in_chapters() {
    use writer_core::export::{self, DocOptions, ExportItem, FileKind};
    use writer_core::format;

    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path());
    let mut items = Vec::new();
    for (i, title) in ["문 닫는 시간", "빗소리가 들리는 밤"].iter().enumerate() {
        let id = project::add_doc(
            &root,
            &project::NewDoc {
                title: (*title).into(),
                ..Default::default()
            },
        )
        .unwrap();
        let body = vec![
            Block::text(&format!("{}번째 회차의 첫 문단.", i + 1)),
            Block::SceneBreak {},
            Block::text("끝."),
        ];
        doc::save_body(
            &root,
            &id,
            body,
            chrono::Duration::minutes(10),
            Default::default(),
        )
        .unwrap();
        items.push(ExportItem {
            doc_id: id,
            heading: format!("{}화 {title}", i + 1),
            file_name: String::new(),
        });
    }
    let out = dir.path().join("out.docx");
    let opts = DocOptions {
        include_titles: true,
        scene_break: "◆".into(),
    };
    let fmt = format::builtin_presets()
        .into_iter()
        .next()
        .map(|p| p.2)
        .unwrap();
    let files =
        export::export_file(&root, &items, &opts, &fmt, FileKind::Docx, &out, false).unwrap();
    let preview = import::preview(&files, &ImportOptions::default());
    assert!(
        preview.files[0].error.is_none(),
        "{:?}",
        preview.files[0].error
    );
    let titles: Vec<_> = preview.chapters.iter().map(|c| c.title.as_str()).collect();
    assert_eq!(titles, ["문 닫는 시간", "빗소리가 들리는 밤"]);
    assert_eq!(preview.chapters[0].paragraphs, 3);
}
