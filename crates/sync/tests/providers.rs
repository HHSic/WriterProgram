//! Signing in and keeping a project in step through stand-ins for Google
//! Drive, OneDrive and Dropbox: small local servers that answer like the
//! real APIs (as documented), so the requests the app sends and the answers
//! it reads are checked end to end without an account.

mod support;

use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::thread;

use support::{Room, set_room, stand_in};
use writer_core::copies;
use writer_core::doc::{self, SaveGuard};
use writer_core::markup::Block;
use writer_core::project::{self, NewProject, ProjectKind};
use writer_sync::accounts::{self, Link};
use writer_sync::base::Base;
use writer_sync::providers::{self, AppId, Endpoints, Provider, Session};
use writer_sync::secrets::{MemorySecrets, Secrets};

// ---------------------------------------------------------------------------
// The same story on each drive

fn body(texts: &[&str]) -> Vec<Block> {
    texts.iter().map(|t| Block::text(t)).collect()
}

fn files(root: &Path) -> Vec<(String, Vec<u8>)> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) {
        for entry in fs::read_dir(dir).unwrap().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                out.push((
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                    fs::read(&path).unwrap(),
                ));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}

fn story(provider: Provider) {
    let dir = tempfile::tempdir().unwrap();
    let ends = stand_in(provider);
    let app = AppId {
        client_id: "writerprogram-test".into(),
        client_secret: (provider == Provider::Google).then(|| "not-a-secret".into()),
        // Any free port, also for Dropbox.
        redirect_port: Some(0),
    };
    let secrets: Arc<dyn Secrets> = Arc::new(MemorySecrets::default());

    // Connecting: the browser signs in, the app keeps only the refresh token.
    let cancel = AtomicBool::new(false);
    // The browser: visits the sign-in page, which sends it back to the app.
    let browser = |url: &str| {
        let url = url.to_string();
        thread::spawn(move || writer_sync::http::fetch(&url).unwrap());
        Ok(())
    };
    let connection = accounts::sign_in(
        provider,
        &app,
        ends.clone(),
        secrets.clone(),
        browser,
        &cancel,
    )
    .unwrap();
    assert_eq!(connection.account.email, "writer@example.com");
    assert_eq!(connection.account.name, "윤서하");
    assert_eq!(
        secrets.get(&connection.key).unwrap().as_deref(),
        Some("RT1")
    );

    // Later: back from the credential store.
    let session = Session::resume(
        provider,
        &app,
        ends.clone(),
        secrets.clone(),
        connection.key.clone(),
    )
    .unwrap();
    assert_eq!(session.account().unwrap().email, "writer@example.com");

    // Device A puts a project on the drive; device B brings it down.
    let a = project::create(&NewProject {
        parent: dir.path().join("a").to_string_lossy().into_owned(),
        title: "달빛 서점".into(),
        kind: ProjectKind::Webnovel,
        per_doc_goal: None,
        count_spaces: true,
        first_chapter: true,
    })
    .unwrap();
    let id = project::overview(&a).unwrap().parts[0].docs[0].id.clone();
    doc::save_body(
        &a,
        &id,
        body(&["노트북에서 쓴 글."]),
        chrono::Duration::hours(1),
        SaveGuard::default(),
    )
    .unwrap();
    let folder = providers::folder_name("달빛 서점");
    let link = Link {
        provider,
        folder: folder.clone(),
        linked_at: String::new(),
        synced_at: None,
        error: None,
    };
    let lock = Mutex::new(());
    let (base_a, base_b) = (
        dir.path().join("base-a.json"),
        dir.path().join("base-b.json"),
    );
    let report = accounts::sync_project(&session, &link, &a, &base_a, &lock).unwrap();
    assert!(
        report.uploaded.contains(&"project.json".to_string()),
        "{report:?}"
    );
    assert_eq!(providers::projects(&session).unwrap(), vec![folder.clone()]);

    let b = dir.path().join("b").join(&folder);
    fs::create_dir_all(&b).unwrap();
    accounts::sync_project(&session, &link, &b, &base_b, &lock).unwrap();
    assert_eq!(files(&a), files(&b));

    // Both change the chapter: B keeps its own, A's arrives as a copy.
    doc::save_body(
        &a,
        &id,
        body(&["노트북에서 고친 글."]),
        chrono::Duration::hours(1),
        SaveGuard::default(),
    )
    .unwrap();
    doc::save_body(
        &b,
        &id,
        body(&["휴대폰에서 고친 글."]),
        chrono::Duration::hours(1),
        SaveGuard::default(),
    )
    .unwrap();
    accounts::sync_project(&session, &link, &a, &base_a, &lock).unwrap();
    let report = accounts::sync_project(&session, &link, &b, &base_b, &lock).unwrap();
    assert_eq!(report.copies.len(), 1, "{report:?}");
    accounts::sync_project(&session, &link, &a, &base_a, &lock).unwrap();
    assert_eq!(
        doc::load(&a, &id).unwrap().body,
        body(&["휴대폰에서 고친 글."])
    );
    assert_eq!(copies::list(&a).unwrap().len(), 1);
    assert_eq!(files(&a), files(&b));

    // A removes the copy (the writer picked): gone from the drive and from B.
    let copy = copies::list(&a).unwrap().remove(0);
    fs::remove_file(a.join("manuscript").join(&copy.file)).unwrap();
    let report = accounts::sync_project(&session, &link, &a, &base_a, &lock).unwrap();
    assert_eq!(report.removed_there.len(), 1, "{report:?}");
    accounts::sync_project(&session, &link, &b, &base_b, &lock).unwrap();
    assert!(copies::list(&b).unwrap().is_empty());
    assert_eq!(files(&a), files(&b));

    // Nothing left to do.
    let quiet = accounts::sync_project(&session, &link, &b, &base_b, &lock).unwrap();
    assert!(
        quiet.uploaded.is_empty() && quiet.downloaded.is_empty(),
        "{quiet:?}"
    );
    assert!(Base::load(&base_b).synced_at.is_some());

    // Forgetting the sign-in removes it from the credential store.
    session.forget().unwrap();
    assert!(secrets.get(&connection.key).unwrap().is_none());
}

#[test]
fn google_drive() {
    story(Provider::Google);
}

#[test]
fn onedrive() {
    story(Provider::Onedrive);
}

#[test]
fn dropbox() {
    story(Provider::Dropbox);
}

// ---------------------------------------------------------------------------
// A drive filling up

/// A stand-in drive and a session signed in to it.
fn signed_in(provider: Provider) -> (Endpoints, Session) {
    let ends = stand_in(provider);
    let app = AppId {
        client_id: "writerprogram-test".into(),
        client_secret: (provider == Provider::Google).then(|| "not-a-secret".into()),
        redirect_port: Some(0),
    };
    let secrets: Arc<dyn Secrets> = Arc::new(MemorySecrets::default());
    let cancel = AtomicBool::new(false);
    let browser = |url: &str| {
        let url = url.to_string();
        thread::spawn(move || writer_sync::http::fetch(&url).unwrap());
        Ok(())
    };
    let connection = accounts::sign_in(
        provider,
        &app,
        ends.clone(),
        secrets.clone(),
        browser,
        &cancel,
    )
    .unwrap();
    let session = Session::resume(provider, &app, ends.clone(), secrets, connection.key).unwrap();
    (ends, session)
}

const MB: u64 = 1024 * 1024;

fn filling_up(provider: Provider) {
    let dir = tempfile::tempdir().unwrap();
    let (ends, session) = signed_in(provider);
    let a = project::create(&NewProject {
        parent: dir.path().join("a").to_string_lossy().into_owned(),
        title: "달빛 서점".into(),
        kind: ProjectKind::Webnovel,
        per_doc_goal: None,
        count_spaces: true,
        first_chapter: true,
    })
    .unwrap();
    let id = project::overview(&a).unwrap().parts[0].docs[0].id.clone();
    let link = Link {
        provider,
        folder: providers::folder_name("달빛 서점"),
        linked_at: String::new(),
        synced_at: None,
        error: None,
    };
    let lock = Mutex::new(());
    let (base_a, base_b) = (
        dir.path().join("base-a.json"),
        dir.path().join("base-b.json"),
    );
    let b = dir.path().join("b");
    fs::create_dir_all(&b).unwrap();

    // Plenty of room: no warning.
    let report = accounts::sync_project(&session, &link, &a, &base_a, &lock).unwrap();
    assert_eq!(report.space_left, None, "{report:?}");
    accounts::sync_project(&session, &link, &b, &base_b, &lock).unwrap();

    // The drive will not say how full it is: the pass goes on all the same.
    set_room(
        &ends,
        Room {
            total: Some(40 * MB),
            elsewhere: 0,
            readable: false,
        },
    );
    let report = accounts::sync_project(&session, &link, &a, &base_a, &lock).unwrap();
    assert_eq!(report.space_left, None);

    // Under 50 MB left: the report says how much.
    set_room(
        &ends,
        Room {
            total: Some(40 * MB),
            elsewhere: 30 * MB,
            readable: true,
        },
    );
    let report = accounts::sync_project(&session, &link, &a, &base_a, &lock).unwrap();
    let left = report.space_left.expect("running low");
    assert!(left < 10 * MB && left > 9 * MB, "{left}");

    // Full: the pass stops with the same words on every drive, and the
    // chapter stays on this device as written.
    set_room(
        &ends,
        Room {
            total: Some(40 * MB),
            elsewhere: 40 * MB,
            readable: true,
        },
    );
    doc::save_body(
        &a,
        &id,
        body(&["공간이 없을 때 쓴 글."]),
        chrono::Duration::hours(1),
        SaveGuard::default(),
    )
    .unwrap();
    let written = files(&a);
    let err = accounts::sync_project(&session, &link, &a, &base_a, &lock).unwrap_err();
    assert!(err.user_message().contains(providers::FULL), "{err}");
    assert_eq!(files(&a), written);
    assert_eq!(
        doc::load(&a, &id).unwrap().body,
        body(&["공간이 없을 때 쓴 글."])
    );
    // Again later, still full: nothing lost, nothing changed here.
    assert!(accounts::sync_project(&session, &link, &a, &base_a, &lock).is_err());
    assert_eq!(files(&a), written);

    // Room again: the chapter goes up on the next pass and reaches B.
    set_room(&ends, Room::default());
    let report = accounts::sync_project(&session, &link, &a, &base_a, &lock).unwrap();
    assert!(
        report
            .uploaded
            .iter()
            .any(|p| p.ends_with(&format!("{id}.md"))),
        "{report:?}"
    );
    accounts::sync_project(&session, &link, &b, &base_b, &lock).unwrap();
    assert_eq!(
        doc::load(&b, &id).unwrap().body,
        body(&["공간이 없을 때 쓴 글."])
    );
}

#[test]
fn google_drive_filling_up() {
    filling_up(Provider::Google);
}

#[test]
fn onedrive_filling_up() {
    filling_up(Provider::Onedrive);
}

#[test]
fn dropbox_filling_up() {
    filling_up(Provider::Dropbox);
}

#[test]
fn a_sign_in_from_elsewhere_is_refused() {
    let ends = stand_in(Provider::Dropbox);
    let app = AppId {
        client_id: "x".into(),
        client_secret: None,
        redirect_port: Some(0),
    };
    let cancel = AtomicBool::new(false);
    let result = accounts::sign_in(
        Provider::Dropbox,
        &app,
        ends,
        Arc::new(MemorySecrets::default()),
        |url: &str| {
            let query = url.split_once('?').unwrap().1;
            let params: HashMap<String, String> = url::form_urlencoded::parse(query.as_bytes())
                .into_owned()
                .collect();
            let redirect = params["redirect_uri"].clone();
            thread::spawn(move || {
                let addr = redirect
                    .trim_start_matches("http://")
                    .trim_end_matches('/')
                    .replace("localhost", "127.0.0.1");
                let mut stream = TcpStream::connect(addr).unwrap();
                // A different state: not the answer to this sign-in.
                write!(stream, "GET /?code=x&state=someone-else HTTP/1.1\r\n\r\n").unwrap();
                let mut page = String::new();
                let _ = stream.read_to_string(&mut page);
            });
            Ok(())
        },
        &cancel,
    );
    assert!(result.unwrap_err().user_message().contains("다른 로그인"));
}
