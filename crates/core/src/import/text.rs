//! Reading txt and md files.

use std::sync::LazyLock;

use encoding_rs::{EUC_KR, UTF_16BE, UTF_16LE};
use regex::Regex;

use super::{ImportOptions, Item, LineMode, Raw, SkipKind};
use crate::markup::{Block, Inline, parse_body};

/// A line that is a scene break: `***`, `---`, `* * *`, `___`, `===`, ◆, ◇.
static SCENE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:(?:\*\s*){3,}|(?:-\s*){3,}|(?:_\s*){3,}|={3,}|[◆◇]{1,3})\s*$").unwrap()
});
static HEADING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s{0,3}(#{1,6})\s+(.*?)(?:\s+#+)?\s*$").unwrap());
static TABLE_ROW: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*\|.*\|\s*$").unwrap());
static IMAGE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"!\[[^\]]*\]\([^)]*\)|<img\b[^>]*>").unwrap());
static LINK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[([^\]]+)\]\([^)]*\)").unwrap());
static MARKUP: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\*\*|\*|~~|`").unwrap());

/// True for a paragraph or line that is only a scene break mark.
pub(super) fn is_scene(text: &str) -> bool {
    SCENE.is_match(text)
}

fn decode(bytes: &[u8], wanted: Option<&str>) -> (String, &'static str) {
    let (bytes, bom) = match bytes {
        [0xEF, 0xBB, 0xBF, rest @ ..] => (rest, true),
        _ => (bytes, false),
    };
    let text = match wanted.map(str::to_lowercase).as_deref() {
        Some("euc-kr" | "cp949" | "ms949") => (
            EUC_KR.decode_without_bom_handling(bytes).0.into_owned(),
            "euc-kr",
        ),
        Some("utf-8" | "utf8") => (String::from_utf8_lossy(bytes).into_owned(), "utf-8"),
        _ => match bytes {
            [0xFF, 0xFE, rest @ ..] => (
                UTF_16LE.decode_without_bom_handling(rest).0.into_owned(),
                "utf-16",
            ),
            [0xFE, 0xFF, rest @ ..] => (
                UTF_16BE.decode_without_bom_handling(rest).0.into_owned(),
                "utf-16",
            ),
            _ => match std::str::from_utf8(bytes) {
                Ok(s) => (s.to_string(), "utf-8"),
                Err(_) if bom => (String::from_utf8_lossy(bytes).into_owned(), "utf-8"),
                Err(_) => (
                    EUC_KR.decode_without_bom_handling(bytes).0.into_owned(),
                    "euc-kr",
                ),
            },
        },
    };
    let (text, name) = text;
    let text = text
        .trim_start_matches('\u{feff}')
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    (text, name)
}

/// Every line a paragraph, or lines run together into paragraphs at empty lines.
fn resolve_mode(lines: &[&str], wanted: LineMode) -> LineMode {
    if wanted != LineMode::Auto {
        return wanted;
    }
    let mut blocks = 0;
    let mut longest = 0;
    let mut run = 0;
    for line in lines {
        if line.trim().is_empty() {
            run = 0;
        } else {
            if run == 0 {
                blocks += 1;
            }
            run += 1;
            longest = longest.max(run);
        }
    }
    let first = lines.iter().position(|l| !l.trim().is_empty());
    let last = lines.iter().rposition(|l| !l.trim().is_empty());
    let gap = match (first, last) {
        (Some(a), Some(b)) => lines[a..=b].iter().any(|l| l.trim().is_empty()),
        _ => false,
    };
    if gap && blocks > 0 && longest > 1 {
        LineMode::Blank
    } else {
        LineMode::Line
    }
}

fn trim_line(line: &str) -> &str {
    line.trim_matches(|c: char| c.is_whitespace())
}

struct Reader {
    md: bool,
    mode: LineMode,
    items: Vec<Item>,
    group: Vec<String>,
    /// What was left out of the lines in `group`, put after their paragraph.
    pending: Vec<(SkipKind, u32)>,
    in_table: bool,
}

impl Reader {
    fn flush(&mut self) {
        let group = std::mem::take(&mut self.group);
        let paragraphs: Vec<Vec<String>> = match self.mode {
            LineMode::Blank => vec![group],
            _ => group.into_iter().map(|l| vec![l]).collect(),
        };
        for lines in paragraphs {
            if let Some(inlines) = self.paragraph(&lines) {
                self.items.push(Item::Para(inlines));
            }
        }
        for (kind, n) in std::mem::take(&mut self.pending) {
            self.items.push(Item::Skip(kind, n));
        }
    }

    fn paragraph(&self, lines: &[String]) -> Option<Vec<Inline>> {
        if lines.is_empty() {
            return None;
        }
        let inlines = if self.md {
            match parse_body(&lines.join("\n")).into_iter().next()? {
                Block::Paragraph { content, .. } => content,
                Block::SceneBreak {} => return None,
            }
        } else {
            let mut out = Vec::new();
            for (i, line) in lines.iter().enumerate() {
                if i > 0 {
                    out.push(Inline::HardBreak {});
                }
                out.push(Inline::Text {
                    text: line.clone(),
                    marks: vec![],
                });
            }
            out
        };
        let text: String = inlines
            .iter()
            .filter_map(|i| match i {
                Inline::Text { text, .. } => Some(text.as_str()),
                Inline::HardBreak {} => None,
            })
            .collect();
        (!text.trim().is_empty()).then_some(inlines)
    }

    fn line(&mut self, raw: &str) {
        let line = trim_line(raw);
        if line.is_empty() {
            self.flush();
            self.in_table = false;
            return;
        }
        if is_scene(line) {
            self.flush();
            self.in_table = false;
            self.items.push(Item::Scene);
            return;
        }
        if !self.md {
            self.group.push(line.to_string());
            return;
        }
        if TABLE_ROW.is_match(line) {
            self.flush();
            if !self.in_table {
                self.items.push(Item::Skip(SkipKind::Table, 1));
                self.in_table = true;
            }
            return;
        }
        self.in_table = false;
        if let Some(c) = HEADING.captures(line) {
            self.flush();
            let linked = LINK.replace_all(&c[2], "$1");
            let text = MARKUP.replace_all(&linked, "");
            self.items.push(Item::Heading {
                level: c[1].len() as u8,
                text: text.trim().to_string(),
            });
            return;
        }
        let quoted = line.strip_prefix('>').map_or(line, |l| trim_line(l));
        let images = IMAGE.find_iter(quoted).count() as u32;
        let no_images = IMAGE.replace_all(quoted, "");
        let linked = LINK.replace_all(&no_images, "$1");
        let stripped = trim_line(&linked);
        if images > 0 {
            self.pending.push((SkipKind::Image, images));
        }
        if stripped.is_empty() {
            if images > 0 && self.group.is_empty() {
                self.flush();
            }
            return;
        }
        self.group.push(stripped.to_string());
    }
}

/// Front matter at the top of an md file: the number of lines to skip.
fn front_matter(lines: &[&str]) -> usize {
    let first = lines.iter().position(|l| !l.trim().is_empty());
    match first {
        Some(0) if lines[0].trim_end() == "---" => lines
            .iter()
            .enumerate()
            .skip(1)
            .take(40)
            .find(|(_, l)| matches!(l.trim_end(), "---" | "..."))
            .map_or(0, |(i, _)| i + 1),
        _ => 0,
    }
}

pub(super) fn read(bytes: &[u8], opts: &ImportOptions, md: bool) -> Result<Raw, String> {
    let (text, encoding) = decode(bytes, opts.encoding.as_deref());
    let mut lines: Vec<&str> = text.split('\n').collect();
    if md {
        lines.drain(..front_matter(&lines));
        lines.retain(|l| !l.trim_start().starts_with("```"));
    }
    let mut reader = Reader {
        md,
        mode: resolve_mode(&lines, opts.line_mode),
        items: Vec::new(),
        group: Vec::new(),
        pending: Vec::new(),
        in_table: false,
    };
    for line in &lines {
        reader.line(line);
    }
    reader.flush();
    Ok(Raw {
        items: reader.items,
        encoding: Some(encoding.into()),
        page: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paras(raw: &Raw) -> Vec<String> {
        raw.items
            .iter()
            .map(|i| match i {
                Item::Para(inl) => crate::markup::inline_text(inl),
                Item::Scene => "<scene>".into(),
                Item::Heading { level, text } => format!("<h{level}>{text}"),
                Item::Skip(kind, n) => format!("<skip {kind:?} {n}>"),
            })
            .collect()
    }

    fn txt(src: &str) -> Vec<String> {
        paras(&read(src.as_bytes(), &ImportOptions::default(), false).unwrap())
    }

    #[test]
    fn every_line_is_a_paragraph_without_empty_lines() {
        assert_eq!(txt("가\n나\n다"), ["가", "나", "다"]);
    }

    #[test]
    fn empty_lines_between_single_lines_change_nothing() {
        assert_eq!(txt("가\n\n나\n\n다"), ["가", "나", "다"]);
    }

    #[test]
    fn lines_in_a_block_stay_line_breaks() {
        assert_eq!(txt("가\n나\n\n다"), ["가\n나", "다"]);
    }

    #[test]
    fn scene_breaks_and_indent() {
        assert_eq!(
            txt("　　가\n* * *\n나\n---\n다"),
            ["가", "<scene>", "나", "<scene>", "다"]
        );
    }

    #[test]
    fn euc_kr_is_found() {
        let (bytes, _, _) = EUC_KR.encode("안녕하세요\n문 닫는 시간");
        let raw = read(&bytes, &ImportOptions::default(), false).unwrap();
        assert_eq!(raw.encoding.as_deref(), Some("euc-kr"));
        assert_eq!(paras(&raw), ["안녕하세요", "문 닫는 시간"]);
    }

    #[test]
    fn utf8_with_bom_and_crlf() {
        let raw = read(
            "\u{feff}가\r\n나".as_bytes(),
            &ImportOptions::default(),
            false,
        )
        .unwrap();
        assert_eq!(raw.encoding.as_deref(), Some("utf-8"));
        assert_eq!(paras(&raw), ["가", "나"]);
    }

    #[test]
    fn markdown_headings_marks_tables_and_images() {
        let src = "---\ntitle: x\n---\n# 1화 문 닫는 시간\n\n**서하**는 문을 닫았다.\n![그림](a.png)\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n끝.";
        let raw = read(src.as_bytes(), &ImportOptions::default(), true).unwrap();
        assert_eq!(
            paras(&raw),
            [
                "<h1>1화 문 닫는 시간",
                "서하는 문을 닫았다.",
                "<skip Image 1>",
                "<skip Table 1>",
                "끝."
            ]
        );
        let bold = raw.items.iter().any(|i| {
            matches!(i, Item::Para(inl) if inl.iter().any(|x| matches!(x, Inline::Text { marks, .. } if !marks.is_empty())))
        });
        assert!(bold);
    }
}
