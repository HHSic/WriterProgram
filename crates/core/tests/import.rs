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
        platform: None,
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
        import_notes: true,
        status: None,
        page_setup: false,
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
            .contains("손상됨")
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
        import_notes: true,
        status: Some("stock".into()),
        page_setup: false,
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
        import_notes: true,
        status: None,
        page_setup: false,
    };
    assert!(import::commit(&root, &[path], &opts, &stale).is_err());
    let files = fs::read_dir(root.join("manuscript")).unwrap().count();
    assert_eq!(files, 3);
}

#[test]
fn our_own_docx_export_comes_back_in_chapters() {
    own_export_comes_back(writer_core::export::FileKind::Docx, "out.docx");
}

#[test]
fn our_own_hwpx_export_comes_back_in_chapters() {
    own_export_comes_back(writer_core::export::FileKind::Hwpx, "out.hwpx");
}

fn own_export_comes_back(kind: writer_core::export::FileKind, name: &str) {
    use writer_core::export::{self, DocOptions, ExportItem};
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
    let out = dir.path().join(name);
    let opts = DocOptions {
        include_titles: true,
        scene_break: "◆".into(),
    };
    let fmt = format::builtin_presets()
        .into_iter()
        .next()
        .map(|p| p.2)
        .unwrap();
    let files = export::export_file(&root, &items, &opts, &fmt, kind, &out, false).unwrap();
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

#[test]
fn hwpx_pages_can_become_the_format() {
    use writer_core::export::{self, DocOptions, ExportItem, FileKind};
    use writer_core::format::{self, HeadAlign, HeadContent, RunningFoot, RunningHead};

    let dir = tempfile::tempdir().unwrap();
    let from = new_project(&dir.path().join("a"));
    let id = project::add_doc(
        &from,
        &project::NewDoc {
            title: "문 닫는 시간".into(),
            ..Default::default()
        },
    )
    .unwrap();
    doc::save_body(
        &from,
        &id,
        vec![Block::text("첫 문단.")],
        chrono::Duration::minutes(10),
        Default::default(),
    )
    .unwrap();
    let mut fmt = format::builtin("book-shinguk").unwrap();
    fmt.header = RunningHead {
        content: HeadContent::Custom,
        text: "달빛 서점".into(),
        align: HeadAlign::Outside,
        skip_chapter_first: true,
    };
    fmt.page_number_align = HeadAlign::Center;
    fmt.footer = RunningFoot {
        text: "투고용 · 무단 배포 금지".into(),
        align: HeadAlign::Right,
    };
    let out = dir.path().join("책.hwpx");
    let items = vec![ExportItem {
        doc_id: id,
        heading: "1화 문 닫는 시간".into(),
        file_name: String::new(),
    }];
    let opts = DocOptions {
        include_titles: true,
        scene_break: "◆".into(),
    };
    let files =
        export::export_file(&from, &items, &opts, &fmt, FileKind::Hwpx, &out, false).unwrap();

    let preview = import::preview(&files, &ImportOptions::default());
    let page = preview.files[0].page.clone().expect("page setup");
    assert_eq!(page.paper.kind, "shinguk");
    assert_eq!(page.header.content, HeadContent::Custom);
    assert_eq!(page.header.text, "달빛 서점");
    assert_eq!(page.header.align, HeadAlign::Outside);
    assert_eq!(page.page_numbers, Some(HeadAlign::Center));
    assert_eq!(page.footer.text, "투고용 · 무단 배포 금지");
    assert_eq!(page.footer.align, HeadAlign::Right);
    assert!(
        page.summary
            .starts_with("용지 신국판, 여백 위 22 · 아래 20 · 안쪽 22 · 바깥쪽 20mm."),
        "{}",
        page.summary
    );

    // Into a web novel, which had no paper: it takes the book's pages.
    let root = new_project(&dir.path().join("b"));
    let spec = CommitSpec {
        part_id: None,
        after: None,
        picks: vec![Pick {
            index: 0,
            title: "문 닫는 시간".into(),
        }],
        leave_notes: false,
        import_notes: true,
        status: None,
        page_setup: true,
    };
    let done = import::commit(&root, &files, &ImportOptions::default(), &spec).unwrap();
    assert!(done.format);
    let got = project::load(&root).unwrap().manuscript_format();
    assert_eq!(got.paper, fmt.paper);
    assert_eq!(got.margins, fmt.margins);
    assert_eq!(got.header, fmt.header);
    assert_eq!(got.footer, fmt.footer);
    assert!(got.page_numbers);
    // The type stays the project's own.
    assert_eq!(got.font, "dotum");
}

#[test]
fn hwpx_keeps_supported_units_literal_spaces_tabs_empty_lines_and_paragraph_shape() {
    use writer_core::markup::{Align, ColorAttrs, ParaAttrs};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("문단.hwpx");
    let mut zip = zip::ZipWriter::new(fs::File::create(&path).unwrap());
    zip.start_file("Contents/header.xml", SimpleFileOptions::default())
        .unwrap();
    zip.write_all(br##"<head><charPr id="0" height="1000" textColor="#2365B8"/><paraPr id="0"><align horizontal="CENTER"/><breakSetting pageBreakBefore="1"/><switch><case required-namespace="http://www.hancom.co.kr/hwpml/2016/HwpUnitChar"><margin><intent value="2000"/><left value="3000"/><right value="1000"/></margin></case><default><margin><intent value="-12000"/><left value="20000"/></margin></default></switch></paraPr><paraPr id="1"><switch><case required-namespace="unsupported"><margin><intent value="99999"/></margin></case><default><margin><intent value="-4000"/><left value="4000"/><right value="2000"/></margin></default></switch></paraPr><paraPr id="2"/></head>"##).unwrap();
    zip.start_file("Contents/section0.xml", SimpleFileOptions::default())
        .unwrap();
    zip.write_all("<sec><p paraPrIDRef=\"0\"><run charPrIDRef=\"0\"><t>  <tab/>가운데</t></run></p><p paraPrIDRef=\"1\"><run charPrIDRef=\"0\"><t>내어쓰기</t></run></p><p paraPrIDRef=\"2\"><run charPrIDRef=\"0\"><t/></run></p><p paraPrIDRef=\"2\"><run charPrIDRef=\"0\"><t>들여쓰기 없음</t></run></p></sec>".as_bytes()).unwrap();
    zip.finish().unwrap();
    let options = ImportOptions {
        rule: SplitRule::File,
        ..Default::default()
    };
    let root = new_project(dir.path());
    let result = import::commit(
        &root,
        &[path],
        &options,
        &CommitSpec {
            part_id: None,
            after: None,
            picks: vec![Pick {
                index: 0,
                title: "문단".into(),
            }],
            leave_notes: false,
            import_notes: true,
            status: None,
            page_setup: false,
        },
    )
    .unwrap();
    let saved = doc::load(&root, &result.docs[0]).unwrap();
    assert_eq!(saved.body.len(), 4);
    assert_eq!(
        saved.body[0].attrs(),
        ParaAttrs {
            left: 3,
            right: 1,
            indent: Some(2),
            align: Some(Align::Center),
            page_break_before: true,
            heading: None
        }
    );
    assert_eq!(saved.body[0].lines(), ["  \t가운데"]);
    assert_eq!(saved.body[1].attrs().indent, Some(-2));
    assert_eq!(
        (saved.body[1].attrs().left, saved.body[1].attrs().right),
        (2, 1)
    );
    assert_eq!(saved.body[3].attrs().indent, Some(0));
    assert!(
        matches!(&saved.body[0], Block::Paragraph { content, .. } if matches!(&content[0], Inline::Text { marks, .. } if marks.contains(&Mark::TextColor { attrs: ColorAttrs { color: "#2365B8".into() } })))
    );
}

#[test]
fn colour_alignment_and_explicit_page_break_round_trip_through_formatted_exports() {
    use writer_core::export::{self, DocOptions, ExportItem, FileKind};
    use writer_core::markup::{Align, ColorAttrs, ParaAttrs};
    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path());
    let id = project::add_doc(
        &root,
        &project::NewDoc {
            title: "서식".into(),
            ..Default::default()
        },
    )
    .unwrap();
    doc::save_body(
        &root,
        &id,
        vec![Block::Paragraph {
            attrs: ParaAttrs {
                heading: None,
                align: Some(Align::Center),
                page_break_before: true,
                indent: Some(-2),
                left: 3,
                right: 1,
            },
            content: vec![Inline::Text {
                text: "색과 문단".into(),
                marks: vec![
                    Mark::Bold {},
                    Mark::TextColor {
                        attrs: ColorAttrs {
                            color: "#2365B8".into(),
                        },
                    },
                ],
            }],
        }],
        chrono::Duration::minutes(10),
        Default::default(),
    )
    .unwrap();
    let format = writer_core::format::builtin("submission-a4").unwrap();
    let items = [ExportItem {
        doc_id: id,
        heading: String::new(),
        file_name: String::new(),
    }];
    let opts = DocOptions {
        include_titles: false,
        scene_break: "◆".into(),
    };
    let options = ImportOptions {
        rule: SplitRule::File,
        ..Default::default()
    };
    for (kind, name) in [(FileKind::Hwpx, "서식.hwpx"), (FileKind::Docx, "서식.docx")] {
        let files = export::export_file(
            &root,
            &items,
            &opts,
            &format,
            kind,
            &dir.path().join(name),
            false,
        )
        .unwrap();
        let result = import::commit(
            &root,
            &files,
            &options,
            &CommitSpec {
                part_id: None,
                after: None,
                picks: vec![Pick {
                    index: 0,
                    title: name.into(),
                }],
                leave_notes: false,
                import_notes: true,
                status: None,
                page_setup: false,
            },
        )
        .unwrap();
        let saved = doc::load(&root, &result.docs[0]).unwrap();
        let attrs = saved.body[0].attrs();
        assert_eq!(attrs.align, Some(Align::Center));
        assert!(attrs.page_break_before);
        if matches!(kind, FileKind::Hwpx) {
            assert_eq!((attrs.left, attrs.right, attrs.indent), (3, 1, Some(-2)));
        }
        assert!(
            matches!(&saved.body[0], Block::Paragraph { content, .. } if matches!(&content[0], Inline::Text { marks, .. } if marks.contains(&Mark::TextColor { attrs: ColorAttrs { color: "#2365B8".into() } }) && marks.contains(&Mark::Bold {})))
        );
    }
}

#[test]
fn embedded_docx_comments_survive_partial_import_and_can_be_excluded() {
    let dir = tempfile::tempdir().unwrap();
    let body = heading("첫 장")
        + r#"<w:p><w:commentRangeStart w:id="0"/><w:r><w:t>원고 😀 인용</w:t></w:r><w:commentRangeEnd w:id="0"/></w:p>"#
        + &heading("둘째 장")
        + &p("둘째 본문");
    let path = docx(dir.path(), "주석.docx", &body);
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();
    let mut zip = zip::ZipWriter::new_append(file).unwrap();
    zip.start_file("word/comments.xml", SimpleFileOptions::default())
        .unwrap();
    write!(zip,r#"<w:comments {NS}><w:comment w:id="0" w:author="교열자" w:date="2026-10-10T00:00:00Z">{}</w:comment><w:comment w:id="1">{}</w:comment></w:comments>"#,p("메모 첫 줄")+&p("메모 다음 줄"),p("범위 없는 메모")).unwrap();
    zip.finish().unwrap();
    let opts = ImportOptions::default();
    let preview = import::preview(std::slice::from_ref(&path), &opts);
    assert_eq!(preview.files[0].annotation_count, 2);
    assert_eq!(preview.chapters.len(), 2);
    let root = new_project(dir.path());
    let mut spec = CommitSpec {
        part_id: None,
        after: None,
        picks: vec![Pick {
            index: 1,
            title: "둘째".into(),
        }],
        leave_notes: false,
        import_notes: true,
        status: None,
        page_setup: false,
    };
    let imported = import::commit(&root, std::slice::from_ref(&path), &opts, &spec).unwrap();
    assert_eq!(imported.notes, 2);
    let all = notes::list(&root).unwrap();
    assert!(all.iter().all(|n| n.target == imported.docs[0]));
    let quoted = all.iter().find(|n| !n.quote.is_empty()).unwrap();
    assert_eq!(quoted.quote, "원고 😀 인용");
    assert!(
        quoted.text.contains("교열자")
            && quoted.text.contains("2026-10-10")
            && quoted.text.contains("메모 다음 줄")
    );
    spec.import_notes = false;
    let result = import::commit(&root, &[path], &opts, &spec).unwrap();
    assert_eq!(result.notes, 0);
    assert_eq!(notes::list(&root).unwrap().len(), 2);
}

#[test]
fn embedded_hwpx_memo_text_stays_out_of_manuscript_and_goes_to_notes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("메모.hwpx");
    let file = fs::File::create(&path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    zip.start_file("Contents/header.xml", SimpleFileOptions::default())
        .unwrap();
    write!(zip, "<hh:head xmlns:hh=\"urn:h\"/>").unwrap();
    zip.start_file("Contents/section0.xml", SimpleFileOptions::default())
        .unwrap();
    write!(zip,r#"<hs:sec xmlns:hs="urn:s" xmlns:hp="urn:p"><hp:p><hp:run><hp:t>원고 본문</hp:t></hp:run></hp:p><hp:memogroup><hp:memo id="m" author="한글 작성자" date="2026-10-10"><hp:paraList><hp:p><hp:run><hp:t>한글 메모 본문</hp:t></hp:run></hp:p></hp:paraList></hp:memo></hp:memogroup></hs:sec>"#).unwrap();
    zip.finish().unwrap();
    let preview = import::preview(std::slice::from_ref(&path), &ImportOptions::default());
    assert_eq!(preview.files[0].annotation_count, 1);
    assert_eq!(preview.chapters[0].chars, 5);
    let root = new_project(dir.path());
    let result = import::commit(
        &root,
        &[path],
        &ImportOptions::default(),
        &CommitSpec {
            part_id: None,
            after: None,
            picks: vec![Pick {
                index: 0,
                title: "본문".into(),
            }],
            leave_notes: false,
            import_notes: true,
            status: None,
            page_setup: false,
        },
    )
    .unwrap();
    assert_eq!(
        plain_text(&doc::load(&root, &result.docs[0]).unwrap().body),
        "원고 본문"
    );
    let notes = notes::list(&root).unwrap();
    assert_eq!(notes.len(), 1);
    assert!(notes[0].text.contains("한글 메모 본문") && notes[0].text.contains("한글 작성자"));
}

#[test]
fn body_heading_styles_survive_docx_and_hwpx_round_trips_without_new_chapters() {
    use writer_core::{
        export::{self, DocOptions, ExportItem, FileKind},
        markup::ParaAttrs,
    };
    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path());
    let id = project::add_doc(
        &root,
        &project::NewDoc {
            title: "제목 서식".into(),
            ..Default::default()
        },
    )
    .unwrap();
    let body = (1..=3)
        .map(|n| Block::Paragraph {
            attrs: ParaAttrs {
                heading: Some(n),
                ..Default::default()
            },
            content: vec![Inline::Text {
                text: format!("본문 제목 {n}"),
                marks: vec![],
            }],
        })
        .collect();
    doc::save_body(
        &root,
        &id,
        body,
        chrono::Duration::minutes(10),
        Default::default(),
    )
    .unwrap();
    for kind in [FileKind::Docx, FileKind::Hwpx] {
        let path = dir.path().join(if matches!(kind, FileKind::Docx) {
            "제목.docx"
        } else {
            "제목.hwpx"
        });
        let format = project::load(&root).unwrap().manuscript_format();
        export::export_file(
            &root,
            &[ExportItem {
                doc_id: id.clone(),
                heading: "장 제목".into(),
                file_name: String::new(),
            }],
            &DocOptions {
                include_titles: false,
                scene_break: "◆".into(),
            },
            &format,
            kind,
            &path,
            false,
        )
        .unwrap();
        let preview = import::preview(std::slice::from_ref(&path), &ImportOptions::default());
        assert_eq!(preview.chapters.len(), 1, "{:?}", preview.files);
        let result = import::commit(
            &root,
            &[path],
            &ImportOptions::default(),
            &CommitSpec {
                part_id: None,
                after: None,
                picks: vec![Pick {
                    index: 0,
                    title: "서식".into(),
                }],
                leave_notes: false,
                import_notes: true,
                status: None,
                page_setup: false,
            },
        )
        .unwrap();
        let saved = doc::load(&root, &result.docs[0]).unwrap();
        assert_eq!(
            saved
                .body
                .iter()
                .map(|b| b.attrs().heading)
                .collect::<Vec<_>>(),
            [Some(1), Some(2), Some(3)]
        );
    }
}
