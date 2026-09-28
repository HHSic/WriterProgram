//! Stand-ins for Google Drive, OneDrive and Dropbox: small local servers that
//! answer like the real APIs (as documented), including the sign-in page and
//! the token endpoint (which checks the one-time code and PKCE). Used by the
//! tests and by the `stand_in` example.

#![allow(dead_code)]

use std::collections::{BTreeMap, HashMap};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

use serde_json::{Value, json};
use writer_core::store::sha256;
use writer_sync::providers::{Endpoints, Provider};

// ---------------------------------------------------------------------------
// A tiny HTTP server

pub struct Request {
    method: String,
    path: String,
    query: HashMap<String, String>,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

impl Request {
    fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap_or(Value::Null)
    }

    fn form(&self) -> HashMap<String, String> {
        url::form_urlencoded::parse(&self.body)
            .into_owned()
            .collect()
    }

    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .get(&name.to_ascii_lowercase())
            .map(String::as_str)
    }
}

pub struct Response {
    status: u16,
    body: Vec<u8>,
    content_type: &'static str,
    location: Option<String>,
}

fn json_reply(status: u16, value: Value) -> Response {
    Response {
        status,
        body: serde_json::to_vec(&value).unwrap(),
        content_type: "application/json",
        location: None,
    }
}

fn bytes_reply(body: Vec<u8>) -> Response {
    Response {
        status: 200,
        body,
        content_type: "application/octet-stream",
        location: None,
    }
}

fn read_request(stream: &mut TcpStream) -> Option<Request> {
    let mut reader = BufReader::new(stream.try_clone().ok()?);
    let mut line = String::new();
    reader.read_line(&mut line).ok()?;
    let mut parts = line.split_whitespace();
    let method = parts.next()?.to_string();
    let target = parts.next()?.to_string();
    let mut headers = HashMap::new();
    loop {
        let mut h = String::new();
        reader.read_line(&mut h).ok()?;
        let h = h.trim_end();
        if h.is_empty() {
            break;
        }
        if let Some((k, v)) = h.split_once(':') {
            headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
        }
    }
    let len: usize = headers
        .get("content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let mut body = vec![0u8; len];
    reader.read_exact(&mut body).ok()?;
    let (path, query) = target.split_once('?').unwrap_or((&target, ""));
    let path = url::form_urlencoded::parse(format!("p={}", path.replace('+', "%2B")).as_bytes())
        .next()
        .map(|(_, v)| v.into_owned())
        .unwrap_or_default();
    let query = url::form_urlencoded::parse(query.as_bytes())
        .into_owned()
        .collect();
    Some(Request {
        method,
        path,
        query,
        headers,
        body,
    })
}

pub type Handler = Box<dyn FnMut(&Request) -> Response + Send>;

/// Serves `handler` on `port` (0: any free one) until the process ends;
/// returns the base URL.
pub fn serve_on(port: u16, handler: Handler) -> String {
    let listener = TcpListener::bind(("127.0.0.1", port)).unwrap();
    let port = listener.local_addr().unwrap().port();
    let handler = Arc::new(Mutex::new(handler));
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let handler = handler.clone();
            thread::spawn(move || {
                let Some(req) = read_request(&mut stream) else {
                    return;
                };
                let res = (handler.lock().unwrap())(&req);
                let reason = if res.status < 400 { "OK" } else { "Error" };
                let location = res
                    .location
                    .as_ref()
                    .map(|l| format!("Location: {l}\r\n"))
                    .unwrap_or_default();
                let head = format!(
                    "HTTP/1.1 {} {reason}\r\nContent-Type: {}\r\nContent-Length: {}\r\n{location}Connection: close\r\n\r\n",
                    res.status,
                    res.content_type,
                    res.body.len()
                );
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(&res.body);
            });
        }
    });
    format!("http://127.0.0.1:{port}")
}

// ---------------------------------------------------------------------------
// Signing in (shared by the three stand-ins)

const CODE: &str = "code-from-the-sign-in-page";

#[derive(Default)]
struct SignIn {
    challenge: String,
    redirect: String,
    /// Access tokens handed out so far.
    issued: Vec<String>,
}

/// The token endpoint: checks the one-time code and PKCE, hands out tokens.
/// The first access token is short-lived, so the app has to renew it at once.
fn token_endpoint(req: &Request, state: &mut SignIn) -> Response {
    let form = req.form();
    match form.get("grant_type").map(String::as_str) {
        Some("authorization_code") => {
            let verifier = form.get("code_verifier").cloned().unwrap_or_default();
            let challenge = base64_url(&sha256(verifier.as_bytes()));
            if form.get("code").map(String::as_str) != Some(CODE)
                || challenge != state.challenge
                || form.get("redirect_uri") != Some(&state.redirect)
            {
                return json_reply(400, json!({ "error": "invalid_grant" }));
            }
            state.issued.push("AT1".into());
            json_reply(
                200,
                json!({ "access_token": "AT1", "refresh_token": "RT1", "expires_in": 30 }),
            )
        }
        Some("refresh_token") if form.get("refresh_token").map(String::as_str) == Some("RT1") => {
            let token = format!("AT{}", state.issued.len() + 1);
            state.issued.push(token.clone());
            json_reply(200, json!({ "access_token": token, "expires_in": 3600 }))
        }
        _ => json_reply(400, json!({ "error": "invalid_grant" })),
    }
}

fn base64_url(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = (chunk[0] as u32) << 16
            | (*chunk.get(1).unwrap_or(&0) as u32) << 8
            | *chunk.get(2).unwrap_or(&0) as u32;
        for i in 0..(chunk.len() + 1) {
            out.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
        }
    }
    out
}

/// The drive's sign-in page, answered as if the writer signed in at once.
fn sign_in_page(req: &Request, state: &mut SignIn) -> Response {
    assert_eq!(
        req.query.get("code_challenge_method").map(String::as_str),
        Some("S256")
    );
    assert_eq!(
        req.query.get("response_type").map(String::as_str),
        Some("code")
    );
    state.challenge = req.query["code_challenge"].clone();
    state.redirect = req.query["redirect_uri"].clone();
    let back = format!(
        "{}?code={CODE}&state={}",
        state.redirect,
        url::form_urlencoded::byte_serialize(req.query["state"].as_bytes()).collect::<String>()
    );
    Response {
        status: 302,
        body: Vec::new(),
        content_type: "text/plain",
        location: Some(back),
    }
}

fn authorized(req: &Request, sign_in: &SignIn) -> bool {
    // Only a renewed token works: the first one ran out right away.
    req.header("authorization")
        .and_then(|h| h.strip_prefix("Bearer "))
        .is_some_and(|t| t != "AT1" && sign_in.issued.iter().any(|i| i == t))
}

// ---------------------------------------------------------------------------
// Stand-in drives

/// Google Drive v3 with drive.file: files by id, folders are files too.
#[derive(Default)]
struct Google {
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

    fn handle(&mut self, req: &Request) -> Response {
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

/// OneDrive through Graph: items with ids, reached by path under approot.
#[derive(Default)]
struct OneDrive {
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

    fn handle(&mut self, req: &Request) -> Response {
        let base = "/v1.0";
        let path = req.path.strip_prefix(base).unwrap_or(&req.path).to_string();
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

/// Dropbox v2: files by path (compared without case), revisions.
#[derive(Default)]
struct Dropbox {
    files: BTreeMap<String, (String, Vec<u8>, u64)>,
    folders: BTreeMap<String, String>,
}

impl Dropbox {
    fn entry(&self, lower: &str) -> Value {
        let (display, _, rev) = &self.files[lower];
        json!({ ".tag": "file", "name": display.rsplit('/').next(), "path_lower": lower, "path_display": display, "rev": format!("r{rev}") })
    }

    fn handle(&mut self, req: &Request) -> Response {
        let conflict = |summary: &str| json_reply(409, json!({ "error_summary": summary }));
        match req.path.as_str() {
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

/// A stand-in for `provider` on `port` (0: any free one).
pub fn stand_in_on(provider: Provider, port: u16) -> Endpoints {
    let sign_in = Arc::new(Mutex::new(SignIn::default()));
    let state = sign_in.clone();
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
    let mut google = Some(google);
    let mut onedrive = Some(OneDrive::default());
    let mut dropbox = Some(Dropbox::default());
    let (mut g, mut o, mut d) = (google.take(), onedrive.take(), dropbox.take());
    let base = serve_on(
        port,
        Box::new(move |req: &Request| {
            let mut s = state.lock().unwrap();
            if req.path == "/token" {
                return token_endpoint(req, &mut s);
            }
            if req.path == "/auth" {
                return sign_in_page(req, &mut s);
            }
            if !authorized(req, &s) {
                return json_reply(401, json!({ "error": "invalid_token" }));
            }
            drop(s);
            match provider {
                Provider::Google => g.as_mut().unwrap().handle(req),
                Provider::Onedrive => o.as_mut().unwrap().handle(req),
                Provider::Dropbox => d.as_mut().unwrap().handle(req),
            }
        }),
    );
    Endpoints::at(provider, &base)
}

/// A stand-in on any free port.
pub fn stand_in(provider: Provider) -> Endpoints {
    stand_in_on(provider, 0)
}
