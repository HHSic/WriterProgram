//! Reading docx files: paragraphs, headings, bold, italic, strikethrough,
//! underline and emphasis dots. Tables, pictures, shapes and footnotes are
//! counted and left out.

use std::collections::HashMap;
use std::io::{Cursor, Read};

use quick_xml::Reader;
use quick_xml::events::Event;
use zip::ZipArchive;

use super::text::is_scene;
use super::{Item, Raw, SkipKind};
use crate::markup::{Inline, Mark};

/// Largest XML part read, against files made to swell.
const PART_LIMIT: u64 = 256 * 1024 * 1024;

type Attrs = Vec<(String, String)>;

enum Tok {
    Start(String, Attrs),
    Empty(String, Attrs),
    End(String),
    Text(String),
    Ref(String),
}

fn local(name: &str) -> String {
    name.rsplit(':').next().unwrap_or_default().to_string()
}

fn attrs(e: &quick_xml::events::BytesStart<'_>) -> Attrs {
    e.attributes()
        .flatten()
        .map(|a| {
            (
                local(a.key.as_ref()),
                a.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                    .map(|v| v.into_owned())
                    .unwrap_or_default(),
            )
        })
        .collect()
}

fn val<'a>(attrs: &'a Attrs, key: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

/// Calls `f` for each token of an XML part until it returns false.
fn walk(xml: &str, mut f: impl FnMut(Tok)) -> Result<(), String> {
    let mut reader = Reader::from_str(xml);
    loop {
        let event = reader
            .read_event()
            .map_err(|_| "Word 문서의 내용이 올바르지 않음".to_string())?;
        match event {
            Event::Start(e) => f(Tok::Start(local(e.name().as_ref()), attrs(&e))),
            Event::Empty(e) => f(Tok::Empty(local(e.name().as_ref()), attrs(&e))),
            Event::End(e) => f(Tok::End(local(e.name().as_ref()))),
            Event::Text(t) => f(Tok::Text(t.to_string())),
            Event::CData(t) => f(Tok::Text(t.to_string())),
            Event::GeneralRef(r) => f(Tok::Ref(r.to_string())),
            Event::Eof => return Ok(()),
            _ => {}
        }
    }
}

fn read_part(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    name: &str,
) -> Option<Result<String, String>> {
    let file = archive.by_name(name).ok()?;
    let mut xml = String::new();
    let read = file.take(PART_LIMIT).read_to_string(&mut xml);
    Some(
        read.map(|_| xml)
            .map_err(|_| "Word 문서를 읽지 못함".to_string()),
    )
}

// ---------------------------------------------------------------------------
// Styles

#[derive(Default)]
struct Styles {
    /// styleId -> (name, basedOn)
    map: HashMap<String, (String, Option<String>)>,
}

impl Styles {
    fn read(xml: &str) -> Styles {
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
    fn heading(&self, id: &str) -> Option<u8> {
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

    fn is_toc(&self, id: &str) -> bool {
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

#[derive(Default, Clone, Copy)]
struct Run {
    bold: bool,
    italic: bool,
    strike: bool,
    underline: bool,
    dot: bool,
}

impl Run {
    fn marks(self) -> Vec<Mark> {
        let mut marks = Vec::new();
        if self.underline {
            marks.push(Mark::Underline {});
        }
        if self.dot {
            marks.push(Mark::Dot {});
        }
        if self.strike {
            marks.push(Mark::Strike {});
        }
        if self.bold {
            marks.push(Mark::Bold {});
        }
        if self.italic {
            marks.push(Mark::Italic {});
        }
        marks
    }
}

#[derive(Default)]
struct Para {
    style: Option<String>,
    outline: Option<u8>,
    inlines: Vec<Inline>,
    skips: Vec<(SkipKind, u32)>,
}

impl Para {
    fn add_text(&mut self, text: &str, run: Run) {
        if text.is_empty() {
            return;
        }
        let marks = run.marks();
        if let Some(Inline::Text {
            text: last,
            marks: m,
        }) = self.inlines.last_mut()
            && *m == marks
        {
            last.push_str(text);
            return;
        }
        self.inlines.push(Inline::Text {
            text: text.into(),
            marks,
        });
    }

    fn skip(&mut self, kind: SkipKind) {
        match self.skips.iter_mut().find(|(k, _)| *k == kind) {
            Some((_, n)) => *n += 1,
            None => self.skips.push((kind, 1)),
        }
    }
}

/// Takes spaces (and the full-width space some writers indent with) off both
/// ends of a paragraph, and line breaks with them.
fn trim_ends(inlines: &mut Vec<Inline>) {
    loop {
        match inlines.first_mut() {
            Some(Inline::HardBreak {}) => {
                inlines.remove(0);
            }
            Some(Inline::Text { text, .. }) => {
                let trimmed = text.trim_start();
                if trimmed.is_empty() {
                    inlines.remove(0);
                } else {
                    *text = trimmed.to_string();
                    break;
                }
            }
            None => return,
        }
    }
    loop {
        match inlines.last_mut() {
            Some(Inline::HardBreak {}) => {
                inlines.pop();
            }
            Some(Inline::Text { text, .. }) => {
                let trimmed = text.trim_end();
                if trimmed.is_empty() {
                    inlines.pop();
                } else {
                    *text = trimmed.to_string();
                    break;
                }
            }
            None => return,
        }
    }
}

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
        let Some(mut p) = para else { return };
        trim_ends(&mut p.inlines);
        let text = super::plain(&p.inlines);
        let toc = p.style.as_deref().is_some_and(|s| styles.is_toc(s));
        if !text.trim().is_empty() && !toc {
            let level = p
                .style
                .as_deref()
                .and_then(|s| styles.heading(s))
                .or_else(|| p.outline.filter(|l| *l < 9).map(|l| l + 1));
            let single = !text.contains('\n');
            if is_scene(&text) && single && level.is_none() {
                items.push(Item::Scene);
            } else if let (Some(level), true) = (level, single) {
                items.push(Item::Heading {
                    level,
                    text: text.trim().to_string(),
                });
            } else {
                items.push(Item::Para(p.inlines));
            }
        }
        for (kind, n) in p.skips {
            items.push(Item::Skip(kind, n));
        }
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
            Tok::Ref(name) => {
                if in_t {
                    let c = match name.as_str() {
                        "amp" => Some('&'),
                        "lt" => Some('<'),
                        "gt" => Some('>'),
                        "quot" => Some('"'),
                        "apos" => Some('\''),
                        n => n.strip_prefix('#').and_then(|n| {
                            match n.strip_prefix(['x', 'X']) {
                                Some(hex) => u32::from_str_radix(hex, 16).ok(),
                                None => n.parse().ok(),
                            }
                            .and_then(char::from_u32)
                        }),
                    };
                    if let (Some(c), Some(p)) = (c, para.as_mut()) {
                        p.add_text(&c.to_string(), run);
                    }
                }
            }
        }
    })?;
    close(para.take(), &mut items);
    Ok(items)
}

/// Paragraph and run properties.
fn property(
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
    })
}
