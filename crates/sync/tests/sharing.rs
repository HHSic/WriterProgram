mod support;

use std::sync::Arc;
use writer_sync::oauth::Tokens;
use writer_sync::providers::{self, AppId, Provider, Session};
use writer_sync::remote::Put;
use writer_sync::secrets::MemorySecrets;
use writer_sync::sharing;

fn connected(provider: Provider, policy: &str) -> Session {
    Session::new(
        provider,
        &AppId {
            client_id: "sharing-test".into(),
            ..Default::default()
        },
        support::share_stand_in(provider, policy),
        Tokens {
            access: "share-test".into(),
            refresh: None,
            expires_at: chrono::Utc::now().timestamp() + 3600,
        },
        Arc::new(MemorySecrets::default()),
    )
}

fn story(provider: Provider) {
    let drive = connected(provider, "ok");
    let mut private = providers::open(&drive, "Private manuscript", true).unwrap();
    assert!(matches!(
        private
            .put("project.json", b"private project", None)
            .unwrap(),
        Put::Done(_)
    ));
    let name = sharing::new_file_name().unwrap();
    let bytes = "읽기용 사본 (ง •̀_•́)ง".as_bytes();
    let link = sharing::publish(&drive, &name, bytes).unwrap();
    assert!(link.url.starts_with("https://"));
    assert!(!link.url.contains("127.0.0.1"));
    let mut copies = providers::open(&drive, "WriterProgram Shared Manuscripts", false).unwrap();
    assert_eq!(copies.get(&name).unwrap(), bytes);
    assert!(sharing::publish(&drive, &name, b"must not overwrite").is_err());
    assert_eq!(copies.get(&name).unwrap(), bytes);
    assert!(sharing::publish(&drive, "../project.json", bytes).is_err());
    sharing::revoke(&drive, &name, &link.url).unwrap();
    assert!(copies.list().unwrap().is_empty());
    sharing::revoke(&drive, &name, &link.url).unwrap();
    assert_eq!(private.get("project.json").unwrap(), b"private project");
    for policy in ["deny", "invalid"] {
        let drive = connected(provider, policy);
        let name = sharing::new_file_name().unwrap();
        assert!(sharing::publish(&drive, &name, bytes).is_err());
        assert!(
            providers::open(&drive, "WriterProgram Shared Manuscripts", false)
                .unwrap()
                .list()
                .unwrap()
                .is_empty()
        );
    }
}

#[test]
fn google_public_reader_copy_and_cleanup() {
    story(Provider::Google);
}
#[test]
fn onedrive_public_reader_copy_and_cleanup() {
    story(Provider::Onedrive);
}
#[test]
fn dropbox_public_reader_copy_and_cleanup() {
    story(Provider::Dropbox);
}
