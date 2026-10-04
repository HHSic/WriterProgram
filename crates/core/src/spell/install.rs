//! The Korean dictionary as separate data in the app's data folder.
//!
//! hunspell-dict-ko is GPL-3.0 (its words CC BY-SA 4.0), so it is never
//! compiled into the app. After the writer agrees, the app fetches the
//! pinned release from the project's GitHub page (`commands/spell.rs`, the
//! only network use), and `install` checks it against the pinned SHA-256 and
//! unpacks it unchanged, license files included, with a notice saying where
//! it came from.
//!
//! ```text
//! <app data>/spelling/hunspell-dict-ko-0.7.94/
//!   ko.aff  ko.dic  LICENSE.md  LICENSE.GPL-3  NOTICE.txt
//! ```

use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::store::{atomic_write, remove_dir_all_retry};
use crate::{Error, Result};

pub const VERSION: &str = "0.7.94";
pub const URL: &str = "https://github.com/spellcheck-ko/hunspell-dict-ko/releases/download/0.7.94/ko-aff-dic-0.7.94.zip";
pub const SHA256: &str = "c7252e2f6bf421e081a457ca007cd8a304ee2045947edba47b367db780e63c48";
/// Bytes to fetch (the zip).
pub const DOWNLOAD_BYTES: u64 = 897_723;
/// Bytes on disk once unpacked.
pub const INSTALLED_BYTES: u64 = 13_996_687;
pub const SOURCE: &str = "https://github.com/spellcheck-ko/hunspell-dict-ko";

pub const AFF: &str = "ko.aff";
pub const DIC: &str = "ko.dic";
const LICENSES: [&str; 2] = ["LICENSE.md", "LICENSE.GPL-3"];
pub const NOTICE_FILE: &str = "NOTICE.txt";

/// The folder for this version under the app's `spelling` folder.
pub fn folder(spelling_dir: &Path) -> PathBuf {
    spelling_dir.join(format!("hunspell-dict-ko-{VERSION}"))
}

/// What the screen says about the dictionary (보기 설정, the download offer).
pub const NOTICE: &str = "\
맞춤법 사전: hunspell-dict-ko 0.7.94
https://github.com/spellcheck-ko/hunspell-dict-ko

Copyright (C) 2008-2019 hunspell-dict-ko contributors
Copyright (C) 2017-2019 National Institute of Korean Language (국립국어원)

이 사전 파일(ko.aff, ko.dic)은 GNU GPL 3.0으로 배포됩니다. 낱말 자료 대부분은
CC BY-SA 4.0입니다(국립국어원 사전 자료 포함). 자세한 조건은 같은 폴더의
LICENSE.md와 LICENSE.GPL-3에 있습니다. WriterProgram은 이 파일을 고치지 않고
읽기만 하며, 앱과 따로 받는 데이터로 둡니다.

The dictionary files (ko.aff, ko.dic) are distributed under the GNU GPL 3.0;
most word data is CC BY-SA 4.0. See LICENSE.md and LICENSE.GPL-3 in this
folder. WriterProgram reads them unchanged and keeps them apart from the app.
";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictInfo {
    pub version: String,
    pub folder: String,
    pub source: String,
    pub notice: String,
}

/// The installed dictionary, if any.
pub fn installed(spelling_dir: &Path) -> Option<DictInfo> {
    let dir = folder(spelling_dir);
    (dir.join(AFF).is_file() && dir.join(DIC).is_file()).then(|| DictInfo {
        version: VERSION.into(),
        folder: dir.display().to_string(),
        source: SOURCE.into(),
        notice: NOTICE.into(),
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Checks the fetched zip against `sha256` and unpacks the dictionary and
/// its license files into the version's folder. Nothing is left behind when
/// a step fails.
pub fn install(spelling_dir: &Path, zip_bytes: &[u8], sha256: &str) -> Result<DictInfo> {
    if hex(&Sha256::digest(zip_bytes)) != sha256 {
        return Err(Error::Invalid(
            "받은 사전 파일이 원래 파일과 다름 (중간에 바뀌었거나 덜 받음)".into(),
        ));
    }
    let bad = |m: String| Error::Invalid(format!("사전 파일을 풀 수 없음 ({m})"));
    let mut archive =
        zip::ZipArchive::new(Cursor::new(zip_bytes)).map_err(|e| bad(e.to_string()))?;
    let target = folder(spelling_dir);
    let part = spelling_dir.join(format!(".hunspell-dict-ko-{VERSION}.part"));
    let _ = clear(&part);
    fs::create_dir_all(&part).map_err(|e| Error::io(&part, e))?;

    let unpacked = (|| -> Result<()> {
        let wanted: Vec<&str> = [AFF, DIC].into_iter().chain(LICENSES).collect();
        let mut found = Vec::new();
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).map_err(|e| bad(e.to_string()))?;
            let name = entry
                .name()
                .rsplit('/')
                .next()
                .unwrap_or_default()
                .to_string();
            if !wanted.contains(&name.as_str()) || entry.is_dir() {
                continue;
            }
            let mut bytes = Vec::new();
            entry
                .read_to_end(&mut bytes)
                .map_err(|e| bad(e.to_string()))?;
            atomic_write(&part.join(&name), &bytes)?;
            found.push(name);
        }
        for name in wanted {
            if !found.iter().any(|f| f == name) {
                return Err(bad(format!("{name} 없음")));
            }
        }
        atomic_write(&part.join(NOTICE_FILE), NOTICE.as_bytes())?;
        let _ = clear(&target);
        fs::rename(&part, &target).map_err(|e| Error::io(&target, e))
    })();
    if unpacked.is_err() {
        let _ = clear(&part);
    }
    unpacked?;
    installed(spelling_dir).ok_or_else(|| bad("파일이 없음".into()))
}

/// Takes the dictionary off this device.
pub fn remove(spelling_dir: &Path) -> Result<()> {
    let dir = folder(spelling_dir);
    clear(&dir).map_err(|e| Error::io(&dir, e))
}

/// Removes a folder, waiting out a brief lock by a virus scanner.
fn clear(dir: &Path) -> std::io::Result<()> {
    remove_dir_all_retry(dir, 4, Duration::from_millis(40))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn zip_of(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut z = zip::ZipWriter::new(Cursor::new(&mut out));
            let opts = zip::write::SimpleFileOptions::default();
            for (name, bytes) in files {
                z.start_file(*name, opts).unwrap();
                z.write_all(bytes).unwrap();
            }
            z.finish().unwrap();
        }
        out
    }

    fn release() -> Vec<u8> {
        let fixture = crate::spell::fixture::AFF.as_bytes();
        zip_of(&[
            ("ko-aff-dic-0.7.94/LICENSE.md", b"license"),
            ("ko-aff-dic-0.7.94/LICENSE.GPL-3", b"gpl"),
            ("ko-aff-dic-0.7.94/ko.aff", fixture),
            (
                "ko-aff-dic-0.7.94/ko.dic",
                crate::spell::fixture::DIC.as_bytes(),
            ),
        ])
    }

    #[test]
    fn installs_a_checked_release_with_its_licenses() {
        let dir = tempfile::tempdir().unwrap();
        let bytes = release();
        let sum = hex(&Sha256::digest(&bytes));
        assert!(installed(dir.path()).is_none());
        let info = install(dir.path(), &bytes, &sum).unwrap();
        assert_eq!(info.version, VERSION);
        let folder = folder(dir.path());
        for name in [AFF, DIC, "LICENSE.md", "LICENSE.GPL-3", NOTICE_FILE] {
            assert!(folder.join(name).is_file(), "{name}");
        }
        let speller = crate::spell::Speller::open(&folder).unwrap();
        assert!(speller.knows("먹었다"));
        remove(dir.path()).unwrap();
        assert!(installed(dir.path()).is_none());
    }

    #[test]
    fn refuses_a_file_that_is_not_the_pinned_one() {
        let dir = tempfile::tempdir().unwrap();
        let err = install(dir.path(), &release(), SHA256).unwrap_err();
        assert!(err.user_message().contains("원래 파일과 다름"));
        assert!(installed(dir.path()).is_none());
    }

    #[test]
    fn refuses_a_release_without_its_licenses() {
        let dir = tempfile::tempdir().unwrap();
        let bytes = zip_of(&[("ko.aff", b"x"), ("ko.dic", b"y")]);
        let sum = hex(&Sha256::digest(&bytes));
        assert!(install(dir.path(), &bytes, &sum).is_err());
        assert!(installed(dir.path()).is_none());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    }
}
