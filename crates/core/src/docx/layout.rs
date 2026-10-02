//! Which header and footer parts a document needs, and what they hold.

use super::body::text_run;
use super::{CHAPTER_STYLE_NAME, R, W, twips_mm};
use crate::export::{DocInfo, DocOptions, ExportDoc, RunStyle};
use crate::format::{HeadAlign, HeadContent, ManuscriptFormat};

/// Header and footer parts: file name, relationship id, and for which pages.
#[derive(Clone, Copy, PartialEq)]
enum PageType {
    Default,
    Even,
    First,
}

impl PageType {
    fn attr(self) -> &'static str {
        match self {
            PageType::Default => "default",
            PageType::Even => "even",
            PageType::First => "first",
        }
    }
}

pub(super) struct Part {
    pub header: bool,
    page: PageType,
    pub file: String,
    pub rel: String,
    pub xml: String,
}

/// What the package holds besides the body.
pub(super) struct Layout {
    pub parts: Vec<Part>,
    pub even_odd: bool,
    /// Each chapter is a section whose first page has no 머리말.
    pub sections: bool,
}

impl Layout {
    pub(super) fn new(
        docs: &[ExportDoc],
        format: &ManuscriptFormat,
        opts: &DocOptions,
        info: &DocInfo,
    ) -> Self {
        let head = &format.header;
        let even_odd = head.facing() || format.footer_facing();
        let sections = format.head_skips_chapter_first();
        let mut parts = Vec::new();
        let mut add = |header: bool, page: PageType, xml: String| {
            let n = parts.len() + 3;
            let kind = if header { "header" } else { "footer" };
            parts.push(Part {
                header,
                page,
                file: format!("{kind}{n}.xml"),
                rel: format!("rId{n}"),
                xml,
            });
        };
        if format.page_numbers || format.footer.is_on() {
            add(false, PageType::Default, footer(format, false));
            if even_odd {
                add(false, PageType::Even, footer(format, true));
            }
            if sections {
                add(false, PageType::First, footer(format, false));
            }
        }
        if head.is_on() {
            // The chapter's title comes from its title paragraphs; without
            // them the work's title stands in.
            let first_heading = docs
                .iter()
                .map(|d| d.heading.trim())
                .find(|h| !h.is_empty())
                .unwrap_or_default();
            let chapter = if opts.include_titles && !first_heading.is_empty() {
                HeadText::Chapter(first_heading.to_string())
            } else {
                HeadText::Plain(info.title.clone())
            };
            let fixed = head.fixed_text(&info.title, &info.author);
            let (odd_text, even_text) = match head.content {
                HeadContent::Chapter => (chapter.clone(), chapter),
                HeadContent::TitleChapter => (chapter, HeadText::Plain(info.title.clone())),
                _ => (HeadText::Plain(fixed.clone()), HeadText::Plain(fixed)),
            };
            let (odd_jc, even_jc) = match head.align {
                HeadAlign::Left => ("left", "left"),
                HeadAlign::Center => ("center", "center"),
                HeadAlign::Right => ("right", "right"),
                HeadAlign::Outside => ("right", "left"),
            };
            add(true, PageType::Default, header_part(&odd_text, odd_jc));
            if even_odd {
                add(true, PageType::Even, header_part(&even_text, even_jc));
            }
            if sections {
                add(
                    true,
                    PageType::First,
                    header_part(&HeadText::Plain(String::new()), "left"),
                );
            }
        }
        Layout {
            parts,
            even_odd,
            sections,
        }
    }

    /// References for a section's properties.
    pub(super) fn references(&self) -> String {
        self.parts
            .iter()
            .map(|p| {
                format!(
                    r#"<w:{kind}Reference w:type="{page}" r:id="{rel}"/>"#,
                    kind = if p.header { "header" } else { "footer" },
                    page = p.page.attr(),
                    rel = p.rel,
                )
            })
            .collect()
    }
}

#[derive(Clone)]
enum HeadText {
    Plain(String),
    /// The chapter title, via STYLEREF; the text is what shows before Word updates it.
    Chapter(String),
}

fn header_part(text: &HeadText, jc: &str) -> String {
    let runs = match text {
        HeadText::Plain(t) if t.is_empty() => String::new(),
        HeadText::Plain(t) => text_run(t, RunStyle::default()),
        HeadText::Chapter(shown) => format!(
            r#"<w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText xml:space="preserve"> STYLEREF "{CHAPTER_STYLE_NAME}" \* MERGEFORMAT </w:instrText></w:r><w:r><w:fldChar w:fldCharType="separate"/></w:r>{}<w:r><w:fldChar w:fldCharType="end"/></w:r>"#,
            text_run(shown, RunStyle::default())
        ),
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:hdr xmlns:w="{W}" xmlns:r="{R}"><w:p><w:pPr><w:pStyle w:val="Header"/><w:jc w:val="{jc}"/></w:pPr>{runs}</w:p></w:hdr>"#
    )
}

const PAGE_NUMBER: &str = r#"<w:r><w:t xml:space="preserve">- </w:t></w:r><w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText xml:space="preserve"> PAGE </w:instrText></w:r><w:r><w:fldChar w:fldCharType="separate"/></w:r><w:r><w:t>1</w:t></w:r><w:r><w:fldChar w:fldCharType="end"/></w:r><w:r><w:t xml:space="preserve"> -</w:t></w:r>"#;

/// The footer of even or odd pages: the page number and the 꼬리말, each in
/// its place. Two things on one line sit at tab stops (centre and right edge
/// of the text area); one alone is simply aligned.
fn footer(format: &ManuscriptFormat, even: bool) -> String {
    let side = |align: HeadAlign| {
        let (e, o) = align.sides();
        if even { e } else { o }
    };
    // Left, centre, right.
    let mut slots = [String::new(), String::new(), String::new()];
    let slot = |align: HeadAlign| match align {
        HeadAlign::Left => 0,
        HeadAlign::Center => 1,
        _ => 2,
    };
    if format.page_numbers {
        slots[slot(side(format.page_number_align))].push_str(PAGE_NUMBER);
    }
    if format.footer.is_on() {
        slots[slot(side(format.footer.align))]
            .push_str(&text_run(format.footer.text.trim(), RunStyle::default()));
    }
    let filled: Vec<usize> = (0..3).filter(|i| !slots[*i].is_empty()).collect();
    let (ppr, runs) = if filled.len() == 1 {
        let jc = ["left", "center", "right"][filled[0]];
        (
            format!(r#"<w:pStyle w:val="Footer"/><w:jc w:val="{jc}"/>"#),
            slots[filled[0]].clone(),
        )
    } else {
        let (w, _) = format.page();
        let m = format.page_margins();
        let width = twips_mm(w - m.inside - m.outside);
        (
            format!(
                r#"<w:pStyle w:val="Footer"/><w:tabs><w:tab w:val="center" w:pos="{}"/><w:tab w:val="right" w:pos="{width}"/></w:tabs><w:jc w:val="left"/>"#,
                width / 2
            ),
            slots.join("<w:r><w:tab/></w:r>"),
        )
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:ftr xmlns:w="{W}" xmlns:r="{R}"><w:p><w:pPr>{ppr}</w:pPr>{runs}</w:p></w:ftr>"#
    )
}
