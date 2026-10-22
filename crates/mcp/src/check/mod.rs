//! The kfiction manuscript check (T0, character rules), ported from
//! `plugins/kfiction/lib/lint.js` so the installer can run it without Node.
//! The two stay in step through `plugins/kfiction/test/fixtures/lint-cases.json`,
//! which both test suites check against.
//!
//! - contract: assistant talk or markdown inside the manuscript. Writing tools
//!   refuse such text before saving.
//! - warn: marks more common in AI text than in people's. Reported, not enforced.
//! - info: notes.

pub(crate) mod speech;
pub(crate) mod text;

use std::collections::HashSet;
use std::sync::LazyLock;

use regex::Regex;
use serde_json::{Value, json};

use speech::mixes_politeness;
use text::{Paragraph, last_word, len, paragraphs, sentences};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Contract,
    Warn,
    Info,
}

impl Severity {
    pub fn key(self) -> &'static str {
        match self {
            Severity::Contract => "contract",
            Severity::Warn => "warn",
            Severity::Info => "info",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Finding {
    pub rule: &'static str,
    pub name: &'static str,
    pub severity: Severity,
    /// Line in the text checked; none for findings about the whole text.
    pub line: Option<usize>,
    pub matched: String,
    pub hint: &'static str,
}

impl Finding {
    pub fn to_json(&self) -> Value {
        json!({
            "rule": self.rule,
            "severity": self.severity.key(),
            "name": self.name,
            "line": self.line,
            "match": self.matched,
            "hint": self.hint,
        })
    }
}

fn re(pattern: &str) -> Regex {
    Regex::new(pattern).expect("pattern compiles")
}

// Rules (plugins/kfiction/lib/rules.js).
static META_START: LazyLock<Regex> = LazyLock::new(|| {
    re(r"^(다음은|아래는|이어서 쓴|수정한 (문장|부분|원고)|고친 (문장|부분|원고))")
});
static META_ANY: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(원하시면|필요하시면|도움이 되었으면|더 다듬어 드릴|어떠신가요\?|말씀해 주세요|수정해 드릴게요|요청하신|말씀하신 대로|마음에 드시면|마음에 드셨으면|다른 버전|피드백)",
    )
});
/// Talk about the writing: "장면을 이어 썼습니다", "다음과 같이 고쳤습니다".
static META_WRITE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(장면|문장|원고|초안|본문|대사|버전|회차|후보|단락|문단)(을|를|은|는|도)? ?((조금|좀|살짝|약간|모두|전부|한 번 더) )?(이어 ?|이어서 |다시 |새로 |고쳐 ?|다듬어 ?)?(썼|써 ?봤|써 ?보았|작성했|수정했|고쳤|다듬었|바꿨|바꾸었|정리했|이어 ?갔)(습니다|어요|어 드렸|드렸)|(다음|아래)(과|와) 같이 .{0,15}(썼|작성했|고쳤|다듬었|바꿨|정리했|이어 ?갔)(습니다|어요)|이어서? 써 ?(봤|보았)(습니다|어요)",
    )
});
/// An answer's opening; talk only when the paragraph is about the writing.
static META_OPENER: LazyLock<Regex> = LazyLock::new(|| {
    re(r"^(물론이죠|물론입니다|물론이에요|좋습니다|좋아요|알겠습니다|알겠어요|네)[.!,]")
});
static META_TOPIC: LazyLock<Regex> = LazyLock::new(|| {
    re(r"(장면|원고|초안|본문|버전|요청|작성|이어 ?쓰|이어 ?썼|이어서|써 ?봤|써 ?보았|고쳐|다듬)")
});
static MARKDOWN_MARK: LazyLock<Regex> = LazyLock::new(|| re(r"\*\*|__"));
static MARKDOWN_LIST: LazyLock<Regex> = LazyLock::new(|| re(r"^[-*+]\s|^\d+\.\s"));
static LATIN: LazyLock<Regex> = LazyLock::new(|| re(r"[A-Za-z]{2,}"));
static DOUBLE_PASSIVE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?:되어|돼)(?:지|집|졌|져|짐)|보여(?:지|집|졌|져|짐)|(?:쓰|불|풀|열|묶|팔|걸|잘)려(?:지|졌|져|짐)|(?:읽|닫|잊|잡|먹|갇|막)혀(?:지|졌|져|짐)|(?:믿|안)겨(?:지|졌|져)|(?:놓|쌓|모)여(?:지|졌|져)",
    )
});
static UI_CHAIN: LazyLock<Regex> = LazyLock::new(|| re(r"[가-힣]+의 [가-힣]+의 "));
static CONNECTIVE_COMMA: LazyLock<Regex> =
    LazyLock::new(|| re(r"(고|며|면서|지만|는데|은데|어서|아서|해서|니까|으면|더니|자마자),\s"));
static NEGATIVE_CONTRAST: LazyLock<Regex> =
    LazyLock::new(|| re(r"(이|가|게|것이|건) 아니라\s|아니었다\.\s+[^.?!]{1,40}(였다|이었다)\."));
static DASH: LazyLock<Regex> = LazyLock::new(|| re(r"[가-힣.,!?]\s?[—–―]\s?[가-힣]"));

const UI_NOUNS: &str = "회의 정의 의의 동의 주의 강의 논의 합의 건의 협의 신의 결의 적의 호의 악의 선의 성의 사의 상의 예의 고의 본의 민의 문의 이의 내의 하의 전의 질의 토의 모의 심의 숙의 창의 열의 경의 대의 축의 탈의";

const TRANSLATIONESE: [(&str, &str); 15] = [
    ("에 있어서", "‘~에서’나 ‘~할 때’로"),
    ("으로부터", "사람이면 ‘~에게서’, 아니면 ‘~에서’로"),
    ("로부터", "사람이면 ‘~에게서’, 아니면 ‘~에서’로"),
    ("에 의해", "행위자를 주어로 세워 능동문으로"),
    ("을 통해", "‘~로’나 ‘~해서’로"),
    ("를 통해", "‘~로’나 ‘~해서’로"),
    ("에 대해", "‘~을/를’로 바꾸거나 문장을 다듬어"),
    ("에 대한", "문장을 다듬어"),
    ("을 가지고 있", "소유가 아니면 ‘~이 있다’로"),
    ("를 가지고 있", "소유가 아니면 ‘~이 있다’로"),
    ("하고 있는 중", "‘~하고 있다’로"),
    ("지 않을 수 없었다", "하고 싶은 말을 바로"),
    ("에서의", "‘의’를 빼고 문장을 다듬어"),
    ("으로 인해", "‘~때문에’, ‘~해서’로"),
    ("로 인해", "‘~때문에’, ‘~해서’로"),
];

const CLICHES: [&str; 12] = [
    "숨을 깊이 들이",
    "숨을 들이켰",
    "심장이 쿵",
    "심장이 내려앉",
    "입술을 깨물",
    "들릴 듯 말 듯",
    "주먹을 꽉 쥐",
    "침을 꿀꺽 삼키",
    "눈시울이 붉어",
    "시간이 멈춘 것 같",
    "등골이 서늘",
    "가슴이 먹먹",
];

const J_NIEUN: i32 = 4;
const J_SSANGSIOS: i32 = 20;

fn jong(c: Option<char>) -> i32 {
    match c.map(|c| c as u32) {
        Some(c) if (0xac00..=0xd7a3).contains(&c) => ((c - 0xac00) % 28) as i32,
        _ => -1,
    }
}

/// How a sentence ends: past (-었다), present (-ㄴ다), or its last two
/// characters.
pub fn ending_class(sentence: &str) -> String {
    let w = last_word(sentence);
    if w.is_empty() {
        return String::new();
    }
    let chars: Vec<char> = w.chars().collect();
    let before_last = chars.len().checked_sub(2).map(|i| chars[i]);
    if w.ends_with('다') && jong(before_last) == J_SSANGSIOS {
        return "-었다".into();
    }
    if w.ends_with("는다") || (w.ends_with('다') && jong(before_last) == J_NIEUN) {
        return "-ㄴ다".into();
    }
    chars[chars.len().saturating_sub(2)..].iter().collect()
}

fn first_word(s: &str) -> &str {
    s.split_whitespace().next().unwrap_or("")
}

pub(crate) fn cut(s: &str) -> String {
    if len(s) > 40 {
        s.chars().take(40).collect::<String>() + "…"
    } else {
        s.to_string()
    }
}

/// Coefficient of variation (standard deviation over mean).
fn variation(xs: &[usize]) -> f64 {
    let n = xs.len() as f64;
    let mean = xs.iter().sum::<usize>() as f64 / n;
    let sd = (xs.iter().map(|&x| (x as f64 - mean).powi(2)).sum::<f64>() / n).sqrt();
    if mean == 0.0 { 0.0 } else { sd / mean }
}

/// Every stretch where the same non-empty key repeats at least `min` times.
fn runs<T: Clone>(list: &[T], key_of: impl Fn(&T) -> String, min: usize) -> Vec<(Vec<T>, String)> {
    let mut out = Vec::new();
    let mut run: Vec<T> = Vec::new();
    let mut key = String::new();
    for x in list {
        let k = key_of(x);
        if !k.is_empty() && k == key {
            run.push(x.clone());
        } else {
            if !key.is_empty() && run.len() >= min {
                out.push((std::mem::take(&mut run), key.clone()));
            }
            run = vec![x.clone()];
            key = k;
        }
    }
    if !key.is_empty() && run.len() >= min {
        out.push((run, key));
    }
    out
}

#[derive(Debug, Clone)]
struct Sentence {
    line: usize,
    text: String,
}

/// Every finding in `text`, ordered by line.
pub fn lint(text: &str) -> Vec<Finding> {
    let paras = paragraphs(text);
    let mut findings = Vec::new();
    let mut add = |rule, name, severity, line: Option<usize>, matched: String, hint| {
        findings.push(Finding {
            rule,
            name,
            severity,
            line,
            matched,
            hint,
        })
    };

    let mut narr: Vec<Sentence> = Vec::new();
    let mut lines: Vec<Sentence> = Vec::new();
    // Narration in order, with a gap wherever a paragraph has dialogue: one
    // line of dialogue breaks a run of endings.
    let mut flow: Vec<Option<Sentence>> = Vec::new();
    for p in paras.iter().filter(|p| !p.excluded) {
        if !p.dialogues.is_empty() {
            flow.push(None);
        }
        for s in sentences(&p.narration) {
            let s = Sentence {
                line: p.line,
                text: s,
            };
            narr.push(s.clone());
            flow.push(Some(s));
        }
        for d in &p.dialogues {
            lines.push(Sentence {
                line: p.line,
                text: d.clone(),
            });
        }
    }
    let prose: Vec<&Paragraph> = paras.iter().filter(|p| !p.excluded).collect();
    let all_text = prose
        .iter()
        .map(|p| p.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    for p in &prose {
        let talk = META_ANY.is_match(&p.narration)
            || META_WRITE.is_match(&p.narration)
            || (META_OPENER.is_match(p.narration.trim()) && META_TOPIC.is_match(&p.narration));
        if META_START.is_match(&p.text) || talk {
            add(
                "F1",
                "어시스턴트 말투",
                Severity::Contract,
                Some(p.line),
                cut(&p.text),
                "원고 밖의 설명이나 인사는 지운다",
            );
        }
    }
    for p in &prose {
        if MARKDOWN_MARK.is_match(&p.text) || MARKDOWN_LIST.is_match(&p.text) {
            add(
                "F2",
                "마크다운",
                Severity::Contract,
                Some(p.line),
                cut(&p.text),
                "원고에는 강조 기호와 목록 기호를 쓰지 않는다",
            );
        }
    }
    for p in &prose {
        if let Some(m) = LATIN.find(&p.text) {
            add(
                "F16",
                "영문",
                Severity::Warn,
                Some(p.line),
                m.as_str().into(),
                "설정에 없는 영문이면 한국어로",
            );
        }
    }
    for p in &prose {
        for m in DOUBLE_PASSIVE.find_iter(&p.text) {
            add(
                "F3",
                "겹친 피동",
                Severity::Warn,
                Some(p.line),
                m.as_str().into(),
                "‘보여졌다 → 보였다’처럼 피동을 하나만",
            );
        }
    }
    let ui_nouns: HashSet<&str> = UI_NOUNS.split(' ').collect();
    for s in &narr {
        let ui: Vec<&str> = s
            .text
            .split_whitespace()
            .filter(|w| {
                len(w) >= 2 && w.ends_with('의') && !ui_nouns.iter().any(|n| w.ends_with(n))
            })
            .collect();
        if ui.len() >= 3 || UI_CHAIN.is_match(&s.text) {
            add(
                "F4",
                "'의'가 이어짐",
                Severity::Warn,
                Some(s.line),
                ui.join(" "),
                "‘할머니의 서점 → 할머니 서점’처럼 하나를 뺀다",
            );
        }
    }
    // Past narration ends in -었다 by default, so counting alone would flag
    // people's writing too (checked on 「운수 좋은 날」): lengths must be even.
    for (run, key) in runs(
        &flow,
        |s| {
            s.as_ref()
                .map(|s| ending_class(&s.text))
                .unwrap_or_default()
        },
        6,
    ) {
        let lengths: Vec<usize> = run.iter().flatten().map(|s| len(&s.text)).collect();
        if variation(&lengths) < 0.35 {
            let line = run[0].as_ref().map(|s| s.line);
            add(
                "F5",
                "같은 끝맺음",
                Severity::Warn,
                line,
                format!("{key} {}번, 길이도 비슷함", run.len()),
                "한 문장은 다른 끝맺음으로, 또는 두 문장을 합친다",
            );
        }
    }
    let starts = |s: &Option<Sentence>| match s {
        Some(s) if len(first_word(&s.text)) >= 2 => first_word(&s.text).to_string(),
        _ => String::new(),
    };
    for (run, key) in runs(&flow, starts, 3) {
        let line = run[0].as_ref().map(|s| s.line);
        add(
            "F6",
            "같은 말로 시작",
            Severity::Warn,
            line,
            format!("{key} {}번", run.len()),
            "주어를 빼거나 문장을 합친다",
        );
    }
    let commas: Vec<&Sentence> = narr
        .iter()
        .filter(|s| CONNECTIVE_COMMA.is_match(&s.text))
        .collect();
    let comma_rate = if narr.is_empty() {
        0.0
    } else {
        commas.len() as f64 / narr.len() as f64
    };
    if commas.len() >= 3 && comma_rate > 0.15 {
        add(
            "F7",
            "연결 어미 뒤 쉼표",
            Severity::Warn,
            Some(commas[0].line),
            format!("지문 문장의 {:.0}%", comma_rate * 100.0),
            "‘-고,’ ‘-는데,’ 뒤 쉼표를 뺀다 (사람 글은 4% 안팎)",
        );
    }
    let contrasts: Vec<&str> = NEGATIVE_CONTRAST
        .find_iter(&all_text)
        .map(|m| m.as_str())
        .collect();
    if contrasts.len() >= 2 && contrasts.len() as f64 / len(&all_text).max(1) as f64 >= 1.0 / 1500.0
    {
        let head: String = contrasts[0].trim().chars().take(12).collect();
        let line = paras
            .iter()
            .find(|p| p.text.contains(&head))
            .map(|p| p.line);
        add(
            "F8",
            "부정 대조 반복",
            Severity::Warn,
            line,
            format!("{}번", contrasts.len()),
            "‘A가 아니라 B’는 꼭 필요한 한 곳에만",
        );
    }
    for p in &prose {
        for (expr, hint) in TRANSLATIONESE {
            if p.narration.contains(expr) {
                add(
                    "F9",
                    "번역 투",
                    Severity::Info,
                    Some(p.line),
                    expr.into(),
                    hint,
                );
            }
        }
    }
    for p in &prose {
        for c in CLICHES {
            if p.text.contains(c) {
                add(
                    "F10",
                    "상투어 후보",
                    Severity::Info,
                    Some(p.line),
                    c.into(),
                    "관찰용 목록(검증 전). 이 인물만 할 수 있는 반응인지 본다",
                );
            }
        }
    }
    let dashes = DASH.find_iter(&all_text).count();
    if dashes >= 3 {
        add(
            "F11",
            "줄표",
            Severity::Info,
            None,
            format!("{dashes}개"),
            "한국어 소설에서는 드물다. 쉼표나 문장 나누기로",
        );
    }
    for s in &narr {
        if len(&s.text) > 100 {
            add(
                "F12",
                "긴 문장",
                Severity::Info,
                Some(s.line),
                format!("{}자", len(&s.text)),
                "끊을 곳이 있는지 본다",
            );
        }
    }
    let narr_chars: usize = narr.iter().map(|s| len(&s.text)).sum();
    if narr_chars >= 3000 && !narr.iter().any(|s| len(&s.text) >= 80) {
        add(
            "F13",
            "긴 문장이 없음",
            Severity::Info,
            None,
            "80자 넘는 지문 문장 0개".into(),
            "동작이 이어지는 긴 문장을 하나쯤 섞는다",
        );
    }
    for (run, _) in runs(
        &prose,
        |p| {
            if p.dialogue_only {
                "d".into()
            } else {
                String::new()
            }
        },
        6,
    ) {
        add(
            "F14",
            "대사만 이어짐",
            Severity::Info,
            Some(run[0].line),
            format!("대사 문단 {}개", run.len()),
            "중간에 행동이나 누가 말했는지를 넣는다",
        );
    }
    for l in &lines {
        if mixes_politeness(&l.text) {
            add(
                "F15",
                "한 대사 안 말 단계 섞임",
                Severity::Info,
                Some(l.line),
                cut(&l.text),
                "청자가 바뀐 게 아니면 말 단계를 맞춘다",
            );
        }
    }
    findings.sort_by_key(|f| f.line.unwrap_or(0));
    findings
}

/// Findings of one severity, as JSON for a tool's answer.
pub fn of(findings: &[Finding], severity: Severity) -> Vec<Value> {
    findings
        .iter()
        .filter(|f| f.severity == severity)
        .map(Finding::to_json)
        .collect()
}

#[cfg(test)]
mod tests;
