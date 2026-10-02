//! The certificate as one self-contained HTML page (§5): readable in any
//! browser, printable to PDF, no outside files, fonts or scripts.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use chrono::{DateTime, Datelike, Duration, FixedOffset, NaiveDate, Utc};

use super::excerpt::{Excerpt, Piece};
use super::facts::{Day, Facts, NOT_ABOUT_AI, date_text, noun, num, ranges};
use super::verify::{Level, Verdict};
use super::{BUNDLE_NAME, Options};

pub(crate) struct Page<'a> {
    pub facts: &'a Facts,
    pub options: &'a Options,
    pub summary: &'a str,
    pub verdict: &'a Verdict,
    pub excerpts: &'a [Excerpt],
    pub made: DateTime<Utc>,
    pub tz: FixedOffset,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

pub(crate) fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

const STYLE: &str = r#"
:root { color-scheme: light; --ink: #1b1b1a; --ink-2: #52514e; --ink-3: #85847f; --line: #dcdad3;
  --paper: #ffffff; --tint: #f6f5f1; --insert: #2a78d6; --delete: #eb6834; --ok: #1d7a46; --warn: #a15c00; --fail: #b3261e; }
* { box-sizing: border-box; }
html { background: var(--paper); }
body { margin: 0 auto; max-width: 820px; padding: 40px 32px 64px; color: var(--ink); background: var(--paper);
  font-family: "Noto Serif KR", "Nanum Myeongjo", "Batang", "바탕", "AppleMyungjo", serif;
  font-size: 15px; line-height: 1.75; word-break: keep-all; overflow-wrap: anywhere; }
h1, h2, h3, th, .meta, .legend, .num, .small, figcaption, .mark {
  font-family: "Pretendard", "Noto Sans KR", "Malgun Gothic", "맑은 고딕", "Apple SD Gothic Neo", sans-serif; }
h1 { font-size: 28px; letter-spacing: 0.02em; margin: 0 0 4px; }
.work { font-size: 19px; margin: 0 0 12px; }
.meta { color: var(--ink-2); font-size: 13px; margin: 0; }
h2 { font-size: 17px; margin: 36px 0 10px; padding-bottom: 6px; border-bottom: 1px solid var(--line); }
h3 { font-size: 15px; margin: 20px 0 6px; }
.summary { margin: 24px 0 8px; padding: 18px 20px; background: var(--tint); border-left: 4px solid var(--ink); font-size: 16px; }
p { margin: 8px 0; }
.small { font-size: 12.5px; color: var(--ink-2); }
table { width: 100%; border-collapse: collapse; font-size: 13px; margin: 8px 0; }
th { text-align: left; font-weight: 600; color: var(--ink-2); border-bottom: 1px solid var(--ink-3); padding: 6px 6px; }
td { border-bottom: 1px solid var(--line); padding: 6px 6px; vertical-align: top; }
td.num, th.num { text-align: right; font-variant-numeric: tabular-nums; }
td.num { white-space: nowrap; }
tr { break-inside: avoid; }
.wide { overflow-x: auto; }
code, .hash { font-family: "D2Coding", "Consolas", "Menlo", monospace; font-size: 12px; overflow-wrap: anywhere; }
.legend { display: flex; gap: 16px; font-size: 12.5px; color: var(--ink-2); margin: 4px 0; }
.legend i { display: inline-block; width: 10px; height: 10px; border-radius: 2px; margin-right: 6px; vertical-align: -1px; }
svg { width: 100%; height: auto; display: block; }
svg text { font-family: "Pretendard", "Noto Sans KR", "Malgun Gothic", sans-serif; font-size: 11px; fill: var(--ink-2); }
svg .hit { fill: transparent; }
svg .hit:hover { fill: rgba(0,0,0,0.05); }
details { margin: 8px 0; }
summary { cursor: pointer; font-size: 13px; color: var(--ink-2); }
.excerpt { border: 1px solid var(--line); border-radius: 6px; padding: 10px 14px; margin: 8px 0; white-space: pre-wrap; }
del { background: #fde3d8; color: #8a2f0c; text-decoration: line-through; }
ins { background: #dbe9fa; color: #123f73; text-decoration: none; }
ul.checks { list-style: none; padding: 0; margin: 8px 0; font-size: 13.5px; }
ul.checks li { padding: 3px 0 3px 52px; text-indent: -52px; }
.mark { display: inline-block; width: 46px; font-size: 12px; font-weight: 600; text-indent: 0; }
.ok .mark { color: var(--ok); } .warn .mark { color: var(--warn); } .fail .mark { color: var(--fail); }
ol li { margin: 4px 0; }
footer { margin-top: 40px; padding-top: 12px; border-top: 1px solid var(--line); font-size: 12.5px; color: var(--ink-2); }
@page { size: A4; margin: 16mm 14mm; }
@media print { body { padding: 0; max-width: none; font-size: 12.5px; } h2 { break-after: avoid; } .summary { break-inside: avoid; } }
@media (max-width: 560px) { body { padding: 24px 16px 48px; } table { font-size: 12px; } }
"#;

impl Page<'_> {
    fn when(&self, t: DateTime<Utc>) -> String {
        let local = t.with_timezone(&self.tz);
        if self.options.dates_only {
            date_text(local.date_naive())
        } else {
            format!(
                "{} {}",
                date_text(local.date_naive()),
                local.format("%H:%M")
            )
        }
    }

    fn label(&self, number: usize) -> String {
        format!("{number}{}", noun(self.facts.kind))
    }

    pub fn render(&self) -> String {
        let f = self.facts;
        let mut h = String::new();
        let _ = write!(
            h,
            "<!doctype html>\n<html lang=\"ko\">\n<head>\n<meta charset=\"utf-8\">\n\
             <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
             <title>창작 과정 증명서 - {}</title>\n<style>{STYLE}</style>\n</head>\n<body>\n",
            esc(&f.title)
        );
        self.head(&mut h);
        let _ = writeln!(h, "<p class=\"summary\">{}</p>", esc(self.summary));
        self.amounts(&mut h);
        self.chapters(&mut h);
        if !self.excerpts.is_empty() {
            self.excerpts(&mut h);
        }
        self.outside(&mut h);
        self.anchors(&mut h);
        self.checks(&mut h);
        self.how_to_verify(&mut h);
        let _ = write!(
            h,
            "<footer>{} 이 증명서와 함께 낸 <code>{BUNDLE_NAME}</code>으로 누구나 내용을 검증할 수 있습니다. WriterProgram에서 만듦.</footer>\n</body>\n</html>\n",
            esc(NOT_ABOUT_AI)
        );
        // Wide tables scroll on a phone instead of widening the page.
        h.replace("<table>", "<div class=\"wide\"><table>")
            .replace("</table>", "</table></div>")
    }

    fn head(&self, h: &mut String) {
        let f = self.facts;
        let _ = write!(
            h,
            "<h1>창작 과정 증명서</h1>\n<p class=\"work\">「{}」",
            esc(&f.title)
        );
        if !f.pen_name.trim().is_empty() {
            let _ = write!(h, " · {}", esc(f.pen_name.trim()));
        }
        h.push_str("</p>\n");
        let numbers: Vec<usize> = f.chapters.iter().map(|c| c.number).collect();
        let what = if self.options.docs.is_none() {
            format!("작품 전체 ({})", ranges(&numbers, noun(f.kind)))
        } else {
            ranges(&numbers, noun(f.kind))
        };
        let period = match (self.from, self.to) {
            (None, None) => "기록이 시작된 때부터".to_string(),
            (Some(a), None) => format!("{}부터", date_text(a)),
            (None, Some(b)) => format!("{}까지", date_text(b)),
            (Some(a), Some(b)) => format!("{} ~ {}", date_text(a), date_text(b)),
        };
        let devices = if f.totals.devices > 1 {
            format!(" · 기기 {}대의 기록", f.totals.devices)
        } else {
            String::new()
        };
        let _ = writeln!(
            h,
            "<p class=\"meta\">범위: {} · 기간: {}{} · 만든 날: {}</p>",
            esc(&what),
            esc(&period),
            devices,
            date_text(self.made.with_timezone(&self.tz).date_naive())
        );
    }

    fn amounts(&self, h: &mut String) {
        let f = self.facts;
        h.push_str("<h2>날짜별 분량</h2>\n");
        let (Some(first), Some(last)) = (f.totals.first, f.totals.last) else {
            h.push_str("<p>이 범위에는 날짜별 기록이 없습니다.</p>\n");
            return;
        };
        let weekly = (last - first).num_days() > 180;
        let buckets = buckets(&f.days, first, last, weekly);
        let _ = writeln!(
            h,
            "<p class=\"small\">날마다 넣은 글자는 위로, 지운 글자는 아래로 그렸습니다{}. 막대에 마우스를 올리면 수가 보입니다.</p>",
            if weekly {
                " (한 막대가 한 주)"
            } else {
                ""
            }
        );
        h.push_str("<div class=\"legend\"><span><i style=\"background:var(--insert)\"></i>넣은 글자</span><span><i style=\"background:var(--delete)\"></i>지운 글자</span></div>\n");
        h.push_str(&chart(&buckets, weekly));
        let _ = writeln!(
            h,
            "<p class=\"small\">모두 넣은 글자 {}자, 지운 글자 {}자. 쓰기 묶음 {}번, 저장 {}번.</p>",
            num(f.totals.inserted),
            num(f.totals.deleted),
            num(f.totals.sessions),
            num(f.totals.saves)
        );
        h.push_str("<details><summary>날짜별 표로 보기</summary>\n<table><thead><tr><th>날짜</th><th class=\"num\">넣은 글자</th><th class=\"num\">지운 글자</th><th class=\"num\">쓰기 묶음</th><th class=\"num\">저장</th></tr></thead><tbody>\n");
        for (date, d) in &f.days {
            let _ = writeln!(
                h,
                "<tr><td>{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td></tr>",
                date_text(*date),
                num(d.inserted),
                num(d.deleted),
                num(d.sessions),
                num(d.saves)
            );
        }
        h.push_str("</tbody></table></details>\n");
    }

    fn chapters(&self, h: &mut String) {
        let f = self.facts;
        h.push_str("<h2>회차별 기록</h2>\n<table><thead><tr><th>회차</th><th>처음 쓴 날 ~ 마지막으로 고친 날</th><th class=\"num\">지금 글자 수</th><th class=\"num\">쓰기 묶음</th><th class=\"num\">넣은·지운 글자</th><th class=\"num\">초고 대비 고친 비율</th><th>바깥 붙여넣기</th></tr></thead><tbody>\n");
        for row in &f.rows {
            let title = if row.title.trim().is_empty() {
                String::new()
            } else {
                format!(" {}", esc(row.title.trim()))
            };
            let first = row.first.map(date_text).unwrap_or_else(|| "—".into());
            let last = row.last_edit.map(date_text).unwrap_or_else(|| "—".into());
            let changed = row
                .changed
                .map(|c| format!("{}%", (c * 100.0).round() as u64))
                .unwrap_or_else(|| "—".into());
            let pastes = if row.outside_pastes == 0 {
                "없음".to_string()
            } else {
                format!(
                    "{}번, {}자",
                    num(row.outside_pastes),
                    num(row.outside_chars)
                )
            };
            let imported = row
                .imported_from
                .as_ref()
                .map(|f| format!("<br><span class=\"small\">'{}'에서 가져옴</span>", esc(f)))
                .unwrap_or_default();
            let _ = writeln!(
                h,
                "<tr><td>{}{title}{imported}</td><td>{first} ~<br>{last}</td><td class=\"num\">{}</td><td class=\"num\">{}</td><td class=\"num\">{} · {}</td><td class=\"num\">{changed}</td><td>{pastes}</td></tr>",
                self.label(row.number),
                num(u64::from(row.chars)),
                num(row.sessions),
                num(row.inserted),
                num(row.deleted),
            );
        }
        h.push_str("</tbody></table>\n<p class=\"small\">고친 비율은 남아 있는 가장 오래된 기록(초고)과 지금 글을 글자 단위로 견주어, 달라진 글자의 비율을 낸 값입니다. 바깥 붙여넣기는 100자 이상을 이 앱 밖에서 가져와 붙여 넣은 것입니다.</p>\n");
    }

    fn excerpts(&self, h: &mut String) {
        h.push_str("<h2>고친 흐름</h2>\n<p class=\"small\">작가가 공개를 고른 회차만 싣습니다. <del>지운 글</del>과 <ins>새로 쓴 글</ins>을 표시했습니다.</p>\n");
        for ex in self.excerpts {
            let title = if ex.title.trim().is_empty() {
                String::new()
            } else {
                format!(" {}", esc(ex.title.trim()))
            };
            let _ = writeln!(h, "<h3>{}{title}</h3>", self.label(ex.number));
            let Some(draft) = &ex.draft else {
                h.push_str("<p>남아 있는 기록이 없어 견줄 초고가 없습니다.</p>\n");
                continue;
            };
            let stage = |s: &super::excerpt::Stage| {
                format!(
                    "{} ({}자)",
                    s.at.map(|t| self.when(t))
                        .unwrap_or_else(|| "때 모름".into()),
                    num(u64::from(s.chars))
                )
            };
            let _ = write!(h, "<p class=\"small\">초고: {}", stage(draft));
            if let Some(m) = &ex.middle {
                let _ = write!(h, " → 중간: {}", stage(m));
            }
            let _ = writeln!(h, " → 지금: {}자</p>", num(u64::from(ex.final_chars)));
            if ex.spots.is_empty() {
                h.push_str("<p>초고와 지금 글이 같습니다.</p>\n");
            }
            for spot in &ex.spots {
                h.push_str("<div class=\"excerpt\">");
                for piece in spot {
                    match piece {
                        Piece::Same(t) => h.push_str(&esc(t)),
                        Piece::Gone(t) => {
                            let _ = write!(h, "<del>{}</del>", esc(t));
                        }
                        Piece::New(t) => {
                            let _ = write!(h, "<ins>{}</ins>", esc(t));
                        }
                    }
                }
                h.push_str("</div>\n");
            }
            if ex.more > 0 {
                let _ = writeln!(
                    h,
                    "<p class=\"small\">이 밖에 {}곳을 더 고쳤습니다.</p>",
                    ex.more
                );
            }
        }
    }

    fn outside(&self, h: &mut String) {
        let f = self.facts;
        h.push_str("<h2>가져온 파일과 교정 주고받기</h2>\n");
        if f.imports.is_empty() && f.exchanges.is_empty() {
            h.push_str("<p>이 범위에는 가져온 파일도, 편집자와 주고받은 원고도 없습니다.</p>\n");
            return;
        }
        if !f.imports.is_empty() {
            h.push_str("<h3>가져온 파일</h3>\n<p class=\"small\">이 앱 밖(한글, Word 등)에서 쓴 원고를 가져온 기록입니다. 파일 지문으로 원래 파일과 같은지 확인할 수 있습니다.</p>\n<table><thead><tr><th>가져온 때</th><th>파일</th><th>만든 회차</th><th>파일 지문 (SHA-256)</th></tr></thead><tbody>\n");
            for i in &f.imports {
                let _ = writeln!(
                    h,
                    "<tr><td>{}</td><td>{}</td><td>{}</td><td class=\"hash\">{}</td></tr>",
                    self.when(i.time),
                    esc(&i.file),
                    if i.numbers.is_empty() {
                        "—".into()
                    } else {
                        ranges(&i.numbers, noun(f.kind))
                    },
                    esc(&i.hash)
                );
            }
            h.push_str("</tbody></table>\n");
        }
        if !f.exchanges.is_empty() {
            h.push_str("<h3>교정 주고받기</h3>\n<table><thead><tr><th>보낸 때</th><th>보낸 파일</th><th>보낸 회차와 지문</th><th>받은 교정본</th></tr></thead><tbody>\n");
            for x in &f.exchanges {
                let sent = x
                    .sent
                    .iter()
                    .map(|(n, fp)| {
                        format!("{} <span class=\"hash\">{}</span>", self.label(*n), esc(fp))
                    })
                    .collect::<Vec<_>>()
                    .join("<br>");
                let received = if x.received.is_empty() {
                    "—".to_string()
                } else {
                    x.received
                        .iter()
                        .map(|(t, name, fp)| {
                            format!(
                                "{} {} <span class=\"hash\">{}</span>",
                                self.when(*t),
                                esc(name),
                                esc(fp)
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("<br>")
                };
                let _ = writeln!(
                    h,
                    "<tr><td>{}</td><td>{}</td><td>{sent}</td><td>{received}</td></tr>",
                    self.when(x.time),
                    esc(&x.files.join(", "))
                );
            }
            h.push_str("</tbody></table>\n");
        }
    }

    fn anchors(&self, h: &mut String) {
        h.push_str("<h2>시각 인증</h2>\n");
        if self.verdict.anchors.is_empty() {
            h.push_str("<p>이 범위에는 시각 인증이 없습니다. 기록이 고쳐지지 않았는지는 확인할 수 있지만, 그 날짜에 있었음을 제3자가 보증하지는 않습니다.</p>\n");
            return;
        }
        h.push_str("<p class=\"small\">하루 한 번, 그날까지의 창작 일지와 회차 본문의 지문을 하나로 묶은 지문(32바이트)만 공개 시각 인증 기관(RFC 3161)에 보내 서명을 받았습니다. 원고는 보내지 않았고, 이 지문으로 원고를 되살릴 수 없습니다.</p>\n<table><thead><tr><th>인증 받은 때</th><th>묶은 지문</th><th>인증 기관</th></tr></thead><tbody>\n");
        for a in &self.verdict.anchors {
            let signed = a
                .signed
                .map(|t| self.when(t))
                .unwrap_or_else(|| "확인 안 됨".into());
            let tsas = a
                .stamps
                .iter()
                .map(|(label, s)| match s {
                    Ok(_) => esc(label),
                    Err(_) => format!("{} (확인 안 됨)", esc(label)),
                })
                .collect::<Vec<_>>()
                .join(", ");
            let _ = writeln!(
                h,
                "<tr><td>{signed}</td><td class=\"hash\">{}</td><td>{tsas}</td></tr>",
                esc(&a.root)
            );
        }
        h.push_str("</tbody></table>\n");
    }

    fn checks(&self, h: &mut String) {
        h.push_str("<h2>만들 때 확인한 결과</h2>\n<p class=\"small\">이 증명서와 함께 저장한 증명 자료를 아래 검증 방법과 같은 계산으로 확인한 결과입니다.</p>\n<ul class=\"checks\">\n");
        let v = self.verdict;
        let fails: Vec<&str> = v
            .findings
            .iter()
            .filter(|f| f.level == Level::Fail)
            .map(|f| f.text.as_str())
            .collect();
        let warns: Vec<&str> = v
            .findings
            .iter()
            .filter(|f| f.level == Level::Warn)
            .map(|f| f.text.as_str())
            .collect();
        let chains = v
            .findings
            .iter()
            .filter(|f| f.text.contains("창작 일지") && f.text.contains("이어져 있습니다"))
            .count();
        if chains > 0 {
            let _ = writeln!(
                h,
                "<li class=\"ok\"><span class=\"mark\">확인</span>기기 {chains}대의 창작 일지가 처음부터 끝까지 이어져 있습니다. 중간에 고치거나 뺀 항목이 없습니다.</li>"
            );
        }
        let stamps: usize = v.anchors.iter().map(|a| a.stamps.len()).sum();
        let good: usize = v
            .anchors
            .iter()
            .map(|a| a.stamps.iter().filter(|(_, s)| s.is_ok()).count())
            .sum();
        if stamps > 0 {
            let _ = writeln!(
                h,
                "<li class=\"{}\"><span class=\"mark\">{}</span>시각 인증 서명 {stamps}개 가운데 {good}개가 맞고, 서명한 지문이 창작 일지와 회차 지문으로 다시 계산한 값과 같습니다.</li>",
                if good == stamps { "ok" } else { "fail" },
                if good == stamps { "확인" } else { "실패" }
            );
        }
        for text in warns {
            let _ = writeln!(
                h,
                "<li class=\"warn\"><span class=\"mark\">주의</span>{}</li>",
                esc(text)
            );
        }
        for text in fails {
            let _ = writeln!(
                h,
                "<li class=\"fail\"><span class=\"mark\">실패</span>{}</li>",
                esc(text)
            );
        }
        h.push_str("</ul>\n");
    }

    fn how_to_verify(&self, h: &mut String) {
        let _ = write!(
            h,
            "<h2>검증 방법</h2>\n<p>이 증명서는 함께 저장된 <code>{BUNDLE_NAME}</code>(창작 일지, 시각 인증 응답, 지문 목록)로 누구나 무료로 검증할 수 있습니다. 증명 자료에는 원고 내용이 들어 있지 않습니다. 범위 밖의 일지 항목은 지문만 들어 있어, 내용은 가려도 사슬은 이어서 확인할 수 있습니다.</p>\n\
<ol>\n\
<li><b>기록이 고쳐지지 않았는지</b>: 창작 일지의 항목마다 바로 앞 항목의 지문(SHA-256)이 들어 있습니다. 중간 항목을 고치거나 빼면 그 뒤가 모두 이어지지 않습니다.</li>\n\
<li><b>그 날짜에 있었는지</b>: 시각 인증 응답은 공개 인증 기관(DigiCert, Sectigo, FreeTSA)이 서명한 RFC 3161 토큰입니다. 서명이 맞는지, 서명한 지문이 증명 자료의 지문 목록으로 다시 계산한 머클 루트(RFC 6962 방식)와 같은지 확인합니다. 인증서 발급자가 믿을 만한 기관인지는 검증하는 사람이 판단합니다.</li>\n\
<li><b>원고 파일이 기록과 같은지</b>: 원고 파일(작품 폴더의 <code>manuscript/&lt;id&gt;.md</code>, 또는 가져온 원래 파일)을 함께 넣으면 그 지문이 기록에 있는지 알려 줍니다.</li>\n\
</ol>\n\
<p>명령줄 검증 도구(오픈소스, WriterProgram 저장소의 <code>crates/core/examples/verify_proof.rs</code>):</p>\n\
<p><code>cargo run -p writer-core --example verify_proof -- {BUNDLE_NAME} [원고 파일 …]</code></p>\n\
<p class=\"small\">시각 인증 토큰 하나를 OpenSSL로 따로 확인할 수도 있습니다: 증명 자료의 <code>reply</code> 값(base64)을 파일로 풀어 <code>openssl ts -verify -digest &lt;지문&gt; -in 응답.tsr -CAfile &lt;인증 기관 루트 인증서&gt;</code>.</p>\n"
        );
    }
}

/// Daily (or weekly, from Monday) amounts from `first` to `last`, with the
/// days in between that have none.
fn buckets(
    days: &BTreeMap<NaiveDate, Day>,
    first: NaiveDate,
    last: NaiveDate,
    weekly: bool,
) -> Vec<(NaiveDate, Day)> {
    let start = |d: NaiveDate| {
        if weekly {
            d - Duration::days(i64::from(d.weekday().num_days_from_monday()))
        } else {
            d
        }
    };
    let step = Duration::days(if weekly { 7 } else { 1 });
    let mut out: Vec<(NaiveDate, Day)> = Vec::new();
    let mut d = start(first);
    while d <= last {
        out.push((d, Day::default()));
        d += step;
    }
    for (date, day) in days.range(first..=last) {
        let key = start(*date);
        if let Some((_, b)) = out.iter_mut().find(|(k, _)| *k == key) {
            b.inserted += day.inserted;
            b.deleted += day.deleted;
            b.sessions += day.sessions;
            b.saves += day.saves;
        }
    }
    out
}

/// Bars up for inserted, down for deleted, on one scale.
fn chart(buckets: &[(NaiveDate, Day)], weekly: bool) -> String {
    const W: f64 = 720.0;
    const H: f64 = 210.0;
    const LEFT: f64 = 54.0;
    const RIGHT: f64 = 6.0;
    const TOP: f64 = 12.0;
    const BOTTOM: f64 = 26.0;
    let up = buckets.iter().map(|(_, d)| d.inserted).max().unwrap_or(0);
    let down = buckets.iter().map(|(_, d)| d.deleted).max().unwrap_or(0);
    let total = (up + down).max(1) as f64;
    let plot = H - TOP - BOTTOM;
    let k = plot / total;
    let base = TOP + up as f64 * k;
    let slot = (W - LEFT - RIGHT) / buckets.len().max(1) as f64;
    let bar = if slot > 4.0 {
        (slot - 2.0).min(28.0)
    } else {
        slot.max(0.6)
    };
    let mut s = String::new();
    let _ = write!(
        s,
        "<figure style=\"margin:0\"><svg viewBox=\"0 0 {W} {H}\" role=\"img\" aria-label=\"날짜별 넣은 글자와 지운 글자\">"
    );
    let _ = write!(
        s,
        "<line x1=\"{LEFT}\" x2=\"{}\" y1=\"{base:.1}\" y2=\"{base:.1}\" stroke=\"#85847f\" stroke-width=\"1\"/>",
        W - RIGHT
    );
    let _ = write!(
        s,
        "<text x=\"{}\" y=\"{}\" text-anchor=\"end\">{}자</text>",
        LEFT - 6.0,
        TOP + 8.0,
        num(up)
    );
    if down > 0 {
        let _ = write!(
            s,
            "<text x=\"{}\" y=\"{}\" text-anchor=\"end\">−{}자</text>",
            LEFT - 6.0,
            H - BOTTOM,
            num(down)
        );
    }
    for (i, (date, d)) in buckets.iter().enumerate() {
        let x = LEFT + i as f64 * slot + (slot - bar) / 2.0;
        let label = if weekly {
            format!("{} 주", date_text(*date))
        } else {
            date_text(*date)
        };
        let _ = write!(
            s,
            "<g><title>{label}: 넣은 글자 {} · 지운 글자 {}</title>",
            num(d.inserted),
            num(d.deleted)
        );
        let _ = write!(
            s,
            "<rect class=\"hit\" x=\"{:.1}\" y=\"{TOP}\" width=\"{slot:.2}\" height=\"{plot:.1}\"/>",
            LEFT + i as f64 * slot
        );
        if d.inserted > 0 {
            let hgt = (d.inserted as f64 * k).max(1.0);
            let _ = write!(
                s,
                "<rect x=\"{x:.2}\" y=\"{:.2}\" width=\"{bar:.2}\" height=\"{hgt:.2}\" rx=\"{}\" fill=\"var(--insert)\"/>",
                base - hgt,
                if bar > 6.0 { 2 } else { 0 }
            );
        }
        if d.deleted > 0 {
            let hgt = (d.deleted as f64 * k).max(1.0);
            let _ = write!(
                s,
                "<rect x=\"{x:.2}\" y=\"{:.2}\" width=\"{bar:.2}\" height=\"{hgt:.2}\" rx=\"{}\" fill=\"var(--delete)\"/>",
                base + 1.0,
                if bar > 6.0 { 2 } else { 0 }
            );
        }
        s.push_str("</g>");
    }
    // Dates along the bottom: first, last and the start of each month.
    let y = H - 8.0;
    let mut last_label_x = f64::MIN;
    for (i, (date, _)) in buckets.iter().enumerate() {
        let x = LEFT + (i as f64 + 0.5) * slot;
        let edge = i == 0 || i + 1 == buckets.len();
        let month_start = date.day() <= if weekly { 7 } else { 1 } && buckets.len() > 20;
        if !(edge || month_start) || x - last_label_x < 64.0 && i + 1 != buckets.len() {
            continue;
        }
        let text = if edge {
            format!("{}.{}.{}", date.year() % 100, date.month(), date.day())
        } else {
            format!("{}월", date.month())
        };
        let anchor = if i == 0 {
            "start"
        } else if i + 1 == buckets.len() {
            "end"
        } else {
            "middle"
        };
        let x = if i == 0 {
            LEFT
        } else if i + 1 == buckets.len() {
            W - RIGHT
        } else {
            x
        };
        let _ = write!(
            s,
            "<text x=\"{x:.1}\" y=\"{y}\" text-anchor=\"{anchor}\">{text}</text>"
        );
        last_label_x = x;
    }
    s.push_str("</svg></figure>\n");
    s
}
