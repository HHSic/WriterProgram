//! Stand-ins for Google Drive, OneDrive and Dropbox: small local servers that
//! answer like the real APIs (as documented), including the sign-in page and
//! the token endpoint (which checks the one-time code and PKCE). Used by the
//! tests and by the `stand_in` example.
//!
//! `server` is the HTTP server, `sign_in` the part all three share, and
//! `google`, `onedrive` and `dropbox` each drive's own API.

#![allow(dead_code)]

mod dropbox;
mod google;
mod onedrive;
mod server;
mod sign_in;

use std::sync::{Arc, Mutex};

use serde_json::json;
use writer_sync::providers::{Endpoints, Provider};

use dropbox::Dropbox;
use google::Google;
use onedrive::OneDrive;
use server::{Request, json_reply, serve_on};
use sign_in::{SignIn, authorized, sign_in_page, token_endpoint};

/// A stand-in for `provider` on `port` (0: any free one).
pub fn stand_in_on(provider: Provider, port: u16) -> Endpoints {
    let sign_in = Arc::new(Mutex::new(SignIn::default()));
    let state = sign_in.clone();
    let (mut g, mut o, mut d) = (Google::new(), OneDrive::default(), Dropbox::default());
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
                Provider::Google => g.handle(req),
                Provider::Onedrive => o.handle(req),
                Provider::Dropbox => d.handle(req),
            }
        }),
    );
    Endpoints::at(provider, &base)
}

/// A stand-in on any free port.
pub fn stand_in(provider: Provider) -> Endpoints {
    stand_in_on(provider, 0)
}
