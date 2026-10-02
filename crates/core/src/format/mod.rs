//! Manuscript format (원고 서식): how the manuscript looks on paper.
//!
//! It is a project setting used by docx/HWPX export and by the page estimate.
//! The writing screen has its own settings in the app, which never touch the
//! files (docs/mvp-scope.md "서식 모델"). Units follow 한글: mm for paper and
//! margins, pt for type size, % for line and letter spacing.

mod heads;
mod paper;
mod presets;

pub use heads::{HeadAlign, HeadContent, RunningFoot, RunningHead};
pub use paper::{FONTS, FontChoice, Margins, PAPERS, Paper, font};
pub use presets::{
    Catalog, FontEntry, PaperEntry, PresetEntry, UserPreset, builtin, builtin_presets, catalog,
    default_for, delete_user_preset, load_user_presets, save_user_preset,
};

use serde::{Deserialize, Serialize};

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

impl ManuscriptFormat {
    pub fn has_paper(&self) -> bool {
        self.paper.kind != "none"
    }

    /// 머리말 is left off the first page of each chapter. That needs chapters
    /// to start on a new page.
    pub fn head_skips_chapter_first(&self) -> bool {
        self.header.is_on() && self.header.skip_chapter_first && self.chapter_new_page
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::ProjectKind;

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
        assert!(head.facing());
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
