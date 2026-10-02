//! The OneDrive stand-in.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use super::Room;
use super::server::{Request, Response, bytes_reply, json_reply};

/// OneDrive through Graph: items with ids, reached by path under approot.
#[derive(Default)]
pub struct OneDrive {
    items: BTreeMap<String, OItem>,
    next: u64,
}

#[derive(Clone)]
struct OItem {
    name: String,
    parent: String,
    folder: bool,
    bytes: Vec<u8>,
    n: u64,
}

impl OneDrive {
    fn etag(&self, id: &str) -> String {
        format!("\"{{{id}}},{}\"", self.items[id].n)
    }

    fn json(&self, id: &str) -> Value {
        let i = &self.items[id];
        let mut v = json!({ "id": id, "name": i.name, "eTag": self.etag(id), "parentReference": { "id": i.parent } });
        if i.folder {
            v["folder"] = json!({ "childCount": 0 });
        } else {
            v["file"] = json!({});
        }
        v
    }

    fn child(&self, parent: &str, name: &str) -> Option<String> {
        self.items
            .iter()
            .find(|(_, i)| i.parent == parent && i.name == name)
            .map(|(id, _)| id.clone())
    }

    fn add(&mut self, parent: &str, name: &str, folder: bool, bytes: Vec<u8>) -> String {
        self.next += 1;
        let id = format!("o{}", self.next);
        self.items.insert(
            id.clone(),
            OItem {
                name: name.into(),
                parent: parent.into(),
                folder,
                bytes,
                n: 1,
            },
        );
        id
    }

    /// Resolves `a/b/c` under approot; with `make`, folders on the way are made.
    fn resolve(&mut self, path: &str, make: bool) -> Option<String> {
        let mut id = "approot".to_string();
        let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
        for (i, part) in parts.iter().enumerate() {
            id = match self.child(&id, part) {
                Some(found) => found,
                None if make && i + 1 < parts.len() => {
                    self.add(&id.clone(), part, true, Vec::new())
                }
                None => return None,
            };
        }
        Some(id)
    }

    /// Folders on the way to a new file are made (as OneDrive does).
    fn ensure_dir(&mut self, dir: &str) -> String {
        let mut id = "approot".to_string();
        for part in dir.split('/').filter(|p| !p.is_empty()) {
            id = match self.child(&id, part) {
                Some(found) => found,
                None => self.add(&id.clone(), part, true, Vec::new()),
            };
        }
        id
    }

    fn stored(&self) -> u64 {
        self.items.values().map(|i| i.bytes.len() as u64).sum()
    }

    pub fn handle(&mut self, req: &Request, room: &Room) -> Response {
        let base = "/v1.0";
        let path = req.path.strip_prefix(base).unwrap_or(&req.path).to_string();
        if path == "/me/drive" {
            // Microsoft lists Files.Read as the least permission for this;
            // with only the app folder it may be refused.
            if !room.readable {
                return json_reply(403, json!({ "error": { "code": "accessDenied" } }));
            }
            let used = room.elsewhere + self.stored();
            let total = room.total.unwrap_or(1 << 50);
            return json_reply(
                200,
                json!({ "quota": { "total": total, "used": used, "remaining": total.saturating_sub(used), "deleted": 0, "state": "normal" } }),
            );
        }
        if path == "/me" {
            return json_reply(
                200,
                json!({ "displayName": "윤서하", "mail": null, "userPrincipalName": "writer@example.com" }),
            );
        }
        if path == "/me/drive/special/approot/children" {
            if req.method == "POST" {
                let body = req.json();
                let name = body["name"].as_str().unwrap();
                if self.child("approot", name).is_some() {
                    return json_reply(409, json!({ "error": { "code": "nameAlreadyExists" } }));
                }
                let id = self.add("approot", name, true, Vec::new());
                return json_reply(201, self.json(&id));
            }
            let value: Vec<Value> = self
                .items
                .iter()
                .filter(|(_, i)| i.parent == "approot")
                .map(|(id, _)| self.json(id))
                .collect();
            return json_reply(200, json!({ "value": value }));
        }
        if let Some(rest) = path.strip_prefix("/me/drive/items/") {
            let id = rest.strip_suffix("/delta").unwrap();
            let mut value = vec![self.json(id)];
            let mut stack = vec![id.to_string()];
            while let Some(p) = stack.pop() {
                for (cid, c) in &self.items {
                    if c.parent == p {
                        value.push(self.json(cid));
                        if c.folder {
                            stack.push(cid.clone());
                        }
                    }
                }
            }
            return json_reply(200, json!({ "value": value, "@odata.deltaLink": "x" }));
        }
        if let Some(rest) = path.strip_prefix("/me/drive/special/approot:/") {
            let (item_path, content) = match rest.strip_suffix(":/content") {
                Some(p) => (p.to_string(), true),
                None => (rest.trim_end_matches(':').to_string(), false),
            };
            match (req.method.as_str(), content) {
                ("GET", false) => {
                    return match self.resolve(&item_path, false) {
                        Some(id) => json_reply(200, self.json(&id)),
                        None => json_reply(404, json!({ "error": { "code": "itemNotFound" } })),
                    };
                }
                ("GET", true) => {
                    let id = self.resolve(&item_path, false).unwrap();
                    return bytes_reply(self.items[&id].bytes.clone());
                }
                ("PUT", true) => {
                    let existing = self.resolve(&item_path, false);
                    let fail_if_exists = req
                        .query
                        .get("@microsoft.graph.conflictBehavior")
                        .map(String::as_str)
                        == Some("fail");
                    if let Some(tag) = req.header("if-match") {
                        match &existing {
                            Some(id) if self.etag(id) == tag => {}
                            _ => {
                                return json_reply(
                                    412,
                                    json!({ "error": { "code": "preconditionFailed" } }),
                                );
                            }
                        }
                    }
                    let before = existing
                        .as_ref()
                        .map_or(0, |id| self.items[id].bytes.len() as u64);
                    if !room.fits(self.stored() - before + req.body.len() as u64) {
                        return json_reply(
                            507,
                            json!({ "error": { "code": "quotaLimitReached", "message": "Insufficient Space Available" } }),
                        );
                    }
                    let id = match existing {
                        Some(_) if fail_if_exists => {
                            return json_reply(
                                409,
                                json!({ "error": { "code": "nameAlreadyExists" } }),
                            );
                        }
                        Some(id) => {
                            let item = self.items.get_mut(&id).unwrap();
                            item.bytes = req.body.clone();
                            item.n += 1;
                            id
                        }
                        None => {
                            let (dir, name) = item_path.rsplit_once('/').unwrap();
                            let parent = self.ensure_dir(dir);
                            self.add(&parent, name, false, req.body.clone())
                        }
                    };
                    return json_reply(200, self.json(&id));
                }
                ("DELETE", false) => {
                    let Some(id) = self.resolve(&item_path, false) else {
                        return json_reply(404, json!({}));
                    };
                    if req.header("if-match") != Some(self.etag(&id).as_str()) {
                        return json_reply(412, json!({}));
                    }
                    self.items.remove(&id);
                    return Response {
                        status: 204,
                        body: Vec::new(),
                        content_type: "text/plain",
                        location: None,
                    };
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
