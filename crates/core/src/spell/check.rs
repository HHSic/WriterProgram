//! Checking text word by word (어절).
//!
//! A word is a run of letters and digits. Particles and endings stay on the
//! word: the dictionary has them as suffixes ("사람을", "먹었다"), so the
//! whole 어절 is looked up. Words without a Hangul syllable (English, 漢字,
//! numbers) and words with lone letters ("ㅋㅋ") are not checked.

use std::collections::HashSet;
use std::ops::Range;

use serde::Serialize;

use super::Speller;
use super::hangul::{final_sound, is_letter, is_syllable, repeated_piece, shorten};

/// A word the dictionary does not know, in UTF-16 units of the text (as the
/// editor counts).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub start: usize,
    pub end: usize,
    pub word: String,
}

/// Words the writer has said are right: setting card names and aliases,
/// the project's word list and this device's (사용자 사전).
#[derive(Debug, Default, Clone)]
pub struct Known {
    words: HashSet<String>,
}

impl Known {
    /// Each entry, and each word of an entry of several ("윤 사장").
    pub fn new<S: AsRef<str>>(entries: impl IntoIterator<Item = S>) -> Self {
        let mut words = HashSet::new();
        for entry in entries {
            let entry = entry.as_ref().trim();
            if entry.is_empty() {
                continue;
            }
            words.insert(entry.to_string());
            for piece in entry.split_whitespace() {
                words.insert(piece.to_string());
            }
        }
        Known { words }
    }

    pub fn has(&self, word: &str) -> bool {
        self.words.contains(word)
    }
}

/// Stand-in nouns for a known word with a particle: "서하는" is right if
/// "나무는" is. One for each way a word can end: a vowel, a consonant, ㄹ.
const AFTER_VOWEL: &str = "나무";
const AFTER_CONSONANT: &str = "사람";
const AFTER_RIEUL: &str = "하늘";

/// Whether `rest` can follow a word ending in `last` ("는" after "서하").
fn attaches(speller: &Speller, last: char, rest: &str) -> bool {
    let stand_ins: &[&str] = match final_sound(last) {
        Some((false, _)) => &[AFTER_VOWEL],
        Some((true, false)) => &[AFTER_CONSONANT],
        Some((true, true)) => &[AFTER_RIEUL],
        // A name in Latin letters or digits: its sound is unknown.
        None => &[AFTER_VOWEL, AFTER_CONSONANT, AFTER_RIEUL],
    };
    stand_ins
        .iter()
        .any(|noun| speller.knows(&format!("{noun}{rest}")))
}

/// Common slips the dictionary lets through because another reading exists
/// ("되요" as 되, a measure, + 요): wrong ending, right ending.
const SLIPS: [(&str, &str); 4] = [
    ("되요", "돼요"),
    ("되서", "돼서"),
    ("께요", "게요"),
    ("께", "게"),
];

/// The word as meant, when it is one of the common slips: "되요" → "돼요",
/// "할께" → "할게". "께" counts only after ㄹ, so "선생님께" stays.
pub(crate) fn slip_fix(speller: &Speller, word: &str) -> Option<String> {
    for (wrong, right) in SLIPS {
        let Some(stem) = word.strip_suffix(wrong) else {
            continue;
        };
        if wrong.starts_with('께')
            && stem.chars().next_back().and_then(final_sound) != Some((true, true))
        {
            continue;
        }
        let fixed = format!("{stem}{right}");
        if speller.knows(&fixed) {
            return Some(fixed);
        }
    }
    None
}

/// Whether a Hangul word is right: in the dictionary (and not a common
/// slip), a known word (maybe with particles), or a drawn-out or repeated
/// sound of a word.
pub fn word_ok(speller: &Speller, known: &Known, word: &str) -> bool {
    if known.has(word) {
        return true;
    }
    if speller.knows(word) {
        return slip_fix(speller, word).is_none();
    }
    // A known word with particles or 이다 after it: "서하는", "서하였다".
    let bounds: Vec<usize> = word.char_indices().map(|(i, _)| i).skip(1).collect();
    for &cut in bounds.iter().rev() {
        let (head, rest) = word.split_at(cut);
        if known.has(head) {
            let last = head.chars().next_back().expect("not empty");
            if attaches(speller, last, rest) {
                return true;
            }
        }
    }
    // 소리 흉내 말: "콰아앙" for "쾅", "두근두근두근".
    if let Some(short) = shorten(word)
        && (known.has(&short) || speller.knows(&short))
    {
        return true;
    }
    if let Some(piece) = repeated_piece(word) {
        let twice = format!("{piece}{piece}");
        if known.has(&piece) || speller.knows(&twice) || speller.knows(&piece) {
            return true;
        }
    }
    false
}

fn word_char(c: char) -> bool {
    c.is_alphanumeric() || is_letter(c)
}

/// Byte ranges of the words of `text` to check.
fn words(text: &str) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    for (i, c) in text.char_indices() {
        match (word_char(c), start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                out.push(s..i);
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        out.push(s..text.len());
    }
    out
}

/// Whether one word of the text is right, or not ours to check.
fn token_ok(speller: &Speller, known: &Known, token: &str) -> bool {
    if !token.chars().any(is_syllable) || token.chars().any(is_letter) {
        return true;
    }
    // Latin letters or digits before Hangul: "SNS를", "3개월이", "5층에서".
    let lead = token
        .char_indices()
        .find(|(_, c)| !c.is_ascii_alphanumeric())
        .map_or(token.len(), |(i, _)| i);
    let rest = &token[lead..];
    if !rest.chars().all(is_syllable) {
        // Hangul mixed with other letters (漢字, Latin in the middle).
        return true;
    }
    if lead == 0 {
        return word_ok(speller, known, token);
    }
    let last = token[..lead].chars().next_back().expect("not empty");
    known.has(&token[..lead])
        || speller.knows(token)
        || attaches(speller, last, rest)
        || word_ok(speller, known, rest)
}

/// Byte ranges of the words in `text` the dictionary does not know.
pub(crate) fn issue_ranges(speller: &Speller, known: &Known, text: &str) -> Vec<Range<usize>> {
    words(text)
        .into_iter()
        .filter(|r| !token_ok(speller, known, &text[r.clone()]))
        .collect()
}

/// Words in `text` (one paragraph) the dictionary does not know.
pub fn check_text(speller: &Speller, known: &Known, text: &str) -> Vec<Issue> {
    let mut out = Vec::new();
    let mut units = 0;
    let mut at = 0;
    for r in issue_ranges(speller, known, text) {
        units += text[at..r.start].encode_utf16().count();
        let len = text[r.clone()].encode_utf16().count();
        out.push(Issue {
            start: units,
            end: units + len,
            word: text[r.clone()].to_string(),
        });
        units += len;
        at = r.end;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spell::fixture::speller;

    fn flagged(text: &str, known: &[&str]) -> Vec<String> {
        check_text(&speller(), &Known::new(known), text)
            .into_iter()
            .map(|i| i.word)
            .collect()
    }

    #[test]
    fn words_with_endings_and_particles() {
        assert!(flagged("사람들은 나무를 먹었다.", &[]).is_empty());
        assert_eq!(
            flagged("사람은 나무을 멱었다", &[]),
            vec!["나무을", "멱었다"]
        );
    }

    #[test]
    fn ranges_count_utf16_units() {
        let s = speller();
        let found = check_text(&s, &Known::default(), "😀 멱었다");
        assert_eq!(
            found,
            vec![Issue {
                start: 3,
                end: 6,
                word: "멱었다".into()
            }]
        );
    }

    #[test]
    fn card_names_with_particles() {
        let known = ["윤서하", "서하", "윤 사장"];
        assert!(flagged("서하는 서하에게 윤서하를 바다로", &known).is_empty());
        // A particle that does not fit the name's last sound.
        assert_eq!(flagged("서하을", &known), vec!["서하을"]);
        assert!(flagged("윤 사장이", &known).is_empty());
        assert_eq!(flagged("서하는", &[]), vec!["서하는"]);
    }

    #[test]
    fn sounds_drawn_out_or_repeated() {
        assert!(flagged("콰아앙 으아아악 두근두근두근 하하하하", &[]).is_empty());
        assert_eq!(flagged("콰앙콰", &[]), vec!["콰앙콰"]);
    }

    #[test]
    fn what_is_not_checked() {
        assert!(flagged("ㅋㅋㅋ hello 漢字 2026 SNS를", &[]).is_empty());
        assert_eq!(flagged("SNS을", &[]), Vec::<String>::new());
        assert_eq!(flagged("3멱었다", &[]), vec!["3멱었다"]);
    }

    #[test]
    fn common_slips_are_flagged_though_the_dictionary_has_them() {
        assert_eq!(flagged("할께", &[]), vec!["할께"]);
        assert_eq!(
            speller().suggest("할께").first().map(String::as_str),
            Some("할게")
        );
        // 께 after a word not ending in ㄹ is the particle.
        assert!(flagged("선생님께 할게", &[]).is_empty());
        assert!(flagged("할께", &["할께"]).is_empty());
    }

    #[test]
    fn forbidden_words_stay_flagged() {
        assert_eq!(flagged("되요", &[]), vec!["되요"]);
        assert!(flagged("되요", &["되요"]).is_empty());
    }
}
