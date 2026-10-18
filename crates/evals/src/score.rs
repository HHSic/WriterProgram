//! Automatic scoring of candidates (docs/context-sentences.md 7.1, 7.5).
//!
//! Each candidate gets points out of 100 from checks a machine can make:
//!
//! | part | points | how |
//! |---|---|---|
//! | contract | gate | meta sentences, markdown, Latin letters or no line at all score 0 |
//! | speech level | 40 | matches 40, unread 20, mismatch 0 |
//! | address | 20 | no vocative meant for another relation |
//! | self-reference | 20 | 저/나 as the slot allows |
//! | style | 20 | minus 10 for each in-text revise warning |
//!
//! The item's score is its best candidate, since the writer picks one.
//! How good the line is as writing is left to people (7.2).

use std::collections::HashSet;

use writer_revise::speech::{self, Verdict};
use writer_revise::{Analyzer, Names, Profile, analyze, check};

use crate::set::Item;

#[derive(Debug, Clone)]
pub struct CandidateScore {
    pub text: String,
    pub contract: Vec<&'static str>,
    pub level: Verdict,
    /// Levels read from each sentence of the line, for reports.
    pub read: Vec<String>,
    pub bad_address: Vec<String>,
    pub bad_self: Option<&'static str>,
    pub style: Vec<&'static str>,
    pub points: u32,
}

impl CandidateScore {
    pub fn polite_match(&self, item: &Item) -> Option<bool> {
        let want = item.level().polite();
        let levels: Vec<bool> = self.read_levels().iter().map(|l| l.polite()).collect();
        if levels.is_empty() {
            None
        } else {
            Some(levels.iter().all(|&p| p == want))
        }
    }

    fn read_levels(&self) -> Vec<speech::Level> {
        self.read
            .iter()
            .filter_map(|r| r.split('/').next().and_then(speech::Level::from_label))
            .collect()
    }
}

pub fn score_candidate(item: &Item, text: &str, analyzer: &Analyzer) -> CandidateScore {
    let line = spoken_part(text);
    let mut contract = contract_problems(text);
    if !line.chars().any(writer_revise::hangul::is_syllable) {
        contract.push("대사 없음");
    }

    let sentences = speech::read_line(&line, analyzer).unwrap_or_default();
    let level = speech::judge_line(item.level(), &sentences);
    let read = sentences
        .iter()
        .filter(|s| !s.levels.is_empty())
        .map(|s| {
            s.levels
                .iter()
                .map(|l| l.label())
                .collect::<Vec<_>>()
                .join("/")
        })
        .collect();

    let bad_address = vocatives(&line, &item.slot.address, &item.bad_addresses());
    let bad_self = match speech::self_reference(&line, analyzer).ok().flatten() {
        Some(found)
            if !item.slot.self_ok.is_empty() && !item.slot.self_ok.iter().any(|s| s == found) =>
        {
            Some(found)
        }
        _ => None,
    };
    let style = style_warnings(item, text, analyzer);

    let points = if !contract.is_empty() {
        0
    } else {
        let level_points = match level {
            Verdict::Match => 40,
            Verdict::Unknown => 20,
            Verdict::Mismatch => 0,
        };
        let address_points = if bad_address.is_empty() { 20 } else { 0 };
        let self_points = if bad_self.is_none() { 20 } else { 0 };
        let style_points = 20u32.saturating_sub(10 * style.len() as u32);
        level_points + address_points + self_points + style_points
    };

    CandidateScore {
        text: text.to_string(),
        contract,
        level,
        read,
        bad_address,
        bad_self,
        style,
        points,
    }
}

/// The quoted speech in a candidate; the whole text when nothing is quoted.
pub fn spoken_part(text: &str) -> String {
    let mut parts = Vec::new();
    let mut chars = text.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        let close = match c {
            '“' => '”',
            '"' => '"',
            '「' => '」',
            _ => continue,
        };
        let start = i + c.len_utf8();
        if let Some(end) = text[start..].find(close) {
            parts.push(text[start..start + end].trim().to_string());
            while let Some(&(j, _)) = chars.peek() {
                if j < start + end + close.len_utf8() {
                    chars.next();
                } else {
                    break;
                }
            }
        }
    }
    if parts.is_empty() {
        text.trim().to_string()
    } else {
        parts.join(" ")
    }
}

/// Breaks of the output contract: the answer must be prose only.
pub fn contract_problems(text: &str) -> Vec<&'static str> {
    let mut out = Vec::new();
    if ["다음은", "후보 1", "후보1", "원하시면", "대안 1"]
        .iter()
        .any(|m| text.contains(m))
    {
        out.push("메타 문장");
    }
    let markdown = text.contains("**")
        || text.contains("```")
        || text.lines().map(str::trim_start).any(|l| {
            l.starts_with('#') || l.starts_with("- ") || l.starts_with("* ") || {
                let digits = l.chars().take_while(char::is_ascii_digit).count();
                digits > 0 && l[digits..].starts_with(". ")
            }
        });
    if markdown {
        out.push("마크다운");
    }
    let mut run = 0;
    for c in text.chars() {
        run = if c.is_ascii_alphabetic() { run + 1 } else { 0 };
        if run >= 2 {
            out.push("영문");
            break;
        }
    }
    out
}

/// Address terms used as vocatives that the slot does not allow: a term at
/// the start of a sentence or right before punctuation. Allowed terms are
/// taken out first so that "한 대리님" does not count as "한 대리".
pub fn vocatives(line: &str, allowed: &[String], bad: &[String]) -> Vec<String> {
    let mut rest = line.to_string();
    let mut allowed: Vec<&String> = allowed.iter().collect();
    allowed.sort_by_key(|a| std::cmp::Reverse(a.chars().count()));
    for a in allowed {
        rest = rest.replace(a.as_str(), " ");
    }
    let mut found = Vec::new();
    for term in bad {
        for (i, _) in rest.match_indices(term.as_str()) {
            let before = rest[..i].trim_end();
            let after = rest[i + term.len()..].chars().next();
            let at_start = before.is_empty() || before.ends_with(['.', '?', '!', '…']);
            let before_mark = matches!(after, None | Some(',' | '.' | '?' | '!' | '…'));
            if (at_start || before_mark) && !found.contains(term) {
                found.push(term.clone());
            }
        }
    }
    found
}

fn style_warnings(item: &Item, text: &str, analyzer: &Analyzer) -> Vec<&'static str> {
    let names = Names::parse(&item.name_list());
    match analyze(text, analyzer) {
        Ok(doc) => check(&doc, &names, &Profile::web_novel())
            .into_iter()
            .filter(|i| i.severity.in_text())
            .map(|i| i.rule)
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// Mean pairwise Jaccard similarity of eojeol bigrams; 1.0 means the
/// candidates say the same thing in the same words.
pub fn similarity(texts: &[&str]) -> Option<f64> {
    let sets: Vec<HashSet<String>> = texts
        .iter()
        .map(|t| {
            let words: Vec<&str> = t.split_whitespace().collect();
            if words.len() < 2 {
                words.iter().map(|w| w.to_string()).collect()
            } else {
                words
                    .windows(2)
                    .map(|w| format!("{} {}", w[0], w[1]))
                    .collect()
            }
        })
        .collect();
    let mut total = 0.0;
    let mut pairs = 0;
    for i in 0..sets.len() {
        for j in i + 1..sets.len() {
            let union = sets[i].union(&sets[j]).count();
            if union > 0 {
                total += sets[i].intersection(&sets[j]).count() as f64 / union as f64;
                pairs += 1;
            }
        }
    }
    (pairs > 0).then(|| total / pairs as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spoken_part_takes_quotes_or_everything() {
        assert_eq!(
            spoken_part("서윤이 웃었다. “팀장님, 가 볼게요.” 그리고 “내일 봬요.”"),
            "팀장님, 가 볼게요. 내일 봬요."
        );
        assert_eq!(spoken_part("그래, 들어가."), "그래, 들어가.");
    }

    #[test]
    fn contract_catches_meta_markdown_and_latin() {
        assert_eq!(contract_problems("다음은 후보입니다"), vec!["메타 문장"]);
        assert_eq!(contract_problems("1. “가자.”"), vec!["마크다운"]);
        assert_eq!(contract_problems("“OK, 가자.”"), vec!["영문"]);
        assert!(contract_problems("“3분 안에 갈게.”").is_empty());
    }

    #[test]
    fn vocatives_only_count_in_address_position() {
        let allowed = vec!["엄마".to_string()];
        let bad = vec!["어미야".to_string(), "할머니".to_string()];
        assert!(vocatives("엄마, 할머니 또 허리 아프신가 봐.", &allowed, &bad).is_empty());
        assert_eq!(
            vocatives("할머니, 좀 쉬세요.", &allowed, &bad),
            vec!["할머니"]
        );
        assert_eq!(vocatives("가자, 어미야.", &allowed, &bad), vec!["어미야"]);
        let allowed = vec!["한 대리님".to_string()];
        assert!(vocatives("한 대리님, 이거요.", &allowed, &["한 대리".to_string()]).is_empty());
    }

    #[test]
    fn similarity_of_same_and_different_lines() {
        assert_eq!(
            similarity(&["가 볼게요 내일 봬요", "가 볼게요 내일 봬요"]),
            Some(1.0)
        );
        assert_eq!(similarity(&["가 볼게요", "먼저 들어갈게요"]), Some(0.0));
        assert_eq!(similarity(&["하나"]), None);
    }
}
