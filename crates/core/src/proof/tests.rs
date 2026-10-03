//! Certificates and their check, on the anchored sample of `anchor::tests`
//! (three chapters, two devices, stamps from DigiCert and FreeTSA).

use std::fs;
use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chrono::{FixedOffset, TimeZone, Utc};

use super::*;
use crate::anchor::tests::{DEV1, DEV2, DOCS, DRAFT, OTHER, TEXTS, anchored};
use crate::doc::{self, DocFile};
use crate::journal::{Entry, Paste, append_at, fingerprint};
use crate::markup::parse_body;

fn korea() -> FixedOffset {
    FixedOffset::east_opt(9 * 3600).unwrap()
}

fn made(root: &Path, options: &Options) -> Made {
    let now = Utc.with_ymd_and_hms(2026, 10, 3, 3, 0, 0).unwrap();
    make_at(root, options, korea(), now).unwrap()
}

fn bundle_of(made: &Made) -> Bundle {
    serde_json::from_str(&made.bundle).unwrap()
}

fn json(bundle: &Bundle) -> Vec<u8> {
    serde_json::to_vec(bundle).unwrap()
}

fn fails(v: &Verdict) -> Vec<&str> {
    v.findings
        .iter()
        .filter(|f| f.level == Level::Fail)
        .map(|f| f.text.as_str())
        .collect()
}

#[test]
fn the_summary_counts_what_the_journals_say() {
    let dir = tempfile::tempdir().unwrap();
    let root = anchored(dir.path());
    let scope = facts::Scope::new(&Options::default(), korea()).unwrap();
    let journals = journal::read_all(&root).unwrap();
    let f = facts::gather(&root, &scope, &journals).unwrap();
    assert_eq!(f.totals.sessions, 4);
    assert_eq!(f.totals.saves, 4);
    assert_eq!((f.totals.inserted, f.totals.deleted), (1150, 100));
    assert_eq!((f.totals.outside_pastes, f.totals.outside_chars), (1, 150));
    assert_eq!(f.totals.active_days, 4);
    assert_eq!(f.totals.devices, 2);
    assert_eq!(f.days.len(), 4);
    let sept = |d| chrono::NaiveDate::from_ymd_opt(2026, 9, d).unwrap();
    assert_eq!(f.days[&sept(2)].inserted, 800);
    let first = &f.rows[0];
    assert_eq!(
        (first.sessions, first.inserted, first.deleted),
        (2, 150, 50)
    );
    assert_eq!(first.first, Some(sept(1)));
    assert_eq!(first.last_edit, Some(sept(3)));
    assert_eq!(first.imported_from.as_deref(), Some("초고.hwpx"));
    assert_eq!(f.rows[2].outside_chars, 150);
    assert_eq!(f.imports[0].numbers, [1, 2]);
    assert_eq!(
        facts::summary(&f),
        "「달빛 서점의 마지막 손님」 1~3화는 2026년 9월 1일부터 2026년 9월 4일까지 4일 동안 \
         4번에 걸쳐 쓰고 고쳤습니다. 바깥에서 붙여 넣은 글은 전체의 13%이고, 1~2화는 \
         '초고.hwpx'에서 가져왔습니다. 이 증명서는 창작 과정의 기록이며 AI 사용 여부를 \
         판단하지 않습니다."
    );
}

#[test]
fn small_words_come_out_right() {
    assert_eq!(facts::ranges(&[5, 1, 2, 3, 7, 8, 9], "화"), "1~3, 5, 7~9화");
    assert_eq!(facts::ranges(&[4], "장"), "4장");
    assert_eq!(facts::topic("3화"), "는");
    assert_eq!(facts::topic("4장"), "은");
    assert_eq!(facts::topic("「Moon」"), "은(는)");
    assert_eq!(facts::num(1234567), "1,234,567");
    assert_eq!(facts::num(12), "12");
}

#[test]
fn the_certificate_page_holds_no_manuscript_text_unless_asked() {
    let dir = tempfile::tempdir().unwrap();
    let root = anchored(dir.path());
    let m = made(&root, &Options::default());
    assert!(m.ok, "{}", m.html);
    for text in [&m.html, &m.bundle] {
        for t in TEXTS {
            let first = t.lines().next().unwrap();
            assert!(!text.contains(first), "manuscript text leaked");
        }
        assert!(text.contains("초고.hwpx"));
    }
    assert!(m.html.contains(&html::esc(&m.summary)));
    assert!(m.html.contains(NOT_ABOUT_AI));
    assert!(m.html.contains("<svg"));
    assert!(m.html.contains("DigiCert"));
    assert!(m.html.contains("FreeTSA"));
    // Times of day show unless the writer keeps them to dates.
    assert!(m.html.contains("2026년 9월 1일 12:00"));
    let dated = made(
        &root,
        &Options {
            dates_only: true,
            ..Options::default()
        },
    );
    assert!(!dated.html.contains("12:00"));

    // Excerpts for a chapter with a draft record: what went and what came.
    let draft = DocFile {
        meta: doc::load(&root, DOCS[0]).unwrap().meta,
        body: parse_body(DRAFT),
    };
    crate::snapshot::create(&root, &draft, "auto", "").unwrap();
    let shown = made(
        &root,
        &Options {
            excerpts: vec![DOCS[0].into()],
            ..Options::default()
        },
    );
    assert!(shown.html.contains("고친 흐름"));
    assert!(shown.html.contains("<del>"));
    assert!(shown.html.contains("<ins>"));
    assert!(shown.html.contains("영업, "));
    // The bundle never carries text.
    assert!(!shown.bundle.contains("영업"));
}

#[test]
fn the_proof_file_checks_out_with_the_manuscript() {
    let dir = tempfile::tempdir().unwrap();
    let root = anchored(dir.path());
    let m = made(&root, &Options::default());
    let file = fs::read(root.join("manuscript").join(format!("{}.md", DOCS[0]))).unwrap();
    let v = verify(
        m.bundle.as_bytes(),
        &[
            ("1화.md".into(), file),
            ("초고.hwpx".into(), b"hwpx bytes".to_vec()),
            ("다른 글.md".into(), "다른 글".as_bytes().to_vec()),
        ],
    );
    assert!(v.ok, "{}", v.text());
    let oks: Vec<&str> = v
        .findings
        .iter()
        .filter(|f| f.level == Level::Ok)
        .map(|f| f.text.as_str())
        .collect();
    assert!(
        oks.iter().any(|t| t.starts_with("DigiCert가 2026-10-02")),
        "{oks:?}"
    );
    assert!(oks.iter().any(|t| t.starts_with("FreeTSA가")));
    assert!(
        oks.iter()
            .any(|t| t.contains(&format!("기기 {DEV2}의 창작 일지 1~2번째 항목은 늦어도")))
    );
    assert!(
        oks.iter()
            .any(|t| t.contains("1화.md: 증명서를 만들 때의 1번째 회차"))
    );
    assert!(oks.iter().any(|t| t.contains("시각 인증으로 확인됩니다")));
    assert!(
        oks.iter()
            .any(|t| t.contains("가져온 파일 '초고.hwpx' 그대로"))
    );
    let warns: Vec<&Finding> = v
        .findings
        .iter()
        .filter(|f| f.level == Level::Warn)
        .collect();
    assert_eq!(warns.len(), 1, "{warns:?}");
    assert!(warns[0].text.contains("다른 글.md"));
    assert_eq!(v.anchors.len(), 1);
    assert!(v.text().contains("결과: 이 증명 자료는 고쳐지지 않았고"));
}

#[test]
fn tampering_shows() {
    let dir = tempfile::tempdir().unwrap();
    let root = anchored(dir.path());
    let good = bundle_of(&made(&root, &Options::default()));
    assert!(verify(&json(&good), &[]).ok);

    // An edited journal line: the next one no longer links to it.
    let mut edited = good.clone();
    let dev1 = edited
        .journals
        .iter_mut()
        .find(|j| j.device == DEV1)
        .unwrap();
    let line = dev1
        .lines
        .iter_mut()
        .find_map(|l| match l {
            Line::Shown(t) if t.contains("\"inserted\":800") => Some(t),
            _ => None,
        })
        .unwrap();
    *line = line.replace("\"inserted\":800", "\"inserted\":80");
    let v = verify(&json(&edited), &[]);
    assert!(!v.ok);
    assert!(fails(&v)[0].contains(&format!(
        "기기 {DEV1}의 창작 일지 5번째 항목이 앞 항목과 이어지지 않습니다"
    )));

    // A line removed.
    let mut removed = good.clone();
    removed.journals[0].lines.remove(2);
    assert!(!verify(&json(&removed), &[]).ok);

    // Another reply put in place of DigiCert's.
    let mut swapped = good.clone();
    swapped.anchors[0].tokens[0].reply = STANDARD.encode(OTHER);
    let v = verify(&json(&swapped), &[]);
    assert!(
        fails(&v)
            .iter()
            .any(|t| t.contains("기록에 적힌 파일과 다름")),
        "{:?}",
        fails(&v)
    );
    // … with the record changed to match: it signed something else.
    swapped.anchors[0].record.tokens[0].sha256 = fingerprint(OTHER);
    let v = verify(&json(&swapped), &[]);
    assert!(
        fails(&v)
            .iter()
            .any(|t| t.contains("다른 지문에 대한 시각 인증"))
    );

    // A wrong root in the record.
    let mut wrong = good.clone();
    wrong.anchors[0].record.root = "00".repeat(32);
    let v = verify(&json(&wrong), &[]);
    assert!(
        fails(&v)
            .iter()
            .any(|t| t.contains("다시 계산한 지문이 기록과 다릅니다"))
    );

    // A leaf changed to point at a made-up journal line.
    let mut leaf = good.clone();
    let l = leaf.anchors[0]
        .record
        .leaves
        .iter_mut()
        .find(|l| l.device() == Some(DEV2))
        .unwrap();
    l.value = "11".repeat(32);
    let v = verify(&json(&leaf), &[]);
    assert!(!v.ok);
    assert!(fails(&v).iter().any(|t| t.contains("가리키는 기기")));

    // Not a proof file at all.
    assert!(!verify(b"{}", &[]).ok);
}

#[test]
fn a_clock_set_back_after_a_stamp_is_noted() {
    let dir = tempfile::tempdir().unwrap();
    let root = anchored(dir.path());
    // Written after the stamp (signed 2 October), dated 6 September.
    append_at(
        &root,
        DEV1,
        &Entry::Paste(Paste {
            doc: DOCS[0].into(),
            chars: 120,
            outside: false,
        }),
        "2026-09-06T03:00:00.000Z",
    )
    .unwrap();
    let m = made(&root, &Options::default());
    let v = verify(m.bundle.as_bytes(), &[]);
    assert!(v.ok);
    assert!(
        v.findings
            .iter()
            .any(|f| f.level == Level::Warn && f.text.contains("날짜를 앞당겨"))
    );
    assert!(m.html.contains("날짜를 앞당겨"));
}

#[test]
fn chosen_chapters_and_days_hide_the_rest() {
    let dir = tempfile::tempdir().unwrap();
    let root = anchored(dir.path());
    let options = Options {
        docs: Some(vec![DOCS[2].into()]),
        from: Some("2026-09-02".into()),
        to: Some("2026-09-02".into()),
        ..Options::default()
    };
    let m = made(&root, &options);
    assert_eq!(
        m.summary,
        "「달빛 서점의 마지막 손님」 3화는 2026년 9월 2일 하루 동안 1번에 걸쳐 쓰고 고쳤습니다. \
         바깥에서 붙여 넣은 글은 전체의 19%입니다. 이 증명서는 창작 과정의 기록이며 AI 사용 \
         여부를 판단하지 않습니다."
    );
    let b = bundle_of(&m);
    assert_eq!(b.chapters.len(), 1);
    for j in &b.journals {
        for line in &j.lines {
            if let Line::Shown(text) = line {
                let v: serde_json::Value = serde_json::from_str(text).unwrap();
                let about_doc3 = v["doc"] == DOCS[2] && text.contains("2026-09-02T");
                assert!(about_doc3 || v["kind"] == "anchor", "{text}");
            }
        }
    }
    // The other device's lines are past the period but under the stamp
    // that covers it: there as hashes only.
    let dev2 = b.journals.iter().find(|j| j.device == DEV2).unwrap();
    assert!(dev2.lines.iter().all(|l| matches!(l, Line::Hidden { .. })));
    assert_eq!(dev2.lines.len(), 2);
    assert_eq!(b.anchors.len(), 1);
    let v = verify(m.bundle.as_bytes(), &[]);
    assert!(v.ok, "{}", v.text());
    assert!(m.ok);

    assert!(
        make_at(
            &root,
            &Options {
                docs: Some(vec![]),
                ..Options::default()
            },
            korea(),
            Utc::now()
        )
        .is_err()
    );
    let backwards = Options {
        from: Some("2026-09-03".into()),
        to: Some("2026-09-01".into()),
        ..Options::default()
    };
    assert!(preview(&root, &backwards).is_err());
}

#[test]
fn certificates_go_into_a_new_folder_each_time() {
    let dir = tempfile::tempdir().unwrap();
    let root = anchored(dir.path());
    let out = dir.path().join("out");
    fs::create_dir_all(&out).unwrap();
    let first = write(&root, &Options::default(), &out).unwrap();
    let second = write(&root, &Options::default(), &out).unwrap();
    assert_ne!(first.folder, second.folder);
    assert!(second.folder.ends_with(" (2)"));
    assert!(Path::new(&first.html).ends_with(HTML_NAME));
    let bundle = fs::read(&first.bundle).unwrap();
    assert!(verify(&bundle, &[]).ok);
    assert!(first.ok);
}

#[test]
fn chapters_that_cannot_be_read_are_listed_as_left_out() {
    let dir = tempfile::tempdir().unwrap();
    let root = anchored(dir.path());
    let path = root.join("manuscript").join(doc::file_name(DOCS[1]));
    let src = format!("---\nid: \"{}\"\ntitle: \"둘째\"\n---\n\n본문\n", DOCS[1]);
    fs::write(&path, encoding_rs::EUC_KR.encode(&src).0).unwrap();

    let m = made(&root, &Options::default());
    assert!(m.html.contains("빠진 회차"));
    assert!(m.html.contains("2화 둘째 · 다른 글자 방식"), "{}", m.html);
    // Picking that chapter alone is not an error: it is listed as left out.
    let only = |id: &str| Options {
        docs: Some(vec![id.into()]),
        ..Options::default()
    };
    assert!(made(&root, &only(DOCS[1])).html.contains("빠진 회차"));
    // A chapter that is there in full is not listed.
    assert!(!made(&root, &only(DOCS[0])).html.contains("빠진 회차"));
}
