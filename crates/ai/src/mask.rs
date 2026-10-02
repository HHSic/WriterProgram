//! Name masking (이름 가리기): before text leaves the device, the names and
//! other names (aliases) of setting cards become stand-ins such as `인물A`,
//! `장소B`, `용어C`; the answer gets the names back.
//!
//! A card's own name becomes `인물A`; its other names `인물A2`, `인물A3` …,
//! so every stand-in turns back into exactly the word that was there. The
//! letters run across all kinds (`인물A`, `장소B`), in the order the cards
//! are given, and only cards that appear in the text get one.
//!
//! Korean particles follow the sound of the word before them (서하가, 윤석이).
//! The AI picks particles for the stand-in, so when a name comes back the
//! particle after it is fixed to suit the name ([`Masker::unmask`]). Quotes
//! from the manuscript are taken back exactly instead
//! ([`Masker::unmask_exact`]).

use regex::Regex;
use writer_core::cards::{Card, names_regex};

/// One name and the stand-in sent in its place.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Swap {
    pub name: String,
    pub stand_in: String,
    /// The card's own name (for an other name, the card it belongs to).
    pub card: String,
}

#[derive(Debug, Clone, Default)]
pub struct Masker {
    swaps: Vec<Swap>,
    /// Stand-ins of each card's own name, by card id.
    heads: Vec<(String, String)>,
    names: Option<Regex>,
    stand_ins: Option<Regex>,
}

/// What a card's stand-ins start with.
fn kind_word(card_type: &str) -> &'static str {
    match card_type {
        "person" => "인물",
        "place" => "장소",
        "term" => "용어",
        _ => "설정",
    }
}

/// A, B, … Z, AA, AB … for the n-th card (from 0).
fn letters(mut n: usize) -> String {
    let mut out = Vec::new();
    loop {
        out.push(b'A' + (n % 26) as u8);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    out.reverse();
    String::from_utf8(out).expect("ascii")
}

fn longest_first(mut words: Vec<String>) -> Vec<String> {
    words.sort_by(|a, b| b.chars().count().cmp(&a.chars().count()).then(a.cmp(b)));
    words.dedup();
    words
}

impl Masker {
    /// Stand-ins for the cards whose names appear in any of `texts`.
    pub fn new(cards: &[Card], texts: &[&str]) -> Masker {
        let mut swaps = Vec::new();
        let mut heads = Vec::new();
        let mut taken: Vec<String> = Vec::new();
        let mut n = 0;
        for card in cards {
            // Names another card already uses stay with the first card.
            let names: Vec<String> = card
                .names()
                .into_iter()
                .filter(|name| !taken.contains(name))
                .collect();
            let seen = names_regex(&names).is_some_and(|re| texts.iter().any(|t| re.is_match(t)));
            if !seen {
                continue;
            }
            let head = format!("{}{}", kind_word(&card.card_type), letters(n));
            n += 1;
            let own = card.name.trim().to_string();
            let mut extra = 1;
            // The card's own name first, then its other names in order.
            let mut ordered: Vec<String> = Vec::new();
            if names.contains(&own) {
                ordered.push(own.clone());
            }
            for alias in &card.aliases {
                let alias = alias.trim().to_string();
                if names.contains(&alias) && !ordered.contains(&alias) {
                    ordered.push(alias);
                }
            }
            for name in ordered {
                let stand_in = if name == own {
                    head.clone()
                } else {
                    extra += 1;
                    format!("{head}{extra}")
                };
                taken.push(name.clone());
                swaps.push(Swap {
                    name,
                    stand_in,
                    card: own.clone(),
                });
            }
            heads.push((card.id.clone(), head));
        }
        let names = names_regex(&longest_first(
            swaps.iter().map(|s| s.name.clone()).collect(),
        ));
        let stand_ins = names_regex(&longest_first(
            swaps.iter().map(|s| s.stand_in.clone()).collect(),
        ));
        Masker {
            swaps,
            heads,
            names,
            stand_ins,
        }
    }

    pub fn swaps(&self) -> &[Swap] {
        &self.swaps
    }

    /// The stand-in of a card's own name, if the card got one.
    pub fn head(&self, card_id: &str) -> Option<&str> {
        self.heads
            .iter()
            .find(|(id, _)| id == card_id)
            .map(|(_, h)| h.as_str())
    }

    /// The text with every name replaced by its stand-in.
    pub fn mask(&self, text: &str) -> String {
        let Some(re) = &self.names else {
            return text.to_string();
        };
        re.replace_all(text, |c: &regex::Captures| {
            let name = &c[0];
            self.swaps
                .iter()
                .find(|s| s.name == name)
                .map(|s| s.stand_in.clone())
                .unwrap_or_else(|| name.to_string())
        })
        .into_owned()
    }

    fn name_of<'a>(&'a self, stand_in: &'a str) -> &'a str {
        self.swaps
            .iter()
            .find(|s| s.stand_in == stand_in)
            .map(|s| s.name.as_str())
            .unwrap_or(stand_in)
    }

    /// Names back, word for word (for quotes from the manuscript).
    pub fn unmask_exact(&self, text: &str) -> String {
        let Some(re) = &self.stand_ins else {
            return text.to_string();
        };
        re.replace_all(text, |c: &regex::Captures| self.name_of(&c[0]).to_string())
            .into_owned()
    }

    /// Names back, with the particle after each fixed to suit the name (for
    /// sentences the AI wrote).
    pub fn unmask(&self, text: &str) -> String {
        let Some(re) = &self.stand_ins else {
            return text.to_string();
        };
        let mut out = String::with_capacity(text.len());
        let mut at = 0;
        for m in re.find_iter(text) {
            if m.start() < at {
                continue;
            }
            out.push_str(&text[at..m.start()]);
            let name = self.name_of(m.as_str());
            out.push_str(name);
            at = m.end();
            if let Some((used, particle)) = fix_particle(name, &text[at..]) {
                out.push_str(particle);
                at += used;
            }
        }
        out.push_str(&text[at..]);
        out
    }
}

/// The last syllable's final consonant: `None` for a word that does not end
/// in a Hangul syllable, `Some(0)` for none, `Some(8)` for ㄹ.
fn final_consonant(word: &str) -> Option<u32> {
    let c = word.chars().last()? as u32;
    (0xAC00..=0xD7A3).contains(&c).then(|| (c - 0xAC00) % 28)
}

fn is_syllable(c: Option<char>) -> bool {
    c.is_some_and(|c| ('\u{AC00}'..='\u{D7A3}').contains(&c))
}

/// Particle pairs: (after a final consonant, after none).
const PAIRS: [(&str, &str); 6] = [
    ("이", "가"),
    ("은", "는"),
    ("을", "를"),
    ("과", "와"),
    ("아", "야"),
    ("으로", "로"),
];

/// If `rest` starts with a particle in the wrong form for `name`, how many
/// bytes it takes and the right form. One-syllable subject, topic, object
/// and calling particles count only when a word ends right after them, so
/// `인물A이다` or `인물A는데` are left alone.
fn fix_particle(name: &str, rest: &str) -> Option<(usize, &'static str)> {
    let jong = final_consonant(name)?;
    for (with, without) in PAIRS {
        for found in [with, without] {
            let Some(after) = rest.strip_prefix(found) else {
                continue;
            };
            let free_standing = matches!(found, "과" | "와" | "으로" | "로");
            if !free_standing && is_syllable(after.chars().next()) {
                return None;
            }
            let right = if found == "으로" || found == "로" {
                // ㄹ takes 로 like a word with no final consonant.
                if jong == 0 || jong == 8 {
                    "로"
                } else {
                    "으로"
                }
            } else if jong == 0 {
                without
            } else {
                with
            };
            return (right != found).then_some((found.len(), right));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(id: &str, kind: &str, name: &str, aliases: &[&str]) -> Card {
        Card {
            id: id.into(),
            card_type: kind.into(),
            name: name.into(),
            aliases: aliases.iter().map(|a| (*a).to_string()).collect(),
            fields: Vec::new(),
            highlight: true,
            created: String::new(),
            description: String::new(),
            extra: Vec::new(),
        }
    }

    fn cast() -> Vec<Card> {
        vec![
            card("p1", "person", "윤서하", &["서하", "윤 사장"]),
            card("p2", "person", "강윤석", &["윤석"]),
            card("x1", "place", "달빛 서점", &["서점"]),
            card("t1", "term", "대여 카드", &[]),
            card("n1", "person", "없는 사람", &[]),
        ]
    }

    #[test]
    fn letters_run_on() {
        assert_eq!(letters(0), "A");
        assert_eq!(letters(25), "Z");
        assert_eq!(letters(26), "AA");
        assert_eq!(letters(27), "AB");
    }

    #[test]
    fn names_with_particles_round_trip() {
        let text = "서하가 윤 사장이라 불리던 밤, 윤서하는 달빛 서점에서 윤석을 기다렸다. 대여 카드를 꺼냈다.";
        let m = Masker::new(&cast(), &[text]);
        let masked = m.mask(text);
        assert_eq!(
            masked,
            "인물A2가 인물A3이라 불리던 밤, 인물A는 장소C에서 인물B2을 기다렸다. 용어D를 꺼냈다."
        );
        assert!(!masked.contains("서하"));
        assert_eq!(m.unmask_exact(&masked), text);
        // Particles already right for the name stay as they are.
        assert_eq!(m.unmask(&masked), text);
        // The unseen card gets no stand-in.
        assert!(m.swaps().iter().all(|s| s.card != "없는 사람"));
        assert_eq!(m.head("p2"), Some("인물B"));
        assert_eq!(m.head("n1"), None);
    }

    #[test]
    fn particles_follow_the_name() {
        let m = Masker::new(&cast(), &["윤석 서하 서점"]);
        // The AI wrote particles for "인물B" (read 인물비, no final consonant).
        assert_eq!(m.unmask("인물A2가 웃었다."), "서하가 웃었다.");
        assert_eq!(m.unmask("인물B2가 웃었다."), "윤석이 웃었다.");
        assert_eq!(
            m.unmask("인물B2는, 인물B2를, 인물B2와 함께"),
            "윤석은, 윤석을, 윤석과 함께"
        );
        assert_eq!(
            m.unmask("인물A2은 인물A2을 인물A2과"),
            "서하는 서하를 서하와"
        );
        assert_eq!(m.unmask("장소C2로 갔다"), "서점으로 갔다");
        assert_eq!(m.unmask("인물B2야!"), "윤석아!");
        // Not a particle: the word goes on.
        assert_eq!(m.unmask("인물B2가방"), "윤석가방");
        assert_eq!(m.unmask("인물A2이다"), "서하이다");
    }

    #[test]
    fn rieul_takes_ro() {
        let cards = vec![card("p", "place", "하늘", &[])];
        let m = Masker::new(&cards, &["하늘"]);
        assert_eq!(m.mask("하늘로"), "장소A로");
        assert_eq!(m.unmask("장소A으로 날았다"), "하늘로 날았다");
        assert_eq!(m.unmask("장소A가 맑다"), "하늘이 맑다");
    }

    #[test]
    fn nothing_to_hide() {
        let m = Masker::new(&cast(), &["아무도 없다"]);
        assert!(m.swaps().is_empty());
        assert_eq!(m.mask("아무도 없다"), "아무도 없다");
        assert_eq!(m.unmask("인물A가"), "인물A가");
    }
}
