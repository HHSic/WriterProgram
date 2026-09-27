use std::sync::LazyLock;

use writer_revise::{Analyzer, Issue, Names, Profile, analyze, check};

static ANALYZER: LazyLock<Analyzer> = LazyLock::new(|| Analyzer::new(None).expect("embedded ko-dic"));

fn run(text: &str, profile: Profile, names: &Names) -> Vec<Issue> {
    let doc = analyze(text, &ANALYZER).expect("analysis");
    check(&doc, names, &profile)
}

fn rules(issues: &[Issue]) -> Vec<&'static str> {
    issues.iter().map(|i| i.rule).collect()
}

#[test]
fn a1_flags_three_sentences_starting_alike() {
    let issues = run("그녀는 카드를 들었다. 그녀는 칸을 세었다. 그녀는 손을 멈췄다.", Profile::web_novel(), &Names::default());
    let a1 = issues.iter().find(|i| i.rule == "A1").expect("A1");
    assert_eq!(a1.title, "‘그녀는’으로 시작하는 문장이 3번");
}

#[test]
fn a1_ignores_broken_runs() {
    let issues = run("그녀는 카드를 들었다. 칸마다 이름이 있었다. 그녀는 손을 멈췄다.", Profile::web_novel(), &Names::default());
    assert!(!issues.iter().any(|i| i.rule == "A1" && i.severity.in_text()));
}

#[test]
fn a2_flags_same_endings_by_profile() {
    let text = "그녀는 카드를 들었다. 칸마다 이름이 적혀 있었다. 서하는 손을 멈췄다. 도장이 찍혀 있었다.";
    assert!(rules(&run(text, Profile::print(), &Names::default())).contains(&"A2"));
    assert!(!rules(&run(text, Profile::web_novel(), &Names::default())).contains(&"A2"));
}

#[test]
fn a2_dialogue_breaks_the_run() {
    let text = "그녀는 카드를 들었다. 칸마다 이름이 적혀 있었다.\n“뭐예요?”\n서하는 손을 멈췄다. 도장이 찍혀 있었다.";
    assert!(!rules(&run(text, Profile::print(), &Names::default())).contains(&"A2"));
}

#[test]
fn a3_flags_nearby_repetition() {
    let text = "남자는 봉투를 계산대에 내려놓았다. 봉투 모서리가 젖은 나무에 닿았다. 서하는 봉투를 바라보았다.";
    let issues = run(text, Profile::web_novel(), &Names::default());
    let a3 = issues.iter().find(|i| i.rule == "A3").expect("A3");
    assert!(a3.title.starts_with("‘봉투’가"));
}

#[test]
fn b1_flags_long_sentence_with_cut_points() {
    let text = "카드의 뒷면에는 할머니의 서점의 옛 주소가 연필로 아주 작게 적혀 있었는데 그 주소는 지금 서점이 있는 이 골목이 아니라 지도에서 이미 사라진 골목의 번지였고 서하는 그 골목의 이름을 어디선가 들어본 것 같았다.";
    let issues = run(text, Profile::web_novel(), &Names::default());
    let b1 = issues.iter().find(|i| i.rule == "B1").expect("B1");
    assert!(!b1.points.is_empty(), "expected cut points");
    assert!(rules(&issues).contains(&"C1"));
}

#[test]
fn c2_double_passive_has_safe_fix() {
    let text = "물 자국이 번져 가는 것이 보여졌다. 그 이름은 오래 잊혀진 채였다.";
    let issues = run(text, Profile::web_novel(), &Names::default());
    let fixes: Vec<&str> = issues.iter().filter_map(|i| i.fix.as_ref()).map(|f| f.replacement.as_str()).collect();
    assert!(fixes.contains(&"보였"));
    assert!(fixes.contains(&"잊힌"));
}

#[test]
fn c2_leaves_plain_passive_alone() {
    let issues = run("멀리 불빛이 보여 주었다. 문이 열려 있었다.", Profile::web_novel(), &Names::default());
    assert!(!rules(&issues).contains(&"C2"));
}

#[test]
fn d1_flags_long_dialogue_run() {
    let text = "“정말요?”\n“네.”\n“언제요?”\n“오래전에요.”\n“이름은요?”\n“기억나지 않습니다.”";
    assert!(rules(&run(text, Profile::web_novel(), &Names::default())).contains(&"D1"));
}

#[test]
fn e1_flags_rare_near_name() {
    let names = Names::parse("윤서하,서하");
    let text = "서하는 웃었다. 서하가 문을 열었다. 서하의 손이 떨렸다. 서화는 대답을 기다렸다.";
    let issues = run(text, Profile::web_novel(), &names);
    let e1 = issues.iter().find(|i| i.rule == "E1").expect("E1");
    assert_eq!(e1.fix.as_ref().map(|f| f.replacement.as_str()), Some("서하"));
}

#[test]
fn status_windows_are_not_checked() {
    let text = "[그녀는 레벨이 올랐다. 그녀는 힘이 올랐다. 그녀는 민첩이 올랐다.]";
    assert!(run(text, Profile::web_novel(), &Names::default()).is_empty());
}
