//! The eval set: scenes with characters, the relation table, a scene card,
//! the manuscript before the slot, and the slots to fill.

use std::collections::HashSet;
use std::path::Path;

use serde::Deserialize;
use writer_revise::speech::Level;

#[derive(Debug, Clone, Deserialize)]
pub struct Scene {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub title: String,
    pub characters: Vec<Character>,
    pub relations: Vec<Relation>,
    pub scene: Card,
    pub before: String,
    pub slots: Vec<Slot>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Character {
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub role: String,
    /// The single 말투 field the setting book has today.
    pub speech: String,
    #[serde(default)]
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Relation {
    pub from: String,
    pub to: String,
    pub level: String,
    #[serde(default)]
    pub address: Vec<String>,
    #[serde(rename = "self")]
    pub self_ref: Option<String>,
    #[serde(default)]
    pub switch: Vec<Switch>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Switch {
    pub when: String,
    pub level: String,
    #[serde(default)]
    pub address: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Card {
    pub pov: String,
    pub tense: String,
    pub place: String,
    pub time: String,
    pub goal: String,
    #[serde(default)]
    pub beats: Vec<String>,
    pub switch_note: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Slot {
    pub n: u32,
    pub speaker: String,
    pub listener: String,
    pub note: String,
    pub hidden: Option<String>,
    /// Text that happened after `before`, for this slot only.
    pub extra: Option<String>,
    /// The level the line must use.
    pub level: String,
    /// Address terms the line may use for the listener.
    #[serde(default)]
    pub address: Vec<String>,
    /// More terms the line must not use, besides other relations' terms.
    #[serde(default)]
    pub address_bad: Vec<String>,
    /// Acceptable self-reference; only 저 and 나 are checked.
    #[serde(rename = "self", default)]
    pub self_ok: Vec<String>,
    /// A reference line written with the set. Never shown to the model.
    pub gold: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// One scene and one of its slots.
#[derive(Debug, Clone, Copy)]
pub struct Item<'a> {
    pub scene: &'a Scene,
    pub slot: &'a Slot,
}

impl Item<'_> {
    pub fn id(&self) -> String {
        format!("{}-{}", self.scene.id, self.slot.n)
    }

    pub fn level(&self) -> Level {
        Level::from_label(&self.slot.level).expect("checked when loading")
    }

    /// Address terms that point at the listener but are not allowed here:
    /// every term any relation uses for the listener, plus `address_bad`.
    pub fn bad_addresses(&self) -> Vec<String> {
        let allowed: HashSet<&str> = self.slot.address.iter().map(String::as_str).collect();
        let mut out: Vec<String> = Vec::new();
        let mut push = |t: &str| {
            if !allowed.contains(t) && !out.iter().any(|o| o == t) {
                out.push(t.to_string());
            }
        };
        for r in self
            .scene
            .relations
            .iter()
            .filter(|r| r.to == self.slot.listener)
        {
            r.address.iter().for_each(|t| push(t));
            r.switch
                .iter()
                .flat_map(|s| &s.address)
                .for_each(|t| push(t));
        }
        self.slot.address_bad.iter().for_each(|t| push(t));
        out
    }

    pub fn relation(&self, from: &str, to: &str) -> Option<&Relation> {
        self.scene
            .relations
            .iter()
            .find(|r| r.from == from && r.to == to)
    }

    /// The characters as a revise name list: one card per line, `이름,다른 이름`.
    pub fn name_list(&self) -> String {
        self.scene
            .characters
            .iter()
            .map(|c| {
                std::iter::once(c.name.as_str())
                    .chain(c.aliases.iter().map(String::as_str))
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

pub fn load(path: &Path) -> Result<Vec<Scene>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let scenes: Vec<Scene> =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    let problems = validate(&scenes);
    if problems.is_empty() {
        Ok(scenes)
    } else {
        Err(problems.join("\n"))
    }
}

pub fn items(scenes: &[Scene]) -> Vec<Item<'_>> {
    scenes
        .iter()
        .flat_map(|scene| scene.slots.iter().map(move |slot| Item { scene, slot }))
        .collect()
}

/// Structural problems in the set: unknown names or levels, missing relations.
pub fn validate(scenes: &[Scene]) -> Vec<String> {
    let mut out = Vec::new();
    let mut ids = HashSet::new();
    for s in scenes {
        if !ids.insert(&s.id) {
            out.push(format!("{}: 장면 ID가 겹침", s.id));
        }
        let names: HashSet<&str> = s.characters.iter().map(|c| c.name.as_str()).collect();
        for r in &s.relations {
            for who in [&r.from, &r.to] {
                if !names.contains(who.as_str()) {
                    out.push(format!("{}: 관계 표의 {who}가 인물 목록에 없음", s.id));
                }
            }
            for level in std::iter::once(&r.level).chain(r.switch.iter().map(|w| &w.level)) {
                if Level::from_label(level).is_none() {
                    out.push(format!("{}: 모르는 말 단계 {level}", s.id));
                }
            }
        }
        for slot in &s.slots {
            let at = format!("{}-{}", s.id, slot.n);
            if Level::from_label(&slot.level).is_none() {
                out.push(format!("{at}: 모르는 말 단계 {}", slot.level));
            }
            if !s
                .relations
                .iter()
                .any(|r| r.from == slot.speaker && r.to == slot.listener)
            {
                out.push(format!(
                    "{at}: {}→{} 관계가 관계 표에 없음",
                    slot.speaker, slot.listener
                ));
            }
        }
    }
    out
}
