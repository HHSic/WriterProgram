//! Asking each company through a local stand-in: what goes out (headers,
//! masked text, no real names), and how answers and errors come back.
//! No real API is called.

mod support;

use std::path::{Path, PathBuf};

use serde_json::json;
use support::serve;
use writer_ai::client::Client;
use writer_ai::tasks::{self, Task};
use writer_ai::{Error, Provider};
use writer_core::cards;
use writer_core::doc;
use writer_core::markup::Block;
use writer_core::project::{self, NewProject, ProjectKind};

/// A project with one chapter and two cards.
fn sample(dir: &Path) -> (PathBuf, String) {
    let root = project::create(&NewProject {
        parent: dir.to_string_lossy().into_owned(),
        title: "달빛 서점".into(),
        kind: ProjectKind::Webnovel,
        per_doc_goal: None,
        count_spaces: true,
        first_chapter: true,
    })
    .unwrap();
    let id = project::load(&root).unwrap().parts[0].docs[0].clone();
    let (_, path) = doc::locate(&root, &id).unwrap();
    let mut file = doc::read_doc(&path).unwrap();
    file.body = vec![
        Block::text("서하는 회색 눈을 비볐다."),
        Block::text("“윤 사장님, 문 닫았어요?” 손님이 달빛 서점 문턱에서 물었다."),
    ];
    doc::write_doc_file(&path, &file).unwrap();

    let mut person = cards::create(&root, "person", "윤서하").unwrap();
    person.aliases = vec!["서하".into(), "윤 사장".into()];
    person.fields = vec![
        ("나이".into(), "29".into()),
        ("외모".into(), "검은 눈".into()),
    ];
    person.description = "강윤석의 오랜 친구.".into();
    cards::save(&root, &person).unwrap();
    cards::create(&root, "person", "강윤석").unwrap();
    let mut place = cards::create(&root, "place", "달빛 서점").unwrap();
    place.fields = vec![("위치".into(), "골목 끝".into())];
    cards::save(&root, &place).unwrap();
    cards::create(&root, "term", "대여 카드").unwrap();
    (root, id)
}

#[test]
fn check_goes_out_masked_and_comes_back_named() {
    let dir = tempfile::tempdir().unwrap();
    let (root, id) = sample(dir.path());
    let stand_in = serve(Box::new(|_| {
        (
            200,
            json!({
                "content": [{ "type": "text", "text": "{\"problems\":[{\"quote\":\"인물A2는 회색 눈을\",\"card\":\"인물A\",\"problem\":\"인물A은 눈이 검은색인데 회색으로 나옴\"}]}" }],
                "stop_reason": "end_turn",
                "usage": { "input_tokens": 812, "output_tokens": 64 }
            }),
        )
    }));

    let out = tasks::prepare(&root, Task::Check, &id).unwrap();
    let preview = out.preview();
    assert_eq!(preview.cards, vec!["윤서하", "달빛 서점"]);
    assert!(preview.text.contains("[설정 카드]"));
    assert!(preview.text.contains("인물A2는 회색 눈을"));
    assert!(preview.text.contains("인물A3님"));
    assert!(preview.text.contains("장소B 문턱"));
    for name in ["서하", "윤 사장", "달빛 서점"] {
        assert!(!preview.text.contains(name), "{name} was sent");
    }
    // A card not in the chapter is not sent, but its name is masked where
    // a sent card mentions it.
    assert!(!preview.text.contains("대여"));
    assert!(preview.text.contains("설명: 인물C의 오랜 친구."));
    assert!(!preview.text.contains("강윤석"));
    assert!(!preview.text.contains("### 인물C"));

    let client = Client::new(Provider::Anthropic, "sk-ant-test", stand_in.base.clone());
    let answer = client.ask("claude-sonnet-5", &out.ask).unwrap();
    let check = tasks::read_check(&out, answer).unwrap();

    let req = stand_in.last();
    assert_eq!(req.method, "POST");
    assert_eq!(req.path, "/v1/messages");
    assert_eq!(req.header("x-api-key"), Some("sk-ant-test"));
    assert_eq!(req.header("anthropic-version"), Some("2023-06-01"));
    let body = req.json();
    assert_eq!(body["model"], "claude-sonnet-5");
    assert_eq!(body["messages"][0]["content"], preview.text);
    assert!(!req.text().contains("서하"));

    assert_eq!(check.findings.len(), 1);
    let f = &check.findings[0];
    assert_eq!(f.quote, "서하는 회색 눈을");
    assert_eq!(f.card, "윤서하");
    assert_eq!(f.problem, "윤서하는 눈이 검은색인데 회색으로 나옴");
    let place = f.place.as_ref().unwrap();
    assert_eq!((place.block, place.start), (0, 0));
    assert_eq!(check.usage.unwrap().input_tokens, Some(812));
}

#[test]
fn summary_through_openai_and_gemini() {
    let dir = tempfile::tempdir().unwrap();
    let (root, id) = sample(dir.path());
    let out = tasks::prepare(&root, Task::Summary, &id).unwrap();
    assert!(out.preview().text.starts_with("[회차 본문]\n인물A2는"));

    let openai = serve(Box::new(|_| {
        (
            200,
            json!({
                "choices": [{ "message": { "content": "인물A가 손님을 맞는다.", "refusal": null }, "finish_reason": "stop" }],
                "usage": { "prompt_tokens": 300, "completion_tokens": 20 }
            }),
        )
    }));
    let client = Client::new(Provider::Openai, "sk-test", openai.base.clone());
    let summary = tasks::read_summary(&out, client.ask("gpt-5-nano", &out.ask).unwrap()).unwrap();
    assert_eq!(summary.text, "윤서하가 손님을 맞는다.");
    let req = openai.last();
    assert_eq!(req.path, "/v1/chat/completions");
    assert_eq!(req.header("authorization"), Some("Bearer sk-test"));
    assert_eq!(req.json()["messages"][1]["content"], out.ask.user);

    let gemini = serve(Box::new(|_| {
        (
            200,
            json!({
                "candidates": [{ "content": { "parts": [{ "text": "장소B에서 인물A가 손님을 맞는다." }] }, "finishReason": "STOP" }],
                "usageMetadata": { "promptTokenCount": 280, "candidatesTokenCount": 18 }
            }),
        )
    }));
    let client = Client::new(Provider::Gemini, "AIza-test", gemini.base.clone());
    let summary =
        tasks::read_summary(&out, client.ask("gemini-2.5-flash-lite", &out.ask).unwrap()).unwrap();
    assert_eq!(summary.text, "달빛 서점에서 윤서하가 손님을 맞는다.");
    let req = gemini.last();
    assert_eq!(
        req.path,
        "/v1beta/models/gemini-2.5-flash-lite:generateContent"
    );
    assert_eq!(req.header("x-goog-api-key"), Some("AIza-test"));
}

#[test]
fn connection_check_tells_key_from_model() {
    let stand_in = serve(Box::new(|req| {
        if req.header("x-api-key") != Some("good") {
            return (
                401,
                json!({ "type": "error", "error": { "type": "authentication_error", "message": "invalid x-api-key" } }),
            );
        }
        if req.path.ends_with("/claude-haiku-4-5") {
            (200, json!({ "id": "claude-haiku-4-5", "type": "model" }))
        } else {
            (
                404,
                json!({ "type": "error", "error": { "type": "not_found_error", "message": "model: nope" } }),
            )
        }
    }));
    let good = Client::new(Provider::Anthropic, "good", stand_in.base.clone());
    good.check_model("claude-haiku-4-5").unwrap();
    assert_eq!(stand_in.last().method, "GET");
    assert_eq!(stand_in.last().path, "/v1/models/claude-haiku-4-5");
    assert!(matches!(
        good.check_model("nope"),
        Err(Error::ModelNotFound(m)) if m == "nope"
    ));
    let bad = Client::new(Provider::Anthropic, "bad", stand_in.base.clone());
    let err = bad.check_model("claude-haiku-4-5").unwrap_err();
    assert!(matches!(err, Error::KeyRejected));
    assert!(err.user_message().contains("API 키가 맞지 않음"));
}

#[test]
fn quota_and_offline() {
    let stand_in = serve(Box::new(|_| {
        (
            429,
            json!({ "error": { "message": "You exceeded your current quota", "type": "insufficient_quota", "code": "insufficient_quota" } }),
        )
    }));
    let client = Client::new(Provider::Openai, "sk", stand_in.base.clone());
    let ask = writer_ai::client::Ask {
        system: "s".into(),
        user: "u".into(),
        max_tokens: 10,
        json: false,
    };
    assert!(matches!(client.ask("gpt-5-nano", &ask), Err(Error::Quota)));

    // Nothing listens on a port that was just freed.
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let offline = Client::new(Provider::Openai, "sk", format!("http://127.0.0.1:{port}"));
    let err = offline.ask("gpt-5-nano", &ask).unwrap_err();
    assert!(matches!(err, Error::Offline(_)), "{err:?}");
    assert_eq!(
        err.user_message(),
        "AI 회사에 닿지 않음. 인터넷 연결을 확인해 주세요."
    );
}

#[test]
fn chapters_without_cards_have_nothing_to_check() {
    let dir = tempfile::tempdir().unwrap();
    let (root, id) = sample(dir.path());
    let (_, path) = doc::locate(&root, &id).unwrap();
    let mut file = doc::read_doc(&path).unwrap();
    file.body = vec![Block::text("아무도 오지 않았다.")];
    doc::write_doc_file(&path, &file).unwrap();
    let err = tasks::prepare(&root, Task::Check, &id).err().unwrap();
    assert!(err.user_message().contains("설정 카드가 없어"));
    // A summary still works, with nothing to mask.
    let out = tasks::prepare(&root, Task::Summary, &id).unwrap();
    assert!(out.preview().swaps.is_empty());
}
