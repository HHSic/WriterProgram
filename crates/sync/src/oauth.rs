//! Signing in to a drive (OAuth 2.0 for apps on the writer's own device).
//!
//! 1. The app opens the drive's own sign-in page in the browser, with a
//!    one-time check (PKCE) and a random `state`.
//! 2. The writer signs in there (usually already signed in: just 허용) and the
//!    page sends the browser back to `http://127.0.0.1:<port>/`, where
//!    [`Listener`] is waiting for the one answer.
//! 3. The app trades the answer's code for tokens ([`exchange`]). The refresh
//!    token goes to the system's credential store; the short-lived access
//!    token stays in memory and is renewed with [`refresh`].
//!
//! The app never sees the writer's password, and the sign-in page is the
//! drive's, not ours.

use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::Deserialize;
use writer_core::store::{rev_of, sha256};

use crate::http::{Body, Http, enc};
use crate::{Error, Result};

/// How to sign in to one drive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Client {
    pub auth_url: String,
    pub token_url: String,
    pub client_id: String,
    /// Google's desktop apps have one; it is not a secret in an installed app.
    pub client_secret: Option<String>,
    pub scopes: Vec<String>,
    /// Extra query parameters for the sign-in page.
    pub extra: Vec<(String, String)>,
    /// Host in the address the browser comes back to (`127.0.0.1` or `localhost`).
    pub redirect_host: String,
    /// A fixed port, for drives that want the exact address registered.
    pub redirect_port: Option<u16>,
}

/// The one-time check that ties the answer to this sign-in (RFC 7636).
pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

fn random(bytes: usize) -> Result<Vec<u8>> {
    let mut buf = vec![0u8; bytes];
    getrandom::fill(&mut buf).map_err(|e| Error::Invalid(format!("난수를 얻지 못함 ({e})")))?;
    Ok(buf)
}

pub fn pkce() -> Result<Pkce> {
    let verifier = URL_SAFE_NO_PAD.encode(random(48)?);
    Ok(Pkce {
        challenge: challenge_of(&verifier),
        verifier,
    })
}

fn challenge_of(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(sha256(verifier.as_bytes()))
}

pub fn new_state() -> Result<String> {
    Ok(URL_SAFE_NO_PAD.encode(random(16)?))
}

/// The drive's sign-in page for this attempt.
pub fn sign_in_url(client: &Client, redirect: &str, state: &str, pkce: &Pkce) -> String {
    let mut params: Vec<(&str, String)> = vec![
        ("client_id", client.client_id.clone()),
        ("response_type", "code".into()),
        ("redirect_uri", redirect.into()),
        ("state", state.into()),
        ("code_challenge", pkce.challenge.clone()),
        ("code_challenge_method", "S256".into()),
    ];
    if !client.scopes.is_empty() {
        params.push(("scope", client.scopes.join(" ")));
    }
    for (k, v) in &client.extra {
        params.push((k.as_str(), v.clone()));
    }
    let query: Vec<String> = params
        .iter()
        .map(|(k, v)| format!("{k}={}", enc(v)))
        .collect();
    format!("{}?{}", client.auth_url, query.join("&"))
}

/// Waits on this computer for the browser to come back from the sign-in page.
pub struct Listener {
    listener: TcpListener,
    pub redirect: String,
}

const DONE_PAGE: &str = "<!doctype html><meta charset=utf-8><title>WriterProgram</title>\
<body style=\"font-family:sans-serif;padding:48px;line-height:1.7\">\
<h2>연결했습니다</h2><p>이 창을 닫고 WriterProgram으로 돌아가세요.</p></body>";
const FAILED_PAGE: &str = "<!doctype html><meta charset=utf-8><title>WriterProgram</title>\
<body style=\"font-family:sans-serif;padding:48px;line-height:1.7\">\
<h2>연결하지 못했습니다</h2><p>이 창을 닫고 WriterProgram에서 다시 시도해 주세요.</p></body>";

impl Listener {
    pub fn bind(client: &Client) -> Result<Listener> {
        let port = client.redirect_port.unwrap_or(0);
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port)).map_err(|e| {
            Error::Invalid(format!("로그인 창의 대답을 받을 준비를 하지 못함 ({e})"))
        })?;
        let port = listener
            .local_addr()
            .map_err(|e| Error::Invalid(e.to_string()))?
            .port();
        listener
            .set_nonblocking(true)
            .map_err(|e| Error::Invalid(e.to_string()))?;
        Ok(Listener {
            listener,
            redirect: format!("http://{}:{port}/", client.redirect_host),
        })
    }

    /// Waits for the answer and returns its code. Stops when `cancel` is set
    /// or after `timeout`.
    pub fn wait(&self, state: &str, timeout: Duration, cancel: &AtomicBool) -> Result<String> {
        let end = Instant::now() + timeout;
        while Instant::now() < end {
            if cancel.load(Ordering::Relaxed) {
                return Err(Error::Invalid("연결을 그만둠".into()));
            }
            match self.listener.accept() {
                Ok((stream, _)) => {
                    if let Some(result) = answer(stream, state) {
                        return result;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(e) => return Err(Error::Invalid(e.to_string())),
            }
        }
        Err(Error::Invalid(
            "로그인을 기다리다 시간이 지남. 다시 시도해 주세요.".into(),
        ))
    }
}

/// Reads one request. None for requests that are not the answer (favicon).
fn answer(mut stream: TcpStream, state: &str) -> Option<Result<String>> {
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut line = String::new();
    BufReader::new(&stream).read_line(&mut line).ok()?;
    let target = line.split_whitespace().nth(1)?.to_string();
    let query = target.split_once('?').map(|(_, q)| q).unwrap_or("");
    let params: Vec<(String, String)> = url::form_urlencoded::parse(query.as_bytes())
        .into_owned()
        .collect();
    let get = |k: &str| {
        params
            .iter()
            .find(|(key, _)| key == k)
            .map(|(_, v)| v.clone())
    };
    let (code, error) = (get("code"), get("error"));
    if code.is_none() && error.is_none() {
        let _ = stream
            .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        return None;
    }
    let result = match (code, error) {
        _ if get("state").as_deref() != Some(state) => Err(Error::Invalid(
            "다른 로그인에서 온 대답이라 받지 않음".into(),
        )),
        (Some(code), None) => Ok(code),
        (_, Some(error)) if error == "access_denied" => {
            Err(Error::Invalid("연결을 허용하지 않음".into()))
        }
        (_, error) => Err(Error::Drive(format!(
            "로그인하지 못함 ({})",
            error.unwrap_or_default()
        ))),
    };
    let page = if result.is_ok() {
        DONE_PAGE
    } else {
        FAILED_PAGE
    };
    let _ = write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{page}",
        page.len()
    );
    Some(result)
}

/// Tokens from the drive. `expires_at` is in seconds since 1970.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tokens {
    pub access: String,
    pub refresh: Option<String>,
    pub expires_at: i64,
}

#[derive(Deserialize)]
struct TokenReply {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<i64>,
}

#[derive(Deserialize)]
struct TokenError {
    #[serde(default)]
    error: String,
}

fn token_request(http: &Http, client: &Client, mut form: Vec<(&str, String)>) -> Result<Tokens> {
    form.push(("client_id", client.client_id.clone()));
    if let Some(secret) = &client.client_secret {
        form.push(("client_secret", secret.clone()));
    }
    let reply = http.send("POST", &client.token_url, &[], Body::Form(form))?;
    if !reply.ok() {
        let error = reply
            .json::<TokenError>()
            .map(|e| e.error)
            .unwrap_or_default();
        return Err(if error == "invalid_grant" {
            Error::SignedOut
        } else {
            Error::Drive(format!(
                "로그인하지 못함 ({} {})",
                reply.status,
                reply.text()
            ))
        });
    }
    let t: TokenReply = reply.json()?;
    Ok(Tokens {
        access: t.access_token,
        refresh: t.refresh_token,
        expires_at: chrono::Utc::now().timestamp() + t.expires_in.unwrap_or(3600),
    })
}

/// Trades the sign-in answer for tokens.
pub fn exchange(
    http: &Http,
    client: &Client,
    code: &str,
    pkce: &Pkce,
    redirect: &str,
) -> Result<Tokens> {
    token_request(
        http,
        client,
        vec![
            ("grant_type", "authorization_code".into()),
            ("code", code.into()),
            ("redirect_uri", redirect.into()),
            ("code_verifier", pkce.verifier.clone()),
        ],
    )
}

/// A new access token from the refresh token. Some drives hand out a new
/// refresh token too; keep it when they do.
pub fn refresh(http: &Http, client: &Client, refresh_token: &str) -> Result<Tokens> {
    let mut tokens = token_request(
        http,
        client,
        vec![
            ("grant_type", "refresh_token".into()),
            ("refresh_token", refresh_token.into()),
        ],
    )?;
    tokens
        .refresh
        .get_or_insert_with(|| refresh_token.to_string());
    Ok(tokens)
}

/// A short, stable name for a sign-in in the credential store.
pub fn secret_key(provider: &str, account: &str) -> String {
    format!(
        "WriterProgram/{provider}/{}",
        &rev_of(account.as_bytes())[..12]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_follows_rfc_7636() {
        // Appendix B of RFC 7636.
        assert_eq!(
            challenge_of("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
        let p = pkce().unwrap();
        assert!(p.verifier.len() >= 43 && p.verifier.len() <= 128);
        assert_eq!(p.challenge, challenge_of(&p.verifier));
    }

    #[test]
    fn sign_in_page_address() {
        let client = Client {
            auth_url: "https://example.test/auth".into(),
            token_url: "https://example.test/token".into(),
            client_id: "app 1".into(),
            client_secret: None,
            scopes: vec!["a".into(), "b".into()],
            extra: vec![("access_type".into(), "offline".into())],
            redirect_host: "127.0.0.1".into(),
            redirect_port: None,
        };
        let pkce = Pkce {
            verifier: "v".into(),
            challenge: "c".into(),
        };
        let url = sign_in_url(&client, "http://127.0.0.1:5000/", "s1", &pkce);
        assert_eq!(
            url,
            "https://example.test/auth?client_id=app%201&response_type=code&redirect_uri=http%3A%2F%2F127.0.0.1%3A5000%2F&state=s1&code_challenge=c&code_challenge_method=S256&scope=a%20b&access_type=offline"
        );
    }
}
