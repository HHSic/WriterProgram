//! Signing in, shared by the three stand-ins: the sign-in page, the token
//! endpoint, and which access tokens are good.

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde_json::json;
use writer_core::store::sha256;

use super::server::{Request, Response, json_reply};

const CODE: &str = "code-from-the-sign-in-page";

#[derive(Default)]
pub struct SignIn {
    challenge: String,
    redirect: String,
    /// Access tokens handed out so far.
    issued: Vec<String>,
}

/// The token endpoint: checks the one-time code and PKCE, hands out tokens.
/// The first access token is short-lived, so the app has to renew it at once.
pub fn token_endpoint(req: &Request, state: &mut SignIn) -> Response {
    let form = req.form();
    match form.get("grant_type").map(String::as_str) {
        Some("authorization_code") => {
            let verifier = form.get("code_verifier").cloned().unwrap_or_default();
            let challenge = URL_SAFE_NO_PAD.encode(sha256(verifier.as_bytes()));
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

/// The drive's sign-in page, answered as if the writer signed in at once.
pub fn sign_in_page(req: &Request, state: &mut SignIn) -> Response {
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

pub fn authorized(req: &Request, sign_in: &SignIn) -> bool {
    // Only a renewed token works: the first one ran out right away.
    req.header("authorization")
        .and_then(|h| h.strip_prefix("Bearer "))
        .is_some_and(|t| t != "AT1" && sign_in.issued.iter().any(|i| i == t))
}
