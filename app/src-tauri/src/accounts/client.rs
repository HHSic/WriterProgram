//! Supabase Auth and the private settings RPCs. Tokens never reach the webview.
use std::time::Duration;

use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

use crate::error::Res;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub url: String,
    pub publishable_key: String,
}

impl Config {
    pub fn checked(url: String, publishable_key: String) -> Res<Self> {
        let parsed = Url::parse(url.trim()).map_err(|_| "프로젝트 URL을 확인해 주세요.")?;
        let local = matches!(parsed.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
        if parsed.host_str().is_none()
            || !(parsed.scheme() == "https" || (parsed.scheme() == "http" && local))
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
        {
            return Err("프로젝트 URL은 HTTPS 주소여야 합니다. 로컬 테스트 주소는 HTTP를 사용할 수 있습니다.".into());
        }
        let key = publishable_key.trim().to_owned();
        let anon = key
            .split('.')
            .nth(1)
            .and_then(|part| {
                base64::engine::general_purpose::URL_SAFE_NO_PAD
                    .decode(part)
                    .ok()
            })
            .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
            .is_some_and(|claims| claims["role"] == "anon");
        if !key.starts_with("sb_publishable_") && !anon {
            return Err("Publishable key 또는 anon 공개 키를 사용하세요. secret·service_role 키는 앱에 저장하지 않습니다.".into());
        }
        Ok(Self {
            url: parsed.as_str().trim_end_matches('/').to_owned(),
            publishable_key: key,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Identity {
    pub id: String,
    pub email: String,
    pub name: String,
}
pub struct Tokens {
    pub access: String,
    pub refresh: String,
    pub expires: i64,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Login {
    Oauth { provider: String },
    Sso { domain: String },
    Password { email: String, password: String },
    Signup { email: String, password: String },
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub revision: u64,
    pub settings: Value,
    pub updated_at: Option<String>,
}
#[derive(Serialize, Deserialize)]
pub struct SettingsWrite {
    pub conflict: bool,
    pub snapshot: Snapshot,
}

/// Don't include response bodies in errors: auth replies may contain credentials.
pub fn call(
    config: &Config,
    path: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> Res<(u16, Value)> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(30)))
        .http_status_as_error(false)
        .max_redirects(0)
        .build()
        .into();
    let url = format!("{}{path}", config.url);
    let mut response = if let Some(body) = body {
        let mut request = agent.post(&url).header("apikey", &config.publishable_key);
        if let Some(token) = token {
            request = request.header("Authorization", format!("Bearer {token}"));
        }
        request
            .content_type("application/json")
            .send(body.to_string().as_bytes())
    } else {
        let mut request = agent.get(&url).header("apikey", &config.publishable_key);
        if let Some(token) = token {
            request = request.header("Authorization", format!("Bearer {token}"));
        }
        request.call()
    }
    .map_err(|_| "계정 서버에 연결하지 못했습니다. 인터넷 연결과 서버 주소를 확인하세요.")?;
    let status = response.status().as_u16();
    let bytes = response
        .body_mut()
        .with_config()
        .limit(128 * 1024)
        .read_to_vec()
        .map_err(|_| "계정 서버의 응답을 읽지 못했습니다.")?;
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).map_err(|_| "계정 서버의 응답 형식이 올바르지 않습니다.")?
    };
    Ok((status, value))
}
pub fn require_ok(status: u16) -> Res<()> {
    match status {
        200..=299 => Ok(()),
        400 | 422 => Err("로그인 정보 또는 서버의 로그인 제공자 설정을 확인하세요.".into()),
        401 | 403 => {
            Err("로그인 정보가 맞지 않거나 세션이 만료되었습니다. 다시 로그인하세요.".into())
        }
        404 => Err("계정 서버 설정이나 개인 설정 DB 설치를 확인하세요.".into()),
        429 => Err("요청이 많아 잠시 기다려야 합니다. 잠시 후 다시 시도하세요.".into()),
        _ => Err("계정 서버에서 요청을 처리하지 못했습니다. 잠시 후 다시 시도하세요.".into()),
    }
}
pub fn token_reply(config: &Config, grant: &str, body: Value) -> Res<Option<Tokens>> {
    let (status, value) = call(config, &format!("/auth/v1/{grant}"), None, Some(body))?;
    require_ok(status)?;
    let Some(access) = value["access_token"].as_str().filter(|s| !s.is_empty()) else {
        return if grant == "signup" {
            Ok(None)
        } else {
            Err("로그인 토큰을 받지 못했습니다.".into())
        };
    };
    let refresh = value["refresh_token"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or("로그인 유지 정보를 받지 못했습니다.")?;
    let life = value["expires_in"].as_i64().unwrap_or(3600);
    if life <= 0 {
        return Err("이미 만료된 로그인 응답입니다.".into());
    }
    Ok(Some(Tokens {
        access: access.into(),
        refresh: refresh.into(),
        expires: chrono::Utc::now().timestamp().saturating_add(life),
    }))
}
pub fn identity(config: &Config, access: &str) -> Res<Identity> {
    let (status, value) = call(config, "/auth/v1/user", Some(access), None)?;
    require_ok(status)?;
    let id = value["id"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or("로그인한 계정을 확인하지 못했습니다.")?;
    let email = value["email"].as_str().unwrap_or("");
    let name = ["full_name", "name", "nickname"]
        .iter()
        .find_map(|key| {
            value["user_metadata"][key]
                .as_str()
                .filter(|s| !s.is_empty())
        })
        .unwrap_or(email);
    Ok(Identity {
        id: id.into(),
        email: email.into(),
        name: name.chars().take(128).collect(),
    })
}
pub fn account_key(config: &Config, identity: &Identity) -> String {
    serde_json::to_string(&(&config.url, &identity.id)).expect("strings serialize")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread::{self, JoinHandle};

    fn server(status: u16, reply: Value) -> (Config, JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let config = Config::checked(
            format!("http://{}", listener.local_addr().unwrap()),
            "sb_publishable_test".into(),
        )
        .unwrap();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut request = Vec::new();
            let mut block = [0; 4096];
            loop {
                let n = stream.read(&mut block).unwrap();
                if n == 0 {
                    break;
                }
                request.extend_from_slice(&block[..n]);
                if let Some(end) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&request[..end]).to_ascii_lowercase();
                    let length: usize = headers
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length:"))
                        .map_or(0, |v| v.trim().parse().unwrap());
                    if request.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            let body = reply.to_string();
            write!(stream, "HTTP/1.1 {status} Response\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            String::from_utf8(request).unwrap()
        });
        (config, handle)
    }
    #[test]
    fn configuration_requires_public_credentials_and_secure_origins() {
        for url in [
            "https://project.supabase.co",
            "http://127.0.0.1:54321",
            "http://localhost:54321",
        ] {
            assert!(Config::checked(url.into(), "sb_publishable_example".into()).is_ok());
        }
        for url in [
            "http://example.com",
            "https://user:password@example.com",
            "https://example.com?token=x",
            "file:///tmp/a",
        ] {
            assert!(Config::checked(url.into(), "sb_publishable_example".into()).is_err());
        }
        assert!(Config::checked("https://example.com".into(), "sb_secret_example".into()).is_err());
        let role = |role: &str| {
            format!(
                "x.{}.x",
                base64::engine::general_purpose::URL_SAFE_NO_PAD
                    .encode(json!({"role":role}).to_string())
            )
        };
        assert!(Config::checked("https://example.com".into(), role("anon")).is_ok());
        assert!(Config::checked("https://example.com".into(), role("service_role")).is_err());
    }
    #[test]
    fn settings_requests_are_bound_to_both_project_and_subject() {
        let identity = Identity {
            id: "u1".into(),
            email: "same@example.com".into(),
            name: "".into(),
        };
        let first = Config::checked(
            "https://first.supabase.co".into(),
            "sb_publishable_example".into(),
        )
        .unwrap();
        let second = Config::checked(
            "https://second.supabase.co".into(),
            "sb_publishable_example".into(),
        )
        .unwrap();
        assert_ne!(
            account_key(&first, &identity),
            account_key(&second, &identity)
        );
    }
    #[test]
    fn password_grant_stays_native_and_signup_can_require_confirmation() {
        let (config, request) = server(
            200,
            json!({"access_token":"test-access","refresh_token":"test-refresh","expires_in":3600}),
        );
        let tokens = token_reply(
            &config,
            "token?grant_type=password",
            json!({"email":"writer@example.com","password":"test-password"}),
        )
        .unwrap()
        .unwrap();
        assert_eq!(tokens.refresh, "test-refresh");
        assert!(tokens.expires > chrono::Utc::now().timestamp());
        let request = request.join().unwrap();
        assert!(request.starts_with("POST /auth/v1/token?grant_type=password "));
        assert!(
            request
                .to_ascii_lowercase()
                .contains("apikey: sb_publishable_test")
        );
        let body: Value = serde_json::from_str(request.split("\r\n\r\n").nth(1).unwrap()).unwrap();
        assert_eq!(body["email"], "writer@example.com");
        assert_eq!(body["password"], "test-password");
        let (config, request) = server(200, json!({"id":"pending-user"}));
        assert!(token_reply(&config, "signup", json!({})).unwrap().is_none());
        request.join().unwrap();
    }
    #[test]
    fn identity_is_checked_with_the_server_and_errors_hide_auth_replies() {
        let (config, request) = server(
            200,
            json!({"id":"u1","email":"writer@example.com","user_metadata":{"nickname":"작가"}}),
        );
        assert_eq!(identity(&config, "test-access").unwrap().name, "작가");
        assert!(
            request
                .join()
                .unwrap()
                .to_ascii_lowercase()
                .contains("authorization: bearer test-access")
        );
        let (config, request) = server(200, json!({"email":"writer@example.com"}));
        assert!(identity(&config, "test-access").is_err());
        request.join().unwrap();
        let (config, request) = server(401, json!({"error":"test-password test-refresh"}));
        let error = token_reply(&config, "token?grant_type=password", json!({}))
            .err()
            .unwrap();
        assert!(!error.contains("test-password") && !error.contains("test-refresh"));
        request.join().unwrap();
    }
}
