//! Paper sizes, margins and the fonts on offer.

use serde::{Deserialize, Serialize};

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

/// Paper of a size in [`PAPERS`]; A4 for an unknown key.
pub(super) fn paper(kind: &str) -> Paper {
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
