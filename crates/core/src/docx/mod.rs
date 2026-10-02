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

mod body;
mod layout;
mod styles;
#[cfg(test)]
mod tests;

use body::document;
use layout::Layout;
use styles::{settings, styles};

use crate::export::{DocInfo, DocOptions, ExportDoc};
use crate::format::ManuscriptFormat;
use crate::{Result, xml};

const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
/// The relationships namespace, which is also where relationship types live.
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
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
    xml::zip_package(
        parts
            .iter()
            .map(|(name, body)| (name.as_str(), body.as_bytes(), true)),
        "docx",
    )
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
    let now = xml::now_w3cdtf();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:dcmitype="http://purl.org/dc/dcmitype/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><dc:title>{}</dc:title><dc:creator>{}</dc:creator><dcterms:created xsi:type="dcterms:W3CDTF">{now}</dcterms:created><dcterms:modified xsi:type="dcterms:W3CDTF">{now}</dcterms:modified></cp:coreProperties>"#,
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
                r#"<Relationship Id="{}" Type="{R}/{}" Target="{}"/>"#,
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
