//! header.xml: fonts, character and paragraph shapes and styles, with the
//! ids the section refers to.

use std::fmt::Write as _;

use super::{NAMESPACES, XML_DECL, hu_pt};
use crate::export::RunStyle;
use crate::format::{ManuscriptFormat, font};
use crate::markup::ParaAttrs;
use crate::xml;

// Char shape ids. 0–6 are 한글's defaults (0 is 바탕글, which carries the
// manuscript format); ours follow.
pub(super) const CHAR_TITLE: usize = 7;
pub(super) const CHAR_HEAD: usize = 8;
pub(super) const CHAR_RUNS: usize = 9;

// Paragraph shape ids. 0–19 are 한글's defaults (0 is 바탕글); ours follow:
// chapter title, scene break, 머리말 left / centre / right, then one per
// 문단 여백 in use.
pub(super) const PARA_TITLE: usize = 20;
pub(super) const PARA_SCENE: usize = 21;
pub(super) const PARA_HEAD_LEFT: usize = 22;
pub(super) const PARA_HEAD_CENTER: usize = 23;
pub(super) const PARA_HEAD_RIGHT: usize = 24;
pub(super) const PARA_MARGINS: usize = 25;

/// Style id of 한글's 머리말 style in the list written by `header`.
pub(super) const STYLE_HEAD: usize = 14;
/// 장 제목: chapter titles, so 한글 lists them and 가져오기 finds them again.
pub(super) const STYLE_TITLE: usize = 23;

const LANGS: [&str; 7] = [
    "HANGUL", "LATIN", "HANJA", "JAPANESE", "OTHER", "SYMBOL", "USER",
];
const TYPE_INFO: &str = r#"<hh:typeInfo familyType="FCAT_GOTHIC" weight="6" proportion="4" contrast="0" strokeVariation="1" armStyle="1" letterform="1" midline="1" xHeight="1"/>"#;

fn per_lang(name: &str, value: impl std::fmt::Display) -> String {
    format!(
        r#"<hh:{name} hangul="{value}" latin="{value}" hanja="{value}" japanese="{value}" other="{value}" symbol="{value}" user="{value}"/>"#
    )
}

#[derive(Clone, Copy, Default)]
struct CharShape {
    height: i64,
    font: usize,
    spacing: i32,
    color: &'static str,
    style: RunStyle,
}

fn char_pr(id: usize, c: CharShape) -> String {
    let s = c.style;
    let sym = if s.dot { "DOT_ABOVE" } else { "NONE" };
    format!(
        r##"<hh:charPr id="{id}" height="{height}" textColor="{color}" shadeColor="none" useFontSpace="0" useKerning="0" symMark="{sym}" borderFillIDRef="2">{font}{ratio}{spacing}{rel}{offset}{italic}{bold}<hh:underline type="{underline}" shape="SOLID" color="#000000"/><hh:strikeout shape="{strike}" color="#000000"/><hh:outline type="NONE"/><hh:shadow type="NONE" color="#C0C0C0" offsetX="10" offsetY="10"/></hh:charPr>"##,
        height = c.height,
        color = c.color,
        font = per_lang("fontRef", c.font),
        ratio = per_lang("ratio", 100),
        spacing = per_lang("spacing", c.spacing),
        rel = per_lang("relSz", 100),
        offset = per_lang("offset", 0),
        italic = if s.italic { "<hh:italic/>" } else { "" },
        bold = if s.bold { "<hh:bold/>" } else { "" },
        underline = if s.underline { "BOTTOM" } else { "NONE" },
        strike = if s.strike { "SOLID" } else { "NONE" },
    )
}

#[derive(Clone, Copy)]
struct ParaShape {
    align: &'static str,
    heading: (&'static str, u32),
    tab: usize,
    condense: u32,
    keep_word: bool,
    keep_with_next: bool,
    /// HWPUNIT: first-line indent, left and right margins, space before, space after.
    indent: i64,
    left: i64,
    right: i64,
    prev: i64,
    next: i64,
    line_spacing: u32,
}

impl Default for ParaShape {
    fn default() -> Self {
        ParaShape {
            align: "JUSTIFY",
            heading: ("NONE", 0),
            tab: 0,
            condense: 0,
            keep_word: true,
            keep_with_next: false,
            indent: 0,
            left: 0,
            right: 0,
            prev: 0,
            next: 0,
            line_spacing: 160,
        }
    }
}

fn para_pr(id: usize, p: ParaShape) -> String {
    // 한글 writes margins twice: exact values for current versions, and
    // doubled values in the fallback branch for older readers.
    let margins = |k: i64| {
        format!(
            r#"<hh:margin><hc:intent value="{}" unit="HWPUNIT"/><hc:left value="{}" unit="HWPUNIT"/><hc:right value="{}" unit="HWPUNIT"/><hc:prev value="{}" unit="HWPUNIT"/><hc:next value="{}" unit="HWPUNIT"/></hh:margin><hh:lineSpacing type="PERCENT" value="{}" unit="HWPUNIT"/>"#,
            p.indent * k,
            p.left * k,
            p.right * k,
            p.prev * k,
            p.next * k,
            p.line_spacing
        )
    };
    format!(
        r#"<hh:paraPr id="{id}" tabPrIDRef="{tab}" condense="{condense}" fontLineHeight="0" snapToGrid="1" suppressLineNumbers="0" checked="0" textDir="LTR"><hh:align horizontal="{align}" vertical="BASELINE"/><hh:heading type="{htype}" idRef="0" level="{hlevel}"/><hh:breakSetting breakLatinWord="KEEP_WORD" breakNonLatinWord="{non_latin}" widowOrphan="0" keepWithNext="{kwn}" keepLines="0" pageBreakBefore="0" lineWrap="BREAK"/><hh:autoSpacing eAsianEng="0" eAsianNum="0"/><hp:switch><hp:case hp:required-namespace="http://www.hancom.co.kr/hwpml/2016/HwpUnitChar">{case}</hp:case><hp:default>{default}</hp:default></hp:switch><hh:border borderFillIDRef="2" offsetLeft="0" offsetRight="0" offsetTop="0" offsetBottom="0" connect="0" ignoreMargin="0"/></hh:paraPr>"#,
        tab = p.tab,
        condense = p.condense,
        align = p.align,
        htype = p.heading.0,
        hlevel = p.heading.1,
        non_latin = if p.keep_word {
            "KEEP_WORD"
        } else {
            "BREAK_WORD"
        },
        kwn = u8::from(p.keep_with_next),
        case = margins(1),
        default = margins(2),
    )
}

/// Width of one character of body text in HWPUNIT, letter spacing included.
fn char_hu(format: &ManuscriptFormat) -> f64 {
    format.size_pt * (1.0 + f64::from(format.letter_spacing) / 100.0) * 100.0
}

pub(super) fn header(
    format: &ManuscriptFormat,
    run_styles: &[RunStyle],
    margins: &[ParaAttrs],
) -> String {
    let body_font = font(&format.font).hwpx;
    let size = hu_pt(format.size_pt);
    let pitch = hu_pt(format.size_pt * f64::from(format.line_spacing) / 100.0);
    let letter = format.letter_spacing;

    let mut out = format!(
        r#"{XML_DECL}<hh:head {NAMESPACES} version="1.5" secCnt="1"><hh:beginNum page="1" footnote="1" endnote="1" pic="1" tbl="1" equation="1"/><hh:refList>"#
    );

    // Fonts: 0 is 함초롬돋움 (한글's UI font, used by the default styles), 1 the body font.
    let _ = write!(out, r#"<hh:fontfaces itemCnt="{}">"#, LANGS.len());
    for lang in LANGS {
        let _ = write!(
            out,
            r#"<hh:fontface lang="{lang}" fontCnt="2"><hh:font id="0" face="함초롬돋움" type="TTF" isEmbedded="0">{TYPE_INFO}</hh:font><hh:font id="1" face="{}" type="TTF" isEmbedded="0">{TYPE_INFO}</hh:font></hh:fontface>"#,
            xml::attr(body_font)
        );
    }
    out.push_str("</hh:fontfaces>");

    out.push_str(r#"<hh:borderFills itemCnt="2">"#);
    for id in 1..=2 {
        let fill = if id == 2 {
            r##"<hc:fillBrush><hc:winBrush faceColor="none" hatchColor="#999999" alpha="0"/></hc:fillBrush>"##
        } else {
            ""
        };
        let _ = write!(
            out,
            r##"<hh:borderFill id="{id}" threeD="0" shadow="0" centerLine="NONE" breakCellSeparateLine="0"><hh:slash type="NONE" Crooked="0" isCounter="0"/><hh:backSlash type="NONE" Crooked="0" isCounter="0"/><hh:leftBorder type="NONE" width="0.1 mm" color="#000000"/><hh:rightBorder type="NONE" width="0.1 mm" color="#000000"/><hh:topBorder type="NONE" width="0.1 mm" color="#000000"/><hh:bottomBorder type="NONE" width="0.1 mm" color="#000000"/><hh:diagonal type="SOLID" width="0.1 mm" color="#000000"/>{fill}</hh:borderFill>"##
        );
    }
    out.push_str("</hh:borderFills>");

    // Char shapes.
    let base = CharShape {
        height: size,
        font: 1,
        spacing: letter,
        color: "#000000",
        style: RunStyle::default(),
    };
    let default = |height: i64, font: usize, spacing: i32, color: &'static str| CharShape {
        height,
        font,
        spacing,
        color,
        style: RunStyle::default(),
    };
    let mut chars = vec![
        base,                           // 0 바탕글 = manuscript text
        default(1000, 0, 0, "#000000"), // 1 쪽 번호
        default(900, 0, 0, "#000000"),  // 2 머리말
        default(900, 1, 0, "#000000"),  // 3 각주·미주
        default(900, 0, -5, "#000000"), // 4 메모
        default(1600, 0, 0, "#2E74B5"), // 5 차례 제목
        default(1100, 0, 0, "#000000"), // 6 차례
        CharShape {
            height: hu_pt((format.size_pt * 1.6 * 2.0).round() / 2.0),
            style: RunStyle {
                bold: true,
                ..RunStyle::default()
            },
            ..base
        }, // 7 장 제목
        CharShape {
            height: hu_pt((format.size_pt * 0.9 * 2.0).round() / 2.0),
            spacing: 0,
            ..base
        }, // 8 머리말
    ];
    for style in run_styles {
        chars.push(CharShape {
            style: *style,
            ..base
        });
    }
    let _ = write!(out, r#"<hh:charProperties itemCnt="{}">"#, chars.len());
    for (id, c) in chars.into_iter().enumerate() {
        out.push_str(&char_pr(id, c));
    }
    out.push_str("</hh:charProperties>");

    out.push_str(r#"<hh:tabProperties itemCnt="3"><hh:tabPr id="0" autoTabLeft="0" autoTabRight="0"/><hh:tabPr id="1" autoTabLeft="1" autoTabRight="0"/><hh:tabPr id="2" autoTabLeft="0" autoTabRight="1"/></hh:tabProperties>"#);

    out.push_str(r#"<hh:numberings itemCnt="1"><hh:numbering id="1" start="0">"#);
    let heads = [
        ("DIGIT", "^1.", "0"),
        ("HANGUL_SYLLABLE", "^2.", "0"),
        ("DIGIT", "^3)", "0"),
        ("HANGUL_SYLLABLE", "^4)", "0"),
        ("DIGIT", "(^5)", "0"),
        ("HANGUL_SYLLABLE", "(^6)", "0"),
        ("CIRCLED_DIGIT", "^7", "1"),
        ("CIRCLED_HANGUL_SYLLABLE", "^8", "1"),
        ("HANGUL_JAMO", "", "0"),
        ("ROMAN_SMALL", "", "1"),
    ];
    for (i, (num_format, text, checkable)) in heads.iter().enumerate() {
        let attrs = format!(
            r#"start="1" level="{}" align="LEFT" useInstWidth="1" autoIndent="1" widthAdjust="0" textOffsetType="PERCENT" textOffset="50" numFormat="{num_format}" charPrIDRef="4294967295" checkable="{checkable}""#,
            i + 1
        );
        if text.is_empty() {
            let _ = write!(out, "<hh:paraHead {attrs}/>");
        } else {
            let _ = write!(out, "<hh:paraHead {attrs}>{text}</hh:paraHead>");
        }
    }
    out.push_str("</hh:numbering></hh:numberings>");

    // Paragraph shapes.
    let body = ParaShape {
        indent: hu_pt(format.indent * format.size_pt),
        next: if format.blank_line_between { pitch } else { 0 },
        line_spacing: format.line_spacing,
        ..ParaShape::default()
    };
    let outline = |level: u32, left: i64| ParaShape {
        heading: ("OUTLINE", level),
        tab: 1,
        condense: 20,
        left,
        ..ParaShape::default()
    };
    let plain = |align: &'static str, line: u32| ParaShape {
        align,
        line_spacing: line,
        ..ParaShape::default()
    };
    let title = ParaShape {
        align: "LEFT",
        keep_with_next: true,
        prev: if format.chapter_new_page {
            pitch * 3
        } else {
            pitch
        },
        next: pitch * 2,
        line_spacing: 140,
        ..ParaShape::default()
    };
    let scene_space = if format.blank_line_between { 0 } else { pitch };
    let scene = ParaShape {
        align: "CENTER",
        prev: scene_space,
        next: scene_space.max(body.next),
        line_spacing: format.line_spacing,
        ..ParaShape::default()
    };
    let paras = vec![
        body, // 0 바탕글 = manuscript text
        ParaShape {
            left: 1500,
            keep_word: true,
            ..ParaShape::default()
        }, // 1 본문
        outline(0, 1000), // 2–8 개요 1–7
        outline(1, 2000),
        outline(2, 3000),
        outline(3, 4000),
        outline(4, 5000),
        outline(5, 6000),
        outline(6, 7000),
        plain("JUSTIFY", 150), // 9 머리말
        ParaShape {
            indent: -1310,
            line_spacing: 130,
            ..ParaShape::default()
        }, // 10 각주·미주
        plain("LEFT", 130),    // 11 메모
        ParaShape {
            align: "LEFT",
            prev: 1200,
            next: 300,
            ..ParaShape::default()
        }, // 12 차례 제목
        ParaShape {
            align: "LEFT",
            next: 700,
            ..ParaShape::default()
        }, // 13 차례 1
        ParaShape {
            align: "LEFT",
            left: 1100,
            next: 700,
            ..ParaShape::default()
        }, // 14 차례 2
        ParaShape {
            align: "LEFT",
            left: 2200,
            next: 700,
            ..ParaShape::default()
        }, // 15 차례 3
        outline(8, 9000),      // 16 개요 9
        outline(9, 10000),     // 17 개요 10
        outline(7, 8000),      // 18 개요 8
        ParaShape {
            next: 800,
            line_spacing: 150,
            ..ParaShape::default()
        }, // 19 캡션
        title,                 // 20 장 제목
        scene,                 // 21 장면 나눔
        plain("LEFT", 150),    // 22–24 머리말 왼쪽 · 가운데 · 오른쪽
        plain("CENTER", 150),
        plain("RIGHT", 150),
    ];
    let mut paras = paras;
    for m in margins {
        // 25– 문단 여백
        paras.push(ParaShape {
            left: (f64::from(m.left) * char_hu(format)).round() as i64,
            right: (f64::from(m.right) * char_hu(format)).round() as i64,
            // The paragraph's own first line; negative is 내어쓰기.
            indent: m
                .indent
                .map_or(body.indent, |i| hu_pt(f64::from(i) * format.size_pt)),
            ..body
        });
    }
    let _ = write!(out, r#"<hh:paraProperties itemCnt="{}">"#, paras.len());
    for (id, p) in paras.into_iter().enumerate() {
        out.push_str(&para_pr(id, p));
    }
    out.push_str("</hh:paraProperties>");

    let styles: [(&str, &str, &str, usize, usize, usize); 24] = [
        ("PARA", "바탕글", "Normal", 0, 0, 0),
        ("PARA", "본문", "Body", 1, 0, 1),
        ("PARA", "개요 1", "Outline 1", 2, 0, 2),
        ("PARA", "개요 2", "Outline 2", 3, 0, 3),
        ("PARA", "개요 3", "Outline 3", 4, 0, 4),
        ("PARA", "개요 4", "Outline 4", 5, 0, 5),
        ("PARA", "개요 5", "Outline 5", 6, 0, 6),
        ("PARA", "개요 6", "Outline 6", 7, 0, 7),
        ("PARA", "개요 7", "Outline 7", 8, 0, 8),
        ("PARA", "개요 8", "Outline 8", 18, 0, 9),
        ("PARA", "개요 9", "Outline 9", 16, 0, 10),
        ("PARA", "개요 10", "Outline 10", 17, 0, 11),
        ("CHAR", "쪽 번호", "Page Number", 0, 1, 0),
        ("CHAR", "줄 번호", "Line Number", 0, 0, 0),
        ("PARA", "머리말", "Header", 9, 2, 14),
        ("PARA", "각주", "Footnote", 10, 3, 15),
        ("PARA", "미주", "Endnote", 10, 3, 16),
        ("PARA", "메모", "Memo", 11, 4, 17),
        ("PARA", "차례 제목", "TOC Heading", 12, 5, 18),
        ("PARA", "차례 1", "TOC 1", 13, 6, 19),
        ("PARA", "차례 2", "TOC 2", 14, 6, 20),
        ("PARA", "차례 3", "TOC 3", 15, 6, 21),
        ("PARA", "캡션", "Caption", 19, 0, 22),
        (
            "PARA",
            "장 제목",
            "Chapter Title",
            PARA_TITLE,
            CHAR_TITLE,
            0,
        ),
    ];
    let _ = write!(out, r#"<hh:styles itemCnt="{}">"#, styles.len());
    for (id, (kind, name, eng, para, chr, next)) in styles.iter().enumerate() {
        let _ = write!(
            out,
            r#"<hh:style id="{id}" type="{kind}" name="{name}" engName="{eng}" paraPrIDRef="{para}" charPrIDRef="{chr}" nextStyleIDRef="{next}" langID="1042" lockForm="0"/>"#
        );
    }
    out.push_str("</hh:styles>");

    out.push_str(r#"</hh:refList><hh:compatibleDocument targetProgram="HWP201X"><hh:layoutCompatibility/></hh:compatibleDocument><hh:docOption><hh:linkinfo path="" pageInherit="0" footnoteInherit="0"/></hh:docOption><hh:trackchageConfig flags="56"/></hh:head>"#);
    out
}
