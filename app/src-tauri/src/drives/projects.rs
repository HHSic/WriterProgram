//! Projects on a drive: linking a project to a drive folder, passes, and
//! bringing a project from a drive to this device.

use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::{AppHandle, Manager};
use writer_core::project::{self, Overview};
use writer_core::store::now_iso;
use writer_sync::accounts::{self, Link, Registry};
use writer_sync::engine::{Choices, Report};
use writer_sync::providers::{self, Provider};
use writer_sync::remote::Remote;

use super::{blocking, session};
use crate::error::{Res, fail};
use crate::paths::{base_dir, recent_file, registry_file};
use crate::state::AppState;

#[tauri::command]
pub async fn project_link_get(app: AppHandle, project_id: String) -> Res<Option<Link>> {
    Ok(Registry::load(&registry_file(&app)?)
        .links
        .get(&project_id)
        .cloned())
}

/// The project's id in a drive folder's project.json, if it has one.
fn folder_project(remote: &mut dyn Remote) -> writer_sync::Result<Option<(String, String)>> {
    if !remote
        .list()?
        .iter()
        .any(|f| f.path == project::PROJECT_FILE)
    {
        return Ok(None);
    }
    let bytes = remote.get(project::PROJECT_FILE)?;
    let p: project::Project = serde_json::from_slice(&bytes).map_err(|e| {
        writer_sync::Error::Drive(format!("드라이브의 작품 정보를 읽지 못함 ({e})"))
    })?;
    Ok(Some((p.id, p.title)))
}

/// Starts keeping a project in step with a folder on a drive: its own folder
/// when it is there already (from another device), else a new one.
#[tauri::command]
pub async fn project_link(
    app: AppHandle,
    project_id: String,
    title: String,
    provider: Provider,
) -> Res<Link> {
    blocking(move || {
        let file = registry_file(&app)?;
        let mut registry = Registry::load(&file);
        let session = session(&app, &registry, provider)?;
        let names = providers::projects(&session).map_err(fail)?;
        let base = providers::folder_name(&title);
        let mut folder = None;
        for n in 1..50 {
            let candidate = if n == 1 {
                base.clone()
            } else {
                format!("{base} ({n})")
            };
            if !names.contains(&candidate) {
                folder = Some(candidate);
                break;
            }
            let mut remote = providers::open(&session, &candidate, false).map_err(fail)?;
            match folder_project(remote.as_mut()).map_err(fail)? {
                Some((id, _)) if id != project_id => continue,
                _ => {
                    folder = Some(candidate);
                    break;
                }
            }
        }
        let folder = folder.ok_or_else(|| "드라이브에 쓸 폴더 이름을 정하지 못함".to_string())?;
        let link = Link {
            provider,
            folder,
            linked_at: now_iso(),
            synced_at: None,
            error: None,
        };
        registry.links.insert(project_id, link.clone());
        registry.save(&file).map_err(fail)?;
        Ok(link)
    })
    .await
}

#[tauri::command]
pub async fn project_unlink(app: AppHandle, project_id: String) -> Res<()> {
    let file = registry_file(&app)?;
    let mut registry = Registry::load(&file);
    registry.links.remove(&project_id);
    registry.save(&file).map_err(fail)?;
    let _ = std::fs::remove_file(accounts::base_file(&base_dir(&app)?, &project_id));
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncOutcome {
    link: Link,
    report: Option<Report>,
}

/// One pass for an open project. Errors end up in the link (and the report is
/// none) rather than failing, so the screen can say what happened. `choices`
/// is what the writer said about removals an earlier pass held back.
#[tauri::command]
pub async fn project_sync(
    app: AppHandle,
    root: String,
    project_id: String,
    choices: Option<Choices>,
) -> Res<SyncOutcome> {
    let choices = choices.unwrap_or_default();
    blocking(move || {
        let file = registry_file(&app)?;
        let registry = Registry::load(&file);
        let link = registry
            .links
            .get(&project_id)
            .cloned()
            .ok_or_else(|| "이 작품은 드라이브와 맞추지 않음".to_string())?;
        let result = session(&app, &registry, link.provider).and_then(|session| {
            let state = app.state::<AppState>();
            let base = accounts::base_file(&base_dir(&app)?, &project_id);
            accounts::sync_project(
                &session,
                &link,
                Path::new(&root),
                &base,
                state.lock(),
                &choices,
            )
            .map_err(fail)
        });
        // Read again: another pass or a link change may have happened meanwhile.
        let mut registry = Registry::load(&file);
        let Some(saved) = registry.links.get_mut(&project_id) else {
            return Err("이 작품은 드라이브와 맞추지 않음".into());
        };
        let report = match result {
            Ok(report) => {
                saved.synced_at = Some(now_iso());
                saved.error = None;
                Some(report)
            }
            Err(e) => {
                saved.error = Some(e);
                None
            }
        };
        let link = saved.clone();
        registry.save(&file).map_err(fail)?;
        Ok(SyncOutcome { link, report })
    })
    .await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveProject {
    folder: String,
    title: String,
    id: String,
}

/// Projects in a drive, to bring one to this device.
#[tauri::command]
pub async fn drive_projects(app: AppHandle, provider: Provider) -> Res<Vec<DriveProject>> {
    blocking(move || {
        let registry = Registry::load(&registry_file(&app)?);
        let session = session(&app, &registry, provider)?;
        let mut out = Vec::new();
        for folder in providers::projects(&session).map_err(fail)? {
            let mut remote = providers::open(&session, &folder, false).map_err(fail)?;
            if let Some((id, title)) = folder_project(remote.as_mut()).map_err(fail)? {
                out.push(DriveProject { folder, title, id });
            }
        }
        Ok(out)
    })
    .await
}

/// Brings a project from a drive into a new folder in `dest`, keeps it in step
/// from now on, and opens it.
#[tauri::command]
pub async fn drive_fetch(
    app: AppHandle,
    provider: Provider,
    folder: String,
    dest: String,
) -> Res<Overview> {
    blocking(move || {
        let file = registry_file(&app)?;
        let registry = Registry::load(&file);
        let session = session(&app, &registry, provider)?;
        let mut remote = providers::open(&session, &folder, false).map_err(fail)?;
        let (project_id, _) = folder_project(remote.as_mut())
            .map_err(fail)?
            .ok_or_else(|| "드라이브의 이 폴더에는 작품이 없음".to_string())?;
        drop(remote);
        let parent = PathBuf::from(&dest);
        let mut root = parent.join(&folder);
        let mut n = 2;
        while root.exists() {
            root = parent.join(format!("{folder} ({n})"));
            n += 1;
        }
        std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        let link = Link {
            provider,
            folder,
            linked_at: now_iso(),
            synced_at: None,
            error: None,
        };
        let state = app.state::<AppState>();
        let base = accounts::base_file(&base_dir(&app)?, &project_id);
        let _ = std::fs::remove_file(&base);
        accounts::sync_project(
            &session,
            &link,
            &root,
            &base,
            state.lock(),
            &Choices::default(),
        )
        .map_err(fail)?;
        let overview = project::open(&root).map_err(fail)?;
        let _ = writer_core::recent::touch(&recent_file(&app)?, &overview);
        let mut registry = Registry::load(&file);
        registry.links.insert(
            project_id,
            Link {
                synced_at: Some(now_iso()),
                ..link
            },
        );
        registry.save(&file).map_err(fail)?;
        Ok(overview)
    })
    .await
}
