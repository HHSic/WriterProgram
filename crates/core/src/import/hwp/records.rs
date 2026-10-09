//! Checked HWP 5 record framing. Never accept a truncated record as partial text.

use super::BROKEN;

pub(super) struct Record<'a> {
    pub tag: u16,
    pub level: u16,
    pub data: &'a [u8],
}

pub(super) fn u16_at(data: &[u8], at: usize) -> Result<u16, String> {
    let bytes = data.get(at..at + 2).ok_or(BROKEN)?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

pub(super) fn u32_at(data: &[u8], at: usize) -> Result<u32, String> {
    let bytes = data.get(at..at + 4).ok_or(BROKEN)?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

pub(super) fn walk(
    data: &[u8],
    mut visit: impl FnMut(Record<'_>) -> Result<(), String>,
) -> Result<(), String> {
    let mut offset = 0;
    let mut count = 0;
    while offset < data.len() {
        count += 1;
        if count > 1_000_000 {
            return Err("한글 문서의 레코드가 너무 많음".into());
        }
        let header = u32_at(data, offset)?;
        offset += 4;
        let mut size = (header >> 20) as usize;
        if size == 0xfff {
            size = u32_at(data, offset)? as usize;
            offset += 4;
        }
        let end = offset
            .checked_add(size)
            .filter(|&end| end <= data.len())
            .ok_or(BROKEN)?;
        visit(Record {
            tag: (header & 0x3ff) as u16,
            level: ((header >> 10) & 0x3ff) as u16,
            data: &data[offset..end],
        })?;
        offset = end;
    }
    Ok(())
}

pub(super) fn wide_string(data: &[u8], offset: &mut usize) -> Result<String, String> {
    let length = usize::from(u16_at(data, *offset)?);
    *offset += 2;
    let end = offset
        .checked_add(length * 2)
        .filter(|&end| end <= data.len())
        .ok_or(BROKEN)?;
    let units = data[*offset..end]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .collect::<Vec<_>>();
    *offset = end;
    String::from_utf16(&units).map_err(|_| BROKEN.into())
}
