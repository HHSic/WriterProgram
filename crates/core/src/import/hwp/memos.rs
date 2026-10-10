//! HWP 5 memo lists live after the last section's main paragraphs.
use super::{
    BROKEN,
    records::{u16_at, walk},
};
use crate::notes::NewNote;

fn text(data: &[u8]) -> Result<String, String> {
    if !data.len().is_multiple_of(2) {
        return Err(BROKEN.into());
    }
    let mut units = Vec::new();
    let mut at = 0;
    while at < data.len() {
        let value = u16_at(data, at)?;
        match value {
            1..=9 | 11..=12 | 14..=23 => {
                if u16_at(data, at + 14)? != value {
                    return Err(BROKEN.into());
                }
                if value == 9 {
                    units.push(9);
                }
                at += 16;
            }
            0 | 13 | 25..=29 => at += 2,
            10 => {
                units.push(10);
                at += 2;
            }
            24 => {
                units.push(45);
                at += 2;
            }
            30 => {
                units.push(0x00a0);
                at += 2;
            }
            31 => {
                units.push(0x2002);
                at += 2;
            }
            _ => {
                units.push(value);
                at += 2;
            }
        }
    }
    String::from_utf16(&units).map_err(|_| BROKEN.into())
}
pub(super) fn read(data: &[u8]) -> Result<Vec<NewNote>, String> {
    let mut notes = Vec::new();
    let mut level = None;
    let mut paragraph = None;
    let mut body = String::new();
    let flush = |body: &mut String, notes: &mut Vec<NewNote>| {
        if !body.trim().is_empty() {
            notes.push(crate::import::annotations::make(body.trim(), "", "", ""));
        }
        body.clear();
    };
    walk(data, |r| {
        if r.tag == 93 {
            flush(&mut body, &mut notes);
            level = Some(r.level);
            paragraph = None;
        } else if let Some(memo_level) = level {
            if r.level < memo_level {
                flush(&mut body, &mut notes);
                level = None;
                paragraph = None;
            } else if r.tag == 92 {
                flush(&mut body, &mut notes);
                paragraph = None;
            } else if r.tag == 66 {
                paragraph = Some(r.level);
                if !body.is_empty() {
                    body.push('\n');
                }
            } else if r.tag == 67 && paragraph.is_some_and(|p| r.level == p + 1) {
                body.push_str(&text(r.data)?);
            }
        }
        Ok(())
    })?;
    flush(&mut body, &mut notes);
    Ok(notes)
}
