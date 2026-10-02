//! A tiny HTTP server for the stand-ins.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

use serde_json::Value;

pub struct Request {
    pub method: String,
    pub path: String,
    pub query: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Request {
    pub fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap_or(Value::Null)
    }

    pub fn form(&self) -> HashMap<String, String> {
        url::form_urlencoded::parse(&self.body)
            .into_owned()
            .collect()
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .get(&name.to_ascii_lowercase())
            .map(String::as_str)
    }
}

pub struct Response {
    pub status: u16,
    pub body: Vec<u8>,
    pub content_type: &'static str,
    pub location: Option<String>,
}

pub fn json_reply(status: u16, value: Value) -> Response {
    Response {
        status,
        body: serde_json::to_vec(&value).unwrap(),
        content_type: "application/json",
        location: None,
    }
}

pub fn bytes_reply(body: Vec<u8>) -> Response {
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
