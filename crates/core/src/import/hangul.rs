//! Reading 한글 files (HWPX, the zip-and-XML format 한글 has saved since 2014
//! and opens by default since 2022): paragraphs, headings (개요 styles, the
//! outline level of a paragraph shape, and this app's own 장 제목), bold,
//! italic, underline, strikethrough and 방점. Line breaks and 묶음·고정폭
//! 빈칸 stay what they are. Tables, pictures, shapes, equations and
//! footnotes are counted and left out; 머리말·꼬리말 and other controls are
//! skipped. The old binary format (.hwp) is not read.

use std::collections::HashMap;
use std::io::Cursor;

use zip::ZipArchive;

use super::page::{Found, Line};
use super::text::is_scene;
use super::word::{Para, Run, Tok, read_part, trim_ends, val, walk};
use super::{Item, Raw, SkipKind};
use crate::format::{HeadAlign, Margins};
use crate::markup::Inline;

const BROKEN: &str = "한글 문서(HWPX)가 아니거나 손상됨";

/// Character shapes: `charPr` id to what the text looks like.
fn char_shapes(header: &str) -> HashMap<String, Run> {
    let mut out = HashMap::new();
    let mut current: Option<(String, Run)> = None;
    let _ = walk(header, |tok| match tok {
        Tok::Start(n, a) if n == "charPr" => {
            let run = Run {
                dot: val(&a, "symMark").is_some_and(|m| m != "NONE"),
                ..Run::default()
            };
            current = Some((val(&a, "id").unwrap_or_default().to_string(), run));
        }
        Tok::Start(n, a) | Tok::Empty(n, a) => {
            if let Some((_, run)) = current.as_mut() {
                match n.as_str() {
                    "bold" => run.bold = true,
                    "italic" => run.italic = true,
                    "underline" => run.underline = val(&a, "type").is_some_and(|t| t != "NONE"),
                    "strikeout" => run.strike = val(&a, "shape").is_some_and(|s| s != "NONE"),
                    _ => {}
                }
            }
        }
        Tok::End(n) if n == "charPr" => {
            if let Some((id, run)) = current.take() {
                out.insert(id, run);
            }
        }
        _ => {}
    });
    out
}

/// Heading levels (1–9) of paragraph shapes with an outline level.
fn outline_shapes(header: &str) -> HashMap<String, u8> {
    let mut out = HashMap::new();
    let mut current: Option<String> = None;
    let _ = walk(header, |tok| match tok {
        Tok::Start(n, a) if n == "paraPr" => current = val(&a, "id").map(str::to_string),
        Tok::Start(n, a) | Tok::Empty(n, a) if n == "heading" => {
            if let (Some(id), Some("OUTLINE")) = (current.as_ref(), val(&a, "type"))
                && let Some(level) = val(&a, "level").and_then(|l| l.parse::<u8>().ok())
                && level < 9
            {
                out.insert(id.clone(), level + 1);
            }
        }
        Tok::End(n) if n == "paraPr" => current = None,
        _ => {}
    });
    out
}

/// What each paragraph style means here: a heading level, or a 차례 line.
#[derive(Clone, Copy, PartialEq, Eq)]
enum StyleKind {
    Heading(u8),
    Toc,
}

fn styles(header: &str) -> HashMap<String, StyleKind> {
    let mut out = HashMap::new();
    let _ = walk(header, |tok| {
        if let Tok::Start(n, a) | Tok::Empty(n, a) = tok
            && n == "style"
        {
            let id = val(&a, "id").unwrap_or_default().to_string();
            let names = [
                val(&a, "name").unwrap_or_default(),
                val(&a, "engName").unwrap_or_default(),
            ];
            let level = |rest: &str| {
                rest.trim()
                    .parse::<u8>()
                    .ok()
                    .filter(|n| (1..=9).contains(n))
            };
            let kind = names.iter().find_map(|name| {
                let lower = name.to_lowercase();
                if *name == "장 제목" || lower == "chapter title" {
                    return Some(StyleKind::Heading(1));
                }
                if name.starts_with("차례") || lower.starts_with("toc") {
                    return Some(StyleKind::Toc);
                }
                name.strip_prefix("개요 ")
                    .or_else(|| lower.strip_prefix("outline "))
                    .or_else(|| name.strip_prefix("제목 "))
                    .and_then(level)
                    .map(StyleKind::Heading)
            });
            if let Some(kind) = kind {
                out.insert(id, kind);
            }
        }
    });
    out
}

/// Paragraph shapes' horizontal alignment, for 머리말 and 꼬리말.
fn para_aligns(header: &str) -> HashMap<String, HeadAlign> {
    let mut out = HashMap::new();
    let mut current: Option<String> = None;
    let _ = walk(header, |tok| match tok {
        Tok::Start(n, a) if n == "paraPr" => current = val(&a, "id").map(str::to_string),
        Tok::Start(n, a) | Tok::Empty(n, a) if n == "align" => {
            if let Some(id) = current.as_ref()
                && !out.contains_key(id)
            {
                let align = match val(&a, "horizontal") {
                    Some("CENTER") => HeadAlign::Center,
                    Some("RIGHT") => HeadAlign::Right,
                    _ => HeadAlign::Left,
                };
                out.insert(id.clone(), align);
            }
        }
        Tok::End(n) if n == "paraPr" => current = None,
        _ => {}
    });
    out
}

/// HWPUNIT (1/7200 inch) in mm.
fn mm(hu: Option<&str>) -> f64 {
    hu.and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0) * 25.4 / 7200.0
}

/// Paper, margins, page number, 머리말 and 꼬리말 of a section. Paper comes
/// from the first section only; the lines from every section.
fn pages(xml: &str, aligns: &HashMap<String, HeadAlign>, first: bool, found: &mut Found) {
    let mut in_page = false;
    let mut gutter_both = false;
    // The 머리말 or 꼬리말 being read, and whether it is a 꼬리말.
    let mut line: Option<(Line, bool)> = None;
    let mut in_t = false;
    let _ = walk(xml, |tok| match tok {
        Tok::Start(n, a) | Tok::Empty(n, a) if n == "pagePr" && first && found.width == 0.0 => {
            let (mut w, mut h) = (mm(val(&a, "width")), mm(val(&a, "height")));
            // Lying paper keeps its upright size in the file.
            if val(&a, "landscape") == Some("NARROWLY") && w < h {
                std::mem::swap(&mut w, &mut h);
            }
            (found.width, found.height) = (w, h);
            gutter_both = val(&a, "gutterType") == Some("LEFT_RIGHT");
            in_page = true;
        }
        Tok::End(n) if n == "pagePr" => in_page = false,
        Tok::Start(n, a) | Tok::Empty(n, a) if n == "margin" && in_page => {
            let get = |k: &str| mm(val(&a, k));
            let gutter = get("gutter");
            found.margins = Some(Margins {
                top: get("top"),
                bottom: get("bottom"),
                inside: get("left") + gutter,
                outside: get("right") + if gutter_both { gutter } else { 0.0 },
                header: get("header"),
                footer: get("footer"),
            });
        }
        Tok::Start(n, a) | Tok::Empty(n, a) if n == "pageNum" && found.page_number.is_none() => {
            found.page_number = match val(&a, "pos").unwrap_or("NONE") {
                "NONE" => None,
                p if p.ends_with("LEFT") => Some(HeadAlign::Left),
                p if p.ends_with("RIGHT") => Some(HeadAlign::Right),
                p if p.starts_with("OUTSIDE") => Some(HeadAlign::Outside),
                _ => Some(HeadAlign::Center),
            };
        }
        Tok::Start(n, a) | Tok::Empty(n, a) if n == "pageHiding" => {
            if val(&a, "hideHeader") == Some("1") {
                found.hides_head = true;
            }
        }
        Tok::Start(n, a) if (n == "header" || n == "footer") && line.is_none() => {
            let pages = val(&a, "applyPageType").unwrap_or("BOTH").to_string();
            line = Some((
                Line {
                    pages,
                    ..Line::default()
                },
                n == "footer",
            ));
        }
        Tok::End(n) if n == "header" || n == "footer" => {
            if let Some((l, foot)) = line.take() {
                if foot {
                    found.foots.push(l);
                } else {
                    found.heads.push(l);
                }
            }
        }
        Tok::Start(n, a) if n == "p" => {
            if let Some((l, _)) = line.as_mut() {
                if l.align.is_none() {
                    l.align = val(&a, "paraPrIDRef").and_then(|p| aligns.get(p)).copied();
                } else {
                    l.text.push(' ');
                }
            }
        }
        Tok::Start(n, a) | Tok::Empty(n, a) if n == "autoNum" => {
            if let Some((l, _)) = line.as_mut()
                && val(&a, "numType") == Some("PAGE")
            {
                l.page_number = true;
            }
        }
        Tok::Start(n, _) if n == "t" => in_t = true,
        Tok::End(n) if n == "t" => in_t = false,
        Tok::Empty(n, _) if matches!(n.as_str(), "nbSpace" | "fwSpace" | "tab") => {
            if let Some((l, _)) = line.as_mut() {
                l.text.push(' ');
            }
        }
        Tok::Text(s) if in_t => {
            if let Some((l, _)) = line.as_mut() {
                l.text.push_str(&s);
            }
        }
        Tok::Ref(r) if in_t => {
            if let (Some((l, _)), Some(c)) = (line.as_mut(), entity(&r)) {
                l.text.push(c);
            }
        }
        _ => {}
    });
}

fn entity(name: &str) -> Option<char> {
    match name {
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
    }
}

/// What a subtree left out is, by the element that opens it.
fn left_out(name: &str) -> Option<Option<SkipKind>> {
    Some(match name {
        "tbl" => Some(SkipKind::Table),
        "pic" | "ole" | "container" | "equation" | "rect" | "ellipse" | "arc" | "polygon"
        | "curve" | "connectLine" | "line" | "textart" | "video" | "chart" => Some(SkipKind::Image),
        // Controls (머리말, 꼬리말, 쪽 번호, 각주 …), the section's page setup,
        // and 한글's own line layout cache: nothing of the text.
        "ctrl" | "secPr" | "linesegarray" => None,
        _ => return None,
    })
}

fn section(
    xml: &str,
    chars: &HashMap<String, Run>,
    outlines: &HashMap<String, u8>,
    styles: &HashMap<String, StyleKind>,
    items: &mut Vec<Item>,
) -> Result<(), String> {
    struct Open {
        para: Para,
        heading: Option<u8>,
        toc: bool,
    }
    // Paragraphs nest in tables and controls; only top-level ones are read,
    // so an open paragraph is kept per depth of `p`.
    let mut stack: Vec<Open> = Vec::new();
    let mut run = Run::default();
    let mut in_t = false;
    let mut ignoring = 0usize;

    let close = |open: Open, items: &mut Vec<Item>| {
        let Open {
            mut para,
            heading,
            toc,
        } = open;
        trim_ends(&mut para.inlines);
        let text = super::plain(&para.inlines);
        if !text.trim().is_empty() && !toc {
            let single = !text.contains('\n');
            if is_scene(&text) && single && heading.is_none() {
                items.push(Item::Scene);
            } else if let (Some(level), true) = (heading, single) {
                items.push(Item::Heading {
                    level,
                    text: text.trim().to_string(),
                });
            } else {
                items.push(Item::Para(para.inlines));
            }
        }
        for (kind, n) in para.skips {
            items.push(Item::Skip(kind, n));
        }
    };

    walk(xml, |tok| {
        if ignoring > 0 {
            match tok {
                // Footnotes live in controls: counted where they are.
                Tok::Start(n, _) => {
                    ignoring += 1;
                    if matches!(n.as_str(), "footNote" | "endNote")
                        && let Some(open) = stack.last_mut()
                    {
                        open.para.skip(SkipKind::Footnote);
                    }
                }
                Tok::End(_) => ignoring -= 1,
                _ => {}
            }
            return;
        }
        match tok {
            Tok::Start(n, a) => {
                if let Some(kind) = left_out(&n) {
                    if let (Some(kind), Some(open)) = (kind, stack.last_mut()) {
                        open.para.skip(kind);
                    }
                    ignoring = 1;
                    return;
                }
                match n.as_str() {
                    "p" => {
                        let style = val(&a, "styleIDRef").and_then(|s| styles.get(s)).copied();
                        let heading = match style {
                            Some(StyleKind::Heading(level)) => Some(level),
                            _ => val(&a, "paraPrIDRef")
                                .and_then(|p| outlines.get(p))
                                .copied(),
                        };
                        stack.push(Open {
                            para: Para::default(),
                            heading,
                            toc: style == Some(StyleKind::Toc),
                        });
                    }
                    "run" => {
                        run = val(&a, "charPrIDRef")
                            .and_then(|c| chars.get(c))
                            .copied()
                            .unwrap_or_default();
                    }
                    "t" => in_t = true,
                    _ => {}
                }
            }
            Tok::Empty(n, _) => {
                if let Some(kind) = left_out(&n) {
                    if let (Some(kind), Some(open)) = (kind, stack.last_mut()) {
                        open.para.skip(kind);
                    }
                    return;
                }
                let Some(open) = stack.last_mut() else { return };
                match n.as_str() {
                    "lineBreak" => open.para.inlines.push(Inline::HardBreak {}),
                    "tab" => open.para.add_text(" ", run),
                    "nbSpace" => open.para.add_text("\u{A0}", run),
                    "fwSpace" => open.para.add_text("\u{2002}", run),
                    "hyphen" => open.para.add_text("-", run),
                    _ => {}
                }
            }
            Tok::End(n) => match n.as_str() {
                "p" => {
                    if let Some(open) = stack.pop() {
                        close(open, items);
                    }
                }
                "t" => in_t = false,
                _ => {}
            },
            Tok::Text(s) => {
                if in_t && let Some(open) = stack.last_mut() {
                    open.para.add_text(&s, run);
                }
            }
            Tok::Ref(name) => {
                if in_t && let (Some(c), Some(open)) = (entity(&name), stack.last_mut()) {
                    open.para.add_text(&c.to_string(), run);
                }
            }
        }
    })
    .map_err(|_| BROKEN.to_string())
}

pub(super) fn read(bytes: &[u8]) -> Result<Raw, String> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|_| BROKEN.to_string())?;
    let header = read_part(&mut archive, "Contents/header.xml")
        .ok_or_else(|| BROKEN.to_string())?
        .map_err(|_| BROKEN.to_string())?;
    // Sections in order: section0.xml, section1.xml, …
    let mut sections: Vec<(u32, String)> = archive
        .file_names()
        .filter_map(|name| {
            let n = name
                .strip_prefix("Contents/section")?
                .strip_suffix(".xml")?
                .parse()
                .ok()?;
            Some((n, name.to_string()))
        })
        .collect();
    sections.sort();
    if sections.is_empty() {
        return Err(BROKEN.into());
    }
    let (chars, outlines, styles) = (
        char_shapes(&header),
        outline_shapes(&header),
        styles(&header),
    );
    let aligns = para_aligns(&header);
    let mut items = Vec::new();
    let mut found = Found::default();
    for (n, (_, name)) in sections.into_iter().enumerate() {
        let xml = read_part(&mut archive, &name)
            .ok_or_else(|| BROKEN.to_string())?
            .map_err(|_| BROKEN.to_string())?;
        section(&xml, &chars, &outlines, &styles, &mut items)?;
        pages(&xml, &aligns, n == 0, &mut found);
    }
    let page = found.setup();
    Ok(Raw {
        items,
        encoding: None,
        page,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markup::Mark;

    #[test]
    fn reads_text_marks_and_leaves_out_tables() {
        let header = r#"<hh:head xmlns:hh="h"><hh:charProperties>
            <hh:charPr id="0" symMark="NONE"><hh:underline type="NONE"/><hh:strikeout shape="NONE"/></hh:charPr>
            <hh:charPr id="1" symMark="DOT_ABOVE"><hh:bold/><hh:underline type="BOTTOM"/><hh:strikeout shape="NONE"/></hh:charPr>
            </hh:charProperties><hh:paraProperties>
            <hh:paraPr id="0"><hh:heading type="NONE" idRef="0" level="0"/></hh:paraPr>
            <hh:paraPr id="5"><hh:heading type="OUTLINE" idRef="0" level="1"/></hh:paraPr>
            </hh:paraProperties><hh:styles>
            <hh:style id="0" type="PARA" name="바탕글" engName="Normal"/>
            <hh:style id="2" type="PARA" name="개요 1" engName="Outline 1"/>
            <hh:style id="19" type="PARA" name="차례 1" engName="TOC 1"/>
            </hh:styles></hh:head>"#;
        let section_xml = r#"<hs:sec xmlns:hs="s" xmlns:hp="p">
            <hp:p paraPrIDRef="0" styleIDRef="0"><hp:run charPrIDRef="0"><hp:secPr><hp:pagePr/></hp:secPr><hp:ctrl><hp:header><hp:subList><hp:p><hp:run><hp:t>머리말</hp:t></hp:run></hp:p></hp:subList></hp:header></hp:ctrl></hp:run></hp:p>
            <hp:p paraPrIDRef="0" styleIDRef="2"><hp:run charPrIDRef="0"><hp:t>1화 비에 젖은 손님</hp:t></hp:run></hp:p>
            <hp:p paraPrIDRef="0" styleIDRef="19"><hp:run charPrIDRef="0"><hp:t>차례에 있는 줄</hp:t></hp:run></hp:p>
            <hp:p paraPrIDRef="0" styleIDRef="0"><hp:run charPrIDRef="0"><hp:t>셔터를 &amp; </hp:t></hp:run><hp:run charPrIDRef="1"><hp:t>반쯤<hp:lineBreak/>내렸다<hp:nbSpace/>끝</hp:t></hp:run></hp:p>
            <hp:p paraPrIDRef="0" styleIDRef="0"><hp:run charPrIDRef="0"><hp:tbl><hp:tr><hp:tc><hp:subList><hp:p><hp:run><hp:t>표 안</hp:t></hp:run></hp:p></hp:subList></hp:tc></hp:tr></hp:tbl><hp:t>표 뒤 글</hp:t><hp:ctrl><hp:footNote><hp:subList><hp:p><hp:run><hp:t>각주</hp:t></hp:run></hp:p></hp:subList></hp:footNote></hp:ctrl></hp:run></hp:p>
            <hp:p paraPrIDRef="5" styleIDRef="0"><hp:run charPrIDRef="0"><hp:t>2화 개요 수준으로</hp:t></hp:run></hp:p>
            <hp:p paraPrIDRef="0" styleIDRef="0"><hp:run charPrIDRef="0"><hp:t>***</hp:t></hp:run></hp:p>
            </hs:sec>"#;
        let mut items = Vec::new();
        section(
            section_xml,
            &char_shapes(header),
            &outline_shapes(header),
            &styles(header),
            &mut items,
        )
        .unwrap();
        let kinds: Vec<String> = items
            .iter()
            .map(|i| match i {
                Item::Para(inl) => format!("p:{}", super::super::plain(inl)),
                Item::Heading { level, text } => format!("h{level}:{text}"),
                Item::Scene => "scene".into(),
                Item::Skip(k, n) => format!("skip:{}:{n}", k.label()),
            })
            .collect();
        assert_eq!(
            kinds,
            [
                "h1:1화 비에 젖은 손님",
                "p:셔터를 & 반쯤\n내렸다\u{a0}끝",
                "p:표 뒤 글",
                "skip:표:1",
                "skip:각주:1",
                "h2:2화 개요 수준으로",
                "scene",
            ]
        );
        let Item::Para(inlines) = &items[1] else {
            panic!()
        };
        let Inline::Text { marks, .. } = &inlines[1] else {
            panic!()
        };
        assert!(
            marks.contains(&Mark::Bold {})
                && marks.contains(&Mark::Dot {})
                && marks.contains(&Mark::Underline {})
        );
    }
}
