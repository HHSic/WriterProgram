//! The context memo (맥락 메모) that comes with every chapter a tool writes:
//! the point of view, the tense, and for dialogue who speaks to whom at what
//! level. The guide has the assistant show it to the writer first; the
//! writing tools require it, then compare it with the text where characters
//! can tell: narration endings for the tense, dialogue endings for polite or
//! plain speech. Who says a line cannot be told from characters, so a scene
//! that mixes polite and plain speech gets each line's reading to check.

use serde_json::{Value, json};

use crate::check::speech::read_line;
use crate::check::text::{paragraphs, sentences};
use crate::check::{cut, ending_class};

pub const LEVELS: [&str; 6] = ["하십시오체", "해요체", "하오체", "하게체", "해체", "해라체"];
const POLITE: [&str; 3] = ["하십시오체", "해요체", "하오체"];
/// Lines of dialogue listed for checking when the scene mixes levels.
const LISTED: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tense {
    Past,
    Present,
}

#[derive(Debug, Clone)]
pub struct Pair {
    pub from: String,
    pub to: String,
    pub level: String,
    pub address: String,
}

#[derive(Debug, Clone)]
pub struct Context {
    pub pov: String,
    pub tense: Tense,
    pub speech: Vec<Pair>,
}

/// `context` in the writing tools' input schema.
pub fn schema() -> Value {
    json!({
        "type": "object",
        "description": "맥락 메모. 회차 본문을 쓸 때 꼭 보낸다. 작가에게 보인 메모와 같은 내용",
        "properties": {
            "pov": {"type": "string", "description": "시점 인물 (예: 윤재, 3인칭)"},
            "tense": {"type": "string", "enum": ["past", "present"], "description": "서술 시제: past(-었다), present(-ㄴ다)"},
            "speech": {
                "type": "array",
                "description": "보낸 글에 대사가 있으면 그 화자→청자 쌍마다 하나",
                "items": {
                    "type": "object",
                    "properties": {
                        "from": {"type": "string"},
                        "to": {"type": "string"},
                        "level": {"type": "string", "enum": LEVELS},
                        "address": {"type": "string", "description": "청자를 부르는 말 (예: 팀장님)"},
                    },
                    "required": ["from", "to", "level"],
                },
            },
        },
        "required": ["pov", "tense"],
    })
}

const FORMAT: &str = "context는 {\"pov\": \"윤재\", \"tense\": \"past\", \"speech\": [{\"from\": \"윤재\", \"to\": \"서아\", \"level\": \"해체\", \"address\": \"서아야\"}]} 꼴이다";

fn has_dialogue(text: &str) -> bool {
    paragraphs(text)
        .iter()
        .any(|p| !p.excluded && !p.dialogues.is_empty())
}

/// Reads the memo sent with `text`, or says what is missing.
pub fn parse(args: &Value, text: &str) -> Result<Context, String> {
    let c = args.get("context").ok_or_else(|| {
        format!("회차 본문을 쓸 때는 맥락 메모(context)를 함께 보낸다. 작가에게 보인 맥락 메모와 같은 내용으로. {FORMAT}")
    })?;
    let field = |v: &Value, key: &str| {
        v.get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
    };
    let pov =
        field(c, "pov").ok_or_else(|| format!("context에 시점 인물(pov)이 없음. {FORMAT}"))?;
    let tense = match field(c, "tense").as_deref() {
        Some("past") => Tense::Past,
        Some("present") => Tense::Present,
        _ => return Err(format!("context의 tense는 past나 present. {FORMAT}")),
    };
    let mut speech = Vec::new();
    for item in c
        .get("speech")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let (Some(from), Some(to), Some(level)) =
            (field(item, "from"), field(item, "to"), field(item, "level"))
        else {
            return Err(format!(
                "context.speech마다 from, to, level을 적는다. {FORMAT}"
            ));
        };
        if !LEVELS.contains(&level.as_str()) {
            return Err(format!(
                "말 단계는 {} 가운데 하나: {level}",
                LEVELS.join(", ")
            ));
        }
        speech.push(Pair {
            from,
            to,
            level,
            address: field(item, "address").unwrap_or_default(),
        });
    }
    if speech.is_empty() && has_dialogue(text) {
        return Err(format!(
            "보낸 글에 대사가 있으니 context.speech에 화자→청자 말 단계를 적는다. {FORMAT}"
        ));
    }
    Ok(Context { pov, tense, speech })
}

/// Polite or plain as a line reads; none when it cannot be read or mixes.
fn politeness(line: &str) -> Option<bool> {
    let read: Vec<Vec<&str>> = read_line(line)
        .into_iter()
        .map(|(_, levels)| levels)
        .filter(|l| !l.is_empty())
        .collect();
    if read.is_empty() {
        None
    } else if read.iter().flatten().all(|l| POLITE.contains(l)) {
        Some(true)
    } else if read.iter().flatten().all(|l| !POLITE.contains(l)) {
        Some(false)
    } else {
        None
    }
}

fn finding(kind: &str, line: usize, matched: String, hint: String) -> Value {
    json!({"kind": kind, "line": line, "match": matched, "hint": hint})
}

/// Where `text` seems to go against the memo. Lines count in `text`.
pub fn compare(ctx: &Context, text: &str) -> Vec<Value> {
    let paras: Vec<_> = paragraphs(text)
        .into_iter()
        .filter(|p| !p.excluded)
        .collect();
    let mut out = Vec::new();

    let narration: Vec<(usize, String)> = paras
        .iter()
        .flat_map(|p| {
            sentences(&p.narration)
                .into_iter()
                .map(move |s| (p.line, s))
        })
        .collect();
    let (want, other, name, other_name) = match ctx.tense {
        Tense::Past => ("-었다", "-ㄴ다", "과거형(-었다)", "현재형(-ㄴ다)"),
        Tense::Present => ("-ㄴ다", "-었다", "현재형(-ㄴ다)", "과거형(-었다)"),
    };
    let wanted = narration
        .iter()
        .filter(|(_, s)| ending_class(s) == want)
        .count();
    let off: Vec<&(usize, String)> = narration
        .iter()
        .filter(|(_, s)| ending_class(s) == other)
        .collect();
    if off.len() >= 2 && off.len() * 10 > (wanted + off.len()) * 3 {
        out.push(finding(
            "시제",
            off[0].0,
            cut(&off[0].1),
            format!(
                "맥락은 {name}인데 {other_name}으로 끝난 서술이 {}문장. 의도한 것이 아니면 맞춘다",
                off.len()
            ),
        ));
    }

    let declared: Vec<bool> = ctx
        .speech
        .iter()
        .map(|p| POLITE.contains(&p.level.as_str()))
        .collect();
    let dialogue: Vec<(usize, &String)> = paras
        .iter()
        .flat_map(|p| p.dialogues.iter().map(move |d| (p.line, d)))
        .collect();
    let all_polite = !declared.is_empty() && declared.iter().all(|&p| p);
    let all_plain = !declared.is_empty() && declared.iter().all(|&p| !p);
    if all_polite || all_plain {
        let pairs: Vec<String> = ctx
            .speech
            .iter()
            .map(|p| format!("{}→{} {}", p.from, p.to, p.level))
            .collect();
        for (line, d) in &dialogue {
            if politeness(d) == Some(!all_polite) {
                let read = if all_polite { "안높임" } else { "높임" };
                out.push(finding(
                    "말 단계",
                    *line,
                    cut(d),
                    format!(
                        "맥락({})과 달리 {read}으로 읽힘. 끝 글자 어림이라 틀릴 수 있다",
                        pairs.join(", ")
                    ),
                ));
            }
        }
    } else if !declared.is_empty() {
        for (line, d) in dialogue.iter().take(LISTED) {
            let read: Vec<String> = read_line(d)
                .into_iter()
                .filter(|(_, l)| !l.is_empty())
                .map(|(_, l)| l.join("/"))
                .collect();
            if read.is_empty() {
                continue;
            }
            out.push(finding(
                "말 단계 어림",
                *line,
                cut(d),
                format!(
                    "{}로 읽힘. 이 대사의 화자→청자 쌍과 맞는지 확인",
                    read.join(" · ")
                ),
            ));
        }
    }
    out
}

/// The memo as one line, for notes.
pub fn line(ctx: &Context) -> String {
    let tense = match ctx.tense {
        Tense::Past => "과거형",
        Tense::Present => "현재형",
    };
    let mut parts = vec![format!("시점 {}", ctx.pov), tense.to_string()];
    for p in &ctx.speech {
        let address = if p.address.is_empty() {
            String::new()
        } else {
            format!(", \"{}\"", p.address)
        };
        parts.push(format!("{}→{} {}{address}", p.from, p.to, p.level));
    }
    parts.join(" · ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(tense: &str, speech: Value) -> Context {
        parse(
            &json!({"context": {"pov": "윤재", "tense": tense, "speech": speech}}),
            "",
        )
        .unwrap()
    }

    #[test]
    fn the_memo_is_required_with_its_parts() {
        let e = parse(&json!({}), "비가 왔다.").unwrap_err();
        assert!(e.contains("맥락 메모"), "{e}");
        let e = parse(
            &json!({"context": {"pov": "윤재", "tense": "past"}}),
            "“가자.”",
        )
        .unwrap_err();
        assert!(e.contains("speech"), "{e}");
        let e = parse(&json!({"context": {"pov": "윤재", "tense": "past", "speech": [{"from": "윤재", "to": "서아", "level": "반말"}]}}), "").unwrap_err();
        assert!(e.contains("말 단계는"), "{e}");
        assert!(
            parse(
                &json!({"context": {"pov": "윤재", "tense": "past"}}),
                "비가 왔다."
            )
            .is_ok()
        );
    }

    #[test]
    fn narration_in_the_other_tense_is_noticed() {
        let found = compare(
            &ctx("past", json!([])),
            "문을 연다. 비가 온다. 길이 젖었다.",
        );
        assert_eq!(found[0]["kind"], "시제");
        assert!(
            compare(
                &ctx("past", json!([])),
                "문을 열었다. 비가 왔다. 길이 젖는다."
            )
            .is_empty()
        );
        assert_eq!(
            compare(&ctx("present", json!([])), "문을 열었다. 비가 왔다.")[0]["kind"],
            "시제"
        );
    }

    #[test]
    fn dialogue_against_the_declared_level_is_noticed() {
        let plain = ctx(
            "past",
            json!([{"from": "윤재", "to": "서아", "level": "해체"}]),
        );
        let found = compare(&plain, "“집에 가.”\n\n“내일 봬요.”");
        assert_eq!(found.len(), 1);
        assert_eq!(
            (found[0]["kind"].as_str(), found[0]["line"].as_u64()),
            (Some("말 단계"), Some(3))
        );

        let polite = ctx(
            "past",
            json!([{"from": "윤재", "to": "손님", "level": "해요체"}]),
        );
        assert_eq!(
            compare(&polite, "“이거 어디 열쇠예요?”\n\n“그냥 줘.”").len(),
            1
        );

        let mixed = ctx(
            "past",
            json!([{"from": "윤재", "to": "어머니", "level": "해요체"}, {"from": "어머니", "to": "윤재", "level": "해라체"}]),
        );
        let found = compare(&mixed, "“엄마, 이거요.”\n\n“그런 열쇠는 안 받아.”");
        assert_eq!(found.len(), 2);
        assert!(found.iter().all(|f| f["kind"] == "말 단계 어림"));
    }

    #[test]
    fn one_line_memo() {
        let c = ctx(
            "past",
            json!([{"from": "윤재", "to": "서아", "level": "해체", "address": "서아야"}]),
        );
        assert_eq!(line(&c), "시점 윤재 · 과거형 · 윤재→서아 해체, \"서아야\"");
    }
}
