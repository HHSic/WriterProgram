//! Round trips between blocks, body text and the editor's JSON.

use super::write::{Run, runs};
use super::*;

fn t(text: &str, marks: &[Mark]) -> Inline {
    Inline::Text {
        text: text.into(),
        marks: marks.to_vec(),
    }
}
fn p(content: Vec<Inline>) -> Block {
    Block::para(content)
}
fn pm(left: u8, right: u8, content: Vec<Inline>) -> Block {
    Block::Paragraph {
        attrs: ParaAttrs {
            left,
            right,
            indent: None,
        },
        content,
    }
}
#[test]
fn first_line_round_trips() {
    let shaped = |left: u8, indent: Option<i8>| Block::Paragraph {
        attrs: ParaAttrs {
            left,
            right: 0,
            indent,
        },
        content: vec![t("문단", &[])],
    };
    let blocks = vec![shaped(0, Some(-2)), shaped(2, Some(0)), shaped(0, Some(3))];
    let text = write_body(&blocks);
    assert!(text.contains("<p data-indent=\"-2\">문단</p>"));
    assert!(text.contains("<p data-left=\"2\" data-indent=\"0\">문단</p>"));
    assert_eq!(parse_body(&text), blocks);
    // Too deep either way is kept within reach.
    assert_eq!(
        parse_body("<p data-indent=\"-99\">x</p>")[0].attrs().indent,
        Some(-10)
    );
}

fn br() -> Inline {
    Inline::HardBreak {}
}
fn memo(id: &str) -> Mark {
    Mark::Memo {
        attrs: MemoAttrs { id: id.into() },
    }
}
const B: Mark = Mark::Bold {};
const I: Mark = Mark::Italic {};
const S: Mark = Mark::Strike {};
const U: Mark = Mark::Underline {};
const D: Mark = Mark::Dot {};

/// Canonical form the parser produces: merged runs, sorted marks.
fn canon(blocks: &[Block]) -> Vec<Block> {
    blocks
        .iter()
        .map(|b| match b {
            Block::SceneBreak {} => Block::SceneBreak {},
            Block::Paragraph { attrs, content } => {
                let mut out: Vec<Inline> = Vec::new();
                for r in runs(content) {
                    match r {
                        Run::Break => out.push(br()),
                        Run::Text(text, marks) => out.push(Inline::Text { text, marks }),
                    }
                }
                Block::Paragraph {
                    attrs: *attrs,
                    content: out,
                }
            }
        })
        .collect()
}

fn round_trip(blocks: Vec<Block>) -> String {
    let text = write_body(&blocks);
    assert_eq!(parse_body(&text), canon(&blocks), "text was:\n{text}");
    text
}

#[test]
fn plain_paragraphs() {
    let text = round_trip(vec![
        Block::text("셔터를 반쯤 내렸을 때 종이 울렸다."),
        Block::text("“영업, 끝났나요?”"),
    ]);
    assert_eq!(
        text,
        "셔터를 반쯤 내렸을 때 종이 울렸다.\n\n“영업, 끝났나요?”\n"
    );
}

#[test]
fn scene_breaks_and_empty_paragraphs() {
    let text = round_trip(vec![
        Block::text("앞 장면"),
        Block::SceneBreak {},
        p(vec![]),
        Block::text("뒤 장면"),
    ]);
    assert_eq!(text, "앞 장면\n\n***\n\n&nbsp;\n\n뒤 장면\n");
}

#[test]
fn line_breaks_inside_paragraph() {
    let text = round_trip(vec![p(vec![t("첫 줄", &[]), br(), t("둘째 줄", &[])])]);
    assert_eq!(text, "첫 줄\n둘째 줄\n");
    let text = round_trip(vec![p(vec![
        br(),
        t("가", &[]),
        br(),
        br(),
        t("나", &[]),
        br(),
    ])]);
    assert_eq!(text, "\\\n가\n\\\n나\n\\\n");
    round_trip(vec![p(vec![br()])]);
}

#[test]
fn marks() {
    let text = round_trip(vec![p(vec![
        t("그녀는 ", &[]),
        t("굵게", &[B]),
        t("와 ", &[]),
        t("기울임", &[I]),
        t(", ", &[]),
        t("취소", &[S]),
        t(", ", &[]),
        t("밑줄", &[U]),
        t(", ", &[]),
        t("방점", &[D]),
    ])]);
    assert_eq!(
        text,
        "그녀는 **굵게**와 *기울임*, ~~취소~~, <u>밑줄</u>, <span class=\"dot\">방점</span>\n"
    );
}

#[test]
fn overlapping_marks() {
    round_trip(vec![p(vec![
        t("a", &[B]),
        t("b", &[B, I]),
        t("c", &[I]),
        t("d", &[U, I]),
        t("e", &[U, D, B]),
        t("f", &[D]),
    ])]);
    round_trip(vec![p(vec![
        t("굵게", &[B, I]),
        t("기울임", &[I]),
        t("굵게", &[B]),
    ])]);
    round_trip(vec![p(vec![t("x", &[B]), br(), t("y", &[B]), t("z", &[])])]);
}

#[test]
fn memo_anchors() {
    let text = round_trip(vec![p(vec![
        t("앞 ", &[]),
        t("구간", &[memo("m1")]),
        t("겹침", &[memo("m1"), memo("m2")]),
        t("끝", &[memo("m2"), B]),
    ])]);
    assert!(text.starts_with("앞 <mark data-memo=\"m1\">구간<mark data-memo=\"m2\">"));
}

#[test]
fn escapes() {
    for s in [
        "별*표",
        "***",
        "* * *",
        "\\",
        "a\\b",
        "물결~",
        "~~",
        "~~~",
        "그래~ 알았어~~",
        "&nbsp;",
        "<u>태그</u> 아님",
        "<span class=\"dot\">",
        "<mark data-memo=\"x\">",
        "<상태창> 레벨 업",
        "a < b > c",
        "\\*",
        " 앞 공백",
        "뒤 공백 ",
        "   ",
    ] {
        round_trip(vec![Block::text(s)]);
        round_trip(vec![p(vec![t(s, &[B])])]);
        round_trip(vec![p(vec![t(s, &[S]), t(s, &[]), t(s, &[U, S])])]);
    }
}

#[test]
fn paragraph_margins() {
    let text = round_trip(vec![
        Block::text("그리고 편지에는 이렇게 적혀 있었다."),
        pm(2, 0, vec![t("서하에게.", &[])]),
        pm(
            2,
            1,
            vec![t("잘 지내니?", &[]), br(), t("나는 ", &[]), t("잘", &[B])],
        ),
        pm(0, 3, vec![]),
        pm(1, 0, vec![br(), t("앞이 빈 줄", &[]), br()]),
    ]);
    assert!(text.contains("\n\n<p data-left=\"2\">서하에게.</p>\n\n"));
    assert!(text.contains("<p data-left=\"2\" data-right=\"1\">잘 지내니?\n나는 **잘**</p>"));
    assert!(text.contains("\n\n<p data-right=\"3\"></p>\n\n"));
    // Text that looks like the tag stays text, and the tag inside text too.
    round_trip(vec![Block::text("<p data-left=\"2\">가짜</p>")]);
    round_trip(vec![
        Block::text("<p>"),
        pm(1, 0, vec![t("끝이 </p>", &[])]),
    ]);
    round_trip(vec![
        pm(1, 1, vec![t("&nbsp;", &[])]),
        pm(1, 0, vec![t("\\", &[])]),
    ]);
    // Hand edits: a missing end tag, too wide a margin, unknown attributes.
    assert_eq!(
        parse_body("<p data-left=\"3\">열림"),
        vec![pm(3, 0, vec![t("열림", &[])])]
    );
    assert_eq!(
        parse_body("<p data-left=\"99\">넓음</p>")[0].attrs().left,
        ParaAttrs::MAX
    );
    assert_eq!(
        parse_body("<p class=\"x\">그대로</p>"),
        vec![Block::text("<p class=\"x\">그대로</p>")]
    );
    // Web novel brackets starting with <p stay readable.
    assert_eq!(write_body(&[Block::text("<power>")]), "<power>\n");
}

#[test]
fn margins_from_the_editor() {
    let json = r#"{"type":"doc","content":[
        {"type":"paragraph","attrs":{"left":2,"right":0},"content":[{"type":"text","text":"편지"}]},
        {"type":"paragraph","attrs":{"left":0,"right":0},"content":[{"type":"text","text":"본문"}]},
        {"type":"paragraph","attrs":{"left":2.6,"right":-4,"textAlign":"left"}},
        {"type":"paragraph","attrs":null},
        {"type":"paragraph","attrs":{"left":null}}
    ]}"#;
    let body: Body = serde_json::from_str(json).unwrap();
    assert_eq!(
        body.content[0].attrs(),
        ParaAttrs {
            left: 2,
            right: 0,
            indent: None
        }
    );
    assert!(body.content[1].attrs().is_plain());
    assert_eq!(
        body.content[2].attrs(),
        ParaAttrs {
            left: 3,
            right: 0,
            indent: None
        }
    );
    let back = serde_json::to_string(&body).unwrap();
    assert!(back.contains(r#""attrs":{"left":2,"right":0}"#));
    assert!(!back.contains(r#"{"type":"paragraph","attrs":{"left":0"#));
}

#[test]
fn web_novel_brackets_stay_readable() {
    let text = round_trip(vec![Block::text("<상태창>"), Block::text("[레벨 업!]")]);
    assert_eq!(text, "<상태창>\n\n[레벨 업!]\n");
}

#[test]
fn lenient_with_hand_edits() {
    let blocks = parse_body("첫 문단\n\n\n\n둘째 문단\n  ***  \n");
    assert_eq!(blocks.len(), 2);
    assert_eq!(parse_body("  ***  "), vec![Block::SceneBreak {}]);
    // An unclosed mark ends with its paragraph.
    assert_eq!(
        parse_body("**열림\n\n닫힘"),
        vec![p(vec![t("열림", &[B])]), Block::text("닫힘")]
    );
    // Unknown backslash sequences stay as they are.
    assert_eq!(parse_body("a\\가"), vec![Block::text("a\\가")]);
}

#[test]
fn json_matches_tiptap() {
    let json = r#"{"type":"doc","content":[
        {"type":"paragraph","content":[
            {"type":"text","text":"굵게","marks":[{"type":"bold"}]},
            {"type":"hardBreak"},
            {"type":"text","text":"메모","marks":[{"type":"memo","attrs":{"id":"m1"}}]}
        ]},
        {"type":"sceneBreak"},
        {"type":"paragraph"}
    ]}"#;
    let body: Body = serde_json::from_str(json).unwrap();
    assert_eq!(body.content.len(), 3);
    let back = serde_json::to_value(&body).unwrap();
    assert_eq!(
        back,
        serde_json::from_str::<serde_json::Value>(json).unwrap()
    );
}

/// Random bodies over a tricky alphabet must survive a write and read.
#[test]
fn random_round_trips() {
    let alphabet: Vec<char> = "가나 다*~<>\\/&;nbspumarkd=\"\n.lefti-".chars().collect();
    let all_marks = [B, I, S, U, D, memo("a"), memo("b")];
    let mut seed: u64 = 0x2545_f491_4f6c_dd1d;
    let mut next = move |n: usize| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed % n as u64) as usize
    };
    for _ in 0..3000 {
        let mut blocks = Vec::new();
        for _ in 0..next(4) + 1 {
            if next(8) == 0 {
                blocks.push(Block::SceneBreak {});
                continue;
            }
            let mut content = Vec::new();
            let attrs = if next(3) == 0 {
                ParaAttrs {
                    left: next(4) as u8,
                    right: next(3) as u8,
                    // None, or a first line from -2 to 2 characters.
                    indent: match next(6) {
                        0 => None,
                        n => Some(n as i8 - 3),
                    },
                }
            } else {
                ParaAttrs::default()
            };
            for _ in 0..next(5) {
                if next(6) == 0 {
                    content.push(br());
                    continue;
                }
                let text: String = (0..next(6) + 1)
                    .map(|_| alphabet[next(alphabet.len())])
                    .collect();
                let marks: Vec<Mark> = all_marks.iter().filter(|_| next(4) == 0).cloned().collect();
                content.push(t(&text, &marks));
            }
            blocks.push(Block::Paragraph { attrs, content });
        }
        round_trip(blocks);
    }
}
