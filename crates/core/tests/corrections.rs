//! 교정본 주고받기: send chapters, edit the exported file the way an editor
//! would (in the XML), read it back and apply the corrections.

use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

use chrono::Duration;
use regex::Regex;
use writer_core::corrections::{self, ChangeClass, ChangeKind, Decisions, Found, Review, State};
use writer_core::doc;
use writer_core::export::{DocOptions, ExportItem, FileKind};
use writer_core::format::builtin;
use writer_core::markup::{Block, Inline, Mark};
use writer_core::project::{self, NewDoc, NewProject, ProjectKind};
use writer_core::snapshot;
use zip::write::SimpleFileOptions;

fn new_project(dir: &Path) -> PathBuf {
    project::create(&NewProject {
        parent: dir.to_string_lossy().into_owned(),
        title: "달빛 서점".into(),
        kind: ProjectKind::Print,
        per_doc_goal: None,
        count_spaces: true,
        first_chapter: true,
        platform: None,
    })
    .unwrap()
}

fn save(root: &Path, id: &str, body: Vec<Block>) {
    doc::save_body(root, id, body, Duration::days(1), Default::default()).unwrap();
}

fn lines(root: &Path, id: &str) -> Vec<String> {
    doc::load(root, id)
        .unwrap()
        .body
        .iter()
        .map(|b| match b {
            Block::SceneBreak {} => "***".to_string(),
            _ => b.lines().join("\n"),
        })
        .collect()
}

/// Rewrites the parts of a zip package.
fn edit_zip(bytes: &[u8], mut edit: impl FnMut(&str, String) -> String) -> Vec<u8> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).unwrap();
        let name = f.name().to_string();
        let mut data = Vec::new();
        f.read_to_end(&mut data).unwrap();
        let data = match String::from_utf8(data) {
            Ok(text) => edit(&name, text).into_bytes(),
            Err(e) => e.into_bytes(),
        };
        out.start_file(name, SimpleFileOptions::default()).unwrap();
        out.write_all(&data).unwrap();
    }
    out.finish().unwrap().into_inner()
}

/// Adds a part to a zip package.
fn add_part(bytes: &[u8], name: &str, text: &str) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new_append(Cursor::new(bytes.to_vec())).unwrap();
    zip.start_file(name, SimpleFileOptions::default()).unwrap();
    zip.write_all(text.as_bytes()).unwrap();
    zip.finish().unwrap().into_inner()
}

fn replace_once(text: &str, from: &str, to: &str) -> String {
    assert!(text.contains(from), "not in the file: {from}");
    text.replacen(from, to, 1)
}

fn opts() -> DocOptions {
    DocOptions {
        include_titles: true,
        scene_break: "◆".into(),
    }
}

fn items(ids: &[&str]) -> Vec<ExportItem> {
    ids.iter()
        .enumerate()
        .map(|(i, id)| ExportItem {
            doc_id: id.to_string(),
            heading: format!("{}장", i + 1),
            file_name: format!("{}장", i + 1),
        })
        .collect()
}

fn change<'a>(review: &'a Review, before: &str, after: &str) -> &'a corrections::Change {
    review
        .chapters
        .iter()
        .flat_map(|c| &c.changes)
        .find(|c| c.before == before && c.after == after)
        .unwrap_or_else(|| panic!("no change {before:?} → {after:?}: {:#?}", review.chapters))
}

const RUN: &str = r#"<hp:run charPrIDRef="0"><hp:t>"#;
const END: &str = "</hp:t></hp:run>";

#[test]
fn hangul_corrections_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path());
    let first = project::overview(&root).unwrap().parts[0].docs[0]
        .id
        .clone();
    let second = project::add_doc(
        &root,
        &NewDoc {
            title: "둘째".into(),
            ..Default::default()
        },
    )
    .unwrap();
    save(
        &root,
        &first,
        vec![
            Block::text("그는 천천히 문을 열었다."),
            Block::text("비가 올것 같았다."),
            Block::text("“정말이요?” 그녀가 물었다"),
            Block::text("첫 문장이다."),
            Block::text("이어지는 둘째 문장이다."),
            Block::SceneBreak {},
            Block::text("지울 말이 여기 있다."),
            Block::para(vec![]),
            Block::text("서점은 조용했다."),
            Block::text("편집자가 밑줄 친 문장."),
            Block::para(vec![
                Inline::Text {
                    text: "원래 밑줄".into(),
                    marks: vec![Mark::Underline {}],
                },
                Inline::Text {
                    text: "이 있는 문단.".into(),
                    marks: vec![],
                },
            ]),
        ],
    );
    save(
        &root,
        &second,
        vec![
            Block::text("둘째 회차의 첫 문단이다."),
            Block::text("작가가 나중에 고친 문단이다."),
            Block::text("편집자가 고친 다른 문단."),
        ],
    );

    // 편집자에게 보내기.
    let dest = dir.path().join("보낸 원고.hwpx");
    let format = builtin("submission-a4").unwrap();
    let ex = corrections::send(
        &root,
        &items(&[&first, &second]),
        &opts(),
        &format,
        FileKind::Hwpx,
        &dest,
        false,
    )
    .unwrap();
    assert_eq!(ex.files, ["보낸 원고.hwpx"]);
    assert_eq!(ex.chapters.len(), 2);
    assert_eq!(ex.chapters[0].fingerprint.len(), 16);
    // Sending exports exactly what export_file writes.
    let plain = dir.path().join("그냥 내보내기.hwpx");
    writer_core::export::export_file(
        &root,
        &items(&[&first, &second]),
        &opts(),
        &format,
        FileKind::Hwpx,
        &plain,
        false,
    )
    .unwrap();
    let section = |bytes: &[u8]| {
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        let mut s = String::new();
        zip.by_name("Contents/section0.xml")
            .unwrap()
            .read_to_string(&mut s)
            .unwrap();
        s
    };
    assert_eq!(
        section(&fs::read(&dest).unwrap()),
        section(&fs::read(&plain).unwrap())
    );

    // The writer keeps working after sending.
    let mut body = doc::load(&root, &second).unwrap().body;
    body[1] = Block::text("작가가 나중에 다시 고친 문단이다.");
    save(&root, &second, body);

    // The editor's file: tracked changes, strikethrough, coloured new text,
    // an underline, direct edits, a merged paragraph and a 메모.
    let corrected = edit_zip(&fs::read(&dest).unwrap(), |name, xml| match name {
        "Contents/header.xml" => {
            let shapes = r##"<hh:charPr id="90" height="1000" textColor="#000000" shadeColor="none" symMark="NONE"><hh:underline type="NONE" shape="SOLID" color="#000000"/><hh:strikeout shape="SOLID" color="#000000"/></hh:charPr><hh:charPr id="91" height="1000" textColor="#FF0000" shadeColor="none" symMark="NONE"><hh:underline type="NONE" shape="SOLID" color="#000000"/><hh:strikeout shape="NONE" color="#000000"/></hh:charPr><hh:charPr id="92" height="1000" textColor="#000000" shadeColor="none" symMark="NONE"><hh:underline type="BOTTOM" shape="SOLID" color="#000000"/><hh:strikeout shape="NONE" color="#000000"/></hh:charPr>"##;
            let xml = xml.replacen(
                "</hh:charProperties>",
                &format!("{shapes}</hh:charProperties>"),
                1,
            );
            let tracks = r##"<hh:trackChanges itemCnt="2"><hh:trackChange type="Delete" date="2026-10-01T09:00:00Z" authorID="1" charshapeID="0" parashapeID="0" hide="0" id="1"/><hh:trackChange type="Insert" date="2026-10-01T09:00:00Z" authorID="1" charshapeID="0" parashapeID="0" hide="0" id="2"/></hh:trackChanges><hh:trackChangeAuthors itemCnt="1"><hh:trackChangeAuthor name="김편집" mark="1" color="#FF0000" id="1"/></hh:trackChangeAuthors>"##;
            replace_once(&xml, "</hh:head>", &format!("{tracks}</hh:head>"))
        }
        "Contents/section0.xml" => {
            let mut x = xml;
            x = replace_once(
                &x,
                "<hp:t>그는 천천히",
                r#"<hp:t>그<hp:deleteBegin Id="1" TcId="1" paraend="0"/>는<hp:deleteEnd Id="1" TcId="1" paraend="0"/><hp:insertBegin Id="2" TcId="2" paraend="0"/>가<hp:insertEnd Id="2" TcId="2" paraend="0"/> 천천히"#,
            );
            let memo_begin = r#"<hp:run charPrIDRef="0"><hp:ctrl><hp:fieldBegin id="77" type="MEMO" name="" editable="1" dirty="0" zorder="-1" fieldid="77"><hp:parameters cnt="3" name=""><hp:stringParam name="Author">김편집</hp:stringParam><hp:stringParam name="CreateDateTime">2026-10-01 10:00</hp:stringParam><hp:stringParam name="MemoID">memo1</hp:stringParam></hp:parameters></hp:fieldBegin></hp:ctrl></hp:run>"#;
            let memo_end = r#"<hp:run charPrIDRef="0"><hp:ctrl><hp:fieldEnd beginIDRef="77" fieldid="77"/></hp:ctrl></hp:run>"#;
            x = replace_once(
                &x,
                &format!("{RUN}비가 올것 같았다.{END}"),
                &format!("{memo_begin}{RUN}비는 올 것 같았다.{END}{memo_end}"),
            );
            x = replace_once(&x, "그녀가 물었다<", "그녀가 물었다.<");
            x = Regex::new(r#"첫 문장이다\.</hp:t></hp:run></hp:p><hp:p [^>]*><hp:run charPrIDRef="0"><hp:t>이어지는"#)
                .unwrap()
                .replace(&x, "첫 문장이다. 이어지는")
                .into_owned();
            x = replace_once(
                &x,
                &format!("지울 말이 여기 있다.{END}"),
                &format!(
                    r#"지울 말이 {END}<hp:run charPrIDRef="90"><hp:t>여기 {END}{RUN}있다.{END}"#
                ),
            );
            x = replace_once(
                &x,
                &format!("서점은 조용했다.{END}"),
                &format!(
                    r#"서점은 {END}<hp:run charPrIDRef="91"><hp:t>무척 {END}{RUN}조용했다.{END}"#
                ),
            );
            x = replace_once(
                &x,
                &format!("편집자가 밑줄 친 문장.{END}"),
                &format!(
                    r#"편집자가 {END}<hp:run charPrIDRef="92"><hp:t>밑줄 친{END}{RUN} 문장.{END}"#
                ),
            );
            x = replace_once(&x, "나중에 고친 문단", "나중에 손본 문단");
            x = replace_once(&x, "편집자가 고친 다른", "편집자가 손본 다른");
            let memo = r#"<hp:memogroup><hp:memo id="memo1" memoShapeIDRef="0"><hp:paraList><hp:p id="0" paraPrIDRef="0" styleIDRef="0"><hp:run charPrIDRef="0"><hp:t>비가 오는 장면이 앞과 안 맞아요.</hp:t></hp:run></hp:p></hp:paraList></hp:memo></hp:memogroup>"#;
            replace_once(&x, "</hs:sec>", &format!("{memo}</hs:sec>"))
        }
        _ => xml,
    });
    let back = dir.path().join("교정본_김편집.hwpx");
    fs::write(&back, &corrected).unwrap();

    let review = corrections::read_corrected(&root, &ex.id, &back).unwrap();
    assert_eq!(review.file, "교정본_김편집.hwpx");
    assert!(
        root.join(".exchanges")
            .join(&ex.id)
            .join(&review.stored)
            .exists()
    );
    let one = &review.chapters[0];
    assert!(one.found && !one.edited && !one.gone);
    let summary: Vec<(String, String, ChangeClass, Found)> = one
        .changes
        .iter()
        .map(|c| (c.before.clone(), c.after.clone(), c.class, c.how))
        .collect();
    let s = |a: &str, b: &str, class, how| (a.to_string(), b.to_string(), class, how);
    assert_eq!(
        summary,
        [
            s("는", "가", ChangeClass::Wording, Found::Tracked),
            s("가", "는", ChangeClass::Wording, Found::Compared),
            s("", " ", ChangeClass::Spacing, Found::Compared),
            s("", ".", ChangeClass::Punctuation, Found::Compared),
            s("\u{2029}", " ", ChangeClass::Wording, Found::Compared),
            s("여기 ", "", ChangeClass::Wording, Found::Strike),
            s("", "무척 ", ChangeClass::Wording, Found::Color),
        ]
    );
    let tracked = &one.changes[0];
    assert_eq!(tracked.author.as_deref(), Some("김편집"));
    assert_eq!(tracked.date.as_deref(), Some("2026-10-01T09:00:00Z"));
    assert_eq!(tracked.lead, "그");
    assert_eq!(tracked.at.from.block, 0);
    assert_eq!(tracked.at.from.offset, 1);
    assert_eq!(one.changes[4].kind, ChangeKind::Merge);
    // The empty paragraph before 서점은 does not come back as a change.
    assert_eq!(one.changes[6].at.from.block, 8);
    // Unchanged text the editor underlined; the sent underline is no mark.
    assert_eq!(one.looks.len(), 1);
    assert_eq!(one.looks[0].quote, "밑줄 친");
    assert!(one.looks[0].underline && !one.looks[0].color);
    assert_eq!(one.notes.len(), 1);
    let note = &one.notes[0];
    assert_eq!(note.text, "비가 오는 장면이 앞과 안 맞아요.");
    assert_eq!(note.author, "김편집");
    assert_eq!(note.quote, "비는 올 것 같았다.");
    assert_eq!(note.at.unwrap().from.block, 1);

    let two = &review.chapters[1];
    assert!(two.found && two.edited);
    assert_eq!(two.changes.len(), 2);
    assert!(two.changes[0].overlap && two.changes[0].now.is_none());
    assert!(!two.changes[1].overlap && two.changes[1].now.is_some());

    // Spacing and punctuation first, as a whole class.
    let applied = corrections::apply(
        &root,
        &ex.id,
        &Decisions {
            accept_classes: vec![ChangeClass::Spacing, ChangeClass::Punctuation],
            ..Decisions::default()
        },
    )
    .unwrap();
    assert_eq!(applied.accepted.len(), 2);
    assert_eq!(applied.docs, std::slice::from_ref(&first));
    let records = snapshot::list(&root, &first).unwrap();
    assert!(records.iter().any(|r| r.kind == "before-corrections"));
    assert_eq!(lines(&root, &first)[1], "비가 올 것 같았다.");

    // Then the rest; the change in the paragraph the writer rewrote stays.
    let ids = |r: &Review| -> Vec<String> {
        r.chapters
            .iter()
            .flat_map(|c| &c.changes)
            .filter(|c| c.state == State::Pending && c.class == ChangeClass::Wording)
            .map(|c| c.id.clone())
            .collect()
    };
    let applied = corrections::apply(
        &root,
        &ex.id,
        &Decisions {
            accept: ids(&applied.review),
            keep_notes: vec![note.id.clone()],
            ..Decisions::default()
        },
    )
    .unwrap();
    assert_eq!(applied.skipped.len(), 1);
    assert_eq!(applied.skipped[0].id, two.changes[0].id);
    assert_eq!(applied.memos.len(), 1);
    assert_eq!(
        lines(&root, &first),
        [
            "그가 천천히 문을 열었다.",
            "비는 올 것 같았다.",
            "“정말이요?” 그녀가 물었다.",
            "첫 문장이다. 이어지는 둘째 문장이다.",
            "***",
            "지울 말이 있다.",
            "",
            "서점은 무척 조용했다.",
            "편집자가 밑줄 친 문장.",
            "원래 밑줄이 있는 문단.",
        ]
    );
    assert_eq!(
        lines(&root, &second),
        [
            "둘째 회차의 첫 문단이다.",
            "작가가 나중에 다시 고친 문단이다.",
            "편집자가 손본 다른 문단."
        ]
    );
    // The kept note is a 메모 on the text it was on, with its mark.
    let memo = writer_core::notes::load(&root, &applied.memos[0]).unwrap();
    assert_eq!(memo.target, first);
    assert!(memo.text.contains("비가 오는 장면"));
    let body = doc::load(&root, &first).unwrap().body;
    let Block::Paragraph { content, .. } = &body[1] else {
        panic!()
    };
    assert!(
        content
            .iter()
            .all(|i| matches!(i, Inline::Text { marks, .. }
        if marks.iter().any(|m| matches!(m, Mark::Memo { attrs } if attrs.id == memo.id))))
    );
    // The sent underline is still there.
    let Block::Paragraph { content, .. } = &body[9] else {
        panic!()
    };
    assert!(
        matches!(&content[0], Inline::Text { marks, .. } if marks.contains(&Mark::Underline {}))
    );

    let list = corrections::list(&root).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].pending, Some(1));
    assert_eq!(list[0].exchange.received.len(), 1);
    let kept = corrections::load_review(&root, &ex.id).unwrap().unwrap();
    assert_eq!(kept.pending(), 1);

    // The writer puts the rewritten paragraph back as it was sent and adds
    // one before it: the change is no longer 겹침 and is placed one down.
    save(
        &root,
        &second,
        vec![
            Block::text("새로 넣은 문단."),
            Block::text("둘째 회차의 첫 문단이다."),
            Block::text("작가가 나중에 고친 문단이다."),
            Block::text("편집자가 손본 다른 문단."),
        ],
    );
    let now = corrections::current_review(&root, &ex.id).unwrap().unwrap();
    let back = &now.chapters[1].changes[0];
    assert_eq!(back.state, State::Pending);
    assert!(!back.overlap);
    let at = back.now.unwrap();
    assert_eq!((at.from.block, at.to.block), (2, 2));
    assert_eq!(at.from.offset, back.at.from.offset);
}

#[test]
fn word_corrections_and_one_file_per_chapter() {
    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path());
    let first = project::overview(&root).unwrap().parts[0].docs[0]
        .id
        .clone();
    let second = project::add_doc(&root, &NewDoc::default()).unwrap();
    save(
        &root,
        &first,
        vec![
            Block::text("그녀는 책장을 넘겼다."),
            Block::text("바람이 불었다 ."),
            Block::text("남자는 대답하지 않았다."),
            Block::text("하나로 길게 이어진 문단을 편집자가 둘로 나누었다."),
        ],
    );
    save(&root, &second, vec![Block::text("다른 회차의 글.")]);
    let folder = dir.path().join("보냄");
    fs::create_dir_all(&folder).unwrap();
    let format = builtin("submission-a4").unwrap();
    let ex = corrections::send(
        &root,
        &items(&[&first, &second]),
        &opts(),
        &format,
        FileKind::Docx,
        &folder,
        true,
    )
    .unwrap();
    assert_eq!(ex.files, ["1장.docx", "2장.docx"]);
    assert_eq!(ex.chapters[1].file, "2장.docx");

    let r = |text: &str| format!(r#"<w:r><w:t xml:space="preserve">{text}</w:t></w:r>"#);
    let corrected = edit_zip(&fs::read(folder.join("1장.docx")).unwrap(), |name, xml| {
        if name != "word/document.xml" {
            return xml;
        }
        let mut x = replace_once(
            &xml,
            &r("그녀는 책장을 넘겼다."),
            &format!(
                r#"<w:commentRangeStart w:id="3"/>{}<w:del w:id="10" w:author="박교정" w:date="2026-10-02T08:00:00Z"><w:r><w:delText>는</w:delText></w:r></w:del><w:ins w:id="11" w:author="박교정" w:date="2026-10-02T08:00:00Z"><w:r><w:t>가</w:t></w:r></w:ins>{}<w:commentRangeEnd w:id="3"/><w:r><w:commentReference w:id="3"/></w:r>"#,
                r("그녀"),
                r(" 책장을 넘겼다.")
            ),
        );
        x = replace_once(&x, "바람이 불었다 .", "바람이 불었다.");
        x = replace_once(
            &x,
            &r("남자는 대답하지 않았다."),
            &format!(
                r#"{}<w:r><w:rPr><w:strike/></w:rPr><w:t xml:space="preserve">대답하지 </w:t></w:r><w:r><w:rPr><w:color w:val="FF0000"/></w:rPr><w:t xml:space="preserve">말하지 </w:t></w:r>{}"#,
                r("남자는 "),
                r("않았다.")
            ),
        );
        replace_once(&x, "나누었다.", "나누었다. 덧붙인 말.")
    });
    let corrected = add_part(
        &corrected,
        "word/comments.xml",
        r#"<?xml version="1.0" encoding="UTF-8"?><w:comments xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:comment w:id="3" w:author="박교정" w:date="2026-10-02T08:01:00Z"><w:p><w:r><w:t>주어를 분명히</w:t></w:r></w:p></w:comment></w:comments>"#,
    );
    let back = dir.path().join("1장_교정.docx");
    fs::write(&back, &corrected).unwrap();

    let review = corrections::read_corrected(&root, &ex.id, &back).unwrap();
    assert!(review.chapters[0].found);
    // Only the first chapter went in this file.
    assert!(!review.chapters[1].found);
    let tracked = change(&review, "는", "가");
    assert_eq!(tracked.how, Found::Tracked);
    assert_eq!(tracked.author.as_deref(), Some("박교정"));
    let space = change(&review, " ", "");
    assert_eq!(space.class, ChangeClass::Spacing);
    let struck = change(&review, "대답", "말");
    assert_eq!(struck.how, Found::Strike);
    let added = change(&review, "", " 덧붙인 말.");
    assert_eq!(added.kind, ChangeKind::Insert);
    let notes = &review.chapters[0].notes;
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].text, "주어를 분명히");
    assert_eq!(notes[0].quote, "그녀가 책장을 넘겼다.");

    let all: Vec<String> = review.chapters[0]
        .changes
        .iter()
        .map(|c| c.id.clone())
        .collect();

    // A locked chapter (완료 회차 잠금) is passed by: the changes stay to
    // decide, and a kept editor note becomes a note on the whole chapter.
    let lock = |locked: bool| {
        doc::update_meta(
            &root,
            &first,
            &doc::MetaPatch {
                locked: Some(locked),
                ..Default::default()
            },
        )
        .unwrap();
    };
    lock(true);
    let before = lines(&root, &first);
    let passed = corrections::apply(
        &root,
        &ex.id,
        &Decisions {
            accept: all.clone(),
            keep_notes: vec![notes[0].id.clone()],
            ..Decisions::default()
        },
    )
    .unwrap();
    assert_eq!(passed.locked, std::slice::from_ref(&first));
    assert!(passed.docs.is_empty() && passed.accepted.is_empty() && passed.skipped.is_empty());
    assert_eq!(passed.memos.len(), 1);
    assert_eq!(lines(&root, &first), before);
    assert!(
        passed.review.chapters[0]
            .changes
            .iter()
            .all(|c| c.state == State::Pending)
    );
    lock(false);

    let applied = corrections::apply(
        &root,
        &ex.id,
        &Decisions {
            accept: all,
            ..Decisions::default()
        },
    )
    .unwrap();
    assert!(applied.skipped.is_empty());
    assert_eq!(
        lines(&root, &first),
        [
            "그녀가 책장을 넘겼다.",
            "바람이 불었다.",
            "남자는 말하지 않았다.",
            "하나로 길게 이어진 문단을 편집자가 둘로 나누었다. 덧붙인 말.",
        ]
    );

    // Rejecting is recorded; nothing is written.
    let again = corrections::read_corrected(&root, &ex.id, &back).unwrap();
    let ids: Vec<String> = again.chapters[0]
        .changes
        .iter()
        .map(|c| c.id.clone())
        .collect();
    let out = corrections::apply(
        &root,
        &ex.id,
        &Decisions {
            reject: ids.clone(),
            ..Decisions::default()
        },
    )
    .unwrap();
    assert_eq!(out.rejected.len(), ids.len());
    assert!(out.docs.is_empty());
}

#[test]
fn a_file_with_none_of_the_text_is_turned_away() {
    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path());
    let first = project::overview(&root).unwrap().parts[0].docs[0]
        .id
        .clone();
    let second = project::add_doc(&root, &NewDoc::default()).unwrap();
    save(&root, &first, vec![Block::text("보낸 첫 회차의 글이다.")]);
    save(
        &root,
        &second,
        vec![Block::text("보낸 둘째 회차의 글이다.")],
    );
    let format = builtin("submission-a4").unwrap();
    let dest = dir.path().join("보냄.hwpx");
    let ex = corrections::send(
        &root,
        &items(&[&first, &second]),
        &opts(),
        &format,
        FileKind::Hwpx,
        &dest,
        false,
    )
    .unwrap();
    let other = edit_zip(&fs::read(&dest).unwrap(), |name, xml| {
        if name == "Contents/section0.xml" {
            let x = xml.replace("보낸 첫 회차의 글이다.", "전혀 다른 원고");
            x.replace("보낸 둘째 회차의 글이다.", "엉뚱한 파일")
        } else {
            xml
        }
    });
    let path = dir.path().join("다른 파일.hwpx");
    fs::write(&path, other).unwrap();
    let err = corrections::read_corrected(&root, &ex.id, &path).unwrap_err();
    assert!(err.user_message().contains("찾지 못함"));
    assert!(corrections::load_review(&root, &ex.id).unwrap().is_none());
}
