//! document.xml: chapters, paragraphs and runs, and the section breaks
//! between chapters.

use super::layout::Layout;
use super::{R, W, twips_mm, twips_pt};
use crate::export::{DocOptions, ExportDoc, Piece, RunStyle, pieces};
use crate::format::ManuscriptFormat;
use crate::markup::{Block, ParaAttrs};
use crate::xml;

fn run_props(style: RunStyle) -> String {
    let mut props = String::new();
    if style.bold {
        props.push_str("<w:b/><w:bCs/>");
    }
    if style.italic {
        props.push_str("<w:i/><w:iCs/>");
    }
    if style.strike {
        props.push_str("<w:strike/>");
    }
    if style.underline {
        props.push_str(r#"<w:u w:val="single"/>"#);
    }
    if style.dot {
        props.push_str(r#"<w:em w:val="dot"/>"#);
    }
    if props.is_empty() {
        props
    } else {
        format!("<w:rPr>{props}</w:rPr>")
    }
}

pub(super) fn text_run(text: &str, style: RunStyle) -> String {
    format!(
        r#"<w:r>{}<w:t xml:space="preserve">{}</w:t></w:r>"#,
        run_props(style),
        xml::text(text)
    )
}

/// A paragraph before it is written: its style, the rest of its properties
/// (in schema order; a section break goes last) and its runs.
#[derive(Default)]
struct Para {
    style: Option<&'static str>,
    ppr: String,
    body: String,
}

impl Para {
    fn xml(&self) -> String {
        let mut ppr = String::new();
        if let Some(style) = self.style {
            ppr.push_str(&format!(r#"<w:pStyle w:val="{style}"/>"#));
        }
        ppr.push_str(&self.ppr);
        match (ppr.is_empty(), self.body.is_empty()) {
            (true, true) => "<w:p/>".into(),
            (true, false) => format!("<w:p>{}</w:p>", self.body),
            _ => format!("<w:p><w:pPr>{ppr}</w:pPr>{}</w:p>", self.body),
        }
    }
}

/// 문단 모양 as Word indents, in characters: margins, and the first line
/// (the paragraph's own, or the format's).
fn margin_ind(format: &ManuscriptFormat, attrs: ParaAttrs) -> String {
    let char_pt = format.size_pt * (1.0 + f64::from(format.letter_spacing) / 100.0);
    let side = |n: u8| (u32::from(n) * 100, twips_pt(f64::from(n) * char_pt));
    let (left_chars, left) = side(attrs.left);
    let (right_chars, right) = side(attrs.right);
    let first = attrs.indent.map_or(format.indent, f64::from);
    let first_line = if first < 0.0 {
        format!(
            r#"w:hangingChars="{}" w:hanging="{}""#,
            (-first * 100.0).round() as i64,
            twips_pt(-first * format.size_pt)
        )
    } else {
        format!(
            r#"w:firstLineChars="{}" w:firstLine="{}""#,
            (first * 100.0).round() as i64,
            twips_pt(first * format.size_pt)
        )
    };
    format!(
        r#"<w:ind w:leftChars="{left_chars}" w:left="{left}" w:rightChars="{right_chars}" w:right="{right}" {first_line}/>"#
    )
}

/// Section properties. Only the first section names its 머리말 and 꼬리말;
/// later ones carry them over.
fn sect_pr(format: &ManuscriptFormat, layout: &Layout, first: bool, new_page: bool) -> String {
    let (width, height) = format.page();
    let m = format.page_margins();
    let refs = if first {
        layout.references()
    } else {
        String::new()
    };
    let kind = if new_page {
        r#"<w:type w:val="nextPage"/>"#
    } else {
        ""
    };
    let title_pg = if layout.sections { "<w:titlePg/>" } else { "" };
    format!(
        r#"<w:sectPr>{refs}{kind}<w:pgSz w:w="{}" w:h="{}"/><w:pgMar w:top="{}" w:right="{}" w:bottom="{}" w:left="{}" w:header="{}" w:footer="{}" w:gutter="0"/><w:cols w:space="425"/>{title_pg}</w:sectPr>"#,
        twips_mm(width),
        twips_mm(height),
        twips_mm(m.top + m.header),
        twips_mm(m.outside),
        twips_mm(m.bottom + m.footer),
        twips_mm(m.inside),
        twips_mm(m.top),
        twips_mm(m.bottom),
    )
}

pub(super) fn document(
    docs: &[ExportDoc],
    format: &ManuscriptFormat,
    opts: &DocOptions,
    layout: &Layout,
) -> String {
    let sections = layout.sections;
    let mut chapters: Vec<Vec<Para>> = Vec::new();
    for (i, doc) in docs.iter().enumerate() {
        let first = i == 0;
        let mut paras: Vec<Para> = Vec::new();
        // With a section per chapter the section starts the new page.
        let mut break_pending = format.chapter_new_page && !first && !sections;
        if opts.include_titles && !doc.heading.trim().is_empty() {
            // The title style breaks the page itself; the first title must not.
            let ppr = if (first || sections) && format.chapter_new_page {
                r#"<w:pageBreakBefore w:val="0"/>"#
            } else {
                ""
            };
            paras.push(Para {
                style: Some("WPChapter"),
                ppr: ppr.into(),
                body: text_run(doc.heading.trim(), RunStyle::default()),
            });
            break_pending = false;
        } else if !first && !format.chapter_new_page {
            paras.push(Para::default());
        }
        for block in &doc.blocks {
            let mut ppr = String::new();
            if std::mem::take(&mut break_pending) {
                ppr.push_str("<w:pageBreakBefore/>");
            }
            match block {
                Block::SceneBreak {} => paras.push(Para {
                    style: Some("WPSceneBreak"),
                    ppr,
                    body: text_run(&opts.scene_break, RunStyle::default()),
                }),
                Block::Paragraph { attrs, content } => {
                    if !attrs.is_plain() {
                        ppr.push_str(&margin_ind(format, *attrs));
                    }
                    let mut runs = String::new();
                    for piece in pieces(content) {
                        match piece {
                            Piece::Text(text, style) => runs.push_str(&text_run(&text, style)),
                            Piece::Break => runs.push_str("<w:r><w:br/></w:r>"),
                        }
                    }
                    paras.push(Para {
                        style: None,
                        ppr,
                        body: runs,
                    });
                }
            }
        }
        chapters.push(paras);
    }

    // A section break ends every chapter but the last; the last section's
    // properties close the body.
    let count = chapters.len();
    if sections {
        for (i, paras) in chapters
            .iter_mut()
            .enumerate()
            .take(count.saturating_sub(1))
        {
            if paras.is_empty() {
                paras.push(Para::default());
            }
            let last = paras.last_mut().expect("not empty");
            last.ppr.push_str(&sect_pr(format, layout, i == 0, i > 0));
        }
    }
    let body: String = chapters.iter().flatten().map(Para::xml).collect();
    let last_sect = if sections {
        sect_pr(format, layout, count <= 1, count > 1)
    } else {
        sect_pr(format, layout, true, false)
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="{W}" xmlns:r="{R}"><w:body>{body}{last_sect}</w:body></w:document>"#
    )
}
