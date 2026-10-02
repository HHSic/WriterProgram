//! Stand-ins for Google Drive, OneDrive and Dropbox: small local servers that
//! answer like the real APIs (as documented), including the sign-in page and
//! the token endpoint (which checks the one-time code and PKCE). Used by the
//! tests and by the `stand_in` example.
//!
//! `server` is the HTTP server, `sign_in` the part all three share, and
//! `google`, `onedrive` and `dropbox` each drive's own API.
//!
//! How full the drive is can be set with [`set_room`] (or by hand: `POST
//! /stand-in/room` with the [`Room`] as JSON). A full stand-in refuses
//! uploads the way its drive does.

#![allow(dead_code)]

mod dropbox;
mod google;
mod onedrive;
mod server;
mod sign_in;

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::json;
use writer_sync::providers::{Endpoints, Provider};

use dropbox::Dropbox;
use google::Google;
use onedrive::OneDrive;
use server::{Request, json_reply, serve_on};
use sign_in::{SignIn, authorized, sign_in_page, token_endpoint};

/// How full a stand-in drive is.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Room {
    /// The drive's size; none for no limit.
    pub total: Option<u64>,
    /// Taken by the writer's other files (mail, photos …).
    pub elsewhere: u64,
    /// False: the drive refuses to say how full it is (a narrow permission).
    pub readable: bool,
}

impl Default for Room {
    fn default() -> Self {
        Room {
            total: None,
            elsewhere: 0,
            readable: true,
        }
    }
}

impl Room {
    /// Whether the drive holds `stored` bytes of the app's files.
    pub fn fits(&self, stored: u64) -> bool {
        self.total.is_none_or(|t| self.elsewhere + stored <= t)
    }
}

/// A stand-in for `provider` on `port` (0: any free one).
pub fn stand_in_on(provider: Provider, port: u16) -> Endpoints {
    let sign_in = Arc::new(Mutex::new(SignIn::default()));
    let state = sign_in.clone();
    let (mut g, mut o, mut d) = (Google::new(), OneDrive::default(), Dropbox::default());
    let mut room = Room::default();
    let base = serve_on(
        port,
        Box::new(move |req: &Request| {
            if req.path == "/stand-in/room" {
                return match serde_json::from_slice(&req.body) {
                    Ok(r) => {
                        room = r;
                        json_reply(200, json!({}))
                    }
                    Err(e) => json_reply(400, json!({ "error": e.to_string() })),
                };
            }
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
                Provider::Google => g.handle(req, &room),
                Provider::Onedrive => o.handle(req, &room),
                Provider::Dropbox => d.handle(req, &room),
            }
        }),
    );
    Endpoints::at(provider, &base)
}

/// A stand-in on any free port.
pub fn stand_in(provider: Provider) -> Endpoints {
    stand_in_on(provider, 0)
}

/// Sets how full the stand-in at `ends` is.
pub fn set_room(ends: &Endpoints, room: Room) {
    let addr = ends
        .token
        .trim_start_matches("http://")
        .trim_end_matches("/token")
        .to_string();
    let body = serde_json::to_vec(&room).unwrap();
    let mut stream = TcpStream::connect(addr).unwrap();
    write!(
        stream,
        "POST /stand-in/room HTTP/1.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
        body.len()
    )
    .unwrap();
    stream.write_all(&body).unwrap();
    let mut answer = String::new();
    stream.read_to_string(&mut answer).unwrap();
    assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
}
