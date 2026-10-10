//! Local, context-based suggestions for setting cards. No manuscript leaves the device.
use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::markup::{Block, inline_text};
use crate::search::{Match, find_in_blocks_by};
use crate::{Error, Result, cards, doc, project};

static WORD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[가-힣A-Za-z][가-힣A-Za-z·\-]{1,39}").unwrap());
const COMMON: &[&str] = &[
    "그녀",
    "그들",
    "그것",
    "이것",
    "저것",
    "여자",
    "남자",
    "사람",
    "아이",
    "모두",
    "누군가",
    "아무도",
    "목소리",
    "바람",
    "한숨",
    "고개",
    "생각",
    "마음",
    "시간",
    "사실",
    "감정",
    "말",
    "대답",
    "일",
    "이름",
    "장면",
    "가게",
    "서점",
    "학교",
    "도시",
    "오늘",
    "내일",
    "어제",
    "아침",
    "다음",
    "지금",
    "갑자기",
    "우리",
    "당신",
    "자신",
    "상대",
    "주인",
    "손님",
    "할머니",
    "아버지",
    "어머니",
    "선생",
    "문제",
    "이유",
    "기억",
    "꿈",
    "눈",
    "손",
    "소리",
    "모습",
    "침묵",
    "얼굴",
    "그곳",
    "이곳",
    "저곳",
    "여기",
    "거기",
    "어디",
    "거리",
    "앞",
    "뒤",
    "안",
    "밖",
    "그때",
    "그날",
    "계속",
    "다시",
    "함께",
    "조용히",
];
const PLACES: &[&str] = &[
    "서점",
    "도서관",
    "학교",
    "왕국",
    "제국",
    "대륙",
    "마을",
    "항구",
    "도시",
    "골목",
    "카페",
    "병원",
    "공원",
    "서고",
    "저택",
    "광장",
    "동굴",
    "시장",
    "지하실",
    "거리",
    "숲",
    "산",
    "강",
    "바다",
    "궁",
    "성",
    "역",
    "탑",
    "섬",
];
const SPEECH: &[&str] = &[
    "말했",
    "말하",
    "말을",
    "물었",
    "물어",
    "대답",
    "외쳤",
    "외치",
    "속삭",
    "웃었",
    "웃으며",
    "고개를",
    "생각했",
    "돌아보",
    "바라보",
    "한숨",
    "인사",
    "걸어",
    "손을",
    "눈을",
];
const TRAVEL: &[&str] = &[
    "갔",
    "가서",
    "향했",
    "향하",
    "들어",
    "도착",
    "살았",
    "살고",
    "머물",
    "떠났",
    "떠나",
    "돌아",
    "지났",
    "지나",
    "서 있었",
    "서있",
    "만났",
    "만나",
    "걸었",
    "걸어",
    "찾아",
];
const LIMIT: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Person,
    Place,
}
impl Kind {
    fn id(self) -> &'static str {
        match self {
            Self::Person => "person",
            Self::Place => "place",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sample {
    pub doc_id: String,
    pub hit: Match,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub name: String,
    pub kind: Kind,
    pub count: usize,
    pub samples: Vec<Sample>,
    pub existing: Option<String>,
}
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Discovered {
    pub candidates: Vec<Candidate>,
    pub skipped: usize,
    pub truncated: bool,
}
#[derive(Debug, Deserialize)]
pub struct Pick {
    pub name: String,
    pub kind: Kind,
}

fn valid(name: &str) -> bool {
    let length = name.chars().count();
    (2..=30).contains(&length) && !COMMON.contains(&name)
}
fn tail(text: &str, end: usize) -> String {
    text[end..]
        .chars()
        .take_while(|c| !matches!(c, '.' | '!' | '?' | '\n'))
        .take(50)
        .collect()
}
fn strip<'a>(word: &'a str, suffixes: &[&str]) -> Option<&'a str> {
    suffixes.iter().find_map(|s| {
        word.strip_suffix(s)
            .filter(|n| (2..=30).contains(&n.chars().count()))
    })
}
fn suggest(text: &str, found: &mut BTreeMap<String, Kind>) {
    let words: Vec<_> = WORD.find_iter(text).collect();
    for (i, word) in words.iter().enumerate() {
        let raw = word.as_str();
        let following = tail(text, word.end());
        let previous = i.checked_sub(1).map(|n| &words[n]);
        let adjacent =
            previous.filter(|p| text[p.end()..word.start()].chars().all(char::is_whitespace));
        let person = strip(
            raw,
            &[
                "에게는",
                "한테는",
                "에게",
                "한테",
                "은",
                "는",
                "이",
                "가",
                "와",
                "과",
            ],
        );
        if let Some(name) = person
            && valid(name)
            && name.chars().count() <= 12
            && !PLACES.iter().any(|s| name.ends_with(s))
            && SPEECH.iter().any(|s| following.contains(s))
        {
            found.entry(name.into()).or_insert(Kind::Person);
        }
        let honorific = strip(
            raw,
            &[
                "씨는", "씨가", "씨", "님은", "님이", "님", "군은", "군이", "군",
            ],
        )
        .or_else(|| {
            matches!(
                raw,
                "씨" | "씨는" | "씨가" | "님" | "님은" | "님이" | "선생님" | "박사" | "공작"
            )
            .then(|| adjacent.map(|m| m.as_str()))
            .flatten()
            .filter(|n| valid(n))
        });
        if let Some(name) = honorific {
            found.entry(name.into()).or_insert(Kind::Person);
        }
        if let Some(base) = strip(
            raw,
            &[
                "에서는",
                "에서도",
                "에서",
                "으로는",
                "으로",
                "에선",
                "에는",
                "에도",
                "에",
                "로는",
                "로",
            ],
        ) {
            let noun = PLACES.iter().find(|s| base.ends_with(**s));
            let mut name = base.to_string();
            if noun.is_some_and(|s| base == *s)
                && let Some(p) = adjacent
                && valid(p.as_str())
                && !["은", "는", "을", "를", "가", "이", "한", "던", "된", "운"]
                    .iter()
                    .any(|s| p.as_str().ends_with(s))
            {
                name = format!("{} {base}", p.as_str());
            }
            if valid(&name) && (noun.is_some() || TRAVEL.iter().any(|s| following.contains(s))) {
                found.insert(name, Kind::Place);
            }
        }
        if found.len() > LIMIT {
            return;
        }
    }
}

pub fn scan(root: &Path, doc_ids: Option<&[String]>) -> Result<Discovered> {
    let project = project::load(root)?;
    let ids: Vec<_> = project
        .parts
        .iter()
        .flat_map(|p| &p.docs)
        .filter(|id| doc_ids.is_none_or(|list| list.contains(id)))
        .collect();
    let mut output = Discovered::default();
    let mut documents = Vec::new();
    let mut names = BTreeMap::new();
    for id in ids {
        match doc::load(root, id) {
            Ok(file) => {
                for block in &file.body {
                    if names.len() > LIMIT {
                        break;
                    }
                    if let Block::Paragraph { content, .. } = block {
                        suggest(&inline_text(content), &mut names);
                    }
                }
                documents.push((id, file.body));
            }
            Err(_) => output.skipped += 1,
        }
    }
    output.truncated = names.len() > LIMIT;
    let existing = cards::list(root)?;
    let mut candidates: BTreeMap<_, _> = names
        .into_iter()
        .take(LIMIT)
        .map(|(name, kind)| {
            let card = existing
                .iter()
                .find(|c| c.name == name || c.aliases.contains(&name));
            (
                name.clone(),
                Candidate {
                    name,
                    kind,
                    count: 0,
                    samples: vec![],
                    existing: card.map(|c| c.id.clone()),
                },
            )
        })
        .collect();
    let mut keys: Vec<_> = candidates.keys().cloned().collect();
    keys.sort_by_key(|s| std::cmp::Reverse(s.len()));
    if let Some(re) = cards::names_regex(&keys) {
        for (id, body) in documents {
            for hit in find_in_blocks_by(&body, usize::MAX, |text| cards::find_names(&re, text)) {
                if let Some(candidate) = candidates.get_mut(&hit.text) {
                    candidate.count += 1;
                    if candidate.samples.len() < 3 {
                        candidate.samples.push(Sample {
                            doc_id: id.clone(),
                            hit,
                        });
                    }
                }
            }
        }
    }
    output.candidates = candidates.into_values().filter(|c| c.count > 0).collect();
    output
        .candidates
        .sort_by(|a, b| b.count.cmp(&a.count).then(a.name.cmp(&b.name)));
    Ok(output)
}

/// Add only selected suggestions, and prevent duplicates including existing aliases.
pub fn add(root: &Path, picks: &[Pick]) -> Result<Vec<String>> {
    let project = project::load(root)?;
    let types = project.card_types();
    if picks.len() > LIMIT
        || picks
            .iter()
            .any(|p| !valid(p.name.trim()) || !types.iter().any(|t| t.id == p.kind.id()))
    {
        return Err(Error::Invalid(
            "이름과 인물·장소 분류를 확인해 주세요".into(),
        ));
    }
    let mut known: HashSet<_> = cards::list(root)?
        .into_iter()
        .flat_map(|c| std::iter::once(c.name).chain(c.aliases))
        .collect();
    let mut made = Vec::new();
    for pick in picks {
        let name = pick.name.trim();
        if !known.insert(name.to_string()) {
            continue;
        }
        match cards::create(root, pick.kind.id(), name) {
            Ok(card) => made.push(card.id),
            Err(error) => {
                for id in &made {
                    let _ =
                        std::fs::remove_file(root.join(cards::CARDS_DIR).join(doc::file_name(id)));
                }
                return Err(error);
            }
        }
    }
    Ok(made)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn suggests_names_from_speech_honorifics_and_travel_but_not_common_subjects() {
        let mut found = BTreeMap::new();
        suggest(
            "윤서하가 말했다. 엘리시아는 고개를 끄덕였다. 민준 씨는 웃었다. 그녀는 물었다. 바람이 속삭였다. 둘은 달빛 서점에서 만났다. 지하 서고에 들어갔다. 서울로 향했다. 마음에 들어왔다.",
            &mut found,
        );
        assert_eq!(found.get("윤서하"), Some(&Kind::Person));
        assert_eq!(found.get("엘리시아"), Some(&Kind::Person));
        assert_eq!(found.get("민준"), Some(&Kind::Person));
        assert_eq!(found.get("달빛 서점"), Some(&Kind::Place));
        assert_eq!(found.get("지하 서고"), Some(&Kind::Place));
        assert_eq!(found.get("서울"), Some(&Kind::Place));
        for name in ["그녀", "바람", "마음", "둘은 달빛 서점"] {
            assert!(!found.contains_key(name), "{name}");
        }
    }
}

#[cfg(test)]
mod project_tests {
    use super::*;
    #[test]
    fn scans_saved_manuscripts_with_utf16_samples_and_registers_only_unique_picks() {
        let temp = tempfile::tempdir().unwrap();
        let root = project::create(&project::NewProject {
            parent: temp.path().to_string_lossy().into_owned(),
            title: "후보 검증".into(),
            kind: project::ProjectKind::Webnovel,
            per_doc_goal: None,
            count_spaces: true,
            first_chapter: true,
            platform: None,
        })
        .unwrap();
        let id = project::load(&root).unwrap().parts[0].docs[0].clone();
        doc::save_body(
            &root,
            &id,
            vec![Block::text(
                "😀 윤서하가 말했다. 윤서하가 웃었다. 달빛 서점에서 만났다.",
            )],
            chrono::Duration::minutes(10),
            Default::default(),
        )
        .unwrap();
        let card = cards::create(&root, "person", "서하").unwrap();
        let mut card = card;
        card.aliases.push("윤서하".into());
        cards::save(&root, &card).unwrap();
        let found = scan(&root, Some(std::slice::from_ref(&id))).unwrap();
        let person = found
            .candidates
            .iter()
            .find(|c| c.name == "윤서하")
            .unwrap();
        assert_eq!(person.existing.as_deref(), Some(card.id.as_str()));
        assert_eq!(person.count, 2);
        assert_eq!(person.samples[0].hit.start, 3);
        let picks = [
            Pick {
                name: "윤서하".into(),
                kind: Kind::Person,
            },
            Pick {
                name: "달빛 서점".into(),
                kind: Kind::Place,
            },
            Pick {
                name: "달빛 서점".into(),
                kind: Kind::Place,
            },
        ];
        assert_eq!(add(&root, &picks).unwrap().len(), 1);
        assert!(add(&root, &picks).unwrap().is_empty());
        assert_eq!(cards::list(&root).unwrap().len(), 2);
        assert!(scan(&root, Some(&[])).unwrap().candidates.is_empty());
        assert!(
            add(
                &root,
                &[
                    Pick {
                        name: "새인물".into(),
                        kind: Kind::Person
                    },
                    Pick {
                        name: "".into(),
                        kind: Kind::Place
                    }
                ]
            )
            .is_err()
        );
        assert_eq!(cards::list(&root).unwrap().len(), 2);
    }
}
