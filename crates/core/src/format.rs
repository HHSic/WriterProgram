//! Manuscript format (원고 서식): how the manuscript looks on paper.
//!
//! It is a project setting used by docx/HWPX export and by the page estimate.
//! The writing screen has its own settings in the app, which never touch the
//! files (docs/mvp-scope.md "서식 모델"). Units follow 한글: mm for paper and
//! margins, pt for type size, % for line and letter spacing.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::project::ProjectKind;
use crate::store::{atomic_write, read_text};
use crate::{Error, Result};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManuscriptFormat {
    /// Id of the built-in preset or name of the user preset it started from.
    #[serde(default)]
    pub preset: String,
    pub paper: Paper,
    pub margins: Margins,
    /// Key into [`FONTS`].
    pub font: String,
    pub size_pt: f64,
    /// Line spacing in percent of the type size, as in 한글 (160 = 160%).
    pub line_spacing: u32,
    /// Letter spacing (자간) in percent of the type size, as in 한글.
    pub letter_spacing: i32,
    /// First-line indent in characters.
    pub indent: f64,
    /// An empty line between paragraphs.
    pub blank_line_between: bool,
    /// Each chapter starts on a new page.
    pub chapter_new_page: bool,
    /// Page numbers at the bottom of the pages.
    pub page_numbers: bool,
    /// Where the page number sits.
    #[serde(default)]
    pub page_number_align: HeadAlign,
    /// 머리말: a line at the top of the pages.
    #[serde(default)]
    pub header: RunningHead,
    /// 꼬리말: a line of the writer's own at the bottom, beside the page number.
    #[serde(default)]
    pub footer: RunningFoot,
    /// Where the first-line indent is left out (indent.rs).
    #[serde(default)]
    pub indent_rules: IndentRules,
}

/// Paragraphs whose first line is not indented, though the format indents.
/// A paragraph's own first line (문단 모양) always wins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndentRules {
    /// The first paragraph of a chapter (common in English-language books).
    #[serde(default)]
    pub chapter_first: bool,
    /// The first paragraph after a scene break.
    #[serde(default)]
    pub after_scene: bool,
    /// Paragraphs set in with margins (letters, quotations).
    #[serde(default)]
    pub margined: bool,
    /// Dialogue: paragraphs opening with a quotation mark.
    #[serde(default)]
    pub dialogue: bool,
    /// Dialogue as on 원고지: every line set in, the first no further.
    #[serde(default)]
    pub dialogue_hang: bool,
}

/// What 머리말 shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HeadContent {
    #[default]
    None,
    /// The work's title.
    Title,
    /// The title of the chapter the page is in.
    Chapter,
    /// As in books: the work's title on even pages, the chapter's on odd ones.
    TitleChapter,
    /// The pen name.
    Author,
    /// Text the writer typed.
    Custom,
}

/// Where 머리말 sits. `Outside` puts it on the outer edge: left on even
/// pages, right on odd ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HeadAlign {
    Left,
    #[default]
    Center,
    Right,
    Outside,
}

impl HeadAlign {
    /// Where it lands on even and odd pages.
    pub fn sides(self) -> (HeadAlign, HeadAlign) {
        match self {
            HeadAlign::Outside => (HeadAlign::Left, HeadAlign::Right),
            a => (a, a),
        }
    }

    /// Lands in the same place as `other` on some page.
    pub fn meets(self, other: HeadAlign) -> bool {
        let (a, b) = (self.sides(), other.sides());
        a.0 == b.0 || a.1 == b.1
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RunningHead {
    pub content: HeadContent,
    /// For `Custom`.
    pub text: String,
    pub align: HeadAlign,
    /// Left off the first page of each chapter, as books do. Only when
    /// chapters start on a new page.
    pub skip_chapter_first: bool,
}

impl Default for RunningHead {
    fn default() -> Self {
        RunningHead {
            content: HeadContent::None,
            text: String::new(),
            align: HeadAlign::Center,
            skip_chapter_first: true,
        }
    }
}

impl RunningHead {
    pub fn is_on(&self) -> bool {
        self.content != HeadContent::None
    }

    /// Different text or place on even and odd pages.
    pub fn facing(&self) -> bool {
        self.is_on()
            && (self.align == HeadAlign::Outside || self.content == HeadContent::TitleChapter)
    }

    /// Changes with the chapter.
    pub fn follows_chapter(&self) -> bool {
        matches!(
            self.content,
            HeadContent::Chapter | HeadContent::TitleChapter
        )
    }
}

/// 꼬리말: text the writer typed, shown at the bottom of every page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RunningFoot {
    /// Empty when there is no 꼬리말.
    pub text: String,
    pub align: HeadAlign,
}

impl Default for RunningFoot {
    fn default() -> Self {
        RunningFoot {
            text: String::new(),
            align: HeadAlign::Left,
        }
    }
}

impl RunningFoot {
    pub fn is_on(&self) -> bool {
        !self.text.trim().is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Paper {
    /// Key into [`PAPERS`], `custom`, or `none` for continuous text (web).
    pub kind: String,
    pub width_mm: f64,
    pub height_mm: f64,
}

/// Page margins in mm, 한글 style: the header and footer areas sit between
/// the top/bottom margins and the text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Margins {
    pub top: f64,
    pub bottom: f64,
    /// Left margin, or the inner one when pages face each other.
    pub inside: f64,
    /// Right margin, or the outer one when pages face each other.
    pub outside: f64,
    pub header: f64,
    pub footer: f64,
}

/// Paper sizes: key, screen name, width and height in mm.
pub const PAPERS: [(&str, &str, f64, f64); 5] = [
    ("a4", "A4", 210.0, 297.0),
    ("b5", "B5", 182.0, 257.0),
    ("a5", "A5", 148.0, 210.0),
    ("shinguk", "신국판", 152.0, 225.0),
    ("46", "46판", 128.0, 188.0),
];

pub struct FontChoice {
    pub key: &'static str,
    pub label: &'static str,
    /// Family name written into HWPX files (한글).
    pub hwpx: &'static str,
    /// Family name written into docx files (Word).
    pub docx: &'static str,
}

pub const FONTS: [FontChoice; 5] = [
    FontChoice {
        key: "batang",
        label: "바탕 계열",
        hwpx: "함초롬바탕",
        docx: "바탕",
    },
    FontChoice {
        key: "dotum",
        label: "돋움 계열",
        hwpx: "함초롬돋움",
        docx: "맑은 고딕",
    },
    FontChoice {
        key: "nanum-myeongjo",
        label: "나눔명조",
        hwpx: "나눔명조",
        docx: "나눔명조",
    },
    FontChoice {
        key: "noto-serif",
        label: "본명조",
        hwpx: "Noto Serif KR",
        docx: "Noto Serif KR",
    },
    FontChoice {
        key: "gowun-batang",
        label: "고운바탕",
        hwpx: "고운바탕",
        docx: "Gowun Batang",
    },
];

pub fn font(key: &str) -> &'static FontChoice {
    FONTS.iter().find(|f| f.key == key).unwrap_or(&FONTS[0])
}

fn paper(kind: &str) -> Paper {
    let (_, _, w, h) = PAPERS
        .iter()
        .copied()
        .find(|(k, ..)| *k == kind)
        .unwrap_or(PAPERS[0]);
    Paper {
        kind: kind.into(),
        width_mm: w,
        height_mm: h,
    }
}

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

impl ManuscriptFormat {
    pub fn has_paper(&self) -> bool {
        self.paper.kind != "none"
    }

    /// The 꼬리말 and the page number would sit on top of each other.
    pub fn footer_clash(&self) -> bool {
        self.page_numbers && self.footer.is_on() && self.footer.align.meets(self.page_number_align)
    }

    /// Something at the bottom differs between even and odd pages.
    pub fn footer_facing(&self) -> bool {
        (self.page_numbers && self.page_number_align == HeadAlign::Outside)
            || (self.footer.is_on() && self.footer.align == HeadAlign::Outside)
    }

    /// Paper used when writing a file: continuous formats print on A4.
    pub fn page(&self) -> (f64, f64) {
        if self.has_paper() {
            (self.paper.width_mm, self.paper.height_mm)
        } else {
            (210.0, 297.0)
        }
    }

    /// Margins used when writing a file: continuous formats use 한글's A4 defaults.
    pub fn page_margins(&self) -> Margins {
        if self.has_paper() {
            self.margins.clone()
        } else {
            builtin("submission-a4").expect("built-in").margins
        }
    }

    /// Checks the values are in ranges 한글 and Word accept.
    pub fn validate(&self) -> Result<()> {
        let bad = |what: &str| Err(Error::Invalid(format!("원고 서식 · {what}")));
        let (w, h) = (self.paper.width_mm, self.paper.height_mm);
        if self.has_paper() && !((50.0..=600.0).contains(&w) && (50.0..=600.0).contains(&h)) {
            return bad("용지 크기는 50~600mm 사이로 적어 주세요");
        }
        let m = &self.margins;
        for v in [m.top, m.bottom, m.inside, m.outside, m.header, m.footer] {
            if !(0.0..=100.0).contains(&v) {
                return bad("여백은 0~100mm 사이로 적어 주세요");
            }
        }
        let (pw, ph) = self.page();
        let m = self.page_margins();
        if pw - m.inside - m.outside < 20.0 || ph - m.top - m.bottom - m.header - m.footer < 20.0 {
            return bad("여백이 너무 넓어 본문 자리가 없습니다");
        }
        if !(5.0..=40.0).contains(&self.size_pt) {
            return bad("글자 크기는 5~40pt 사이로 적어 주세요");
        }
        if !(50..=500).contains(&self.line_spacing) {
            return bad("줄 간격은 50~500% 사이로 적어 주세요");
        }
        if !(-50..=50).contains(&self.letter_spacing) {
            return bad("자간은 -50~50% 사이로 적어 주세요");
        }
        if !(0.0..=10.0).contains(&self.indent) {
            return bad("들여쓰기는 0~10자 사이로 적어 주세요");
        }
        if self.header.text.chars().count() > 100 {
            return bad("머리말은 100자까지 적을 수 있습니다");
        }
        if self.footer.text.chars().count() > 100 {
            return bad("꼬리말은 100자까지 적을 수 있습니다");
        }
        if self.footer_clash() {
            return bad("꼬리말과 쪽 번호가 같은 자리에 있습니다. 한쪽 자리를 바꿔 주세요");
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Catalog for the settings screen

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_presets_are_valid() {
        for (id, _, format) in builtin_presets() {
            format.validate().unwrap_or_else(|e| panic!("{id}: {e}"));
            assert_eq!(format.preset, id);
        }
        assert!(!default_for(ProjectKind::Webnovel).has_paper());
        assert_eq!(default_for(ProjectKind::Print).paper.kind, "a4");
    }

    #[test]
    fn validation_catches_bad_values() {
        let mut f = default_for(ProjectKind::Print);
        f.size_pt = 2.0;
        assert!(f.validate().is_err());
        let mut f = default_for(ProjectKind::Print);
        f.margins.inside = 100.0;
        f.margins.outside = 100.0;
        assert!(
            f.validate()
                .unwrap_err()
                .user_message()
                .contains("본문 자리")
        );
    }

    #[test]
    fn older_formats_read_without_a_header() {
        let mut value = serde_json::to_value(default_for(ProjectKind::Print)).unwrap();
        value.as_object_mut().unwrap().remove("header");
        let f: ManuscriptFormat = serde_json::from_value(value).unwrap();
        assert!(!f.header.is_on());
        assert!(f.header.skip_chapter_first);

        let head: RunningHead =
            serde_json::from_str(r#"{"content":"titleChapter","align":"outside"}"#).unwrap();
        assert!(head.facing() && head.follows_chapter());
        assert!(head.skip_chapter_first);
    }

    #[test]
    fn footer_and_page_number_keep_apart() {
        let mut value = serde_json::to_value(default_for(ProjectKind::Print)).unwrap();
        let obj = value.as_object_mut().unwrap();
        obj.remove("footer");
        obj.remove("pageNumberAlign");
        let mut f: ManuscriptFormat = serde_json::from_value(value).unwrap();
        assert_eq!(f.page_number_align, HeadAlign::Center);
        assert!(!f.footer.is_on() && !f.footer_facing());

        f.footer.text = "달빛 서점 · 투고 원고".into();
        f.footer.align = HeadAlign::Center;
        assert!(
            f.validate()
                .unwrap_err()
                .user_message()
                .contains("같은 자리")
        );
        f.footer.align = HeadAlign::Left;
        f.validate().unwrap();
        // Outside is left on even pages: it meets a page number on the left.
        f.page_number_align = HeadAlign::Outside;
        assert!(f.footer_clash() && f.footer_facing());
        f.page_number_align = HeadAlign::Right;
        f.footer.align = HeadAlign::Left;
        assert!(!f.footer_clash());
        f.page_numbers = false;
        f.footer.align = HeadAlign::Right;
        assert!(!f.footer_clash());
    }

    #[test]
    fn user_presets_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("presets.json");
        let mut f = default_for(ProjectKind::Print);
        f.size_pt = 11.0;
        save_user_preset(&file, "출판사 A 투고", &f).unwrap();
        save_user_preset(&file, "출판사 A 투고", &f).unwrap();
        let list = load_user_presets(&file);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].format.preset, "출판사 A 투고");
        assert!(save_user_preset(&file, "웹소설 플랫폼", &f).is_err());
        assert!(
            delete_user_preset(&file, "출판사 A 투고")
                .unwrap()
                .is_empty()
        );
    }
}
