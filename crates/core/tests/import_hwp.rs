//! Real Hancom files plus targeted CFB/record fixtures for import safety.

use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

use flate2::{Compression, write::DeflateEncoder};
use writer_core::doc;
use writer_core::import::{self, CommitSpec, ImportOptions, Pick, SplitRule};
use writer_core::markup::{Inline, Mark, plain_text};
use writer_core::project::{self, NewProject, ProjectKind};

fn record(tag: u32, level: u32, data: &[u8]) -> Vec<u8> {
    let length = data.len() as u32;
    let mut bytes = (tag | level << 10 | length.min(0xfff) << 20)
        .to_le_bytes()
        .to_vec();
    if length >= 0xfff {
        bytes.extend_from_slice(&length.to_le_bytes());
    }
    bytes.extend_from_slice(data);
    bytes
}

fn utf16(text: &str) -> Vec<u8> {
    text.encode_utf16().flat_map(u16::to_le_bytes).collect()
}
fn wide(text: &str) -> Vec<u8> {
    let mut data = (text.encode_utf16().count() as u16).to_le_bytes().to_vec();
    data.extend(utf16(text));
    data
}
fn info() -> Vec<u8> {
    let mut bytes = Vec::new();
    for flags in [0u32, 2, 1, (1 << 2) | (1 << 18) | (1 << 21)] {
        let mut data = vec![0; 68];
        data[42..46].copy_from_slice(&1000u32.to_le_bytes());
        data[46..50].copy_from_slice(&flags.to_le_bytes());
        bytes.extend(record(21, 0, &data));
    }
    bytes.extend(record(25, 0, &[0; 42]));
    for name in ["본문", "장 제목", "차례 1"] {
        let mut data = wide(name);
        data.extend(wide(""));
        data.extend([0; 10]);
        bytes.extend(record(26, 0, &data));
    }
    bytes
}
fn paragraph(text: &str, style: u8, runs: &[(u32, u32)], level: u32) -> Vec<u8> {
    let units: Vec<u16> = format!("{text}\r")
        .encode_utf16()
        .flat_map(|u| {
            if u == 9 {
                vec![9, 0, 0, 0, 0, 0, 0, 9]
            } else {
                vec![u]
            }
        })
        .collect();
    let mut header = vec![0; 22];
    header[..4].copy_from_slice(&(units.len() as u32).to_le_bytes());
    header[10] = style;
    header[12..14].copy_from_slice(&(runs.len() as u16).to_le_bytes());
    let mut bytes = record(66, level, &header);
    bytes.extend(record(
        67,
        level + 1,
        &units
            .iter()
            .flat_map(|u| u.to_le_bytes())
            .collect::<Vec<_>>(),
    ));
    let data = runs
        .iter()
        .flat_map(|(position, shape)| [position.to_le_bytes(), shape.to_le_bytes()].concat())
        .collect::<Vec<_>>();
    bytes.extend(record(68, level + 1, &data));
    bytes
}
fn deflate(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(bytes).unwrap();
    encoder.finish().unwrap()
}
fn write_hwp(path: &Path, flags: u32, sections: &[Vec<u8>]) {
    write_hwp_data(path, flags, sections, &info());
}
fn write_hwp_data(path: &Path, flags: u32, sections: &[Vec<u8>], data: &[u8]) {
    let mut file = cfb::CompoundFile::create(Cursor::new(Vec::new())).unwrap();
    let mut header = vec![0; 256];
    header[..17].copy_from_slice(b"HWP Document File");
    header[32..36].copy_from_slice(&[1, 0, 1, 5]);
    header[36..40].copy_from_slice(&flags.to_le_bytes());
    file.create_stream("/FileHeader")
        .unwrap()
        .write_all(&header)
        .unwrap();
    file.create_stream("/DocInfo")
        .unwrap()
        .write_all(&if flags & 1 != 0 {
            deflate(data)
        } else {
            data.to_vec()
        })
        .unwrap();
    file.create_storage("/BodyText").unwrap();
    for (index, section) in sections.iter().enumerate() {
        file.create_stream(format!("/BodyText/Section{index}"))
            .unwrap()
            .write_all(&if flags & 1 != 0 {
                deflate(section)
            } else {
                section.clone()
            })
            .unwrap();
    }
    // This preview is intentionally different: it must never be the import source.
    file.create_stream("/PrvText")
        .unwrap()
        .write_all(&utf16("잘린 미리보기만 읽으면 안 됨"))
        .unwrap();
    fs::write(path, file.into_inner().into_inner()).unwrap();
}
fn root(dir: &Path) -> PathBuf {
    project::create(&NewProject {
        parent: dir.to_string_lossy().into(),
        title: "HWP 가져오기".into(),
        kind: ProjectKind::Webnovel,
        per_doc_goal: None,
        count_spaces: true,
        first_chapter: false,
        platform: None,
    })
    .unwrap()
}
fn import_text(path: &Path, root: &Path, options: &ImportOptions) -> (String, Vec<Mark>) {
    let preview = import::preview(&[path.to_path_buf()], options);
    assert!(
        preview.files[0].error.is_none(),
        "{:?}",
        preview.files[0].error
    );
    let spec = CommitSpec {
        part_id: None,
        after: None,
        picks: preview
            .chapters
            .iter()
            .map(|chapter| Pick {
                index: chapter.index,
                title: chapter.title.clone(),
            })
            .collect(),
        leave_notes: true,
        import_notes: true,
        status: None,
        page_setup: false,
    };
    let committed = import::commit(root, &[path.to_path_buf()], options, &spec).unwrap();
    let mut text = Vec::new();
    let mut marks = Vec::new();
    for id in committed.docs {
        let doc = doc::load(root, &id).unwrap();
        text.push(plain_text(&doc.body));
        for block in &doc.body {
            if let writer_core::markup::Block::Paragraph { content, .. } = block {
                for inline in content {
                    if let Inline::Text { marks: found, .. } = inline {
                        marks.extend(found.clone());
                    }
                }
            }
        }
    }
    (text.join("\n"), marks)
}

#[test]
fn real_hancom_file_reads_full_body_and_leaves_original_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("한글 원고.HWP");
    let original = include_bytes!("fixtures/hwp/hello_world.hwp");
    fs::write(&path, original).unwrap();
    let (text, _) = import_text(&path, &root(dir.path()), &ImportOptions::default());
    assert_eq!(text.trim(), "Hello World!");
    assert_eq!(fs::read(&path).unwrap(), original);
}

#[test]
fn real_shape_text_never_leaks_into_main_manuscript() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/hwp/draw_text.hwp");
    let dir = tempfile::tempdir().unwrap();
    let preview = import::preview(std::slice::from_ref(&path), &ImportOptions::default());
    assert!(
        preview.files[0].error.is_none(),
        "{:?}",
        preview.files[0].error
    );
    assert!(preview.skipped.images > 0);
    let (text, _) = import_text(&path, &root(dir.path()), &ImportOptions::default());
    assert!(text.trim().is_empty(), "shape text leaked: {text}");
}

#[test]
fn compressed_and_uncompressed_sections_preserve_unicode_and_marks() {
    let options = ImportOptions {
        rule: SplitRule::File,
        ..Default::default()
    };
    for flags in [0, 1] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("원고.hwp");
        let text = "한글 ᄒᆞᆫ글 (ง •̀_•́)ง 👩🏽‍💻𠮷";
        let first = paragraph(text, 0, &[(0, 1), (2, 2), (5, 3)], 0);
        let long = "끝까지 읽는 긴 원고.".repeat(3000);
        write_hwp(&path, flags, &[first, paragraph(&long, 0, &[(0, 0)], 0)]);
        let (imported, marks) = import_text(&path, &root(dir.path()), &options);
        assert!(imported.starts_with(text), "{imported}");
        assert!(imported.ends_with(&long));
        assert!(!imported.contains("잘린 미리보기"));
        for mark in [
            Mark::Bold {},
            Mark::Italic {},
            Mark::Underline {},
            Mark::Strike {},
            Mark::Dot {},
        ] {
            assert!(marks.contains(&mark));
        }
    }
}

#[test]
fn headings_split_chapters_and_nested_content_is_reported_and_omitted() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("회차.hwp");
    let mut section = paragraph("목차에서 빠지는 줄", 2, &[(0, 0)], 0);
    section.extend(paragraph("첫 장", 1, &[(0, 0)], 0));
    section.extend(paragraph("본문 앞", 0, &[(0, 0)], 0));
    for (id, text) in [
        (b"tbl ", "표 안의 내용"),
        (b"gso ", "그림 안의 내용"),
        (b"fn  ", "각주 내용"),
        (b"head", "머리말 내용"),
    ] {
        section.extend(record(71, 1, &u32::from_be_bytes(*id).to_le_bytes()));
        section.extend(paragraph(text, 0, &[(0, 0)], 2));
    }
    section.extend(paragraph("본문 뒤", 0, &[(0, 0)], 0));
    section.extend(paragraph("둘째 장", 1, &[(0, 0)], 0));
    section.extend(paragraph("둘째 본문", 0, &[(0, 0)], 0));
    write_hwp(&path, 1, &[section]);
    let preview = import::preview(std::slice::from_ref(&path), &ImportOptions::default());
    assert!(
        preview.files[0].error.is_none(),
        "{:?}",
        preview.files[0].error
    );
    assert_eq!(
        preview
            .chapters
            .iter()
            .map(|c| c.title.as_str())
            .collect::<Vec<_>>(),
        ["첫 장", "둘째 장"]
    );
    assert_eq!(
        (
            preview.skipped.tables,
            preview.skipped.images,
            preview.skipped.footnotes
        ),
        (1, 1, 1)
    );
    let (text, _) = import_text(&path, &root(dir.path()), &ImportOptions::default());
    for skipped in ["목차에서", "표 안", "그림 안", "각주 내용", "머리말 내용"] {
        assert!(!text.contains(skipped), "{text}");
    }
    assert!(text.contains("본문 앞") && text.contains("본문 뒤") && text.contains("둘째 본문"));
}

#[test]
fn password_distribution_and_hwp3_have_specific_errors() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("보호.hwp");
    for (flags, reason) in [(2, "암호"), (4, "배포용")] {
        write_hwp(&path, flags, &[paragraph("원고", 0, &[(0, 0)], 0)]);
        let preview = import::preview(std::slice::from_ref(&path), &ImportOptions::default());
        assert!(preview.files[0].error.as_deref().unwrap().contains(reason));
        assert!(preview.chapters.is_empty());
    }
    fs::write(&path, b"HWP Document File V3.00\x1a").unwrap();
    let preview = import::preview(&[path], &ImportOptions::default());
    assert!(
        preview.files[0]
            .error
            .as_deref()
            .unwrap()
            .contains("한글 97")
    );
}

#[test]
fn corrupt_records_and_invalid_utf16_never_import_partial_chapters() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("손상.hwp");
    let good = paragraph("앞부분은 정상", 0, &[(0, 0)], 0);
    for tail in [
        vec![0xff],
        record(67, 1, &[0x00, 0xd8]),
        record(68, 1, &[0]),
        record(67, 1, &utf16("\u{0002}")),
    ] {
        let mut data = good.clone();
        data.extend(tail);
        write_hwp(&path, 1, &[data]);
        let preview = import::preview(std::slice::from_ref(&path), &ImportOptions::default());
        assert!(preview.files[0].error.is_some());
        assert!(preview.chapters.is_empty());
    }
}

#[test]
fn truncated_deflate_and_missing_sections_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("중간 끊김.hwp");
    write_hwp(&path, 1, &[paragraph("본문", 0, &[(0, 0)], 0)]);
    let mut file = cfb::open_rw(&path).unwrap();
    let mut compressed = deflate(&paragraph("본문", 0, &[(0, 0)], 0));
    compressed.pop();
    file.create_stream("/BodyText/Section0")
        .unwrap()
        .write_all(&compressed)
        .unwrap();
    file.flush().unwrap();
    drop(file);
    let preview = import::preview(std::slice::from_ref(&path), &ImportOptions::default());
    assert!(preview.files[0].error.is_some());
    assert!(preview.chapters.is_empty());
    write_hwp(&path, 0, &[]);
    assert!(
        import::preview(&[path], &ImportOptions::default()).files[0]
            .error
            .is_some()
    );
}

#[test]
fn paragraph_dimensions_literal_indentation_colour_and_page_break_survive_save() {
    use writer_core::markup::{Align, Block};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("들여쓰기.hwp");
    let mut data = info();
    let mut shape = vec![0; 42];
    shape[0..4].copy_from_slice(&(3u32 << 2).to_le_bytes());
    shape[4..8].copy_from_slice(&6000i32.to_le_bytes());
    shape[8..12].copy_from_slice(&2000i32.to_le_bytes());
    shape[12..16].copy_from_slice(&(-4000i32).to_le_bytes());
    data.extend(record(25, 0, &shape));
    // A character shape with COLORREF (low byte red), base size 10 pt.
    let mut color = vec![0; 68];
    color[42..46].copy_from_slice(&1000u32.to_le_bytes());
    color[52..56].copy_from_slice(&0xB86523u32.to_le_bytes());
    data.extend(record(21, 0, &color));
    let mut shaped = paragraph("  \t가져온 문단", 0, &[(0, 4)], 0);
    shaped[12..14].copy_from_slice(&1u16.to_le_bytes());
    shaped[15] = 4; // PARA_HEADER's explicit page-break flag.
    let section = [
        shaped,
        paragraph("", 0, &[(0, 0)], 0),
        paragraph("다음", 0, &[(0, 0)], 0),
    ]
    .concat();
    write_hwp_data(&path, 1, &[section], &data);
    let options = ImportOptions {
        rule: SplitRule::File,
        ..Default::default()
    };
    let root = root(dir.path());
    let preview = import::preview(std::slice::from_ref(&path), &options);
    assert!(
        preview.files[0].error.is_none(),
        "{:?}",
        preview.files[0].error
    );
    let result = import::commit(
        &root,
        &[path],
        &options,
        &CommitSpec {
            part_id: None,
            after: None,
            picks: vec![Pick {
                index: 0,
                title: "들여쓰기".into(),
            }],
            leave_notes: false,
            import_notes: true,
            status: None,
            page_setup: false,
        },
    )
    .unwrap();
    let saved = doc::load(&root, &result.docs[0]).unwrap();
    assert_eq!(saved.body.len(), 3);
    let Block::Paragraph { attrs, content } = &saved.body[0] else {
        panic!()
    };
    assert_eq!((attrs.left, attrs.right, attrs.indent), (3, 1, Some(-2)));
    assert_eq!(attrs.align, Some(Align::Center));
    assert!(attrs.page_break_before);
    assert_eq!(writer_core::markup::inline_text(content), "  \t가져온 문단");
    assert!(
        matches!(&content[0], Inline::Text { marks, .. } if marks.iter().any(|m| matches!(m, Mark::TextColor { attrs } if attrs.color == "#2365B8")))
    );
    assert_eq!(saved.body[2].attrs().indent, Some(0));
}

#[test]
fn binary_memo_lists_are_imported_as_notes_without_entering_manuscript() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("메모.hwp");
    let mut section = paragraph("원고 본문", 0, &[(0, 0)], 0);
    section.extend(record(93, 1, &[0; 4]));
    section.extend(record(92, 1, &[0; 22]));
    section.extend(paragraph("이 문장을 다시 검토하세요 😀", 0, &[(0, 0)], 2));
    section.extend(paragraph("두 번째 메모 줄", 0, &[(0, 0)], 2));
    section.extend(record(93, 1, &[0; 4]));
    section.extend(paragraph("다른 메모", 0, &[(0, 0)], 2));
    write_hwp(&path, 1, &[section]);
    let preview = import::preview(std::slice::from_ref(&path), &ImportOptions::default());
    assert_eq!(preview.files[0].annotation_count, 2);
    assert_eq!(preview.chapters[0].chars, 5);
    let root = project::create(&NewProject {
        title: "메모 검증".into(),
        parent: dir.path().to_string_lossy().into_owned(),
        kind: ProjectKind::Webnovel,
        per_doc_goal: None,
        count_spaces: true,
        first_chapter: false,
        platform: None,
    })
    .unwrap();
    let result = import::commit(
        &root,
        &[path],
        &ImportOptions::default(),
        &CommitSpec {
            part_id: None,
            after: None,
            picks: vec![Pick {
                index: 0,
                title: "가져오기".into(),
            }],
            leave_notes: false,
            import_notes: true,
            status: None,
            page_setup: false,
        },
    )
    .unwrap();
    assert_eq!(result.notes, 2);
    let notes = writer_core::notes::list(&root).unwrap();
    assert!(notes.iter().all(|n| n.target == result.docs[0]));
    assert!(
        notes
            .iter()
            .any(|n| n.text.contains("😀") && n.text.contains("두 번째 메모 줄"))
    );
    assert_eq!(
        plain_text(&doc::load(&root, &result.docs[0]).unwrap().body),
        "원고 본문"
    );
}
