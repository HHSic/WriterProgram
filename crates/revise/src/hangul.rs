//! Hangul syllable helpers: jamo decomposition, final consonants, particles.

const S_BASE: u32 = 0xAC00;
const S_END: u32 = 0xD7A3;
const V_COUNT: u32 = 21;
const T_COUNT: u32 = 28;

pub const JONG_NIEUN: u32 = 4;
pub const JONG_RIEUL: u32 = 8;
pub const JONG_SSANGSIOS: u32 = 20;

pub fn is_syllable(c: char) -> bool {
    (S_BASE..=S_END).contains(&(c as u32))
}

/// Splits a syllable into (initial, medial, final) indices.
pub fn decompose(c: char) -> Option<(u32, u32, u32)> {
    let u = c as u32;
    if !(S_BASE..=S_END).contains(&u) {
        return None;
    }
    let s = u - S_BASE;
    Some((s / (V_COUNT * T_COUNT), (s % (V_COUNT * T_COUNT)) / T_COUNT, s % T_COUNT))
}

/// Returns the syllable with its final consonant replaced.
pub fn with_final(c: char, jong: u32) -> char {
    match decompose(c) {
        Some((l, v, _)) => char::from_u32(S_BASE + (l * V_COUNT + v) * T_COUNT + jong).unwrap_or(c),
        None => c,
    }
}

/// Replaces the final consonant of the last syllable of `word`.
pub fn with_last_final(word: &str, jong: u32) -> String {
    let mut chars: Vec<char> = word.chars().collect();
    if let Some(last) = chars.last_mut() {
        *last = with_final(*last, jong);
    }
    chars.into_iter().collect()
}

/// Jamo sequence used for typo distance ("서화" vs "서하" differ by one jamo).
pub fn jamo(s: &str) -> Vec<u32> {
    let mut out = Vec::new();
    for c in s.chars() {
        match decompose(c) {
            Some((l, v, t)) => {
                out.push(l);
                out.push(100 + v);
                if t > 0 {
                    out.push(200 + t);
                }
            }
            None => out.push(1000 + c as u32),
        }
    }
    out
}

pub fn edit_distance(a: &[u32], b: &[u32]) -> usize {
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0; b.len() + 1];
    for (i, x) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, y) in b.iter().enumerate() {
            let cost = usize::from(x != y);
            cur[j + 1] = (prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

fn last_final(word: &str) -> Option<u32> {
    word.chars().rev().find_map(decompose).map(|(_, _, t)| t)
}

/// 은/는
pub fn eun_neun(word: &str) -> &'static str {
    match last_final(word) {
        Some(t) if t > 0 => "은",
        _ => "는",
    }
}

/// 이/가
pub fn i_ga(word: &str) -> &'static str {
    match last_final(word) {
        Some(t) if t > 0 => "이",
        _ => "가",
    }
}

/// 을/를
pub fn eul_reul(word: &str) -> &'static str {
    match last_final(word) {
        Some(t) if t > 0 => "을",
        _ => "를",
    }
}

/// 으로/로 (a final ㄹ takes 로)
pub fn euro(word: &str) -> &'static str {
    match last_final(word) {
        Some(t) if t > 0 && t != JONG_RIEUL => "으로",
        _ => "로",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typo_distance_is_one_jamo() {
        assert_eq!(edit_distance(&jamo("서화"), &jamo("서하")), 1);
        assert!(edit_distance(&jamo("서울"), &jamo("서하")) > 1);
    }

    #[test]
    fn finals_and_particles() {
        assert_eq!(with_last_final("보이", JONG_NIEUN), "보인");
        assert_eq!(with_last_final("보여", JONG_SSANGSIOS), "보였");
        assert_eq!(euro("그녀는"), "으로");
        assert_eq!(euro("하늘"), "로");
        assert_eq!(i_ga("봉투"), "가");
        assert_eq!(eun_neun("서화"), "는");
    }
}
