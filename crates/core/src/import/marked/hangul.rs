//! Corrected 한글 files (HWPX).
//!
//! - 변경 내용 추적: `hp:insertBegin`/`hp:insertEnd` and
//!   `hp:deleteBegin`/`hp:deleteEnd` inside `hp:t`, whose `TcId` points at an
//!   `hh:trackChange` in header.xml (date, `authorID` into
//!   `hh:trackChangeAuthor`). A change that runs over the end of a paragraph
//!   inserts or deletes the paragraph break itself.
//! - 형광펜: `hp:markpenBegin`/`hp:markpenEnd` inside `hp:t`.
//! - 메모: a `MEMO` field (`hp:fieldBegin` … `hp:fieldEnd`) around the text,
//!   whose parameters name the memo, its author and date; the memo's text is
//!   in the field's own paragraphs or in an `hp:memo` (of `hp:memogroup`).

use std::collections::HashMap;
use std::io::Cursor;

use zip::ZipArchive;

use super::super::hangul::{BROKEN, StyleKind, char_shapes, left_out, outline_shapes, styles};
use super::super::para::Run;
use super::super::xml::{Attrs, Tok, read_part, val, walk};
use super::{Builder, Edit, FileNote, Look, Marked, Tracked, Who};

/// First attribute found among spellings of one name.
fn any_val<'a>(attrs: &'a Attrs, keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|k| val(attrs, k))
}

/// Tracked changes in header.xml: `trackChange` id to who and when.
fn track_changes(header: &str) -> HashMap<String, Who> {
    let mut authors: HashMap<String, String> = HashMap::new();
    let mut changes: Vec<(String, Option<String>, String, Option<String>)> = Vec::new();
    let _ = walk(header, |tok| {
        if let Tok::Start(n, a) | Tok::Empty(n, a) = tok {
            match n.as_str() {
                "trackChangeAuthor" => {
                    if let Some(id) = val(&a, "id") {
                        authors.insert(
                            id.to_string(),
                            any_val(&a, &["name", "author"])
                                .unwrap_or_default()
                                .to_string(),
                        );
                    }
                }
                "trackChange" => {
                    if let Some(id) = val(&a, "id") {
                        changes.push((
                            id.to_string(),
                            any_val(&a, &["authorID", "authorId", "authorIDRef"])
                                .map(str::to_string),
                            val(&a, "date").unwrap_or_default().to_string(),
                            val(&a, "author").map(str::to_string),
                        ));
                    }
                }
                _ => {}
            }
        }
    });
    changes
        .into_iter()
        .map(|(id, author_id, date, author)| {
            let author = author
                .or_else(|| author_id.and_then(|a| authors.get(&a).cloned()))
                .unwrap_or_default();
            (id, Who { author, date })
        })
        .collect()
}

/// A 메모 field being read.
#[derive(Default)]
struct Field {
    id: String,
    params: Vec<(String, String)>,
    text: String,
    start: usize,
    end: Option<usize>,
}

impl Field {
    fn param(&self, names: &[&str]) -> Option<&str> {
        self.params
            .iter()
            .find(|(n, _)| names.iter().any(|w| n.eq_ignore_ascii_case(w)))
            .map(|(_, v)| v.as_str())
            .filter(|v| !v.is_empty())
    }
}

/// An `hp:memo` of the memo group.
#[derive(Default)]
struct Memo {
    id: String,
    text: String,
    author: String,
    date: String,
}

struct Reader<'a> {
    b: Builder,
    chars: HashMap<String, Run>,
    outlines: HashMap<String, u8>,
    styles: HashMap<String, StyleKind>,
    changes: &'a HashMap<String, Who>,
    /// `trackChange` id to index in `b.out.who`.
    who_of: HashMap<String, usize>,
    run: Run,
    shade: bool,
    tracked: Option<Tracked>,
    /// Open top-level paragraph: is it a heading?
    para: Option<bool>,
    depth_p: usize,
    in_t: bool,
    /// Depth inside a subtree left out (tables, pictures, controls).
    skipping: usize,
    /// Fields being read (inside a control) and fields whose text is open.
    reading: Option<Field>,
    param: Option<String>,
    open: Vec<Field>,
    done: Vec<Field>,
    memo: Option<Memo>,
    memo_depth: usize,
    memos: Vec<Memo>,
}

impl Reader<'_> {
    fn who(&mut self, attrs: &Attrs) -> usize {
        let id = any_val(attrs, &["TcId", "tcId", "tcid", "Id", "id"])
            .unwrap_or_default()
            .to_string();
        if let Some(&i) = self.who_of.get(&id) {
            return i;
        }
        let who = self.changes.get(&id).cloned().unwrap_or_default();
        let i = self.b.out.who.len();
        self.b.out.who.push(who);
        self.who_of.insert(id, i);
        i
    }

    fn look(&self) -> Look {
        let mut look = Look::of(self.run);
        look.shade |= self.shade;
        look
    }

    fn text(&mut self, s: &str) {
        if self.para.is_some() {
            let (look, tracked) = (self.look(), self.tracked);
            self.b.push(s, look, tracked);
        }
    }

    /// Tokens inside a control: only 메모 fields are read.
    fn control(&mut self, tok: Tok) {
        match tok {
            Tok::Start(n, a) | Tok::Empty(n, a) if n == "fieldBegin" => {
                if val(&a, "type").is_some_and(|t| t.eq_ignore_ascii_case("MEMO")) {
                    self.reading = Some(Field {
                        id: val(&a, "id").unwrap_or_default().to_string(),
                        start: self.b.out.cells.len(),
                        ..Field::default()
                    });
                }
            }
            Tok::Start(n, a) | Tok::Empty(n, a) if n == "fieldEnd" => {
                let begin = any_val(&a, &["beginIDRef", "beginIdRef"]).unwrap_or_default();
                let at = self
                    .open
                    .iter()
                    .rposition(|f| f.id == begin)
                    .or_else(|| (!self.open.is_empty()).then(|| self.open.len() - 1));
                if let Some(at) = at {
                    let mut field = self.open.remove(at);
                    field.end = Some(self.b.out.cells.len());
                    self.done.push(field);
                }
            }
            Tok::Start(n, a) if n.ends_with("Param") && self.reading.is_some() => {
                self.param = Some(val(&a, "name").unwrap_or_default().to_string());
            }
            Tok::End(n) if n.ends_with("Param") => self.param = None,
            Tok::Start(n, _) if n == "p" => {
                if let Some(f) = self.reading.as_mut()
                    && !f.text.is_empty()
                {
                    f.text.push('\n');
                }
            }
            Tok::Text(s) => {
                if let Some(f) = self.reading.as_mut() {
                    match self.param.as_ref() {
                        Some(name) => f.params.push((name.clone(), s.trim().to_string())),
                        None if self.in_t => f.text.push_str(&s),
                        None => {}
                    }
                }
            }
            _ => {}
        }
    }

    fn step(&mut self, tok: Tok) {
        // Memo group: the memos' own text.
        if let Some(memo) = self.memo.as_mut() {
            match tok {
                Tok::Start(n, _) => {
                    self.memo_depth += 1;
                    if n == "p" && !memo.text.is_empty() {
                        memo.text.push('\n');
                    }
                    if n == "t" {
                        self.in_t = true;
                    }
                }
                Tok::End(n) => {
                    if n == "t" {
                        self.in_t = false;
                    }
                    self.memo_depth -= 1;
                    if self.memo_depth == 0 {
                        let memo = self.memo.take().expect("open");
                        self.memos.push(memo);
                    }
                }
                Tok::Text(s) if self.in_t => memo.text.push_str(&s),
                _ => {}
            }
            return;
        }
        if self.skipping > 0 {
            match &tok {
                Tok::Start(n, _) => {
                    if n == "t" {
                        self.in_t = true;
                    }
                    self.skipping += 1;
                }
                Tok::End(n) => {
                    if n == "t" {
                        self.in_t = false;
                    }
                    self.skipping -= 1;
                }
                _ => {}
            }
            self.control(tok);
            if self.skipping == 0 {
                self.in_t = false;
                // A 메모 field read in this control covers what follows.
                if let Some(field) = self.reading.take() {
                    self.open.push(Field {
                        start: self.b.out.cells.len(),
                        ..field
                    });
                }
            }
            return;
        }
        match tok {
            Tok::Start(n, a) => {
                if n == "memo" {
                    self.memo = Some(Memo {
                        id: val(&a, "id").unwrap_or_default().to_string(),
                        author: val(&a, "author").unwrap_or_default().to_string(),
                        date: any_val(&a, &["createDateTime", "date"])
                            .unwrap_or_default()
                            .to_string(),
                        text: String::new(),
                    });
                    self.memo_depth = 1;
                    return;
                }
                if left_out(&n).is_some() {
                    self.skipping = 1;
                    // A control may open a 메모 field right here.
                    if n == "ctrl" {
                        self.control(Tok::Start(n, a));
                    }
                    return;
                }
                match n.as_str() {
                    "p" => {
                        self.depth_p += 1;
                        if self.depth_p == 1 {
                            let style = val(&a, "styleIDRef")
                                .and_then(|s| self.styles.get(s))
                                .copied();
                            let heading = match style {
                                Some(StyleKind::Heading(_)) | Some(StyleKind::Toc) => true,
                                None => val(&a, "paraPrIDRef")
                                    .is_some_and(|p| self.outlines.contains_key(p)),
                            };
                            self.para = Some(heading);
                            self.b.begin();
                        }
                    }
                    "run" => {
                        self.run = val(&a, "charPrIDRef")
                            .and_then(|c| self.chars.get(c))
                            .copied()
                            .unwrap_or_default();
                    }
                    "t" => self.in_t = true,
                    _ => {}
                }
            }
            Tok::Empty(n, a) => {
                if left_out(&n).is_some() {
                    return;
                }
                match n.as_str() {
                    "insertBegin" => {
                        let by = self.who(&a);
                        self.tracked = Some(Tracked {
                            edit: Edit::Insert,
                            by,
                        });
                    }
                    "deleteBegin" => {
                        let by = self.who(&a);
                        self.tracked = Some(Tracked {
                            edit: Edit::Delete,
                            by,
                        });
                    }
                    "insertEnd" | "deleteEnd" => self.tracked = None,
                    "markpenBegin" => self.shade = true,
                    "markpenEnd" => self.shade = false,
                    "lineBreak" => self.text("\n"),
                    "tab" => self.text(" "),
                    "nbSpace" => self.text("\u{A0}"),
                    "fwSpace" => self.text("\u{2002}"),
                    "hyphen" => self.text("-"),
                    _ => {}
                }
            }
            Tok::End(n) => match n.as_str() {
                "p" => {
                    if self.depth_p == 1
                        && let Some(heading) = self.para.take()
                    {
                        let tracked = self.tracked;
                        self.b.end(heading, tracked);
                    }
                    self.depth_p = self.depth_p.saturating_sub(1);
                }
                "t" => self.in_t = false,
                _ => {}
            },
            Tok::Text(s) => {
                if self.in_t {
                    self.text(&s);
                }
            }
        }
    }

    /// Joins fields with their memos and hands out the notes.
    fn notes(&mut self) -> Vec<FileNote> {
        let mut fields = std::mem::take(&mut self.done);
        // Fields never closed run to the end.
        let end = self.b.out.cells.len();
        for mut f in std::mem::take(&mut self.open) {
            f.end = Some(end);
            fields.push(f);
        }
        fields.sort_by_key(|f| f.start);
        let mut memos: Vec<Option<Memo>> = std::mem::take(&mut self.memos)
            .into_iter()
            .map(Some)
            .collect();
        let mut notes = Vec::new();
        let mut unnamed = Vec::new();
        for f in fields {
            let named = memos.iter().position(|m| {
                m.as_ref().is_some_and(|m| {
                    !m.id.is_empty() && (m.id == f.id || f.params.iter().any(|(_, v)| *v == m.id))
                })
            });
            match named {
                Some(i) => {
                    let memo = memos[i].take();
                    notes.push((f, memo));
                }
                None => {
                    unnamed.push(notes.len());
                    notes.push((f, None));
                }
            }
        }
        // Fields that name no memo take the memos left, in order.
        for i in unnamed {
            if notes[i].0.text.trim().is_empty()
                && let Some(m) = memos
                    .iter_mut()
                    .find(|m| m.is_some())
                    .and_then(Option::take)
            {
                notes[i].1 = Some(m);
            }
        }
        let mut out: Vec<FileNote> = notes
            .into_iter()
            .map(|(f, m)| {
                let m = m.unwrap_or_default();
                let text = if f.text.trim().is_empty() {
                    m.text
                } else {
                    f.text.clone()
                };
                FileNote {
                    text: text.trim().to_string(),
                    author: f
                        .param(&["Author", "author", "Writer"])
                        .map(str::to_string)
                        .unwrap_or(m.author),
                    date: f
                        .param(&["CreateDateTime", "Date", "date", "CreateTime"])
                        .map(str::to_string)
                        .unwrap_or(m.date),
                    on: Some((f.start, f.end.unwrap_or(f.start))),
                }
            })
            .filter(|n| !n.text.is_empty())
            .collect();
        // Memos no field points at: notes on no particular text.
        out.extend(
            memos
                .into_iter()
                .flatten()
                .filter(|m| !m.text.trim().is_empty())
                .map(|m| FileNote {
                    text: m.text.trim().to_string(),
                    author: m.author,
                    date: m.date,
                    on: None,
                }),
        );
        out
    }
}

pub(super) fn read(bytes: &[u8], scene_mark: &str) -> Result<Marked, String> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|_| BROKEN.to_string())?;
    let header = read_part(&mut archive, "Contents/header.xml")
        .ok_or_else(|| BROKEN.to_string())?
        .map_err(|_| BROKEN.to_string())?;
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
    let changes = track_changes(&header);
    let mut r = Reader {
        b: Builder::new(scene_mark),
        chars: char_shapes(&header),
        outlines: outline_shapes(&header),
        styles: styles(&header),
        changes: &changes,
        who_of: HashMap::new(),
        run: Run::default(),
        shade: false,
        tracked: None,
        para: None,
        depth_p: 0,
        in_t: false,
        skipping: 0,
        reading: None,
        param: None,
        open: Vec::new(),
        done: Vec::new(),
        memo: None,
        memo_depth: 0,
        memos: Vec::new(),
    };
    for (_, name) in sections {
        let xml = read_part(&mut archive, &name)
            .ok_or_else(|| BROKEN.to_string())?
            .map_err(|_| BROKEN.to_string())?;
        walk(&xml, |tok| r.step(tok)).map_err(|_| BROKEN.to_string())?;
    }
    let notes = r.notes();
    r.b.out.notes = notes;
    Ok(r.b.finish())
}
