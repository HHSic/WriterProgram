//! writer-mcp: lets an AI assistant read and write WriterProgram project
//! folders, as an MCP server or from the command line.
//!
//! ```text
//! writer-mcp serve [--project <작품 폴더>]          MCP 서버 (stdio)
//! writer-mcp tools                                  도구 목록
//! writer-mcp call <도구> [<JSON 인자>|-] [--project <작품 폴더>]
//! ```
//!
//! The server speaks JSON-RPC 2.0, one message per line on stdin and stdout
//! (the MCP stdio transport). Only tools are offered.

mod check;
mod context;
mod guide;
mod manage;
mod review;
mod text;
mod tools;

use std::io::{self, BufRead, Read, Write};
use std::path::PathBuf;

use serde_json::{Value, json};

const USAGE: &str = "사용법:
  writer-mcp serve [--project <작품 폴더>]
  writer-mcp tools
  writer-mcp call <도구> [<JSON 인자>|-] [--project <작품 폴더>]";

/// Protocol version answered when the client does not name one.
const PROTOCOL_VERSION: &str = "2025-06-18";

/// Told to the assistant on `initialize`: how to use the tools with a writer.
const INSTRUCTIONS: &str = "WriterProgram 작품 폴더를 읽고 쓰는 도구다. 작가가 앱으로 함께 연다.
- 한국어 소설을 쓰거나 고치거나 후보를 내기 전에 guide로 kfiction 작문 지침을 읽고 따른다. 읽기 전에는 회차를 쓸 수 없다.
- 먼저 outline으로 작품 구조를 본다. 회차 본문은 read_doc으로 읽는다.
- 원고 본문은 작가가 넣어 달라거나 쓰라고 했을 때만 add_doc, write_doc으로 쓴다. 후보와 제안은 채팅으로 보이거나 add_note로 메모에 남긴다.
- write_doc 전에는 read_doc으로 읽고 그 rev를 base_rev로 넘긴다. 고치기 직전 본문은 '지금 원고 보관' 기록으로 남아 앱에서 되돌릴 수 있다.
- 본문은 한 줄에 한 문단, 장면 나눔은 ***. 본문에 설명, 인사, 마크다운을 넣지 않는다.
- 잠근 회차는 고치지 않는다. 서식이 있는 회차를 replace로 바꾸려면 작가에게 서식이 사라져도 되는지 먼저 묻는다.
- trash_doc, replace_all, restore_record, export는 작가가 시켰을 때만 쓴다. replace_all 전에는 search로 몇 곳인지 보인다.
- write_doc, add_doc은 보낸 글에 설명·인사·마크다운이 섞이면 저장하지 않고 고칠 줄을 돌려준다. 그 줄만 고쳐 다시 보낸다. 저장 뒤 돌아오는 check의 경고는 작가에게 보이고, 작가가 원할 때만 고친다.
- 잘못 고쳤으면 records로 기록을 찾아 restore_record로 되돌린다. 지운 문서는 trash와 restore_trash로 되살린다.";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(match run(&args) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{e}");
            2
        }
    });
}

fn run(args: &[String]) -> Result<(), String> {
    let mut project = None;
    let mut rest = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == "--project" {
            project = Some(PathBuf::from(
                it.next().ok_or("--project 뒤에 폴더를 주세요")?,
            ));
        } else {
            rest.push(a.as_str());
        }
    }
    let project = project.or_else(|| std::env::var_os("WRITERPROGRAM_PROJECT").map(PathBuf::from));
    match rest.as_slice() {
        ["serve"] => serve(project),
        ["tools"] => {
            for t in tools::list().as_array().expect("a list") {
                println!(
                    "{}\n  {}",
                    t["name"].as_str().unwrap_or(""),
                    t["description"].as_str().unwrap_or("")
                );
            }
            Ok(())
        }
        ["call", name, more @ ..] => {
            let args = match more {
                [] => json!({}),
                ["-"] => {
                    let mut s = String::new();
                    io::stdin()
                        .read_to_string(&mut s)
                        .map_err(|e| e.to_string())?;
                    serde_json::from_str(&s).map_err(|e| format!("JSON 인자를 읽지 못함: {e}"))?
                }
                [json] => {
                    serde_json::from_str(json).map_err(|e| format!("JSON 인자를 읽지 못함: {e}"))?
                }
                _ => return Err(USAGE.into()),
            };
            let out = tools::call(name, &args, project.as_deref())?;
            println!("{}", tools::as_text(&out));
            Ok(())
        }
        _ => Err(USAGE.into()),
    }
}

fn serve(project: Option<PathBuf>) -> Result<(), String> {
    guide::require();
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line.map_err(|e| e.to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(reply) = handle(&line, project.as_deref()) {
            writeln!(stdout, "{reply}").map_err(|e| e.to_string())?;
            stdout.flush().map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Answers one JSON-RPC message. Notifications (no id) get no answer.
fn handle(line: &str, project: Option<&std::path::Path>) -> Option<String> {
    let msg: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => return Some(error(Value::Null, -32700, &format!("parse error: {e}"))),
    };
    let id = msg.get("id").cloned()?;
    let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
    let params = msg.get("params").cloned().unwrap_or(json!({}));
    let result = match method {
        "initialize" => json!({
            "protocolVersion": params.get("protocolVersion").and_then(Value::as_str).unwrap_or(PROTOCOL_VERSION),
            "capabilities": {"tools": {"listChanged": false}, "prompts": {"listChanged": false}},
            "serverInfo": {"name": "writerprogram", "version": env!("CARGO_PKG_VERSION")},
            "instructions": INSTRUCTIONS,
        }),
        "ping" => json!({}),
        "tools/list" => json!({"tools": tools::list()}),
        "prompts/list" => json!({"prompts": guide::prompts()}),
        "prompts/get" => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            match guide::prompt(name, &args) {
                Ok(text) => json!({
                    "messages": [{"role": "user", "content": {"type": "text", "text": text}}],
                }),
                Err(e) => return Some(error(id, -32602, &e)),
            }
        }
        "tools/call" => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            match tools::call(name, &args, project) {
                Ok(out) => json!({
                    "content": [{"type": "text", "text": tools::as_text(&out)}],
                    "isError": false,
                }),
                Err(e) => json!({"content": [{"type": "text", "text": e}], "isError": true}),
            }
        }
        _ => return Some(error(id, -32601, &format!("method not found: {method}"))),
    };
    Some(json!({"jsonrpc": "2.0", "id": id, "result": result}).to_string())
}

fn error(id: Value, code: i64, message: &str) -> String {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}}).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ask(line: &str) -> Value {
        serde_json::from_str(&handle(line, None).expect("an answer")).unwrap()
    }

    #[test]
    fn initialize_lists_tools_and_instructions() {
        let r = ask(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26"}}"#,
        );
        assert_eq!(r["result"]["protocolVersion"], "2025-03-26");
        assert!(
            r["result"]["instructions"]
                .as_str()
                .unwrap()
                .contains("base_rev")
        );
        let r = ask(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#);
        let names: Vec<_> = r["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap().to_string())
            .collect();
        assert!(names.contains(&"write_doc".to_string()));
        for t in r["result"]["tools"].as_array().unwrap() {
            assert_eq!(t["inputSchema"]["type"], "object");
        }
    }

    #[test]
    fn notifications_get_no_answer_and_errors_are_tool_results() {
        assert!(
            handle(
                r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
                None
            )
            .is_none()
        );
        let r = ask(
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"outline","arguments":{"project":"/없는/폴더"}}}"#,
        );
        assert_eq!(r["result"]["isError"], true);
        let r = ask(r#"{"jsonrpc":"2.0","id":4,"method":"nope"}"#);
        assert_eq!(r["error"]["code"], -32601);
        let r = ask("not json");
        assert_eq!(r["error"]["code"], -32700);
    }
}
