//! Tests of the Word export: page layout, indents, running heads and footers.

use std::io::{Cursor, Read};

use super::*;
use crate::format::{HeadAlign, HeadContent};
use crate::format::{RunningHead, builtin};
use crate::markup::{Block, ParaAttrs};
use crate::markup::{Inline, Mark};

fn unzip(bytes: &[u8], name: &str) -> String {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = zip.by_name(name).unwrap();
    let mut s = String::new();
    file.read_to_string(&mut s).unwrap();
    s
}

fn names(bytes: &[u8]) -> Vec<String> {
    let zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    zip.file_names().map(str::to_string).collect()
}

fn sample() -> Vec<ExportDoc> {
    vec![
        ExportDoc {
            heading: "1장 서점의 문".into(),
            blocks: vec![
                Block::para(vec![
                    Inline::Text {
                        text: "굵게 & <표시>".into(),
                        marks: vec![Mark::Bold {}],
                    },
                    Inline::HardBreak {},
                    Inline::Text {
                        text: "방점".into(),
                        marks: vec![Mark::Dot {}],
                    },
                ]),
                Block::SceneBreak {},
                Block::para(vec![]),
            ],
        },
        ExportDoc {
            heading: "2장".into(),
            blocks: vec![Block::text("둘째")],
        },
    ]
}

fn info() -> DocInfo {
    DocInfo {
        title: "달빛 서점".into(),
        author: "달무리".into(),
    }
}

#[test]
fn a4_submission_layout() {
    let format = builtin("submission-a4").unwrap();
    let opts = DocOptions {
        include_titles: true,
        scene_break: "* * *".into(),
    };
    let bytes = docx_bytes(&sample(), &format, &opts, &info()).unwrap();

    let doc = unzip(&bytes, "word/document.xml");
    assert!(doc.contains(r#"<w:pgSz w:w="11906" w:h="16838"/>"#));
    // Body starts 35mm down (20mm margin + 15mm header area).
    assert!(doc.contains(r#"w:top="1984""#));
    assert!(doc.contains("굵게 &amp; &lt;표시&gt;"));
    assert!(doc.contains(r#"<w:em w:val="dot"/>"#));
    assert!(doc.contains("<w:br/>"));
    assert!(doc.contains("* * *"));
    assert_eq!(doc.matches(r#"<w:pStyle w:val="WPChapter"/>"#).count(), 2);
    assert_eq!(doc.matches(r#"<w:pageBreakBefore w:val="0"/>"#).count(), 1);
    assert_eq!(doc.matches("<w:sectPr>").count(), 1);
    assert!(!doc.contains("headerReference"));

    let styles = unzip(&bytes, "word/styles.xml");
    assert!(styles.contains(r#"w:eastAsia="바탕""#));
    assert!(styles.contains(r#"<w:sz w:val="20"/>"#));
    assert!(styles.contains(r#"w:line="320" w:lineRule="exact""#)); // 10pt × 160%
    assert!(styles.contains(r#"w:firstLineChars="100""#));
    assert!(unzip(&bytes, "word/footer3.xml").contains("PAGE"));
    assert!(unzip(&bytes, "docProps/core.xml").contains("<dc:title>달빛 서점</dc:title>"));
}

#[test]
fn letter_spacing_and_facing_pages() {
    let format = builtin("book-shinguk").unwrap();
    let opts = DocOptions {
        include_titles: false,
        scene_break: "◆".into(),
    };
    let bytes = docx_bytes(&sample(), &format, &opts, &DocInfo::default()).unwrap();
    let styles = unzip(&bytes, "word/styles.xml");
    assert!(styles.contains(r#"<w:spacing w:val="-6"/>"#)); // -3% of 10pt
    assert!(unzip(&bytes, "word/settings.xml").contains("<w:mirrorMargins/>"));
    let doc = unzip(&bytes, "word/document.xml");
    assert!(!doc.contains("WPChapter"));
    // Without titles the second chapter's first paragraph breaks the page.
    assert_eq!(doc.matches("<w:pageBreakBefore/>").count(), 1);
}

#[test]
fn paragraph_margins_become_indents() {
    let format = builtin("submission-a4").unwrap();
    let opts = DocOptions {
        include_titles: false,
        scene_break: "*".into(),
    };
    let docs = vec![ExportDoc {
        heading: String::new(),
        blocks: vec![Block::Paragraph {
            attrs: ParaAttrs {
                left: 2,
                right: 1,
                indent: None,
            },
            content: vec![Inline::Text {
                text: "편지".into(),
                marks: vec![],
            }],
        }],
    }];
    let doc = unzip(
        &docx_bytes(&docs, &format, &opts, &info()).unwrap(),
        "word/document.xml",
    );
    assert!(doc.contains(
        r#"<w:ind w:leftChars="200" w:left="400" w:rightChars="100" w:right="200" w:firstLineChars="100" w:firstLine="200"/>"#
    ));
}

#[test]
fn own_first_lines_become_indents() {
    let format = builtin("submission-a4").unwrap();
    let opts = DocOptions {
        include_titles: false,
        scene_break: "*".into(),
    };
    let para = |indent| Block::Paragraph {
        attrs: ParaAttrs {
            left: 0,
            right: 0,
            indent,
        },
        content: vec![Inline::Text {
            text: "문단".into(),
            marks: vec![],
        }],
    };
    let docs = vec![ExportDoc {
        heading: String::new(),
        blocks: vec![para(Some(-1)), para(Some(0)), para(None)],
    }];
    let doc = unzip(
        &docx_bytes(&docs, &format, &opts, &info()).unwrap(),
        "word/document.xml",
    );
    assert!(doc.contains(r#"w:hangingChars="100" w:hanging="200"/>"#));
    assert!(doc.contains(r#"w:firstLineChars="0" w:firstLine="0"/>"#));
    // The third follows the style (Normal) and needs no indent of its own.
    assert_eq!(doc.matches("<w:ind ").count(), 2);
}

#[test]
fn running_head_with_the_title() {
    let mut format = builtin("submission-a4").unwrap();
    format.header = RunningHead {
        content: HeadContent::Title,
        align: HeadAlign::Right,
        skip_chapter_first: false,
        ..RunningHead::default()
    };
    let opts = DocOptions {
        include_titles: true,
        scene_break: "*".into(),
    };
    let bytes = docx_bytes(&sample(), &format, &opts, &info()).unwrap();
    let header = unzip(&bytes, "word/header4.xml");
    assert!(header.contains(r#"<w:jc w:val="right"/>"#));
    assert!(header.contains("달빛 서점"));
    let doc = unzip(&bytes, "word/document.xml");
    assert!(doc.contains(r#"<w:headerReference w:type="default" r:id="rId4"/>"#));
    assert!(doc.contains(r#"<w:footerReference w:type="default" r:id="rId3"/>"#));
    assert_eq!(doc.matches("<w:sectPr>").count(), 1);
    assert!(unzip(&bytes, "word/_rels/document.xml.rels").contains(r#"Target="header4.xml""#));
    assert!(unzip(&bytes, "[Content_Types].xml").contains("/word/header4.xml"));
}

#[test]
fn footer_text_beside_the_page_number() {
    let mut format = builtin("submission-a4").unwrap();
    format.footer = crate::format::RunningFoot {
        text: "달빛 서점 · 투고".into(),
        align: HeadAlign::Outside,
    };
    format.page_number_align = HeadAlign::Center;
    let opts = DocOptions {
        include_titles: true,
        scene_break: "*".into(),
    };
    let bytes = docx_bytes(&sample(), &format, &opts, &info()).unwrap();
    // Outside: odd pages on the right, even pages on the left.
    assert!(unzip(&bytes, "word/settings.xml").contains("<w:evenAndOddHeaders/>"));
    let odd = unzip(&bytes, "word/footer3.xml");
    let even = unzip(&bytes, "word/footer4.xml");
    // A4 with 30mm margins: 150mm of text, 8504 twips.
    assert!(
        odd.contains(r#"<w:tab w:val="center" w:pos="4252"/><w:tab w:val="right" w:pos="8504"/>"#)
    );
    let tab = "<w:r><w:tab/></w:r>";
    // Odd: nothing on the left, the number in the middle, the 꼬리말 on the right.
    assert!(odd.find(tab).unwrap() < odd.find("PAGE").unwrap());
    assert!(odd.find("PAGE").unwrap() < odd.rfind(tab).unwrap());
    assert!(odd.rfind(tab).unwrap() < odd.find("달빛 서점").unwrap());
    assert!(even.find("달빛 서점").unwrap() < even.find(tab).unwrap());
    let doc = unzip(&bytes, "word/document.xml");
    assert!(doc.contains(r#"<w:footerReference w:type="even""#));

    // The 꼬리말 alone is simply aligned.
    format.page_numbers = false;
    format.footer.align = HeadAlign::Right;
    let bytes = docx_bytes(&sample(), &format, &opts, &info()).unwrap();
    let only = unzip(&bytes, "word/footer3.xml");
    assert!(only.contains(r#"<w:jc w:val="right"/>"#) && !only.contains("PAGE"));
}

#[test]
fn book_heads_face_each_other_and_skip_chapter_openings() {
    let mut format = builtin("book-shinguk").unwrap();
    format.header = RunningHead {
        content: HeadContent::TitleChapter,
        align: HeadAlign::Outside,
        skip_chapter_first: true,
        ..RunningHead::default()
    };
    let opts = DocOptions {
        include_titles: true,
        scene_break: "◆".into(),
    };
    let bytes = docx_bytes(&sample(), &format, &opts, &info()).unwrap();
    let files = names(&bytes);
    // Footers: default, even, first. Headers: odd, even, and an empty first.
    for n in 3..=8 {
        assert!(
            files.iter().any(|f| f.ends_with(&format!("{n}.xml"))),
            "part {n} in {files:?}"
        );
    }
    assert!(unzip(&bytes, "word/settings.xml").contains("<w:evenAndOddHeaders/>"));
    let odd = unzip(&bytes, "word/header6.xml");
    assert!(odd.contains(r#"STYLEREF "장 제목""#) && odd.contains(r#"<w:jc w:val="right"/>"#));
    assert!(odd.contains("1장 서점의 문"));
    let even = unzip(&bytes, "word/header7.xml");
    assert!(even.contains("달빛 서점") && even.contains(r#"<w:jc w:val="left"/>"#));
    assert!(!unzip(&bytes, "word/header8.xml").contains("<w:t"));

    let doc = unzip(&bytes, "word/document.xml");
    // Two chapters, two sections: the first names the parts, the second
    // starts a new page and carries them over.
    assert_eq!(doc.matches("<w:sectPr>").count(), 2);
    assert_eq!(doc.matches("<w:titlePg/>").count(), 2);
    assert_eq!(doc.matches("headerReference").count(), 3);
    assert_eq!(doc.matches(r#"<w:type w:val="nextPage"/>"#).count(), 1);
    // The section break does the page break; titles do not add another.
    assert_eq!(doc.matches(r#"<w:pageBreakBefore w:val="0"/>"#).count(), 2);
    assert!(!doc.contains("<w:pageBreakBefore/>"));
}

#[test]
fn chapter_head_without_titles_uses_the_work_title() {
    let mut format = builtin("submission-a4").unwrap();
    format.header.content = HeadContent::Chapter;
    let opts = DocOptions {
        include_titles: false,
        scene_break: "*".into(),
    };
    let bytes = docx_bytes(&sample(), &format, &opts, &info()).unwrap();
    let files = names(&bytes);
    let header = files.iter().find(|f| f.contains("header")).unwrap().clone();
    let text = unzip(&bytes, &header);
    assert!(!text.contains("STYLEREF"));
    assert!(text.contains("달빛 서점"));
}
