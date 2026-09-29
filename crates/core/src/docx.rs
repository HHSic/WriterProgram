//! Word (.docx) export with the manuscript format applied.
//!
//! Line spacing is written as an exact pitch (type size × line spacing %), the
//! way 한글 measures it, so a manuscript looks the same in Word and 한글.
//! Word keeps Korean words whole at line ends by default (its `wordWrap`
//! paragraph setting left on), so that setting is not written.
//!
//! 머리말 with the chapter's title uses a STYLEREF field on the chapter title
//! style, which Word fills in page by page. Leaving 머리말 off each chapter's
//! first page needs a section per chapter with a different first page.

use std::io::{Cursor, Write};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::export::{DocInfo, DocOptions, ExportDoc, Piece, RunStyle, pieces};
use crate::format::{HeadAlign, HeadContent, ManuscriptFormat, font};
use crate::markup::{Block, ParaAttrs};
use crate::store::now_iso;
use crate::{Error, Result, xml};

const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const CT_HEADER: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml";
const CT_FOOTER: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml";
/// Name of the chapter title style, which STYLEREF looks for.
const CHAPTER_STYLE_NAME: &str = "장 제목";

fn twips_mm(mm: f64) -> i64 {
    (mm * 1440.0 / 25.4).round() as i64
}

fn twips_pt(pt: f64) -> i64 {
    (pt * 20.0).round() as i64
}

/// Header and footer parts: file name, relationship id, and for which pages.
#[derive(Clone, Copy, PartialEq)]
enum PageType {
    Default,
    Even,
    First,
}

impl PageType {
    fn attr(self) -> &'static str {
        match self {
            PageType::Default => "default",
            PageType::Even => "even",
            PageType::First => "first",
        }
    }
}

struct Part {
    header: bool,
    page: PageType,
    file: String,
    rel: String,
    xml: String,
}

/// What the package holds besides the body.
struct Layout {
    parts: Vec<Part>,
    even_odd: bool,
    /// Each chapter is a section whose first page has no 머리말.
    sections: bool,
}

impl Layout {
    fn new(
        docs: &[ExportDoc],
        format: &ManuscriptFormat,
        opts: &DocOptions,
        info: &DocInfo,
    ) -> Self {
        let head = &format.header;
        let even_odd = head.facing() || format.footer_facing();
        let sections = head.is_on() && head.skip_chapter_first && format.chapter_new_page;
        let mut parts = Vec::new();
        let mut add = |header: bool, page: PageType, xml: String| {
            let n = parts.len() + 3;
            let kind = if header { "header" } else { "footer" };
            parts.push(Part {
                header,
                page,
                file: format!("{kind}{n}.xml"),
                rel: format!("rId{n}"),
                xml,
            });
        };
        if format.page_numbers || format.footer.is_on() {
            add(false, PageType::Default, footer(format, false));
            if even_odd {
                add(false, PageType::Even, footer(format, true));
            }
            if sections {
                add(false, PageType::First, footer(format, false));
            }
        }
        if head.is_on() {
            // The chapter's title comes from its title paragraphs; without
            // them the work's title stands in.
            let first_heading = docs
                .iter()
                .map(|d| d.heading.trim())
                .find(|h| !h.is_empty())
                .unwrap_or_default();
            let chapter = if opts.include_titles && !first_heading.is_empty() {
                HeadText::Chapter(first_heading.to_string())
            } else {
                HeadText::Plain(info.title.clone())
            };
            let fixed = match head.content {
                HeadContent::Author => info.author.clone(),
                HeadContent::Custom => head.text.clone(),
                _ => info.title.clone(),
            };
            let (odd_text, even_text) = match head.content {
                HeadContent::Chapter => (chapter.clone(), chapter),
                HeadContent::TitleChapter => (chapter, HeadText::Plain(info.title.clone())),
                _ => (HeadText::Plain(fixed.clone()), HeadText::Plain(fixed)),
            };
            let (odd_jc, even_jc) = match head.align {
                HeadAlign::Left => ("left", "left"),
                HeadAlign::Center => ("center", "center"),
                HeadAlign::Right => ("right", "right"),
                HeadAlign::Outside => ("right", "left"),
            };
            add(true, PageType::Default, header_part(&odd_text, odd_jc));
            if even_odd {
                add(true, PageType::Even, header_part(&even_text, even_jc));
            }
            if sections {
                add(
                    true,
                    PageType::First,
                    header_part(&HeadText::Plain(String::new()), "left"),
                );
            }
        }
        Layout {
            parts,
            even_odd,
            sections,
        }
    }

    /// References for a section's properties.
    fn references(&self) -> String {
        self.parts
            .iter()
            .map(|p| {
                format!(
                    r#"<w:{kind}Reference w:type="{page}" r:id="{rel}"/>"#,
                    kind = if p.header { "header" } else { "footer" },
                    page = p.page.attr(),
                    rel = p.rel,
                )
            })
            .collect()
    }
}

#[derive(Clone)]
enum HeadText {
    Plain(String),
    /// The chapter title, via STYLEREF; the text is what shows before Word updates it.
    Chapter(String),
}

fn header_part(text: &HeadText, jc: &str) -> String {
    let runs = match text {
        HeadText::Plain(t) if t.is_empty() => String::new(),
        HeadText::Plain(t) => text_run(t, RunStyle::default()),
        HeadText::Chapter(shown) => format!(
            r#"<w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText xml:space="preserve"> STYLEREF "{CHAPTER_STYLE_NAME}" \* MERGEFORMAT </w:instrText></w:r><w:r><w:fldChar w:fldCharType="separate"/></w:r>{}<w:r><w:fldChar w:fldCharType="end"/></w:r>"#,
            text_run(shown, RunStyle::default())
        ),
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:hdr xmlns:w="{W}" xmlns:r="{R}"><w:p><w:pPr><w:pStyle w:val="Header"/><w:jc w:val="{jc}"/></w:pPr>{runs}</w:p></w:hdr>"#
    )
}

/// Builds the .docx file in memory.
pub fn docx_bytes(
    docs: &[ExportDoc],
    format: &ManuscriptFormat,
    opts: &DocOptions,
    info: &DocInfo,
) -> Result<Vec<u8>> {
    let layout = Layout::new(docs, format, opts, info);
    let mut parts: Vec<(String, String)> = vec![
        ("[Content_Types].xml".into(), content_types(&layout)),
        ("_rels/.rels".into(), root_rels()),
        ("docProps/core.xml".into(), core_props(info)),
        ("docProps/app.xml".into(), app_props()),
        (
            "word/_rels/document.xml.rels".into(),
            document_rels(&layout),
        ),
        (
            "word/document.xml".into(),
            document(docs, format, opts, &layout),
        ),
        ("word/styles.xml".into(), styles(format)),
        ("word/settings.xml".into(), settings(format, &layout)),
    ];
    for part in &layout.parts {
        parts.push((format!("word/{}", part.file), part.xml.clone()));
    }

    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for (name, body) in parts {
        zip.start_file(name, options)
            .and_then(|_| zip.write_all(body.as_bytes()).map_err(Into::into))
            .map_err(|e| Error::Invalid(format!("docx 파일을 만들지 못함 ({e})")))?;
    }
    let cursor = zip
        .finish()
        .map_err(|e| Error::Invalid(format!("docx 파일을 만들지 못함 ({e})")))?;
    Ok(cursor.into_inner())
}

fn content_types(layout: &Layout) -> String {
    let parts: String = layout
        .parts
        .iter()
        .map(|p| {
            format!(
                r#"<Override PartName="/word/{}" ContentType="{}"/>"#,
                p.file,
                if p.header { CT_HEADER } else { CT_FOOTER }
            )
        })
        .collect();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/settings.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml"/>{parts}<Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/><Override PartName="/docProps/app.xml" ContentType="application/vnd.openxmlformats-officedocument.extended-properties+xml"/></Types>"#
    )
}

fn root_rels() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties" Target="docProps/app.xml"/></Relationships>"#
        .into()
}

fn core_props(info: &DocInfo) -> String {
    let now = now_iso();
    let now = now.split('.').next().unwrap_or(&now);
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:dcmitype="http://purl.org/dc/dcmitype/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><dc:title>{}</dc:title><dc:creator>{}</dc:creator><dcterms:created xsi:type="dcterms:W3CDTF">{now}Z</dcterms:created><dcterms:modified xsi:type="dcterms:W3CDTF">{now}Z</dcterms:modified></cp:coreProperties>"#,
        xml::text(&info.title),
        xml::text(&info.author),
    )
}

fn app_props() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"><Application>WriterProgram</Application></Properties>"#
        .into()
}

fn document_rels(layout: &Layout) -> String {
    let parts: String = layout
        .parts
        .iter()
        .map(|p| {
            format!(
                r#"<Relationship Id="{}" Type="{REL}/{}" Target="{}"/>"#,
                p.rel,
                if p.header { "header" } else { "footer" },
                p.file
            )
        })
        .collect();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings" Target="settings.xml"/>{parts}</Relationships>"#
    )
}

/// Line pitch of body text in twips.
fn pitch(format: &ManuscriptFormat) -> i64 {
    twips_pt(format.size_pt * f64::from(format.line_spacing) / 100.0)
}

fn title_size(format: &ManuscriptFormat) -> f64 {
    (format.size_pt * 1.6 * 2.0).round() / 2.0
}

fn styles(format: &ManuscriptFormat) -> String {
    let face = xml::attr(font(&format.font).docx);
    let half_points = (format.size_pt * 2.0).round() as i64;
    let pitch = pitch(format);
    let after = if format.blank_line_between { pitch } else { 0 };
    let indent_chars = (format.indent * 100.0).round() as i64;
    let indent_twips = twips_pt(format.indent * format.size_pt);
    let letter = twips_pt(format.size_pt * f64::from(format.letter_spacing) / 100.0);
    let title_half = (title_size(format) * 2.0) as i64;
    let title_pitch = twips_pt(title_size(format) * 1.4);
    let break_before = if format.chapter_new_page {
        "<w:pageBreakBefore/>"
    } else {
        ""
    };
    let title_before = if format.chapter_new_page {
        pitch * 3
    } else {
        pitch
    };
    let scene_space = if format.blank_line_between { 0 } else { pitch };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="{W}"><w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii="{face}" w:hAnsi="{face}" w:eastAsia="{face}" w:cs="{face}"/><w:sz w:val="{half_points}"/><w:szCs w:val="{half_points}"/><w:lang w:val="ko-KR" w:eastAsia="ko-KR" w:bidi="ar-SA"/></w:rPr></w:rPrDefault><w:pPrDefault><w:pPr><w:spacing w:after="0" w:line="{pitch}" w:lineRule="exact"/></w:pPr></w:pPrDefault></w:docDefaults><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/><w:qFormat/><w:pPr><w:spacing w:after="{after}" w:line="{pitch}" w:lineRule="exact"/><w:ind w:firstLineChars="{indent_chars}" w:firstLine="{indent_twips}"/><w:jc w:val="both"/></w:pPr><w:rPr><w:spacing w:val="{letter}"/></w:rPr></w:style><w:style w:type="paragraph" w:customStyle="1" w:styleId="WPChapter"><w:name w:val="{CHAPTER_STYLE_NAME}"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:qFormat/><w:pPr><w:keepNext/>{break_before}<w:spacing w:before="{title_before}" w:after="{title_after}" w:line="{title_pitch}" w:lineRule="exact"/><w:ind w:firstLineChars="0" w:firstLine="0"/><w:jc w:val="left"/></w:pPr><w:rPr><w:b/><w:bCs/><w:sz w:val="{title_half}"/><w:szCs w:val="{title_half}"/></w:rPr></w:style><w:style w:type="paragraph" w:customStyle="1" w:styleId="WPSceneBreak"><w:name w:val="장면 나눔"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:pPr><w:spacing w:before="{scene_space}" w:after="{scene_after}"/><w:ind w:firstLineChars="0" w:firstLine="0"/><w:jc w:val="center"/></w:pPr></w:style><w:style w:type="paragraph" w:styleId="Header"><w:name w:val="header"/><w:basedOn w:val="Normal"/><w:pPr><w:spacing w:after="0" w:line="240" w:lineRule="auto"/><w:ind w:firstLineChars="0" w:firstLine="0"/></w:pPr><w:rPr><w:spacing w:val="0"/><w:sz w:val="{small_half}"/><w:szCs w:val="{small_half}"/></w:rPr></w:style><w:style w:type="paragraph" w:styleId="Footer"><w:name w:val="footer"/><w:basedOn w:val="Normal"/><w:pPr><w:spacing w:after="0" w:line="240" w:lineRule="auto"/><w:ind w:firstLineChars="0" w:firstLine="0"/><w:jc w:val="center"/></w:pPr><w:rPr><w:spacing w:val="0"/><w:sz w:val="{small_half}"/><w:szCs w:val="{small_half}"/></w:rPr></w:style></w:styles>"#,
        title_after = pitch * 2,
        scene_after = scene_space.max(after),
        small_half = (half_points - 2).max(12),
    )
}

fn settings(format: &ManuscriptFormat, layout: &Layout) -> String {
    let m = format.page_margins();
    let mirror = if (m.inside - m.outside).abs() > f64::EPSILON {
        "<w:mirrorMargins/>"
    } else {
        ""
    };
    let even_odd = if layout.even_odd {
        "<w:evenAndOddHeaders/>"
    } else {
        ""
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:settings xmlns:w="{W}"><w:zoom w:percent="100"/>{mirror}<w:defaultTabStop w:val="800"/>{even_odd}<w:characterSpacingControl w:val="doNotCompress"/><w:compat><w:doNotExpandShiftReturn/><w:compatSetting w:name="compatibilityMode" w:uri="http://schemas.microsoft.com/office/word" w:val="15"/></w:compat></w:settings>"#
    )
}

const PAGE_NUMBER: &str = r#"<w:r><w:t xml:space="preserve">- </w:t></w:r><w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText xml:space="preserve"> PAGE </w:instrText></w:r><w:r><w:fldChar w:fldCharType="separate"/></w:r><w:r><w:t>1</w:t></w:r><w:r><w:fldChar w:fldCharType="end"/></w:r><w:r><w:t xml:space="preserve"> -</w:t></w:r>"#;

/// The footer of even or odd pages: the page number and the 꼬리말, each in
/// its place. Two things on one line sit at tab stops (centre and right edge
/// of the text area); one alone is simply aligned.
fn footer(format: &ManuscriptFormat, even: bool) -> String {
    let side = |align: HeadAlign| {
        let (e, o) = align.sides();
        if even { e } else { o }
    };
    // Left, centre, right.
    let mut slots = [String::new(), String::new(), String::new()];
    let slot = |align: HeadAlign| match align {
        HeadAlign::Left => 0,
        HeadAlign::Center => 1,
        _ => 2,
    };
    if format.page_numbers {
        slots[slot(side(format.page_number_align))].push_str(PAGE_NUMBER);
    }
    if format.footer.is_on() {
        slots[slot(side(format.footer.align))]
            .push_str(&text_run(format.footer.text.trim(), RunStyle::default()));
    }
    let filled: Vec<usize> = (0..3).filter(|i| !slots[*i].is_empty()).collect();
    let (ppr, runs) = if filled.len() == 1 {
        let jc = ["left", "center", "right"][filled[0]];
        (
            format!(r#"<w:pStyle w:val="Footer"/><w:jc w:val="{jc}"/>"#),
            slots[filled[0]].clone(),
        )
    } else {
        let (w, _) = format.page();
        let m = format.page_margins();
        let width = twips_mm(w - m.inside - m.outside);
        (
            format!(
                r#"<w:pStyle w:val="Footer"/><w:tabs><w:tab w:val="center" w:pos="{}"/><w:tab w:val="right" w:pos="{width}"/></w:tabs><w:jc w:val="left"/>"#,
                width / 2
            ),
            slots.join("<w:r><w:tab/></w:r>"),
        )
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:ftr xmlns:w="{W}" xmlns:r="{R}"><w:p><w:pPr>{ppr}</w:pPr>{runs}</w:p></w:ftr>"#
    )
}

fn run_props(style: RunStyle) -> String {
    let mut props = String::new();
    if style.bold {
        props.push_str("<w:b/><w:bCs/>");
    }
    if style.italic {
        props.push_str("<w:i/><w:iCs/>");
    }
    if style.strike {
        props.push_str("<w:strike/>");
    }
    if style.underline {
        props.push_str(r#"<w:u w:val="single"/>"#);
    }
    if style.dot {
        props.push_str(r#"<w:em w:val="dot"/>"#);
    }
    if props.is_empty() {
        props
    } else {
        format!("<w:rPr>{props}</w:rPr>")
    }
}

fn text_run(text: &str, style: RunStyle) -> String {
    format!(
        r#"<w:r>{}<w:t xml:space="preserve">{}</w:t></w:r>"#,
        run_props(style),
        xml::text(text)
    )
}

/// A paragraph before it is written: its style, the rest of its properties
/// (in schema order; a section break goes last) and its runs.
#[derive(Default)]
struct Para {
    style: Option<&'static str>,
    ppr: String,
    body: String,
}

impl Para {
    fn xml(&self) -> String {
        let mut ppr = String::new();
        if let Some(style) = self.style {
            ppr.push_str(&format!(r#"<w:pStyle w:val="{style}"/>"#));
        }
        ppr.push_str(&self.ppr);
        match (ppr.is_empty(), self.body.is_empty()) {
            (true, true) => "<w:p/>".into(),
            (true, false) => format!("<w:p>{}</w:p>", self.body),
            _ => format!("<w:p><w:pPr>{ppr}</w:pPr>{}</w:p>", self.body),
        }
    }
}

/// 문단 모양 as Word indents, in characters: margins, and the first line
/// (the paragraph's own, or the format's).
fn margin_ind(format: &ManuscriptFormat, attrs: ParaAttrs) -> String {
    let char_pt = format.size_pt * (1.0 + f64::from(format.letter_spacing) / 100.0);
    let side = |n: u8| (u32::from(n) * 100, twips_pt(f64::from(n) * char_pt));
    let (left_chars, left) = side(attrs.left);
    let (right_chars, right) = side(attrs.right);
    let first = attrs.indent.map_or(format.indent, f64::from);
    let first_line = if first < 0.0 {
        format!(
            r#"w:hangingChars="{}" w:hanging="{}""#,
            (-first * 100.0).round() as i64,
            twips_pt(-first * format.size_pt)
        )
    } else {
        format!(
            r#"w:firstLineChars="{}" w:firstLine="{}""#,
            (first * 100.0).round() as i64,
            twips_pt(first * format.size_pt)
        )
    };
    format!(
        r#"<w:ind w:leftChars="{left_chars}" w:left="{left}" w:rightChars="{right_chars}" w:right="{right}" {first_line}/>"#
    )
}

/// Section properties. Only the first section names its 머리말 and 꼬리말;
/// later ones carry them over.
fn sect_pr(format: &ManuscriptFormat, layout: &Layout, first: bool, new_page: bool) -> String {
    let (width, height) = format.page();
    let m = format.page_margins();
    let refs = if first {
        layout.references()
    } else {
        String::new()
    };
    let kind = if new_page {
        r#"<w:type w:val="nextPage"/>"#
    } else {
        ""
    };
    let title_pg = if layout.sections { "<w:titlePg/>" } else { "" };
    format!(
        r#"<w:sectPr>{refs}{kind}<w:pgSz w:w="{}" w:h="{}"/><w:pgMar w:top="{}" w:right="{}" w:bottom="{}" w:left="{}" w:header="{}" w:footer="{}" w:gutter="0"/><w:cols w:space="425"/>{title_pg}</w:sectPr>"#,
        twips_mm(width),
        twips_mm(height),
        twips_mm(m.top + m.header),
        twips_mm(m.outside),
        twips_mm(m.bottom + m.footer),
        twips_mm(m.inside),
        twips_mm(m.top),
        twips_mm(m.bottom),
    )
}

fn document(
    docs: &[ExportDoc],
    format: &ManuscriptFormat,
    opts: &DocOptions,
    layout: &Layout,
) -> String {
    let sections = layout.sections;
    let mut chapters: Vec<Vec<Para>> = Vec::new();
    for (i, doc) in docs.iter().enumerate() {
        let first = i == 0;
        let mut paras: Vec<Para> = Vec::new();
        // With a section per chapter the section starts the new page.
        let mut break_pending = format.chapter_new_page && !first && !sections;
        if opts.include_titles && !doc.heading.trim().is_empty() {
            // The title style breaks the page itself; the first title must not.
            let ppr = if (first || sections) && format.chapter_new_page {
                r#"<w:pageBreakBefore w:val="0"/>"#
            } else {
                ""
            };
            paras.push(Para {
                style: Some("WPChapter"),
                ppr: ppr.into(),
                body: text_run(doc.heading.trim(), RunStyle::default()),
            });
            break_pending = false;
        } else if !first && !format.chapter_new_page {
            paras.push(Para::default());
        }
        for block in &doc.blocks {
            let mut ppr = String::new();
            if std::mem::take(&mut break_pending) {
                ppr.push_str("<w:pageBreakBefore/>");
            }
            match block {
                Block::SceneBreak {} => paras.push(Para {
                    style: Some("WPSceneBreak"),
                    ppr,
                    body: text_run(&opts.scene_break, RunStyle::default()),
                }),
                Block::Paragraph { attrs, content } => {
                    if !attrs.is_plain() {
                        ppr.push_str(&margin_ind(format, *attrs));
                    }
                    let mut runs = String::new();
                    for piece in pieces(content) {
                        match piece {
                            Piece::Text(text, style) => runs.push_str(&text_run(&text, style)),
                            Piece::Break => runs.push_str("<w:r><w:br/></w:r>"),
                        }
                    }
                    paras.push(Para {
                        style: None,
                        ppr,
                        body: runs,
                    });
                }
            }
        }
        chapters.push(paras);
    }

    // A section break ends every chapter but the last; the last section's
    // properties close the body.
    let count = chapters.len();
    if sections {
        for (i, paras) in chapters
            .iter_mut()
            .enumerate()
            .take(count.saturating_sub(1))
        {
            if paras.is_empty() {
                paras.push(Para::default());
            }
            let last = paras.last_mut().expect("not empty");
            last.ppr.push_str(&sect_pr(format, layout, i == 0, i > 0));
        }
    }
    let body: String = chapters.iter().flatten().map(Para::xml).collect();
    let last_sect = if sections {
        sect_pr(format, layout, count <= 1, count > 1)
    } else {
        sect_pr(format, layout, true, false)
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="{W}" xmlns:r="{R}"><w:body>{body}{last_sect}</w:body></w:document>"#
    )
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use super::*;
    use crate::format::{RunningHead, builtin};
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
        assert!(odd.contains(
            r#"<w:tab w:val="center" w:pos="4252"/><w:tab w:val="right" w:pos="8504"/>"#
        ));
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
}
