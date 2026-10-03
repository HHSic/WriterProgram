//! Anchoring with real replies recorded once from DigiCert and FreeTSA
//! (`tests/fixtures/anchor/`) for the root of `sample`'s journals and
//! chapters, so signatures are checked as they are in the field.
//!
//! The sample is built the same way every time (fixed ids, texts and
//! times), so its root does not change. If the journal's line format or
//! the Merkle hashing ever changes, `the_sample_root_is_the_one_stamped`
//! fails: new replies must then be recorded for the new root. They were made
//! with the request `tsp::request(root, 7)` writes (hex
//! `30430201013031300d06096086480165030402010500 0420 <root>
//! 0208 0100000000000007 0101ff`, spaces removed, through `xxd -r -p`) and
//! `curl -H "Content-Type: application/timestamp-query" --data-binary
//! @sample.tsq <TSA url>`. `other.tsr` is Sectigo's reply for another hash.

use std::fs;
use std::path::{Path, PathBuf};

use der::Decode;

use super::*;
use crate::doc::{DocFile, DocMeta, write_doc_file};
use crate::journal::{Entry, Import, Paste, Save, Session, append_at};
use crate::markup::parse_body;

pub(crate) const DEV1: &str = "dev1aaaaaaaa";
pub(crate) const DEV2: &str = "dev2bbbbbbbb";
pub(crate) const DOCS: [&str; 3] = ["doc000000001", "doc000000002", "doc000000003"];
/// The root of `sample`, which the recorded replies signed.
pub(crate) const SAMPLE_ROOT: &str =
    "f755185f142f6d62b5576533cb2954dea216f1a9ab4727c72f57976f52cb51cf";
pub(crate) const DIGICERT: &[u8] = include_bytes!("../../tests/fixtures/anchor/digicert.tsr");
pub(crate) const FREETSA: &[u8] = include_bytes!("../../tests/fixtures/anchor/freetsa.tsr");
/// A reply for something else (the SHA-256 of "hello\n").
pub(crate) const OTHER: &[u8] = include_bytes!("../../tests/fixtures/anchor/other.tsr");
/// When the sample's anchoring was asked for.
pub(crate) const ASKED: &str = "2026-09-05T03:00:00.000Z";

pub(crate) const TEXTS: [&str; 3] = [
    "셔터를 반쯤 내렸을 때 종이 울렸다.\n\n“영업, 끝났나요?”",
    "남자가 봉투에서 꺼낸 것은 책이 아니라 대여 카드였다.",
    "비에 젖은 손님은 끝내 이름을 말하지 않았다.\n\n***\n\n다음 날 아침, 카드는 계산대 위에 놓여 있었다.",
];
pub(crate) const DRAFT: &str = "셔터를 내렸을 때 종이 울렸다.\n\n“끝났어요?”";

pub(crate) fn body_print(text: &str) -> String {
    fingerprint(write_body(&parse_body(text)).as_bytes())
}

fn write_doc(root: &Path, id: &str, title: &str, text: &str) {
    let mut meta = DocMeta::new(id, title);
    meta.created = "2026-09-01T03:00:00.000Z".into();
    let file = DocFile {
        meta,
        body: parse_body(text),
    };
    write_doc_file(&root.join("manuscript").join(format!("{id}.md")), &file).unwrap();
}

/// A three-chapter work written on two devices from 1 to 4 September 2026
/// (all at noon in Korea), not yet anchored.
pub(crate) fn sample(parent: &Path) -> PathBuf {
    let root = parent.join("달빛 서점");
    fs::create_dir_all(root.join("manuscript")).unwrap();
    let project = serde_json::json!({
        "app": "WriterProgram",
        "format": 1,
        "id": "proj00000001",
        "title": "달빛 서점의 마지막 손님",
        "kind": "webnovel",
        "penName": "",
        "created": "2026-09-01T03:00:00.000Z",
        "goal": { "perDoc": null, "countSpaces": true, "daily": null },
        "parts": [{ "id": "part00000001", "title": "1부", "docs": DOCS }],
        "planning": [],
    });
    fs::write(root.join("project.json"), project.to_string()).unwrap();
    for (i, (id, text)) in DOCS.iter().zip(TEXTS).enumerate() {
        write_doc(&root, id, &format!("{}화 제목", i + 1), text);
    }
    let save = |doc: &str, text: &str, added| {
        Entry::Save(Save {
            doc: doc.into(),
            body: body_print(text),
            chars: text.chars().count() as u32,
            added,
            removed: 0,
            saves: None,
            since: None,
        })
    };
    let session = |doc: &str, start: &str, end: &str, inserted, deleted| {
        Entry::Session(Session {
            doc: doc.into(),
            start: start.into(),
            end: end.into(),
            inserted,
            deleted,
        })
    };
    let lines = [
        (
            DEV1,
            "2026-09-01T03:00:00.000Z",
            Entry::Import(Import {
                file: "초고.hwpx".into(),
                file_hash: fingerprint(b"hwpx bytes"),
                docs: vec![DOCS[0].into(), DOCS[1].into()],
            }),
        ),
        (
            DEV1,
            "2026-09-01T03:20:00.000Z",
            session(
                DOCS[0],
                "2026-09-01T03:05:00.000Z",
                "2026-09-01T03:20:00.000Z",
                120,
                10,
            ),
        ),
        (DEV1, "2026-09-01T03:20:01.000Z", save(DOCS[0], DRAFT, 110)),
        (
            DEV1,
            "2026-09-02T03:30:00.000Z",
            session(
                DOCS[2],
                "2026-09-02T03:00:00.000Z",
                "2026-09-02T03:30:00.000Z",
                800,
                50,
            ),
        ),
        (
            DEV1,
            "2026-09-02T03:31:00.000Z",
            Entry::Paste(Paste {
                doc: DOCS[2].into(),
                chars: 150,
                outside: true,
            }),
        ),
        (
            DEV1,
            "2026-09-02T03:32:00.000Z",
            save(DOCS[2], TEXTS[2], 750),
        ),
        (
            DEV1,
            "2026-09-03T03:10:00.000Z",
            session(
                DOCS[0],
                "2026-09-03T03:00:00.000Z",
                "2026-09-03T03:10:00.000Z",
                30,
                40,
            ),
        ),
        (
            DEV1,
            "2026-09-03T03:10:01.000Z",
            save(DOCS[0], TEXTS[0], 10),
        ),
        (
            DEV2,
            "2026-09-04T03:15:00.000Z",
            session(
                DOCS[1],
                "2026-09-04T03:00:00.000Z",
                "2026-09-04T03:15:00.000Z",
                200,
                0,
            ),
        ),
        (
            DEV2,
            "2026-09-04T03:15:01.000Z",
            save(DOCS[1], TEXTS[1], 20),
        ),
    ];
    for (device, time, entry) in lines {
        append_at(&root, device, &entry, time).unwrap();
    }
    root
}

pub(crate) fn pending(root: &Path) -> Pending {
    let leaves = leaves(root).unwrap();
    Pending {
        device: DEV1.into(),
        time: ASKED.into(),
        root: merkle_root(&leaves),
        leaves,
        nonce: 7,
    }
}

/// `sample`, anchored with both recorded replies (Sectigo unreachable).
pub(crate) fn anchored(parent: &Path) -> PathBuf {
    let root = sample(parent);
    let p = pending(&root);
    finish(
        &root,
        &p,
        vec![
            (TSAS[0], Ok(DIGICERT.to_vec())),
            (TSAS[1], Err("연결 안 됨".into())),
            (TSAS[2], Ok(FREETSA.to_vec())),
        ],
    )
    .unwrap();
    root
}

#[test]
fn the_sample_root_is_the_one_stamped() {
    let dir = tempfile::tempdir().unwrap();
    let root = sample(dir.path());
    let p = pending(&root);
    assert_eq!(hex(&p.root), SAMPLE_ROOT, "record new fixture replies");
    // Two journals and three chapters, sorted by label.
    let labels: Vec<&str> = p.leaves.iter().map(|l| l.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "doc:doc000000001",
            "doc:doc000000002",
            "doc:doc000000003",
            "journal:dev1aaaaaaaa",
            "journal:dev2bbbbbbbb",
        ]
    );
    assert_eq!(p.leaves[0].value, body_print(TEXTS[0]));
}

#[test]
fn requests_ask_for_a_sha256_imprint_with_nonce_and_certificate() {
    let imprint = [0xab; 32];
    let der = tsp::request(&imprint, 0x1234);
    let req = x509_tsp::TimeStampReq::from_der(&der).unwrap();
    assert_eq!(req.message_imprint.hashed_message.as_bytes(), &imprint);
    assert_eq!(
        req.message_imprint.hash_algorithm.oid.to_string(),
        "2.16.840.1.101.3.4.2.1"
    );
    assert!(req.cert_req);
    assert_eq!(
        req.nonce.unwrap().as_bytes(),
        [0x01, 0, 0, 0, 0, 0, 0x12, 0x34]
    );
    // Byte for byte: SEQUENCE { 1, { {sha256, NULL}, OCTET STRING }, nonce, TRUE }.
    let mut expected = vec![
        0x30, 0x43, 0x02, 0x01, 0x01, 0x30, 0x31, 0x30, 0x0d, 0x06, 0x09,
    ];
    expected.extend([
        0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01, 0x05, 0x00,
    ]);
    expected.extend([0x04, 0x20]);
    expected.extend(imprint);
    expected.extend([
        0x02, 0x08, 0x01, 0, 0, 0, 0, 0, 0x12, 0x34, 0x01, 0x01, 0xff,
    ]);
    assert_eq!(der, expected);
    // The top bit never makes the nonce negative.
    assert_eq!(tsp::nonce_bytes(u64::MAX)[0], 0x7f);
}

#[test]
fn recorded_replies_check_out() {
    for (reply, signer, chain) in [(DIGICERT, "DigiCert", 3), (FREETSA, "freetsa", 2)] {
        let stamp = tsp::check(reply).unwrap();
        assert_eq!(hex(&stamp.imprint), SAMPLE_ROOT);
        assert!(stamp.sha256);
        assert!(
            stamp.signer.to_lowercase().contains(&signer.to_lowercase()),
            "{}",
            stamp.signer
        );
        assert_eq!(stamp.chain, chain, "{}", stamp.top);
    }
    let other = tsp::check(OTHER).unwrap();
    assert_ne!(hex(&other.imprint), SAMPLE_ROOT);
}

#[test]
fn a_changed_reply_does_not_check_out() {
    // One byte of the signed time changed.
    let at = DIGICERT
        .windows(4)
        .position(|w| w == b"2026")
        .expect("a time in the token");
    let mut changed = DIGICERT.to_vec();
    changed[at + 3] = b'5';
    assert!(tsp::check(&changed).is_err());
    // Not a reply at all.
    assert!(tsp::check(b"<html>busy</html>").is_err());
    assert!(tsp::check(&DIGICERT[..DIGICERT.len() - 10]).is_err());
}

#[test]
fn finishing_keeps_good_replies_and_journals_them() {
    let dir = tempfile::tempdir().unwrap();
    let root = sample(dir.path());
    let p = pending(&root);
    let outcome = finish(
        &root,
        &p,
        vec![
            (TSAS[0], Ok(DIGICERT.to_vec())),
            (TSAS[1], Err("연결 안 됨".into())),
            (TSAS[2], Ok(OTHER.to_vec())),
        ],
    )
    .unwrap();
    assert_eq!(outcome.signed, ["DigiCert"]);
    assert_eq!(outcome.failed.len(), 2);
    assert_eq!(outcome.failed[1].1, "다른 지문에 서명함");
    let name = outcome.record.unwrap();
    assert_eq!(name, format!("20260905-030000-000-{DEV1}.json"));

    let records = records(&root).unwrap();
    assert_eq!(records.len(), 1);
    let record = &records[0];
    // The root is recomputed from the leaves kept beside the token.
    assert_eq!(hex(&merkle_root(&record.leaves)), record.root);
    assert_eq!(record.root, SAMPLE_ROOT);
    let token = &record.tokens[0];
    assert_eq!(
        token.file,
        format!("20260905-030000-000-{DEV1}-digicert.tsr")
    );
    assert_eq!(token_bytes(&root, &token.file).unwrap(), DIGICERT);
    assert_eq!(token.sha256, fingerprint(DIGICERT));

    // One anchor line, and the chain still holds.
    let journals = crate::journal::read_all(&root).unwrap();
    let dev1 = &journals.iter().find(|(d, _)| d == DEV1).unwrap().1;
    let last = std::str::from_utf8(dev1.last().unwrap()).unwrap();
    assert!(last.contains("\"kind\":\"anchor\""));
    assert!(last.contains(&format!("\"root\":\"{SAMPLE_ROOT}\"")));
    assert!(last.contains(&format!("\"token\":\"{}\"", fingerprint(DIGICERT))));
    assert!(crate::journal::verify(&root).unwrap().ok);
    let summary = crate::journal::summary(&root, Some(DEV1)).unwrap();
    assert_eq!(summary.anchors, 1);
    assert!(summary.last_anchor.is_some());

    // Anchor lines alone change nothing: no new stamp is needed.
    assert_eq!(
        prepare(&root, DEV1, true).unwrap().unwrap_err(),
        Skip::Unchanged
    );
    assert!(token_bytes(&root, "../project.json").is_err());
}

#[test]
fn nothing_is_sent_without_a_journal_or_twice_a_day() {
    let dir = tempfile::tempdir().unwrap();
    let root = sample(dir.path());
    fs::remove_dir_all(root.join(".journal")).unwrap();
    assert_eq!(
        prepare(&root, DEV1, false).unwrap().unwrap_err(),
        Skip::NoJournal
    );

    let root = anchored(&dir.path().join("b"));
    // A new save after the stamp: due again, but not on the same day.
    append_at(
        &root,
        DEV1,
        &Entry::Paste(Paste {
            doc: DOCS[0].into(),
            chars: 100,
            outside: false,
        }),
        "2026-09-05T04:00:00.000Z",
    )
    .unwrap();
    let asked: DateTime<Utc> = parse_iso(ASKED).unwrap();
    let later = asked + chrono::Duration::hours(1);
    assert_eq!(
        prepare_at(&root, DEV1, false, later).unwrap().unwrap_err(),
        Skip::DoneToday
    );
    assert!(prepare_at(&root, DEV1, true, later).unwrap().is_ok());
    // Another device has not stamped today.
    assert!(prepare_at(&root, DEV2, false, later).unwrap().is_ok());
    let next_day = asked + chrono::Duration::days(1);
    let p = prepare_at(&root, DEV1, false, next_day).unwrap().unwrap();
    assert_ne!(hex(&p.root), SAMPLE_ROOT);
    assert_eq!(p.request().len(), 69);
}
