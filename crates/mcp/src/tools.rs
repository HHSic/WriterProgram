//! The tools: what an assistant can do with a project folder. The MCP server
//! and the CLI both call [`call`], so they behave the same.
//!
//! Everything goes through writer-core, the same code the app saves with:
//! atomic writes, records, and the fingerprint check that notices text changed
//! elsewhere. The app watches an open project's folder, so what a tool writes
//! shows up on screen.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use writer_core::cards::{self, Card};
use writer_core::count::count_blocks;
use writer_core::doc::{self, MetaPatch, SaveGuard, Section};
use writer_core::notes::{self, Anchor, NewNote};
use writer_core::project::{self, NewDoc, NewProject, ProjectKind};
use writer_core::{markup::Block, snapshot};

use crate::text::{has_formatting, to_blocks, to_text};
use crate::{context, guide, manage, readings, review};

/// Same interval as the app's editor (`app/src-tauri/src/commands/doc.rs`).
const AUTO_RECORD_EVERY_MINUTES: i64 = 10;

/// Name of the record kept before a tool changes a body. It is a "manual"
/// record (지금 원고 보관), so it stays until the writer removes it.
pub const RECORD_NAME: &str = "Claude가 고치기 전";

pub type Outcome = Result<Value, String>;

/// One tool's entry in `tools/list`, with the `project` argument every tool
/// on a project takes.
pub(crate) fn tool(name: &str, description: &str, props: Value, required: &[&str]) -> Value {
    let mut props = props;
    props["project"] = json!({"type": "string", "description": "작품 폴더 경로 (project.json이 있는 폴더). 서버를 켤 때 정했으면 생략"});
    json!({
        "name": name,
        "description": description,
        "inputSchema": {"type": "object", "properties": props, "required": required},
    })
}

/// Tool list for MCP `tools/list`.
pub fn list() -> Value {
    let new_facts = json!({"type": "array", "items": {"type": "string"}, "description": "보낸 글에서 새로 지어낸 이름이나 사실. 회차에 메모로 남는다"});
    let mut tools = vec![
        json!({
            "name": "guide",
            "description": "kfiction 한국어 소설 작문 지침. 글을 쓰거나 고치거나 후보를 내기 전에 먼저 읽는다(읽기 전에는 회차를 쓸 수 없다). topic을 주면 참고 문서 하나(context, speech-levels, figurative, vocabulary, revision, reading=작품 해석 절차).",
            "inputSchema": {"type": "object", "properties": {"topic": {"type": "string", "enum": ["context", "speech-levels", "figurative", "vocabulary", "revision", "reading"]}}, "required": []},
        }),
        tool(
            "outline",
            "작품 목차: 부와 회차(id, 제목, 상태, 글자 수, 잠금), 기획 문서, 설정 카드 목록, 전체 글자 수.",
            json!({}),
            &[],
        ),
        tool(
            "read_doc",
            "회차나 기획 문서 하나를 읽는다. 본문은 한 줄에 한 문단, 장면 나눔은 ***. rev는 write_doc에 base_rev로 넘긴다.",
            json!({"id": {"type": "string"}}),
            &["id"],
        ),
        tool(
            "add_doc",
            "새 회차(또는 기획 문서)를 만든다. text를 주면 본문으로 넣는다. 작가가 쓰라고 했을 때만 쓴다. 회차 본문은 write_doc과 같이 점검한다.",
            json!({
                "title": {"type": "string"},
                "text": {"type": "string", "description": "본문. 한 줄에 한 문단, 장면 나눔은 ***"},
                "section": {"type": "string", "enum": ["manuscript", "planning"], "description": "기본 manuscript(회차)"},
                "part_id": {"type": "string", "description": "넣을 부. 기본은 마지막 부"},
                "after": {"type": "string", "description": "이 문서 바로 뒤에 넣음"},
                "context": context::schema(),
                "new_facts": new_facts.clone(),
            }),
            &["title"],
        ),
        tool(
            "write_doc",
            "회차 본문을 고친다. 답의 chars는 회차 전체, added_chars는 보낸 글의 글자 수(공백 포함). mode=append는 끝에 덧붙이고 replace는 통째로 바꾼다. 고치기 직전 본문을 '지금 원고 보관' 기록으로 남기므로 앱의 기록에서 되돌릴 수 있다. 잠근 회차는 고치지 않는다. 작가가 넣어 달라고 했을 때만 쓴다. 보낸 글에 설명·인사·마크다운이 섞이면 저장하지 않고 고칠 줄을 돌려준다. 저장하면 회차의 kfiction 경고를 check에 담아 준다.",
            json!({
                "id": {"type": "string"},
                "text": {"type": "string", "description": "한 줄에 한 문단, 장면 나눔은 ***"},
                "mode": {"type": "string", "enum": ["append", "replace"]},
                "base_rev": {"type": "string", "description": "read_doc에서 받은 rev. 그 뒤에 앱이나 다른 기기에서 바뀌었으면 고치지 않는다"},
                "allow_plain": {"type": "boolean", "description": "replace로 굵게·방점·메모 구간·문단 여백 같은 서식이 사라져도 된다고 작가가 확인했을 때만 true"},
                "context": context::schema(),
                "new_facts": new_facts,
            }),
            &["id", "text", "mode"],
        ),
        tool(
            "check",
            "kfiction 원고 점검: 계약 위반(원고에 섞인 설명·인사·마크다운), 경고(겹친 피동, 같은 끝맺음 반복 등), 참고(번역 투 등). id로 회차를, 또는 text를 점검한다. 파일은 고치지 않는다.",
            json!({"id": {"type": "string"}, "text": {"type": "string"}}),
            &[],
        ),
        tool(
            "set_meta",
            "회차의 제목, 시놉시스, 상태(draft, revise, done 등)를 바꾼다. 본문은 건드리지 않는다.",
            json!({
                "id": {"type": "string"},
                "title": {"type": "string"},
                "synopsis": {"type": "string"},
                "status": {"type": "string"},
            }),
            &["id"],
        ),
        tool(
            "cards",
            "설정집 카드 전부: 분류, 이름, 다른 이름, 항목, 설명.",
            json!({}),
            &[],
        ),
        tool(
            "save_card",
            "설정 카드를 만들거나 고친다. id가 없으면 새로 만든다. fields는 {항목: 값}으로, 준 항목만 바꾼다.",
            json!({
                "id": {"type": "string"},
                "type": {"type": "string", "description": "분류 id. 기본 person(인물), place(장소), term(용어)"},
                "name": {"type": "string"},
                "aliases": {"type": "array", "items": {"type": "string"}},
                "fields": {"type": "object", "additionalProperties": {"type": "string"}},
                "description": {"type": "string"},
            }),
            &[],
        ),
        tool(
            "notes",
            "메모 목록: 붙은 곳(작품, 회차, 본문 구간), 인용, 내용, 완료 여부.",
            json!({}),
            &[],
        ),
        tool(
            "add_note",
            "메모를 남긴다. doc_id를 주면 그 회차에, 없으면 작품에 붙는다. 문장 후보나 제안을 본문 대신 남길 때 쓴다.",
            json!({
                "text": {"type": "string"},
                "doc_id": {"type": "string"},
            }),
            &["text"],
        ),
        json!({
                "name": "create_project",
                "description": "새 작품 폴더를 만든다. Google Drive 같은 동기화 폴더 안에 만들면 앱과 다른 기기에서 함께 열린다.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "parent": {"type": "string", "description": "작품 폴더를 만들 상위 폴더"},
                        "title": {"type": "string"},
                        "kind": {"type": "string", "enum": ["webnovel", "print"], "description": "웹소설 또는 출판 원고. 기본 print"},
                    },
                    "required": ["parent", "title"],
                },
        }),
    ];
    tools.extend(manage::list());
    tools.extend(readings::list());
    Value::Array(tools)
}

/// Runs a tool. `default_project` is the folder given when the server started.
pub fn call(name: &str, args: &Value, default_project: Option<&Path>) -> Outcome {
    match name {
        "create_project" => return create_project(args),
        "guide" => return guide::read(str_arg(args, "topic")).map(Value::String),
        _ => {}
    }
    let root = project_root(args, default_project)?;
    match name {
        "outline" => outline(&root),
        "read_doc" => read_doc(&root, args),
        "add_doc" => add_doc(&root, args),
        "write_doc" => write_doc(&root, args),
        "set_meta" => set_meta(&root, args),
        "cards" => list_cards(&root),
        "save_card" => save_card(&root, args),
        "notes" => list_notes(&root),
        "add_note" => add_note(&root, args),
        "check" => check_doc(&root, args),
        _ => manage::call(name, &root, args)
            .or_else(|| readings::call(name, &root, args))
            .unwrap_or_else(|| Err(format!("알 수 없는 도구: {name}"))),
    }
}

/// A tool's answer as the text it hands back: strings as they are, values
/// as pretty JSON.
pub fn as_text(out: &Value) -> String {
    match out {
        Value::String(s) => s.clone(),
        v => serde_json::to_string_pretty(v).expect("values serialize"),
    }
}

pub(crate) fn err(e: writer_core::Error) -> String {
    e.to_string()
}

pub(crate) fn str_arg<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(Value::as_str)
}

pub(crate) fn required<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    str_arg(args, key).ok_or_else(|| format!("{key}를 주세요"))
}

/// The project folder: the `project` argument, else the server's, else the
/// current folder when it is a project.
fn project_root(args: &Value, default_project: Option<&Path>) -> Result<PathBuf, String> {
    let given = str_arg(args, "project").map(PathBuf::from);
    let root = given
        .or_else(|| default_project.map(Path::to_path_buf))
        .or_else(|| {
            std::env::current_dir()
                .ok()
                .filter(|d| d.join("project.json").is_file())
        })
        .ok_or("작품 폴더를 정해 주세요 (project 인자, 또는 서버를 켤 때 --project)")?;
    if !root.join("project.json").is_file() {
        return Err(format!(
            "작품 폴더가 아님 (project.json 없음): {}",
            root.display()
        ));
    }
    Ok(root)
}

fn outline(root: &Path) -> Outcome {
    let o = project::overview(root).map_err(err)?;
    let summary = |d: &project::DocSummary| {
        json!({
            "id": d.id,
            "title": d.title,
            "status": d.status,
            "chars": d.counts.with_spaces,
            "locked": d.locked,
            "synopsis": d.synopsis,
        })
    };
    Ok(json!({
        "root": o.root,
        "title": o.project.title,
        "kind": o.project.kind,
        "parts": o.parts.iter().map(|p| json!({
            "id": p.id,
            "title": p.title,
            "docs": p.docs.iter().map(summary).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "planning": o.planning.iter().map(summary).collect::<Vec<_>>(),
        "cards": o.cards.iter().map(|c| json!({"id": c.id, "type": c.card_type, "name": c.name, "summary": c.summary})).collect::<Vec<_>>(),
        "total_chars": o.total.with_spaces,
        "unreadable": o.unreadable.iter().map(|u| json!({"id": u.id, "reason": u.reason})).collect::<Vec<_>>(),
    }))
}

fn read_doc(root: &Path, args: &Value) -> Outcome {
    let id = required(args, "id")?;
    let (section, _) = doc::locate(root, id).map_err(err)?;
    let d = doc::load(root, id).map_err(err)?;
    Ok(json!({
        "id": d.meta.id,
        "section": section,
        "title": d.meta.title,
        "synopsis": d.meta.synopsis,
        "status": d.meta.status,
        "locked": d.meta.locked,
        "rev": d.rev(),
        "formatted": has_formatting(&d.body),
        "text": to_text(&d.body),
    }))
}

fn add_doc(root: &Path, args: &Value) -> Outcome {
    let section = match str_arg(args, "section") {
        None | Some("manuscript") => Section::Manuscript,
        Some("planning") => Section::Planning,
        Some(other) => return Err(format!("section은 manuscript나 planning: {other}")),
    };
    let spec = NewDoc {
        section: Some(section),
        part_id: str_arg(args, "part_id").map(String::from),
        after: str_arg(args, "after").map(String::from),
        title: required(args, "title")?.to_string(),
    };
    let text = str_arg(args, "text").unwrap_or_default();
    let manuscript = section == Section::Manuscript && !text.trim().is_empty();
    let gate = if manuscript {
        Some(gate(
            &format!("{}#new:{}", root.display(), spec.title),
            args,
            text,
        )?)
    } else {
        None
    };
    let id = project::add_doc(root, &spec).map_err(err)?;
    let blocks = to_blocks(text);
    let chars = if blocks.is_empty() {
        0
    } else {
        save(root, &id, blocks.clone(), None)?.counts.with_spaces
    };
    let mut out = json!({"id": id, "chars": chars});
    if let Some(gate) = gate {
        out["check"] = report(root, &id, gate, args, text, &to_text(&blocks))?;
    }
    Ok(out)
}

fn write_doc(root: &Path, args: &Value) -> Outcome {
    let id = required(args, "id")?;
    let text = required(args, "text")?;
    let new = to_blocks(text);
    let added_chars = count_blocks(&new).with_spaces;
    let (section, _) = doc::locate(root, id).map_err(err)?;
    let current = doc::load(root, id).map_err(err)?;
    if current.meta.locked {
        return Err("잠근 회차라 고치지 않음. 작가가 앱에서 잠금을 풀어야 함".into());
    }
    if let Some(base) = str_arg(args, "base_rev")
        && base != current.rev()
    {
        return Err(
            "read_doc 뒤에 앱이나 다른 기기에서 본문이 바뀌었음. 다시 읽고 고쳐 주세요".into(),
        );
    }
    let body = match required(args, "mode")? {
        "append" => {
            let mut body = current.body.clone();
            body.extend(new);
            body
        }
        "replace" => {
            let allow_plain = args.get("allow_plain").and_then(Value::as_bool) == Some(true);
            if has_formatting(&current.body) && !allow_plain {
                return Err("이 회차에는 굵게·방점·메모 구간·문단 여백 같은 서식이 있어 통째로 바꾸면 사라짐. append를 쓰거나, 작가가 괜찮다고 하면 allow_plain: true로 다시 부르세요".into());
            }
            new
        }
        other => return Err(format!("mode는 append나 replace: {other}")),
    };
    let gate = if section == Section::Manuscript {
        Some(gate(&format!("{}#{id}", root.display()), args, text)?)
    } else {
        None
    };
    let record = keep_before(root, &current)?;
    let chapter_text = to_text(&body);
    let outcome = save(root, id, body, Some(&current.rev()))?;
    let mut out = json!({
        "id": id,
        "rev": outcome.rev,
        "chars": outcome.counts.with_spaces,
        "added_chars": added_chars,
        "record": record,
    });
    if let Some(gate) = gate {
        out["check"] = report(root, id, gate, args, text, &chapter_text)?;
    }
    Ok(out)
}

/// What a chapter write is checked against afterwards.
struct Gate {
    context: context::Context,
    unfixed: Vec<Value>,
}

/// Before `text` goes into a chapter: the guide was read, the context memo
/// came with it, and it has no contract breaks (review.rs).
fn gate(key: &str, args: &Value, text: &str) -> Result<Gate, String> {
    guide::check_read()?;
    let context = context::parse(args, text)?;
    let unfixed = review::before(key, text)?;
    Ok(Gate { context, unfixed })
}

/// After a chapter write: its warnings, the memo against the text sent, and
/// the names or facts the assistant made up, kept as a note on the chapter.
fn report(
    root: &Path,
    id: &str,
    gate: Gate,
    args: &Value,
    text: &str,
    chapter_text: &str,
) -> Result<Value, String> {
    let mut check = review::after(chapter_text, gate.unfixed);
    let memo = context::line(&gate.context);
    check["context"] = Value::Array(context::compare(&gate.context, text));
    check["memo"] = json!(memo);
    let facts: Vec<&str> = args
        .get("new_facts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|f| !f.is_empty())
        .collect();
    if !facts.is_empty() {
        let lines: Vec<String> = facts.iter().map(|f| format!("- {f}")).collect();
        let note = notes::create(
            root,
            &NewNote {
                id: None,
                anchor: Anchor::Doc,
                target: id.to_string(),
                quote: String::new(),
                text: format!("새 설정 (Claude가 지어냄 · {memo})\n{}", lines.join("\n")),
            },
        )
        .map_err(err)?;
        check["new_facts_note"] = json!(note.id);
    }
    Ok(check)
}

fn check_doc(root: &Path, args: &Value) -> Outcome {
    match (str_arg(args, "id"), str_arg(args, "text")) {
        (Some(id), _) => Ok(review::full(&to_text(
            &doc::load(root, id).map_err(err)?.body,
        ))),
        (None, Some(text)) => Ok(review::full(text)),
        (None, None) => Err("id나 text를 주세요".into()),
    }
}

/// Keeps the body as it is before a tool changes it, unless the newest record
/// already has it. Returns the record id when one was made.
fn keep_before(root: &Path, current: &doc::DocFile) -> Result<Option<String>, String> {
    let id = &current.meta.id;
    if current.body.is_empty() {
        return Ok(None);
    }
    if let Some(newest) = snapshot::list(root, id).map_err(err)?.first()
        && snapshot::load(root, id, &newest.id).map_err(err)?.body == current.body
    {
        return Ok(None);
    }
    let info = snapshot::create(root, current, "manual", RECORD_NAME).map_err(err)?;
    Ok(Some(info.id))
}

fn save(
    root: &Path,
    id: &str,
    body: Vec<Block>,
    base: Option<&str>,
) -> Result<doc::SaveOutcome, String> {
    let outcome = doc::save_body(
        root,
        id,
        body,
        chrono::Duration::minutes(AUTO_RECORD_EVERY_MINUTES),
        SaveGuard { base, force: false },
    )
    .map_err(err)?;
    if outcome.conflict {
        return Err(
            "저장하는 사이 다른 곳에서 본문이 바뀌어 저장하지 않음. 쓰려던 글은 기록에 남겼음. 다시 읽고 고쳐 주세요"
                .into(),
        );
    }
    Ok(outcome)
}

fn set_meta(root: &Path, args: &Value) -> Outcome {
    let id = required(args, "id")?;
    let patch = MetaPatch {
        title: str_arg(args, "title").map(String::from),
        synopsis: str_arg(args, "synopsis").map(String::from),
        status: str_arg(args, "status").map(String::from),
        ..MetaPatch::default()
    };
    let meta = doc::update_meta(root, id, &patch).map_err(err)?;
    Ok(
        json!({"id": meta.id, "title": meta.title, "synopsis": meta.synopsis, "status": meta.status}),
    )
}

fn card_json(c: &Card) -> Value {
    let fields: serde_json::Map<String, Value> = c
        .fields
        .iter()
        .map(|(k, v)| (k.clone(), Value::String(v.clone())))
        .collect();
    json!({
        "id": c.id,
        "type": c.card_type,
        "name": c.name,
        "aliases": c.aliases,
        "fields": fields,
        "description": c.description,
    })
}

fn list_cards(root: &Path) -> Outcome {
    let cards = cards::load_all(root).map_err(err)?;
    Ok(Value::Array(cards.iter().map(card_json).collect()))
}

fn save_card(root: &Path, args: &Value) -> Outcome {
    let mut card = match str_arg(args, "id") {
        Some(id) => cards::load(root, id).map_err(err)?,
        None => cards::create(
            root,
            str_arg(args, "type").unwrap_or("person"),
            required(args, "name")?,
        )
        .map_err(err)?,
    };
    if let Some(name) = str_arg(args, "name") {
        card.name = name.to_string();
    }
    if let Some(aliases) = args.get("aliases").and_then(Value::as_array) {
        card.aliases = aliases
            .iter()
            .filter_map(Value::as_str)
            .map(String::from)
            .collect();
    }
    if let Some(fields) = args.get("fields").and_then(Value::as_object) {
        for (key, value) in fields {
            let value = value.as_str().unwrap_or_default().to_string();
            match card.fields.iter_mut().find(|(k, _)| k == key) {
                Some(field) => field.1 = value,
                None => card.fields.push((key.clone(), value)),
            }
        }
    }
    if let Some(description) = str_arg(args, "description") {
        card.description = description.to_string();
    }
    cards::save(root, &card).map_err(err)?;
    Ok(card_json(&card))
}

fn list_notes(root: &Path) -> Outcome {
    let notes = notes::list(root).map_err(err)?;
    Ok(Value::Array(
        notes
            .iter()
            .map(|n| {
                json!({
                    "id": n.id,
                    "anchor": n.anchor,
                    "target": n.target,
                    "quote": n.quote,
                    "text": n.text,
                    "done": n.done,
                })
            })
            .collect(),
    ))
}

fn add_note(root: &Path, args: &Value) -> Outcome {
    let doc_id = str_arg(args, "doc_id");
    let spec = NewNote {
        id: None,
        anchor: if doc_id.is_some() {
            Anchor::Doc
        } else {
            Anchor::Project
        },
        target: doc_id.unwrap_or_default().to_string(),
        quote: String::new(),
        text: required(args, "text")?.to_string(),
    };
    let note = notes::create(root, &spec).map_err(err)?;
    Ok(json!({"id": note.id, "anchor": note.anchor, "target": note.target}))
}

fn create_project(args: &Value) -> Outcome {
    let kind = match str_arg(args, "kind") {
        None | Some("print") => ProjectKind::Print,
        Some("webnovel") => ProjectKind::Webnovel,
        Some(other) => return Err(format!("kind는 webnovel이나 print: {other}")),
    };
    let spec = NewProject {
        parent: required(args, "parent")?.to_string(),
        title: required(args, "title")?.to_string(),
        kind,
        per_doc_goal: None,
        count_spaces: true,
        first_chapter: true,
        platform: None,
    };
    let root = project::create(&spec).map_err(err)?;
    Ok(json!({"project": root.display().to_string()}))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn new_project() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let made = call(
            "create_project",
            &json!({"parent": dir.path().to_str().unwrap(), "title": "사흘째 맑음"}),
            None,
        )
        .unwrap();
        let root = PathBuf::from(made["project"].as_str().unwrap());
        (dir, root)
    }

    /// Writing tools get a context memo unless the test gives its own.
    pub(crate) fn with_context(name: &str, mut args: Value) -> Value {
        if matches!(name, "write_doc" | "add_doc") && args.get("context").is_none() {
            args["context"] = json!({"pov": "윤재", "tense": "past", "speech": [{"from": "윤재", "to": "서아", "level": "해체"}]});
        }
        args
    }

    fn run(root: &Path, name: &str, args: Value) -> Outcome {
        call(name, &with_context(name, args), Some(root))
    }

    fn first_chapter(root: &Path) -> String {
        let o = run(root, "outline", json!({})).unwrap();
        o["parts"][0]["docs"][0]["id"].as_str().unwrap().to_string()
    }

    #[test]
    fn writes_a_new_chapter_and_reads_it_back() {
        let (_dir, root) = new_project();
        let made = run(
            &root,
            "add_doc",
            json!({"title": "2", "text": "노인이 왔다.\n\n“하나 깎아 줘.”\n***\n비가 왔다."}),
        )
        .unwrap();
        let id = made["id"].as_str().unwrap();
        let read = run(&root, "read_doc", json!({"id": id})).unwrap();
        assert_eq!(read["title"], "2");
        assert_eq!(
            read["text"],
            "노인이 왔다.\n\n“하나 깎아 줘.”\n\n***\n\n비가 왔다."
        );
        let o = run(&root, "outline", json!({})).unwrap();
        assert_eq!(o["parts"][0]["docs"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn changing_a_body_keeps_the_text_before() {
        let (_dir, root) = new_project();
        let id = first_chapter(&root);
        run(
            &root,
            "write_doc",
            json!({"id": id, "text": "첫 문단.", "mode": "append"}),
        )
        .unwrap();
        let rev = run(&root, "read_doc", json!({"id": id})).unwrap()["rev"].clone();
        let w = run(
            &root,
            "write_doc",
            json!({"id": id, "text": "둘째 문단.", "mode": "append", "base_rev": rev}),
        )
        .unwrap();
        let record = w["record"].as_str().unwrap();
        let kept = snapshot::load(&root, &id, record).unwrap();
        assert_eq!(to_text(&kept.body), "첫 문단.");
        let info = &snapshot::list(&root, &id).unwrap()[0];
        assert_eq!(
            (info.kind.as_str(), info.name.as_str()),
            ("manual", RECORD_NAME)
        );
        let text = run(&root, "read_doc", json!({"id": id})).unwrap()["text"].clone();
        assert_eq!(text, "첫 문단.\n\n둘째 문단.");
        assert_eq!(
            (w["chars"].as_u64(), w["added_chars"].as_u64()),
            (Some(11), Some(6))
        );
    }

    #[test]
    fn a_stale_rev_or_a_locked_chapter_is_refused() {
        let (_dir, root) = new_project();
        let id = first_chapter(&root);
        let stale = run(&root, "read_doc", json!({"id": id})).unwrap()["rev"].clone();
        run(
            &root,
            "write_doc",
            json!({"id": id, "text": "앱에서 쓴 글.", "mode": "append"}),
        )
        .unwrap();
        let e = run(
            &root,
            "write_doc",
            json!({"id": id, "text": "x", "mode": "replace", "base_rev": stale}),
        )
        .unwrap_err();
        assert!(e.contains("다시 읽고"), "{e}");

        doc::update_meta(
            &root,
            &id,
            &MetaPatch {
                locked: Some(true),
                ..MetaPatch::default()
            },
        )
        .unwrap();
        let e = run(
            &root,
            "write_doc",
            json!({"id": id, "text": "x", "mode": "append"}),
        )
        .unwrap_err();
        assert!(e.contains("잠근 회차"), "{e}");
    }

    #[test]
    fn replacing_formatted_text_needs_consent() {
        let (_dir, root) = new_project();
        let id = first_chapter(&root);
        doc::save_body(
            &root,
            &id,
            writer_core::markup::parse_body("그는 **분명히** 말했다."),
            chrono::Duration::minutes(10),
            SaveGuard::default(),
        )
        .unwrap();
        let e = run(
            &root,
            "write_doc",
            json!({"id": id, "text": "새 글.", "mode": "replace"}),
        )
        .unwrap_err();
        assert!(e.contains("allow_plain"), "{e}");
        run(
            &root,
            "write_doc",
            json!({"id": id, "text": "새 글.", "mode": "replace", "allow_plain": true}),
        )
        .unwrap();
        assert_eq!(
            run(&root, "read_doc", json!({"id": id})).unwrap()["text"],
            "새 글."
        );
    }

    #[test]
    fn cards_and_notes() {
        let (_dir, root) = new_project();
        let made = run(
            &root,
            "save_card",
            json!({"name": "윤재", "aliases": ["윤재 씨"], "fields": {"나이": "28", "말투": "손님에게 해요체"}}),
        )
        .unwrap();
        assert_eq!(made["type"], "person");
        assert_eq!(made["fields"]["나이"], "28");
        let id = made["id"].as_str().unwrap();
        run(
            &root,
            "save_card",
            json!({"id": id, "fields": {"직업": "열쇠 가게"}}),
        )
        .unwrap();
        let cards = run(&root, "cards", json!({})).unwrap();
        assert_eq!(cards[0]["fields"]["직업"], "열쇠 가게");
        assert_eq!(cards[0]["fields"]["나이"], "28");

        let chapter = first_chapter(&root);
        run(
            &root,
            "add_note",
            json!({"text": "후보 ① …", "doc_id": chapter}),
        )
        .unwrap();
        run(&root, "add_note", json!({"text": "제목 다시 생각"})).unwrap();
        let notes = run(&root, "notes", json!({})).unwrap();
        assert_eq!(notes.as_array().unwrap().len(), 2);
    }

    #[test]
    fn assistant_talk_is_refused_before_saving_twice_at_most() {
        let (_dir, root) = new_project();
        let id = first_chapter(&root);
        run(
            &root,
            "write_doc",
            json!({"id": id, "text": "윤재는 문을 열었다.", "mode": "append"}),
        )
        .unwrap();
        let dirty = json!({"id": id, "text": "다음은 이어서 쓴 장면입니다.\n서아가 뒤따랐다.", "mode": "append"});
        for n in 1..=crate::review::MAX_REFUSALS {
            let e = run(&root, "write_doc", dirty.clone()).unwrap_err();
            assert!(
                e.contains(&format!("({n}/2회차)")) && e.contains("L1 F1"),
                "{e}"
            );
        }
        assert_eq!(
            run(&root, "read_doc", json!({"id": id})).unwrap()["text"],
            "윤재는 문을 열었다."
        );
        assert!(
            snapshot::list(&root, &id).unwrap().is_empty(),
            "a refusal keeps no record"
        );

        let third = run(&root, "write_doc", dirty.clone()).unwrap();
        assert_eq!(third["check"]["unfixed_contract"][0]["rule"], "F1");
        // A clean write starts the count again.
        run(
            &root,
            "write_doc",
            json!({"id": id, "text": "비가 왔다.", "mode": "append"}),
        )
        .unwrap();
        assert!(run(&root, "write_doc", dirty).is_err());
    }

    #[test]
    fn json_escapes_left_in_the_text_are_refused() {
        let (_dir, root) = new_project();
        let id = first_chapter(&root);
        let e = run(
            &root,
            "write_doc",
            json!({"id": id, "text": "문이 열렸다.\n\\\"가자.\\\"", "mode": "append"}),
        )
        .unwrap_err();
        assert!(e.contains("L2 E1 이스케이프 흔적"), "{e}");
        let ok = run(
            &root,
            "write_doc",
            json!({"id": id, "text": "문이 열렸다.\n\"가자.\"", "mode": "append"}),
        );
        assert!(ok.is_ok(), "{ok:?}");
    }

    #[test]
    fn a_save_reports_the_chapter_warnings() {
        let (_dir, root) = new_project();
        let made = run(
            &root,
            "add_doc",
            json!({"title": "2", "text": "물 자국이 번지는 것이 보여졌다.\n“들어오세요.”"}),
        )
        .unwrap();
        assert_eq!(made["check"]["warn"][0]["rule"], "F3");
        assert_eq!(made["check"]["warn_count"], 1);
        assert!(made["check"].get("unfixed_contract").is_none());

        let report = run(&root, "check", json!({"id": made["id"]})).unwrap();
        assert_eq!(report["warn"][0]["match"], "보여졌");
        let text = run(
            &root,
            "check",
            json!({"text": "원하시면 더 다듬어 드릴게요."}),
        )
        .unwrap();
        assert_eq!(text["contract"][0]["rule"], "F1");
    }

    #[test]
    fn a_refused_new_chapter_is_not_made_and_planning_is_not_checked() {
        let (_dir, root) = new_project();
        let e = run(
            &root,
            "add_doc",
            json!({"title": "2", "text": "**굵게** 쓴 장면"}),
        )
        .unwrap_err();
        assert!(e.contains("F2"), "{e}");
        let o = run(&root, "outline", json!({})).unwrap();
        assert_eq!(o["parts"][0]["docs"].as_array().unwrap().len(), 1);

        let plan = run(
            &root,
            "add_doc",
            json!({"title": "기획", "section": "planning", "text": "- **주인공**: 윤재\n- 열쇠 가게"}),
        )
        .unwrap();
        assert!(plan.get("check").is_none());
    }

    #[test]
    fn the_guide_is_read_before_a_chapter_is_written() {
        let (_dir, root) = new_project();
        let id = first_chapter(&root);
        guide::require();
        let e = run(
            &root,
            "write_doc",
            json!({"id": id, "text": "비가 왔다.", "mode": "append"}),
        )
        .unwrap_err();
        assert!(e.contains("guide"), "{e}");
        let text = call("guide", &json!({}), None).unwrap();
        assert!(text.as_str().unwrap().contains("## 확인 목록"));
        run(
            &root,
            "write_doc",
            json!({"id": id, "text": "비가 왔다.", "mode": "append"}),
        )
        .unwrap();
    }

    #[test]
    fn a_chapter_needs_its_context_memo() {
        let (_dir, root) = new_project();
        let id = first_chapter(&root);
        let e = call(
            "write_doc",
            &json!({"id": id, "text": "비가 왔다.", "mode": "append"}),
            Some(&root),
        )
        .unwrap_err();
        assert!(e.contains("맥락 메모"), "{e}");
        let e = call(
            "write_doc",
            &json!({"id": id, "text": "“가자.”", "mode": "append", "context": {"pov": "윤재", "tense": "past"}}),
            Some(&root),
        )
        .unwrap_err();
        assert!(e.contains("speech"), "{e}");
        assert_eq!(
            run(&root, "read_doc", json!({"id": id})).unwrap()["text"],
            ""
        );
    }

    #[test]
    fn the_memo_is_compared_and_new_facts_become_a_note() {
        let (_dir, root) = new_project();
        let id = first_chapter(&root);
        let w = run(
            &root,
            "write_doc",
            json!({
                "id": id,
                "mode": "append",
                "text": "서아가 골목을 달린다. 윤재가 뒤따른다.\n“오빠, 빨리 와요.”",
                "new_facts": ["서아의 할머니는 반찬가게를 한다"],
            }),
        )
        .unwrap();
        let kinds: Vec<&str> = w["check"]["context"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["kind"].as_str().unwrap())
            .collect();
        assert_eq!(kinds, ["시제", "말 단계"]);
        assert_eq!(w["check"]["memo"], "시점 윤재 · 과거형 · 윤재→서아 해체");
        let notes = run(&root, "notes", json!({})).unwrap();
        assert_eq!(notes[0]["target"], id.as_str());
        assert!(
            notes[0]["text"]
                .as_str()
                .unwrap()
                .contains("- 서아의 할머니는 반찬가게를 한다")
        );
    }

    #[test]
    fn a_refusal_is_counted_in_the_state_file() {
        let (_dir, root) = new_project();
        let id = first_chapter(&root);
        let dirty = json!({"id": id, "text": "**굵게**", "mode": "append"});
        run(&root, "write_doc", dirty).unwrap_err();
        let kept = std::fs::read_to_string(
            std::env::temp_dir()
                .join("writer-mcp")
                .join("refusals.json"),
        )
        .unwrap_or_default();
        assert!(kept.contains(&format!("#{id}")), "{kept}");
    }

    #[test]
    fn a_folder_that_is_not_a_project_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let e = call(
            "outline",
            &json!({"project": dir.path().to_str().unwrap()}),
            None,
        )
        .unwrap_err();
        assert!(e.contains("project.json"), "{e}");
    }
}
