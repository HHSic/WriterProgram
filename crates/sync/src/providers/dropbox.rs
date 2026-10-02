//! Dropbox (API v2) in the app's own folder (앱/WriterProgram). Files are
//! reached by path; versions are Dropbox's `rev`s, and uploads and removals
//! are conditional on one.

use serde::Deserialize;
use serde_json::{Value, json};

use super::{Account, Session, Space, drive_error, space_full};
use crate::http::{Body, header_json};
use crate::remote::{Put, Remote, RemoteFile};
use crate::{Error, Result};

#[derive(Deserialize)]
struct Name {
    #[serde(default)]
    display_name: String,
}

#[derive(Deserialize)]
struct Me {
    name: Name,
    #[serde(default)]
    email: String,
}

pub fn account(s: &Session) -> Result<Account> {
    let url = format!("{}/users/get_current_account", s.ends.api);
    let reply = s.call("POST", &url, &[], &|| Body::Empty)?;
    if !reply.ok() {
        return Err(drive_error("계정 정보를 읽지 못함", &reply));
    }
    let me: Me = reply.json()?;
    Ok(Account {
        name: me.name.display_name,
        email: me.email,
    })
}

#[derive(Deserialize)]
struct Usage {
    /// This account's use.
    used: u64,
    allocation: Allocation,
}

/// `individual`: the account's own space. `team`: shared with the team, maybe
/// with a limit for each member (0: none).
#[derive(Deserialize)]
struct Allocation {
    #[serde(rename = ".tag", default)]
    tag: String,
    #[serde(default)]
    allocated: u64,
    /// The whole team's use.
    #[serde(default)]
    used: u64,
    #[serde(default)]
    user_within_team_space_allocated: u64,
    #[serde(default)]
    user_within_team_space_used_cached: u64,
}

/// How full the Dropbox is (`users/get_space_usage`, `account_info.read`).
pub fn space(s: &Session) -> Option<Space> {
    let url = format!("{}/users/get_space_usage", s.ends.api);
    let reply = s.call("POST", &url, &[], &|| Body::Empty).ok()?;
    if !reply.ok() {
        return None;
    }
    let usage: Usage = reply.json().ok()?;
    let a = usage.allocation;
    let space = match a.tag.as_str() {
        "team" if a.user_within_team_space_allocated > 0 => Space {
            used: a.user_within_team_space_used_cached,
            total: a.user_within_team_space_allocated,
        },
        "team" => Space {
            used: a.used,
            total: a.allocated,
        },
        _ => Space {
            used: usage.used,
            total: a.allocated,
        },
    };
    (space.total > 0).then_some(space)
}

#[derive(Debug, Deserialize)]
struct Entry {
    /// "file" or "folder" in listings; missing in an upload's answer.
    #[serde(rename = ".tag", default)]
    tag: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    path_display: Option<String>,
    #[serde(default)]
    path_lower: Option<String>,
    #[serde(default)]
    rev: Option<String>,
}

#[derive(Deserialize)]
struct Listing {
    #[serde(default)]
    entries: Vec<Entry>,
    #[serde(default)]
    cursor: String,
    #[serde(default)]
    has_more: bool,
}

#[derive(Deserialize)]
struct Failure {
    #[serde(default)]
    error_summary: String,
}

fn rpc(s: &Session, endpoint: &str, arg: Value) -> Result<crate::http::Reply> {
    let url = format!("{}/{endpoint}", s.ends.api);
    s.call("POST", &url, &[], &|| Body::Json(arg.clone()))
}

fn listing(s: &Session, path: &str, recursive: bool) -> Result<Vec<Entry>> {
    let reply = rpc(
        s,
        "files/list_folder",
        json!({ "path": path, "recursive": recursive, "include_deleted": false }),
    )?;
    if !reply.ok() {
        return Err(drive_error("드라이브 목록을 읽지 못함", &reply));
    }
    let mut page: Listing = reply.json()?;
    let mut out = std::mem::take(&mut page.entries);
    while page.has_more {
        let reply = rpc(
            s,
            "files/list_folder/continue",
            json!({ "cursor": page.cursor }),
        )?;
        if !reply.ok() {
            return Err(drive_error("드라이브 목록을 읽지 못함", &reply));
        }
        page = reply.json()?;
        out.append(&mut page.entries);
    }
    Ok(out)
}

/// Project folders in the app's folder.
pub fn projects(s: &Session) -> Result<Vec<String>> {
    let mut names: Vec<String> = listing(s, "", false)?
        .into_iter()
        .filter(|e| e.tag == "folder")
        .map(|e| e.name)
        .collect();
    names.sort();
    Ok(names)
}

/// A project folder as a remote; made when missing and `create`.
pub fn open<'s>(s: &'s Session, folder: &str, create: bool) -> Result<DropboxRemote<'s>> {
    let path = format!("/{folder}");
    let reply = rpc(s, "files/get_metadata", json!({ "path": path }))?;
    if !reply.ok() {
        let missing = reply.status == 409
            && reply
                .json::<Failure>()
                .is_ok_and(|f| f.error_summary.contains("not_found"));
        if !missing {
            return Err(drive_error("드라이브 폴더를 찾지 못함", &reply));
        }
        if !create {
            return Err(Error::Drive(format!("드라이브에 '{folder}' 폴더가 없음")));
        }
        let made = rpc(
            s,
            "files/create_folder_v2",
            json!({ "path": path, "autorename": false }),
        )?;
        if !made.ok() {
            return Err(drive_error("드라이브에 폴더를 만들지 못함", &made));
        }
    }
    Ok(DropboxRemote { s, folder: path })
}

pub struct DropboxRemote<'s> {
    s: &'s Session,
    /// `/<project folder>`.
    folder: String,
}

impl DropboxRemote<'_> {
    fn full(&self, path: &str) -> String {
        format!("{}/{path}", self.folder)
    }
}

impl Remote for DropboxRemote<'_> {
    fn list(&mut self) -> Result<Vec<RemoteFile>> {
        let prefix = format!("{}/", self.folder.to_lowercase());
        let mut out: Vec<RemoteFile> = listing(self.s, &self.folder, true)?
            .into_iter()
            .filter(|e| e.tag == "file")
            .filter_map(|e| {
                let lower = e.path_lower?;
                let display = e.path_display.unwrap_or_else(|| lower.clone());
                // Dropbox compares names without case; keep the shown one.
                lower.starts_with(&prefix).then(|| RemoteFile {
                    path: display[prefix.len()..].to_string(),
                    rev: e.rev.unwrap_or_default(),
                })
            })
            .collect();
        out.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(out)
    }

    fn get(&mut self, path: &str) -> Result<Vec<u8>> {
        let url = format!("{}/files/download", self.s.ends.content);
        let arg = header_json(&json!({ "path": self.full(path) }));
        let reply = self
            .s
            .call("POST", &url, &[("Dropbox-API-Arg", arg)], &|| Body::Empty)?;
        if !reply.ok() {
            return Err(drive_error("드라이브에서 파일을 받지 못함", &reply));
        }
        Ok(reply.body)
    }

    fn put(&mut self, path: &str, bytes: &[u8], expected: Option<&str>) -> Result<Put> {
        let mode = match expected {
            Some(rev) => json!({ ".tag": "update", "update": rev }),
            None => json!({ ".tag": "add" }),
        };
        let arg = header_json(&json!({
            "path": self.full(path),
            "mode": mode,
            "autorename": false,
            "mute": true,
            "strict_conflict": true,
        }));
        let url = format!("{}/files/upload", self.s.ends.content);
        let reply = self
            .s
            .call("POST", &url, &[("Dropbox-API-Arg", arg)], &|| {
                Body::Bytes(bytes, "application/octet-stream")
            })?;
        // A 409 is a version conflict, unless the drive is full.
        if reply.status == 409 && !space_full(&reply) {
            return Ok(Put::Changed);
        }
        if !reply.ok() {
            return Err(drive_error("드라이브에 올리지 못함", &reply));
        }
        let entry: Entry = reply.json()?;
        Ok(Put::Done(entry.rev.unwrap_or_default()))
    }

    fn remove(&mut self, path: &str, expected: &str) -> Result<bool> {
        let reply = rpc(
            self.s,
            "files/delete_v2",
            json!({ "path": self.full(path), "parent_rev": expected }),
        )?;
        if reply.ok() {
            return Ok(true);
        }
        if reply.status == 409 {
            let summary = reply
                .json::<Failure>()
                .map(|f| f.error_summary)
                .unwrap_or_default();
            return Ok(summary.contains("not_found"));
        }
        Err(drive_error("드라이브에서 지우지 못함", &reply))
    }
}
