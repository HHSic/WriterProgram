//! Read-only copies shared by already connected drive accounts. No separate
//! server or credentials; the local registry contains only link metadata.
use std::path::Path;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};
use writer_core::export::{DocInfo, DocOptions, ExportDoc};
use writer_core::{docx, format, markup::Block, project, share, store};
use writer_sync::accounts::Registry;
use writer_sync::providers::Provider;
use writer_sync::sharing;

use crate::drives::{blocking, session};
use crate::error::{Res, fail};
use crate::paths::{registry_file, share_file};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    pub id: String,
    pub url: String,
    pub title: String,
    pub provider: Provider,
    pub account: String,
    pub chapter_count: usize,
    pub created_at: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Record {
    project_id: String,
    name: String,
    link: Link,
}

fn records(path: &Path) -> Res<Vec<Record>> {
    match std::fs::read(path) {
        Ok(bytes) => {
            serde_json::from_slice(&bytes).map_err(|_| "공유 링크 기록을 읽지 못했습니다".into())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(_) => Err("공유 링크 기록을 읽지 못했습니다".into()),
    }
}

fn save_records(path: &Path, records: &[Record]) -> Res<()> {
    let bytes =
        serde_json::to_vec_pretty(records).map_err(|_| "공유 링크 기록을 만들지 못했습니다")?;
    store::atomic_write(path, &bytes).map_err(fail)
}

fn reader_docx(snapshot: &share::Snapshot) -> Res<Vec<u8>> {
    let docs: Vec<_> = snapshot
        .chapters
        .iter()
        .map(|c| ExportDoc {
            heading: c.title.clone(),
            blocks: c.text.split('\n').map(Block::text).collect(),
        })
        .collect();
    // A neutral format prevents private running headers and pen names from
    // entering the shared copy. Only the chosen titles and text go out.
    docx::docx_bytes(
        &docs,
        &format::builtin("submission-a4").ok_or("공유 서식을 찾지 못함")?,
        &DocOptions {
            include_titles: true,
            scene_break: "***".into(),
        },
        &DocInfo {
            title: snapshot.title.clone(),
            author: String::new(),
        },
    )
    .map_err(fail)
}

#[tauri::command]
pub async fn share_list(app: AppHandle, root: String) -> Res<Vec<Link>> {
    let path = share_file(&app)?;
    blocking(move || {
        let project = project::load(Path::new(&root)).map_err(fail)?;
        Ok(records(&path)?
            .into_iter()
            .filter(|r| r.project_id == project.id)
            .map(|r| r.link)
            .rev()
            .collect())
    })
    .await
}

#[tauri::command]
pub async fn share_create(
    app: AppHandle,
    state: State<'_, AppState>,
    root: String,
    doc_ids: Vec<String>,
    provider: Provider,
) -> Res<Link> {
    let (snapshot, project_id) = {
        let _write = state.write();
        (
            share::snapshot(Path::new(&root), &doc_ids).map_err(fail)?,
            project::load(Path::new(&root)).map_err(fail)?.id,
        )
    };
    blocking(move || {
        let registry = Registry::load(&registry_file(&app)?);
        let connected = registry.connection(provider).ok_or("공유하려면 드라이브 계정을 먼저 연결하세요")?;
        let account = connected.account.email.clone();
        if account.trim().is_empty() {
            return Err("드라이브 계정을 확인하지 못했습니다. 계정을 다시 연결하세요".into());
        }
        let drive = session(&app, &registry, provider)?;
        let bytes = reader_docx(&snapshot)?;
        let name = sharing::new_file_name().map_err(fail)?;
        let published = sharing::publish(&drive, &name, &bytes).map_err(fail)?;
        let link = Link {
            id: name.trim_start_matches("share-").trim_end_matches(".docx").into(),
            url: published.url, title: snapshot.title, provider, account,
            chapter_count: snapshot.chapters.len(), created_at: store::now_iso(),
        };
        let saved = (|| {
            let state = app.state::<AppState>();
            let _write = state.write();
            let path = share_file(&app)?;
            let mut list = records(&path)?;
            list.push(Record { project_id, name: name.clone(), link: link.clone() });
            save_records(&path, &list)
        })();
        if saved.is_err() {
            if sharing::revoke(&drive, &name, &link.url).is_ok() {
                return Err("공유 기록을 저장하지 못해 새 공유를 취소했습니다. 다시 시도하세요".into());
            }
            return Err(format!("공유 기록을 저장하지 못했습니다. 드라이브의 공유용 사본 '{name}'을 직접 삭제해 종료하세요. 생성된 링크: {}", link.url));
        }
        Ok(link)
    }).await
}

#[tauri::command]
pub async fn share_revoke(app: AppHandle, root: String, id: String) -> Res<()> {
    blocking(move || {
        let project_id = project::load(Path::new(&root)).map_err(fail)?.id;
        let path = share_file(&app)?;
        let record = records(&path)?
            .into_iter()
            .find(|r| r.project_id == project_id && r.link.id == id)
            .ok_or("이 작품의 공유 링크를 찾지 못했습니다")?;
        let registry = Registry::load(&registry_file(&app)?);
        let connected = registry
            .connection(record.link.provider)
            .ok_or("공유한 드라이브 계정을 다시 연결하세요")?;
        if !connected
            .account
            .email
            .eq_ignore_ascii_case(&record.link.account)
        {
            return Err("이 링크를 만든 드라이브 계정으로 연결하세요".into());
        }
        let drive = session(&app, &registry, record.link.provider)?;
        sharing::revoke(&drive, &record.name, &record.link.url).map_err(fail)?;
        let state = app.state::<AppState>();
        let _write = state.write();
        let mut list = records(&path)?;
        list.retain(|r| !(r.project_id == project_id && r.link.id == id));
        save_records(&path, &list)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reader_copy_preserves_unicode_and_excludes_private_metadata() {
        let snapshot = share::Snapshot {
            title: "작품".into(),
            chapters: vec![share::Chapter {
                title: "첫 장".into(),
                text: "한글 (ง •̀_•́)ง\n<표정> 😊".into(),
            }],
        };
        let bytes = reader_docx(&snapshot).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("shared.docx");
        std::fs::write(&path, bytes).unwrap();
        let preview = writer_core::import::preview(&[path], &Default::default());
        assert!(preview.files[0].error.is_none());
        assert_eq!(preview.chapters.len(), 1);
        assert!(preview.chapters[0].snippet.contains("한글 (ง •̀_•́)ง"));
        assert!(preview.chapters[0].snippet.contains("<표정> 😊"));
    }
}
