//! Tests of the HWPX export: the package, running heads and footers.

use std::io::{Cursor, Read};

use zip::CompressionMethod;

use super::header::{PARA_HEAD_CENTER, PARA_TITLE, STYLE_HEAD};
use super::*;
use crate::format::{HeadAlign, HeadContent, builtin};
use crate::markup::{Inline, Mark};

fn entries(bytes: &[u8]) -> Vec<(String, String, bool)> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    (0..zip.len())
        .map(|i| {
            let mut f = zip.by_index(i).unwrap();
            let stored = f.compression() == CompressionMethod::Stored;
            let mut s = String::new();
            f.read_to_string(&mut s).unwrap();
            (f.name().to_string(), s, stored)
        })
        .collect()
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
                        text: "방점\u{3000}끝\u{2002}고정".into(),
                        marks: vec![Mark::Dot {}],
                    },
                ]),
                Block::SceneBreak {},
                Block::para(vec![]),
                Block::Paragraph {
                    attrs: ParaAttrs {
                        left: 2,
                        right: 1,
                        indent: None,
                    },
                    content: vec![Inline::Text {
                        text: "편지\u{a0}묶음".into(),
                        marks: vec![],
                    }],
                },
            ],
        },
        ExportDoc {
            heading: "2장".into(),
            blocks: vec![Block::text("둘째")],
        },
    ]
}

#[test]
fn package_layout() {
    let format = builtin("submission-a4").unwrap();
    let opts = DocOptions {
        include_titles: true,
        scene_break: "* * *".into(),
    };
    let info = DocInfo {
        title: "달빛 서점".into(),
        author: "달무리".into(),
    };
    let bytes = hwpx_bytes(&sample(), &format, &opts, &info).unwrap();
    let files = entries(&bytes);
    assert_eq!(files[0].0, "mimetype");
    assert_eq!(files[0].1, "application/hwp+zip");
    assert!(files[0].2, "mimetype is stored");

    let get = |name: &str| files.iter().find(|f| f.0 == name).unwrap().1.clone();
    let section = get("Contents/section0.xml");
    assert!(section.contains(
        r#"<hp:pagePr landscape="WIDELY" width="59528" height="84189" gutterType="LEFT_ONLY">"#
    ));
    assert!(section.contains(r#"left="8504" right="8504" top="5669" bottom="4252""#));
    assert!(section.contains("굵게 &amp; &lt;표시&gt;<hp:lineBreak/>"));
    assert!(section.contains("방점\u{3000}끝<hp:fwSpace/>고정"));
    assert!(section.contains(r#"<hp:pageNum pos="BOTTOM_CENTER""#));
    // The second chapter title starts a new page, the first does not.
    assert_eq!(section.matches(r#"pageBreak="1""#).count(), 1);
    assert_eq!(section.matches("<hp:secPr").count(), 1);
    assert!(!section.contains("linesegarray"));
    // 문단 여백: its own paragraph shape; 묶음 빈칸 as 한글 writes it.
    assert!(section.contains(r#"paraPrIDRef="25""#));
    assert!(section.contains("편지<hp:nbSpace/>묶음"));
    assert!(!section.contains("<hp:header"));

    let header = get("Contents/header.xml");
    assert!(header.contains(r#"face="함초롬바탕""#));
    // 바탕글 carries the format: 10pt, 160%, 1-character indent.
    assert!(header.contains(r#"<hh:charPr id="0" height="1000""#));
    assert!(header.contains(r#"<hc:intent value="1000" unit="HWPUNIT"/>"#));
    assert!(header.contains(r#"<hc:intent value="2000" unit="HWPUNIT"/>"#));
    assert!(header.contains(r#"<hh:lineSpacing type="PERCENT" value="160" unit="HWPUNIT"/>"#));
    assert!(header.contains(r#"symMark="DOT_ABOVE""#));
    // 2 characters of 10pt on the left, 1 on the right; doubled in the fallback.
    assert!(header.contains(r#"<hh:paraPr id="25""#));
    assert!(header.contains(
        r#"<hc:left value="2000" unit="HWPUNIT"/><hc:right value="1000" unit="HWPUNIT"/>"#
    ));
    assert!(header.contains(
        r#"<hc:left value="4000" unit="HWPUNIT"/><hc:right value="2000" unit="HWPUNIT"/>"#
    ));
    assert!(header.contains(r#"<hh:paraProperties itemCnt="26">"#));
    assert!(header.contains("<hh:bold/>"));
    assert!(get("Contents/content.hpf").contains("<opf:title>달빛 서점</opf:title>"));
    assert!(get("Preview/PrvText.txt").starts_with("1장 서점의 문\r\n"));
}

#[test]
fn running_heads() {
    use crate::format::RunningHead;

    let opts = DocOptions {
        include_titles: true,
        scene_break: "*".into(),
    };
    let info = DocInfo {
        title: "달빛 서점".into(),
        author: "달무리".into(),
    };
    let section_of = |format: &ManuscriptFormat| {
        let bytes = hwpx_bytes(&sample(), format, &opts, &info).unwrap();
        entries(&bytes)
            .into_iter()
            .find(|f| f.0 == "Contents/section0.xml")
            .unwrap()
            .1
    };

    // The work's title on every page, once, in the first run with the section settings.
    let mut format = builtin("submission-a4").unwrap();
    format.header = RunningHead {
        content: HeadContent::Title,
        skip_chapter_first: false,
        ..RunningHead::default()
    };
    let section = section_of(&format);
    assert_eq!(section.matches("<hp:header ").count(), 1);
    assert!(section.contains(r#"applyPageType="BOTH""#));
    assert!(section.contains(&format!(
        r#"paraPrIDRef="{PARA_HEAD_CENTER}" styleIDRef="14""#
    )));
    assert!(section.contains("<hp:t>달빛 서점</hp:t>"));
    assert!(section.find("<hp:header ").unwrap() > section.find("<hp:secPr").unwrap());
    assert!(!section.contains("pageHiding"));

    // Books: title on even pages at the left, each chapter's title on odd
    // pages at the right, none on chapter openings.
    let mut format = builtin("book-shinguk").unwrap();
    format.header = RunningHead {
        content: HeadContent::TitleChapter,
        align: HeadAlign::Outside,
        skip_chapter_first: true,
        ..RunningHead::default()
    };
    let section = section_of(&format);
    assert_eq!(section.matches(r#"applyPageType="EVEN""#).count(), 1);
    assert_eq!(section.matches(r#"applyPageType="ODD""#).count(), 2);
    assert!(section.contains("<hp:t>1장 서점의 문</hp:t>"));
    assert!(section.contains("<hp:t>2장</hp:t>"));
    assert_eq!(
        section.matches(r#"<hp:pageHiding hideHeader="1""#).count(),
        2
    );
    // The second chapter's controls lead its title paragraph.
    let second = section.find("<hp:t>2장</hp:t>").unwrap();
    let title_para = section[..second]
        .rfind(&format!(r#"paraPrIDRef="{PARA_TITLE}""#))
        .unwrap();
    assert!(
        section[title_para..]
            .find(r#"applyPageType="ODD""#)
            .unwrap()
            < second - title_para
    );
}

#[test]
fn letter_spacing_and_facing_pages() {
    let format = builtin("book-shinguk").unwrap();
    let opts = DocOptions {
        include_titles: false,
        scene_break: "◆".into(),
    };
    let bytes = hwpx_bytes(&sample(), &format, &opts, &DocInfo::default()).unwrap();
    let files = entries(&bytes);
    let get = |name: &str| files.iter().find(|f| f.0 == name).unwrap().1.clone();
    assert!(get("Contents/header.xml").contains(r#"<hh:spacing hangul="-3""#));
    let section = get("Contents/section0.xml");
    assert!(section.contains(r#"gutterType="LEFT_RIGHT""#));
    assert!(section.contains(r#"width="43087" height="63780""#));
    assert_eq!(section.matches(r#"pageBreak="1""#).count(), 1);
}

#[test]
fn footer_beside_the_page_number() {
    let mut format = builtin("submission-a4").unwrap();
    format.page_number_align = HeadAlign::Outside;
    format.footer = crate::format::RunningFoot {
        text: "달빛 서점 · 투고".into(),
        align: HeadAlign::Center,
    };
    let opts = DocOptions {
        include_titles: true,
        scene_break: "* * *".into(),
    };
    let bytes = hwpx_bytes(&sample(), &format, &opts, &DocInfo::default()).unwrap();
    let files = entries(&bytes);
    let section = files
        .iter()
        .find(|f| f.0 == "Contents/section0.xml")
        .unwrap()
        .1
        .clone();
    assert!(section.contains(r#"<hp:pageNum pos="OUTSIDE_BOTTOM""#));
    // One 꼬리말, in the first chapter only, centred, at the bottom.
    assert_eq!(section.matches("<hp:footer ").count(), 1);
    assert!(section.contains(r#"<hp:footer id="1" applyPageType="BOTH"><hp:subList id="" textDirection="HORIZONTAL" lineWrap="BREAK" vertAlign="BOTTOM""#));
    assert!(section.contains(&format!(
        r#"paraPrIDRef="{PARA_HEAD_CENTER}" styleIDRef="{STYLE_HEAD}""#
    )));
    assert!(section.contains("<hp:t>달빛 서점 · 투고</hp:t>"));

    format.footer.align = HeadAlign::Outside;
    format.page_number_align = HeadAlign::Center;
    let bytes = hwpx_bytes(&sample(), &format, &opts, &DocInfo::default()).unwrap();
    let section = entries(&bytes)
        .into_iter()
        .find(|f| f.0 == "Contents/section0.xml")
        .unwrap()
        .1;
    assert!(section.contains(r#"<hp:footer id="1" applyPageType="EVEN""#));
    assert!(section.contains(r#"<hp:footer id="2" applyPageType="ODD""#));
}
