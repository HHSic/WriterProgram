//! Corrected Word files (docx).
//!
//! - Track changes: runs inside `w:ins` (or `w:moveTo`) are insertions,
//!   runs inside `w:del` (or `w:moveFrom`, text in `w:delText`) deletions,
//!   with `w:author` and `w:date`. `w:ins`/`w:del` in a paragraph mark's
//!   properties insert or delete the paragraph break itself.
//! - Comments: `w:commentRangeStart` … `w:commentRangeEnd` around the text,
//!   the comment itself in word/comments.xml.

use std::collections::HashMap;
use std::io::Cursor;

use zip::ZipArchive;

use super::super::para::{Para, Run};
use super::super::word::{Styles, property};
use super::super::xml::{Attrs, Tok, read_part, val, walk};
use super::{Builder, Edit, FileNote, Look, Marked, Tracked, Who};

const BROKEN: &str = "Word 문서(docx)가 아니거나 손상됨";

/// word/comments.xml: comment id to its text, author and date.
fn comments(xml: &str) -> HashMap<String, FileNote> {
    let mut out = HashMap::new();
    let mut current: Option<(String, FileNote)> = None;
    let mut in_t = false;
    let _ = walk(xml, |tok| match tok {
        Tok::Start(n, a) if n == "comment" => {
            current = Some((
                val(&a, "id").unwrap_or_default().to_string(),
                FileNote {
                    author: val(&a, "author").unwrap_or_default().to_string(),
                    date: val(&a, "date").unwrap_or_default().to_string(),
                    ..FileNote::default()
                },
            ));
        }
        Tok::End(n) if n == "comment" => {
            if let Some((id, note)) = current.take() {
                out.insert(id, note);
            }
        }
        Tok::Start(n, _) if n == "p" => {
            if let Some((_, note)) = current.as_mut()
                && !note.text.is_empty()
            {
                note.text.push('\n');
            }
        }
        Tok::Start(n, _) if n == "t" => in_t = true,
        Tok::End(n) if n == "t" => in_t = false,
        Tok::Text(s) if in_t => {
            if let Some((_, note)) = current.as_mut() {
                note.text.push_str(&s);
            }
        }
        _ => {}
    });
    out
}

struct Reader<'a> {
    b: Builder,
    styles: &'a Styles,
    para: Option<Para>,
    /// Track change of the paragraph mark of the open paragraph.
    para_end: Option<Tracked>,
    run: Run,
    in_ppr: bool,
    in_rpr: bool,
    in_t: bool,
    /// Open `w:ins`/`w:del` around runs, innermost last.
    tracks: Vec<Tracked>,
    skipping: usize,
    /// Comment id to where its range starts, and finished ranges.
    starts: HashMap<String, usize>,
    ranges: Vec<(String, usize, usize)>,
}

impl Reader<'_> {
    fn who(&mut self, a: &Attrs) -> usize {
        self.b.out.who.push(Who {
            author: val(a, "author").unwrap_or_default().to_string(),
            date: val(a, "date").unwrap_or_default().to_string(),
        });
        self.b.out.who.len() - 1
    }

    fn text(&mut self, s: &str) {
        if self.para.is_some() {
            let tracked = self.tracks.last().copied();
            self.b.push(s, Look::of(self.run), tracked);
        }
    }

    fn close(&mut self) {
        let Some(p) = self.para.take() else { return };
        let heading = p
            .style
            .as_deref()
            .is_some_and(|s| self.styles.is_toc(s) || self.styles.heading(s).is_some())
            || p.outline.is_some_and(|l| l < 9);
        let end = self.para_end.take();
        self.b.end(heading, end);
    }

    fn step(&mut self, tok: Tok) {
        if self.skipping > 0 {
            match tok {
                Tok::Start(..) => self.skipping += 1,
                Tok::End(_) => self.skipping -= 1,
                _ => {}
            }
            return;
        }
        match tok {
            Tok::Start(n, a) => match n.as_str() {
                "tbl" | "drawing" | "pict" | "object" | "Fallback" | "rt" | "rPrChange"
                | "pPrChange" => self.skipping = 1,
                "p" => {
                    self.close();
                    self.para = Some(Para::default());
                    self.b.begin();
                }
                "pPr" => self.in_ppr = true,
                "rPr" => self.in_rpr = true,
                "r" => self.run = Run::default(),
                "t" | "delText" => self.in_t = true,
                "ins" | "moveTo" | "del" | "moveFrom" if !self.in_ppr => {
                    let edit = if matches!(n.as_str(), "ins" | "moveTo") {
                        Edit::Insert
                    } else {
                        Edit::Delete
                    };
                    let by = self.who(&a);
                    self.tracks.push(Tracked { edit, by });
                }
                _ => self.attr(&n, &a),
            },
            Tok::Empty(n, a) => match n.as_str() {
                "tab" => self.text(" "),
                "noBreakHyphen" => self.text("-"),
                "br" if !matches!(val(&a, "type"), Some("page" | "column")) => self.text("\n"),
                "cr" => self.text("\n"),
                "commentRangeStart" => {
                    let id = val(&a, "id").unwrap_or_default().to_string();
                    self.starts.insert(id, self.b.out.cells.len());
                }
                "commentRangeEnd" => {
                    let id = val(&a, "id").unwrap_or_default().to_string();
                    if let Some(start) = self.starts.remove(&id) {
                        self.ranges.push((id, start, self.b.out.cells.len()));
                    }
                }
                "commentReference" => {
                    let id = val(&a, "id").unwrap_or_default().to_string();
                    if !self.starts.contains_key(&id) && !self.ranges.iter().any(|r| r.0 == id) {
                        let at = self.b.out.cells.len();
                        self.ranges.push((id, at, at));
                    }
                }
                _ => self.attr(&n, &a),
            },
            Tok::End(n) => match n.as_str() {
                "p" => self.close(),
                "pPr" => self.in_ppr = false,
                "rPr" => self.in_rpr = false,
                "t" | "delText" => self.in_t = false,
                "ins" | "moveTo" | "del" | "moveFrom" if !self.in_ppr => {
                    self.tracks.pop();
                }
                _ => {}
            },
            Tok::Text(s) => {
                if self.in_t {
                    self.text(&s);
                }
            }
        }
    }

    /// Paragraph, paragraph mark and run properties.
    fn attr(&mut self, n: &str, a: &Attrs) {
        if self.in_ppr && self.in_rpr {
            // The paragraph mark: inserted or deleted with track changes.
            if matches!(n, "ins" | "del") {
                let edit = if n == "ins" {
                    Edit::Insert
                } else {
                    Edit::Delete
                };
                let by = self.who(a);
                self.para_end = Some(Tracked { edit, by });
            }
            return;
        }
        property(
            n,
            a,
            self.in_ppr,
            self.in_rpr && !self.in_ppr,
            &mut self.run,
            &mut self.para,
        );
    }
}

pub(super) fn read(bytes: &[u8], scene_mark: &str) -> Result<Marked, String> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|_| BROKEN.to_string())?;
    let document =
        read_part(&mut archive, "word/document.xml").ok_or_else(|| BROKEN.to_string())??;
    let styles = read_part(&mut archive, "word/styles.xml")
        .and_then(Result::ok)
        .map(|xml| Styles::read(&xml))
        .unwrap_or_default();
    let mut notes = read_part(&mut archive, "word/comments.xml")
        .and_then(Result::ok)
        .map(|xml| comments(&xml))
        .unwrap_or_default();
    let mut r = Reader {
        b: Builder::new(scene_mark),
        styles: &styles,
        para: None,
        para_end: None,
        run: Run::default(),
        in_ppr: false,
        in_rpr: false,
        in_t: false,
        tracks: Vec::new(),
        skipping: 0,
        starts: HashMap::new(),
        ranges: Vec::new(),
    };
    walk(&document, |tok| r.step(tok))?;
    r.close();
    let end = r.b.out.cells.len();
    let open: Vec<(String, usize)> = r.starts.drain().collect();
    r.ranges
        .extend(open.into_iter().map(|(id, s)| (id, s, end)));
    r.ranges.sort_by_key(|(_, s, _)| *s);
    let mut out: Vec<FileNote> = Vec::new();
    for (id, s, e) in std::mem::take(&mut r.ranges) {
        if let Some(mut note) = notes.remove(&id) {
            note.on = Some((s, e));
            out.push(note);
        }
    }
    // Comments with no place in the text.
    let mut rest: Vec<(String, FileNote)> = notes.into_iter().collect();
    rest.sort_by(|a, b| a.0.cmp(&b.0));
    out.extend(rest.into_iter().map(|(_, n)| n));
    out.retain(|n| !n.text.trim().is_empty());
    for n in &mut out {
        n.text = n.text.trim().to_string();
    }
    r.b.out.notes = out;
    Ok(r.b.finish())
}
