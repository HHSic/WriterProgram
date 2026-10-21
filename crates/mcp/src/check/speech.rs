//! Speech level of a line of dialogue, guessed from the end of its last word.
//! Port of `plugins/kfiction/lib/speech.js`; a guess, shown only as a note.

use std::sync::LazyLock;

use regex::Regex;

use super::text::{last_word, sentences};

const POLITE: [&str; 3] = ["하십시오체", "해요체", "하오체"];
const FRAGMENTS: [&str; 16] = [
    "네", "예", "응", "음", "흠", "아", "어", "오", "와", "뭐", "왜", "저기", "저", "나", "너",
    "야",
];

const J_NIEUN: i32 = 4;
const J_RIEUL: i32 = 8;
const J_BIEUP: i32 = 17;

static TRAILS_OFF: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(…|\.\.)$").expect("pattern compiles"));
static CONNECTIVE_END: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(는데|은데|고|며|서|니까|면|지만)$").expect("pattern compiles"));

fn syllable(c: Option<char>) -> Option<u32> {
    let c = c? as u32;
    (0xac00..=0xd7a3).contains(&c).then(|| c - 0xac00)
}

/// Final consonant index, 0 for none, -1 for not a syllable.
fn jong(c: Option<char>) -> i32 {
    syllable(c).map_or(-1, |s| (s % 28) as i32)
}

fn vowel(c: Option<char>) -> i32 {
    syllable(c).map_or(-1, |s| ((s % 588) / 28) as i32)
}

/// Candidate levels for one word's ending; empty when it cannot be read.
pub fn read_word(w: &str, question: bool) -> Vec<&'static str> {
    if w.is_empty() || FRAGMENTS.contains(&w) {
        return vec![];
    }
    let chars: Vec<char> = w.chars().collect();
    let at = |back: usize| chars.len().checked_sub(back).map(|i| chars[i]);
    let (last, prev, prev2) = (at(1), at(2), at(3));
    let ends = |s: &str| w.ends_with(s);

    if ends("습니다") || ends("습니까") || ends("십시오") || ends("읍시다") {
        return vec!["하십시오체"];
    }
    if (ends("니다") || ends("니까")) && jong(prev2) == J_BIEUP {
        return vec!["하십시오체"];
    }
    if ends("시다") && jong(prev2) == J_BIEUP {
        return vec!["하십시오체"];
    }
    if last == Some('요') || last == Some('죠') {
        return vec!["해요체"];
    }
    if ends("구려") || ends("시오") || ends("하오") || ends("이오") || ends("되오") {
        return vec!["하오체"];
    }
    if last == Some('소') && jong(prev) > 0 {
        return vec!["하오체"];
    }
    if ends("게나") || ends("구먼") || ends("는가") || ends("던가") || ends("하세") {
        return vec!["하게체"];
    }
    if last == Some('네')
        || (last == Some('가') && jong(prev) == J_NIEUN)
        || (question && last == Some('나'))
    {
        return vec!["해체", "하게체"];
    }
    if last == Some('게') && jong(prev) == J_RIEUL {
        return vec!["해체"];
    }
    if last == Some('다') || matches!(last, Some('냐' | '니' | '라' | '자')) || ends("구나")
    {
        return vec!["해라체"];
    }
    if last.is_some_and(|c| "어아여야지래대데걸군게까든".contains(c)) {
        return vec!["해체"];
    }
    if jong(last) == 0 && [0, 1, 4, 6, 9, 10, 14].contains(&vowel(last)) {
        return vec!["해체"];
    }
    vec![]
}

/// Each sentence of a line with the levels it reads as.
pub fn read_line(line: &str) -> Vec<(String, Vec<&'static str>)> {
    sentences(line)
        .into_iter()
        .map(|s| {
            let w = last_word(&s);
            let levels = if TRAILS_OFF.is_match(&s) && CONNECTIVE_END.is_match(&w) {
                vec![]
            } else {
                read_word(&w, s.ends_with('?'))
            };
            (s, levels)
        })
        .collect()
}

/// Whether one line mixes polite and plain speech (usually a slip when it is
/// said to one listener).
pub fn mixes_politeness(line: &str) -> bool {
    let (mut polite, mut plain) = (false, false);
    for (_, levels) in read_line(line) {
        if levels.is_empty() {
            continue;
        }
        polite |= levels.iter().all(|l| POLITE.contains(l));
        plain |= levels.iter().all(|l| !POLITE.contains(l));
    }
    polite && plain
}
