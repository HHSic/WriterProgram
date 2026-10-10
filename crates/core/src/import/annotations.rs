//! Preserve embedded comments independently of unsupported-object notices.
use super::{ImportOptions, Raw, marked};
use crate::notes::{Anchor, NewNote};

pub(super) fn read(bytes: &[u8], ext: &str, _opts: &ImportOptions) -> Result<Raw, String> {
    let mut raw = if ext == "docx" {
        super::word::read(bytes)?
    } else {
        super::hangul::read(bytes)?
    };
    let marked = marked::read_bytes(bytes, ext, "◆")?;
    raw.notes = marked
        .notes
        .iter()
        .map(|note| {
            let quote = note
                .on
                .and_then(|(start, end)| marked.cells.get(start..end))
                .map(|cells| {
                    cells
                        .iter()
                        .filter(|c| c.tracked.is_none_or(|t| t.edit != marked::Edit::Delete))
                        .map(|c| {
                            if matches!(c.ch, marked::PARA | marked::SCENE) {
                                '\n'
                            } else {
                                c.ch
                            }
                        })
                        .collect::<String>()
                })
                .unwrap_or_default();
            make(&note.text, &note.author, &note.date, &quote)
        })
        .collect();
    Ok(raw)
}
pub(super) fn make(text: &str, author: &str, date: &str, quote: &str) -> NewNote {
    let mut body = text.to_string();
    if !author.is_empty() || !date.is_empty() {
        body.push_str("\n\n— 가져온 메모");
        if !author.is_empty() {
            body.push_str(&format!(" · 작성자: {author}"));
        }
        if !date.is_empty() {
            body.push_str(&format!(" · 원문 날짜: {date}"));
        }
    }
    NewNote {
        id: None,
        anchor: Anchor::Doc,
        target: String::new(),
        quote: quote.trim().into(),
        text: body,
    }
}
