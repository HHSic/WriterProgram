//! Hangul syllables and their letters (자모), as the spell checker needs them.

const BASE: u32 = 0xAC00;
const LAST: u32 = 0xD7A3;
const VOWELS: u32 = 21;
const FINALS: u32 = 28;

pub fn is_syllable(c: char) -> bool {
    (BASE..=LAST).contains(&(c as u32))
}

/// Compatibility letters typed on their own: ㅋㅋ, ㅠㅠ.
pub fn is_letter(c: char) -> bool {
    ('\u{3131}'..='\u{318E}').contains(&c) || ('\u{1100}'..='\u{11FF}').contains(&c)
}

/// Syllables written as conjoining letters (as in the dictionary's `.dic`,
/// and Unicode NFD).
pub fn to_jamo(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 3);
    for c in text.chars() {
        match parts(c) {
            Some((l, v, t)) => {
                out.push(char::from_u32(0x1100 + l).unwrap_or(c));
                out.push(char::from_u32(0x1161 + v).unwrap_or(c));
                if t > 0 {
                    out.push(char::from_u32(0x11A7 + t).unwrap_or(c));
                }
            }
            None => out.push(c),
        }
    }
    out
}

/// (initial, vowel, final) indexes of a syllable; final 0 is none.
pub fn parts(c: char) -> Option<(u32, u32, u32)> {
    if !is_syllable(c) {
        return None;
    }
    let i = c as u32 - BASE;
    Some((i / (VOWELS * FINALS), (i / FINALS) % VOWELS, i % FINALS))
}

pub fn compose(l: u32, v: u32, t: u32) -> char {
    char::from_u32(BASE + (l * VOWELS + v) * FINALS + t).unwrap_or('\u{FFFD}')
}

/// Whether a syllable ends in a consonant (받침), and whether that is ㄹ.
pub fn final_sound(c: char) -> Option<(bool, bool)> {
    parts(c).map(|(_, _, t)| (t > 0, t == 8))
}

/// Initial ㅇ (silent).
const SILENT: u32 = 11;

/// The vowel a syllable's vowel ends on, so "콰" + "앙" and "쿠" + "웅"
/// can be read as one drawn-out sound. Vowel indexes: ㅏ0 ㅐ1 ㅑ2 ㅒ3 ㅓ4
/// ㅔ5 ㅕ6 ㅖ7 ㅗ8 ㅘ9 ㅙ10 ㅚ11 ㅛ12 ㅜ13 ㅝ14 ㅞ15 ㅟ16 ㅠ17 ㅡ18 ㅢ19 ㅣ20.
fn sounds_like(v: u32) -> u32 {
    match v {
        2 | 9 => 0,
        3 | 10 => 1,
        6 | 14 => 4,
        7 | 15 => 5,
        12 => 8,
        17 => 13,
        11 | 16 | 19 => 20,
        v => v,
    }
}

/// A drawn-out sound written short: "콰아앙" → "쾅", "으아아악" → "으악",
/// "흐으음" → "흠". A syllable starting with silent ㅇ whose vowel the one
/// before already ends on is folded into it. None when nothing folds.
pub fn shorten(word: &str) -> Option<String> {
    let mut out: Vec<char> = Vec::new();
    let mut folded = false;
    for c in word.chars() {
        if let (Some(&prev), Some((l, v, t))) = (out.last(), parts(c))
            && l == SILENT
            && let Some((pl, pv, pt)) = parts(prev)
            && pt == 0
            && sounds_like(pv) == v
        {
            *out.last_mut().expect("checked") = compose(pl, pv, t);
            folded = true;
            continue;
        }
        out.push(c);
    }
    folded.then(|| out.into_iter().collect())
}

/// A word that is one piece said over and over ("두근두근두근", "하하하"):
/// the piece. None for other words.
pub fn repeated_piece(word: &str) -> Option<String> {
    let chars: Vec<char> = word.chars().collect();
    let n = chars.len();
    for size in 1..=n / 2 {
        if !n.is_multiple_of(size) || n / size < 2 {
            continue;
        }
        let piece = &chars[..size];
        if chars.chunks(size).all(|c| c == piece) {
            return Some(piece.iter().collect());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syllables_to_letters() {
        assert_eq!(to_jamo("먹"), "\u{1106}\u{1165}\u{11A8}");
        assert_eq!(to_jamo("가a"), "\u{1100}\u{1161}a");
    }

    #[test]
    fn drawn_out_sounds() {
        assert_eq!(shorten("콰아앙").as_deref(), Some("쾅"));
        assert_eq!(shorten("콰앙").as_deref(), Some("쾅"));
        assert_eq!(shorten("으아아악").as_deref(), Some("으악"));
        assert_eq!(shorten("쿠우웅").as_deref(), Some("쿵"));
        assert_eq!(shorten("흐으음").as_deref(), Some("흠"));
        assert_eq!(shorten("사랑"), None);
        // 아 after 사 (ㅏ) folds; 이 after 나 (ㅏ) does not.
        assert_eq!(shorten("나이"), None);
    }

    #[test]
    fn pieces_said_again() {
        assert_eq!(repeated_piece("두근두근두근").as_deref(), Some("두근"));
        assert_eq!(repeated_piece("하하하").as_deref(), Some("하"));
        assert_eq!(repeated_piece("하하호"), None);
    }

    #[test]
    fn final_consonants() {
        assert_eq!(final_sound('나'), Some((false, false)));
        assert_eq!(final_sound('람'), Some((true, false)));
        assert_eq!(final_sound('늘'), Some((true, true)));
        assert_eq!(final_sound('a'), None);
    }
}
