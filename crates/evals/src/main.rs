//! context-eval: runs the context-fitting sentence eval set against a model
//! and scores the candidates (docs/context-sentences.md).
//!
//! ```text
//! context-eval check  <set.json>
//! context-eval prompt <set.json> --condition x2 [--item R01-1]
//! context-eval run    <set.json> --condition x2 --out results.jsonl
//!                     [--provider anthropic|openai|gemini] [--model NAME] [--item ID] [--limit N]
//! context-eval score  <set.json> <results.jsonl>... [--detail]
//! ```
//!
//! `run` reads the key from ANTHROPIC_API_KEY, OPENAI_API_KEY or
//! GEMINI_API_KEY. It appends to the results file and skips items already
//! there, so a broken run can be started again.

mod prompt;
mod score;
mod set;

use std::collections::{BTreeMap, HashSet};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use writer_ai::Provider;
use writer_ai::client::{Ask, Client};
use writer_revise::Analyzer;
use writer_revise::speech::Verdict;

use prompt::Condition;
use score::CandidateScore;
use set::{Item, Scene};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Record {
    item: String,
    condition: String,
    provider: String,
    model: String,
    #[serde(default)]
    candidates: Vec<Candidate>,
    #[serde(default)]
    new_facts: Vec<String>,
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    #[serde(default)]
    cut: bool,
    error: Option<String>,
    /// The model's answer when it could not be read as JSON.
    raw: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Candidate {
    text: String,
    #[serde(default)]
    angle: String,
}

#[derive(Debug, Deserialize)]
struct ModelAnswer {
    #[serde(default)]
    candidates: Vec<Candidate>,
    #[serde(default)]
    new_facts: Vec<String>,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(e) = run(&args) {
        eprintln!("오류: {e}");
        std::process::exit(1);
    }
}

struct Opts {
    positional: Vec<String>,
    condition: Option<Condition>,
    out: Option<PathBuf>,
    provider: Provider,
    model: Option<String>,
    item: Option<String>,
    limit: Option<usize>,
    detail: bool,
}

fn parse(args: &[String]) -> Result<Opts, String> {
    let mut o = Opts {
        positional: Vec::new(),
        condition: None,
        out: None,
        provider: Provider::Anthropic,
        model: None,
        item: None,
        limit: None,
        detail: false,
    };
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut value = |name: &str| {
            it.next()
                .cloned()
                .ok_or(format!("{name} 다음에 값이 필요함"))
        };
        match a.as_str() {
            "--condition" => {
                o.condition = Some(
                    Condition::parse(&value("--condition")?).ok_or("조건은 x0, x1, x2 중 하나")?,
                )
            }
            "--out" => o.out = Some(PathBuf::from(value("--out")?)),
            "--provider" => {
                o.provider = match value("--provider")?.as_str() {
                    "anthropic" => Provider::Anthropic,
                    "openai" => Provider::Openai,
                    "gemini" => Provider::Gemini,
                    other => return Err(format!("모르는 회사 {other}")),
                }
            }
            "--model" => o.model = Some(value("--model")?),
            "--item" => o.item = Some(value("--item")?),
            "--limit" => o.limit = Some(value("--limit")?.parse().map_err(|_| "--limit은 숫자")?),
            "--detail" => o.detail = true,
            _ => o.positional.push(a.clone()),
        }
    }
    Ok(o)
}

const USAGE: &str = "사용법: context-eval check|prompt|run|score <set.json> … (자세한 것은 evals/context-sentences/README.md)";

fn run(args: &[String]) -> Result<(), String> {
    let o = parse(args)?;
    let (command, rest) = o.positional.split_first().ok_or(USAGE)?;
    let set_path = rest.first().ok_or(USAGE)?;
    let scenes = set::load(Path::new(set_path))?;
    let items: Vec<Item> = set::items(&scenes)
        .into_iter()
        .filter(|i| {
            o.item
                .as_ref()
                .is_none_or(|id| &i.id() == id || &i.scene.id == id)
        })
        .collect();
    if items.is_empty() {
        return Err("고른 항목이 없음".into());
    }
    match command.as_str() {
        "check" => check_set(&scenes, &items),
        "prompt" => {
            let condition = o.condition.ok_or("--condition이 필요함")?;
            println!("<system>\n{}\n</system>\n", prompt::SYSTEM);
            for item in &items {
                println!(
                    "===== {} · {} =====\n{}\n",
                    item.id(),
                    item.scene.title,
                    prompt::user_message(item, condition)
                );
            }
            Ok(())
        }
        "run" => run_model(&o, &items),
        "score" => {
            let files = &rest[1..];
            if files.is_empty() {
                return Err("결과 파일이 필요함".into());
            }
            let analyzer = analyzer()?;
            for f in files {
                score_file(Path::new(f), &items, &analyzer, o.detail)?;
            }
            Ok(())
        }
        _ => Err(USAGE.into()),
    }
}

fn analyzer() -> Result<Analyzer, String> {
    Analyzer::new(None).map_err(|e| e.to_string())
}

/// Validates the set and scores the gold lines, which should all pass.
/// This also measures the speech-level reader against lines written by people.
fn check_set(scenes: &[Scene], items: &[Item]) -> Result<(), String> {
    let analyzer = analyzer()?;
    println!("장면 {} · 항목 {}", scenes.len(), items.len());
    let mut failed = 0;
    let mut unread = 0;
    for item in items {
        let s = score::score_candidate(item, &item.slot.gold, &analyzer);
        if s.level == Verdict::Unknown {
            unread += 1;
        }
        if s.points < 100 && !(s.level == Verdict::Unknown && s.points == 80) {
            failed += 1;
            println!(
                "  {} {}점 · 기대 {} · 읽음 [{}] · {}",
                item.id(),
                s.points,
                item.slot.level,
                s.read.join(", "),
                describe(&s)
            );
            println!("      {}", item.slot.gold);
        }
    }
    let read = items.len() - unread;
    println!(
        "정답 대사 점검: 어긋남 {failed} · 말 단계를 읽은 대사 {read}/{} ({:.1}%)",
        items.len(),
        100.0 * read as f64 / items.len() as f64
    );
    if failed > 0 {
        Err(format!("정답 대사 {failed}개가 기대와 다름"))
    } else {
        Ok(())
    }
}

fn describe(s: &CandidateScore) -> String {
    let mut parts = Vec::new();
    if !s.contract.is_empty() {
        parts.push(format!("계약 위반 {}", s.contract.join(", ")));
    }
    match s.level {
        Verdict::Mismatch => parts.push("말 단계 어긋남".to_string()),
        Verdict::Unknown => parts.push("말 단계 판정 불가".to_string()),
        Verdict::Match => {}
    }
    if !s.bad_address.is_empty() {
        parts.push(format!("호칭 {}", s.bad_address.join(", ")));
    }
    if let Some(found) = s.bad_self {
        parts.push(format!("자칭 {found}"));
    }
    if !s.style.is_empty() {
        parts.push(format!("문체 {}", s.style.join(", ")));
    }
    if parts.is_empty() {
        "문제 없음".into()
    } else {
        parts.join(" · ")
    }
}

fn run_model(o: &Opts, items: &[Item]) -> Result<(), String> {
    let condition = o.condition.ok_or("--condition이 필요함")?;
    let out = o.out.as_ref().ok_or("--out이 필요함")?;
    let var = format!("{}_API_KEY", o.provider.key().to_uppercase());
    let key = std::env::var(&var).map_err(|_| format!("{var} 환경 변수에 API 키를 넣어 주세요"))?;
    let model = o
        .model
        .clone()
        .unwrap_or_else(|| o.provider.check_model().to_string());
    let client = Client::new(o.provider, &key, o.provider.base());

    let done: HashSet<String> = read_records(out)
        .unwrap_or_default()
        .into_iter()
        .filter(|r| r.condition == condition.key() && r.model == model && r.error.is_none())
        .map(|r| r.item)
        .collect();
    let todo: Vec<&Item> = items
        .iter()
        .filter(|i| !done.contains(&i.id()))
        .take(o.limit.unwrap_or(usize::MAX))
        .collect();
    eprintln!(
        "{} · {} · 남은 항목 {} (이미 한 것 {})",
        condition.key(),
        model,
        todo.len(),
        done.len()
    );

    if let Some(dir) = out.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(out)
        .map_err(|e| e.to_string())?;
    for (n, item) in todo.iter().enumerate() {
        let ask = Ask {
            system: prompt::SYSTEM.to_string(),
            user: prompt::user_message(item, condition),
            max_tokens: 2000,
            json: true,
        };
        let mut record = Record {
            item: item.id(),
            condition: condition.key().into(),
            provider: o.provider.key().into(),
            model: model.clone(),
            candidates: Vec::new(),
            new_facts: Vec::new(),
            input_tokens: None,
            output_tokens: None,
            cut: false,
            error: None,
            raw: None,
        };
        match client.ask(&model, &ask) {
            Ok(answer) => {
                record.cut = answer.cut;
                record.input_tokens = answer.usage.and_then(|u| u.input_tokens);
                record.output_tokens = answer.usage.and_then(|u| u.output_tokens);
                match read_answer(&answer.text) {
                    Some(a) => {
                        record.candidates = a.candidates;
                        record.new_facts = a.new_facts;
                    }
                    None => {
                        record.error = Some("답을 JSON으로 읽지 못함".into());
                        record.raw = Some(answer.text);
                    }
                }
            }
            Err(e) => record.error = Some(e.user_message()),
        }
        eprintln!(
            "  [{}/{}] {} {}",
            n + 1,
            todo.len(),
            record.item,
            record.error.as_deref().unwrap_or("완료")
        );
        let line = serde_json::to_string(&record).expect("record serializes");
        writeln!(file, "{line}").map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn read_answer(text: &str) -> Option<ModelAnswer> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    serde_json::from_str(&text[start..=end]).ok()
}

fn read_records(path: &Path) -> Result<Vec<Record>, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut out = Vec::new();
    for (n, line) in std::io::BufReader::new(file).lines().enumerate() {
        let line = line.map_err(|e| e.to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        out.push(
            serde_json::from_str(&line)
                .map_err(|e| format!("{}:{}: {e}", path.display(), n + 1))?,
        );
    }
    Ok(out)
}

#[derive(Default)]
struct Tally {
    total: usize,
    matched: usize,
}

impl Tally {
    fn add(&mut self, ok: bool) {
        self.total += 1;
        self.matched += ok as usize;
    }

    fn rate(&self) -> String {
        if self.total == 0 {
            "-".into()
        } else {
            format!(
                "{:.1}% ({}/{})",
                100.0 * self.matched as f64 / self.total as f64,
                self.matched,
                self.total
            )
        }
    }
}

fn score_file(
    path: &Path,
    items: &[Item],
    analyzer: &Analyzer,
    detail: bool,
) -> Result<(), String> {
    let records = read_records(path)?;
    let by_id: BTreeMap<String, &Item> = items.iter().map(|i| (i.id(), i)).collect();
    // Results may hold several conditions or models; report each on its own.
    let mut groups: BTreeMap<(String, String), Vec<&Record>> = BTreeMap::new();
    for r in &records {
        groups
            .entry((r.condition.clone(), format!("{}/{}", r.provider, r.model)))
            .or_default()
            .push(r);
    }
    for ((condition, model), records) in groups {
        let mut errors = 0;
        let mut candidates = 0;
        let mut contract = 0;
        let (mut matched, mut mismatched, mut unread) = (0, 0, 0);
        let mut polite = Tally::default();
        let mut bad_address = 0;
        let mut bad_self = 0;
        let mut best_points = Vec::new();
        let mut all_points = Vec::new();
        let mut sims = Vec::new();
        let mut all_good = Tally::default();
        let mut by_type: BTreeMap<String, Tally> = BTreeMap::new();
        let mut by_tag: BTreeMap<String, Tally> = BTreeMap::new();
        let mut tokens = (0u64, 0u64);
        let mut lines = Vec::new();

        for r in records {
            let Some(item) = by_id.get(&r.item) else {
                continue;
            };
            tokens.0 += r.input_tokens.unwrap_or(0);
            tokens.1 += r.output_tokens.unwrap_or(0);
            if r.error.is_some() || r.candidates.is_empty() {
                errors += 1;
                continue;
            }
            let scores: Vec<CandidateScore> = r
                .candidates
                .iter()
                .map(|c| score::score_candidate(item, &c.text, analyzer))
                .collect();
            let texts: Vec<&str> = r.candidates.iter().map(|c| c.text.as_str()).collect();
            if let Some(s) = score::similarity(&texts) {
                sims.push(s);
            }
            best_points.push(scores.iter().map(|s| s.points).max().unwrap_or(0));
            all_good.add(scores.iter().all(|s| s.level != Verdict::Mismatch));
            for s in &scores {
                candidates += 1;
                all_points.push(s.points);
                contract += !s.contract.is_empty() as usize;
                bad_address += !s.bad_address.is_empty() as usize;
                bad_self += s.bad_self.is_some() as usize;
                match s.level {
                    Verdict::Match => matched += 1,
                    Verdict::Mismatch => mismatched += 1,
                    Verdict::Unknown => unread += 1,
                }
                if let Some(p) = s.polite_match(item) {
                    polite.add(p);
                }
                if s.level != Verdict::Unknown {
                    let ok = s.level == Verdict::Match;
                    by_type.entry(item.scene.kind.clone()).or_default().add(ok);
                    for t in &item.slot.tags {
                        by_tag.entry(t.clone()).or_default().add(ok);
                    }
                }
                if detail && s.points < 100 {
                    lines.push(format!(
                        "  {} {}점 · 기대 {} · 읽음 [{}] · {}\n      {}",
                        item.id(),
                        s.points,
                        item.slot.level,
                        s.read.join(", "),
                        describe(s),
                        s.text.replace('\n', " ")
                    ));
                }
            }
        }

        let mean = |v: &[u32]| {
            if v.is_empty() {
                0.0
            } else {
                v.iter().sum::<u32>() as f64 / v.len() as f64
            }
        };
        let level = Tally {
            total: matched + mismatched,
            matched,
        };
        println!(
            "\n조건 {condition} · {model} · 항목 {} · 후보 {candidates} · 오류 {errors}",
            best_points.len() + errors
        );
        println!(
            "점수: 항목 최고 후보 평균 {:.1} · 후보 평균 {:.1}",
            mean(&best_points),
            mean(&all_points)
        );
        println!(
            "계약 위반 후보: {contract} ({:.1}%)",
            if candidates == 0 {
                0.0
            } else {
                100.0 * contract as f64 / candidates as f64
            }
        );
        println!(
            "말 단계: 일치 {matched} · 어긋남 {mismatched} · 판정 불가 {unread} → 일치율 {}",
            level.rate()
        );
        println!("높임·안높임 일치율: {}", polite.rate());
        println!("후보 셋 모두 말 단계가 맞은 항목: {}", all_good.rate());
        println!("호칭 위반 후보 {bad_address} · 자칭 위반 후보 {bad_self}");
        if !sims.is_empty() {
            println!(
                "후보 유사도(어절 바이그램 자카드 평균): {:.2}",
                sims.iter().sum::<f64>() / sims.len() as f64
            );
        }
        println!("토큰: 입력 {} · 출력 {}", tokens.0, tokens.1);
        println!("유형별 말 단계 일치율:");
        for (k, t) in &by_type {
            println!("  {k}: {}", t.rate());
        }
        println!("태그별 말 단계 일치율:");
        for (k, t) in &by_tag {
            println!("  {k}: {}", t.rate());
        }
        if detail && !lines.is_empty() {
            println!("100점이 아닌 후보:");
            for l in lines {
                println!("{l}");
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers_are_read_even_with_text_around_the_json() {
        let a = read_answer("```json\n{\"candidates\": [{\"text\": \"“가자.”\", \"angle\": \"짧게\"}], \"new_facts\": []}\n```").unwrap();
        assert_eq!(a.candidates.len(), 1);
        assert!(read_answer("후보를 쓸 수 없습니다").is_none());
    }
}
