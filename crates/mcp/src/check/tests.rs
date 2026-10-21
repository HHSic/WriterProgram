//! The port must judge exactly as the JS checker does: every case in the
//! shared fixture, made by `plugins/kfiction/scripts/lint-cases.mjs`.

use serde_json::Value;

use super::speech::read_line;
use super::{Severity, lint};

const FIXTURE: &str = include_str!("../../../../plugins/kfiction/test/fixtures/lint-cases.json");

fn fixture() -> Value {
    serde_json::from_str(FIXTURE).expect("fixture is JSON")
}

#[test]
fn findings_match_the_js_checker() {
    let fixture = fixture();
    let cases = fixture["cases"].as_array().unwrap();
    assert!(cases.len() >= 10);
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let got: Vec<Value> = lint(case["text"].as_str().unwrap())
            .iter()
            .map(|f| {
                serde_json::json!({
                    "rule": f.rule,
                    "severity": f.severity.key(),
                    "line": f.line,
                    "match": f.matched,
                })
            })
            .collect();
        assert_eq!(&got, case["findings"].as_array().unwrap(), "case: {name}");
    }
}

#[test]
fn speech_levels_match_the_js_checker() {
    let fixture = fixture();
    for case in fixture["speech"].as_array().unwrap() {
        let line = case["line"].as_str().unwrap();
        let got: Vec<Vec<&str>> = read_line(line).into_iter().map(|(_, l)| l).collect();
        let want: Vec<Vec<&str>> = case["levels"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| {
                l.as_array()
                    .unwrap()
                    .iter()
                    .map(|x| x.as_str().unwrap())
                    .collect()
            })
            .collect();
        assert_eq!(got, want, "line: {line}");
    }
}

#[test]
fn contract_breaks_are_told_apart() {
    let found = lint("다음은 이어서 쓴 장면입니다.\n\n그는 문을 열었다. **비**가 왔다.");
    let contract: Vec<_> = found
        .iter()
        .filter(|f| f.severity == Severity::Contract)
        .map(|f| f.rule)
        .collect();
    assert_eq!(contract, ["F1", "F2"]);
}
