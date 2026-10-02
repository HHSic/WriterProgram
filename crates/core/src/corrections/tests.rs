//! Reading small hand-made 한글 files against a sent chapter.

use super::review::build;
use super::{ChangeClass, ChangeKind, ChapterReview, Found};
use crate::import::marked;
use crate::markup::Block;

/// A small 한글 file: one plain char shape, a tracked change by 이교열,
/// the paragraphs given and `tail` at the end of the section.
fn hwpx(paragraphs: &[&str], tail: &str) -> Vec<u8> {
    let header = r##"<hh:head xmlns:hh="h"><hh:charProperties><hh:charPr id="0" textColor="#000000" shadeColor="none" symMark="NONE"><hh:underline type="NONE"/><hh:strikeout shape="NONE"/></hh:charPr></hh:charProperties><hh:trackChanges><hh:trackChange type="Delete" date="2026-10-02T01:00:00Z" authorID="7" id="5"/></hh:trackChanges><hh:trackChangeAuthors><hh:trackChangeAuthor name="이교열" id="7"/></hh:trackChangeAuthors></hh:head>"##;
    let body: String = paragraphs
        .iter()
        .map(|p| {
            format!(r#"<hp:p paraPrIDRef="0" styleIDRef="0"><hp:run charPrIDRef="0"><hp:t>{p}</hp:t></hp:run></hp:p>"#)
        })
        .collect();
    let section = format!(r#"<hs:sec xmlns:hs="s" xmlns:hp="p">{body}{tail}</hs:sec>"#);
    crate::xml::zip_package(
        [
            ("mimetype", b"application/hwp+zip".as_slice(), false),
            ("Contents/header.xml", header.as_bytes(), true),
            ("Contents/section0.xml", section.as_bytes(), true),
        ],
        "한글",
    )
    .unwrap()
}

fn review(sent: &[&str], file: &[u8]) -> ChapterReview {
    let blocks: Vec<Block> = sent.iter().map(|t| Block::text(t)).collect();
    let marked = marked::read_bytes(file, "hwpx", "◆").unwrap();
    let now = Some(blocks.clone());
    build(&[("d".into(), "1장".into())], &[blocks], &[now], &marked)
        .unwrap()
        .remove(0)
}

#[test]
fn tracked_paragraph_join_highlight_and_split() {
    let file = hwpx(
        &[
            r#"가나<hp:deleteBegin Id="1" TcId="5"/>다"#,
            r#"라<hp:deleteEnd Id="1" TcId="5"/>마"#,
            r##"<hp:markpenBegin color="#FFFF00"/>바<hp:markpenEnd/>사 아자"##,
            "차카타.",
            "파하.",
        ],
        "",
    );
    let r = review(&["가나다", "라마", "바사 아자", "차카타. 파하."], &file);
    let changes: Vec<(&str, &str, ChangeKind, Found)> = r
        .changes
        .iter()
        .map(|c| (c.before.as_str(), c.after.as_str(), c.kind, c.how))
        .collect();
    assert_eq!(
        changes,
        [
            ("다\u{2029}라", "", ChangeKind::Delete, Found::Tracked),
            (" ", "\u{2029}", ChangeKind::Split, Found::Compared),
        ]
    );
    assert_eq!(r.changes[0].author.as_deref(), Some("이교열"));
    assert_eq!(r.changes[1].class, ChangeClass::Wording);
    // The join runs from the end of 가나 to 마.
    assert_eq!(
        (r.changes[0].at.from.block, r.changes[0].at.from.offset),
        (0, 2)
    );
    assert_eq!(
        (r.changes[0].at.to.block, r.changes[0].at.to.offset),
        (1, 1)
    );
    assert_eq!(r.looks.len(), 1);
    assert_eq!(r.looks[0].quote, "바");
    assert!(r.looks[0].highlight && !r.looks[0].underline);
}

#[test]
fn memo_without_a_field_is_a_note_on_no_text() {
    let memo = r#"<hp:memogroup><hp:memo id="m1"><hp:paraList><hp:p><hp:run><hp:t>전체적으로 좋아요.</hp:t></hp:run></hp:p></hp:paraList></hp:memo></hp:memogroup>"#;
    let r = review(&["첫 문단."], &hwpx(&["첫 문단."], memo));
    assert!(r.changes.is_empty());
    assert_eq!(r.notes.len(), 1);
    assert_eq!(r.notes[0].text, "전체적으로 좋아요.");
    assert!(r.notes[0].at.is_none());
}

#[test]
fn scene_breaks_and_the_sent_symbol() {
    let sent = vec![Block::text("앞."), Block::SceneBreak {}, Block::text("뒤.")];
    let file = hwpx(&["앞.", "§", "뒤."], "");
    let marked = marked::read_bytes(&file, "hwpx", "§").unwrap();
    let r = build(
        &[("d".into(), "1장".into())],
        std::slice::from_ref(&sent),
        &[Some(sent.clone())],
        &marked,
    )
    .unwrap();
    assert!(r[0].changes.is_empty());
}
