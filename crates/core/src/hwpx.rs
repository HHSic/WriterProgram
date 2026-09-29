//! 한글 (.hwpx, OWPML) export with the manuscript format applied.
//!
//! The package mirrors what 한글 itself writes for a new document: the same
//! parts, default styles (바탕글, 본문, 개요 1–10, 쪽 번호, 머리말, 각주, 미주,
//! 메모, 차례, 캡션) and section settings. The 바탕글 style carries the
//! manuscript format, so text stays formatted when edited in 한글. Line
//! layout caches (`hp:linesegarray`) are left out on purpose: 한글 lays the
//! text out itself when it opens the file.
//!
//! 머리말 is a header control in the paragraph where it starts to apply: the
//! first one for text that never changes, each chapter's first one for the
//! chapter's title, with a 감추기 control to leave a chapter's first page
//! without it.

use std::fmt::Write as _;
use std::io::{Cursor, Write};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::export::{DocInfo, DocOptions, ExportDoc, Piece, RunStyle, pieces};
use crate::format::{HeadAlign, HeadContent, ManuscriptFormat, font};
use crate::markup::{Block, ParaAttrs};
use crate::store::now_iso;
use crate::{Error, Result, xml};

const NAMESPACES: &str = r#"xmlns:ha="http://www.hancom.co.kr/hwpml/2011/app" xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph" xmlns:hp10="http://www.hancom.co.kr/hwpml/2016/paragraph" xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section" xmlns:hc="http://www.hancom.co.kr/hwpml/2011/core" xmlns:hh="http://www.hancom.co.kr/hwpml/2011/head" xmlns:hhs="http://www.hancom.co.kr/hwpml/2011/history" xmlns:hm="http://www.hancom.co.kr/hwpml/2011/master-page" xmlns:hpf="http://www.hancom.co.kr/schema/2011/hpf" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:opf="http://www.idpf.org/2007/opf/" xmlns:ooxmlchart="http://www.hancom.co.kr/hwpml/2016/ooxmlchart" xmlns:hwpunitchar="http://www.hancom.co.kr/hwpml/2016/HwpUnitChar" xmlns:epub="http://www.idpf.org/2007/ops" xmlns:config="urn:oasis:names:tc:opendocument:xmlns:config:1.0""#;

const XML_DECL: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes" ?>"#;

/// HWPUNIT: 1/7200 inch. 1pt = 100.
fn hu_mm(mm: f64) -> i64 {
    (mm * 7200.0 / 25.4).round() as i64
}

fn hu_pt(pt: f64) -> i64 {
    (pt * 100.0).round() as i64
}

// Char shape ids. 0–6 are 한글's defaults (0 is 바탕글, which carries the
// manuscript format); ours follow.
const CHAR_TITLE: usize = 7;
const CHAR_HEAD: usize = 8;
const CHAR_RUNS: usize = 9;

// Paragraph shape ids. 0–19 are 한글's defaults (0 is 바탕글); ours follow:
// chapter title, scene break, 머리말 left / centre / right, then one per
// 문단 여백 in use.
const PARA_TITLE: usize = 20;
const PARA_SCENE: usize = 21;
const PARA_HEAD_LEFT: usize = 22;
const PARA_HEAD_CENTER: usize = 23;
const PARA_HEAD_RIGHT: usize = 24;
const PARA_MARGINS: usize = 25;

/// Style id of 한글's 머리말 style in the list written by `header`.
const STYLE_HEAD: usize = 14;

/// Builds the .hwpx file in memory.
pub fn hwpx_bytes(
    docs: &[ExportDoc],
    format: &ManuscriptFormat,
    opts: &DocOptions,
    info: &DocInfo,
) -> Result<Vec<u8>> {
    // Every distinct text style used gets its own char shape, and every
    // distinct set of paragraph margins its own paragraph shape.
    let mut run_styles: Vec<RunStyle> = Vec::new();
    let mut margins: Vec<ParaAttrs> = Vec::new();
    for doc in docs {
        for block in &doc.blocks {
            if let Block::Paragraph { attrs, content } = block {
                if !attrs.is_plain() && !margins.contains(attrs) {
                    margins.push(*attrs);
                }
                for piece in pieces(content) {
                    if let Piece::Text(_, style) = piece
                        && style != RunStyle::default()
                        && !run_styles.contains(&style)
                    {
                        run_styles.push(style);
                    }
                }
            }
        }
    }
    let char_id = |style: RunStyle| -> usize {
        if style == RunStyle::default() {
            0
        } else {
            CHAR_RUNS
                + run_styles
                    .iter()
                    .position(|s| *s == style)
                    .expect("collected")
        }
    };

    let para_id = |attrs: ParaAttrs| -> usize {
        if attrs.is_plain() {
            0
        } else {
            PARA_MARGINS + margins.iter().position(|m| *m == attrs).expect("collected")
        }
    };

    let (section, preview) = section(docs, format, opts, info, &char_id, &para_id);
    let parts: Vec<(&str, Vec<u8>, bool)> = vec![
        ("mimetype", b"application/hwp+zip".to_vec(), false),
        ("version.xml", version().into_bytes(), true),
        (
            "Contents/header.xml",
            header(format, &run_styles, &margins).into_bytes(),
            true,
        ),
        ("Contents/section0.xml", section.into_bytes(), true),
        ("Preview/PrvText.txt", preview.into_bytes(), true),
        ("settings.xml", settings().into_bytes(), true),
        ("META-INF/container.rdf", container_rdf().into_bytes(), true),
        ("Contents/content.hpf", content_hpf(info).into_bytes(), true),
        ("META-INF/container.xml", container().into_bytes(), true),
        ("META-INF/manifest.xml", manifest().into_bytes(), true),
    ];

    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, body, deflate) in parts {
        // The mimetype entry must come first and stay uncompressed.
        let method = if deflate {
            CompressionMethod::Deflated
        } else {
            CompressionMethod::Stored
        };
        let options = SimpleFileOptions::default().compression_method(method);
        zip.start_file(name, options)
            .and_then(|_| zip.write_all(&body).map_err(Into::into))
            .map_err(|e| Error::Invalid(format!("한글 파일을 만들지 못함 ({e})")))?;
    }
    let cursor = zip
        .finish()
        .map_err(|e| Error::Invalid(format!("한글 파일을 만들지 못함 ({e})")))?;
    Ok(cursor.into_inner())
}

fn version() -> String {
    format!(
        r#"{XML_DECL}<hv:HCFVersion xmlns:hv="http://www.hancom.co.kr/hwpml/2011/version" tagetApplication="WORDPROCESSOR" major="5" minor="1" micro="1" buildNumber="0" os="1" xmlVersion="1.5" application="WriterProgram" appVersion="0.1.0"/>"#
    )
}

fn settings() -> String {
    format!(
        r#"{XML_DECL}<ha:HWPApplicationSetting xmlns:ha="http://www.hancom.co.kr/hwpml/2011/app" xmlns:config="urn:oasis:names:tc:opendocument:xmlns:config:1.0"><ha:CaretPosition listIDRef="0" paraIDRef="0" pos="0"/></ha:HWPApplicationSetting>"#
    )
}

fn container() -> String {
    format!(
        r#"{XML_DECL}<ocf:container xmlns:ocf="urn:oasis:names:tc:opendocument:xmlns:container" xmlns:hpf="http://www.hancom.co.kr/schema/2011/hpf"><ocf:rootfiles><ocf:rootfile full-path="Contents/content.hpf" media-type="application/hwpml-package+xml"/><ocf:rootfile full-path="Preview/PrvText.txt" media-type="text/plain"/><ocf:rootfile full-path="META-INF/container.rdf" media-type="application/rdf+xml"/></ocf:rootfiles></ocf:container>"#
    )
}

fn manifest() -> String {
    format!(
        r#"{XML_DECL}<odf:manifest xmlns:odf="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0"/>"#
    )
}

fn container_rdf() -> String {
    format!(
        r#"{XML_DECL}<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description rdf:about=""><ns0:hasPart xmlns:ns0="http://www.hancom.co.kr/hwpml/2016/meta/pkg#" rdf:resource="Contents/header.xml"/></rdf:Description><rdf:Description rdf:about="Contents/header.xml"><rdf:type rdf:resource="http://www.hancom.co.kr/hwpml/2016/meta/pkg#HeaderFile"/></rdf:Description><rdf:Description rdf:about=""><ns0:hasPart xmlns:ns0="http://www.hancom.co.kr/hwpml/2016/meta/pkg#" rdf:resource="Contents/section0.xml"/></rdf:Description><rdf:Description rdf:about="Contents/section0.xml"><rdf:type rdf:resource="http://www.hancom.co.kr/hwpml/2016/meta/pkg#SectionFile"/></rdf:Description><rdf:Description rdf:about=""><rdf:type rdf:resource="http://www.hancom.co.kr/hwpml/2016/meta/pkg#Document"/></rdf:Description></rdf:RDF>"#
    )
}

fn content_hpf(info: &DocInfo) -> String {
    let now = now_iso();
    let now = format!("{}Z", now.split('.').next().unwrap_or(&now));
    format!(
        r#"{XML_DECL}<opf:package {NAMESPACES} version="" unique-identifier="" id=""><opf:metadata><opf:title>{title}</opf:title><opf:language>ko</opf:language><opf:meta name="creator" content="text">{author}</opf:meta><opf:meta name="subject" content="text"/><opf:meta name="description" content="text"/><opf:meta name="lastsaveby" content="text">{author}</opf:meta><opf:meta name="CreatedDate" content="text">{now}</opf:meta><opf:meta name="ModifiedDate" content="text">{now}</opf:meta><opf:meta name="keyword" content="text"/></opf:metadata><opf:manifest><opf:item id="header" href="Contents/header.xml" media-type="application/xml"/><opf:item id="section0" href="Contents/section0.xml" media-type="application/xml"/><opf:item id="settings" href="settings.xml" media-type="application/xml"/></opf:manifest><opf:spine><opf:itemref idref="header" linear="yes"/><opf:itemref idref="section0" linear="yes"/></opf:spine></opf:package>"#,
        title = xml::text(&info.title),
        author = xml::text(&info.author),
    )
}

// ---------------------------------------------------------------------------
// header.xml

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

fn header(format: &ManuscriptFormat, run_styles: &[RunStyle], margins: &[ParaAttrs]) -> String {
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

    let styles: [(&str, &str, &str, usize, usize, usize); 23] = [
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

// ---------------------------------------------------------------------------
// section0.xml

/// Text inside `hp:t`: 묶음 빈칸 (no-break space) and 고정폭 빈칸 (U+2002, a
/// fixed-width space) become the elements 한글 uses for them. A full-width
/// space (U+3000) stays a character.
fn t_content(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut plain = String::new();
    for c in text.chars() {
        let element = match c {
            '\u{A0}' => "<hp:nbSpace/>",
            '\u{2002}' => "<hp:fwSpace/>",
            '\t' => {
                plain.push_str("    ");
                continue;
            }
            c => {
                plain.push(c);
                continue;
            }
        };
        out.push_str(&xml::text(&plain));
        plain.clear();
        out.push_str(element);
    }
    out.push_str(&xml::text(&plain));
    out
}

struct ParagraphWriter {
    out: String,
    next_id: u32,
    /// Section settings, opening the first paragraph's first run.
    pending_sec: Option<String>,
    /// Controls (머리말, 감추기) for the next paragraph written.
    pending_ctrls: String,
}

impl ParagraphWriter {
    fn take_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn paragraph(&mut self, para_pr: usize, page_break: bool, runs: &str) {
        let id = self.take_id();
        let ctrls = std::mem::take(&mut self.pending_ctrls);
        let lead = match self.pending_sec.take() {
            Some(sec) => format!("{sec}{ctrls}</hp:run>"),
            None if !ctrls.is_empty() => format!(r#"<hp:run charPrIDRef="0">{ctrls}</hp:run>"#),
            None => String::new(),
        };
        let runs = if runs.is_empty() {
            r#"<hp:run charPrIDRef="0"><hp:t/></hp:run>"#.to_string()
        } else {
            runs.to_string()
        };
        let _ = write!(
            self.out,
            r#"<hp:p id="{id}" paraPrIDRef="{para_pr}" styleIDRef="0" pageBreak="{}" columnBreak="0" merged="0">{lead}{runs}</hp:p>"#,
            u8::from(page_break)
        );
    }
}

/// The first run of the section, left open so controls can follow it.
fn sec_pr(format: &ManuscriptFormat) -> String {
    let (w, h) = format.page();
    let m = format.page_margins();
    let gutter = if (m.inside - m.outside).abs() > f64::EPSILON {
        "LEFT_RIGHT"
    } else {
        "LEFT_ONLY"
    };
    let page_num = if format.page_numbers {
        r#"<hp:ctrl><hp:pageNum pos="BOTTOM_CENTER" formatType="DIGIT" sideChar="-"/></hp:ctrl>"#
    } else {
        ""
    };
    format!(
        r##"<hp:run charPrIDRef="0"><hp:secPr id="" textDirection="HORIZONTAL" spaceColumns="1134" tabStop="8000" tabStopVal="4000" tabStopUnit="HWPUNIT" outlineShapeIDRef="1" memoShapeIDRef="0" textVerticalWidthHead="0" masterPageCnt="0"><hp:grid lineGrid="0" charGrid="0" wonggojiFormat="0"/><hp:startNum pageStartsOn="BOTH" page="0" pic="0" tbl="0" equation="0"/><hp:visibility hideFirstHeader="0" hideFirstFooter="0" hideFirstMasterPage="0" border="SHOW_ALL" fill="SHOW_ALL" hideFirstPageNum="0" hideFirstEmptyLine="0" showLineNumber="0"/><hp:lineNumberShape restartType="0" countBy="0" distance="0" startNumber="0"/><hp:pagePr landscape="WIDELY" width="{w}" height="{h}" gutterType="{gutter}"><hp:margin header="{header}" footer="{footer}" gutter="0" left="{left}" right="{right}" top="{top}" bottom="{bottom}"/></hp:pagePr><hp:footNotePr><hp:autoNumFormat type="DIGIT" userChar="" prefixChar="" suffixChar=")" supscript="0"/><hp:noteLine length="-1" type="SOLID" width="0.12 mm" color="#000000"/><hp:noteSpacing betweenNotes="283" belowLine="567" aboveLine="850"/><hp:numbering type="CONTINUOUS" newNum="1"/><hp:placement place="EACH_COLUMN" beneathText="0"/></hp:footNotePr><hp:endNotePr><hp:autoNumFormat type="DIGIT" userChar="" prefixChar="" suffixChar=")" supscript="0"/><hp:noteLine length="14692344" type="SOLID" width="0.12 mm" color="#000000"/><hp:noteSpacing betweenNotes="0" belowLine="567" aboveLine="850"/><hp:numbering type="CONTINUOUS" newNum="1"/><hp:placement place="END_OF_DOCUMENT" beneathText="0"/></hp:endNotePr><hp:pageBorderFill type="BOTH" borderFillIDRef="1" textBorder="PAPER" headerInside="0" footerInside="0" fillArea="PAPER"><hp:offset left="1417" right="1417" top="1417" bottom="1417"/></hp:pageBorderFill><hp:pageBorderFill type="EVEN" borderFillIDRef="1" textBorder="PAPER" headerInside="0" footerInside="0" fillArea="PAPER"><hp:offset left="1417" right="1417" top="1417" bottom="1417"/></hp:pageBorderFill><hp:pageBorderFill type="ODD" borderFillIDRef="1" textBorder="PAPER" headerInside="0" footerInside="0" fillArea="PAPER"><hp:offset left="1417" right="1417" top="1417" bottom="1417"/></hp:pageBorderFill></hp:secPr><hp:ctrl><hp:colPr id="" type="NEWSPAPER" layout="LEFT" colCount="1" sameSz="1" sameGap="0"/></hp:ctrl>{page_num}"##,
        w = hu_mm(w),
        h = hu_mm(h),
        header = hu_mm(m.header),
        footer = hu_mm(m.footer),
        left = hu_mm(m.inside),
        right = hu_mm(m.outside),
        top = hu_mm(m.top),
        bottom = hu_mm(m.bottom),
    )
}

/// Writes 머리말 controls.
struct Heads {
    next_id: u32,
    /// Text area width and header height in HWPUNIT.
    width: i64,
    height: i64,
    odd_para: usize,
    even_para: usize,
    facing: bool,
}

impl Heads {
    fn new(format: &ManuscriptFormat) -> Self {
        let (w, _) = format.page();
        let m = format.page_margins();
        let para = |align: HeadAlign| match align {
            HeadAlign::Left => PARA_HEAD_LEFT,
            HeadAlign::Center => PARA_HEAD_CENTER,
            HeadAlign::Right | HeadAlign::Outside => PARA_HEAD_RIGHT,
        };
        let align = format.header.align;
        Heads {
            next_id: 1,
            width: hu_mm(w - m.inside - m.outside),
            height: hu_mm(m.header),
            odd_para: para(align),
            even_para: if align == HeadAlign::Outside {
                PARA_HEAD_LEFT
            } else {
                para(align)
            },
            facing: align == HeadAlign::Outside,
        }
    }

    fn ctrl(&mut self, w: &mut ParagraphWriter, pages: &str, para_pr: usize, text: &str) -> String {
        let id = self.next_id;
        self.next_id += 1;
        let p_id = w.take_id();
        format!(
            r#"<hp:ctrl><hp:header id="{id}" applyPageType="{pages}"><hp:subList id="" textDirection="HORIZONTAL" lineWrap="BREAK" vertAlign="TOP" linkListIDRef="0" linkListNextIDRef="0" textWidth="{}" textHeight="{}" hasTextRef="0" hasNumRef="0"><hp:p id="{p_id}" paraPrIDRef="{para_pr}" styleIDRef="{STYLE_HEAD}" pageBreak="0" columnBreak="0" merged="0"><hp:run charPrIDRef="{CHAR_HEAD}"><hp:t>{}</hp:t></hp:run></hp:p></hp:subList></hp:header></hp:ctrl>"#,
            self.width,
            self.height,
            t_content(text)
        )
    }

    /// The same text on every page: one control, or one per side when the
    /// place differs on even and odd pages.
    fn both(&mut self, w: &mut ParagraphWriter, text: &str) -> String {
        if self.facing {
            let even = self.ctrl(w, "EVEN", self.even_para, text);
            let odd = self.ctrl(w, "ODD", self.odd_para, text);
            format!("{even}{odd}")
        } else {
            self.ctrl(w, "BOTH", self.odd_para, text)
        }
    }
}

const HIDE_HEAD: &str = r#"<hp:ctrl><hp:pageHiding hideHeader="1" hideFooter="0" hideMasterPage="0" hideBorder="0" hideFill="0" hidePageNum="0"/></hp:ctrl>"#;

/// Returns section0.xml and the preview text.
fn section(
    docs: &[ExportDoc],
    format: &ManuscriptFormat,
    opts: &DocOptions,
    info: &DocInfo,
    char_id: &dyn Fn(RunStyle) -> usize,
    para_id: &dyn Fn(ParaAttrs) -> usize,
) -> (String, String) {
    let mut w = ParagraphWriter {
        out: format!("{XML_DECL}<hs:sec {NAMESPACES}>"),
        next_id: 0,
        pending_sec: Some(sec_pr(format)),
        pending_ctrls: String::new(),
    };
    let head = &format.header;
    let mut heads = Heads::new(format);
    let skip_first = head.is_on() && head.skip_chapter_first && format.chapter_new_page;
    let mut preview = String::new();
    let mut add_preview = |text: &str| {
        if preview.chars().count() < 1000 {
            preview.push_str(text);
            preview.push_str("\r\n");
        }
    };

    for (i, doc) in docs.iter().enumerate() {
        let first = i == 0;
        let mut break_pending = format.chapter_new_page && !first;
        // 머리말 for this chapter goes into its first paragraph.
        if head.is_on() {
            let chapter = if doc.heading.trim().is_empty() {
                info.title.as_str()
            } else {
                doc.heading.trim()
            };
            let mut ctrls = String::new();
            match head.content {
                HeadContent::None => {}
                HeadContent::Chapter => ctrls.push_str(&heads.both(&mut w, chapter)),
                HeadContent::TitleChapter => {
                    if first {
                        let para = heads.even_para;
                        ctrls.push_str(&heads.ctrl(&mut w, "EVEN", para, &info.title));
                    }
                    let para = heads.odd_para;
                    ctrls.push_str(&heads.ctrl(&mut w, "ODD", para, chapter));
                }
                HeadContent::Title | HeadContent::Author | HeadContent::Custom if first => {
                    let text = match head.content {
                        HeadContent::Author => info.author.clone(),
                        HeadContent::Custom => head.text.clone(),
                        _ => info.title.clone(),
                    };
                    ctrls.push_str(&heads.both(&mut w, &text));
                }
                _ => {}
            }
            if skip_first {
                ctrls.push_str(HIDE_HEAD);
            }
            w.pending_ctrls.push_str(&ctrls);
        }
        if opts.include_titles && !doc.heading.trim().is_empty() {
            let runs = format!(
                r#"<hp:run charPrIDRef="{CHAR_TITLE}"><hp:t>{}</hp:t></hp:run>"#,
                t_content(doc.heading.trim())
            );
            w.paragraph(PARA_TITLE, break_pending, &runs);
            add_preview(doc.heading.trim());
            break_pending = false;
        } else if !first && !format.chapter_new_page {
            w.paragraph(0, false, "");
        }
        for block in &doc.blocks {
            let page_break = std::mem::take(&mut break_pending);
            match block {
                Block::SceneBreak {} => {
                    let runs = format!(
                        r#"<hp:run charPrIDRef="0"><hp:t>{}</hp:t></hp:run>"#,
                        t_content(&opts.scene_break)
                    );
                    w.paragraph(PARA_SCENE, page_break, &runs);
                    add_preview(&opts.scene_break);
                }
                Block::Paragraph { attrs, content } => {
                    // Line breaks go inside the text of the run before them.
                    let mut runs: Vec<(usize, String)> = Vec::new();
                    let mut plain = String::new();
                    for piece in pieces(content) {
                        match piece {
                            Piece::Text(text, style) => {
                                plain.push_str(&text);
                                runs.push((char_id(style), t_content(&text)));
                            }
                            Piece::Break => {
                                plain.push(' ');
                                match runs.last_mut() {
                                    Some((_, t)) => t.push_str("<hp:lineBreak/>"),
                                    None => runs.push((0, "<hp:lineBreak/>".into())),
                                }
                            }
                        }
                    }
                    let runs: String = runs
                        .into_iter()
                        .map(|(id, t)| {
                            format!(r#"<hp:run charPrIDRef="{id}"><hp:t>{t}</hp:t></hp:run>"#)
                        })
                        .collect();
                    w.paragraph(para_id(*attrs), page_break, &runs);
                    add_preview(&plain);
                }
            }
        }
        // An empty chapter still needs a paragraph for its controls.
        if !w.pending_ctrls.is_empty() {
            w.paragraph(0, std::mem::take(&mut break_pending), "");
        }
    }
    if w.pending_sec.is_some() {
        w.paragraph(0, false, "");
    }
    w.out.push_str("</hs:sec>");
    (w.out, preview)
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use super::*;
    use crate::format::builtin;
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
}
