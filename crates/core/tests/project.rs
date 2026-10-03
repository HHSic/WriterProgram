use std::fs;
use std::path::{Path, PathBuf};

use chrono::Duration;
use writer_core::count::{Counts, count_blocks};
use writer_core::doc::{self, MetaPatch, Section};
use writer_core::export::{self, ExportItem, TextOptions};
use writer_core::markup::{Block, Body};
use writer_core::project::{self, NewDoc, NewProject, ProjectKind};
use writer_core::{recent, snapshot, trash};

fn new_project(dir: &Path, kind: ProjectKind) -> PathBuf {
    project::create(&NewProject {
        parent: dir.to_string_lossy().into_owned(),
        title: "달빛 서점의 마지막 손님".into(),
        kind,
        per_doc_goal: Some(5000),
        count_spaces: true,
        first_chapter: true,
    })
    .unwrap()
}

fn first_doc(root: &Path) -> String {
    project::overview(root).unwrap().parts[0].docs[0].id.clone()
}

fn body(texts: &[&str]) -> Vec<Block> {
    texts.iter().map(|t| Block::text(t)).collect()
}

#[test]
fn create_and_open() {
    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path(), ProjectKind::Webnovel);
    assert_eq!(root.file_name().unwrap(), "달빛 서점의 마지막 손님");

    let ov = project::open(&root).unwrap();
    assert_eq!(ov.project.kind, ProjectKind::Webnovel);
    assert_eq!(ov.project.goal.per_doc, Some(5000));
    assert_eq!(ov.parts.len(), 1);
    assert_eq!(ov.parts[0].title, "1부");
    assert_eq!(ov.parts[0].docs.len(), 1);
    let planning: Vec<_> = ov.planning.iter().map(|d| d.title.as_str()).collect();
    assert_eq!(planning, ["시놉시스", "작품 소개"]);
    assert_eq!(ov.trash_count, 0);

    // A second project with the same title gets its own folder.
    let again = new_project(dir.path(), ProjectKind::Print);
    assert_eq!(again.file_name().unwrap(), "달빛 서점의 마지막 손님 (2)");
    assert_eq!(project::load(&again).unwrap().scene_break, "*");
}

#[test]
fn not_a_project_folder() {
    let dir = tempfile::tempdir().unwrap();
    let err = project::open(dir.path()).unwrap_err();
    assert!(err.user_message().contains("작품 폴더가 아님"));
}

#[test]
fn saving_keeps_meta_changed_elsewhere() {
    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path(), ProjectKind::Webnovel);
    let id = first_doc(&root);

    doc::update_meta(
        &root,
        &id,
        &MetaPatch {
            title: Some("비에 젖은 손님".into()),
            status: Some("stock".into()),
            target: Some(Some(4000)),
            ..Default::default()
        },
    )
    .unwrap();
    let out = doc::save_body(
        &root,
        &id,
        body(&["셔터를 반쯤 내렸을 때 종이 울렸다."]),
        Duration::minutes(10),
        Default::default(),
    )
    .unwrap();
    assert_eq!(out.counts.with_spaces, 20);
    assert!(out.snapshot.is_none(), "an empty document needs no record");

    let file = doc::load(&root, &id).unwrap();
    assert_eq!(file.meta.title, "비에 젖은 손님");
    assert_eq!(file.meta.status, "stock");
    assert_eq!(file.meta.target, Some(4000));
    assert_eq!(file.body, body(&["셔터를 반쯤 내렸을 때 종이 울렸다."]));

    // The file on disk is plain text a writer can read.
    let text = fs::read_to_string(root.join("manuscript").join(format!("{id}.md"))).unwrap();
    assert!(text.contains("title: \"비에 젖은 손님\""));
    assert!(text.ends_with("셔터를 반쯤 내렸을 때 종이 울렸다.\n"));

    let err = doc::update_meta(
        &root,
        &id,
        &MetaPatch {
            status: Some("nope".into()),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(err.user_message().contains("알 수 없는 상태"));
}

#[test]
fn automatic_records() {
    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path(), ProjectKind::Webnovel);
    let id = first_doc(&root);
    let every = Duration::zero();

    doc::save_body(&root, &id, body(&["하나"]), every, Default::default()).unwrap();
    let out = doc::save_body(&root, &id, body(&["하나", "둘"]), every, Default::default()).unwrap();
    let rec = out.snapshot.expect("previous text is kept");
    assert_eq!(rec.kind, "auto");
    assert_eq!(rec.counts.with_spaces, 2);

    // Saving the same text again does nothing.
    let out = doc::save_body(&root, &id, body(&["하나", "둘"]), every, Default::default()).unwrap();
    assert!(out.snapshot.is_none());

    // Not due yet with a long interval.
    let out = doc::save_body(
        &root,
        &id,
        body(&["하나", "둘", "셋"]),
        Duration::hours(1),
        Default::default(),
    )
    .unwrap();
    assert!(out.snapshot.is_none());
    assert_eq!(snapshot::list(&root, &id).unwrap().len(), 1);
}

#[test]
fn records_by_hand_and_going_back() {
    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path(), ProjectKind::Print);
    let id = first_doc(&root);
    let hour = Duration::hours(1);

    doc::save_body(
        &root,
        &id,
        body(&["퇴고 전 문장."]),
        hour,
        Default::default(),
    )
    .unwrap();
    let current = doc::load(&root, &id).unwrap();
    let kept = snapshot::create(&root, &current, "manual", "  1교 보내기 전 ").unwrap();
    assert_eq!(kept.name, "1교 보내기 전");

    doc::save_body(
        &root,
        &id,
        body(&["완전히 새로 쓴 문장."]),
        hour,
        Default::default(),
    )
    .unwrap();
    let before = snapshot::restore(&root, &id, &kept.id).unwrap();
    assert_eq!(before.kind, "before-restore");

    assert_eq!(
        doc::load(&root, &id).unwrap().body,
        body(&["퇴고 전 문장."])
    );
    let list = snapshot::list(&root, &id).unwrap();
    let kinds: Vec<_> = list.iter().map(|r| r.kind.as_str()).collect();
    assert_eq!(kinds, ["before-restore", "manual"]);
    assert_eq!(
        snapshot::load(&root, &id, &list[0].id).unwrap().body,
        body(&["완전히 새로 쓴 문장."])
    );

    assert!(snapshot::load(&root, &id, "../project").is_err());
}

/// A record file of `kind`, `days` old, for the document `id`.
fn old_record(root: &Path, id: &str, kind: &str, days: i64) -> String {
    record_at(root, id, kind, chrono::Utc::now() - Duration::days(days))
}

/// A record file of `kind` made `at`, for the document `id`.
fn record_at(root: &Path, id: &str, kind: &str, at: chrono::DateTime<chrono::Utc>) -> String {
    let current = doc::load(root, id).unwrap();
    let made = snapshot::create(root, &current, kind, "").unwrap();
    let records = root.join(".snapshots").join(id);
    let stem = format!("{}.{kind}", writer_core::store::stamp(at));
    fs::rename(
        records.join(format!("{}.md", made.id)),
        records.join(format!("{stem}.md")),
    )
    .unwrap();
    stem
}

#[test]
fn tidying_records_removes_only_old_automatic_ones() {
    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path(), ProjectKind::Webnovel);
    let id = first_doc(&root);
    doc::save_body(
        &root,
        &id,
        body(&["기록할 문장."]),
        Duration::hours(1),
        Default::default(),
    )
    .unwrap();

    let auto_5 = old_record(&root, &id, "auto", 5);
    let auto_20 = old_record(&root, &id, "auto", 20);
    let auto_30 = old_record(&root, &id, "auto", 30);
    let manual_100 = old_record(&root, &id, "manual", 100);
    let copy_100 = old_record(&root, &id, "before-copy", 100);
    let reload_20 = old_record(&root, &id, "before-reload", 20);
    let reload_100 = old_record(&root, &id, "before-reload", 100);
    let replace_40 = old_record(&root, &id, "before-replace", 40);

    // Another chapter whose automatic records are all old: its newest stays.
    let other = project::add_doc(&root, &NewDoc::default()).unwrap();
    doc::save_body(
        &root,
        &other,
        body(&["다른 회차."]),
        Duration::hours(1),
        Default::default(),
    )
    .unwrap();
    let other_40 = old_record(&root, &other, "auto", 40);
    let other_30 = old_record(&root, &other, "auto", 30);

    let sizes = project::sizes(&root).unwrap();
    assert!(sizes.writing > 0 && sizes.records > 0);
    assert_eq!(sizes.trash, 0);
    assert_eq!(sizes.journal, 0);
    // Far from crowded: a few small records.
    assert!(!sizes.suggest_tidy);

    let freed = project::tidy_records(&root).unwrap();
    assert_eq!(freed, sizes.tidy_frees);
    assert!(freed > 0);

    let stems = |doc: &str| -> Vec<String> {
        snapshot::list(&root, doc)
            .unwrap()
            .into_iter()
            .map(|r| r.id)
            .collect()
    };
    let kept = stems(&id);
    for stem in [&auto_5, &manual_100, &copy_100, &reload_20, &replace_40] {
        assert!(kept.contains(stem), "{stem} should stay: {kept:?}");
    }
    for stem in [&auto_20, &auto_30, &reload_100] {
        assert!(!kept.contains(stem), "{stem} should go: {kept:?}");
    }
    // The other chapter keeps its newest automatic record only.
    assert_eq!(stems(&other), [other_30]);
    assert!(other_40 < stems(&other)[0]);

    // Nothing more to tidy.
    assert_eq!(project::sizes(&root).unwrap().tidy_frees, 0);
}

#[test]
fn keeping_daily_states_spares_each_days_last_record() {
    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path(), ProjectKind::Webnovel);
    let id = first_doc(&root);
    doc::save_body(
        &root,
        &id,
        body(&["하루의 마지막 상태."]),
        Duration::hours(1),
        Default::default(),
    )
    .unwrap();
    // Noon (local time) `days` ago, give or take some hours: same day.
    let noon = |days: i64, hours: i64| {
        let day = chrono::Local::now().date_naive() - Duration::days(days);
        day.and_hms_opt(12, 0, 0)
            .unwrap()
            .and_local_timezone(chrono::Local)
            .unwrap()
            .with_timezone(&chrono::Utc)
            + Duration::hours(hours)
    };
    let early_100 = record_at(&root, &id, "auto", noon(100, -3));
    let late_100 = record_at(&root, &id, "auto", noon(100, 2));
    let early_30 = record_at(&root, &id, "auto", noon(30, -3));
    let late_30 = record_at(&root, &id, "auto", noon(30, 2));
    // The newest automatic record stays anyway; the ones above are older.
    record_at(&root, &id, "auto", noon(20, 0));
    let stems = || -> Vec<String> {
        snapshot::list(&root, &id)
            .unwrap()
            .into_iter()
            .map(|r| r.id)
            .collect()
    };

    let off = project::sizes(&root).unwrap();
    assert_eq!(off.daily, 0);
    let info = project::update(
        &root,
        &project::ProjectPatch {
            keep_daily: Some(true),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(info.keep_daily);
    let on = project::sizes(&root).unwrap();
    assert!(on.daily > 0);
    assert_eq!(on.tidy_frees + on.daily, off.tidy_frees);

    // Opening clears records past 90 days, but not the day's last one.
    project::open(&root).unwrap();
    let kept = stems();
    assert!(
        kept.contains(&late_100) && !kept.contains(&early_100),
        "{kept:?}"
    );
    project::tidy_records(&root).unwrap();
    let kept = stems();
    assert!(
        kept.contains(&late_30) && !kept.contains(&early_30),
        "{kept:?}"
    );
    assert!(kept.contains(&late_100));

    // Turned off, the next opening clears it like any old automatic record.
    project::update(
        &root,
        &project::ProjectPatch {
            keep_daily: Some(false),
            ..Default::default()
        },
    )
    .unwrap();
    project::open(&root).unwrap();
    assert!(!stems().contains(&late_100));
}

#[test]
fn structure_editing() {
    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path(), ProjectKind::Webnovel);
    let first = first_doc(&root);
    let part1 = project::overview(&root).unwrap().parts[0].id.clone();

    let third = project::add_doc(&root, &NewDoc::default()).unwrap();
    let second = project::add_doc(
        &root,
        &NewDoc {
            after: Some(first.clone()),
            title: "끼워 넣은 회차".into(),
            ..Default::default()
        },
    )
    .unwrap();
    let ids = |root: &Path| -> Vec<Vec<String>> {
        project::overview(root)
            .unwrap()
            .parts
            .iter()
            .map(|p| p.docs.iter().map(|d| d.id.clone()).collect())
            .collect()
    };
    assert_eq!(
        ids(&root),
        vec![vec![first.clone(), second.clone(), third.clone()]]
    );

    let part2 = project::add_part(&root, "").unwrap();
    assert_eq!(project::overview(&root).unwrap().parts[1].title, "2부");
    project::rename_part(&root, &part2, "2부 · 비밀 서가").unwrap();
    project::move_doc(&root, &third, Some(&part2), 0).unwrap();
    project::move_doc(&root, &first, Some(&part1), 1).unwrap();
    assert_eq!(
        ids(&root),
        vec![vec![second.clone(), first.clone()], vec![third.clone()]]
    );

    assert!(project::remove_part(&root, &part2).is_err());
    project::move_doc(&root, &third, Some(&part1), 99).unwrap();
    project::remove_part(&root, &part2).unwrap();
    assert_eq!(ids(&root), vec![vec![second, first, third]]);

    let planning = project::add_doc(
        &root,
        &NewDoc {
            section: Some(Section::Planning),
            title: "플롯 구상".into(),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(project::move_doc(&root, &planning, Some(&part1), 0).is_err());
    project::move_doc(&root, &planning, None, 0).unwrap();
    assert_eq!(
        project::overview(&root).unwrap().planning[0].title,
        "플롯 구상"
    );
}

#[test]
fn trash_and_restore() {
    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path(), ProjectKind::Webnovel);
    let first = first_doc(&root);
    let second = project::add_doc(&root, &NewDoc::default()).unwrap();
    doc::save_body(
        &root,
        &first,
        body(&["지울 회차"]),
        Duration::hours(1),
        Default::default(),
    )
    .unwrap();

    let item = trash::trash_doc(&root, &first).unwrap();
    assert_eq!(item.chars, 5);
    let ov = project::overview(&root).unwrap();
    assert_eq!(ov.parts[0].docs.len(), 1);
    assert_eq!(ov.trash_count, 1);
    assert!(doc::load(&root, &first).is_err());

    trash::restore(&root, &item.id).unwrap();
    let ov = project::overview(&root).unwrap();
    let order: Vec<_> = ov.parts[0].docs.iter().map(|d| d.id.clone()).collect();
    assert_eq!(order, [first.clone(), second.clone()]);
    assert_eq!(ov.trash_count, 0);

    // Deleting for good also removes the document's records.
    let current = doc::load(&root, &second).unwrap();
    snapshot::create(&root, &current, "manual", "").unwrap();
    let item = trash::trash_doc(&root, &second).unwrap();
    trash::delete(&root, &item.id).unwrap();
    assert!(trash::list(&root).unwrap().is_empty());
    assert!(!root.join(".snapshots").join(&second).exists());
}

#[test]
fn open_repairs_structure_from_files() {
    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path(), ProjectKind::Webnovel);
    let first = first_doc(&root);

    // A chapter file arrives by folder sync; another one disappears.
    let manuscript = root.join("manuscript");
    fs::write(
        manuscript.join("synced123.md"),
        "---\nid: \"synced123\"\ntitle: \"다른 기기에서 쓴 회차\"\n---\n\n본문\n",
    )
    .unwrap();
    fs::remove_file(manuscript.join(format!("{first}.md"))).unwrap();
    // A file without front matter is read as a plain chapter.
    fs::write(manuscript.join("loose.md"), "그냥 쓴 글\r\n").unwrap();

    let ov = project::open(&root).unwrap();
    let titles: Vec<_> = ov.parts[0].docs.iter().map(|d| d.title.as_str()).collect();
    assert_eq!(titles.len(), 2);
    assert!(titles.contains(&"다른 기기에서 쓴 회차"));
    assert_eq!(
        ov.total.with_spaces,
        "본문".chars().count() as u32 + "그냥 쓴 글".chars().count() as u32
    );
}

/// Holds a file the way a sync program or virus scanner does, so it cannot
/// be removed until the guard is dropped. On Windows an open handle that
/// shares nothing; elsewhere a folder that cannot be written.
struct Held {
    #[cfg(windows)]
    _file: fs::File,
    #[cfg(not(windows))]
    dir: PathBuf,
}

fn hold(file: &Path) -> Held {
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        let _file = fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(file)
            .unwrap();
        Held { _file }
    }
    #[cfg(not(windows))]
    {
        use std::os::unix::fs::PermissionsExt;
        let dir = file.parent().unwrap().to_path_buf();
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o555)).unwrap();
        Held { dir }
    }
}

#[cfg(not(windows))]
impl Drop for Held {
    fn drop(&mut self) {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&self.dir, fs::Permissions::from_mode(0o755));
    }
}

/// A trash item made to look `days` old.
fn age_trash_item(root: &Path, id: &str, days: i64) {
    let path = root.join(".trash").join(id).join("item.json");
    let mut item: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    item["deletedAt"] =
        writer_core::store::to_iso(chrono::Utc::now() - Duration::days(days)).into();
    fs::write(&path, serde_json::to_string(&item).unwrap()).unwrap();
}

#[test]
fn open_succeeds_when_cleanup_cannot_remove_a_file() {
    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path(), ProjectKind::Webnovel);
    let held_doc = first_doc(&root);
    let free_doc = project::add_doc(&root, &NewDoc::default()).unwrap();
    for id in [&held_doc, &free_doc] {
        doc::save_body(
            &root,
            id,
            body(&["오래된 기록이 남은 회차."]),
            Duration::hours(1),
            Default::default(),
        )
        .unwrap();
    }
    let held_record = old_record(&root, &held_doc, "auto", 120);
    let free_record = old_record(&root, &free_doc, "auto", 120);

    // Two chapters deleted long ago; one of them is held by another program.
    let gone_a = project::add_doc(&root, &NewDoc::default()).unwrap();
    let gone_b = project::add_doc(&root, &NewDoc::default()).unwrap();
    let held_item = trash::trash_doc(&root, &gone_a).unwrap();
    let free_item = trash::trash_doc(&root, &gone_b).unwrap();
    age_trash_item(&root, &held_item.id, 40);
    age_trash_item(&root, &free_item.id, 40);

    let records = root.join(".snapshots");
    let record_file = |doc: &str, stem: &str| records.join(doc).join(format!("{stem}.md"));
    let item_file = root
        .join(".trash")
        .join(&held_item.id)
        .join(format!("{gone_a}.md"));
    {
        let _record = hold(&record_file(&held_doc, &held_record));
        let _item = hold(&item_file);

        let ov = project::open(&root).expect("opens although clearing failed");
        assert_eq!(ov.parts[0].docs.len(), 2);

        // What could be removed was; what was held stays for next time.
        assert!(!record_file(&free_doc, &free_record).exists());
        assert!(record_file(&held_doc, &held_record).exists());
        let left: Vec<_> = trash::list(&root)
            .unwrap()
            .into_iter()
            .map(|i| i.id)
            .collect();
        assert_eq!(left, std::slice::from_ref(&held_item.id));
    }

    // Let go: the next opening clears the rest.
    project::open(&root).unwrap();
    assert!(!record_file(&held_doc, &held_record).exists());
    assert!(trash::list(&root).unwrap().is_empty());
}

#[test]
fn steps_that_guard_the_manuscript_still_stop_opening_in_plain_words() {
    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path(), ProjectKind::Webnovel);
    // A sync program's copy whose original is gone takes its place on
    // opening; held by another program, it cannot be moved.
    let copy = root.join("manuscript").join("k7q2m9x4t1ab (1).md");
    fs::write(
        &copy,
        "---\nid: \"k7q2m9x4t1ab\"\ntitle: \"사본\"\n---\n\n본문\n",
    )
    .unwrap();
    {
        let _held = hold(&copy);
        let err = project::open(&root).unwrap_err().user_message();
        assert!(
            err.starts_with("동기화 프로그램이 남긴 사본을 정리하지 못함 · "),
            "{err}"
        );
        assert!(err.ends_with("(k7q2m9x4t1ab (1).md)"), "{err}");
    }
    assert!(project::open(&root).is_ok());
}

#[test]
fn clearing_reports_what_it_could_not_remove() {
    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path(), ProjectKind::Webnovel);
    let id = first_doc(&root);
    doc::save_body(
        &root,
        &id,
        body(&["기록할 문장."]),
        Duration::hours(1),
        Default::default(),
    )
    .unwrap();
    let old = old_record(&root, &id, "auto", 120);
    let path = root.join(".snapshots").join(&id).join(format!("{old}.md"));
    let held = hold(&path);
    let done = snapshot::prune(&root, Duration::days(90), false);
    assert_eq!(done.removed, 0);
    assert_eq!(done.failed, std::slice::from_ref(&path));
    drop(held);
    let done = snapshot::prune(&root, Duration::days(90), false);
    assert_eq!((done.removed, done.failed.len()), (1, 0));
}

#[test]
fn text_export() {
    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path(), ProjectKind::Webnovel);
    let first = first_doc(&root);
    let second = project::add_doc(&root, &NewDoc::default()).unwrap();
    doc::save_body(
        &root,
        &first,
        vec![
            Block::text("첫 화"),
            Block::SceneBreak {},
            Block::text("장면 둘"),
        ],
        Duration::hours(1),
        Default::default(),
    )
    .unwrap();
    doc::save_body(
        &root,
        &second,
        body(&["둘째 화"]),
        Duration::hours(1),
        Default::default(),
    )
    .unwrap();

    let items = vec![
        ExportItem {
            doc_id: first.clone(),
            heading: "1화 시작".into(),
            file_name: "달빛_001".into(),
        },
        ExportItem {
            doc_id: second.clone(),
            heading: "2화".into(),
            file_name: "달빛_002".into(),
        },
    ];
    let opts = TextOptions {
        include_titles: true,
        blank_line_between: true,
        scene_break: "◆".into(),
    };
    let text = export::items_text(&root, &items, &opts).unwrap();
    assert_eq!(
        text,
        "1화 시작\n\n첫 화\n\n◆\n\n장면 둘\n\n\n2화\n\n둘째 화"
    );

    let out = dir.path().join("out");
    fs::create_dir_all(&out).unwrap();
    let files = export::export_txt(&root, &items, &opts, &out, true).unwrap();
    assert_eq!(files.len(), 2);
    let bytes = fs::read(&files[0]).unwrap();
    assert!(
        bytes.starts_with(b"\xEF\xBB\xBF1\xED\x99\x94"),
        "BOM then text"
    );
    assert!(String::from_utf8_lossy(&bytes).contains("첫 화\r\n\r\n◆"));

    let single = dir.path().join("전체.txt");
    export::export_txt(&root, &items, &opts, &single, false).unwrap();
    assert!(single.is_file());
}

#[test]
fn recent_list() {
    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path(), ProjectKind::Webnovel);
    let file = dir.path().join("settings").join("recent.json");
    assert!(recent::list(&file).is_empty());

    let ov = project::open(&root).unwrap();
    recent::touch(&file, &ov).unwrap();
    recent::touch(&file, &ov).unwrap();
    let items = recent::list(&file);
    assert_eq!(items.len(), 1);
    assert!(items[0].exists);
    assert_eq!(items[0].title, "달빛 서점의 마지막 손님");

    fs::rename(&root, dir.path().join("moved")).unwrap();
    assert!(!recent::list(&file)[0].exists);
    recent::remove(&file, &items[0].path).unwrap();
    assert!(recent::list(&file).is_empty());
}

/// Same cases as app/src/editor/counts.test.ts.
#[test]
fn counts_fixture() {
    #[derive(serde::Deserialize)]
    struct Case {
        name: String,
        body: Body,
        counts: Counts,
    }
    let text = include_str!("fixtures/counts.json");
    let cases: Vec<Case> = serde_json::from_str(text).unwrap();
    for case in cases {
        assert_eq!(
            count_blocks(&case.body.content),
            case.counts,
            "{}",
            case.name
        );
    }
}

#[test]
fn find_and_replace_across_chapters() {
    use writer_core::search::{self, SearchQuery};

    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path(), ProjectKind::Webnovel);
    let first = first_doc(&root);
    let second = project::add_doc(&root, &NewDoc::default()).unwrap();
    doc::save_body(
        &root,
        &first,
        body(&["서하는 우산을 폈다.", "서하가 웃었다."]),
        Duration::hours(1),
        Default::default(),
    )
    .unwrap();
    doc::save_body(
        &root,
        &second,
        body(&["윤서하의 서점."]),
        Duration::hours(1),
        Default::default(),
    )
    .unwrap();

    let query = SearchQuery {
        text: "서하".into(),
        regex: false,
        whole_word: false,
        doc_ids: None,
    };
    let found = search::search(&root, &query).unwrap();
    assert_eq!(found.total, 3);
    assert_eq!(found.docs[0].doc_id, first);
    assert_eq!(found.docs[0].matches.len(), 2);

    // Only the first chapter.
    let scoped = SearchQuery {
        doc_ids: Some(vec![second.clone()]),
        ..query.clone()
    };
    assert_eq!(search::search(&root, &scoped).unwrap().total, 1);

    let outcome = search::replace_all(&root, &query, "하윤").unwrap();
    assert_eq!(outcome.replaced, 3);
    assert_eq!(outcome.docs.len(), 2);
    assert_eq!(outcome.docs[0].snapshot.kind, "before-replace");
    assert_eq!(
        doc::load(&root, &first).unwrap().body,
        body(&["하윤는 우산을 폈다.", "하윤가 웃었다."])
    );
    assert_eq!(
        doc::load(&root, &second).unwrap().body,
        body(&["윤하윤의 서점."])
    );

    // The record keeps the text from before, so it can be taken back.
    snapshot::restore(&root, &first, &outcome.docs[0].snapshot.id).unwrap();
    assert_eq!(
        doc::load(&root, &first).unwrap().body,
        body(&["서하는 우산을 폈다.", "서하가 웃었다."])
    );
}

#[test]
fn setting_cards() {
    use writer_core::cards;

    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path(), ProjectKind::Webnovel);
    let first = first_doc(&root);
    let second = project::add_doc(&root, &NewDoc::default()).unwrap();
    doc::save_body(
        &root,
        &first,
        body(&["윤서하는 우산을 폈다.", "서하가 웃었다."]),
        Duration::hours(1),
        Default::default(),
    )
    .unwrap();
    doc::save_body(
        &root,
        &second,
        body(&["달빛 서점은 밤에만 연다."]),
        Duration::hours(1),
        Default::default(),
    )
    .unwrap();

    // A new card starts with its kind's fields.
    let mut card = cards::create(&root, "person", " 윤서하 ").unwrap();
    assert_eq!(card.name, "윤서하");
    assert_eq!(card.fields[0], ("나이".to_string(), String::new()));
    card.aliases = vec!["서하".into(), "윤서하".into(), " ".into()];
    card.fields[1].1 = "서점 주인".into();
    let summary = cards::save(&root, &card).unwrap();
    assert_eq!(summary.aliases, vec!["서하".to_string()]);
    assert_eq!(summary.summary, "직업: 서점 주인");
    let place = cards::create(&root, "place", "달빛 서점").unwrap();

    let ov = project::overview(&root).unwrap();
    assert_eq!(ov.card_types.len(), 3);
    let names: Vec<_> = ov.cards.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["달빛 서점", "윤서하"]);

    let seen = cards::appearances(&root, &card.id).unwrap();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].doc_id, first);
    assert_eq!(seen[0].count, 2);
    let counts = cards::appearance_counts(&root).unwrap();
    assert!(counts.contains(&(card.id.clone(), 1)));
    assert!(counts.contains(&(place.id.clone(), 1)));

    // Kinds: add one, and a kind in use cannot be removed.
    let faction = cards::add_type(&root, "세력").unwrap();
    assert_eq!(project::overview(&root).unwrap().card_types.len(), 4);
    assert!(cards::remove_type(&root, "person").is_err());
    cards::remove_type(&root, &faction.id).unwrap();

    // Trash and back.
    let item = trash::trash_card(&root, &place.id).unwrap();
    assert_eq!(project::overview(&root).unwrap().cards.len(), 1);
    trash::restore(&root, &item.id).unwrap();
    assert_eq!(project::overview(&root).unwrap().cards.len(), 2);
}

#[test]
fn notes_on_text_chapters_cards_and_project() {
    use writer_core::cards;
    use writer_core::markup::{Inline, Mark, MemoAttrs};
    use writer_core::notes::{self, Anchor, NewNote, Reply};

    let dir = tempfile::tempdir().unwrap();
    let root = new_project(dir.path(), ProjectKind::Webnovel);
    let chapter = first_doc(&root);

    // The editor marks the text first, with an id it chose, then makes the note.
    let marked = vec![Block::Paragraph {
        attrs: Default::default(),
        content: vec![
            Inline::Text {
                text: "“영업, 끝났나요?”".into(),
                marks: vec![Mark::Memo {
                    attrs: MemoAttrs { id: "memo1".into() },
                }],
            },
            Inline::Text {
                text: " 남자가 물었다.".into(),
                marks: vec![],
            },
        ],
    }];
    doc::save_body(
        &root,
        &chapter,
        marked,
        Duration::hours(1),
        Default::default(),
    )
    .unwrap();
    let file = fs::read_to_string(root.join("manuscript").join(format!("{chapter}.md"))).unwrap();
    assert!(file.contains("<mark data-memo=\"memo1\">“영업, 끝났나요?”</mark> 남자가 물었다."));

    let mut note = notes::create(
        &root,
        &NewNote {
            id: Some("memo1".into()),
            anchor: Anchor::Text,
            target: chapter.clone(),
            quote: "“영업, 끝났나요?”".into(),
            text: String::new(),
        },
    )
    .unwrap();
    assert_eq!(note.id, "memo1");
    // The same id cannot be used twice, and the target must exist.
    let again = NewNote {
        id: Some("memo1".into()),
        anchor: Anchor::Doc,
        target: chapter.clone(),
        quote: String::new(),
        text: String::new(),
    };
    assert!(notes::create(&root, &again).is_err());
    let nowhere = NewNote {
        id: None,
        anchor: Anchor::Card,
        target: "nope".into(),
        quote: String::new(),
        text: String::new(),
    };
    assert!(notes::create(&root, &nowhere).is_err());

    note.text = "이 대사 너무 설명조".into();
    note.tags = vec![" #퇴고".into(), "퇴고".into(), "".into()];
    note.replies = vec![
        Reply {
            at: "2026-09-27T02:00:00.000Z".into(),
            text: "12화에서 다시 보기".into(),
        },
        Reply {
            at: "2026-09-27T02:01:00.000Z".into(),
            text: "  ".into(),
        },
    ];
    // What it hangs on cannot be changed by saving.
    note.target = "elsewhere".into();
    let saved = notes::save(&root, &note).unwrap();
    assert_eq!(saved.tags, vec!["퇴고".to_string()]);
    assert_eq!(saved.replies.len(), 1);
    assert_eq!(saved.target, chapter);

    let card = cards::create(&root, "person", "윤서하").unwrap();
    let on_card = notes::create(
        &root,
        &NewNote {
            id: None,
            anchor: Anchor::Card,
            target: card.id.clone(),
            quote: String::new(),
            text: "과거사 아직 미정".into(),
        },
    )
    .unwrap();
    let on_project = notes::create(
        &root,
        &NewNote {
            id: None,
            anchor: Anchor::Project,
            target: "ignored".into(),
            quote: String::new(),
            text: "떠오른 장면".into(),
        },
    )
    .unwrap();
    assert_eq!(on_project.target, "");
    let all = notes::list(&root).unwrap();
    assert_eq!(all.len(), 3);
    assert!(
        all.iter()
            .any(|n| n.id == on_card.id && n.anchor == Anchor::Card)
    );

    // To the trash: the mark leaves the chapter; back again, the note stays
    // with the chapter and still shows its text.
    let item = trash::trash_note(&root, "memo1").unwrap();
    assert_eq!(item.section, Section::Notes);
    assert_eq!(item.title, "이 대사 너무 설명조");
    let file = fs::read_to_string(root.join("manuscript").join(format!("{chapter}.md"))).unwrap();
    assert!(file.contains("“영업, 끝났나요?” 남자가 물었다."));
    assert!(!file.contains("data-memo"));
    assert_eq!(notes::list(&root).unwrap().len(), 2);
    trash::restore(&root, &item.id).unwrap();
    let back = notes::load(&root, "memo1").unwrap();
    assert_eq!(back.quote, "“영업, 끝났나요?”");
    assert_eq!(notes::list(&root).unwrap().len(), 3);
}
