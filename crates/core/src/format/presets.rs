//! Built-in presets, the catalog for the settings screen, and the writer's
//! own presets (내 서식).

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::paper::paper;
use super::{
    FONTS, HeadAlign, IndentRules, ManuscriptFormat, Margins, PAPERS, Paper, RunningFoot,
    RunningHead,
};
use crate::project::ProjectKind;
use crate::store::{atomic_write, read_text};
use crate::{Error, Result};

/// Built-in presets: id, screen name, format.
pub fn builtin_presets() -> Vec<(&'static str, &'static str, ManuscriptFormat)> {
    let submission = ManuscriptFormat {
        preset: "submission-a4".into(),
        paper: paper("a4"),
        // 한글's defaults for A4.
        margins: Margins {
            top: 20.0,
            bottom: 15.0,
            inside: 30.0,
            outside: 30.0,
            header: 15.0,
            footer: 15.0,
        },
        font: "batang".into(),
        size_pt: 10.0,
        line_spacing: 160,
        letter_spacing: 0,
        indent: 1.0,
        blank_line_between: false,
        chapter_new_page: true,
        page_numbers: true,
        page_number_align: HeadAlign::Center,
        header: RunningHead::default(),
        footer: RunningFoot::default(),
        indent_rules: IndentRules {
            margined: true,
            ..IndentRules::default()
        },
    };
    let webnovel = ManuscriptFormat {
        preset: "webnovel".into(),
        paper: Paper {
            kind: "none".into(),
            width_mm: 210.0,
            height_mm: 297.0,
        },
        font: "dotum".into(),
        indent: 0.0,
        blank_line_between: true,
        chapter_new_page: false,
        page_numbers: false,
        ..submission.clone()
    };
    let shinguk = ManuscriptFormat {
        preset: "book-shinguk".into(),
        paper: paper("shinguk"),
        margins: Margins {
            top: 22.0,
            bottom: 20.0,
            inside: 22.0,
            outside: 20.0,
            header: 8.0,
            footer: 8.0,
        },
        font: "noto-serif".into(),
        size_pt: 10.0,
        line_spacing: 180,
        letter_spacing: -3,
        ..submission.clone()
    };
    let small = ManuscriptFormat {
        preset: "book-46".into(),
        paper: paper("46"),
        margins: Margins {
            top: 18.0,
            bottom: 18.0,
            inside: 18.0,
            outside: 15.0,
            header: 7.0,
            footer: 7.0,
        },
        size_pt: 9.5,
        line_spacing: 175,
        ..shinguk.clone()
    };
    vec![
        ("submission-a4", "투고 원고 (A4)", submission),
        ("webnovel", "웹소설 플랫폼", webnovel),
        ("book-shinguk", "책 (신국판)", shinguk),
        ("book-46", "책 (46판)", small),
    ]
}

pub fn builtin(id: &str) -> Option<ManuscriptFormat> {
    builtin_presets()
        .into_iter()
        .find(|(i, ..)| *i == id)
        .map(|(.., f)| f)
}

/// The preset a new project of this kind starts with.
pub fn default_for(kind: ProjectKind) -> ManuscriptFormat {
    builtin(match kind {
        ProjectKind::Webnovel => "webnovel",
        ProjectKind::Print => "submission-a4",
    })
    .expect("built-in preset")
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetEntry {
    pub id: String,
    pub name: String,
    pub format: ManuscriptFormat,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FontEntry {
    pub key: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperEntry {
    pub key: String,
    pub label: String,
    pub width_mm: f64,
    pub height_mm: f64,
}

/// Everything the format settings screen offers.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub builtin: Vec<PresetEntry>,
    pub user: Vec<UserPreset>,
    pub fonts: Vec<FontEntry>,
    pub papers: Vec<PaperEntry>,
}

pub fn catalog(user_presets: &Path) -> Catalog {
    Catalog {
        builtin: builtin_presets()
            .into_iter()
            .map(|(id, name, format)| PresetEntry {
                id: id.into(),
                name: name.into(),
                format,
            })
            .collect(),
        user: load_user_presets(user_presets),
        fonts: FONTS
            .iter()
            .map(|f| FontEntry {
                key: f.key.into(),
                label: f.label.into(),
            })
            .collect(),
        papers: PAPERS
            .iter()
            .map(|(key, label, w, h)| PaperEntry {
                key: (*key).into(),
                label: (*label).into(),
                width_mm: *w,
                height_mm: *h,
            })
            .collect(),
    }
}

// ---------------------------------------------------------------------------
// User presets (내 서식), shared by all projects on this computer.

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserPreset {
    pub name: String,
    pub format: ManuscriptFormat,
}

pub fn load_user_presets(file: &Path) -> Vec<UserPreset> {
    read_text(file)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn write_user_presets(file: &Path, presets: &[UserPreset]) -> Result<()> {
    let text = serde_json::to_string_pretty(presets).expect("presets serialize");
    atomic_write(file, text.as_bytes())
}

/// Saves (or replaces) a user preset under `name`.
pub fn save_user_preset(
    file: &Path,
    name: &str,
    format: &ManuscriptFormat,
) -> Result<Vec<UserPreset>> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Error::Invalid("서식 이름을 적어 주세요".into()));
    }
    if builtin_presets().iter().any(|(_, n, _)| *n == name) {
        return Err(Error::Invalid("기본 서식과 같은 이름은 쓸 수 없음".into()));
    }
    format.validate()?;
    let mut presets = load_user_presets(file);
    let mut format = format.clone();
    format.preset = name.to_string();
    match presets.iter_mut().find(|p| p.name == name) {
        Some(p) => p.format = format,
        None => presets.push(UserPreset {
            name: name.to_string(),
            format,
        }),
    }
    write_user_presets(file, &presets)?;
    Ok(presets)
}

pub fn delete_user_preset(file: &Path, name: &str) -> Result<Vec<UserPreset>> {
    let mut presets = load_user_presets(file);
    presets.retain(|p| p.name != name);
    write_user_presets(file, &presets)?;
    Ok(presets)
}
