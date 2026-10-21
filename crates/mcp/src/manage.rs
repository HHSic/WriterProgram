//! Tools that reorganize and look across a project: parts, moving and
//! trashing documents, find and replace, records, and export.
//!
//! Each one is a thin call into writer-core, so it keeps the app's safeguards:
//! replacing keeps a "before-replace" record per document and leaves locked
//! chapters alone, restoring keeps a "before-restore" record, trash keeps
//! documents for 30 days.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use writer_core::export::{self, DocOptions, ExportItem, FileKind, TextOptions};
use writer_core::project::{self, ProjectKind};
use writer_core::search::{self, SearchQuery};
use writer_core::{doc, snapshot, trash};

use crate::tools::{Outcome, err, required, str_arg, tool};

pub fn list() -> Vec<Value> {
    let query = || {
        json!({
            "text": {"type": "string", "description": "찾을 말"},
            "regex": {"type": "boolean", "description": "text를 정규식으로"},
            "whole_word": {"type": "boolean", "description": "낱말 단위로만"},
            "doc_ids": {"type": "array", "items": {"type": "string"}, "description": "찾을 문서. 없으면 작품 전체(원고, 기획 문서)"},
        })
    };
    let mut replace = query();
    replace["replacement"] =
        json!({"type": "string", "description": "바꿀 말. 정규식이면 $1 같은 묶음을 쓸 수 있다"});
    vec![
        tool(
            "add_part",
            "새 부를 맨 뒤에 만든다.",
            json!({"title": {"type": "string"}}),
            &["title"],
        ),
        tool(
            "rename_part",
            "부 이름을 바꾼다.",
            json!({"part_id": {"type": "string"}, "title": {"type": "string"}}),
            &["part_id", "title"],
        ),
        tool(
            "remove_part",
            "빈 부를 지운다. 회차가 남은 부나 하나뿐인 부는 지우지 않는다.",
            json!({"part_id": {"type": "string"}}),
            &["part_id"],
        ),
        tool(
            "move_doc",
            "회차를 옮긴다. part_id 부의 index 자리(0부터, 옮길 회차를 뺀 순서)로 간다. 기획 문서는 part_id 없이 기획 문서 사이에서만 옮긴다.",
            json!({
                "id": {"type": "string"},
                "part_id": {"type": "string"},
                "index": {"type": "integer", "minimum": 0},
            }),
            &["id", "index"],
        ),
        tool(
            "trash_doc",
            "회차나 기획 문서를 휴지통으로 보낸다. 30일 동안 restore_trash로 되살릴 수 있다. 작가가 지우라고 했을 때만 쓴다.",
            json!({"id": {"type": "string"}}),
            &["id"],
        ),
        tool(
            "trash",
            "휴지통 목록: 휴지통 id, 문서, 제목, 지운 때, 글자 수.",
            json!({}),
            &[],
        ),
        tool(
            "restore_trash",
            "휴지통 항목을 원래 자리로 되살린다.",
            json!({"trash_id": {"type": "string"}}),
            &["trash_id"],
        ),
        tool(
            "search",
            "작품 전체나 고른 문서에서 찾는다. 찾은 곳마다 앞뒤 문맥을 보인다.",
            query(),
            &["text"],
        ),
        tool(
            "replace_all",
            "찾은 곳을 모두 바꾼다. 굵게·방점 같은 서식은 그대로 두고, 바뀌는 문서마다 '바꾸기 전' 기록을 남긴다. 잠근 회차는 건너뛴다. 먼저 search로 몇 곳인지 작가에게 보이고, 바꾸라고 했을 때만 쓴다. 이름을 바꾸면 뒤의 조사(은/는, 이/가, 을/를)가 어긋날 수 있으니 바꾼 뒤 search로 다시 확인한다.",
            replace,
            &["text", "replacement"],
        ),
        tool(
            "records",
            "문서 하나의 기록 목록(최근 것부터): 기록 id, 종류, 이름, 때, 글자 수.",
            json!({"id": {"type": "string"}}),
            &["id"],
        ),
        tool(
            "restore_record",
            "문서를 기록 하나의 본문으로 되돌린다. 지금 본문은 '되돌리기 전' 기록으로 남는다. 잠근 회차는 되돌리지 않는다.",
            json!({"id": {"type": "string"}, "record": {"type": "string", "description": "records에서 받은 기록 id"}}),
            &["id", "record"],
        ),
        tool(
            "export",
            "원고를 txt, docx(Word), hwpx(한글) 파일로 내보낸다. docx와 hwpx에는 작품의 원고 서식(용지, 글꼴, 들여쓰기)을 적용한다.",
            json!({
                "format": {"type": "string", "enum": ["txt", "docx", "hwpx"]},
                "dest": {"type": "string", "description": "만들 파일의 전체 경로. per_doc이면 파일들을 넣을 폴더"},
                "doc_ids": {"type": "array", "items": {"type": "string"}, "description": "내보낼 회차. 없으면 원고 전체를 순서대로"},
                "per_doc": {"type": "boolean", "description": "회차마다 파일 하나"},
                "include_titles": {"type": "boolean", "description": "회차 제목을 본문 위에 (기본 true)"},
                "overwrite": {"type": "boolean", "description": "이미 있는 파일을 덮어써도 된다고 작가가 확인했을 때만 true"},
            }),
            &["format", "dest"],
        ),
    ]
}

/// Runs one of these tools, or `None` when `name` is not one of them.
pub fn call(name: &str, root: &Path, args: &Value) -> Option<Outcome> {
    Some(match name {
        "add_part" => add_part(root, args),
        "rename_part" => rename_part(root, args),
        "remove_part" => remove_part(root, args),
        "move_doc" => move_doc(root, args),
        "trash_doc" => trash_doc(root, args),
        "trash" => list_trash(root),
        "restore_trash" => restore_trash(root, args),
        "search" => find(root, args),
        "replace_all" => replace_all(root, args),
        "records" => records(root, args),
        "restore_record" => restore_record(root, args),
        "export" => export_docs(root, args),
        _ => return None,
    })
}

fn bool_arg(args: &Value, key: &str) -> Option<bool> {
    args.get(key).and_then(Value::as_bool)
}

fn strings(args: &Value, key: &str) -> Option<Vec<String>> {
    args.get(key).and_then(Value::as_array).map(|a| {
        a.iter()
            .filter_map(Value::as_str)
            .map(String::from)
            .collect()
    })
}

fn add_part(root: &Path, args: &Value) -> Outcome {
    let id = project::add_part(root, required(args, "title")?).map_err(err)?;
    Ok(json!({"part_id": id}))
}

fn rename_part(root: &Path, args: &Value) -> Outcome {
    let part_id = required(args, "part_id")?;
    project::rename_part(root, part_id, required(args, "title")?).map_err(err)?;
    Ok(json!({"part_id": part_id}))
}

fn remove_part(root: &Path, args: &Value) -> Outcome {
    let part_id = required(args, "part_id")?;
    project::remove_part(root, part_id).map_err(err)?;
    Ok(json!({"removed": part_id}))
}

fn move_doc(root: &Path, args: &Value) -> Outcome {
    let id = required(args, "id")?;
    let index = args
        .get("index")
        .and_then(Value::as_u64)
        .ok_or("index를 주세요 (0부터)")? as usize;
    project::move_doc(root, id, str_arg(args, "part_id"), index).map_err(err)?;
    Ok(json!({"id": id, "part_id": str_arg(args, "part_id"), "index": index}))
}

fn trash_doc(root: &Path, args: &Value) -> Outcome {
    let item = trash::trash_doc(root, required(args, "id")?).map_err(err)?;
    Ok(json!({"trash_id": item.id, "doc_id": item.doc_id, "title": item.title}))
}

fn list_trash(root: &Path) -> Outcome {
    let items = trash::list(root).map_err(err)?;
    Ok(Value::Array(
        items
            .iter()
            .map(|i| {
                json!({
                    "trash_id": i.id,
                    "doc_id": i.doc_id,
                    "section": i.section,
                    "title": i.title,
                    "deleted_at": i.deleted_at,
                    "chars": i.chars,
                })
            })
            .collect(),
    ))
}

fn restore_trash(root: &Path, args: &Value) -> Outcome {
    let item = trash::restore(root, required(args, "trash_id")?).map_err(err)?;
    Ok(json!({"doc_id": item.doc_id, "title": item.title, "part_id": item.part_id}))
}

fn query(args: &Value) -> Result<SearchQuery, String> {
    Ok(SearchQuery {
        text: required(args, "text")?.to_string(),
        regex: bool_arg(args, "regex").unwrap_or(false),
        whole_word: bool_arg(args, "whole_word").unwrap_or(false),
        doc_ids: strings(args, "doc_ids"),
    })
}

fn find(root: &Path, args: &Value) -> Outcome {
    let found = search::search(root, &query(args)?).map_err(err)?;
    serde_json::to_value(found).map_err(|e| e.to_string())
}

fn replace_all(root: &Path, args: &Value) -> Outcome {
    let q = query(args)?;
    let done = search::replace_all(root, &q, required(args, "replacement")?).map_err(err)?;
    serde_json::to_value(done).map_err(|e| e.to_string())
}

fn records(root: &Path, args: &Value) -> Outcome {
    let list = snapshot::list(root, required(args, "id")?).map_err(err)?;
    Ok(Value::Array(
        list.iter()
            .map(|r| {
                json!({
                    "record": r.id,
                    "kind": r.kind,
                    "name": r.name,
                    "at": r.at,
                    "chars": r.counts.with_spaces,
                })
            })
            .collect(),
    ))
}

fn restore_record(root: &Path, args: &Value) -> Outcome {
    let id = required(args, "id")?;
    if doc::load(root, id).map_err(err)?.meta.locked {
        return Err("잠근 회차라 되돌리지 않음. 작가가 앱에서 잠금을 풀어야 함".into());
    }
    let before = snapshot::restore(root, id, required(args, "record")?).map_err(err)?;
    Ok(json!({"id": id, "before_restore": before.id}))
}

fn export_docs(root: &Path, args: &Value) -> Outcome {
    let dest = PathBuf::from(required(args, "dest")?);
    if !dest.is_absolute() {
        return Err("dest는 전체 경로로 주세요".into());
    }
    let per_doc = bool_arg(args, "per_doc").unwrap_or(false);
    let overwrite = bool_arg(args, "overwrite").unwrap_or(false);
    let include_titles = bool_arg(args, "include_titles").unwrap_or(true);
    if per_doc {
        std::fs::create_dir_all(&dest).map_err(|e| format!("{}: {e}", dest.display()))?;
        let full = std::fs::read_dir(&dest)
            .map(|mut d| d.next().is_some())
            .unwrap_or(false);
        if full && !overwrite {
            return Err("내보낼 폴더가 비어 있지 않음. 같은 이름 파일을 덮어쓸 수 있으니 빈 폴더를 주거나, 작가가 괜찮다고 하면 overwrite: true".into());
        }
    } else if dest.exists() && !overwrite {
        return Err(format!(
            "이미 있는 파일: {}. 다른 이름을 주거나, 작가가 괜찮다고 하면 overwrite: true",
            dest.display()
        ));
    }

    let overview = project::overview(root).map_err(err)?;
    let chapters: Vec<_> = overview.parts.iter().flat_map(|p| &p.docs).collect();
    let wanted = strings(args, "doc_ids");
    let items: Vec<ExportItem> = match &wanted {
        None => chapters
            .iter()
            .enumerate()
            .map(|(i, d)| item(&d.id, &d.title, i + 1))
            .collect(),
        Some(ids) => ids
            .iter()
            .map(|id| {
                let pos = chapters.iter().position(|d| &d.id == id);
                let title = pos.map(|i| chapters[i].title.as_str()).unwrap_or("");
                item(id, title, pos.map_or(0, |i| i + 1))
            })
            .collect(),
    };

    let project = project::load(root).map_err(err)?;
    let paths = match required(args, "format")? {
        "txt" => export::export_txt(
            root,
            &items,
            &TextOptions {
                include_titles,
                blank_line_between: matches!(project.kind, ProjectKind::Webnovel),
                scene_break: project.scene_break.clone(),
            },
            &dest,
            per_doc,
        ),
        format @ ("docx" | "hwpx") => export::export_file(
            root,
            &items,
            &DocOptions {
                include_titles,
                scene_break: project.scene_break.clone(),
            },
            &project.manuscript_format(),
            if format == "docx" {
                FileKind::Docx
            } else {
                FileKind::Hwpx
            },
            &dest,
            per_doc,
        ),
        other => return Err(format!("format은 txt, docx, hwpx 가운데 하나: {other}")),
    }
    .map_err(err)?;
    Ok(json!({
        "files": paths.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
        "docs": items.len(),
    }))
}

/// A chapter to export, headed by its title (or its place when untitled).
fn item(id: &str, title: &str, place: usize) -> ExportItem {
    let heading = if title.trim().is_empty() {
        place.to_string()
    } else {
        title.to_string()
    };
    ExportItem {
        doc_id: id.to_string(),
        file_name: heading.clone(),
        heading,
    }
}

#[cfg(test)]
mod tests {
    use crate::tools::call;
    use serde_json::{Value, json};
    use std::path::{Path, PathBuf};

    fn new_project() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let made = call(
            "create_project",
            &json!({"parent": dir.path().to_str().unwrap(), "title": "사흘째 맑음"}),
            None,
        )
        .unwrap();
        let root = PathBuf::from(made["project"].as_str().unwrap());
        (dir, root)
    }

    fn run(root: &Path, name: &str, args: Value) -> Value {
        call(name, &args, Some(root)).unwrap_or_else(|e| panic!("{name}: {e}"))
    }

    fn chapter(root: &Path, title: &str, text: &str) -> String {
        run(root, "add_doc", json!({"title": title, "text": text}))["id"]
            .as_str()
            .unwrap()
            .to_string()
    }

    fn titles(root: &Path) -> Vec<Vec<String>> {
        run(root, "outline", json!({}))["parts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| {
                p["docs"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|d| d["title"].as_str().unwrap().to_string())
                    .collect()
            })
            .collect()
    }

    #[test]
    fn parts_and_moving_chapters() {
        let (_dir, root) = new_project();
        let first = run(&root, "outline", json!({}))["parts"][0]["docs"][0]["id"].clone();
        run(&root, "set_meta", json!({"id": first, "title": "1"}));
        chapter(&root, "2", "둘");
        let three = chapter(&root, "3", "셋");
        let part = run(&root, "add_part", json!({"title": "2부"}))["part_id"].clone();
        run(
            &root,
            "move_doc",
            json!({"id": three, "part_id": part, "index": 0}),
        );
        assert_eq!(titles(&root), vec![vec!["1", "2"], vec!["3"]]);
        run(
            &root,
            "rename_part",
            json!({"part_id": part, "title": "끝"}),
        );
        let e = call("remove_part", &json!({"part_id": part}), Some(&root)).unwrap_err();
        assert!(e.contains("회차"), "{e}");
        run(
            &root,
            "move_doc",
            json!({"id": three, "part_id": run(&root, "outline", json!({}))["parts"][0]["id"], "index": 0}),
        );
        assert_eq!(titles(&root), vec![vec!["3", "1", "2"], vec![]]);
        run(&root, "remove_part", json!({"part_id": part}));
        assert_eq!(titles(&root).len(), 1);
    }

    #[test]
    fn trash_and_back() {
        let (_dir, root) = new_project();
        let id = chapter(&root, "지울 회차", "본문");
        let trashed = run(&root, "trash_doc", json!({"id": id}));
        assert_eq!(titles(&root), vec![vec![""]]);
        let listed = run(&root, "trash", json!({}));
        assert_eq!(listed[0]["title"], "지울 회차");
        run(
            &root,
            "restore_trash",
            json!({"trash_id": trashed["trash_id"]}),
        );
        assert_eq!(titles(&root), vec![vec!["", "지울 회차"]]);
        assert_eq!(run(&root, "read_doc", json!({"id": id}))["text"], "본문");
    }

    #[test]
    fn find_and_replace_keeps_marks_and_records() {
        let (_dir, root) = new_project();
        let a = chapter(&root, "1", "윤재는 열쇠를 깎았다.\n서아가 윤재를 불렀다.");
        let b = chapter(&root, "2", "윤재가 뛰었다.");
        writer_core::doc::save_body(
            &root,
            &b,
            writer_core::markup::parse_body("**윤재**가 뛰었다."),
            chrono::Duration::minutes(10),
            writer_core::doc::SaveGuard::default(),
        )
        .unwrap();
        let found = run(&root, "search", json!({"text": "윤재"}));
        assert_eq!(found["total"], 3);
        let done = run(
            &root,
            "replace_all",
            json!({"text": "윤재", "replacement": "도윤"}),
        );
        assert_eq!(done["replaced"], 3);
        assert_eq!(
            run(&root, "read_doc", json!({"id": a}))["text"],
            "도윤는 열쇠를 깎았다.\n\n서아가 도윤를 불렀다."
        );
        let body = writer_core::doc::load(&root, &b).unwrap().body;
        assert_eq!(
            writer_core::markup::write_body(&body).trim_end(),
            "**도윤**가 뛰었다."
        );
        let records = run(&root, "records", json!({"id": a}));
        assert_eq!(records[0]["kind"], "before-replace");

        run(
            &root,
            "restore_record",
            json!({"id": a, "record": records[0]["record"]}),
        );
        assert_eq!(
            run(&root, "read_doc", json!({"id": a}))["text"],
            "윤재는 열쇠를 깎았다.\n\n서아가 윤재를 불렀다."
        );
        assert_eq!(
            run(&root, "records", json!({"id": a}))[0]["kind"],
            "before-restore"
        );
    }

    #[test]
    fn exports_txt_docx_and_hwpx() {
        let (dir, root) = new_project();
        chapter(
            &root,
            "열쇠",
            "오후 네 시가 지나면 시장은 한동안 비었다.\n***\n비가 왔다.",
        );
        let txt = dir.path().join("out.txt");
        run(
            &root,
            "export",
            json!({"format": "txt", "dest": txt.to_str().unwrap()}),
        );
        let text = std::fs::read_to_string(&txt).unwrap();
        assert!(
            text.contains("열쇠") && text.contains("비가 왔다."),
            "{text}"
        );

        let e = call(
            "export",
            &json!({"format": "txt", "dest": txt.to_str().unwrap()}),
            Some(&root),
        )
        .unwrap_err();
        assert!(e.contains("이미 있는 파일"), "{e}");

        for format in ["docx", "hwpx"] {
            let dest = dir.path().join(format!("out.{format}"));
            run(
                &root,
                "export",
                json!({"format": format, "dest": dest.to_str().unwrap()}),
            );
            let bytes = std::fs::read(&dest).unwrap();
            assert_eq!(&bytes[..2], b"PK", "{format} is a zip");
        }

        let folder = dir.path().join("each");
        let out = run(
            &root,
            "export",
            json!({"format": "txt", "dest": folder.to_str().unwrap(), "per_doc": true}),
        );
        assert_eq!(out["files"].as_array().unwrap().len(), 2);
        let e = call(
            "export",
            &json!({"format": "txt", "dest": "상대/경로.txt"}),
            Some(&root),
        )
        .unwrap_err();
        assert!(e.contains("전체 경로"), "{e}");
    }
}
