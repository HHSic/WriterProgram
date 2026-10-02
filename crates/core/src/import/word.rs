//! Reading docx files: paragraphs, headings, bold, italic, strikethrough,
//! underline and emphasis dots. Tables, pictures, shapes and footnotes are
//! counted and left out.

use std::collections::HashMap;
use std::io::Cursor;

use zip::ZipArchive;

use super::para::{Para, Run};
use super::xml::{Attrs, Tok, read_part, val, walk};
use super::{Item, Raw, SkipKind};
use crate::markup::Inline;

// ---------------------------------------------------------------------------
// Styles

#[derive(Default)]
pub(super) struct Styles {
    /// styleId -> (name, basedOn)
    map: HashMap<String, (String, Option<String>)>,
}

impl Styles {
    pub(super) fn read(xml: &str) -> Styles {
        let mut styles = Styles::default();
        let mut id = String::new();
        let mut name = String::new();
        let mut based = None;
        let _ = walk(xml, |tok| match tok {
            Tok::Start(n, a) if n == "style" => {
                id = val(&a, "styleId").unwrap_or_default().to_string();
                name.clear();
                based = None;
            }
            Tok::Start(n, a) | Tok::Empty(n, a) if n == "name" => {
                name = val(&a, "val").unwrap_or_default().to_string();
            }
            Tok::Start(n, a) | Tok::Empty(n, a) if n == "basedOn" => {
                based = val(&a, "val").map(str::to_string);
            }
            Tok::End(n) if n == "style" && !id.is_empty() => {
                styles.map.insert(
                    std::mem::take(&mut id),
                    (std::mem::take(&mut name), based.take()),
                );
            }
            _ => {}
        });
        styles
    }

    /// Heading level of a paragraph style (1 to 9), following `basedOn`.
    pub(super) fn heading(&self, id: &str) -> Option<u8> {
        let mut id = id;
        for _ in 0..6 {
            let (name, based) = match self.map.get(id) {
                Some((name, based)) => (name.to_lowercase(), based.as_deref()),
                None => (String::new(), None),
            };
            let level = |rest: &str| {
                rest.trim()
                    .parse::<u8>()
                    .ok()
                    .filter(|n| (1..=9).contains(n))
            };
            if let Some(n) = name
                .strip_prefix("heading ")
                .or_else(|| name.strip_prefix("제목 "))
                .and_then(level)
                .or_else(|| id.strip_prefix("Heading").and_then(level))
            {
                return Some(n);
            }
            // The style this app's own docx export gives chapter titles.
            if name == "장 제목" {
                return Some(1);
            }
            id = based?;
        }
        None
    }

    pub(super) fn is_toc(&self, id: &str) -> bool {
        self.map.get(id).is_some_and(|(name, _)| {
            let name = name.to_lowercase();
            name.starts_with("toc")
                || name.starts_with("목차")
                || name.starts_with("table of contents")
        })
    }
}

// ---------------------------------------------------------------------------
// Body

fn flag(attrs: &Attrs) -> bool {
    !matches!(val(attrs, "val"), Some("0" | "false" | "off" | "none"))
}

fn body(xml: &str, styles: &Styles) -> Result<Vec<Item>, String> {
    let mut items: Vec<Item> = Vec::new();
    let mut para: Option<Para> = None;
    let mut run = Run::default();
    let (mut in_ppr, mut in_rpr, mut in_t) = (false, false, false);
    // Depth inside a subtree being left out.
    let mut ignoring = 0usize;

    let close = |para: Option<Para>, items: &mut Vec<Item>| {
        let Some(p) = para else { return };
        let toc = p.style.as_deref().is_some_and(|s| styles.is_toc(s));
        let level = p
            .style
            .as_deref()
            .and_then(|s| styles.heading(s))
            .or_else(|| p.outline.filter(|l| *l < 9).map(|l| l + 1));
        p.close(level, toc, items);
    };

    walk(xml, |tok| {
        if ignoring > 0 {
            match tok {
                Tok::Start(..) => ignoring += 1,
                Tok::End(_) => ignoring -= 1,
                _ => {}
            }
            return;
        }
        match tok {
            Tok::Start(n, a) => {
                match n.as_str() {
                    "tbl" => {
                        items.push(Item::Skip(SkipKind::Table, 1));
                        ignoring = 1;
                    }
                    "drawing" | "pict" | "object" => {
                        if let Some(p) = para.as_mut() {
                            p.skip(SkipKind::Image);
                        }
                        ignoring = 1;
                    }
                    "Fallback" | "rt" => ignoring = 1,
                    "p" => {
                        close(para.take(), &mut items);
                        para = Some(Para::default());
                    }
                    "pPr" => in_ppr = true,
                    "rPr" => in_rpr = !in_ppr,
                    "r" => run = Run::default(),
                    "t" => in_t = true,
                    "footnoteReference" | "endnoteReference" => {
                        if let Some(p) = para.as_mut() {
                            p.skip(SkipKind::Footnote);
                        }
                    }
                    _ => property(&n, &a, in_ppr, in_rpr, &mut run, &mut para),
                };
            }
            Tok::Empty(n, a) => match n.as_str() {
                "tbl" => items.push(Item::Skip(SkipKind::Table, 1)),
                "drawing" | "pict" | "object" => {
                    if let Some(p) = para.as_mut() {
                        p.skip(SkipKind::Image);
                    }
                }
                "footnoteReference" | "endnoteReference" => {
                    if let Some(p) = para.as_mut() {
                        p.skip(SkipKind::Footnote);
                    }
                }
                "tab" => {
                    if let Some(p) = para.as_mut() {
                        p.add_text(" ", run);
                    }
                }
                "noBreakHyphen" => {
                    if let Some(p) = para.as_mut() {
                        p.add_text("-", run);
                    }
                }
                "br" => {
                    if !matches!(val(&a, "type"), Some("page" | "column"))
                        && let Some(p) = para.as_mut()
                    {
                        p.inlines.push(Inline::HardBreak {});
                    }
                }
                "cr" => {
                    if let Some(p) = para.as_mut() {
                        p.inlines.push(Inline::HardBreak {});
                    }
                }
                _ => property(&n, &a, in_ppr, in_rpr, &mut run, &mut para),
            },
            Tok::End(n) => match n.as_str() {
                "p" => close(para.take(), &mut items),
                "pPr" => in_ppr = false,
                "rPr" => in_rpr = false,
                "t" => in_t = false,
                _ => {}
            },
            Tok::Text(s) => {
                if in_t && let Some(p) = para.as_mut() {
                    p.add_text(&s, run);
                }
            }
        }
    })?;
    close(para.take(), &mut items);
    Ok(items)
}

/// Paragraph and run properties.
pub(super) fn property(
    name: &str,
    a: &Attrs,
    in_ppr: bool,
    in_rpr: bool,
    run: &mut Run,
    para: &mut Option<Para>,
) {
    if in_ppr {
        if let Some(p) = para.as_mut() {
            match name {
                "pStyle" => p.style = val(a, "val").map(str::to_string),
                "outlineLvl" => p.outline = val(a, "val").and_then(|v| v.parse().ok()),
                _ => {}
            }
        }
    } else if in_rpr {
        match name {
            "b" => run.bold = flag(a),
            "i" => run.italic = flag(a),
            "strike" | "dstrike" => run.strike = flag(a),
            "u" => run.underline = flag(a),
            "em" => run.dot = flag(a),
            "color" => {
                run.color = val(a, "val")
                    .is_some_and(|c| !matches!(c.to_ascii_lowercase().as_str(), "auto" | "000000"))
            }
            "highlight" => run.shade = val(a, "val").is_some_and(|c| c != "none"),
            "shd" => {
                run.shade = val(a, "fill").is_some_and(|c| {
                    !matches!(c.to_ascii_lowercase().as_str(), "" | "auto" | "ffffff")
                })
            }
            _ => {}
        }
    }
}

pub(super) fn read(bytes: &[u8]) -> Result<Raw, String> {
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|_| "Word 문서(docx)가 아니거나 손상됨".to_string())?;
    let document = read_part(&mut archive, "word/document.xml")
        .ok_or_else(|| "Word 문서(docx)가 아니거나 손상됨".to_string())??;
    let styles = read_part(&mut archive, "word/styles.xml")
        .and_then(Result::ok)
        .map(|xml| Styles::read(&xml))
        .unwrap_or_default();
    Ok(Raw {
        items: body(&document, &styles)?,
        encoding: None,
        page: None,
    })
}
