//! Main paragraphs only; nested table cells, headers and note bodies stay out.
//! Run offsets count UTF-16 units, including the eight-unit controls.

use super::BROKEN;
use super::records::{u16_at, u32_at, walk};
use super::styles::Styles;
use crate::import::hangul::StyleKind;
use crate::import::para::Para;
use crate::import::{Item, SkipKind};

struct Paragraph {
    para: Para,
    units: Vec<u16>,
    runs: Vec<(usize, usize)>,
    shape: usize,
    style: usize,
    expected_units: usize,
    page_break: bool,
}

impl Paragraph {
    fn finish(mut self, styles: &Styles, items: &mut Vec<Item>) -> Result<(), String> {
        if self.units.len() != self.expected_units
            && !(self.units.is_empty() && self.expected_units <= 1)
        {
            return Err(BROKEN.into());
        }
        if self
            .runs
            .first()
            .is_some_and(|(position, _)| *position != 0)
            || self.runs.windows(2).any(|runs| runs[0].0 >= runs[1].0)
            || self.runs.iter().any(|(position, shape)| {
                *position > self.units.len() || *shape >= styles.chars.len()
            })
        {
            return Err(BROKEN.into());
        }
        self.para.keep_blank = self.units == [13];
        let source = styles.shapes.get(self.shape).ok_or(BROKEN)?;
        let height = self
            .runs
            .first()
            .and_then(|(_, id)| styles.chars.get(*id))
            .or_else(|| styles.chars.first())
            .map_or(1000.0, |run| run.height);
        self.para.attrs = source.attrs(height);
        self.para.attrs.page_break_before |= self.page_break;
        let mut position = 0;
        let mut run_index = 0;
        let mut text = String::new();
        let mut shape = self.runs.first().map_or(0, |(_, shape)| *shape);
        while position < self.units.len() {
            while run_index + 1 < self.runs.len() && self.runs[run_index + 1].0 <= position {
                self.para
                    .add_text(&text, styles.chars.get(shape).copied().unwrap_or_default());
                text.clear();
                run_index += 1;
                shape = self.runs[run_index].1;
            }
            let unit = self.units[position];
            let length = match unit {
                1..=9 | 11..=12 | 14..=23 => {
                    if self.units.get(position + 7) != Some(&unit) {
                        return Err(BROKEN.into());
                    }
                    if unit == 9 {
                        text.push('\t');
                    }
                    8
                }
                10 => {
                    text.push('\n');
                    1
                }
                24 => {
                    text.push('-');
                    1
                }
                30 => {
                    text.push('\u{00a0}');
                    1
                }
                31 => {
                    text.push('\u{2002}');
                    1
                }
                0 | 13 | 25..=29 => 1,
                0xd800..=0xdbff => {
                    let low = *self
                        .units
                        .get(position + 1)
                        .filter(|n| (0xdc00..=0xdfff).contains(*n))
                        .ok_or(BROKEN)?;
                    if self
                        .runs
                        .get(run_index + 1)
                        .is_some_and(|(start, _)| *start == position + 1)
                    {
                        return Err(BROKEN.into());
                    }
                    let scalar =
                        0x10000 + ((u32::from(unit) - 0xd800) << 10) + (u32::from(low) - 0xdc00);
                    text.push(char::from_u32(scalar).ok_or(BROKEN)?);
                    2
                }
                0xdc00..=0xdfff => return Err(BROKEN.into()),
                _ => {
                    text.push(char::from_u32(u32::from(unit)).ok_or(BROKEN)?);
                    1
                }
            };
            position += length;
        }
        self.para
            .add_text(&text, styles.chars.get(shape).copied().unwrap_or_default());
        let kind = styles.names.get(self.style).copied().flatten();
        let level = match kind {
            Some(StyleKind::Heading(level)) => Some(level),
            _ => styles.outlines.get(self.shape).copied().flatten(),
        };
        self.para.close(level, kind == Some(StyleKind::Toc), items);
        Ok(())
    }
}

pub(super) fn read(data: &[u8], styles: &Styles, items: &mut Vec<Item>) -> Result<(), String> {
    let mut current: Option<Paragraph> = None;
    walk(data, |record| {
        if record.tag == 66 && record.level == 0 {
            if let Some(paragraph) = current.take() {
                paragraph.finish(styles, items)?;
            }
            if record.data.len() < 22 {
                return Err(BROKEN.into());
            }
            current = Some(Paragraph {
                para: Para {
                    preserve_spaces: true,
                    ..Para::default()
                },
                units: Vec::new(),
                runs: Vec::new(),
                page_break: record.data[11] & 4 != 0,
                expected_units: (u32_at(record.data, 0)? & 0x7fff_ffff) as usize,
                shape: usize::from(u16_at(record.data, 8)?),
                style: usize::from(record.data[10]),
            });
        } else if record.level == 1 {
            match record.tag {
                67 => {
                    let paragraph = current.as_mut().ok_or(BROKEN)?;
                    if record.data.len() % 2 != 0 {
                        return Err(BROKEN.into());
                    }
                    paragraph.units.extend(
                        record
                            .data
                            .as_chunks::<2>()
                            .0
                            .iter()
                            .map(|b| u16::from_le_bytes([b[0], b[1]])),
                    );
                }
                68 => {
                    let paragraph = current.as_mut().ok_or(BROKEN)?;
                    if record.data.len() % 8 != 0 {
                        return Err(BROKEN.into());
                    }
                    for at in (0..record.data.len()).step_by(8) {
                        paragraph.runs.push((
                            u32_at(record.data, at)? as usize,
                            u32_at(record.data, at + 4)? as usize,
                        ));
                    }
                }
                71 => {
                    let id = u32_at(record.data, 0)?.to_be_bytes();
                    let skip = match &id {
                        b"tbl " => Some(SkipKind::Table),
                        b"fn  " | b"en  " | b"%fn " | b"%en " => Some(SkipKind::Footnote),
                        b"gso " | b"eqed" => Some(SkipKind::Image),
                        _ => None,
                    };
                    if let Some(skip) = skip {
                        current.as_mut().ok_or(BROKEN)?.para.skip(skip);
                    }
                }
                _ => {}
            }
        }
        Ok(())
    })?;
    if let Some(paragraph) = current {
        paragraph.finish(styles, items)?;
    }
    Ok(())
}
