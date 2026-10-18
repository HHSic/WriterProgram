//! What the model is given under each condition (docs/context-sentences.md 8).
//!
//! - X0: the manuscript before the slot and the task;
//! - X1: X0 plus character cards as the setting book has them today (one 말투 field);
//! - X2: X1 plus the relation table and the scene card.
//!
//! Long material comes first and the task last, each block in a tag.

use std::fmt::Write;

use crate::set::Item;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Condition {
    X0,
    X1,
    X2,
}

impl Condition {
    pub fn parse(s: &str) -> Option<Condition> {
        match s.to_ascii_lowercase().as_str() {
            "x0" => Some(Condition::X0),
            "x1" => Some(Condition::X1),
            "x2" => Some(Condition::X2),
            _ => None,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Condition::X0 => "x0",
            Condition::X1 => "x1",
            Condition::X2 => "x2",
        }
    }
}

pub const SYSTEM: &str = "너는 한국 소설 작가의 집필을 돕는다. 작가가 고른 자리에 들어갈 대사 후보를 쓴다.
주어진 원고와 자료만 근거로 삼는다. 자료에 없는 사건과 사실을 더하지 않는다.
후보는 3개이고 서로 다른 선택을 담는다. 각 후보는 큰따옴표로 감싼 대사 한두 문장이며, 필요하면 대사 앞이나 뒤에 짧은 행동 묘사 한 문장을 붙일 수 있다.
출력은 JSON 객체 하나뿐이다. 형식: {\"candidates\": [{\"text\": \"후보\", \"angle\": \"다른 후보와 다른 점 한 줄\"}], \"new_facts\": [\"새로 지어낸 이름이나 사실, 없으면 빈 배열\"]}
설명, 머리말, 마크다운은 쓰지 않는다.";

pub fn user_message(item: &Item, condition: Condition) -> String {
    let scene = item.scene;
    let slot = item.slot;
    let mut out = String::new();

    if condition != Condition::X0 {
        out.push_str("<인물>\n");
        for c in &scene.characters {
            let _ = write!(out, "- {}", c.name);
            if !c.aliases.is_empty() {
                let _ = write!(out, " (다른 이름: {})", c.aliases.join(", "));
            }
            let _ = writeln!(out, "\n  신분: {}\n  말투: {}", c.role, c.speech);
            if !c.lines.is_empty() {
                let lines: Vec<String> = c.lines.iter().map(|l| format!("“{l}”")).collect();
                let _ = writeln!(out, "  대사 예: {}", lines.join(" "));
            }
        }
        out.push_str("</인물>\n\n");
    }

    if condition == Condition::X2 {
        out.push_str(
            "<관계>\n| 화자 → 청자 | 말 단계 | 호칭 | 자칭 | 바뀌는 때 |\n|---|---|---|---|---|\n",
        );
        for r in &scene.relations {
            let switch: Vec<String> = r
                .switch
                .iter()
                .map(|s| {
                    let address = if s.address.is_empty() {
                        "호칭 없음".to_string()
                    } else {
                        s.address.join(", ")
                    };
                    format!("{}: {}, {}", s.when, s.level, address)
                })
                .collect();
            let _ = writeln!(
                out,
                "| {} → {} | {} | {} | {} | {} |",
                r.from,
                r.to,
                r.level,
                if r.address.is_empty() {
                    "-".to_string()
                } else {
                    r.address.join(", ")
                },
                r.self_ref.as_deref().unwrap_or("-"),
                if switch.is_empty() {
                    "-".to_string()
                } else {
                    switch.join(" / ")
                },
            );
        }
        out.push_str("</관계>\n\n");
    }

    out.push_str("<직전_원고>\n");
    out.push_str(scene.before.trim());
    if let Some(extra) = &slot.extra {
        out.push('\n');
        out.push_str(extra.trim());
    }
    out.push_str("\n</직전_원고>\n\n");

    if condition == Condition::X2 {
        let card = &scene.scene;
        let _ = writeln!(
            out,
            "<장면_카드>\n시점 인물: {}\n시제: {}\n장소: {}\n시간: {}\n장면 목표: {}",
            card.pov, card.tense, card.place, card.time, card.goal
        );
        if !card.beats.is_empty() {
            let _ = writeln!(out, "흐름: {}", card.beats.join(" → "));
        }
        if let Some(note) = &card.switch_note {
            let _ = writeln!(out, "말 단계 전환: {note}");
        }
        if let Some(hidden) = &slot.hidden {
            let _ = writeln!(
                out,
                "이 대사의 숨은 의도 (대사로 직접 말하지 않음): {hidden}"
            );
        }
        out.push_str("</장면_카드>\n\n");
    }

    let _ = writeln!(
        out,
        "<과제>\n직전 원고 바로 다음에 들어갈 대사 후보 3개를 써라.\n화자: {}\n청자: {}\n대사가 할 일: {}",
        slot.speaker, slot.listener, slot.note
    );
    if condition == Condition::X2
        && let Some(r) = item.relation(&slot.speaker, &slot.listener)
    {
        let address = if r.address.is_empty() {
            "-".to_string()
        } else {
            r.address.join(", ")
        };
        let _ = writeln!(
            out,
            "이 두 사람의 관계 표: {} → {}, {}, 호칭 {}, 자칭 {}. 장면 카드의 전환 조건이 있으면 그것을 따른다.",
            r.from,
            r.to,
            r.level,
            address,
            r.self_ref.as_deref().unwrap_or("-")
        );
    }
    out.push_str("</과제>");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::set::{self, Scene};

    fn scenes() -> Vec<Scene> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../evals/context-sentences/R.json");
        set::load(&path).expect("R set loads")
    }

    #[test]
    fn conditions_add_blocks_in_order() {
        let scenes = scenes();
        let items = set::items(&scenes);
        let item = items.iter().find(|i| i.id() == "R12-3").unwrap();
        let x0 = user_message(item, Condition::X0);
        let x1 = user_message(item, Condition::X1);
        let x2 = user_message(item, Condition::X2);
        assert!(!x0.contains("<인물>") && !x0.contains("<관계>"));
        assert!(x1.contains("<인물>") && !x1.contains("<관계>") && !x1.contains("숨은 의도"));
        assert!(
            x2.contains("<관계>")
                && x2.contains("<장면_카드>")
                && x2.contains("둘만 있을 때: 해체")
        );
        // The slot's extra text follows the shared manuscript, and the task comes last.
        assert!(x0.contains("넥타이를 느슨하게"));
        assert!(x2.trim_end().ends_with("</과제>"));
        assert!(x2.find("<관계>").unwrap() < x2.find("<직전_원고>").unwrap());
    }

    #[test]
    fn gold_lines_never_reach_the_model() {
        let scenes = scenes();
        for item in set::items(&scenes) {
            let x2 = user_message(&item, Condition::X2);
            assert!(!x2.contains(&item.slot.gold), "{}", item.id());
        }
    }
}
