//! 작품 해석 tools: the chapter with paragraph numbers to quote from, saving a
//! reading (writer-core checks every quote against the text), and reading it
//! back. The app shows saved readings in 해석 모드 (docs/reading.md).

use std::path::Path;

use serde_json::{Value, json};
use writer_core::doc;
use writer_core::markup::Block;
use writer_core::reading::{self, Reading};

use crate::check::text::{self as prose, sentences};
use crate::guide;
use crate::tools::{Outcome, err, required, tool};

pub fn list() -> Vec<Value> {
    vec![
        tool(
            "paragraphs",
            "작품 해석용: 회차 본문을 문단 번호([0], [1], …)와 함께 보인다. 장면 나눔도 번호를 차지한다. save_reading의 block은 이 번호다. 끝에 리듬 읽기용으로 60자 이상 지문 문장과 짧은 지문 문장이 이어지는 곳을 센 것을 붙인다.",
            json!({"id": {"type": "string"}}),
            &["id"],
        ),
        tool(
            "save_reading",
            "작품 해석을 저장한다. guide(\"reading\") 절차를 따른다. 인용(quote)은 본문 그대로여야 하고, 없는 인용이나 없는 문장을 가리키는 연결이 있으면 저장하지 않는다. 같은 회차의 해석은 새로 덮어쓴다. 다른 회차의 문장과 이으려면 link에 doc과 quote를 준다(그 회차 본문으로 검증). 앱의 해석 모드에서 문장을 누르면 보인다.",
            json!({
                "id": {"type": "string", "description": "회차 id"},
                "summary": {"type": "string", "description": "회차가 밑바닥에서 묻는 것, 두세 문장"},
                "rhythm": {"type": "string", "description": "리듬 읽기: 회차의 호흡을 두세 줄로. 어디서 길게 잇고 어디서 짧게 끊는지, 그것이 장면에 맞는지"},
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
                        "to": {"type": "string", "description": "같은 회차의 다른 문장 id"},
                        "kind": {"type": "string", "description": "회수, 복선, 같은 이미지, 대비, 변화 등"},
                        "note": {"type": "string"},
                        "doc": {"type": "string", "description": "다른 회차로 이을 때 그 회차 id. 그 회차가 아직 해석되지 않았어도 된다"},
                        "quote": {"type": "string", "description": "다른 회차로 이을 때 그 회차 본문 그대로의 문장"}},
                        "required": ["kind"]}},
                    "unclear": {"type": "string", "description": "의도가 흐려 보이면 그 이유. 작가가 판단한다"},
                    "rhythm": {"type": "object", "description": "리듬 읽기: 이 문장의 길이가 장면에서 하는 일", "properties": {
                        "kind": {"type": "string", "enum": ["이어지는 동작", "멈추지 못하는 생각", "쌓이는 감각", "흘러가는 시간", "숨을 모으는 대목", "정보를 한꺼번에 넘김", "이유 없이 김", "충격", "결정", "깨달음", "침묵"]},
                        "note": {"type": "string", "description": "길이가 여기서 하는 일, 한 줄"},
                        "advice": {"type": "string", "description": "끊거나 잇는 편이 나을 때만: 어디서 끊을지(또는 무엇과 이을지)와 고친 문장"}},
                        "required": ["kind", "note"]}},
                    "required": ["id", "block", "quote", "roles", "meaning", "intent"]}},
                "overwrite": {"type": "boolean", "description": "작가가 앱에서 고친 해석을 덮어쓸 때만 true. 먼저 reading으로 보고 작가에게 묻는다"},
            }),
            &["id", "sentences"],
        ),
        tool(
            "reading",
            "저장된 작품 해석. 문장마다 지금 본문에서의 자리(places), 본문이 바뀌어 찾지 못한 문장(gone), 다른 회차로 가는 연결(remote)과 다른 회차에서 이 회차로 오는 연결(incoming)을 함께 준다.",
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
    out.push(String::new());
    out.extend(rhythm_lines(&d.body, &texts));
    Ok(Value::String(out.join("\n")))
}

/// A narration sentence of this many characters or more is long, of
/// [`SHORT`] or fewer short (the kfiction guide's 문체 카드 and check F13).
const LONG: usize = 60;
const SHORT: usize = 20;
/// Short sentences in a row that make a stretch worth reading.
const SHORT_RUN: usize = 4;

/// For 리듬 읽기: the chapter's narration sentences counted, the long ones,
/// and the stretches of short ones, by paragraph number.
fn rhythm_lines(body: &[Block], texts: &[String]) -> Vec<String> {
    let mut all: Vec<(usize, String)> = Vec::new();
    for (i, (block, text)) in body.iter().zip(texts).enumerate() {
        if !matches!(block, Block::Paragraph { .. }) {
            continue;
        }
        for p in prose::paragraphs(text).into_iter().filter(|p| !p.excluded) {
            all.extend(sentences(&p.narration).into_iter().map(|s| (i, s)));
        }
    }
    let len = |s: &str| s.chars().count();
    if all.is_empty() {
        return vec!["[리듬] 지문 문장이 없음".into()];
    }
    let total: usize = all.iter().map(|(_, s)| len(s)).sum();
    let long: Vec<&(usize, String)> = all.iter().filter(|(_, s)| len(s) >= LONG).collect();
    let mut out = vec![format!(
        "[리듬] 지문 {}문장 · 평균 {}자 · {LONG}자 이상 문장 {}개",
        all.len(),
        (total as f64 / all.len() as f64).round(),
        long.len()
    )];
    for (block, s) in &long {
        out.push(format!("- [{block}] ({}자) {s}", len(s)));
    }
    let mut runs: Vec<&[(usize, String)]> = Vec::new();
    let mut start = 0;
    for i in 0..=all.len() {
        if i == all.len() || len(&all[i].1) > SHORT {
            if i - start >= SHORT_RUN {
                runs.push(&all[start..i]);
            }
            start = i + 1;
        }
    }
    if !runs.is_empty() {
        out.push(format!(
            "{SHORT}자 이하 지문 문장이 {SHORT_RUN}개 이상 이어지는 곳:"
        ));
        for run in runs {
            let quoted: Vec<&str> = run.iter().map(|(_, s)| s.as_str()).collect();
            out.push(format!(
                "- [{}] {}문장: {}",
                run[0].0,
                run.len(),
                quoted.join(" ")
            ));
        }
    }
    out
}

fn save(root: &Path, args: &Value) -> Outcome {
    guide::check_read()?;
    let id = required(args, "id")?;
    let overwrite = args.get("overwrite").and_then(Value::as_bool) == Some(true);
    if !overwrite
        && let Some(edited) = reading::load(root, id)
            .map_err(err)?
            .filter(|v| v.read)
            .and_then(|v| v.reading.edited)
    {
        return Err(format!(
            "작가가 앱에서 이 회차 해석을 고쳤다({edited}). 새로 저장하면 고친 것이 사라진다. reading으로 고친 해석을 보고 작가에게 물은 뒤, 덮어쓰기로 했으면 overwrite: true로 다시 저장한다"
        ));
    }
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
        rhythm: args
            .get("rhythm")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        edited: None,
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
    let rhythm = saved
        .sentences
        .iter()
        .filter(|s| s.rhythm.is_some())
        .count();
    Ok(json!({
        "id": id,
        "scenes": saved.scenes.len(),
        "sentences": saved.sentences.len(),
        "linked": linked,
        "unclear": unclear,
        "rhythm": rhythm,
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
        assert!(
            numbered
                .as_str()
                .unwrap()
                .contains("[리듬] 지문 2문장 · 평균 13자 · 60자 이상 문장 0개"),
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

    #[test]
    fn a_link_reaches_into_another_chapter() {
        let dir = tempfile::tempdir().unwrap();
        let made = call(
            "create_project",
            &json!({"parent": dir.path().to_str().unwrap(), "title": "연결"}),
            None,
        )
        .unwrap();
        let root = PathBuf::from(made["project"].as_str().unwrap());
        let add = |title: &str, text: &str| {
            call(
                "add_doc",
                &with_context("add_doc", json!({"title": title, "text": text})),
                Some(&root),
            )
            .unwrap()["id"]
                .as_str()
                .unwrap()
                .to_string()
        };
        let one = add("1", "전단지를 서랍에 넣었다.");
        let five = add("5", "서랍이 두꺼워졌다.");
        let sentences = json!([{"id": "a", "block": 0, "quote": "전단지를 서랍에 넣었다.", "roles": ["복선"], "meaning": "", "intent": "",
            "links": [{"doc": five, "quote": "서랍이 두꺼워졌다.", "kind": "회수"}]}]);
        call(
            "save_reading",
            &json!({"id": one, "sentences": sentences}),
            Some(&root),
        )
        .unwrap();
        let back = call("reading", &json!({"id": five}), Some(&root)).unwrap();
        assert_eq!(back["read"], false);
        assert_eq!(back["incoming"][0]["fromQuote"], "전단지를 서랍에 넣었다.");
    }

    #[test]
    fn the_writers_edits_are_not_overwritten_without_asking() {
        let dir = tempfile::tempdir().unwrap();
        let made = call(
            "create_project",
            &json!({"parent": dir.path().to_str().unwrap(), "title": "고침"}),
            None,
        )
        .unwrap();
        let root = PathBuf::from(made["project"].as_str().unwrap());
        let added = call(
            "add_doc",
            &with_context("add_doc", json!({"title": "1", "text": "비가 왔다."})),
            Some(&root),
        )
        .unwrap();
        let id = added["id"].as_str().unwrap();
        let sentences = json!([{"id": "a", "block": 0, "quote": "비가 왔다.", "roles": [], "meaning": "", "intent": ""}]);
        let save = |extra: serde_json::Value| {
            let mut args = json!({"id": id, "sentences": sentences});
            args.as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            call("save_reading", &args, Some(&root))
        };
        save(json!({})).unwrap();
        // The writer changes it in the app.
        let mut kept = writer_core::reading::load(&root, id)
            .unwrap()
            .unwrap()
            .reading;
        kept.sentences[0].meaning = "작가의 읽기".into();
        writer_core::reading::save_edit(&root, kept).unwrap();

        let e = save(json!({})).unwrap_err();
        assert!(e.contains("작가가 앱에서"), "{e}");
        save(json!({"overwrite": true})).unwrap();
        let back = call("reading", &json!({"id": id}), Some(&root)).unwrap();
        assert_eq!(back["reading"]["sentences"][0]["meaning"], "");
        assert!(back["reading"].get("edited").is_none());
    }
}
