//! HWP 5.x binary import, from the complete BodyText streams (never PrvText).
//! Uses CFB and raw DEFLATE; no Hancom installation or external converter.

mod body;
mod memos;
mod records;
mod styles;

use std::fs::File;
use std::io::{Read, Seek};
use std::path::Path;

use cfb::CompoundFile;
use flate2::{Crc, Decompress, FlushDecompress, Status};

use super::Raw;
use records::u32_at;

pub(super) const BROKEN: &str = "한글 문서(HWP)가 아니거나 손상됨";
const LIMIT: usize = 64 * 1024 * 1024;
const FILE_LIMIT: u64 = 256 * 1024 * 1024;

fn stream<R: Read + Seek>(
    file: &mut CompoundFile<R>,
    name: &str,
    compressed: bool,
    remaining: &mut usize,
) -> Result<Vec<u8>, String> {
    let mut input = file.open_stream(name).map_err(|_| BROKEN)?;
    if input.len() > LIMIT as u64 {
        return Err("한글 문서의 데이터가 너무 큼".into());
    }
    let mut bytes = Vec::new();
    input
        .by_ref()
        .take(LIMIT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| BROKEN)?;
    if compressed {
        // Fixed output bound, including across sections. Check StreamEnd so a
        // truncated deflate stream can never produce a deceptively complete import.
        let mut decoder = Decompress::new(false);
        let mut output = Vec::new();
        loop {
            let mut chunk = [0u8; 8192];
            let before_in = decoder.total_in();
            let before_out = decoder.total_out();
            let status = decoder
                .decompress(
                    &bytes[before_in as usize..],
                    &mut chunk,
                    FlushDecompress::None,
                )
                .map_err(|_| BROKEN)?;
            let written = (decoder.total_out() - before_out) as usize;
            if output
                .len()
                .checked_add(written)
                .is_none_or(|size| size > *remaining)
            {
                return Err("한글 문서의 압축을 푼 내용이 너무 큼 (최대 64MB)".into());
            }
            output.extend_from_slice(&chunk[..written]);
            if status == Status::StreamEnd {
                // Some Hancom writers retain the gzip CRC32/ISIZE footer after
                // the raw deflate stream. Accept it only when both checks match.
                let trailer = &bytes[decoder.total_in() as usize..];
                if !trailer.is_empty() {
                    let mut crc = Crc::new();
                    crc.update(&output);
                    if trailer.len() != 8
                        || u32_at(trailer, 0)? != crc.sum()
                        || u32_at(trailer, 4)? != output.len() as u32
                    {
                        return Err(BROKEN.into());
                    }
                }
                bytes = output;
                break;
            }
            if decoder.total_in() == before_in && written == 0 {
                return Err(BROKEN.into());
            }
        }
    }
    *remaining = remaining
        .checked_sub(bytes.len())
        .ok_or("한글 문서의 내용이 너무 큼 (최대 64MB)")?;
    Ok(bytes)
}

pub(super) fn read(path: &Path) -> Result<Raw, String> {
    let mut file = File::open(path).map_err(|e| crate::Error::io(path, e).user_message())?;
    let length = file.metadata().map_err(|_| BROKEN)?.len();
    if length > FILE_LIMIT {
        return Err("한글 파일이 너무 큼 (최대 256MB)".into());
    }
    let mut magic = [0; 30];
    let read = file.read(&mut magic).map_err(|_| BROKEN)?;
    if magic[..read].starts_with(b"HWP Document File V3.00") {
        return Err("한글 97 문서(HWP 3.x)는 아직 읽을 수 없음 · 한글에서 HWP 5.x 또는 HWPX로 저장해 주세요".into());
    }
    file.rewind().map_err(|_| BROKEN)?;
    let mut compound = CompoundFile::open(file).map_err(|_| BROKEN)?;
    let mut remaining = LIMIT;
    let header = stream(&mut compound, "/FileHeader", false, &mut remaining)?;
    if header.len() != 256 || !header.starts_with(b"HWP Document File\0") {
        return Err(BROKEN.into());
    }
    if header[35] != 5 {
        return Err("HWP 5.x 문서만 가져올 수 있습니다. 한글에서 HWPX로 저장해 주세요.".into());
    }
    let flags = u32_at(&header, 36)?;
    if flags & 2 != 0 {
        return Err(
            "암호가 걸린 한글 문서는 읽을 수 없음 · 한글에서 암호를 해제한 사본을 저장해 주세요"
                .into(),
        );
    }
    if flags & 4 != 0 {
        return Err(
            "배포용 한글 문서는 읽을 수 없음 · 편집 가능한 HWP 또는 HWPX 원본을 골라 주세요".into(),
        );
    }
    let compressed = flags & 1 != 0;
    let doc_info = stream(&mut compound, "/DocInfo", compressed, &mut remaining)?;
    let styles = styles::read(&doc_info)?;
    let mut sections = compound
        .read_storage("/BodyText")
        .map_err(|_| BROKEN)?
        .filter_map(|entry| {
            entry
                .name()
                .strip_prefix("Section")
                .and_then(|n| n.parse::<u32>().ok())
                .map(|n| (n, entry.path().to_path_buf()))
        })
        .collect::<Vec<_>>();
    sections.sort_by_key(|(n, _)| *n);
    if sections.is_empty() || sections.len() > 1024 {
        return Err(BROKEN.into());
    }
    // Missing or duplicate section numbers mean missing manuscript text.
    if sections
        .iter()
        .enumerate()
        .any(|(i, (n, _))| *n != i as u32)
    {
        return Err(BROKEN.into());
    }
    let mut items = Vec::new();
    let mut notes = Vec::new();
    for (_, path) in sections {
        let data = stream(
            &mut compound,
            &path.to_string_lossy(),
            compressed,
            &mut remaining,
        )?;
        body::read(&data, &styles, &mut items)?;
        notes.extend(memos::read(&data)?);
    }
    Ok(Raw {
        notes,
        items,
        encoding: None,
        page: None,
    })
}
