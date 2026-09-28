//! 앱 안 브라우저 (시제품): web pages next to the manuscript, in child
//! webviews laid over a tab of the middle column. The screen keeps each one's
//! place and size in step with its tab (`browser_bounds`) and hides it while
//! menus or dialogs are open, since a webview always draws on top.
//!
//! Pages get no access to the app: the app's permissions only cover its own
//! pages, never outside addresses. What the app needs from a page (address,
//! title, selected text) it reads by running a fixed line of script there;
//! nothing from the page is ever run as script.

use std::sync::mpsc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::webview::{NewWindowResponse, PageLoadEvent};
use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Url, Webview, WebviewBuilder,
    WebviewUrl,
};

type Res<T> = Result<T, String>;

const HOME: &str = "https://www.google.com/";

/// What the screen hears about a page: its address, title, or whether it is loading.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PageEvent {
    label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    loading: Option<bool>,
}

fn check_label(label: &str) -> Res<()> {
    let ok = label.starts_with("web-")
        && label.len() <= 64
        && label
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if ok {
        Ok(())
    } else {
        Err("올바르지 않은 브라우저 탭".into())
    }
}

/// An address typed in the bar: a web address, or else words to search for.
pub fn address(input: &str) -> Url {
    let input = input.trim();
    if input.is_empty() {
        return Url::parse(HOME).expect("home parses");
    }
    if let Ok(url) = Url::parse(input)
        && matches!(url.scheme(), "http" | "https")
    {
        return url;
    }
    let looks_like_host = !input.contains(' ')
        && input.contains('.')
        && !input.starts_with('.')
        && !input.ends_with('.');
    if looks_like_host && let Ok(url) = Url::parse(&format!("https://{input}")) {
        return url;
    }
    let mut url = Url::parse("https://www.google.com/search").expect("search parses");
    url.query_pairs_mut().append_pair("q", input);
    url
}

fn emit(app: &AppHandle, event: PageEvent) {
    let _ = app.emit_to("main", "browser-page", event);
}

fn webview(app: &AppHandle, label: &str) -> Res<Webview> {
    check_label(label)?;
    app.get_webview(label)
        .ok_or_else(|| "브라우저 탭이 없음".into())
}

/// Runs one of our own fixed scripts in the page.
fn run(webview: &Webview, script: &'static str) -> Res<()> {
    webview
        .eval_with_callback(script, |_| {})
        .map_err(|e| e.to_string())
}

/// Opens (or moves and shows) the browser of a tab at `x, y` with size `w, h`
/// in the window, in CSS pixels.
#[tauri::command]
pub async fn browser_open(
    app: AppHandle,
    label: String,
    url: String,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> Res<()> {
    check_label(&label)?;
    if let Some(webview) = app.get_webview(&label) {
        webview
            .set_position(LogicalPosition::new(x, y))
            .map_err(|e| e.to_string())?;
        webview
            .set_size(LogicalSize::new(w, h))
            .map_err(|e| e.to_string())?;
        return webview.show().map_err(|e| e.to_string());
    }
    let window = app.get_window("main").ok_or("창을 찾을 수 없음")?;
    let (on_load, on_title, on_popup) = (app.clone(), app.clone(), app.clone());
    let (l1, l2, l3) = (label.clone(), label.clone(), label.clone());
    let builder = WebviewBuilder::new(&label, WebviewUrl::External(address(&url)))
        .on_page_load(move |_, payload| {
            emit(
                &on_load,
                PageEvent {
                    label: l1.clone(),
                    url: Some(payload.url().to_string()),
                    title: None,
                    loading: Some(matches!(payload.event(), PageLoadEvent::Started)),
                },
            );
        })
        .on_document_title_changed(move |_, title| {
            emit(
                &on_title,
                PageEvent {
                    label: l2.clone(),
                    url: None,
                    title: Some(title),
                    loading: None,
                },
            );
        })
        // Links that want a new window open in the same tab.
        .on_new_window(move |url, _| {
            if let Some(webview) = on_popup.get_webview(&l3) {
                let _ = webview.navigate(url);
            }
            NewWindowResponse::Deny
        });
    window
        .add_child(builder, LogicalPosition::new(x, y), LogicalSize::new(w, h))
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Moves a tab's browser, or hides it (`visible: false`) while its tab is in
/// the back or something is shown over it.
#[tauri::command]
pub async fn browser_bounds(
    app: AppHandle,
    label: String,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    visible: bool,
) -> Res<()> {
    check_label(&label)?;
    let Some(webview) = app.get_webview(&label) else {
        return Ok(());
    };
    if !visible {
        return webview.hide().map_err(|e| e.to_string());
    }
    webview
        .set_position(LogicalPosition::new(x, y))
        .map_err(|e| e.to_string())?;
    webview
        .set_size(LogicalSize::new(w, h))
        .map_err(|e| e.to_string())?;
    webview.show().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn browser_navigate(app: AppHandle, label: String, url: String) -> Res<String> {
    let webview = webview(&app, &label)?;
    let url = address(&url);
    webview.navigate(url.clone()).map_err(|e| e.to_string())?;
    Ok(url.to_string())
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Step {
    Back,
    Forward,
    Reload,
}

#[tauri::command]
pub async fn browser_step(app: AppHandle, label: String, step: Step) -> Res<()> {
    let webview = webview(&app, &label)?;
    run(
        &webview,
        match step {
            Step::Back => "history.back()",
            Step::Forward => "history.forward()",
            Step::Reload => "location.reload()",
        },
    )
}

#[tauri::command]
pub async fn browser_close(app: AppHandle, label: String) -> Res<()> {
    check_label(&label)?;
    if let Some(webview) = app.get_webview(&label) {
        webview.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// The page as it is now, for 자료로 보관.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Clip {
    url: String,
    title: String,
    /// The selected text, if any.
    text: String,
}

/// Address, title and selected text of the page.
const CLIP_SCRIPT: &str = "JSON.stringify({ url: String(location.href), title: String(document.title), text: String(window.getSelection() || '').slice(0, 20000) })";

#[tauri::command]
pub async fn browser_clip(app: AppHandle, label: String) -> Res<Clip> {
    let webview = webview(&app, &label)?;
    let (tx, rx) = mpsc::channel();
    webview
        .eval_with_callback(CLIP_SCRIPT, move |result| {
            let _ = tx.send(result);
        })
        .map_err(|e| e.to_string())?;
    let result =
        tauri::async_runtime::spawn_blocking(move || rx.recv_timeout(Duration::from_secs(5)))
            .await
            .map_err(|e| e.to_string())?
            .map_err(|_| "페이지가 대답하지 않음".to_string())?;
    // The script's value comes as JSON: a JSON string holding our JSON.
    let inner: String = serde_json::from_str(&result).map_err(|e| e.to_string())?;
    serde_json::from_str(&inner).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_and_searches() {
        assert_eq!(
            address("namu.wiki/w/서점").as_str(),
            "https://namu.wiki/w/%EC%84%9C%EC%A0%90"
        );
        assert_eq!(
            address("https://example.com/a").as_str(),
            "https://example.com/a"
        );
        assert_eq!(
            address("조선 시대 서점").as_str(),
            "https://www.google.com/search?q=%EC%A1%B0%EC%84%A0+%EC%8B%9C%EB%8C%80+%EC%84%9C%EC%A0%90"
        );
        assert_eq!(
            address("javascript:alert(1)").host_str(),
            Some("www.google.com")
        );
        assert_eq!(address("").as_str(), HOME);
    }
}
