//! Writes sample chapters as docx and HWPX in every built-in manuscript
//! format, for checking the output in Word or 한글.
//!
//! ```bash
//! cargo run -p writer-core --example export_formats -- <output folder>
//! ```

use std::path::PathBuf;

use writer_core::docx::docx_bytes;
use writer_core::export::{DocInfo, DocOptions, ExportDoc};
use writer_core::format::{HeadAlign, HeadContent, RunningHead, builtin_presets};
use writer_core::hwpx::hwpx_bytes;
use writer_core::layout::PageMetrics;
use writer_core::markup::{Block, Inline, Mark, ParaAttrs};

fn text(s: &str, marks: &[Mark]) -> Inline {
    Inline::Text {
        text: s.into(),
        marks: marks.to_vec(),
    }
}

fn chapters() -> Vec<ExportDoc> {
    let long = "셔터를 반쯤 내렸을 때 종이 울렸다. 이 시간에 문을 여는 사람은 없었다. 적어도 지난 삼 년 동안은 그랬다. 윤서하는 계산대 아래에서 우산을 꺼내 들었다. 문턱에 선 남자는 우산도 없이 젖어 있었고, 품에 안은 종이봉투만은 이상하리만치 말라 있었다.";
    let mut first = vec![
        Block::text(long),
        Block::text("“영업, 끝났나요?”"),
        Block::text("“끝났어요. 그런데 들어오세요. 그 봉투가 젖으면 곤란할 것 같으니까.”"),
        Block::Paragraph {
            attrs: Default::default(),
            content: vec![
                text("표시: ", &[]),
                text("굵게", &[Mark::Bold {}]),
                text(", ", &[]),
                text("기울임", &[Mark::Italic {}]),
                text(", ", &[]),
                text("밑줄", &[Mark::Underline {}]),
                text(", ", &[]),
                text("취소선", &[Mark::Strike {}]),
                text(", ", &[]),
                text("방점", &[Mark::Dot {}]),
                text(". 그리고 <상태창>과 & 기호.", &[]),
            ],
        },
        Block::Paragraph {
            attrs: Default::default(),
            content: vec![
                text("문단 안에서 줄을 바꾼 첫 줄", &[]),
                Inline::HardBreak {},
                text("둘째 줄", &[]),
            ],
        },
        Block::text("대여 카드 뒷면에는 짧은 편지가 붙어 있었다."),
        Block::Paragraph {
            attrs: ParaAttrs {
                left: 3,
                right: 2,
                indent: None,
            },
            content: vec![
                text("서하에게.", &[]),
                Inline::HardBreak {},
                text(
                    "이 카드를 가져오는 사람이 있으면, 지하 서고의 문을 열어 주렴. 할머니가.",
                    &[],
                ),
            ],
        },
        Block::SceneBreak {},
        Block::text(
            "남자가 봉투에서 꺼낸 것은 책이 아니라 대여 카드였다. 누렇게 바랜 칸마다 같은 이름이 적혀 있었다. 마지막 칸의 날짜는 서하가 태어나기도 전이었다.",
        ),
    ];
    for i in 0..12 {
        first.push(Block::text(&format!(
            "{}번째로 이어지는 문단. {long}",
            i + 1
        )));
    }
    vec![
        ExportDoc {
            heading: "1장 비에 젖은 손님".into(),
            blocks: first,
        },
        ExportDoc {
            heading: "2장 지하 서고의 열쇠".into(),
            blocks: vec![
                Block::text("둘째 장은 새 쪽에서 시작한다."),
                Block::text(long),
            ],
        },
    ]
}

fn main() {
    let out = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .expect("usage: export_formats <output folder>");
    std::fs::create_dir_all(&out).expect("output folder");
    let docs = chapters();
    let info = DocInfo {
        title: "달빛 서점의 마지막 손님".into(),
        author: "달무리".into(),
    };
    // Two more with 머리말: the work's title at the right on every page, and
    // a book's running heads (title on even pages, chapter on odd ones).
    let mut presets = builtin_presets();
    let mut titled = presets[0].2.clone();
    titled.header = RunningHead {
        content: HeadContent::Title,
        align: HeadAlign::Right,
        skip_chapter_first: false,
        ..RunningHead::default()
    };
    presets.push(("submission-a4-head", "투고 원고 + 머리말", titled));
    let mut book = presets[2].2.clone();
    book.header = RunningHead {
        content: HeadContent::TitleChapter,
        align: HeadAlign::Outside,
        skip_chapter_first: true,
        ..RunningHead::default()
    };
    presets.push(("book-shinguk-head", "책 (신국판) + 머리말", book));
    for (id, name, format) in presets {
        let opts = DocOptions {
            include_titles: true,
            scene_break: "* * *".into(),
        };
        let docx = docx_bytes(&docs, &format, &opts, &info).expect("docx");
        std::fs::write(out.join(format!("{id}.docx")), docx).expect("write docx");
        let hwpx = hwpx_bytes(&docs, &format, &opts, &info).expect("hwpx");
        std::fs::write(out.join(format!("{id}.hwpx")), hwpx).expect("write hwpx");
        let estimate = PageMetrics::of(&format)
            .map(|m| {
                m.pages(docs.iter().map(|d| d.blocks.as_slice()), true)
                    .to_string()
            })
            .unwrap_or_else(|| "-".into());
        println!("{id}: {name} · 예상 {estimate}쪽");
    }
}
