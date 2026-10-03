//! Creates a sample web novel project for trying the app.
//!
//! ```bash
//! cargo run -p writer-core --example sample_project -- <parent folder>
//! ```

use std::path::PathBuf;

use chrono::Duration;
use writer_core::doc::{self, MetaPatch};
use writer_core::markup::Block;
use writer_core::project::{self, NewDoc, NewProject, ProjectKind};

fn main() {
    let parent = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .expect("usage: sample_project <parent folder>");
    let root = project::create(&NewProject {
        parent: parent.to_string_lossy().into_owned(),
        title: "달빛 서점의 마지막 손님".into(),
        kind: ProjectKind::Webnovel,
        per_doc_goal: Some(5000),
        count_spaces: true,
        first_chapter: true,
        platform: None,
    })
    .expect("create project");

    let chapters: [(&str, &str, &str, &[&str]); 3] = [
        (
            "문 닫는 시간",
            "stock",
            "서하가 할머니에게 물려받은 서점을 닫는 밤.",
            &[
                "서하는 매일 밤 열한 시에 서점 문을 닫았다. 할머니가 그랬고, 할머니의 어머니도 그랬다고 했다.",
                "“오늘도 손님은 없었네.”",
                "혼잣말은 오래된 서가 사이로 흩어졌다.",
            ],
        ),
        (
            "빗소리가 들리는 밤",
            "stock",
            "",
            &[
                "비는 저녁부터 내렸다. 유리문에 맺힌 빗방울 너머로 골목의 가로등이 번졌다.",
                "서하는 장부를 덮고 창밖을 오래 보았다.",
            ],
        ),
        (
            "비에 젖은 손님",
            "draft",
            "폐점 직전 찾아온 손님이 서하에게 할머니 시절의 대여 카드를 내민다.",
            &[
                "셔터를 반쯤 내렸을 때 종이 울렸다. 이 시간에 문을 여는 사람은 없었다. 적어도 지난 삼 년 동안은 그랬다.",
                "윤서하는 계산대 아래에서 우산을 꺼내 들었다. 문턱에 선 남자는 우산도 없이 젖어 있었고, 품에 안은 종이봉투만은 이상하리만치 말라 있었다.",
                "“영업, 끝났나요?”",
                "“끝났어요. 그런데 들어오세요. 그 봉투가 젖으면 곤란할 것 같으니까.”",
                "***",
                "남자가 봉투에서 꺼낸 것은 책이 아니라 대여 카드였다. 누렇게 바랜 칸마다 같은 이름이 적혀 있었다.",
            ],
        ),
    ];

    let mut previous: Option<String> = None;
    for (i, (title, status, synopsis, paragraphs)) in chapters.iter().enumerate() {
        let id = if i == 0 {
            project::overview(&root).expect("overview").parts[0].docs[0]
                .id
                .clone()
        } else {
            project::add_doc(
                &root,
                &NewDoc {
                    after: previous.clone(),
                    ..Default::default()
                },
            )
            .expect("add chapter")
        };
        doc::update_meta(
            &root,
            &id,
            &MetaPatch {
                title: Some((*title).into()),
                synopsis: Some((*synopsis).into()),
                status: Some((*status).into()),
                ..Default::default()
            },
        )
        .expect("meta");
        let body = paragraphs
            .iter()
            .map(|p| {
                if *p == "***" {
                    Block::SceneBreak {}
                } else {
                    Block::text(p)
                }
            })
            .collect();
        doc::save_body(&root, &id, body, Duration::minutes(10), Default::default()).expect("body");
        previous = Some(id);
    }
    println!("{}", root.display());
}
