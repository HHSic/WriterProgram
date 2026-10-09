use writer_core::doc::{self, SaveGuard, Section};
use writer_core::markup::Block;
use writer_core::project::{self, NewDoc, NewProject, ProjectKind};
use writer_core::share;

#[test]
fn only_selected_manuscript_text_is_shared_in_project_order() {
    let dir = tempfile::tempdir().unwrap();
    let root = project::create(&NewProject {
        parent: dir.path().to_string_lossy().into_owned(),
        title: "공유 작품".into(),
        kind: ProjectKind::Webnovel,
        per_doc_goal: None,
        count_spaces: true,
        first_chapter: false,
        platform: None,
    })
    .unwrap();
    let mut ids = Vec::new();
    for (title, text) in [
        ("첫 장", "원고 (ง •̀_•́)ง"),
        ("둘째", "공유하지 않는 원고"),
        ("셋째", "마지막 <표정> 😊"),
    ] {
        let id = project::add_doc(
            &root,
            &NewDoc {
                title: title.into(),
                ..Default::default()
            },
        )
        .unwrap();
        doc::save_body(
            &root,
            &id,
            vec![Block::text(text)],
            chrono::Duration::minutes(10),
            SaveGuard::default(),
        )
        .unwrap();
        ids.push(id);
    }
    let private = project::add_doc(
        &root,
        &NewDoc {
            section: Some(Section::Planning),
            title: "비밀 기획".into(),
            ..Default::default()
        },
    )
    .unwrap();
    let snapshot = share::snapshot(&root, &[ids[2].clone(), ids[0].clone()]).unwrap();
    assert_eq!(snapshot.chapters.len(), 2);
    assert_eq!(snapshot.chapters[0].title, "첫 장");
    assert_eq!(snapshot.chapters[0].text, "원고 (ง •̀_•́)ง");
    assert_eq!(snapshot.chapters[1].text, "마지막 <표정> 😊");
    let json = serde_json::to_string(&snapshot).unwrap();
    for hidden in [
        "공유하지 않는 원고",
        "비밀 기획",
        &ids[0],
        "synopsis",
        "root",
    ] {
        assert!(!json.contains(hidden), "{hidden}");
    }
    assert!(share::snapshot(&root, &[]).is_err());
    assert!(share::snapshot(&root, &[private]).is_err());
    assert!(share::snapshot(&root, &[ids[0].clone(), "../project.json".into()]).is_err());

    doc::save_body(
        &root,
        &ids[0],
        vec![Block::text(&"\"".repeat(5 * 1024 * 1024))],
        chrono::Duration::minutes(10),
        SaveGuard::default(),
    )
    .unwrap();
    assert!(share::snapshot(&root, &[ids[0].clone()]).is_err());
}
