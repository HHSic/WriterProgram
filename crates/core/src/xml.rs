//! Small helpers for writing docx and HWPX files by hand: escaping XML,
//! dates, and the zip package that holds the parts.

use std::io::{Cursor, Write};

use chrono::{SecondsFormat, Utc};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::{Error, Result};

fn allowed(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\r' | '\u{20}'..='\u{D7FF}' | '\u{E000}'..='\u{FFFD}' | '\u{10000}'..='\u{10FFFF}')
}

/// Escapes text content and drops characters XML cannot hold.
pub fn text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars().filter(|c| allowed(*c)) {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            c => out.push(c),
        }
    }
    out
}

/// Escapes an attribute value.
pub fn attr(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars().filter(|c| allowed(*c)) {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            '\n' => out.push_str("&#10;"),
            c => out.push(c),
        }
    }
    out
}

/// The current time as W3CDTF in UTC, to the second (`2026-09-27T10:15:00Z`),
/// as document properties carry it.
pub fn now_w3cdtf() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// Writes a zip package from `(name, content, compressed)` entries, in order.
/// `what` names the kind of file in the error message.
pub fn zip_package<'a>(
    entries: impl IntoIterator<Item = (&'a str, &'a [u8], bool)>,
    what: &str,
) -> Result<Vec<u8>> {
    let failed =
        |e: &dyn std::fmt::Display| Error::Invalid(format!("{what} 파일을 만들지 못함 ({e})"));
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, body, compressed) in entries {
        let method = if compressed {
            CompressionMethod::Deflated
        } else {
            CompressionMethod::Stored
        };
        let options = SimpleFileOptions::default().compression_method(method);
        zip.start_file(name, options)
            .and_then(|_| zip.write_all(body).map_err(Into::into))
            .map_err(|e| failed(&e))?;
    }
    let cursor = zip.finish().map_err(|e| failed(&e))?;
    Ok(cursor.into_inner())
}

#[cfg(test)]
mod tests {
    #[test]
    fn escapes() {
        assert_eq!(
            super::text("<상태창> & 레벨\u{1}"),
            "&lt;상태창&gt; &amp; 레벨"
        );
        assert_eq!(
            super::attr("\"따옴표\" 'a'"),
            "&quot;따옴표&quot; &apos;a&apos;"
        );
    }
}
