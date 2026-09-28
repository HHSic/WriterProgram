//! OneDrive through Microsoft Graph, in the app's own folder (앱/WriterProgram,
//! `Files.ReadWrite.AppFolder`). Files are reached by path; versions are
//! eTags, and every change is conditional on one (`If-Match`).

use std::collections::HashMap;

use serde::Deserialize;
use serde_json::json;

use super::{Account, Session, drive_error};
use crate::http::{Body, enc};
use crate::remote::{Put, Remote, RemoteFile};
use crate::{Error, Result};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Me {
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    mail: Option<String>,
    #[serde(default)]
    user_principal_name: Option<String>,
}

pub fn account(s: &Session) -> Result<Account> {
    let url = format!(
        "{}/me?$select=displayName,mail,userPrincipalName",
        s.ends.api
    );
    let reply = s.call("GET", &url, &[], &|| Body::Empty)?;
    if !reply.ok() {
        return Err(drive_error("계정 정보를 읽지 못함", &reply));
    }
    let me: Me = reply.json()?;
    Ok(Account {
        name: me.display_name.unwrap_or_default(),
        email: me.mail.or(me.user_principal_name).unwrap_or_default(),
    })
}

fn approot(s: &Session) -> String {
    format!("{}/me/drive/special/approot", s.ends.api)
}

/// `a/b c.md` → `a/b%20c.md`, each part encoded.
fn enc_path(path: &str) -> String {
    path.split('/').map(enc).collect::<Vec<_>>().join("/")
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Item {
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    e_tag: Option<String>,
    #[serde(default)]
    folder: Option<serde_json::Value>,
    #[serde(default)]
    deleted: Option<serde_json::Value>,
    #[serde(default)]
    parent_reference: Option<ParentRef>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ParentRef {
    #[serde(default)]
    id: Option<String>,
}

#[derive(Deserialize)]
struct Page {
    #[serde(default)]
    value: Vec<Item>,
    #[serde(rename = "@odata.nextLink", default)]
    next: Option<String>,
}

/// Project folders in the app's folder.
pub fn projects(s: &Session) -> Result<Vec<String>> {
    let mut url = Some(format!("{}/children?$select=name,folder", approot(s)));
    let mut names = Vec::new();
    while let Some(next) = url {
        let reply = s.call("GET", &next, &[], &|| Body::Empty)?;
        if !reply.ok() {
            return Err(drive_error("드라이브 목록을 읽지 못함", &reply));
        }
        let page: Page = reply.json()?;
        names.extend(
            page.value
                .into_iter()
                .filter(|i| i.folder.is_some())
                .map(|i| i.name),
        );
        url = page.next;
    }
    names.sort();
    Ok(names)
}

/// A project folder as a remote; made when missing and `create`.
pub fn open<'s>(s: &'s Session, folder: &str, create: bool) -> Result<OneDriveRemote<'s>> {
    let url = format!(
        "{}:/{}?$select=id,name,folder",
        approot(s),
        enc_path(folder)
    );
    let reply = s.call("GET", &url, &[], &|| Body::Empty)?;
    let id = if reply.ok() {
        reply.json::<Item>()?.id
    } else if reply.status == 404 && create {
        let url = format!("{}/children", approot(s));
        let body =
            json!({ "name": folder, "folder": {}, "@microsoft.graph.conflictBehavior": "fail" });
        let made = s.call("POST", &url, &[], &|| Body::Json(body.clone()))?;
        if !made.ok() {
            return Err(drive_error("드라이브에 폴더를 만들지 못함", &made));
        }
        made.json::<Item>()?.id
    } else if reply.status == 404 {
        return Err(Error::Drive(format!("드라이브에 '{folder}' 폴더가 없음")));
    } else {
        return Err(drive_error("드라이브 폴더를 찾지 못함", &reply));
    };
    Ok(OneDriveRemote {
        s,
        folder: folder.to_string(),
        root_id: id,
    })
}

pub struct OneDriveRemote<'s> {
    s: &'s Session,
    folder: String,
    root_id: String,
}

impl OneDriveRemote<'_> {
    fn url(&self, path: &str) -> String {
        format!(
            "{}:/{}/{}:",
            approot(self.s),
            enc_path(&self.folder),
            enc_path(path)
        )
    }
}

impl Remote for OneDriveRemote<'_> {
    fn list(&mut self) -> Result<Vec<RemoteFile>> {
        // Everything under the project folder, pages at a time.
        let mut url = Some(format!(
            "{}/me/drive/items/{}/delta?$select=id,name,eTag,file,folder,deleted,parentReference",
            self.s.ends.api, self.root_id
        ));
        let mut items: Vec<Item> = Vec::new();
        while let Some(next) = url {
            let reply = self.s.call("GET", &next, &[], &|| Body::Empty)?;
            if !reply.ok() {
                return Err(drive_error("드라이브 목록을 읽지 못함", &reply));
            }
            let page: Page = reply.json()?;
            items.extend(page.value);
            url = page.next;
        }
        let items: Vec<Item> = items.into_iter().filter(|i| i.deleted.is_none()).collect();
        let by_id: HashMap<&str, &Item> = items.iter().map(|i| (i.id.as_str(), i)).collect();
        let root = self.root_id.clone();
        let path_of = |item: &Item| -> Option<String> {
            let mut parts = vec![item.name.clone()];
            let mut parent = item.parent_reference.as_ref()?.id.clone()?;
            for _ in 0..64 {
                if parent == root {
                    parts.reverse();
                    return Some(parts.join("/"));
                }
                let up = by_id.get(parent.as_str())?;
                parts.push(up.name.clone());
                parent = up.parent_reference.as_ref()?.id.clone()?;
            }
            None
        };
        let mut out: Vec<RemoteFile> = items
            .iter()
            .filter(|i| i.folder.is_none() && i.id != self.root_id)
            .filter_map(|i| {
                Some(RemoteFile {
                    path: path_of(i)?,
                    rev: i.e_tag.clone()?,
                })
            })
            .collect();
        out.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(out)
    }

    fn get(&mut self, path: &str) -> Result<Vec<u8>> {
        let url = format!("{}/content", self.url(path));
        let reply = self.s.call("GET", &url, &[], &|| Body::Empty)?;
        if !reply.ok() {
            return Err(drive_error("드라이브에서 파일을 받지 못함", &reply));
        }
        Ok(reply.body)
    }

    fn put(&mut self, path: &str, bytes: &[u8], expected: Option<&str>) -> Result<Put> {
        let (url, headers) = match expected {
            Some(tag) => (
                format!("{}/content", self.url(path)),
                vec![("If-Match", tag.to_string())],
            ),
            None => (
                format!(
                    "{}/content?@microsoft.graph.conflictBehavior=fail",
                    self.url(path)
                ),
                Vec::new(),
            ),
        };
        let reply = self.s.call("PUT", &url, &headers, &|| {
            Body::Bytes(bytes, "application/octet-stream")
        })?;
        if matches!(reply.status, 409 | 412) {
            return Ok(Put::Changed);
        }
        if reply.status == 404 && expected.is_some() {
            return Ok(Put::Changed);
        }
        if !reply.ok() {
            return Err(drive_error("드라이브에 올리지 못함", &reply));
        }
        let item: Item = reply.json()?;
        Ok(Put::Done(item.e_tag.unwrap_or_default()))
    }

    fn remove(&mut self, path: &str, expected: &str) -> Result<bool> {
        let reply = self.s.call(
            "DELETE",
            &self.url(path),
            &[("If-Match", expected.to_string())],
            &|| Body::Empty,
        )?;
        match reply.status {
            204 | 200 | 404 => Ok(true),
            412 | 409 => Ok(false),
            _ => Err(drive_error("드라이브에서 지우지 못함", &reply)),
        }
    }
}
