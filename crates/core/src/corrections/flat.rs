//! A chapter as one stream of characters, every paragraph (empty ones too)
//! ending with [`PARA`]: positions in it, changing it, turning it back into
//! paragraphs, and finding where a place in one version of a chapter is in
//! another.

use std::ops::Range;

use similar::{Algorithm, DiffTag, capture_diff_slices};

use super::compare::paragraphs;
use super::{Pos, Span};
use crate::import::marked::{PARA, SCENE};
use crate::markup::{Block, Inline, Mark, ParaAttrs};

/// A character with its marks; a [`PARA`] carries its paragraph's shape.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Cell {
    pub ch: char,
    pub marks: Vec<Mark>,
    pub attrs: ParaAttrs,
}

impl Cell {
    fn plain(ch: char) -> Cell {
        Cell {
            ch,
            marks: Vec::new(),
            attrs: ParaAttrs::default(),
        }
    }
}

/// The characters of one block: line breaks as `\n`, a scene break as
/// [`SCENE`].
pub(crate) fn block_cells(block: &Block) -> Vec<Cell> {
    let mut out = Vec::new();
    match block {
        Block::SceneBreak {} => out.push(Cell::plain(SCENE)),
        Block::Paragraph { content, .. } => {
            for inline in content {
                match inline {
                    Inline::HardBreak {} => out.push(Cell::plain('\n')),
                    Inline::Text { text, marks } => {
                        for ch in text.chars() {
                            out.push(if ch == '\n' {
                                Cell::plain('\n')
                            } else {
                                Cell {
                                    ch,
                                    marks: marks.clone(),
                                    attrs: ParaAttrs::default(),
                                }
                            });
                        }
                    }
                }
            }
        }
    }
    out
}

/// A chapter as cells, and where each block starts.
pub(crate) struct Flat {
    pub cells: Vec<Cell>,
    pub starts: Vec<usize>,
}

impl Flat {
    pub(crate) fn of(blocks: &[Block]) -> Flat {
        let mut cells = Vec::new();
        let mut starts = Vec::with_capacity(blocks.len());
        for block in blocks {
            starts.push(cells.len());
            cells.extend(block_cells(block));
            cells.push(Cell {
                ch: PARA,
                marks: Vec::new(),
                attrs: block.attrs(),
            });
        }
        Flat { cells, starts }
    }

    pub(crate) fn chars(&self) -> Vec<char> {
        self.cells.iter().map(|c| c.ch).collect()
    }

    /// Stream position of a place given in characters; past the end for
    /// the block after the last.
    pub(crate) fn index(&self, block: usize, offset: usize) -> usize {
        self.starts
            .get(block)
            .map_or(self.cells.len(), |s| s + offset)
    }

    /// The place of a stream position, in characters.
    pub(crate) fn place(&self, at: usize) -> (usize, usize) {
        if at >= self.cells.len() {
            return (self.starts.len(), 0);
        }
        let block = self.starts.partition_point(|s| *s <= at) - 1;
        (block, at - self.starts[block])
    }

    /// A position as the editor counts it (UTF-16).
    pub(crate) fn pos(&self, at: usize) -> Pos {
        let (block, offset) = self.place(at);
        let start = self.starts.get(block).copied().unwrap_or(0);
        let offset = self.cells[start..start + offset]
            .iter()
            .map(|c| c.ch.len_utf16())
            .sum();
        Pos { block, offset }
    }

    pub(crate) fn span(&self, r: Range<usize>) -> Span {
        Span {
            from: self.pos(r.start),
            to: self.pos(r.end),
        }
    }

    /// The stream position of a place the editor gave (UTF-16).
    pub(crate) fn index_of(&self, pos: Pos) -> usize {
        let Some(&start) = self.starts.get(pos.block) else {
            return self.cells.len();
        };
        let mut units = 0;
        let mut at = start;
        while units < pos.offset && at < self.cells.len() && self.cells[at].ch != PARA {
            units += self.cells[at].ch.len_utf16();
            at += 1;
        }
        at
    }

    pub(crate) fn range_of(&self, span: Span) -> Range<usize> {
        let (s, e) = (self.index_of(span.from), self.index_of(span.to));
        s..e.max(s)
    }

    /// Replaces `range` with `text`. New characters take the marks of the
    /// first character replaced, or of the one before them in the
    /// paragraph; a new paragraph break takes the shape of the paragraph it
    /// splits. Joined paragraphs keep the shape of the first.
    pub(crate) fn splice(&mut self, range: Range<usize>, text: &[char]) {
        let cells = &mut self.cells;
        let usable = |c: &Cell| c.ch != PARA && c.ch != SCENE && c.ch != '\n';
        let marks = if !range.is_empty() && usable(&cells[range.start]) {
            cells[range.start].marks.clone()
        } else if range.start > 0 && usable(&cells[range.start - 1]) {
            cells[range.start - 1].marks.clone()
        } else if cells.get(range.end).is_some_and(usable) {
            cells[range.end].marks.clone()
        } else {
            Vec::new()
        };
        let next_para = cells[range.end.min(cells.len())..]
            .iter()
            .position(|c| c.ch == PARA)
            .map(|p| p + range.end);
        let at_para_start = range.start == 0 || cells[range.start - 1].ch == PARA;
        // A paragraph break taken out from the middle of a paragraph joins
        // it with the next, which takes this paragraph's shape.
        if !at_para_start
            && let Some(first) = cells[range.clone()]
                .iter()
                .find(|c| c.ch == PARA)
                .map(|c| c.attrs)
            && let Some(next) = next_para
        {
            cells[next].attrs = first;
        }
        let split = next_para.map(|p| cells[p].attrs).unwrap_or_default();
        let new: Vec<Cell> = text
            .iter()
            .map(|&ch| match ch {
                PARA => Cell {
                    ch,
                    marks: Vec::new(),
                    attrs: split,
                },
                SCENE | '\n' => Cell::plain(ch),
                _ => Cell {
                    ch,
                    marks: marks.clone(),
                    attrs: ParaAttrs::default(),
                },
            })
            .collect();
        cells.splice(range, new);
    }

    /// Back to blocks.
    pub(crate) fn blocks(&self) -> Vec<Block> {
        let mut out = Vec::new();
        let mut para: Vec<&Cell> = Vec::new();
        let finish = |para: &mut Vec<&Cell>, attrs: ParaAttrs, out: &mut Vec<Block>| {
            if para.len() == 1 && para[0].ch == SCENE {
                out.push(Block::SceneBreak {});
            } else {
                let mut content: Vec<Inline> = Vec::new();
                for c in para.iter().filter(|c| c.ch != SCENE) {
                    if c.ch == '\n' {
                        content.push(Inline::HardBreak {});
                        continue;
                    }
                    match content.last_mut() {
                        Some(Inline::Text { text, marks }) if *marks == c.marks => text.push(c.ch),
                        _ => content.push(Inline::Text {
                            text: c.ch.to_string(),
                            marks: c.marks.clone(),
                        }),
                    }
                }
                out.push(Block::Paragraph { attrs, content });
            }
            para.clear();
        };
        for c in &self.cells {
            if c.ch == PARA {
                finish(&mut para, c.attrs, &mut out);
            } else {
                para.push(c);
            }
        }
        if !para.is_empty() {
            finish(&mut para, ParaAttrs::default(), &mut out);
        }
        out
    }
}

/// Finds where places in the text as sent are in the chapter as it is now.
/// Changes already accepted are taken into account; a place is found only
/// when every paragraph it touches is, as expected, unchanged.
pub(crate) struct Mapper {
    /// Accepted changes: stream range in the sent text, and its new length.
    shifts: Vec<(Range<usize>, usize)>,
    expected: Vec<Range<usize>>,
    current: Vec<Range<usize>>,
    same: Vec<Option<usize>>,
    expected_len: usize,
}

impl Mapper {
    pub(crate) fn new(
        sent: &[char],
        accepted: &[(Range<usize>, Vec<char>)],
        now: &[char],
    ) -> Mapper {
        let mut accepted: Vec<&(Range<usize>, Vec<char>)> = accepted.iter().collect();
        accepted.sort_by_key(|(r, _)| r.start);
        let mut expected: Vec<char> = Vec::with_capacity(sent.len());
        let mut at = 0;
        for (r, text) in &accepted {
            if r.start < at || r.end > sent.len() {
                continue;
            }
            expected.extend_from_slice(&sent[at..r.start]);
            expected.extend_from_slice(text);
            at = r.end;
        }
        expected.extend_from_slice(&sent[at.min(sent.len())..]);
        let (pe, pc) = (paragraphs(&expected), paragraphs(now));
        let ke: Vec<&[char]> = pe.iter().map(|r| &expected[r.clone()]).collect();
        let kc: Vec<&[char]> = pc.iter().map(|r| &now[r.clone()]).collect();
        let mut same = vec![None; pe.len()];
        for op in capture_diff_slices(Algorithm::Patience, &ke, &kc) {
            let (tag, ra, rb) = op.as_tag_tuple();
            if tag == DiffTag::Equal {
                for (i, j) in ra.zip(rb) {
                    same[i] = Some(j);
                }
            }
        }
        Mapper {
            shifts: accepted
                .into_iter()
                .map(|(r, t)| (r.clone(), t.len()))
                .collect(),
            expected: pe,
            current: pc,
            same,
            expected_len: expected.len(),
        }
    }

    /// A sent stream position in the expected text.
    fn shift(&self, p: usize) -> usize {
        let mut out = p as isize;
        for (r, len) in &self.shifts {
            if r.end <= p && !(r.is_empty() && r.start == p) {
                out += *len as isize - r.len() as isize;
            }
        }
        out.max(0) as usize
    }

    /// The paragraph of the expected text holding position `p`.
    fn para(&self, p: usize) -> Option<usize> {
        if self.expected.is_empty() {
            return None;
        }
        if p >= self.expected_len {
            return Some(self.expected.len() - 1);
        }
        Some(self.expected.partition_point(|r| r.end <= p))
    }

    /// Where a range of the sent text is now, or none (겹침).
    pub(crate) fn map(&self, range: Range<usize>) -> Option<Range<usize>> {
        let (s, e) = (self.shift(range.start), self.shift(range.end));
        let k1 = self.para(s)?;
        let k2 = if e > s { self.para(e - 1)? } else { k1 };
        let j1 = self.same[k1]?;
        for (n, k) in (k1..=k2).enumerate() {
            if self.same[k]? != j1 + n {
                return None;
            }
        }
        let j2 = j1 + (k2 - k1);
        let from = self.current[j1].start + (s - self.expected[k1].start);
        let to = if e > s {
            self.current[j2].start + (e - self.expected[k2].start)
        } else {
            from
        };
        Some(from..to)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markup::MemoAttrs;

    #[test]
    fn splices_keep_marks_and_shapes() {
        let bold = vec![Mark::Bold {}];
        let blocks = vec![
            Block::Paragraph {
                attrs: ParaAttrs {
                    left: 2,
                    right: 0,
                    indent: None,
                },
                content: vec![
                    Inline::Text {
                        text: "그는".into(),
                        marks: bold.clone(),
                    },
                    Inline::Text {
                        text: " 웃었다.".into(),
                        marks: vec![Mark::Memo {
                            attrs: MemoAttrs { id: "m".into() },
                        }],
                    },
                ],
            },
            Block::text("둘째"),
            Block::SceneBreak {},
        ];
        let mut flat = Flat::of(&blocks);
        assert_eq!(flat.blocks(), blocks);
        // 는 → 가 keeps bold; the join keeps the first paragraph's margins.
        flat.splice(1..2, &['가']);
        let para_end = flat.index(0, 7);
        flat.splice(para_end..para_end + 1, &[' ']);
        let out = flat.blocks();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].attrs().left, 2);
        assert_eq!(out[0].lines(), ["그가 웃었다. 둘째"]);
        let Block::Paragraph { content, .. } = &out[0] else {
            panic!()
        };
        assert_eq!(
            content[0],
            Inline::Text {
                text: "그가".into(),
                marks: bold
            }
        );
        assert_eq!(out[1], Block::SceneBreak {});
    }

    #[test]
    fn places_follow_accepted_changes_and_stop_at_edited_paragraphs() {
        let s = |t: &str| t.chars().collect::<Vec<char>>();
        let sent = s("가나다\u{2029}라마바\u{2029}사아\u{2029}");
        // Accepted earlier: 나 → 너무.
        let accepted = vec![(1..2, s("너무"))];
        // The writer then changed the third paragraph.
        let now = s("가너무다\u{2029}라마바\u{2029}사아자\u{2029}");
        let m = Mapper::new(&sent, &accepted, &now);
        assert_eq!(m.map(5..6), Some(6..7)); // 마
        assert_eq!(m.map(2..3), Some(3..4)); // 다
        assert_eq!(m.map(8..9), None); // 사: paragraph edited
    }
}
