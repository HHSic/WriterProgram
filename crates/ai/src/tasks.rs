//! What is sent for each kind of help, and reading what comes back.
//!
//! - 회차 요약: the chapter's text, names masked; back come 2–4 sentences.
//! - 설정 모순 점검: the chapter and the setting cards whose names appear in
//!   it, names masked; back comes a JSON list of `{quote, card, problem}`,
//!   each quote found again in the chapter so the screen can jump to it.
//!
//! The instructions say plainly that the AI must not rewrite or continue the
//! manuscript. Nothing built here is written anywhere: the screen shows it
//! before it is sent, and forgets it afterwards.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use writer_core::cards::{self, Card, names_regex};
use writer_core::markup::{Block, inline_text};
use writer_core::search::{Match, find_in_blocks};
use writer_core::{doc, project};

use crate::client::{Answer, Ask, Usage};
use crate::mask::{Masker, Swap};
use crate::{Error, Result};

/// Longest chapter text sent at once.
pub const MAX_CHARS: usize = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Task {
    /// 회차 요약.
    Summary,
    /// 설정 모순 점검.
    Check,
}

const SUMMARY_SYSTEM: &str = "당신은 소설가의 원고 정리를 돕는 도우미입니다. 원고를 고쳐 쓰거나 이어 쓰거나 새 문장을 지어내지 않습니다.
주어진 회차 본문의 줄거리를 2~4문장으로 요약하세요. 작가가 참고용으로 볼 요약입니다.
- 본문에 있는 일만 쓰고, 평가나 조언은 붙이지 마세요.
- 인물·장소·용어 이름은 '인물A', '장소B'처럼 가려져 있습니다. 그 표기를 그대로 쓰세요.
- 요약 문장만 쓰세요. 제목, 머리말, 따옴표, 목록 기호는 붙이지 마세요.";

const CHECK_SYSTEM: &str = "당신은 소설가의 설정 점검을 돕는 도우미입니다. 원고를 고쳐 쓰거나 이어 쓰거나 고친 문장을 제안하지 않습니다.
[설정 카드]와 [회차 본문]을 견주어, 본문이 카드에 적힌 설정과 어긋나는 곳만 찾으세요. 예: 나이, 눈 색·머리색 같은 외모, 호칭과 말투, 능력, 관계, 장소의 특징.
- 카드에 적혀 있지 않은 내용은 어긋남으로 보지 마세요. 확실한 것만 적으세요.
- 이름은 '인물A', '장소B'처럼 가려져 있습니다. '인물A2'처럼 숫자가 붙은 것은 같은 카드의 다른 이름입니다. 그 표기를 그대로 쓰세요.
- quote에는 본문에서 어긋난 부분을 한 글자도 바꾸지 말고 짧게(한 문장 이내) 그대로 옮기세요.
- 결과는 다음 모양의 JSON 하나로만 답하세요: {\"problems\":[{\"quote\":\"본문 그대로의 짧은 구절\",\"card\":\"카드 이름(예: 인물A)\",\"problem\":\"카드와 무엇이 어떻게 다른지 한 문장\"}]}
- 어긋난 곳이 없으면 {\"problems\":[]} 로 답하세요.";

/// A question ready to send, with what is needed to read the answer.
pub struct Outgoing {
    pub task: Task,
    pub doc_id: String,
    pub title: String,
    pub ask: Ask,
    pub masker: Masker,
    /// Names of the setting cards sent along (설정 모순 점검).
    pub cards: Vec<String>,
    body: Vec<Block>,
}

/// What the writer sees before anything is sent.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub task: Task,
    pub doc_id: String,
    pub title: String,
    /// The instructions sent with it (no manuscript in them).
    pub instructions: String,
    /// The text exactly as it will be sent, names masked.
    pub text: String,
    /// Which names were swapped for which stand-ins.
    pub swaps: Vec<Swap>,
    pub cards: Vec<String>,
    /// Characters sent in all (instructions and text).
    pub chars: usize,
}

/// A chapter as plain text: one paragraph per line, scene breaks as `* * *`.
fn chapter_text(body: &[Block]) -> String {
    body.iter()
        .filter_map(|b| match b {
            Block::SceneBreak {} => Some("* * *".to_string()),
            Block::Paragraph { content, .. } => {
                let text = inline_text(content);
                (!text.trim().is_empty()).then_some(text)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// A card as lines for the AI (before masking).
fn card_text(card: &Card, kind: &str) -> String {
    let mut lines = vec![format!("### {} ({kind})", card.name.trim())];
    let others: Vec<&str> = card
        .aliases
        .iter()
        .map(|a| a.trim())
        .filter(|a| !a.is_empty())
        .collect();
    if !others.is_empty() {
        lines.push(format!("다른 이름: {}", others.join(", ")));
    }
    for (k, v) in &card.fields {
        if !v.trim().is_empty() {
            lines.push(format!("{}: {}", k.trim(), v.trim()));
        }
    }
    if !card.description.trim().is_empty() {
        lines.push(format!("설명: {}", card.description.trim()));
    }
    lines.join("\n")
}

/// The cards whose names appear in `text`, the first one met first (so the
/// first person in the chapter becomes `인물A`).
fn in_order_met(all: &[Card], text: &str) -> Vec<Card> {
    let mut met: Vec<(usize, &Card)> = all
        .iter()
        .filter_map(|c| {
            let re = names_regex(&c.names())?;
            re.find(text).map(|m| (m.start(), c))
        })
        .collect();
    met.sort_by_key(|(at, _)| *at);
    met.into_iter().map(|(_, c)| c.clone()).collect()
}

/// Builds the question for `task` on one chapter, from what is on disk.
pub fn prepare(root: &Path, task: Task, doc_id: &str) -> Result<Outgoing> {
    let file = doc::load(root, doc_id)?;
    let text = chapter_text(&file.body);
    if text.trim().is_empty() {
        return Err(Error::Invalid("이 회차는 비어 있어 보낼 글이 없음".into()));
    }
    if text.chars().count() > MAX_CHARS {
        return Err(Error::Invalid(format!(
            "회차가 너무 길어 한 번에 보낼 수 없음 ({}만 자까지)",
            MAX_CHARS / 10_000
        )));
    }
    let all = cards::load_all(root)?;
    let related = in_order_met(&all, &text);
    let (system, user, masker, sent_cards, max_tokens) = match task {
        Task::Summary => {
            let masker = Masker::new(&related, &[&text]);
            let user = format!("[회차 본문]\n{}", masker.mask(&text));
            (SUMMARY_SYSTEM, user, masker, Vec::new(), 4_000)
        }
        Task::Check => {
            let types = project::load(root)?.card_types();
            let kind_name = |id: &str| {
                types
                    .iter()
                    .find(|t| t.id == id)
                    .map(|t| t.name.clone())
                    .unwrap_or_else(|| "설정".into())
            };
            if related.is_empty() {
                return Err(Error::Invalid(
                    "이 회차에 나오는 설정 카드가 없어 견줄 것이 없음. 설정집에 인물·장소를 먼저 적어 주세요."
                        .into(),
                ));
            }
            let card_texts: Vec<String> = related
                .iter()
                .map(|c| card_text(c, &kind_name(&c.card_type)))
                .collect();
            let mut seen: Vec<&str> = vec![&text];
            seen.extend(card_texts.iter().map(String::as_str));
            // Cards in the chapter first, so they get the first letters;
            // others named only in their descriptions come after.
            let mut ordered = related.clone();
            ordered.extend(
                all.iter()
                    .filter(|c| !related.iter().any(|r| r.id == c.id))
                    .cloned(),
            );
            let masker = Masker::new(&ordered, &seen);
            let user = format!(
                "[설정 카드]\n{}\n\n[회차 본문]\n{}",
                masker.mask(&card_texts.join("\n\n")),
                masker.mask(&text)
            );
            let names = related.iter().map(|c| c.name.trim().to_string()).collect();
            (CHECK_SYSTEM, user, masker, names, 16_000)
        }
    };
    Ok(Outgoing {
        task,
        doc_id: doc_id.to_string(),
        title: file.meta.title,
        ask: Ask {
            system: system.to_string(),
            user,
            max_tokens,
            json: task == Task::Check,
        },
        masker,
        cards: sent_cards,
        body: file.body,
    })
}

impl Outgoing {
    /// Characters sent in all.
    pub fn chars(&self) -> usize {
        self.ask.system.chars().count() + self.ask.user.chars().count()
    }

    pub fn preview(&self) -> Preview {
        Preview {
            task: self.task,
            doc_id: self.doc_id.clone(),
            title: self.title.clone(),
            instructions: self.ask.system.clone(),
            text: self.ask.user.clone(),
            swaps: self.masker.swaps().to_vec(),
            cards: self.cards.clone(),
            chars: self.chars(),
        }
    }
}

/// 회차 요약's result.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub doc_id: String,
    pub text: String,
    pub usage: Option<Usage>,
    pub sent_chars: usize,
}

pub fn read_summary(out: &Outgoing, answer: Answer) -> Result<Summary> {
    let text = out.masker.unmask(answer.text.trim());
    let text = text
        .trim()
        .trim_matches(|c| matches!(c, '"' | '“' | '”'))
        .trim()
        .to_string();
    if text.is_empty() {
        return Err(Error::Unreadable("empty summary".into()));
    }
    Ok(Summary {
        doc_id: out.doc_id.clone(),
        text,
        usage: answer.usage,
        sent_chars: out.chars(),
    })
}

/// One place where the chapter and a card disagree.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub quote: String,
    pub card: String,
    pub problem: String,
    /// Where the quote is in the chapter, if it was found again.
    pub place: Option<Match>,
}

/// 설정 모순 점검's result.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    pub doc_id: String,
    pub findings: Vec<Finding>,
    pub cards: Vec<String>,
    pub usage: Option<Usage>,
    pub sent_chars: usize,
}

/// The JSON in an answer, even when wrapped in a code fence or a sentence.
fn json_in(text: &str) -> Option<Value> {
    if let Ok(v) = serde_json::from_str(text.trim()) {
        return Some(v);
    }
    let start = text.find(['{', '['])?;
    let end = text.rfind(['}', ']'])?;
    (end > start)
        .then(|| serde_json::from_str(&text[start..=end]).ok())
        .flatten()
}

fn field(item: &Value, key: &str) -> String {
    item[key].as_str().unwrap_or_default().trim().to_string()
}

pub fn read_check(out: &Outgoing, answer: Answer) -> Result<Check> {
    let v = json_in(&answer.text).ok_or_else(|| {
        Error::Unreadable(if answer.cut {
            "answer cut short".into()
        } else {
            "no json".into()
        })
    })?;
    let items = match &v {
        Value::Array(items) => items.clone(),
        Value::Object(map) => map
            .get("problems")
            .and_then(Value::as_array)
            .cloned()
            .or_else(|| map.values().find_map(|x| x.as_array().cloned()))
            .unwrap_or_default(),
        _ => Vec::new(),
    };
    let findings = items
        .iter()
        .filter_map(|item| {
            let problem = field(item, "problem");
            if problem.is_empty() {
                return None;
            }
            let quote = out.masker.unmask_exact(&field(item, "quote"));
            let card = out.masker.unmask_exact(&field(item, "card"));
            let card = out
                .masker
                .swaps()
                .iter()
                .find(|s| s.name == card)
                .map(|s| s.card.clone())
                .unwrap_or(card);
            Some(Finding {
                place: locate(&out.body, &quote),
                quote,
                card,
                problem: out.masker.unmask(&problem),
            })
        })
        .collect();
    Ok(Check {
        doc_id: out.doc_id.clone(),
        findings,
        cards: out.cards.clone(),
        usage: answer.usage,
        sent_chars: out.chars(),
    })
}

/// Finds a quote in the chapter: as written, then with any spacing.
pub fn locate(body: &[Block], quote: &str) -> Option<Match> {
    let quote = quote
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())?
        .trim_matches(|c: char| matches!(c, '"' | '\'' | '…') || c.is_whitespace());
    if quote.chars().count() < 2 {
        return None;
    }
    let exact = regex::Regex::new(&regex::escape(quote)).ok()?;
    if let Some(m) = find_in_blocks(body, &exact, 1).pop() {
        return Some(m);
    }
    let spaced = quote
        .split_whitespace()
        .map(regex::escape)
        .collect::<Vec<_>>()
        .join(r"\s*");
    let loose = regex::Regex::new(&spaced).ok()?;
    find_in_blocks(body, &loose, 1).pop()
}

#[cfg(test)]
mod tests {
    use super::*;
    use writer_core::cards::Card;

    fn masker() -> Masker {
        let card = Card {
            id: "p".into(),
            card_type: "person".into(),
            name: "윤서하".into(),
            aliases: vec!["서하".into()],
            fields: vec![("눈 색".into(), "검은색".into())],
            highlight: true,
            created: String::new(),
            description: String::new(),
            extra: Vec::new(),
        };
        Masker::new(&[card], &["서하"])
    }

    fn outgoing(body: Vec<Block>) -> Outgoing {
        Outgoing {
            task: Task::Check,
            doc_id: "d".into(),
            title: "t".into(),
            ask: Ask {
                system: String::new(),
                user: String::new(),
                max_tokens: 1,
                json: true,
            },
            masker: masker(),
            cards: vec!["윤서하".into()],
            body,
        }
    }

    #[test]
    fn findings_come_back_with_names_and_places() {
        let out = outgoing(vec![
            Block::text("첫 문단."),
            Block::text("서하가 회색 눈을 깜빡였다."),
        ]);
        let answer = Answer {
            text: "```json\n{\"problems\":[{\"quote\":\"인물A2가 회색 눈을\",\"card\":\"인물A\",\"problem\":\"인물A은 눈 색이 검은색인데 회색으로 나옴\"},{\"quote\":\"없는 말\",\"card\":\"인물A\",\"problem\":\"x\"},{\"quote\":\"q\"}]}\n```".into(),
            usage: None,
            cut: false,
        };
        let check = read_check(&out, answer).unwrap();
        assert_eq!(check.findings.len(), 2);
        let f = &check.findings[0];
        assert_eq!(f.quote, "서하가 회색 눈을");
        assert_eq!(f.card, "윤서하");
        assert_eq!(f.problem, "윤서하는 눈 색이 검은색인데 회색으로 나옴");
        let place = f.place.as_ref().unwrap();
        assert_eq!((place.block, place.start, place.end), (1, 0, 9));
        assert!(check.findings[1].place.is_none());
    }

    #[test]
    fn a_bare_list_and_odd_spacing_are_read() {
        let out = outgoing(vec![Block::text("서하는  웃었다.")]);
        let answer = Answer {
            text: "[{\"quote\":\"인물A2는 웃었다\",\"card\":\"인물A2\",\"problem\":\"p\"}]".into(),
            usage: None,
            cut: false,
        };
        let check = read_check(&out, answer).unwrap();
        assert_eq!(check.findings[0].card, "윤서하");
        assert!(check.findings[0].place.is_some());
    }

    #[test]
    fn unreadable_answers_say_so() {
        let out = outgoing(Vec::new());
        let answer = Answer {
            text: "모르겠습니다".into(),
            usage: None,
            cut: true,
        };
        assert!(matches!(
            read_check(&out, answer),
            Err(Error::Unreadable(_))
        ));
    }

    #[test]
    fn summaries_get_their_names_back() {
        let out = outgoing(Vec::new());
        let s = read_summary(
            &out,
            Answer {
                text: "\"인물A2은 손님을 맞는다.\"".into(),
                usage: Some(Usage {
                    input_tokens: Some(1),
                    output_tokens: Some(2),
                }),
                cut: false,
            },
        )
        .unwrap();
        assert_eq!(s.text, "서하는 손님을 맞는다.");
    }
}
