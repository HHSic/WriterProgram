//! Google Drive (API v3) with the `drive.file` permission: the app sees only
//! the files it made. Projects live in a "WriterProgram" folder in 내 드라이브.
//! Drive keeps files by id, so paths are worked out from the folder tree.
//! Versions are Drive's `version` numbers; a file removed here goes to Drive's
//! trash, where the writer can still find it.

use std::collections::HashMap;

use serde::Deserialize;
use serde_json::json;

use super::{Account, Session, drive_error, path_in};
use crate::http::{Body, enc};
use crate::remote::{Put, Remote, RemoteFile, split};
use crate::{Error, Result};

const FOLDER: &str = "application/vnd.google-apps.folder";
pub const APP_FOLDER: &str = "WriterProgram";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct About {
    user: AboutUser,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AboutUser {
    #[serde(default)]
    display_name: String,
    #[serde(default)]
    email_address: String,
}

pub fn account(s: &Session) -> Result<Account> {
    let url = format!("{}/about?fields=user(displayName,emailAddress)", s.ends.api);
    let reply = s.call("GET", &url, &[], &|| Body::Empty)?;
    if !reply.ok() {
        return Err(drive_error("계정 정보를 읽지 못함", &reply));
    }
    let about: About = reply.json()?;
    Ok(Account {
        name: about.user.display_name,
        email: about.user.email_address,
    })
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct File {
    id: String,
    name: String,
    #[serde(default)]
    mime_type: String,
    #[serde(default)]
    parents: Vec<String>,
    #[serde(default)]
    version: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileList {
    #[serde(default)]
    files: Vec<File>,
    #[serde(default)]
    next_page_token: Option<String>,
}

fn query(s: &Session, q: &str) -> Result<Vec<File>> {
    let mut out = Vec::new();
    let mut page: Option<String> = None;
    loop {
        let mut url = format!(
            "{}/files?q={}&spaces=drive&pageSize=1000&fields={}",
            s.ends.api,
            enc(q),
            enc("nextPageToken,files(id,name,mimeType,parents,version)")
        );
        if let Some(token) = &page {
            url.push_str(&format!("&pageToken={}", enc(token)));
        }
        let reply = s.call("GET", &url, &[], &|| Body::Empty)?;
        if !reply.ok() {
            return Err(drive_error("드라이브 목록을 읽지 못함", &reply));
        }
        let list: FileList = reply.json()?;
        out.extend(list.files);
        match list.next_page_token {
            Some(token) => page = Some(token),
            None => return Ok(out),
        }
    }
}

/// A name inside a Drive query, quoted.
fn quoted(name: &str) -> String {
    format!("'{}'", name.replace('\\', "\\\\").replace('\'', "\\'"))
}

fn make_folder(s: &Session, name: &str, parent: &str) -> Result<String> {
    let url = format!("{}/files?fields=id", s.ends.api);
    let meta = json!({ "name": name, "mimeType": FOLDER, "parents": [parent] });
    let reply = s.call("POST", &url, &[], &|| Body::Json(meta.clone()))?;
    if !reply.ok() {
        return Err(drive_error("드라이브에 폴더를 만들지 못함", &reply));
    }
    Ok(reply.json::<File>()?.id)
}

fn child_folder(s: &Session, parent: &str, name: &str) -> Result<Option<String>> {
    let q = format!(
        "name = {} and mimeType = '{FOLDER}' and {} in parents and trashed = false",
        quoted(name),
        quoted(parent)
    );
    Ok(query(s, &q)?.into_iter().next().map(|f| f.id))
}

/// The app's folder in 내 드라이브, made when missing.
fn app_folder(s: &Session) -> Result<String> {
    match child_folder(s, "root", APP_FOLDER)? {
        Some(id) => Ok(id),
        None => make_folder(s, APP_FOLDER, "root"),
    }
}

/// Project folders on the drive.
pub fn projects(s: &Session) -> Result<Vec<String>> {
    let app = app_folder(s)?;
    let q = format!(
        "mimeType = '{FOLDER}' and {} in parents and trashed = false",
        quoted(&app)
    );
    let mut names: Vec<String> = query(s, &q)?.into_iter().map(|f| f.name).collect();
    names.sort();
    Ok(names)
}

/// A project folder as a remote; made when missing and `create`.
pub fn open<'s>(s: &'s Session, folder: &str, create: bool) -> Result<GoogleRemote<'s>> {
    let app = app_folder(s)?;
    let root = match child_folder(s, &app, folder)? {
        Some(id) => id,
        None if create => make_folder(s, folder, &app)?,
        None => return Err(Error::Drive(format!("드라이브에 '{folder}' 폴더가 없음"))),
    };
    Ok(GoogleRemote {
        s,
        root,
        files: HashMap::new(),
        folders: HashMap::new(),
    })
}

struct Item {
    id: String,
    version: String,
}

pub struct GoogleRemote<'s> {
    s: &'s Session,
    /// The project folder's id.
    root: String,
    files: HashMap<String, Item>,
    /// Folder paths inside the project ("" is the project folder) to ids.
    folders: HashMap<String, String>,
}

impl GoogleRemote<'_> {
    fn item(&mut self, path: &str) -> Result<Option<(String, String)>> {
        if !self.files.contains_key(path) {
            self.list()?;
        }
        Ok(self
            .files
            .get(path)
            .map(|i| (i.id.clone(), i.version.clone())))
    }

    /// The current version of a file, asked of Drive right before a change.
    fn version_now(&self, id: &str) -> Result<Option<String>> {
        let url = format!(
            "{}/files/{id}?fields={}",
            self.s.ends.api,
            enc("version,trashed")
        );
        let reply = self.s.call("GET", &url, &[], &|| Body::Empty)?;
        if reply.status == 404 {
            return Ok(None);
        }
        if !reply.ok() {
            return Err(drive_error("드라이브 파일을 확인하지 못함", &reply));
        }
        #[derive(Deserialize)]
        struct V {
            version: Option<String>,
            #[serde(default)]
            trashed: bool,
        }
        let v: V = reply.json()?;
        Ok(if v.trashed { None } else { v.version })
    }

    fn folder_id(&mut self, dirs: &[&str]) -> Result<String> {
        let mut path = String::new();
        let mut id = self.root.clone();
        for dir in dirs {
            path = if path.is_empty() {
                dir.to_string()
            } else {
                format!("{path}/{dir}")
            };
            id = match self.folders.get(&path) {
                Some(known) => known.clone(),
                None => {
                    let made = match child_folder(self.s, &id, dir)? {
                        Some(found) => found,
                        None => make_folder(self.s, dir, &id)?,
                    };
                    self.folders.insert(path.clone(), made.clone());
                    made
                }
            };
        }
        Ok(id)
    }
}

impl Remote for GoogleRemote<'_> {
    fn list(&mut self) -> Result<Vec<RemoteFile>> {
        let all = query(self.s, "trashed = false")?;
        let by_id: HashMap<&str, &File> = all.iter().map(|f| (f.id.as_str(), f)).collect();
        let root = self.root.clone();
        self.files.clear();
        self.folders.clear();
        let mut out = Vec::new();
        for file in &all {
            let Some(path) = path_in(
                file,
                &root,
                &by_id,
                |f| &f.name,
                |f| f.parents.first().map(String::as_str),
            ) else {
                continue;
            };
            if file.mime_type == FOLDER {
                self.folders.insert(path, file.id.clone());
                continue;
            }
            let version = file.version.clone().unwrap_or_default();
            // Two devices making the same file at once: the first one counts.
            if self.files.contains_key(&path) {
                continue;
            }
            self.files.insert(
                path.clone(),
                Item {
                    id: file.id.clone(),
                    version: version.clone(),
                },
            );
            out.push(RemoteFile { path, rev: version });
        }
        out.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(out)
    }

    fn get(&mut self, path: &str) -> Result<Vec<u8>> {
        let (id, _) = self
            .item(path)?
            .ok_or_else(|| Error::Drive(format!("드라이브에 없음: {path}")))?;
        let url = format!("{}/files/{id}?alt=media", self.s.ends.api);
        let reply = self.s.call("GET", &url, &[], &|| Body::Empty)?;
        if !reply.ok() {
            return Err(drive_error("드라이브에서 파일을 받지 못함", &reply));
        }
        Ok(reply.body)
    }

    fn put(&mut self, path: &str, bytes: &[u8], expected: Option<&str>) -> Result<Put> {
        let existing = self.item(path)?;
        let now = match &existing {
            Some((id, _)) => self.version_now(id)?,
            None => None,
        };
        if now.as_deref() != expected {
            return Ok(Put::Changed);
        }
        let reply = match existing.filter(|_| now.is_some()) {
            Some((id, _)) => {
                let url = format!(
                    "{}/files/{id}?uploadType=media&fields=id,version",
                    self.s.ends.content
                );
                self.s.call("PATCH", &url, &[], &|| {
                    Body::Bytes(bytes, "application/octet-stream")
                })?
            }
            None => {
                let (dirs, name) = split(path);
                let parent = self.folder_id(&dirs)?;
                let boundary = "writerprogram-part-7f3a9c";
                let meta = json!({ "name": name, "parents": [parent] }).to_string();
                let mut body = format!(
                    "--{boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n{meta}\r\n--{boundary}\r\nContent-Type: application/octet-stream\r\n\r\n"
                )
                .into_bytes();
                body.extend_from_slice(bytes);
                body.extend_from_slice(format!("\r\n--{boundary}--").as_bytes());
                let content_type = format!("multipart/related; boundary={boundary}");
                let url = format!(
                    "{}/files?uploadType=multipart&fields=id,version",
                    self.s.ends.content
                );
                self.s
                    .call("POST", &url, &[], &|| Body::Bytes(&body, &content_type))?
            }
        };
        if !reply.ok() {
            return Err(drive_error("드라이브에 올리지 못함", &reply));
        }
        let file: File = reply.json()?;
        let version = file.version.unwrap_or_default();
        self.files.insert(
            path.to_string(),
            Item {
                id: file.id,
                version: version.clone(),
            },
        );
        Ok(Put::Done(version))
    }

    fn remove(&mut self, path: &str, expected: &str) -> Result<bool> {
        let Some((id, _)) = self.item(path)? else {
            return Ok(true);
        };
        match self.version_now(&id)? {
            None => return Ok(true),
            Some(v) if v != expected => return Ok(false),
            Some(_) => {}
        }
        // To Drive's trash, where the writer can still find it.
        let url = format!("{}/files/{id}", self.s.ends.api);
        let reply = self.s.call("PATCH", &url, &[], &|| {
            Body::Json(json!({ "trashed": true }))
        })?;
        if !reply.ok() && reply.status != 404 {
            return Err(drive_error("드라이브에서 지우지 못함", &reply));
        }
        self.files.remove(path);
        Ok(true)
    }
}
