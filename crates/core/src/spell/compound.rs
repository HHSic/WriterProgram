//! Rule compounds of the Korean dictionary, matched as Hunspell does
//! (see `prepare`): every part but the last must be a bare word with the
//! rule's flag, and the last may carry suffixes.
//!
//! hunspell-dict-ko uses them for plurals ("사람" + "들은"), auxiliary verbs
//! written together ("새겨" + "져", "보여" + "주었다"), "할" + "만하다" and
//! numbers with counters ("3" + "개월").

use std::collections::HashMap;

use super::hangul::to_jamo;
use super::prepare::{Repeat, Rule};

pub struct Compounds {
    rules: Vec<Rule>,
    heads: HashMap<String, Vec<u32>>,
}

impl Compounds {
    pub fn new(rules: Vec<Rule>, heads: HashMap<String, Vec<u32>>) -> Self {
        Compounds { rules, heads }
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Whether `word` is a rule compound. `tail(flag, part)` says whether
    /// `part`, maybe with suffixes, is a word that may end a compound with
    /// `flag`.
    pub fn matches(&self, word: &str, tail: &mut dyn FnMut(u32, &str) -> bool) -> bool {
        let chars: Vec<char> = word.chars().collect();
        if chars.len() < 2 {
            return false;
        }
        self.rules
            .iter()
            .any(|rule| self.walk(rule, 0, &chars, 0, 0, tail))
    }

    fn head(&self, part: &[char], flag: u32) -> bool {
        let text: String = part.iter().collect();
        self.heads
            .get(&to_jamo(&text))
            .is_some_and(|flags| flags.contains(&flag))
    }

    fn walk(
        &self,
        rule: &Rule,
        item: usize,
        chars: &[char],
        pos: usize,
        parts: usize,
        tail: &mut dyn FnMut(u32, &str) -> bool,
    ) -> bool {
        let Some(&(flag, repeat)) = rule.get(item) else {
            return false;
        };
        if repeat != Repeat::Once && self.walk(rule, item + 1, chars, pos, parts, tail) {
            return true;
        }
        let rest_optional = rule[item + 1..].iter().all(|(_, r)| *r != Repeat::Once);
        for end in pos + 1..=chars.len() {
            let part = &chars[pos..end];
            if end == chars.len() {
                if rest_optional && parts >= 1 {
                    let text: String = part.iter().collect();
                    if tail(flag, &text) {
                        return true;
                    }
                }
            } else if self.head(part, flag) {
                let next = if repeat == Repeat::Any {
                    item
                } else {
                    item + 1
                };
                if self.walk(rule, next, chars, end, parts + 1, tail) {
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compounds() -> Compounds {
        let rules = vec![
            vec![(9, Repeat::Once), (14, Repeat::Once)],
            vec![(12, Repeat::Any), (12, Repeat::Once), (13, Repeat::Once)],
        ];
        let mut heads = HashMap::new();
        heads.insert(to_jamo("사람"), vec![9]);
        heads.insert(to_jamo("들"), vec![14]);
        heads.insert("1".to_string(), vec![12]);
        heads.insert("2".to_string(), vec![12]);
        heads.insert(to_jamo("개월"), vec![13]);
        Compounds::new(rules, heads)
    }

    fn tails(flag: u32, part: &str) -> bool {
        matches!(
            (flag, part),
            (14, "들" | "들은" | "들이") | (13, "개월" | "개월이")
        )
    }

    #[test]
    fn last_part_may_carry_suffixes() {
        let c = compounds();
        assert!(c.matches("사람들은", &mut tails));
        assert!(c.matches("사람들", &mut tails));
        assert!(!c.matches("사람들께", &mut tails));
        assert!(!c.matches("들은", &mut tails));
    }

    #[test]
    fn repeated_parts() {
        let c = compounds();
        assert!(c.matches("12개월이", &mut tails));
        assert!(c.matches("1개월", &mut tails));
        assert!(!c.matches("개월이", &mut tails));
    }
}
