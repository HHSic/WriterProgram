//! A small HTTP client for sign-in and the drives' APIs (ureq, blocking: the
//! app runs sync passes off the screen's thread).

use std::time::Duration;

use serde::de::DeserializeOwned;

use crate::{Error, Result};

/// Files larger than this are not taken from a drive (a manuscript file is a
/// few dozen kilobytes; this only guards against something odd).
const MAX_BODY: u64 = 256 * 1024 * 1024;

pub enum Body<'a> {
    Empty,
    Bytes(&'a [u8], &'a str),
    Form(Vec<(&'a str, String)>),
    Json(serde_json::Value),
}

#[derive(Debug)]
pub struct Reply {
    pub status: u16,
    pub body: Vec<u8>,
}

impl Reply {
    pub fn ok(&self) -> bool {
        (200..300).contains(&self.status)
    }

    pub fn json<T: DeserializeOwned>(&self) -> Result<T> {
        serde_json::from_slice(&self.body)
            .map_err(|e| Error::Drive(format!("드라이브의 대답을 읽지 못함 ({e})")))
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body)
            .chars()
            .take(300)
            .collect()
    }
}

/// The HTTP client shared by sign-in and the drives.
#[derive(Clone)]
pub struct Http {
    agent: ureq::Agent,
}

impl Default for Http {
    fn default() -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(60)))
            .http_status_as_error(false)
            .user_agent("WriterProgram")
            .build()
            .into();
        Http { agent }
    }
}

impl Http {
    pub(crate) fn send(
        &self,
        method: &str,
        url: &str,
        headers: &[(&str, String)],
        body: Body,
    ) -> Result<Reply> {
        let offline = |e: ureq::Error| Error::Offline(e.to_string());
        let response = match method {
            "GET" | "DELETE" => {
                let mut req = if method == "GET" {
                    self.agent.get(url)
                } else {
                    self.agent.delete(url)
                };
                for (k, v) in headers {
                    req = req.header(*k, v.as_str());
                }
                req.call()
            }
            _ => {
                let mut req = match method {
                    "POST" => self.agent.post(url),
                    "PUT" => self.agent.put(url),
                    "PATCH" => self.agent.patch(url),
                    other => return Err(Error::Invalid(format!("unsupported method {other}"))),
                };
                for (k, v) in headers {
                    req = req.header(*k, v.as_str());
                }
                match body {
                    Body::Empty => req.send_empty(),
                    Body::Bytes(bytes, content_type) => req.content_type(content_type).send(bytes),
                    Body::Form(fields) => req.send_form(fields),
                    Body::Json(value) => req.content_type("application/json").send(
                        serde_json::to_vec(&value)
                            .expect("json serializes")
                            .as_slice(),
                    ),
                }
            }
        };
        let mut response = response.map_err(offline)?;
        let status = response.status().as_u16();
        let body = response
            .body_mut()
            .with_config()
            .limit(MAX_BODY)
            .read_to_vec()
            .map_err(offline)?;
        Ok(Reply { status, body })
    }
}

/// Visits a page and follows its redirects, reading the answer (used to play
/// the browser against a stand-in drive).
pub fn fetch(url: &str) -> Result<()> {
    let reply = Http::default().send("GET", url, &[], Body::Empty)?;
    if reply.ok() {
        Ok(())
    } else {
        Err(Error::Drive(format!("{} {}", reply.status, reply.text())))
    }
}

/// Percent-encodes one path part or query value.
pub(crate) fn enc(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes())
        .collect::<String>()
        .replace('+', "%20")
}

/// JSON for an HTTP header: characters outside ASCII written as `\uXXXX`
/// (Dropbox reads its `Dropbox-API-Arg` header that way).
pub(crate) fn header_json(value: &serde_json::Value) -> String {
    let text = serde_json::to_string(value).expect("json serializes");
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_ascii() {
            out.push(c);
        } else {
            let mut units = [0u16; 2];
            for unit in c.encode_utf16(&mut units) {
                out.push_str(&format!("\\u{unit:04x}"));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_json_escapes_korean() {
        let v = serde_json::json!({ "path": "/달빛 서점/a.md" });
        // Each Korean letter as a backslash, "u" and four hex digits.
        let esc = |hex: &str| format!("{}u{hex}", char::from(92));
        let expected = format!(
            "{{\"path\":\"/{}{} {}{}/a.md\"}}",
            esc("b2ec"),
            esc("be5b"),
            esc("c11c"),
            esc("c810")
        );
        assert_eq!(header_json(&v), expected);
    }

    #[test]
    fn encodes_paths() {
        assert_eq!(
            enc("다른 기기 (1).md"),
            "%EB%8B%A4%EB%A5%B8%20%EA%B8%B0%EA%B8%B0%20%281%29.md"
        );
    }
}
