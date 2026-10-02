//! Setting cards (설정집, S8): people, places, terms and user-made kinds.
//!
//! One card per file in `cards/<id>.md`, readable like a manuscript file:
//!
//! ```text
//! ---
//! id: "k7q2m9x4t1ab"
//! type: "person"
//! name: "윤서하"
//! aliases: ["서하","윤 사장"]
//! fields: [["나이","29"],["직업","서점 주인"]]
//! highlight: true
//! created: "2026-09-27T01:00:00.000Z"
//! ---
//!
//! 할머니에게 서점을 물려받았다.
//! ```
//!
//! Kinds (분류) and their default fields live in `project.json`.

use std::path::{Path, PathBuf};

use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};

use crate::doc::Section;
use crate::doc::{decode_value, encode, split_front_matter};
use crate::markup::{Block, parse_body, write_body};
use crate::search::{Match, find_in_blocks};
use crate::store::{atomic_write, new_id, now_iso, read_text};
use crate::{Error, Result, copies, doc, project};

pub const CARDS_DIR: &str = "cards";

/// Names shorter than this are not looked for in the text (too many false hits).
pub const MIN_NAME_CHARS: usize = 2;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardType {
    pub id: String,
    pub name: String,
    /// Fields a new card of this kind starts with.
    #[serde(default)]
    pub fields: Vec<String>,
}

pub fn default_types() -> Vec<CardType> {
    let t = |id: &str, name: &str, fields: &[&str]| CardType {
        id: id.into(),
        name: name.into(),
        fields: fields.iter().map(|f| (*f).to_string()).collect(),
    };
    vec![
        t("person", "인물", &["나이", "직업", "말투", "외모", "관계"]),
        t("place", "장소", &["위치", "특징"]),
        t("term", "용어", &["정의"]),
    ]
}

/// A card as the screen edits it. The description is plain text; each line is
/// a paragraph in the file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    pub id: String,
    pub card_type: String,
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub fields: Vec<(String, String)>,
    /// Highlight this card's names in the manuscript.
    #[serde(default = "yes")]
    pub highlight: bool,
    #[serde(default)]
    pub created: String,
    #[serde(default)]
    pub description: String,
    /// Front matter lines this version does not know.
    #[serde(skip)]
    pub extra: Vec<(String, String)>,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardSummary {
    pub id: String,
    pub card_type: String,
    pub name: String,
    pub aliases: Vec<String>,
    pub highlight: bool,
    /// One line for lists: the first filled field or the start of the description.
    pub summary: String,
}

impl Card {
    /// Name and aliases long enough to look for in the text, longest first.
    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = std::iter::once(&self.name)
            .chain(&self.aliases)
            .map(|n| n.trim().to_string())
            .filter(|n| n.chars().count() >= MIN_NAME_CHARS)
            .collect();
        names.sort_by_key(|n| std::cmp::Reverse(n.chars().count()));
        names.dedup();
        names
    }

    /// The first three filled-in fields, or else the first line of the
    /// description (등장 설정 shows this under the name).
    pub fn summary(&self) -> CardSummary {
        let from_fields: Vec<String> = self
            .fields
            .iter()
            .filter(|(_, v)| !v.trim().is_empty())
            .take(3)
            .map(|(k, v)| format!("{k}: {}", v.trim()))
            .collect();
        let text = if from_fields.is_empty() {
            self.description
                .lines()
                .find(|l| !l.trim().is_empty())
                .unwrap_or_default()
                .to_string()
        } else {
            from_fields.join(" · ")
        };
        let summary: String = text.chars().take(80).collect();
        CardSummary {
            id: self.id.clone(),
            card_type: self.card_type.clone(),
            name: self.name.clone(),
            aliases: self.aliases.clone(),
            highlight: self.highlight,
            summary,
        }
    }
}

fn description_blocks(text: &str) -> Vec<Block> {
    if text.trim().is_empty() {
        return Vec::new();
    }
    text.split('\n').map(Block::text).collect()
}

fn description_text(blocks: &[Block]) -> String {
    blocks
        .iter()
        .map(|b| b.lines().join(" "))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn parse_card(src: &str, fallback_id: &str) -> Card {
    let (front, body) = split_front_matter(src);
    let mut card = Card {
        id: fallback_id.into(),
        card_type: "term".into(),
        name: String::new(),
        aliases: Vec::new(),
        fields: Vec::new(),
        highlight: true,
        created: String::new(),
        description: description_text(&parse_body(body)),
        extra: Vec::new(),
    };
    for line in front.unwrap_or_default().lines() {
        let Some((key, raw)) = line.split_once(':') else {
            continue;
        };
        let (key, raw) = (key.trim(), raw.trim());
        match key {
            "id" => card.id = decode_value(raw),
            "type" => card.card_type = decode_value(raw),
            "name" => card.name = decode_value(raw),
            "aliases" => card.aliases = serde_json::from_str(raw).unwrap_or_default(),
            "fields" => card.fields = serde_json::from_str(raw).unwrap_or_default(),
            "highlight" => card.highlight = raw != "false",
            "created" => card.created = decode_value(raw),
            _ => card.extra.push((key.into(), raw.into())),
        }
    }
    if card.id.is_empty() {
        card.id = fallback_id.into();
    }
    card
}

pub fn write_card(card: &Card) -> String {
    let mut out = String::from("---\n");
    out.push_str(&format!("id: {}\n", encode(&card.id)));
    out.push_str(&format!("type: {}\n", encode(&card.card_type)));
    out.push_str(&format!("name: {}\n", encode(&card.name)));
    out.push_str(&format!(
        "aliases: {}\n",
        serde_json::to_string(&card.aliases).expect("aliases serialize")
    ));
    out.push_str(&format!(
        "fields: {}\n",
        serde_json::to_string(&card.fields).expect("fields serialize")
    ));
    out.push_str(&format!("highlight: {}\n", card.highlight));
    out.push_str(&format!("created: {}\n", encode(&card.created)));
    for (key, raw) in &card.extra {
        out.push_str(&format!("{key}: {raw}\n"));
    }
    out.push_str("---\n\n");
    out.push_str(&write_body(&description_blocks(&card.description)));
    out
}

fn check_id(id: &str) -> Result<()> {
    if id.is_empty()
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(Error::Invalid("올바르지 않은 카드 이름".into()));
    }
    Ok(())
}

fn path(root: &Path, id: &str) -> Result<PathBuf> {
    check_id(id)?;
    Ok(root.join(CARDS_DIR).join(doc::file_name(id)))
}

pub fn load(root: &Path, id: &str) -> Result<Card> {
    let path = path(root, id)?;
    if !path.is_file() {
        return Err(Error::NotFound("카드를 찾을 수 없음".into()));
    }
    Ok(parse_card(&read_text(&path)?, id))
}

/// Every card, sorted by name. Copies left by sync programs are not cards of
/// their own (copies/).
pub fn load_all(root: &Path) -> Result<Vec<Card>> {
    let dir = root.join(CARDS_DIR);
    let mut cards = Vec::new();
    for id in copies::scan(root, Section::Cards)?.ids {
        // A file a sync program is still writing is left out for now.
        let Ok(text) = read_text(&dir.join(doc::file_name(&id))) else {
            continue;
        };
        cards.push(parse_card(&text, &id));
    }
    cards.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(cards)
}

pub fn list(root: &Path) -> Result<Vec<CardSummary>> {
    Ok(load_all(root)?.iter().map(Card::summary).collect())
}

/// Saves a card as edited on screen.
pub fn save(root: &Path, card: &Card) -> Result<CardSummary> {
    let mut card = card.clone();
    card.name = card.name.trim().to_string();
    if card.name.is_empty() {
        return Err(Error::Invalid("카드 이름을 적어 주세요".into()));
    }
    card.aliases = card
        .aliases
        .iter()
        .map(|a| a.trim().to_string())
        .filter(|a| !a.is_empty() && *a != card.name)
        .collect();
    card.fields.retain(|(k, _)| !k.trim().is_empty());
    let path = path(root, &card.id)?;
    // Keep keys written by newer versions.
    if let Ok(text) = read_text(&path) {
        card.extra = parse_card(&text, &card.id).extra;
    }
    atomic_write(&path, write_card(&card).as_bytes())?;
    Ok(card.summary())
}

/// Makes a card of `type_id` with that kind's default fields, empty.
pub fn create(root: &Path, type_id: &str, name: &str) -> Result<Card> {
    let project = project::load(root)?;
    let kind = project
        .card_types()
        .into_iter()
        .find(|t| t.id == type_id)
        .ok_or_else(|| Error::NotFound("분류를 찾을 수 없음".into()))?;
    let name = name.trim();
    if name.is_empty() {
        return Err(Error::Invalid("카드 이름을 적어 주세요".into()));
    }
    let dir = root.join(CARDS_DIR);
    let id = loop {
        let id = new_id();
        if !dir.join(doc::file_name(&id)).exists() {
            break id;
        }
    };
    let card = Card {
        id,
        card_type: kind.id,
        name: name.into(),
        aliases: Vec::new(),
        fields: kind
            .fields
            .into_iter()
            .map(|f| (f, String::new()))
            .collect(),
        highlight: true,
        created: now_iso(),
        description: String::new(),
        extra: Vec::new(),
    };
    atomic_write(
        &dir.join(doc::file_name(&card.id)),
        write_card(&card).as_bytes(),
    )?;
    Ok(card)
}

// ---------------------------------------------------------------------------
// Where cards appear

/// One pattern for a list of names, longest first so "윤서하" wins over "서하".
pub fn names_regex(names: &[String]) -> Option<Regex> {
    if names.is_empty() {
        return None;
    }
    let pattern = names
        .iter()
        .map(|n| regex::escape(n))
        .collect::<Vec<_>>()
        .join("|");
    RegexBuilder::new(&pattern).size_limit(1 << 22).build().ok()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Appearance {
    pub doc_id: String,
    pub count: usize,
    /// The first few places, with context.
    pub samples: Vec<Match>,
}

/// Chapters where a card's names appear, in manuscript order.
pub fn appearances(root: &Path, card_id: &str) -> Result<Vec<Appearance>> {
    let card = load(root, card_id)?;
    let Some(re) = names_regex(&card.names()) else {
        return Ok(Vec::new());
    };
    let project = project::load(root)?;
    let mut out = Vec::new();
    for id in project.parts.iter().flat_map(|p| &p.docs) {
        let Ok(file) = doc::load(root, id) else {
            continue;
        };
        let matches = find_in_blocks(&file.body, &re, usize::MAX);
        if !matches.is_empty() {
            out.push(Appearance {
                doc_id: id.clone(),
                count: matches.len(),
                samples: matches.into_iter().take(3).collect(),
            });
        }
    }
    Ok(out)
}

/// For each card, in how many chapters it appears.
pub fn appearance_counts(root: &Path) -> Result<Vec<(String, usize)>> {
    let cards = load_all(root)?;
    let patterns: Vec<(String, Regex)> = cards
        .iter()
        .filter_map(|c| names_regex(&c.names()).map(|re| (c.id.clone(), re)))
        .collect();
    let mut counts: Vec<(String, usize)> = cards.iter().map(|c| (c.id.clone(), 0)).collect();
    let project = project::load(root)?;
    for id in project.parts.iter().flat_map(|p| &p.docs) {
        let Ok(file) = doc::load(root, id) else {
            continue;
        };
        let text = crate::markup::plain_text(&file.body);
        for (card_id, re) in &patterns {
            if re.is_match(&text)
                && let Some(entry) = counts.iter_mut().find(|(id, _)| id == card_id)
            {
                entry.1 += 1;
            }
        }
    }
    Ok(counts)
}

// ---------------------------------------------------------------------------
// Kinds (분류)

pub fn add_type(root: &Path, name: &str) -> Result<CardType> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Error::Invalid("분류 이름을 적어 주세요".into()));
    }
    let mut project = project::load(root)?;
    let mut types = project.card_types();
    if types.iter().any(|t| t.name == name) {
        return Err(Error::Invalid("같은 이름의 분류가 있음".into()));
    }
    let kind = CardType {
        id: new_id(),
        name: name.into(),
        fields: Vec::new(),
    };
    types.push(kind.clone());
    project.card_types = Some(types);
    project::save(root, &project)?;
    Ok(kind)
}

/// Renames a kind and sets its default fields.
pub fn update_type(root: &Path, kind: &CardType) -> Result<()> {
    let name = kind.name.trim();
    if name.is_empty() {
        return Err(Error::Invalid("분류 이름을 적어 주세요".into()));
    }
    let mut project = project::load(root)?;
    let mut types = project.card_types();
    let slot = types
        .iter_mut()
        .find(|t| t.id == kind.id)
        .ok_or_else(|| Error::NotFound("분류를 찾을 수 없음".into()))?;
    slot.name = name.into();
    slot.fields = kind
        .fields
        .iter()
        .map(|f| f.trim().to_string())
        .filter(|f| !f.is_empty())
        .collect();
    project.card_types = Some(types);
    project::save(root, &project)
}

/// Removes a kind that no card uses.
pub fn remove_type(root: &Path, type_id: &str) -> Result<()> {
    if load_all(root)?.iter().any(|c| c.card_type == type_id) {
        return Err(Error::Invalid(
            "카드가 남아 있는 분류는 지울 수 없음. 카드를 먼저 다른 분류로 옮기거나 휴지통으로 보내 주세요.".into(),
        ));
    }
    let mut project = project::load(root)?;
    let mut types = project.card_types();
    types.retain(|t| t.id != type_id);
    project.card_types = Some(types);
    project::save(root, &project)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_file_round_trips() {
        let card = Card {
            id: "c1".into(),
            card_type: "person".into(),
            name: "윤서하".into(),
            aliases: vec!["서하".into(), "윤 사장".into()],
            fields: vec![
                ("나이".into(), "29".into()),
                ("말투".into(), "존댓말, \"조용함\"".into()),
            ],
            highlight: false,
            created: "2026-09-27T01:00:00.000Z".into(),
            description: "할머니에게 서점을 물려받았다.\n\n밤에만 연다.".into(),
            extra: vec![("future".into(), "1".into())],
        };
        let text = write_card(&card);
        assert!(text.contains("aliases: [\"서하\",\"윤 사장\"]"));
        assert_eq!(parse_card(&text, "x"), card);
    }

    #[test]
    fn names_are_longest_first_and_skip_short_ones() {
        let card = Card {
            name: "서하".into(),
            aliases: vec!["윤서하".into(), "하".into(), " 서하 ".into()],
            ..parse_card("", "c")
        };
        assert_eq!(card.names(), vec!["윤서하".to_string(), "서하".to_string()]);
        let re = names_regex(&card.names()).unwrap();
        let found: Vec<&str> = re
            .find_iter("윤서하와 서하는")
            .map(|m| m.as_str())
            .collect();
        assert_eq!(found, vec!["윤서하", "서하"]);
    }

    #[test]
    fn summary_prefers_fields() {
        let mut card = parse_card("", "c");
        card.fields = vec![
            ("나이".into(), "".into()),
            ("직업".into(), "서점 주인".into()),
        ];
        card.description = "첫 줄".into();
        assert_eq!(card.summary().summary, "직업: 서점 주인");
        card.fields.extend([
            ("말투".into(), "존댓말".into()),
            ("외모".into(), "안경".into()),
            ("관계".into(), "주인공".into()),
        ]);
        assert_eq!(
            card.summary().summary,
            "직업: 서점 주인 · 말투: 존댓말 · 외모: 안경"
        );
        card.fields.clear();
        assert_eq!(card.summary().summary, "첫 줄");
    }
}
