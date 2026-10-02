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

mod header;
mod section;
#[cfg(test)]
mod tests;

use header::{CHAR_RUNS, PARA_MARGINS, header};
use section::section;

use crate::export::{DocInfo, DocOptions, ExportDoc, Piece, RunStyle, pieces};
use crate::format::ManuscriptFormat;
use crate::markup::{Block, ParaAttrs};
use crate::{Result, xml};

const NAMESPACES: &str = r#"xmlns:ha="http://www.hancom.co.kr/hwpml/2011/app" xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph" xmlns:hp10="http://www.hancom.co.kr/hwpml/2016/paragraph" xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section" xmlns:hc="http://www.hancom.co.kr/hwpml/2011/core" xmlns:hh="http://www.hancom.co.kr/hwpml/2011/head" xmlns:hhs="http://www.hancom.co.kr/hwpml/2011/history" xmlns:hm="http://www.hancom.co.kr/hwpml/2011/master-page" xmlns:hpf="http://www.hancom.co.kr/schema/2011/hpf" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:opf="http://www.idpf.org/2007/opf/" xmlns:ooxmlchart="http://www.hancom.co.kr/hwpml/2016/ooxmlchart" xmlns:hwpunitchar="http://www.hancom.co.kr/hwpml/2016/HwpUnitChar" xmlns:epub="http://www.idpf.org/2007/ops" xmlns:config="urn:oasis:names:tc:opendocument:xmlns:config:1.0""#;

const XML_DECL: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes" ?>"#;

/// HWPUNIT: 1/7200 inch. 1pt = 100.
fn hu_mm(mm: f64) -> i64 {
    (mm * 7200.0 / 25.4).round() as i64
}

fn hu_pt(pt: f64) -> i64 {
    (pt * 100.0).round() as i64
}

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
    // The mimetype entry must come first and stay uncompressed.
    xml::zip_package(
        parts
            .iter()
            .map(|(name, body, deflate)| (*name, body.as_slice(), *deflate)),
        "한글",
    )
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
    let now = xml::now_w3cdtf();
    format!(
        r#"{XML_DECL}<opf:package {NAMESPACES} version="" unique-identifier="" id=""><opf:metadata><opf:title>{title}</opf:title><opf:language>ko</opf:language><opf:meta name="creator" content="text">{author}</opf:meta><opf:meta name="subject" content="text"/><opf:meta name="description" content="text"/><opf:meta name="lastsaveby" content="text">{author}</opf:meta><opf:meta name="CreatedDate" content="text">{now}</opf:meta><opf:meta name="ModifiedDate" content="text">{now}</opf:meta><opf:meta name="keyword" content="text"/></opf:metadata><opf:manifest><opf:item id="header" href="Contents/header.xml" media-type="application/xml"/><opf:item id="section0" href="Contents/section0.xml" media-type="application/xml"/><opf:item id="settings" href="settings.xml" media-type="application/xml"/></opf:manifest><opf:spine><opf:itemref idref="header" linear="yes"/><opf:itemref idref="section0" linear="yes"/></opf:spine></opf:package>"#,
        title = xml::text(&info.title),
        author = xml::text(&info.author),
    )
}
