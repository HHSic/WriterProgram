//! The Google Drive stand-in.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use super::server::{Request, Response, bytes_reply, json_reply};

/// Google Drive v3 with drive.file: files by id, folders are files too.
#[derive(Default)]
pub struct Google {
    files: BTreeMap<String, GFile>,
    next: u64,
}

#[derive(Clone)]
struct GFile {
    name: String,
    folder: bool,
    parent: String,
    bytes: Vec<u8>,
    version: u64,
    trashed: bool,
}

fn quoted_after(q: &str, marker: &str) -> Option<String> {
    let start = q.find(marker)? + marker.len();
    let rest = &q[start..];
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => out.push(chars.next()?),
            '\'' => return Some(out),
            c => out.push(c),
        }
    }
    None
}

fn quoted_before(q: &str, marker: &str) -> Option<String> {
    let end = q.find(marker)?;
    let head = q[..end].trim_end().strip_suffix('\'')?;
    let start = head.rfind('\'')?;
    Some(head[start + 1..].to_string())
}

impl Google {
    /// An empty drive: only 내 드라이브 itself, under the id "root".
    pub fn new() -> Google {
        let mut google = Google::default();
        google.files.insert(
            "root".into(),
            GFile {
                name: "내 드라이브".into(),
                folder: true,
                parent: String::new(),
                bytes: Vec::new(),
                version: 1,
                trashed: false,
            },
        );
        google
    }

    fn meta(&self, id: &str) -> Value {
        let f = &self.files[id];
        json!({ "id": id, "name": f.name, "mimeType": if f.folder { "application/vnd.google-apps.folder" } else { "application/octet-stream" }, "parents": [f.parent], "version": f.version.to_string() })
    }

    fn add(&mut self, name: &str, parent: &str, folder: bool, bytes: Vec<u8>) -> String {
        self.next += 1;
        let id = format!("g{}", self.next);
        self.files.insert(
            id.clone(),
            GFile {
                name: name.into(),
                folder,
                parent: parent.into(),
                bytes,
                version: 1,
                trashed: false,
            },
        );
        id
    }

    pub fn handle(&mut self, req: &Request) -> Response {
        let api = "/drive/v3";
        let up = "/upload/drive/v3";
        let path = req.path.as_str();
        if path == format!("{api}/about") {
            return json_reply(
                200,
                json!({ "user": { "displayName": "윤서하", "emailAddress": "writer@example.com" } }),
            );
        }
        if path == format!("{api}/files") && req.method == "GET" {
            let q = req.query.get("q").cloned().unwrap_or_default();
            let name = quoted_after(&q, "name = '");
            let parent = quoted_before(&q, "in parents");
            let folders_only = q.contains("mimeType = 'application/vnd.google-apps.folder'");
            let files: Vec<Value> = self
                .files
                .iter()
                .filter(|(_, f)| !f.trashed)
                .filter(|(_, f)| name.as_ref().is_none_or(|n| &f.name == n))
                .filter(|(_, f)| parent.as_ref().is_none_or(|p| &f.parent == p))
                .filter(|(_, f)| !folders_only || f.folder)
                .map(|(id, _)| self.meta(id))
                .collect();
            return json_reply(200, json!({ "files": files }));
        }
        if path == format!("{api}/files") && req.method == "POST" {
            let meta = req.json();
            let id = self.add(
                meta["name"].as_str().unwrap(),
                meta["parents"][0].as_str().unwrap(),
                true,
                Vec::new(),
            );
            return json_reply(200, self.meta(&id));
        }
        if path == format!("{up}/files") && req.method == "POST" {
            assert_eq!(
                req.query.get("uploadType").map(String::as_str),
                Some("multipart")
            );
            let ct = req.header("content-type").unwrap().to_string();
            let boundary = ct.split("boundary=").nth(1).unwrap();
            let sep = format!("--{boundary}");
            let body = req.body.clone();
            let text = String::from_utf8_lossy(&body).into_owned();
            let parts: Vec<&str> = text.split(&sep).collect();
            let meta: Value =
                serde_json::from_str(parts[1].split("\r\n\r\n").nth(1).unwrap().trim()).unwrap();
            // The file part, as bytes: after its blank line, up to the closing boundary.
            let start = find(
                &body,
                format!("{sep}\r\nContent-Type: application/octet-stream\r\n\r\n").as_bytes(),
            )
            .unwrap()
                + sep.len()
                + "\r\nContent-Type: application/octet-stream\r\n\r\n".len();
            let end = find(&body, format!("\r\n{sep}--").as_bytes()).unwrap();
            let id = self.add(
                meta["name"].as_str().unwrap(),
                meta["parents"][0].as_str().unwrap(),
                false,
                body[start..end].to_vec(),
            );
            return json_reply(200, self.meta(&id));
        }
        if let Some(id) = path.strip_prefix(&format!("{up}/files/")) {
            let f = self.files.get_mut(id).unwrap();
            f.bytes = req.body.clone();
            f.version += 1;
            return json_reply(200, self.meta(id));
        }
        if let Some(id) = path.strip_prefix(&format!("{api}/files/")) {
            let Some(f) = self.files.get_mut(id) else {
                return json_reply(404, json!({}));
            };
            match req.method.as_str() {
                "GET" if req.query.get("alt").map(String::as_str) == Some("media") => {
                    return bytes_reply(f.bytes.clone());
                }
                "GET" => {
                    return json_reply(
                        200,
                        json!({ "version": f.version.to_string(), "trashed": f.trashed }),
                    );
                }
                "PATCH" => {
                    f.trashed = req.json()["trashed"].as_bool().unwrap_or(false);
                    f.version += 1;
                    return json_reply(200, json!({ "id": id }));
                }
                _ => {}
            }
        }
        json_reply(
            404,
            json!({ "error": format!("no route {} {path}", req.method) }),
        )
    }
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}
