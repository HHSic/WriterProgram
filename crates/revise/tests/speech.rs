use std::sync::LazyLock;

use writer_revise::Analyzer;
use writer_revise::speech::{Level, Verdict, judge_line, read_line, self_reference};

static ANALYZER: LazyLock<Analyzer> = LazyLock::new(|| Analyzer::new(None).expect("embedded ko-dic"));

fn verdict(line: &str, expected: Level) -> Verdict {
    judge_line(expected, &read_line(line, &ANALYZER).expect("analysis"))
}

fn levels(line: &str) -> Vec<Vec<Level>> {
    read_line(line, &ANALYZER).expect("analysis").into_iter().map(|s| s.levels.to_vec()).collect()
}

#[test]
fn reads_each_level_from_real_lines() {
    assert_eq!(levels("네, 알겠습니다."), vec![vec![Level::Hasipsio]]);
    assert_eq!(levels("보고서는 내일까지 드릴게요."), vec![vec![Level::Haeyo]]);
    assert_eq!(levels("그만 좀 해. 내가 할게."), vec![vec![Level::Hae], vec![Level::Hae]]);
    assert_eq!(levels("어서 들어가거라."), vec![vec![Level::Haera]]);
    assert_eq!(levels("자네도 한잔하게나."), vec![vec![Level::Hage]]);
    assert_eq!(levels("그대는 어디서 왔소?"), vec![vec![Level::Hao]]);
    assert_eq!(levels("이쪽으로 오시오."), vec![vec![Level::Hao]]);
    assert_eq!(levels("다들 나갑시다."), vec![vec![Level::Hasipsio]]);
}

#[test]
fn fragments_stay_unread() {
    assert_eq!(levels("네. 저요?"), vec![vec![], vec![]]);
    assert_eq!(levels("바다."), vec![Vec::<Level>::new()]);
    assert_eq!(verdict("그게…", Level::Haeyo), Verdict::Unknown);
}

#[test]
fn trailing_off_is_not_an_ending() {
    assert_eq!(levels("무릎이 시원찮은데… 천천히 가 볼게요."), vec![vec![], vec![Level::Haeyo]]);
}

#[test]
fn hage_lines_read_as_hage() {
    assert_eq!(verdict("요즘 대궐 안이 시끄럽다네. 자네도 말을 아끼게.", Level::Hage), Verdict::Match);
}

#[test]
fn connective_endings_left_hanging_are_half_speech_unless_followed_by_yo() {
    assert_eq!(levels("지금은 바쁘니까."), vec![vec![Level::Hae]]);
    assert_eq!(levels("지금은 바쁘니까요."), vec![vec![Level::Haeyo]]);
}

#[test]
fn polite_levels_do_not_stand_in_for_each_other() {
    assert_eq!(verdict("알겠어요. 바로 갈게요.", Level::Hasipsio), Verdict::Mismatch);
    assert_eq!(verdict("알겠습니다.", Level::Haeyo), Verdict::Mismatch);
    assert_eq!(verdict("네. 바로 가겠습니다.", Level::Hasipsio), Verdict::Match);
}

#[test]
fn half_speech_accepts_plain_endings_but_not_polite_ones() {
    assert_eq!(verdict("밥은 먹었냐? 얼른 먹자.", Level::Hae), Verdict::Match);
    assert_eq!(verdict("그래, 알았어. 가 봐요.", Level::Hae), Verdict::Mismatch);
}

#[test]
fn finds_self_reference() {
    assert_eq!(self_reference("제가 할게요.", &ANALYZER).unwrap(), Some("저"));
    assert_eq!(self_reference("내가 갈게.", &ANALYZER).unwrap(), Some("나"));
    assert_eq!(self_reference("저 사람은 누구야?", &ANALYZER).unwrap(), None);
    assert_eq!(self_reference("선생님이 말했지?", &ANALYZER).unwrap(), None);
}
