//! The kfiction writing guide (`plugins/kfiction/skills/kfiction`), built into
//! writer-mcp so that connecting the server is enough, without the plugin.
//! The plugin's files stay the one source. Only the parts that speak of the
//! plugin (its command line checker, its hook, links to reference files) are
//! rewritten for the tools here.

use std::cell::Cell;

const SKILL: &str = include_str!("../../../plugins/kfiction/skills/kfiction/SKILL.md");

/// Reference documents, by the name `guide` takes.
pub const TOPICS: [(&str, &str); 6] = [
    (
        "context",
        include_str!("../../../plugins/kfiction/skills/kfiction/references/context.md"),
    ),
    (
        "speech-levels",
        include_str!("../../../plugins/kfiction/skills/kfiction/references/speech-levels.md"),
    ),
    (
        "figurative",
        include_str!("../../../plugins/kfiction/skills/kfiction/references/figurative.md"),
    ),
    (
        "vocabulary",
        include_str!("../../../plugins/kfiction/skills/kfiction/references/vocabulary.md"),
    ),
    (
        "revision",
        include_str!("../../../plugins/kfiction/skills/kfiction/references/revision.md"),
    ),
    (
        "reading",
        include_str!("../../../plugins/kfiction/skills/kfiction/references/reading.md"),
    ),
];

/// Takes the place of the guide's "원고 점검" section, which runs the
/// plugin's checker from the command line.
const IN_WRITERPROGRAM: &str = "## WriterProgram 작품에 쓰기

- 회차에 넣을 때는 write_doc이나 add_doc에 context(시점 인물, 시제, 대사가 있으면 화자→청자 말 단계와 호칭)를 함께 보낸다. 작가에게 보인 맥락 메모와 같은 내용이다. 지어낸 이름이나 사실은 new_facts로 보낸다. 회차에 메모로 남아 작가가 앱에서 본다.
- 보낸 글에 설명·인사·마크다운이 섞이면 저장하지 않고 고칠 줄을 돌려준다. 그 줄만 고쳐 다시 보낸다.
- 저장하면 check가 온다. context는 보낸 맥락과 본문이 어긋나 보이는 곳(시제, 말 단계)이니 먼저 본다. 끝 글자로 어림한 것이라 틀릴 수 있다. warn은 문체 경고로, 작가에게 보이고 작가가 원할 때만 고친다.
- 회차를 따로 점검하려면 check 도구를 쓴다. 점검기가 보지 못하는 시점, 숨은 의도, 설정 모순은 위의 확인 목록으로 직접 본다.
";

// The server answers on one thread, so per-thread state is the session's.
thread_local! {
    static READ: Cell<bool> = const { Cell::new(false) };
    static REQUIRED: Cell<bool> = const { Cell::new(false) };
}

/// The server asks for the guide to be read before chapters are written.
pub fn require() {
    REQUIRED.set(true);
}

/// Refuses writing a chapter until the guide has been read, when required.
pub fn check_read() -> Result<(), String> {
    if REQUIRED.get() && !READ.get() {
        return Err("회차를 쓰기 전에 guide 도구로 kfiction 작문 지침을 한 번 읽어 주세요. 지침의 순서(맥락 메모, 서로 다른 후보 3개, 확인 목록)대로 쓴 글만 넣습니다.".into());
    }
    Ok(())
}

/// The guide as the assistant reads it here.
pub fn skill() -> String {
    let body = SKILL
        .strip_prefix("---\n")
        .and_then(|s| s.split_once("\n---\n"))
        .map_or(SKILL, |(_, body)| body);
    let body = match body.find("\n## 원고 점검\n") {
        Some(at) => {
            let rest = &body[at + 1..];
            let next = rest[3..].find("\n## ").map_or(rest.len(), |i| i + 4);
            format!("{}{}", &body[..at + 1], &rest[next..])
        }
        None => body.to_string(),
    };
    let mut body = body
        .replace(
            "hook이 있는 환경에서는 파일을 고치면 원고 점검이 돈다.",
            "회차에 넣으면 원고 점검이 돈다.",
        )
        .replace(
            "파일로 쓰면 hook 결과를 보고",
            "회차에 넣으면 check 결과를 보고",
        );
    for (name, _) in TOPICS {
        body = body.replace(
            &format!("[references/{name}.md](references/{name}.md)"),
            &format!("guide(\"{name}\")"),
        );
    }
    format!("{}\n\n{IN_WRITERPROGRAM}", body.trim_end())
}

/// The `guide` tool: the guide, or one reference by name.
pub fn read(topic: Option<&str>) -> Result<String, String> {
    READ.set(true);
    match topic {
        None => {
            let names: Vec<&str> = TOPICS.iter().map(|(n, _)| *n).collect();
            Ok(format!(
                "{}\n\n참고 문서: {} (guide에 topic으로 이름을 준다)",
                skill(),
                names.join(", ")
            ))
        }
        Some(name) => TOPICS
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, text)| text.to_string())
            .ok_or_else(|| format!("참고 문서가 없음: {name}")),
    }
}

/// MCP prompts, offered as slash commands by clients that show them.
pub fn prompts() -> serde_json::Value {
    serde_json::json!([
        {
            "name": "candidates",
            "description": "원고의 한 자리에 들어갈 문장·대사 후보 3개를 맥락에 맞게 낸다 (kfiction)",
            "arguments": [{"name": "memo", "description": "무엇을 쓸지 한 줄 메모", "required": false}],
        },
        {
            "name": "check",
            "description": "회차를 kfiction으로 점검하고 리포트를 보인다. 고치지 않는다",
            "arguments": [{"name": "chapter", "description": "회차 제목이나 id", "required": false}],
        },
        {
            "name": "interpret",
            "description": "회차를 해석해 문장마다 하는 일과 의도, 복선과 회수를 남긴다. 앱의 해석 모드에서 본다",
            "arguments": [{"name": "chapter", "description": "회차 제목이나 id", "required": false}],
        },
    ])
}

pub fn prompt(name: &str, args: &serde_json::Value) -> Result<String, String> {
    let arg = |key: &str| {
        args.get(key)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string()
    };
    match name {
        "candidates" => Ok(format!(
            "kfiction 지침(guide 도구, 아직 안 읽었으면 먼저 읽는다)의 순서대로 문장·대사 후보를 낸다. 작가의 메모: {}\n\n1. outline과 read_doc으로 자리를 정한다. 자리가 분명하지 않으면 어느 문장 뒤인지 한 줄로 묻는다.\n2. 맥락 메모 한 줄(시점, 시제, 이 자리의 화자→청자 말 단계와 호칭)을 먼저 보이고, 서로 다른 후보 3개와 각각의 한 줄 차이점을 낸다.\n3. 원고는 고치지 않는다. 작가가 고른 후보를 넣어 달라고 하면 그때 그 후보만 write_doc으로 넣는다.",
            arg("memo")
        )),
        "check" => Ok(format!(
            "회차 {}을(를) check 도구로 점검한다. 정하지 않았으면 가장 최근에 다룬 회차, 그것도 없으면 어느 회차인지 묻는다.\n1. 계약 위반과 경고는 줄이지 말고 보인다. 참고는 작가가 원할 때만.\n2. 점검기가 보지 못하는 것(말 단계와 관계, 시점, 숨은 의도, 설정 모순)은 지침의 확인 목록으로 직접 보고, 의심스러운 곳만 한두 줄로 말한다.\n3. 원고는 고치지 않는다. 고치려면 작가가 고를 문장을 정해 달라고 한 줄로 안내한다.",
            arg("chapter")
        )),
        "interpret" => Ok(format!(
            "회차 {}을(를) 해석한다. 정하지 않았으면 outline으로 보고 어느 회차인지 묻는다.\n1. guide와 guide(\"reading\")를 읽는다.\n2. paragraphs로 문단 번호가 붙은 본문을 읽는다.\n3. 절차대로 장면, 문장의 하는 일·의미·의도, 연결, 의도가 흐린 문장을 정리해 save_reading으로 남긴다. 인용은 본문 그대로.\n4. 작가에게는 회차 요약, 연결 몇 쌍, 흐린 문장을 짧게 보이고 앱의 해석 모드에서 볼 수 있다고 알린다. 원고는 고치지 않는다.",
            arg("chapter")
        )),
        _ => Err(format!("알 수 없는 프롬프트: {name}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_guide_speaks_of_these_tools_not_the_plugin() {
        let guide = skill();
        assert!(guide.contains("## 확인 목록"));
        assert!(guide.contains("## WriterProgram 작품에 쓰기"));
        assert!(guide.contains("guide(\"speech-levels\")"));
        for gone in [
            "references/",
            "hook",
            "CLAUDE_PLUGIN_ROOT",
            "## 원고 점검",
            "name: kfiction",
        ] {
            assert!(!guide.contains(gone), "{gone}");
        }
    }

    #[test]
    fn references_by_name() {
        assert!(read(Some("speech-levels")).unwrap().contains("해요체"));
        assert!(read(Some("없는 문서")).is_err());
    }
}
