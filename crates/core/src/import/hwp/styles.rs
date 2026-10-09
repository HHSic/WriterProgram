//! DocInfo character marks and heading styles. Fonts and page layout are not imported.

use super::BROKEN;
use super::records::{u32_at, walk, wide_string};
use crate::import::hangul::StyleKind;
use crate::import::para::Run;
use crate::import::shape::Shape;

#[derive(Default)]
pub(super) struct Styles {
    pub chars: Vec<Run>,
    pub shapes: Vec<Shape>,
    pub outlines: Vec<Option<u8>>,
    pub names: Vec<Option<StyleKind>>,
}

fn heading(name: &str) -> Option<StyleKind> {
    let lower = name.to_lowercase();
    if name == "장 제목" || lower == "chapter title" {
        return Some(StyleKind::Heading(1));
    }
    if name.starts_with("차례") || lower.starts_with("toc") {
        return Some(StyleKind::Toc);
    }
    let level = name
        .strip_prefix("개요 ")
        .or_else(|| name.strip_prefix("제목 "))
        .or_else(|| lower.strip_prefix("outline "))
        .or_else(|| lower.strip_prefix("heading "))?
        .trim()
        .parse::<u8>()
        .ok()
        .filter(|n| (1..=9).contains(n))?;
    Some(StyleKind::Heading(level))
}

pub(super) fn read(data: &[u8]) -> Result<Styles, String> {
    let mut styles = Styles::default();
    walk(data, |record| {
        match record.tag {
            21 => {
                if record.data.len() < 68 {
                    return Err(BROKEN.into());
                }
                let flags = u32_at(record.data, 46)?;
                styles.chars.push(Run {
                    height: f64::from(u32_at(record.data, 42)?),
                    text_color: {
                        let c = u32_at(record.data, 52)?;
                        (c != 0 && c != u32::MAX)
                            .then_some((c & 255) << 16 | (c & 0xff00) | (c >> 16 & 255))
                    },
                    italic: flags & 1 != 0,
                    bold: flags & 2 != 0,
                    underline: flags >> 2 & 3 == 1,
                    strike: flags >> 18 & 7 != 0,
                    dot: flags >> 21 & 15 != 0,
                    ..Run::default()
                });
            }
            25 => {
                if record.data.len() < 42 {
                    return Err(BROKEN.into());
                }
                let flags = u32_at(record.data, 0)?;
                let dimension = |at| u32_at(record.data, at).map(|v| f64::from(v as i32) / 2.0);
                styles.shapes.push(Shape {
                    align: Some(match flags >> 2 & 7 {
                        1 => crate::markup::Align::Left,
                        2 => crate::markup::Align::Right,
                        3 => crate::markup::Align::Center,
                        _ => crate::markup::Align::Justify,
                    }),
                    page_break: flags & (1 << 19) != 0,
                    left: dimension(4)?,
                    right: dimension(8)?,
                    indent: dimension(12)?,
                });
                styles
                    .outlines
                    .push((flags >> 23 & 3 == 1).then_some(((flags >> 25 & 7) + 1) as u8));
            }
            26 => {
                let mut offset = 0;
                let local = wide_string(record.data, &mut offset)?;
                let english = wide_string(record.data, &mut offset)?;
                styles
                    .names
                    .push(heading(&local).or_else(|| heading(&english)));
            }
            _ => {}
        }
        Ok(())
    })?;
    Ok(styles)
}
