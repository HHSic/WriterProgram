//! What a 한글 file says about its pages: paper, margins, 머리말, 꼬리말 and
//! page numbers. The import can make the 원고 서식 follow it, when the writer
//! asks; the body never gets these lines as text.

use serde::Serialize;

use crate::format::{
    HeadAlign, HeadContent, ManuscriptFormat, Margins, PAPERS, Paper, RunningFoot, RunningHead,
};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageSetup {
    pub paper: Paper,
    pub margins: Margins,
    pub header: RunningHead,
    /// Where the page number sits; `None` without page numbers.
    pub page_numbers: Option<HeadAlign>,
    pub footer: RunningFoot,
    /// The same in a sentence, for the last step of the import.
    pub summary: String,
}

/// A 머리말 or 꼬리말 as found in the file.
#[derive(Debug, Clone, Default)]
pub(super) struct Line {
    /// BOTH, EVEN or ODD.
    pub pages: String,
    pub text: String,
    pub align: Option<HeadAlign>,
    /// Holds the page number.
    pub page_number: bool,
}

/// Paper size and margins in mm, as the file has them.
#[derive(Debug, Clone, Default)]
pub(super) struct Found {
    pub width: f64,
    pub height: f64,
    pub margins: Option<Margins>,
    pub heads: Vec<Line>,
    pub foots: Vec<Line>,
    /// From a page number control.
    pub page_number: Option<HeadAlign>,
    /// The 머리말 is hidden on some pages (chapter openings, in books).
    pub hides_head: bool,
}

fn round(mm: f64) -> f64 {
    (mm * 10.0).round() / 10.0
}

fn paper(width: f64, height: f64) -> Paper {
    let (w, h) = (round(width), round(height));
    let kind = PAPERS
        .iter()
        .find(|(_, _, pw, ph)| (pw - w).abs() < 1.0 && (ph - h).abs() < 1.0)
        .map_or("custom", |(k, ..)| *k);
    Paper {
        kind: kind.into(),
        width_mm: w,
        height_mm: h,
    }
}

/// One place from the even and odd pages' places.
fn place(even: Option<HeadAlign>, odd: Option<HeadAlign>) -> HeadAlign {
    match (even, odd) {
        (Some(HeadAlign::Left), Some(HeadAlign::Right)) => HeadAlign::Outside,
        (_, Some(a)) | (Some(a), None) => a,
        (None, None) => HeadAlign::Center,
    }
}

fn distinct(lines: &[&Line]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in lines {
        let t = line.text.trim();
        if !t.is_empty() && !out.iter().any(|o| o == t) {
            out.push(t.to_string());
        }
    }
    out
}

fn split(lines: &[Line]) -> (Vec<&Line>, Vec<&Line>) {
    let even = lines.iter().filter(|l| l.pages == "EVEN").collect();
    let odd = lines.iter().filter(|l| l.pages != "EVEN").collect();
    (even, odd)
}

/// Marks around a page number ("- 3 -", "(3)", "3 |") that are no 꼬리말.
fn bare(text: &str) -> &str {
    text.trim_matches(|c: char| c.is_whitespace() || "-–—()[]|·.".contains(c))
}

fn align_word(align: HeadAlign) -> &'static str {
    match align {
        HeadAlign::Left => "왼쪽",
        HeadAlign::Center => "가운데",
        HeadAlign::Right => "오른쪽",
        HeadAlign::Outside => "바깥쪽",
    }
}

fn mm(v: f64) -> String {
    let s = format!("{v:.1}");
    s.strip_suffix(".0").map(str::to_string).unwrap_or(s)
}

impl Found {
    /// What the pages come to, or `None` when the file gave no paper.
    pub(super) fn setup(self) -> Option<PageSetup> {
        if self.width <= 0.0 || self.height <= 0.0 {
            return None;
        }
        let paper = paper(self.width, self.height);
        let m = self.margins.clone()?;
        let margins = Margins {
            top: round(m.top),
            bottom: round(m.bottom),
            inside: round(m.inside),
            outside: round(m.outside),
            header: round(m.header),
            footer: round(m.footer),
        };

        // 머리말: one text throughout, or the chapter's (a text per chapter),
        // with the work's title on the other side as in books.
        let (even, odd) = split(&self.heads);
        let (even_texts, odd_texts) = (distinct(&even), distinct(&odd));
        let content = if odd_texts.len() > 1 {
            if even_texts.len() == 1 {
                HeadContent::TitleChapter
            } else {
                HeadContent::Chapter
            }
        } else if odd_texts.is_empty() && even_texts.is_empty() {
            HeadContent::None
        } else {
            HeadContent::Custom
        };
        let header = RunningHead {
            content,
            text: if content == HeadContent::Custom {
                odd_texts
                    .first()
                    .or(even_texts.first())
                    .cloned()
                    .unwrap_or_default()
            } else {
                String::new()
            },
            align: place(
                even.first().and_then(|l| l.align),
                odd.first().and_then(|l| l.align),
            ),
            skip_chapter_first: self.hides_head,
        };

        // 꼬리말, and the page number when it is written inside one.
        let (even, odd) = split(&self.foots);
        let mut page_numbers = self.page_number;
        let numbered = |lines: &[&Line]| {
            lines
                .iter()
                .find(|l| l.page_number)
                .and_then(|l| l.align.or(Some(HeadAlign::Center)))
        };
        if page_numbers.is_none() && self.foots.iter().any(|l| l.page_number) {
            page_numbers = Some(place(numbered(&even), numbered(&odd)));
        }
        let texts: Vec<Line> = self
            .foots
            .iter()
            .map(|l| Line {
                text: bare(&l.text).to_string(),
                ..l.clone()
            })
            .collect();
        let (even, odd) = split(&texts);
        let text = distinct(&odd)
            .into_iter()
            .next()
            .or_else(|| distinct(&even).into_iter().next())
            .unwrap_or_default();
        let mut footer = if text.is_empty() {
            RunningFoot::default()
        } else {
            RunningFoot {
                text,
                align: place(
                    even.iter()
                        .find(|l| !l.text.is_empty())
                        .and_then(|l| l.align),
                    odd.iter()
                        .find(|l| !l.text.is_empty())
                        .and_then(|l| l.align),
                ),
            }
        };
        // 한글 can put both in one spot; here each needs its own.
        if let Some(number) = page_numbers
            && footer.is_on()
            && footer.align.meets(number)
        {
            footer.align = [HeadAlign::Left, HeadAlign::Right, HeadAlign::Center]
                .into_iter()
                .find(|a| !a.meets(number))
                .unwrap_or(HeadAlign::Center);
        }

        let summary = summary(&paper, &margins, &header, page_numbers, &footer);
        Some(PageSetup {
            paper,
            margins,
            header,
            page_numbers,
            footer,
            summary,
        })
    }
}

fn summary(
    paper: &Paper,
    m: &Margins,
    header: &RunningHead,
    page_numbers: Option<HeadAlign>,
    footer: &RunningFoot,
) -> String {
    let paper_name = PAPERS.iter().find(|(k, ..)| *k == paper.kind).map_or_else(
        || format!("{}×{}mm", mm(paper.width_mm), mm(paper.height_mm)),
        |(_, name, ..)| (*name).to_string(),
    );
    let margins = if (m.inside - m.outside).abs() < 0.05 {
        format!(
            "위 {} · 아래 {} · 양옆 {}mm",
            mm(m.top),
            mm(m.bottom),
            mm(m.inside)
        )
    } else {
        format!(
            "위 {} · 아래 {} · 안쪽 {} · 바깥쪽 {}mm",
            mm(m.top),
            mm(m.bottom),
            mm(m.inside),
            mm(m.outside)
        )
    };
    let head = match header.content {
        HeadContent::None => "머리말은 없음".to_string(),
        HeadContent::Chapter => format!("머리말은 장 제목({})", align_word(header.align)),
        HeadContent::TitleChapter => "머리말은 책처럼 작품 제목과 장 제목".to_string(),
        _ => format!("머리말은 “{}”({})", header.text, align_word(header.align)),
    };
    let number = match page_numbers {
        Some(a) => format!("쪽 번호는 아래 {}", align_word(a)),
        None => "쪽 번호는 없음".to_string(),
    };
    let foot = if footer.is_on() {
        format!("꼬리말은 “{}”({})", footer.text, align_word(footer.align))
    } else {
        "꼬리말은 없음".to_string()
    };
    format!("용지 {paper_name}, 여백 {margins}. {head}, {number}, {foot}.")
}

impl PageSetup {
    /// Makes `format` follow the file's pages; the type and spacing stay.
    pub fn apply(&self, format: &mut ManuscriptFormat) {
        format.paper = self.paper.clone();
        format.margins = self.margins.clone();
        format.header = self.header.clone();
        format.page_numbers = self.page_numbers.is_some();
        if let Some(align) = self.page_numbers {
            format.page_number_align = align;
        }
        format.footer = self.footer.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(pages: &str, text: &str, align: HeadAlign, page_number: bool) -> Line {
        Line {
            pages: pages.into(),
            text: text.into(),
            align: Some(align),
            page_number,
        }
    }

    fn found() -> Found {
        Found {
            width: 210.0,
            height: 297.0,
            margins: Some(Margins {
                top: 20.0,
                bottom: 15.0,
                inside: 30.0,
                outside: 30.0,
                header: 15.0,
                footer: 15.0,
            }),
            ..Found::default()
        }
    }

    #[test]
    fn book_heads_and_a_numbered_footer() {
        let mut f = found();
        f.heads = vec![
            line("EVEN", "달빛 서점", HeadAlign::Left, false),
            line("ODD", "1장 문 닫는 시간", HeadAlign::Right, false),
            line("ODD", "2장 빗소리", HeadAlign::Right, false),
        ];
        f.hides_head = true;
        // "- 3 -" in the middle, written with the page number inside.
        f.foots = vec![line("BOTH", "-  -", HeadAlign::Center, true)];
        let s = f.setup().unwrap();
        assert_eq!(s.paper.kind, "a4");
        assert_eq!(s.header.content, HeadContent::TitleChapter);
        assert_eq!(s.header.align, HeadAlign::Outside);
        assert!(s.header.skip_chapter_first);
        assert_eq!(s.page_numbers, Some(HeadAlign::Center));
        assert_eq!(s.footer, RunningFoot::default());
        assert_eq!(
            s.summary,
            "용지 A4, 여백 위 20 · 아래 15 · 양옆 30mm. 머리말은 책처럼 작품 제목과 장 제목, 쪽 번호는 아래 가운데, 꼬리말은 없음."
        );
    }

    #[test]
    fn footer_text_moves_off_the_page_number() {
        let mut f = found();
        f.width = 152.0;
        f.height = 225.0;
        f.heads = vec![line("BOTH", "투고 원고", HeadAlign::Center, false)];
        f.foots = vec![line("BOTH", "달빛 서점 - ", HeadAlign::Center, false)];
        f.page_number = Some(HeadAlign::Center);
        let s = f.setup().unwrap();
        assert_eq!(s.paper.kind, "shinguk");
        assert_eq!(s.header.content, HeadContent::Custom);
        assert_eq!(s.header.text, "투고 원고");
        assert_eq!(s.footer.text, "달빛 서점");
        assert_eq!(s.footer.align, HeadAlign::Left);

        let mut format = crate::format::default_for(crate::project::ProjectKind::Webnovel);
        s.apply(&mut format);
        assert!(format.has_paper() && format.page_numbers);
        format.validate().unwrap();
    }

    #[test]
    fn no_paper_no_setup() {
        assert!(Found::default().setup().is_none());
    }
}
