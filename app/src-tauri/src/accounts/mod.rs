//! Account sessions live in the OS credential store; settings live behind Supabase RLS.
mod client;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;
use writer_core::store::{atomic_write, read_text, rev_of};
use writer_sync::oauth::{self, Client, Listener};

use crate::drives::{blocking, secrets};
use crate::error::{Res, fail};
use crate::paths::{account_config_file, account_session_file};
use client::{Config, Identity, Login, Session, SettingsWrite, Snapshot};

#[derive(Default)]
pub struct AccountState {
    lock: Arc<Mutex<Option<Session>>>,
    cancel: Arc<AtomicBool>,
    connecting: Arc<AtomicBool>,
}
#[derive(Serialize)]
pub struct Status {
    url: Option<String>,
    account: Option<Identity>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResult {
    account: Option<Identity>,
    confirmation_required: bool,
}
#[derive(Serialize, Deserialize)]
struct Remembered {
    url: String,
    identity: Identity,
}
fn load<T: serde::de::DeserializeOwned>(file: &Path) -> Option<T> {
    read_text(file)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
}
fn save<T: Serialize>(file: &Path, value: &T) -> Res<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|_| "설정을 저장하지 못함")?;
    atomic_write(file, &bytes).map_err(fail)
}
fn config(app: &AppHandle) -> Res<Option<Config>> {
    if let Some(value) = load::<Config>(&account_config_file(app)?) {
        return Config::checked(value.url, value.publishable_key).map(Some);
    }
    match (
        option_env!("WRITER_SUPABASE_URL"),
        option_env!("WRITER_SUPABASE_PUBLISHABLE_KEY"),
    ) {
        (Some(url), Some(key)) if !url.is_empty() && !key.is_empty() => {
            Config::checked(url.into(), key.into()).map(Some)
        }
        _ => Ok(None),
    }
}
fn secret_key(config: &Config, identity: &Identity) -> String {
    format!(
        "account/{}",
        rev_of(client::account_key(config, identity).as_bytes())
    )
}
fn remembered(app: &AppHandle, config: &Config) -> Option<Identity> {
    load::<Remembered>(&account_session_file(app).ok()?)
        .filter(|value| value.url == config.url)
        .map(|value| value.identity)
}
fn status(app: &AppHandle) -> Res<Status> {
    let Some(config) = config(app)? else {
        return Ok(Status {
            url: None,
            account: None,
        });
    };
    let account = remembered(app, &config).filter(|identity| {
        secrets()
            .get(&secret_key(&config, identity))
            .ok()
            .flatten()
            .is_some()
    });
    Ok(Status {
        url: Some(config.url),
        account,
    })
}
fn connection(app: &AppHandle, expected: &str) -> Res<(Config, Identity)> {
    let config = config(app)?.ok_or("계정 서버 연결을 먼저 설정하세요.")?;
    let identity = remembered(app, &config).ok_or("먼저 로그인하세요.")?;
    if client::account_key(&config, &identity) != expected {
        return Err("계정이 바뀌었습니다. 이전 계정의 설정 요청을 취소했습니다.".into());
    }
    Ok((config, identity))
}
fn access(
    config: &Config,
    identity: &Identity,
    cache: &mut Option<Session>,
    force: bool,
) -> Res<String> {
    if !force
        && let Some(session) = cache
        && let Some(token) = session.access_for(config, identity)
    {
        return Ok(token.to_owned());
    }
    let key = secret_key(config, identity);
    let refresh = secrets()
        .get(&key)
        .map_err(fail)?
        .ok_or("로그인이 만료되었습니다. 다시 로그인하세요.")?;
    let tokens = client::token_reply(
        config,
        "token?grant_type=refresh_token",
        json!({"refresh_token":refresh}),
    )?
    .ok_or("로그인을 갱신하지 못했습니다.")?;
    let actual = client::identity(config, &tokens.access)?;
    if actual.id != identity.id {
        return Err("로그인 계정이 바뀌어 설정을 적용하지 않았습니다. 다시 로그인하세요.".into());
    }
    secrets().set(&key, &tokens.refresh).map_err(fail)?;
    let token = tokens.access.clone();
    *cache = Some(Session::new(config, identity, tokens));
    Ok(token)
}
fn rpc(
    app: &AppHandle,
    expected: &str,
    cache: &mut Option<Session>,
    name: &str,
    body: Value,
) -> Res<Value> {
    let (config, identity) = connection(app, expected)?;
    let token = access(&config, &identity, cache, false)?;
    let path = format!("/rest/v1/rpc/{name}");
    let (mut status, mut value) = client::call(&config, &path, Some(&token), Some(body.clone()))?;
    if status == 401 {
        let token = access(&config, &identity, cache, true)?;
        (status, value) = client::call(&config, &path, Some(&token), Some(body))?;
    }
    client::require_ok(status)?;
    Ok(value)
}
struct LoginGuard(Arc<AtomicBool>);
impl Drop for LoginGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Relaxed);
    }
}

#[tauri::command]
pub async fn account_status(app: AppHandle) -> Res<Status> {
    blocking(move || status(&app)).await
}
#[tauri::command]
pub async fn account_configure(
    app: AppHandle,
    state: State<'_, AccountState>,
    url: String,
    publishable_key: String,
) -> Res<Status> {
    if state.connecting.load(Ordering::Relaxed) {
        return Err("로그인 중에는 서버 연결을 바꿀 수 없습니다.".into());
    }
    let lock = state.lock.clone();
    blocking(move || {
        let mut cache = lock.lock().map_err(|_| "계정 설정을 열지 못함")?;
        if status(&app)?.account.is_some() {
            return Err("로그아웃 후 서버 연결을 바꿔 주세요.".into());
        }
        let value = Config::checked(url, publishable_key)?;
        let (code, _) = client::call(&value, "/auth/v1/settings", None, None)?;
        client::require_ok(code)?;
        save(&account_config_file(&app)?, &value)?;
        *cache = None;
        status(&app)
    })
    .await
}
#[tauri::command]
pub async fn account_login(
    app: AppHandle,
    state: State<'_, AccountState>,
    request: Login,
) -> Res<LoginResult> {
    if state.connecting.swap(true, Ordering::Relaxed) {
        return Err("이미 로그인하고 있습니다.".into());
    }
    state.cancel.store(false, Ordering::Relaxed);
    let guard = LoginGuard(state.connecting.clone());
    let cancel = state.cancel.clone();
    let lock = state.lock.clone();
    blocking(move||{
        let _guard=guard;let mut cache=lock.lock().map_err(|_|"계정 설정을 열지 못함")?;
        let config=config(&app)?.ok_or("계정 서버 연결을 먼저 설정하세요.")?;
        if status(&app)?.account.is_some(){return Err("다른 계정으로 로그인하려면 먼저 로그아웃하세요.".into());}
        let tokens=match request {
            Login::Password{email,password}=>client::token_reply(&config,"token?grant_type=password",json!({"email":email.trim(),"password":password}))?,
            Login::Signup{email,password}=>client::token_reply(&config,"signup",json!({"email":email.trim(),"password":password}))?,
            request=>{
                let client=Client{auth_url:String::new(),token_url:String::new(),client_id:String::new(),client_secret:None,scopes:vec![],extra:vec![],redirect_host:"127.0.0.1".into(),redirect_port:Some(47821)};
                let listener=Listener::bind(&client).map_err(fail)?;
                let pkce=oauth::pkce().map_err(fail)?;let state=oauth::new_state().map_err(fail)?;
                let redirect=format!("{}?state={state}",listener.redirect);
                let url=match request {
                    Login::Oauth{provider}=>{
                        if !matches!(provider.as_str(),"google"|"kakao"){return Err("지원하지 않는 로그인 제공자입니다.".into());}
                        let mut url=Url::parse(&format!("{}/auth/v1/authorize",config.url)).map_err(|_|"로그인 URL을 만들지 못함")?;
                        url.query_pairs_mut().append_pair("provider",&provider).append_pair("redirect_to",&redirect).append_pair("code_challenge",&pkce.challenge).append_pair("code_challenge_method","s256");
                        url.to_string()
                    }
                    Login::Sso{domain}=>{
                        let (code,value)=client::call(&config,"/auth/v1/sso",None,Some(json!({"domain":domain.trim(),"redirect_to":redirect,"skip_http_redirect":true,"code_challenge":pkce.challenge,"code_challenge_method":"s256"})))?;
                        if matches!(code, 400 | 403 | 404 | 422) {
                            return Err("이 도메인의 SSO가 서버에 등록되었는지 확인하세요. SAML SSO는 Supabase Pro 이상에서 사용할 수 있습니다.".into());
                        }
                        client::require_ok(code)?;
                        let url=value["url"].as_str().ok_or("SSO 로그인 주소를 받지 못했습니다.")?;
                        if Url::parse(url).ok().is_none_or(|u|u.scheme()!="https"){return Err("SSO 로그인 주소가 안전한 HTTPS 주소가 아닙니다.".into());}
                        url.into()
                    }
                    _=>unreachable!(),
                };
                app.opener().open_url(url,None::<&str>).map_err(|_|"로그인 창을 열지 못했습니다.")?;
                let code=listener.wait(&state,Duration::from_secs(300),&cancel).map_err(fail)?;
                client::token_reply(&config,"token?grant_type=pkce",json!({"auth_code":code,"code_verifier":pkce.verifier}))?
            }
        };
        let Some(tokens)=tokens else{return Ok(LoginResult{account:None,confirmation_required:true});};
        if cancel.load(Ordering::Relaxed){return Err("로그인을 취소했습니다.".into());}
        let identity=client::identity(&config,&tokens.access)?;
        if cancel.load(Ordering::Relaxed){return Err("로그인을 취소했습니다.".into());}
        let key=secret_key(&config,&identity);
        secrets().set(&key,&tokens.refresh).map_err(fail)?;
        if let Err(error)=save(&account_session_file(&app)?,&Remembered{url:config.url.clone(),identity:identity.clone()}) {
            let _=secrets().delete(&key);return Err(error);
        }
        *cache=Some(Session::new(&config,&identity,tokens));Ok(LoginResult{account:Some(identity),confirmation_required:false})
    }).await
}
#[tauri::command]
pub async fn account_cancel(state: State<'_, AccountState>) -> Res<()> {
    state.cancel.store(true, Ordering::Relaxed);
    Ok(())
}
#[tauri::command]
pub async fn account_logout(app: AppHandle, state: State<'_, AccountState>) -> Res<()> {
    state.cancel.store(true, Ordering::Relaxed);
    let lock = state.lock.clone();
    blocking(move || {
        let mut cache = lock.lock().map_err(|_| "계정 설정을 열지 못함")?;
        if let Some(config) = config(&app)?
            && let Some(identity) = remembered(&app, &config)
        {
            secrets()
                .delete(&secret_key(&config, &identity))
                .map_err(fail)?;
            save(&account_session_file(&app)?, &Value::Null)?;
            if let Some(session) = cache.as_ref()
                && let Some(token) = session.access_for(&config, &identity)
            {
                let _ = client::call(
                    &config,
                    "/auth/v1/logout?scope=local",
                    Some(token),
                    Some(json!({})),
                );
            }
        }
        *cache = None;
        Ok(())
    })
    .await
}
#[tauri::command]
pub async fn account_settings_read(
    app: AppHandle,
    state: State<'_, AccountState>,
    account_key: String,
) -> Res<Snapshot> {
    let lock = state.lock.clone();
    blocking(move || {
        let mut cache = lock.lock().map_err(|_| "계정 설정을 열지 못함")?;
        serde_json::from_value(rpc(
            &app,
            &account_key,
            &mut cache,
            "writer_settings_read",
            json!({}),
        )?)
        .map_err(|_| "개인 설정 응답 형식이 올바르지 않습니다.".into())
    })
    .await
}
#[tauri::command]
pub async fn account_settings_write(
    app: AppHandle,
    state: State<'_, AccountState>,
    account_key: String,
    revision: u64,
    patch: Value,
) -> Res<SettingsWrite> {
    if !patch.is_object()
        || serde_json::to_vec(&patch).map_or(true, |bytes| bytes.len() > 32 * 1024)
    {
        return Err("개인 설정 데이터가 올바르지 않습니다.".into());
    }
    let lock = state.lock.clone();
    blocking(move || {
        let mut cache = lock.lock().map_err(|_| "계정 설정을 열지 못함")?;
        serde_json::from_value(rpc(
            &app,
            &account_key,
            &mut cache,
            "writer_settings_write",
            json!({"expected_revision":revision,"patch":patch}),
        )?)
        .map_err(|_| "개인 설정 저장 응답 형식이 올바르지 않습니다.".into())
    })
    .await
}

use url::Url;
