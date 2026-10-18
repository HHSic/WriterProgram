//! Speech level (말 단계) of a line of dialogue, read from its sentence-final
//! endings. Used to check a line against the relation table in the setting
//! book (docs/context-sentences.md 7.4).
//!
//! Each sentence is read from the ending of its last eojeol. Endings shared by
//! two levels (-네 in 해체 and 하게체) give both; fragments with no predicate
//! ("네.", "저요.", "그게…") give nothing. A wrong mark is worse than a
//! missing one, so anything unclear is left unread.

use lindera::LinderaResult;

use crate::morph::{Analyzer, Morph, Tok};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Level {
    /// 하십시오체: -습니다, -ㅂ니까, -십시오
    Hasipsio,
    /// 해요체: -어요, -죠, -네요
    Haeyo,
    /// 하오체: -오, -소, -구려
    Hao,
    /// 하게체: -게나, -ㄹ세, -구먼
    Hage,
    /// 해체: -어, -지, -야, -ㄹ게
    Hae,
    /// 해라체: -는다, -냐, -라, -자
    Haera,
}

pub const ALL: [Level; 6] = [Level::Hasipsio, Level::Haeyo, Level::Hao, Level::Hage, Level::Hae, Level::Haera];

impl Level {
    pub fn label(self) -> &'static str {
        match self {
            Level::Hasipsio => "하십시오체",
            Level::Haeyo => "해요체",
            Level::Hao => "하오체",
            Level::Hage => "하게체",
            Level::Hae => "해체",
            Level::Haera => "해라체",
        }
    }

    pub fn from_label(s: &str) -> Option<Level> {
        ALL.into_iter().find(|l| l.label() == s.trim())
    }

    /// 높임 (하십시오체, 해요체, 하오체) as opposed to 안높임.
    pub fn polite(self) -> bool {
        matches!(self, Level::Hasipsio | Level::Haeyo | Level::Hao)
    }

    /// Levels a line may use when this one is expected. 해체 and 해라체 are
    /// both 반말 and mix freely in speech; every other level stands alone.
    fn accepts(self, read: Level) -> bool {
        match self {
            Level::Hae | Level::Haera => matches!(read, Level::Hae | Level::Haera),
            _ => self == read,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Match,
    Mismatch,
    Unknown,
}

/// One sentence of a line and the levels its ending allows (empty: unread).
#[derive(Clone, Debug)]
pub struct SentenceLevel {
    pub text: String,
    pub levels: &'static [Level],
}

/// Whether a sentence read as `read` fits `expected`. An ambiguous reading
/// matches when either of its levels fits.
pub fn judge(expected: Level, read: &[Level]) -> Verdict {
    if read.is_empty() {
        Verdict::Unknown
    } else if read.iter().any(|&r| expected.accepts(r)) {
        Verdict::Match
    } else {
        Verdict::Mismatch
    }
}

/// A whole line: a mismatch anywhere is a mismatch; otherwise one readable
/// matching sentence is enough.
pub fn judge_line(expected: Level, sentences: &[SentenceLevel]) -> Verdict {
    let verdicts: Vec<Verdict> = sentences.iter().map(|s| judge(expected, s.levels)).collect();
    if verdicts.contains(&Verdict::Mismatch) {
        Verdict::Mismatch
    } else if verdicts.contains(&Verdict::Match) {
        Verdict::Match
    } else {
        Verdict::Unknown
    }
}

/// Splits a line into sentences and reads each one.
pub fn read_line(line: &str, analyzer: &Analyzer) -> LinderaResult<Vec<SentenceLevel>> {
    let mut out = Vec::new();
    for sentence in split_sentences(line) {
        let toks = analyzer.analyze(&sentence, 0)?;
        out.push(SentenceLevel { levels: read_tokens(&toks), text: sentence });
    }
    Ok(out)
}

/// Self-reference in a line: 저/제 (humble) or 나/내 (plain), whichever comes first.
pub fn self_reference(line: &str, analyzer: &Analyzer) -> LinderaResult<Option<&'static str>> {
    let toks = analyzer.analyze(line, 0)?;
    let morphs: Vec<&Morph> = toks.iter().flat_map(|t| &t.morphs).collect();
    for (i, m) in morphs.iter().enumerate() {
        if m.pos != "NP" {
            continue;
        }
        match m.form.as_str() {
            // ko-dic tags the determiner in "저 사람" as NP too; a noun right after gives it away.
            "저" if morphs.get(i + 1).is_some_and(|n| n.pos.starts_with("NN") || n.pos == "NP") => {}
            "저" | "제" => return Ok(Some("저")),
            "나" | "내" => return Ok(Some("나")),
            _ => {}
        }
    }
    Ok(None)
}

fn split_sentences(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        cur.push(c);
        if matches!(c, '.' | '?' | '!' | '…' | '~') {
            while let Some(&n) = chars.peek() {
                if matches!(n, '.' | '?' | '!' | '…' | '~') {
                    cur.push(n);
                    chars.next();
                } else {
                    break;
                }
            }
            push_sentence(&mut out, &mut cur);
        }
    }
    push_sentence(&mut out, &mut cur);
    out
}

fn push_sentence(out: &mut Vec<String>, cur: &mut String) {
    let s = cur.trim();
    if s.chars().any(crate::hangul::is_syllable) {
        out.push(s.to_string());
    }
    cur.clear();
}

/// Reads one analyzed sentence from the morphemes of its last eojeol.
pub fn read_tokens(toks: &[Tok]) -> &'static [Level] {
    let is_mark = |t: &Tok| t.morphs.iter().all(|m| m.pos.starts_with('S'));
    // A sentence that trails off (무릎이 시원찮은데…) has not reached its ending yet.
    let trails_off = toks.iter().rev().take_while(|t| is_mark(t)).any(|t| t.surface.contains('…') || t.surface.contains(".."));
    let toks: Vec<&Tok> = toks.iter().filter(|t| !is_mark(t)).collect();
    if toks.is_empty() {
        return &[];
    }
    // The last eojeol: tokens back to the first gap in the byte offsets.
    let mut first = toks.len() - 1;
    while first > 0 && toks[first - 1].end == toks[first].start {
        first -= 1;
    }
    let morphs: Vec<&Morph> = toks[first..].iter().flat_map(|t| &t.morphs).collect();
    if trails_off && morphs.last().is_some_and(|m| m.pos == "EC") {
        return &[];
    }
    read_morphs(&morphs)
}

fn read_morphs(morphs: &[&Morph]) -> &'static [Level] {
    let Some(last) = morphs.last() else { return &[] };
    let has_predicate = morphs.iter().any(|m| is_predicate(&m.pos));
    // 바쁘니까요, 그렇거든요: a connective or final ending followed by 요.
    if last.pos == "JX" && last.form == "요" {
        return if has_predicate { &[Level::Haeyo] } else { &[] };
    }
    match last.pos.as_str() {
        "EF" => {
            let after_si = morphs.len() >= 2 && morphs[morphs.len() - 2].pos == "EP" && morphs[morphs.len() - 2].form == "시";
            read_final(&normalize(&last.form), after_si)
        }
        // 아끼게. is a 하게체 command or a 반말 one; ko-dic calls the ending connective.
        "EC" if last.form == "게" => &[Level::Hae, Level::Hage],
        // A sentence left on a connective ending (바쁘니까., 그런데.) is 반말.
        "EC" => &[Level::Hae],
        _ => &[],
    }
}

fn is_predicate(pos: &str) -> bool {
    pos.starts_with('V') || matches!(pos, "XSV" | "XSA" | "EP" | "EF" | "EC")
}

/// ko-dic writes some endings with conjoining jamo (ᆯ게, ᄇ시다) and others
/// with compatibility jamo (ㄹ까). Both become compatibility jamo.
fn normalize(form: &str) -> String {
    form.chars()
        .map(|c| match c {
            'ᆫ' | 'ᄂ' => 'ㄴ',
            'ᆯ' | 'ᄅ' => 'ㄹ',
            'ᆸ' | 'ᄇ' => 'ㅂ',
            'ᆷ' | 'ᄆ' => 'ㅁ',
            _ => c,
        })
        .collect()
}

fn read_final(f: &str, after_si: bool) -> &'static [Level] {
    const HASIPSIO: &[Level] = &[Level::Hasipsio];
    const HAEYO: &[Level] = &[Level::Haeyo];
    const HAO: &[Level] = &[Level::Hao];
    const HAGE: &[Level] = &[Level::Hage];
    const HAE: &[Level] = &[Level::Hae];
    const HAERA: &[Level] = &[Level::Haera];
    const HAE_OR_HAGE: &[Level] = &[Level::Hae, Level::Hage];

    let ends = |s: &str| f.ends_with(s);
    if ["습니다", "ㅂ니다", "습니까", "ㅂ니까", "십시오", "ㅂ시오", "ㅂ시다", "읍시다"].iter().any(|s| ends(s)) {
        return HASIPSIO;
    }
    if ends("요") || ends("죠") {
        return HAEYO;
    }
    if f == "오" || f == "소" || ends("구려") || (after_si && ends("오")) {
        return HAO;
    }
    if ["게나", "ㄹ세", "구먼", "는구먼"].contains(&f) || f == "게" || f == "세" {
        return HAGE;
    }
    if ends("네") || ["나", "는가", "ㄴ가", "던가", "은가"].contains(&f) {
        return HAE_OR_HAGE;
    }
    if ["냐", "느냐", "니", "자", "마"].contains(&f) || ends("다") || ends("라") || ends("구나") {
        return HAERA;
    }
    if ["어", "아", "여", "야", "지", "래", "대", "데", "걸", "군", "든", "까", "게", "잖아"].iter().any(|s| ends(s)) {
        return HAE;
    }
    &[]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn final_endings_map_to_levels() {
        let cases: [(&str, &[Level]); 20] = [
            ("습니다", &[Level::Hasipsio]),
            ("ㅂ니까", &[Level::Hasipsio]),
            ("ㅂ시다", &[Level::Hasipsio]),
            ("어요", &[Level::Haeyo]),
            ("ㄹ까요", &[Level::Haeyo]),
            ("죠", &[Level::Haeyo]),
            ("소", &[Level::Hao]),
            ("구려", &[Level::Hao]),
            ("게나", &[Level::Hage]),
            ("다네", &[Level::Hae, Level::Hage]),
            ("ㄴ가", &[Level::Hae, Level::Hage]),
            ("는다", &[Level::Haera]),
            ("냐", &[Level::Haera]),
            ("거라", &[Level::Haera]),
            ("자", &[Level::Haera]),
            ("어", &[Level::Hae]),
            ("지", &[Level::Hae]),
            ("ㄹ게", &[Level::Hae]),
            ("ㄹ까", &[Level::Hae]),
            ("잖아", &[Level::Hae]),
        ];
        for (form, want) in cases {
            assert_eq!(read_final(form, false), want, "{form}");
        }
        assert_eq!(read_final("오", true), &[Level::Hao]);
    }

    #[test]
    fn conjoining_jamo_are_normalized() {
        assert_eq!(normalize("ᆯ게"), "ㄹ게");
        assert_eq!(normalize("ᄇ시다"), "ㅂ시다");
    }

    #[test]
    fn half_speech_and_plain_speech_mix() {
        assert_eq!(judge(Level::Hae, &[Level::Haera]), Verdict::Match);
        assert_eq!(judge(Level::Hasipsio, &[Level::Haeyo]), Verdict::Mismatch);
        assert_eq!(judge(Level::Hage, &[Level::Hae, Level::Hage]), Verdict::Match);
        assert_eq!(judge(Level::Haeyo, &[]), Verdict::Unknown);
    }

    #[test]
    fn sentences_split_on_runs_of_marks() {
        assert_eq!(split_sentences("뭐?! 진짜…  그래."), vec!["뭐?!", "진짜…", "그래."]);
        assert_eq!(split_sentences("……"), Vec::<String>::new());
    }
}
