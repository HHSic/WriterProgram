//! revise-check <원고.txt> [--names 설정집.txt] [--profile webnovel|print] [--dump]

use std::collections::BTreeMap;
use std::error::Error;
use std::path::PathBuf;
use std::time::Instant;

use writer_revise::{Analyzer, Doc, Issue, Names, Profile, analyze, check};

fn main() {
    if let Err(e) = run() {
        eprintln!("오류: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut input = None;
    let mut names_path = None;
    let mut profile = Profile::web_novel();
    let mut dump = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--names" => names_path = args.next().map(PathBuf::from),
            "--profile" => {
                profile = match args.next().as_deref() {
                    Some("print") => Profile::print(),
                    _ => Profile::web_novel(),
                }
            }
            "--dump" => dump = true,
            _ => input = Some(PathBuf::from(arg)),
        }
    }
    let input =
        input.ok_or("사용법: revise-check <원고.txt> [--names 설정집.txt] [--profile webnovel|print] [--dump]")?;
    let text = std::fs::read_to_string(&input)?.trim_start_matches('\u{feff}').replace("\r\n", "\n");
    let names = match &names_path {
        Some(p) => Names::parse(&std::fs::read_to_string(p)?),
        None => Names::default(),
    };
    let csv = names.user_dictionary_csv();
    let dict_path = if csv.is_empty() {
        None
    } else {
        let p = std::env::temp_dir().join("writer-revise-names.csv");
        std::fs::write(&p, csv)?;
        Some(p)
    };

    let t = Instant::now();
    let analyzer = Analyzer::new(dict_path.as_deref()).map_err(|e| e.to_string())?;
    let load = t.elapsed();
    let t = Instant::now();
    let doc = analyze(&text, &analyzer).map_err(|e| e.to_string())?;
    let analyzed = t.elapsed();
    let t = Instant::now();
    let issues = check(&doc, &names, &profile);
    let checked = t.elapsed();

    if dump {
        print_dump(&doc);
    }
    print_report(&doc, &issues, &profile);
    println!(
        "\n시간 · 사전 불러오기 {} ms · 형태소 분석 {} ms · 점검 {} ms · 원고 {}자",
        load.as_millis(),
        analyzed.as_millis(),
        checked.as_millis(),
        text.chars().count()
    );
    Ok(())
}

fn excerpt(s: &str, n: usize) -> String {
    let head: String = s.chars().take(n).collect();
    if s.chars().count() > n { format!("{head}…") } else { head }
}

fn print_dump(doc: &Doc) {
    for s in &doc.sents {
        println!("\n[{:?}] {}", s.kind, &doc.text[s.start..s.end]);
        for t in &s.toks {
            let morphs: Vec<String> = t.morphs.iter().map(|m| format!("{}/{}", m.form, m.pos)).collect();
            println!("  {}\t{}", t.surface, morphs.join(" + "));
        }
    }
    println!();
}

fn print_report(doc: &Doc, issues: &[Issue], p: &Profile) {
    let checked_paras = doc.paras.iter().filter(|x| !x.excluded).count();
    println!(
        "퇴고 점검 · {} 기준 · 문단 {} (점검 제외 {}) · 문장 {}",
        p.label(),
        checked_paras,
        doc.paras.len() - checked_paras,
        doc.sents.len()
    );

    let mut by_para: BTreeMap<usize, Vec<&Issue>> = BTreeMap::new();
    let mut notes = Vec::new();
    for i in issues {
        if i.severity.in_text() {
            by_para.entry(i.para).or_default().push(i);
        } else {
            notes.push(i);
        }
    }

    for (para, list) in &by_para {
        let pr = &doc.paras[*para];
        println!("\n[문단 {} · {}번째 줄] {}", para + 1, pr.line, excerpt(&doc.text[pr.start..pr.end], 28));
        for (k, i) in list.iter().enumerate() {
            if k == p.per_paragraph_cap {
                let rest: Vec<String> = list[k..].iter().map(|x| format!("{} {}", x.rule, x.title)).collect();
                println!("  외 {}건 · {}", rest.len(), rest.join(" / "));
                break;
            }
            println!("  ● {} {}", i.rule, i.title);
            println!("      {}", i.hint);
            let spans: Vec<String> = i.ranges.iter().take(6).map(|r| excerpt(&doc.text[r.0..r.1], 20)).collect();
            println!("      표시: {}", spans.join(" / "));
            if !i.points.is_empty() {
                let points: Vec<&str> = i.points.iter().map(|r| &doc.text[r.0..r.1]).collect();
                println!("      끊을 곳: {}", points.join(", "));
            }
            if let Some(f) = &i.fix {
                println!("      바꾸기: {} → {}", &doc.text[f.start..f.end], f.replacement);
            }
        }
    }

    if !notes.is_empty() {
        println!("\n참고 (본문에는 표시하지 않음)");
        for i in &notes {
            println!("  · {} {} [문단 {}] {}", i.rule, i.title, i.para + 1, i.hint);
        }
    }

    let shown: usize = by_para.values().map(|l| l.len().min(p.per_paragraph_cap)).sum();
    let capped: usize = by_para.values().map(|l| l.len().saturating_sub(p.per_paragraph_cap)).sum();
    println!("\n본문 표시 {shown}곳 · 문단당 {}개가 넘어 줄인 것 {capped}곳 · 참고 {}곳", p.per_paragraph_cap, notes.len());
}
