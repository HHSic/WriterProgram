//! The Dropbox stand-in.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use super::Room;
use super::server::{Request, Response, bytes_reply, json_reply};

/// Dropbox v2: files by path (compared without case), revisions.
#[derive(Default)]
pub struct Dropbox {
    files: BTreeMap<String, (String, Vec<u8>, u64)>,
    folders: BTreeMap<String, String>,
}

impl Dropbox {
    fn entry(&self, lower: &str) -> Value {
        let (display, _, rev) = &self.files[lower];
        json!({ ".tag": "file", "name": display.rsplit('/').next(), "path_lower": lower, "path_display": display, "rev": format!("r{rev}") })
    }

    fn stored(&self) -> u64 {
        self.files.values().map(|(_, b, _)| b.len() as u64).sum()
    }

    pub fn handle(&mut self, req: &Request, room: &Room) -> Response {
        let conflict = |summary: &str| json_reply(409, json!({ "error_summary": summary }));
        match req.path.as_str() {
            "/2/users/get_space_usage" if !room.readable => json_reply(
                401,
                json!({ "error_summary": "missing_scope/..", "error": { ".tag": "missing_scope", "required_scope": "account_info.read" } }),
            ),
            "/2/users/get_space_usage" => json_reply(
                200,
                json!({ "used": room.elsewhere + self.stored(), "allocation": { ".tag": "individual", "allocated": room.total.unwrap_or(0) } }),
            ),
            "/2/users/get_current_account" => json_reply(
                200,
                json!({ "name": { "display_name": "윤서하" }, "email": "writer@example.com" }),
            ),
            "/2/files/get_metadata" => {
                let path = req.json()["path"].as_str().unwrap().to_lowercase();
                if self.folders.contains_key(&path) {
                    json_reply(200, json!({ ".tag": "folder", "path_lower": path }))
                } else {
                    conflict("path/not_found/..")
                }
            }
            "/2/files/create_folder_v2" => {
                let path = req.json()["path"].as_str().unwrap().to_string();
                self.folders.insert(path.to_lowercase(), path.clone());
                json_reply(200, json!({ "metadata": { "path_display": path } }))
            }
            "/2/files/list_folder" => {
                let arg = req.json();
                let path = arg["path"].as_str().unwrap().to_lowercase();
                let recursive = arg["recursive"].as_bool().unwrap_or(false);
                let prefix = format!("{path}/");
                let mut entries: Vec<Value> = Vec::new();
                for (lower, display) in &self.folders {
                    let inside =
                        lower.starts_with(&prefix) || (path.is_empty() && lower.starts_with('/'));
                    let rest = &lower[prefix.len().min(lower.len())..];
                    if inside && (recursive || !rest.contains('/')) {
                        entries.push(json!({ ".tag": "folder", "name": display.rsplit('/').next(), "path_lower": lower, "path_display": display }));
                    }
                }
                if !path.is_empty() {
                    for lower in self.files.keys() {
                        if lower.starts_with(&prefix)
                            && (recursive || !lower[prefix.len()..].contains('/'))
                        {
                            entries.push(self.entry(lower));
                        }
                    }
                }
                json_reply(
                    200,
                    json!({ "entries": entries, "cursor": "c", "has_more": false }),
                )
            }
            "/2/files/upload" => {
                let arg: Value =
                    serde_json::from_str(req.header("dropbox-api-arg").unwrap()).unwrap();
                assert!(
                    req.header("dropbox-api-arg").unwrap().is_ascii(),
                    "the header must be ASCII"
                );
                let path = arg["path"].as_str().unwrap().to_string();
                let lower = path.to_lowercase();
                let current = self.files.get(&lower).map(|(_, _, r)| format!("r{r}"));
                match arg["mode"][".tag"].as_str().unwrap() {
                    "add" if current.is_some() => return conflict("path/conflict/file/.."),
                    "update" if current.as_deref() != arg["mode"]["update"].as_str() => {
                        return conflict("path/conflict/file/..");
                    }
                    _ => {}
                }
                let before = self.files.get(&lower).map_or(0, |(_, b, _)| b.len() as u64);
                if !room.fits(self.stored() - before + req.body.len() as u64) {
                    return json_reply(
                        409,
                        json!({ "error_summary": "path/insufficient_space/..", "error": { ".tag": "path", "reason": { ".tag": "insufficient_space" }, "upload_session_id": "s1" } }),
                    );
                }
                let rev = self.files.get(&lower).map_or(1, |(_, _, r)| r + 1);
                self.files
                    .insert(lower.clone(), (path, req.body.clone(), rev));
                // The upload's answer has no ".tag".
                let mut e = self.entry(&lower);
                e.as_object_mut().unwrap().remove(".tag");
                json_reply(200, e)
            }
            "/2/files/download" => {
                let arg: Value =
                    serde_json::from_str(req.header("dropbox-api-arg").unwrap()).unwrap();
                match self
                    .files
                    .get(&arg["path"].as_str().unwrap().to_lowercase())
                {
                    Some((_, bytes, _)) => bytes_reply(bytes.clone()),
                    None => conflict("path/not_found/.."),
                }
            }
            "/2/files/delete_v2" => {
                let arg = req.json();
                let lower = arg["path"].as_str().unwrap().to_lowercase();
                match self.files.get(&lower) {
                    None => conflict("path_lookup/not_found/.."),
                    Some((_, _, r))
                        if Some(format!("r{r}").as_str()) != arg["parent_rev"].as_str() =>
                    {
                        conflict("path_write/conflict/..")
                    }
                    Some(_) => {
                        let e = self.entry(&lower);
                        self.files.remove(&lower);
                        json_reply(200, json!({ "metadata": e }))
                    }
                }
            }
            other => json_reply(404, json!({ "error_summary": format!("no route {other}") })),
        }
    }
}
