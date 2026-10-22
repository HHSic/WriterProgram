//! 작품 해석 tools: the chapter with paragraph numbers to quote from, saving a
//! reading (writer-core checks every quote against the text), and reading it
//! back. The app shows saved readings in 해석 모드 (docs/reading.md).

use std::path::Path;

use serde_json::{Value, json};
use writer_core::doc;
use writer_core::markup::Block;
use writer_core::reading::{self, Reading};

use crate::guide;
use crate::tools::{Outcome, err, required, tool};

pub fn list() -> Vec<Value> {
    vec![
        tool(
            "paragraphs",
            "작품 해석용: 회차 본문을 문단 번호([0], [1], …)와 함께 보인다. 장면 나눔도 번호를 차지한다. save_reading의 block은 이 번호다.",
            json!({"id": {"type": "string"}}),
            &["id"],
        ),
        tool(
            "save_reading",
            "작품 해석을 저장한다. guide(\"reading\") 절차를 따른다. 인용(quote)은 본문 그대로여야 하고, 없는 인용이나 없는 문장을 가리키는 연결이 있으면 저장하지 않는다. 같은 회차의 해석은 새로 덮어쓴다. 앱의 해석 모드에서 문장을 누르면 보인다.",
            json!({
                "id": {"type": "string", "description": "회차 id"},
                "summary": {"type": "string", "description": "회차가 밑바닥에서 묻는 것, 두세 문장"},
                "scenes": {"type": "array", "items": {"type": "object", "properties": {
                    "block": {"type": "integer"}, "title": {"type": "string"}, "purpose": {"type": "string"}},
                    "required": ["block", "title", "purpose"]}},
                "sentences": {"type": "array", "items": {"type": "object", "properties": {
                    "id": {"type": "string"},
                    "block": {"type": "integer"},
                    "quote": {"type": "string", "description": "본문 그대로의 한 문장이나 그 일부"},
                    "roles": {"type": "array", "items": {"type": "string"}, "description": "복선, 회수, 인물, 관계, 긴장, 정보, 분위기, 이미지, 리듬, 전환, 주제, 시점"},
                    "meaning": {"type": "string"},
                    "intent": {"type": "string"},
                    "links": {"type": "array", "items": {"type": "object", "properties": {
                        "to": {"type": "string"}, "kind": {"type": "string"}, "note": {"type": "string"}},
                        "required": ["to", "kind"]}},
                    "unclear": {"type": "string", "description": "의도가 흐려 보이면 그 이유. 작가가 판단한다"}},
                    "required": ["id", "block", "quote", "roles", "meaning", "intent"]}},
            }),
            &["id", "sentences"],
        ),
        tool(
            "reading",
            "저장된 작품 해석. 문장마다 지금 본문에서의 자리(places)와, 본문이 바뀌어 찾지 못한 문장(gone)을 함께 준다.",
            json!({"id": {"type": "string"}}),
            &["id"],
        ),
    ]
}

pub fn call(name: &str, root: &Path, args: &Value) -> Option<Outcome> {
    Some(match name {
        "paragraphs" => paragraphs(root, args),
        "save_reading" => save(root, args),
        "reading" => load(root, args),
        _ => return None,
    })
}

fn paragraphs(root: &Path, args: &Value) -> Outcome {
    let d = doc::load(root, required(args, "id")?).map_err(err)?;
    let texts = reading::block_texts(&d.body);
    let mut out = vec![format!(
        "[해석용 본문] {} · 문단 {}개",
        d.meta.title,
        texts.len()
    )];
    for (i, (block, text)) in d.body.iter().zip(&texts).enumerate() {
        match block {
            Block::SceneBreak {} => out.push(format!("[{i}] ***")),
            Block::Paragraph { .. } if text.trim().is_empty() => {}
            Block::Paragraph { .. } => out.push(format!("[{i}] {}", text.replace('\n', " / "))),
        }
    }
    Ok(Value::String(out.join("\n")))
}

fn save(root: &Path, args: &Value) -> Outcome {
    guide::check_read()?;
    let id = required(args, "id")?;
    let reading = Reading {
        doc: id.to_string(),
        rev: String::new(),
        made: String::new(),
        by: "Claude".into(),
        summary: args
            .get("summary")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        scenes: serde_json::from_value(args.get("scenes").cloned().unwrap_or(json!([])))
            .map_err(|e| format!("scenes를 읽지 못함: {e}"))?,
        sentences: serde_json::from_value(args.get("sentences").cloned().unwrap_or(json!([])))
            .map_err(|e| format!("sentences를 읽지 못함: {e}"))?,
    };
    let saved = reading::save(root, reading).map_err(err)?;
    let linked = saved
        .sentences
        .iter()
        .filter(|s| !s.links.is_empty())
        .count();
    let unclear = saved
        .sentences
        .iter()
        .filter(|s| s.unclear.is_some())
        .count();
    Ok(json!({
        "id": id,
        "scenes": saved.scenes.len(),
        "sentences": saved.sentences.len(),
        "linked": linked,
        "unclear": unclear,
        "note": "앱의 해석 모드에서 문장을 누르면 보인다.",
    }))
}

fn load(root: &Path, args: &Value) -> Outcome {
    match reading::load(root, required(args, "id")?).map_err(err)? {
        Some(view) => serde_json::to_value(view).map_err(|e| e.to_string()),
        None => Ok(json!({"reading": null, "note": "이 회차는 아직 해석하지 않았다"})),
    }
}

#[cfg(test)]
mod tests {
    use crate::tools::call;
    use crate::tools::tests::with_context;
    use serde_json::json;
    use std::path::PathBuf;

    #[test]
    fn a_reading_is_saved_against_the_text_and_read_back() {
        let dir = tempfile::tempdir().unwrap();
        let made = call(
            "create_project",
            &json!({"parent": dir.path().to_str().unwrap(), "title": "해석"}),
            None,
        )
        .unwrap();
        let root = PathBuf::from(made["project"].as_str().unwrap());
        let added = call(
            "add_doc",
            &with_context("add_doc", json!({"title": "1", "text": "손을 뻗으려다 말았다.\n***\n손이 머뭇거리지 않았다."})),
            Some(&root),
        )
        .unwrap();
        let id = added["id"].as_str().unwrap();

        let numbered = call("paragraphs", &json!({"id": id}), Some(&root)).unwrap();
        assert!(
            numbered
                .as_str()
                .unwrap()
                .contains("[1] ***\n[2] 손이 머뭇거리지 않았다."),
            "{numbered}"
        );

        let sentences = json!([
            {"id": "a", "block": 0, "quote": "손을 뻗으려다 말았다.", "roles": ["복선", "인물"], "meaning": "망설임", "intent": "변화의 출발점",
             "links": [{"to": "b", "kind": "회수"}]},
            {"id": "b", "block": 2, "quote": "머뭇거리지 않았다", "roles": ["회수"], "meaning": "", "intent": ""},
        ]);
        let saved = call(
            "save_reading",
            &json!({"id": id, "summary": "망설임에서 행동으로", "sentences": sentences}),
            Some(&root),
        )
        .unwrap();
        assert_eq!(
            (saved["sentences"].as_u64(), saved["linked"].as_u64()),
            (Some(2), Some(1))
        );

        let back = call("reading", &json!({"id": id}), Some(&root)).unwrap();
        assert_eq!(back["reading"]["summary"], "망설임에서 행동으로");
        assert_eq!(back["places"][1]["start"], 3);

        let e = call(
            "save_reading",
            &json!({"id": id, "sentences": [{"id": "x", "block": 0, "quote": "지어낸 문장.", "roles": [], "meaning": "", "intent": ""}]}),
            Some(&root),
        )
        .unwrap_err();
        assert!(e.contains("본문에 없는 인용"), "{e}");
    }
}
