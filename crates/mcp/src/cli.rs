//! `writer`: the writer-mcp tools as shell commands, for assistants that work
//! in a terminal and for people (docs/cli.md). Commands are made from the
//! tool list, so a tool added there is a command here too, with the same
//! checks: the guide before writing, the context memo, the kfiction check
//! before and after saving.
//!
//! ```text
//! writer <명령> [인자…] [--이름 값…] [--project 작품폴더] [--json]
//! ```
//!
//! A tool's required arguments are taken in order as plain arguments, every
//! argument as `--이름 값` (`-` for `_`). A chapter's text comes from
//! `--file`, `--text` or standard input; the context memo from `--pov`,
//! `--tense` and `--speech 화자>청자:말단계[:호칭]`. `--args` takes the
//! arguments as JSON, for what does not fit on a line (save_reading).

#[cfg(not(unix))]
use std::io::IsTerminal;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

use crate::{guide, path, state, tools};

/// Short names for the tools used most; the rest go by their name with `-`.
const ALIASES: &[(&str, &str)] = &[
    ("read", "read_doc"),
    ("add", "add_doc"),
    ("write", "write_doc"),
    ("meta", "set_meta"),
    ("card", "save_card"),
    ("note", "add_note"),
    ("new", "create_project"),
];

/// Tools whose `text` is a chapter's text: it comes from a file or standard
/// input rather than a plain argument.
const BODY: &[&str] = &["add_doc", "write_doc", "check"];

const RULES: &str = "규칙 (MCP 서버와 같다)
  - 소설을 쓰거나 고치기 전에 writer guide로 kfiction 작문 지침을 읽는다. 읽은 지 12시간이 지나면 다시 읽어야 쓸 수 있다.
  - 먼저 writer outline으로 구조를 본다. 고치기 전에 writer read <회차>로 읽고 그 rev를 --base-rev로 넘긴다.
  - 원고는 작가가 쓰라고 했을 때만 add, write로 넣는다. 후보와 제안은 writer note로 메모에 남긴다.
  - 본문은 한 줄에 한 문단, 장면 나눔은 ***. 설명·인사·마크다운이 섞이면 저장하지 않고 고칠 줄을 알려 준다.
  - 회차 본문에는 맥락 메모(--pov, --tense, 대사가 있으면 --speech)를 함께 준다.
  - trash-doc, replace-all, restore-record, export는 작가가 시켰을 때만 쓴다.";

/// Runs `writer` with `args` (without the program name). Returns the exit
/// code: 0 done, 1 the tool refused or failed, 2 the command was not understood.
pub fn main(args: &[String]) -> i32 {
    match run(args) {
        Ok(out) => {
            if !out.is_empty() {
                println!("{out}");
            }
            0
        }
        Err(Failure::Tool(e)) => {
            eprintln!("{e}");
            1
        }
        Err(Failure::Usage(e)) => {
            eprintln!("{e}");
            2
        }
    }
}

#[derive(Debug)]
pub enum Failure {
    Usage(String),
    Tool(String),
}

fn usage(e: impl Into<String>) -> Failure {
    Failure::Usage(e.into())
}

/// The tool a command names.
fn tool_name(command: &str) -> Option<String> {
    let name = ALIASES
        .iter()
        .find(|(a, _)| *a == command)
        .map_or_else(|| command.replace('-', "_"), |(_, t)| t.to_string());
    find_tool(&name).map(|_| name)
}

fn find_tool(name: &str) -> Option<Value> {
    tools::list()
        .as_array()?
        .iter()
        .find(|t| t["name"] == name)
        .cloned()
}

fn command_of(tool: &str) -> String {
    ALIASES
        .iter()
        .find(|(_, t)| *t == tool)
        .map_or_else(|| tool.replace('_', "-"), |(a, _)| a.to_string())
}

/// The project: `--project`, else `WRITERPROGRAM_PROJECT`, else the folder
/// with project.json that the current folder is in.
fn project(given: Option<PathBuf>) -> Option<PathBuf> {
    given
        .or_else(|| std::env::var_os("WRITERPROGRAM_PROJECT").map(PathBuf::from))
        .or_else(|| {
            let here = std::env::current_dir().ok()?;
            here.ancestors()
                .find(|d| d.join("project.json").is_file())
                .map(Path::to_path_buf)
        })
}

pub fn run(args: &[String]) -> Result<String, Failure> {
    let mut words = Vec::new();
    let mut project_arg = None;
    let mut as_json = false;
    let mut help = false;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--project" => {
                project_arg = Some(PathBuf::from(
                    it.next()
                        .ok_or_else(|| usage("--project 뒤에 작품 폴더를 주세요"))?,
                ))
            }
            "--json" => as_json = true,
            "--help" | "-h" => help = true,
            _ => words.push(a.clone()),
        }
    }
    let Some(command) = words.first().map(String::as_str) else {
        return Ok(overview());
    };
    match command {
        "help" => {
            return match words.get(1) {
                None => Ok(overview()),
                Some(c) => command_help(c),
            };
        }
        "--version" | "version" => return Ok(format!("writer {}", env!("CARGO_PKG_VERSION"))),
        "path" => {
            return path::run(words.get(1).map_or("status", String::as_str)).map_err(Failure::Tool);
        }
        _ => {}
    }
    if help {
        return command_help(command);
    }
    let tool = tool_name(command).ok_or_else(|| {
        usage(format!(
            "알 수 없는 명령: {command}\n명령 목록은 writer help"
        ))
    })?;
    let schema = find_tool(&tool).expect("named tools exist");
    let args = arguments(&tool, &schema, &words[1..])?;

    guide::require();
    if state::guide_fresh() {
        guide::mark_read();
    }
    let out = tools::call(&tool, &args, project(project_arg).as_deref()).map_err(Failure::Tool)?;
    if tool == "guide" {
        state::remember_guide();
    }
    Ok(if as_json {
        serde_json::to_string_pretty(&out).expect("values serialize")
    } else {
        tools::as_text(&out)
    })
}

fn properties(schema: &Value) -> Map<String, Value> {
    schema["inputSchema"]["properties"]
        .as_object()
        .cloned()
        .unwrap_or_default()
}

fn required(schema: &Value) -> Vec<String> {
    schema["inputSchema"]["required"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str().map(String::from))
        .collect()
}

/// The plain arguments a tool takes, in order.
fn positional(tool: &str, schema: &Value) -> Vec<String> {
    let props = properties(schema);
    // Lists and objects need JSON or repeated options, so they are named.
    let plain = |n: &String| {
        let kind = props
            .get(n)
            .and_then(|p| p["type"].as_str())
            .unwrap_or("string");
        kind == "string" || kind == "integer"
    };
    let mut names: Vec<String> = required(schema)
        .into_iter()
        .filter(|n| !(n == "text" && BODY.contains(&tool)) && plain(n))
        .collect();
    // An optional id or guide's topic reads best as a plain argument too.
    for optional in ["id", "topic"] {
        if props.contains_key(optional) && !names.iter().any(|n| n == optional) {
            names.push(optional.into());
        }
    }
    names
}

/// A value given on the command line, as the schema wants it.
fn parse_value(name: &str, prop: &Value, raw: &str) -> Result<Value, Failure> {
    let kind = prop["type"].as_str().unwrap_or("string");
    match kind {
        "integer" => raw.parse::<i64>().map(Value::from).map_err(|_| {
            usage(format!(
                "--{}에는 수를 주세요: {raw}",
                name.replace('_', "-")
            ))
        }),
        "boolean" => match raw {
            "true" | "yes" | "1" => Ok(Value::Bool(true)),
            "false" | "no" | "0" => Ok(Value::Bool(false)),
            _ => Err(usage(format!(
                "--{}에는 true나 false를 주세요",
                name.replace('_', "-")
            ))),
        },
        "object" => serde_json::from_str(raw).map_err(|e| {
            usage(format!(
                "--{}에는 JSON 객체를 주세요: {e}",
                name.replace('_', "-")
            ))
        }),
        "array" if prop["items"]["type"] != "string" => serde_json::from_str(raw).map_err(|e| {
            usage(format!(
                "--{}에는 JSON 배열을 주세요: {e}",
                name.replace('_', "-")
            ))
        }),
        _ => Ok(Value::String(raw.to_string())),
    }
}

/// Standard input is a pipe or a file (`< 장면.md`), not a terminal or
/// nothing, so reading it will not wait for someone to type.
#[cfg(unix)]
fn piped() -> bool {
    use std::os::fd::AsFd;
    use std::os::unix::fs::FileTypeExt;
    let Ok(fd) = std::io::stdin().as_fd().try_clone_to_owned() else {
        return false;
    };
    std::fs::File::from(fd)
        .metadata()
        .is_ok_and(|m| m.file_type().is_fifo() || m.file_type().is_file())
}

#[cfg(not(unix))]
fn piped() -> bool {
    !std::io::stdin().is_terminal()
}

fn read_stdin() -> Result<String, Failure> {
    let mut s = String::new();
    std::io::stdin()
        .read_to_string(&mut s)
        .map_err(|e| usage(format!("표준 입력을 읽지 못함: {e}")))?;
    Ok(s)
}

fn read_file(path: &str) -> Result<String, Failure> {
    if path == "-" {
        return read_stdin();
    }
    std::fs::read_to_string(path).map_err(|e| usage(format!("파일을 읽지 못함 {path}: {e}")))
}

/// `화자>청자:말단계[:호칭]` as one context.speech entry.
fn speech(raw: &str) -> Result<Value, Failure> {
    let bad = || {
        usage(format!(
            "--speech는 화자>청자:말단계[:호칭] 꼴입니다 (예: 윤재>서아:해체:서아야): {raw}"
        ))
    };
    let (pair, rest) = raw.split_once(':').ok_or_else(bad)?;
    let (from, to) = pair.split_once('>').ok_or_else(bad)?;
    let (level, address) = rest.split_once(':').unwrap_or((rest, ""));
    let mut entry = json!({"from": from.trim(), "to": to.trim(), "level": level.trim()});
    if !address.trim().is_empty() {
        entry["address"] = json!(address.trim());
    }
    Ok(entry)
}

/// The tool's arguments from what followed the command.
pub fn arguments(tool: &str, schema: &Value, words: &[String]) -> Result<Value, Failure> {
    let props = properties(schema);
    let mut args = Map::new();
    let mut plain = Vec::new();
    let mut context = Map::new();
    let mut body: Option<String> = None;
    let flag = |s: &str| s.len() > 2 && s.starts_with("--");
    let mut i = 0;
    while i < words.len() {
        let word = &words[i];
        i += 1;
        if !flag(word) {
            plain.push(word.clone());
            continue;
        }
        let (key, inline) = match word[2..].split_once('=') {
            Some((k, v)) => (k.replace('-', "_"), Some(v.to_string())),
            None => (word[2..].replace('-', "_"), None),
        };
        let prop = props.get(&key);
        // A flag without a value is `true` for a yes-or-no argument.
        if inline.is_none() && prop.is_some_and(|p| p["type"] == "boolean") {
            let next_is_value = words
                .get(i)
                .is_some_and(|v| matches!(v.as_str(), "true" | "false" | "yes" | "no"));
            if !next_is_value {
                args.insert(key, Value::Bool(true));
                continue;
            }
        }
        let mut value = || -> Result<String, Failure> {
            if let Some(v) = &inline {
                return Ok(v.clone());
            }
            let v = words
                .get(i)
                .ok_or_else(|| usage(format!("{word} 뒤에 값을 주세요")))?
                .clone();
            i += 1;
            Ok(v)
        };
        match key.as_str() {
            "args" => {
                let raw = value()?;
                let raw = if raw == "-" { read_stdin()? } else { raw };
                let more: Value = serde_json::from_str(&raw)
                    .map_err(|e| usage(format!("--args에는 JSON 객체를 주세요: {e}")))?;
                let Value::Object(more) = more else {
                    return Err(usage("--args에는 JSON 객체를 주세요"));
                };
                args.extend(more);
            }
            "args_file" => {
                let raw = read_file(&value()?)?;
                let Ok(Value::Object(more)) = serde_json::from_str::<Value>(&raw) else {
                    return Err(usage("--args-file에는 JSON 객체 파일을 주세요"));
                };
                args.extend(more);
            }
            "file" if props.contains_key("text") => body = Some(read_file(&value()?)?),
            "pov" | "tense" if props.contains_key("context") => {
                context.insert(key, Value::String(value()?));
            }
            "speech" if props.contains_key("context") => {
                let entry = speech(&value()?)?;
                context
                    .entry("speech")
                    .or_insert_with(|| json!([]))
                    .as_array_mut()
                    .expect("an array")
                    .push(entry);
            }
            _ => {
                let Some(prop) = prop else {
                    return Err(usage(format!(
                        "{} 명령에 없는 옵션: {word}\n쓸 수 있는 것은 writer help {}",
                        command_of(tool),
                        command_of(tool)
                    )));
                };
                let raw = value()?;
                let v = parse_value(&key, prop, &raw)?;
                // A list of words can be given again and again.
                if prop["type"] == "array" && prop["items"]["type"] == "string" {
                    args.entry(key)
                        .or_insert_with(|| json!([]))
                        .as_array_mut()
                        .expect("an array")
                        .push(v);
                } else {
                    args.insert(key, v);
                }
            }
        }
    }

    if !context.is_empty() {
        let given = args.entry("context").or_insert_with(|| json!({}));
        if let Value::Object(given) = given {
            given.extend(context);
        }
    }

    let free: Vec<String> = positional(tool, schema)
        .into_iter()
        .filter(|n| !args.contains_key(n))
        .collect();
    let mut names = free.into_iter();
    for word in plain {
        let Some(name) = names.next() else {
            return Err(usage(format!(
                "인자가 너무 많음: {word}\n띄어쓰기가 있는 값은 따옴표로 감쌉니다 (예: \"1화 · 셔터\"). 사용법은 writer help {}",
                command_of(tool)
            )));
        };
        let v = parse_value(&name, &props[&name], &word)?;
        args.insert(name, v);
    }

    if BODY.contains(&tool) && !args.contains_key("text") {
        // check reads a chapter by id when it has one.
        let wanted = !(tool == "check" && args.contains_key("id"));
        if body.is_none() && wanted && piped() {
            body = Some(read_stdin()?).filter(|s| !s.trim().is_empty());
        }
        if let Some(text) = body {
            args.insert("text".into(), Value::String(text));
        }
    }

    let missing: Vec<String> = required(schema)
        .into_iter()
        .filter(|n| !args.contains_key(n))
        .map(|n| match n.as_str() {
            "text" if BODY.contains(&tool) => "본문(--file, --text, 또는 표준 입력)".to_string(),
            _ => format!("--{}", n.replace('_', "-")),
        })
        .collect();
    if !missing.is_empty() {
        return Err(usage(format!(
            "빠진 인자: {}\n사용법은 writer help {}",
            missing.join(", "),
            command_of(tool)
        )));
    }
    Ok(Value::Object(args))
}

/// The first sentence of a description.
fn first_sentence(text: &str) -> &str {
    text.find(". ")
        .map_or(text, |at| &text[..at + 1])
        .trim_end_matches(':')
}

fn overview() -> String {
    let mut lines = vec![
        format!(
            "writer {} — WriterProgram 작품 폴더를 터미널에서 읽고 쓴다. AI 도우미 MCP 서버(writer-mcp)와 같은 도구와 점검.",
            env!("CARGO_PKG_VERSION")
        ),
        String::new(),
        "사용법: writer <명령> [인자…] [--옵션 값…] [--project 작품폴더] [--json]".into(),
        "작품 폴더: --project, 환경 변수 WRITERPROGRAM_PROJECT, 아니면 지금 폴더(와 그 위)에서 project.json을 찾는다.".into(),
        String::new(),
        "명령:".into(),
    ];
    for t in tools::list().as_array().into_iter().flatten() {
        let tool = t["name"].as_str().unwrap_or("");
        let schema = find_tool(tool).expect("listed");
        let args: Vec<String> = positional(tool, &schema)
            .iter()
            .map(|n| {
                if required(&schema).contains(n) {
                    format!("<{n}>")
                } else {
                    format!("[{n}]")
                }
            })
            .collect();
        let head = format!("{} {}", command_of(tool), args.join(" "));
        lines.push(format!(
            "  {:<28} {}",
            head.trim_end(),
            first_sentence(t["description"].as_str().unwrap_or(""))
        ));
    }
    lines.push(format!(
        "  {:<28} 터미널에서 writer를 이름으로 부르게 PATH에 등록하거나 뺀다 (Windows)",
        "path add|remove|status"
    ));
    lines.push(String::new());
    lines.push(RULES.into());
    lines.push(String::new());
    lines.push("자세히: writer help <명령>   예: writer help write".into());
    lines.join("\n")
}

fn kind_label(prop: &Value) -> String {
    if let Some(choices) = prop["enum"].as_array() {
        let names: Vec<&str> = choices.iter().filter_map(Value::as_str).collect();
        return names.join("|");
    }
    match prop["type"].as_str().unwrap_or("string") {
        "integer" => "수".into(),
        "boolean" => String::new(),
        "object" => "JSON".into(),
        "array" if prop["items"]["type"] == "string" => "값 (여러 번 줄 수 있음)".into(),
        "array" => "JSON 배열".into(),
        _ => "값".into(),
    }
}

fn command_help(command: &str) -> Result<String, Failure> {
    if command == "path" {
        return Ok("writer path add | remove | status\n  writer가 든 폴더를 사용자 PATH에 등록하거나 빼거나, 등록됐는지 본다 (Windows). 등록한 뒤 새로 연 터미널부터 writer로 부를 수 있다.".into());
    }
    let tool = tool_name(command).ok_or_else(|| {
        usage(format!(
            "알 수 없는 명령: {command}\n명령 목록은 writer help"
        ))
    })?;
    let schema = find_tool(&tool).expect("named tools exist");
    let props = properties(&schema);
    let need = required(&schema);
    let plain = positional(&tool, &schema);
    let body = BODY.contains(&tool.as_str());

    let head: Vec<String> = plain
        .iter()
        .map(|n| {
            if need.contains(n) {
                format!("<{n}>")
            } else {
                format!("[{n}]")
            }
        })
        .collect();
    let mut lines = vec![
        format!("writer {} {}", command_of(&tool), head.join(" "))
            .trim_end()
            .to_string(),
        format!("  {}", schema["description"].as_str().unwrap_or("")),
        format!("  (MCP 도구 이름: {tool})"),
        String::new(),
    ];
    let mut opts = Vec::new();
    let mut order: Vec<(&String, &Value)> = props.iter().collect();
    order.sort_by_key(|(n, _)| {
        (
            !plain.contains(n),
            !need.contains(n),
            n.as_str() == "project",
        )
    });
    for (name, prop) in order {
        if name == "context" || (body && name == "text") {
            continue;
        }
        let label = kind_label(prop);
        let mut head = format!("--{}", name.replace('_', "-"));
        if !label.is_empty() {
            head = format!("{head} <{label}>");
        }
        let mut note = vec![];
        if need.contains(name) {
            note.push("(필수)".to_string());
        }
        note.push(prop["description"].as_str().unwrap_or("").to_string());
        if plain.contains(name) {
            note.push("— 옵션 이름 없이 차례대로 써도 된다".into());
        }
        let mut note = note
            .into_iter()
            .filter(|n| !n.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        if name == "project" {
            note = "작품 폴더. 생략하면 WRITERPROGRAM_PROJECT나 지금 폴더에서 찾는다".into();
        }
        opts.push(format!("  {head}\n      {}", note.trim()));
    }
    if !opts.is_empty() {
        lines.push("옵션:".into());
        lines.extend(opts);
    }
    if body {
        lines.push("  --file <경로>  또는  --text <글>  또는  표준 입력(파이프)".into());
        lines.push("      본문. 한 줄에 한 문단, 장면 나눔은 ***".into());
    }
    if props.contains_key("context") {
        lines.push(
            "  --pov <시점 인물> --tense <past|present> [--speech <화자>청자:말단계[:호칭]> …]"
                .into(),
        );
        lines.push(format!(
            "      맥락 메모. 회차 본문을 쓸 때 꼭 준다. 말단계: {}. --context <JSON>도 된다",
            crate::context::LEVELS.join(", ")
        ));
    }
    lines.push("  --args <JSON|->  --args-file <경로>".into());
    lines.push("      인자를 JSON 객체로 한꺼번에 (다른 옵션과 합쳐진다)".into());
    if let Some(example) = example(&tool) {
        lines.push(String::new());
        lines.push(format!("예: {example}"));
    }
    Ok(lines.join("\n"))
}

fn example(tool: &str) -> Option<&'static str> {
    Some(match tool {
        "guide" => "writer guide   /   writer guide reading",
        "read_doc" => "writer read 01HV3K…",
        "add_doc" => "writer add \"3화 · 시장\" --pov 하진 --tense past --file 3화.md",
        "write_doc" => {
            "writer write 01HV3K… append --base-rev 9f2c… --pov 하진 --tense past --speech \"하진>상인:해요체\" < 장면.md"
        }
        "check" => "writer check 01HV3K…   /   writer check < 초고.md",
        "set_meta" => "writer meta 01HV3K… --status revise",
        "save_card" => {
            "writer card --name 윤서하 --type person --aliases 서하 --fields '{\"나이\": \"29\"}'"
        }
        "add_note" => "writer note \"이 장면 끝 문장 후보 셋\" --doc-id 01HV3K…",
        "create_project" => "writer new 사람을 찾습니다 --parent ~/소설",
        "search" => "writer search 서랍",
        "save_reading" => "writer save-reading 01HV3K… --args-file 해석.json",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(s: &[&str]) -> Vec<String> {
        s.iter().map(|w| w.to_string()).collect()
    }

    fn args_of(command: &str, rest: &[&str]) -> Result<Value, Failure> {
        let tool = tool_name(command).unwrap();
        arguments(&tool, &find_tool(&tool).unwrap(), &words(rest))
    }

    #[test]
    fn commands_name_tools_by_alias_or_by_name() {
        assert_eq!(tool_name("write").as_deref(), Some("write_doc"));
        assert_eq!(tool_name("write-doc").as_deref(), Some("write_doc"));
        assert_eq!(tool_name("replace-all").as_deref(), Some("replace_all"));
        assert_eq!(tool_name("nope"), None);
        assert_eq!(command_of("read_doc"), "read");
        assert_eq!(command_of("move_doc"), "move-doc");
    }

    #[test]
    fn plain_arguments_fill_the_required_ones_in_order() {
        let a = args_of("move-doc", &["d1", "3"]).unwrap();
        assert_eq!(a, json!({"id": "d1", "index": 3}));
        assert_eq!(args_of("check", &["d1"]).unwrap(), json!({"id": "d1"}));
        assert_eq!(
            args_of("guide", &["reading"]).unwrap(),
            json!({"topic": "reading"})
        );
        let a = args_of(
            "write",
            &[
                "d1",
                "--mode=append",
                "--text",
                "비가 왔다.",
                "--allow-plain",
            ],
        )
        .unwrap();
        assert_eq!(
            a,
            json!({"id": "d1", "mode": "append", "text": "비가 왔다.", "allow_plain": true})
        );
        assert!(
            matches!(args_of("read", &["a", "b"]), Err(Failure::Usage(e)) if e.contains("너무 많음"))
        );
        assert!(
            matches!(args_of("write", &["d1", "append"]), Err(Failure::Usage(e)) if e.contains("본문"))
        );
        assert!(
            matches!(args_of("read", &["a", "--nope", "1"]), Err(Failure::Usage(e)) if e.contains("--nope"))
        );
    }

    #[test]
    fn the_context_memo_comes_from_short_options() {
        let a = args_of(
            "add",
            &[
                "3화",
                "--text",
                "“가요.”",
                "--pov",
                "하진",
                "--tense",
                "past",
                "--speech",
                "하진>상인:해요체",
                "--speech",
                "상인>하진:해체:총각",
                "--new-facts",
                "골목 이름",
            ],
        )
        .unwrap();
        assert_eq!(
            a["context"],
            json!({"pov": "하진", "tense": "past", "speech": [
                {"from": "하진", "to": "상인", "level": "해요체"},
                {"from": "상인", "to": "하진", "level": "해체", "address": "총각"},
            ]})
        );
        assert_eq!(a["new_facts"], json!(["골목 이름"]));
        assert!(args_of("add", &["3화", "--speech", "하진상인"]).is_err());
    }

    #[test]
    fn json_arguments_merge_with_the_rest() {
        let a = args_of(
            "save-reading",
            &["d1", "--args", r#"{"summary": "요약", "sentences": []}"#],
        )
        .unwrap();
        assert_eq!(a, json!({"id": "d1", "summary": "요약", "sentences": []}));
        let a = args_of(
            "card",
            &[
                "--fields",
                r#"{"나이": "29"}"#,
                "--aliases",
                "서하",
                "--aliases",
                "윤 사장",
            ],
        )
        .unwrap();
        assert_eq!(
            a,
            json!({"fields": {"나이": "29"}, "aliases": ["서하", "윤 사장"]})
        );
    }

    #[test]
    fn help_lists_every_tool_and_explains_one() {
        let all = overview();
        for t in tools::list().as_array().unwrap() {
            assert!(
                all.contains(&command_of(t["name"].as_str().unwrap())),
                "{t}"
            );
        }
        let one = command_help("write").unwrap();
        assert!(one.starts_with("writer write <id> <mode>"), "{one}");
        assert!(one.contains("--base-rev"));
        assert!(one.contains("--pov"));
        assert!(!one.contains("--text <값>"));
        assert!(
            command_help("save-reading")
                .unwrap()
                .starts_with("writer save-reading <id>\n")
        );
    }
}
