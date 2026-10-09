//! Public reader links to isolated DOCX copies on an already connected drive.
//! The sync project folder is never made public.
use serde::{Deserialize, Serialize};

use crate::providers::{Provider, Session, dropbox, google, onedrive};
use crate::{Error, Result};

pub(crate) const FOLDER: &str = "WriterProgram Shared Manuscripts";

#[derive(Debug, Serialize, Deserialize)]
pub struct Published {
    pub url: String,
}

pub fn new_file_name() -> Result<String> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes)
        .map_err(|_| Error::Invalid("공유 파일 이름을 만들지 못함".into()))?;
    let id: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    Ok(format!("share-{id}.docx"))
}

fn checked_name(name: &str) -> Result<()> {
    let id = name
        .strip_prefix("share-")
        .and_then(|s| s.strip_suffix(".docx"));
    if !id.is_some_and(|id| id.len() == 32 && id.bytes().all(|c| c.is_ascii_hexdigit())) {
        return Err(Error::Invalid("공유용 사본만 관리할 수 있습니다".into()));
    }
    Ok(())
}

pub(crate) fn checked_url(provider: Provider, value: &str) -> Result<()> {
    let url = url::Url::parse(value)
        .map_err(|_| Error::Drive("드라이브가 올바르지 않은 공유 링크를 반환함".into()))?;
    let host = url.host_str().unwrap_or_default();
    let trusted = match provider {
        Provider::Google => matches!(host, "drive.google.com" | "docs.google.com"),
        Provider::Onedrive => {
            matches!(host, "1drv.ms" | "onedrive.live.com") || host.ends_with(".sharepoint.com")
        }
        Provider::Dropbox => matches!(host, "www.dropbox.com" | "dropbox.com"),
    };
    if !trusted || url.scheme() != "https" || !url.username().is_empty() || url.password().is_some()
    {
        return Err(Error::Drive(
            "드라이브가 올바르지 않은 공유 링크를 반환함".into(),
        ));
    }
    Ok(())
}

pub fn publish(session: &Session, name: &str, bytes: &[u8]) -> Result<Published> {
    checked_name(name)?;
    if bytes.len() > 16 * 1024 * 1024 {
        return Err(Error::Invalid(
            "공유 원고가 너무 큽니다. 회차를 나누어 공유하세요".into(),
        ));
    }
    match session.provider {
        Provider::Google => google::publish_share(session, name, bytes),
        Provider::Onedrive => onedrive::publish_share(session, name, bytes),
        Provider::Dropbox => dropbox::publish_share(session, name, bytes),
    }
}

pub fn revoke(session: &Session, name: &str, url: &str) -> Result<()> {
    checked_name(name)?;
    checked_url(session.provider, url)?;
    match session.provider {
        Provider::Google => google::revoke_share(session, name),
        Provider::Onedrive => onedrive::revoke_share(session, name),
        Provider::Dropbox => dropbox::revoke_share(session, name, url),
    }
}

pub(crate) fn refused() -> Error {
    Error::Drive("드라이브가 링크 공유를 허용하지 않았습니다. 공유 권한·조직 정책을 확인하고, Dropbox는 공유 권한을 허용해 다시 연결하세요".into())
}
