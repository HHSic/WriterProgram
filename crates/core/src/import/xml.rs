//! Reading the XML parts of docx and HWPX files: a token stream with local
//! names, entities already turned into text.

use std::io::{Cursor, Read};

use quick_xml::Reader;
use quick_xml::escape::resolve_xml_entity;
use quick_xml::events::{BytesRef, BytesStart, Event};
use zip::ZipArchive;

/// Largest XML part read, against files made to swell.
const PART_LIMIT: u64 = 256 * 1024 * 1024;

pub(super) type Attrs = Vec<(String, String)>;

pub(super) enum Tok {
    Start(String, Attrs),
    Empty(String, Attrs),
    End(String),
    /// Text, CDATA, or what an entity or character reference stands for.
    Text(String),
}

fn local(name: &str) -> String {
    name.rsplit(':').next().unwrap_or_default().to_string()
}

fn attrs(e: &BytesStart<'_>) -> Attrs {
    e.attributes()
        .flatten()
        .map(|a| {
            (
                local(a.key.as_ref()),
                a.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                    .map(|v| v.into_owned())
                    .unwrap_or_default(),
            )
        })
        .collect()
}

pub(super) fn val<'a>(attrs: &'a Attrs, key: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

/// What `&amp;`, `&#44032;` or `&#xAC00;` stands for. Unknown entities and
/// numbers that are no character give nothing. Character references are read
/// by hand: quick-xml's own reading turns away `&#X…;`, `&#0;` and `&#+…;`,
/// which these readers have always taken.
fn reference(r: &BytesRef<'_>) -> Option<String> {
    let name: &str = r;
    if let Some(text) = resolve_xml_entity(name) {
        return Some(text.to_string());
    }
    let n = name.strip_prefix('#')?;
    match n.strip_prefix(['x', 'X']) {
        Some(hex) => u32::from_str_radix(hex, 16).ok(),
        None => n.parse().ok(),
    }
    .and_then(char::from_u32)
    .map(String::from)
}

/// Calls `f` for each token of an XML part.
pub(super) fn walk(xml: &str, mut f: impl FnMut(Tok)) -> Result<(), String> {
    let mut reader = Reader::from_str(xml);
    loop {
        let event = reader
            .read_event()
            .map_err(|_| "Word 문서의 내용이 올바르지 않음".to_string())?;
        match event {
            Event::Start(e) => f(Tok::Start(local(e.name().as_ref()), attrs(&e))),
            Event::Empty(e) => f(Tok::Empty(local(e.name().as_ref()), attrs(&e))),
            Event::End(e) => f(Tok::End(local(e.name().as_ref()))),
            Event::Text(t) => f(Tok::Text(t.to_string())),
            Event::CData(t) => f(Tok::Text(t.to_string())),
            Event::GeneralRef(r) => {
                if let Some(text) = reference(&r) {
                    f(Tok::Text(text));
                }
            }
            Event::Eof => return Ok(()),
            _ => {}
        }
    }
}

/// Reads one part of a zip package; none when the package has no such part.
pub(super) fn read_part(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    name: &str,
) -> Option<Result<String, String>> {
    let file = archive.by_name(name).ok()?;
    let mut xml = String::new();
    let read = file.take(PART_LIMIT).read_to_string(&mut xml);
    Some(
        read.map(|_| xml)
            .map_err(|_| "Word 문서를 읽지 못함".to_string()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(xml: &str) -> Vec<String> {
        let mut out = Vec::new();
        walk(xml, |tok| {
            if let Tok::Text(t) = tok {
                out.push(t);
            }
        })
        .unwrap();
        out
    }

    #[test]
    fn references_become_text() {
        assert_eq!(
            texts("<t>a &amp; &lt;b&gt; &quot;&apos; &#44032;&#xAC01;&#XAC02;</t>").concat(),
            "a & <b> \"' 가각갂"
        );
        // Unknown entities and numbers that are no character are left out.
        assert_eq!(texts("<t>&nbsp;&#xD800;&#xZZ;x</t>").concat(), "x");
    }
}
