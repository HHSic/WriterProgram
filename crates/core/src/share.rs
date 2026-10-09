//! A reader's copy of selected manuscript chapters. Private project metadata,
//! planning, notes and history never enter this payload.
use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{Error, Result, doc, markup, project};

// Bound the reader copy before generating and uploading its document.
pub const MAX_SHARE_BYTES: usize = 8 * 1024 * 1024 - 1024;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub title: String,
    pub chapters: Vec<Chapter>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Chapter {
    pub title: String,
    pub text: String,
}

pub fn snapshot(root: &Path, selected: &[String]) -> Result<Snapshot> {
    if selected.is_empty() || selected.len() > 500 {
        return Err(Error::Invalid("공유할 원고를 1~500개 골라 주세요".into()));
    }
    let project = project::load(root)?;
    if project.title.chars().count() > 500 {
        return Err(Error::Invalid(
            "공유할 작품 제목은 500자 이내로 적어 주세요".into(),
        ));
    }
    let selected: HashSet<_> = selected.iter().collect();
    let mut found = HashSet::new();
    let mut chapters = Vec::new();
    for id in project.parts.iter().flat_map(|part| &part.docs) {
        if !selected.contains(id) {
            continue;
        }
        let manuscript = doc::load(root, id)?;
        if manuscript.meta.title.chars().count() > 500 {
            return Err(Error::Invalid(
                "공유할 원고 제목은 500자 이내로 적어 주세요".into(),
            ));
        }
        chapters.push(Chapter {
            title: manuscript.meta.title,
            text: markup::plain_text(&manuscript.body),
        });
        found.insert(id);
    }
    if found.len() != selected.len() {
        return Err(Error::Invalid(
            "작품에 있는 원고만 공유할 수 있습니다".into(),
        ));
    }
    let snapshot = Snapshot {
        title: project.title,
        chapters,
    };
    if serde_json::to_vec(&snapshot)
        .map_err(|_| Error::Invalid("공유 원고를 만들 수 없습니다".into()))?
        .len()
        > MAX_SHARE_BYTES
    {
        return Err(Error::Invalid(
            "공유할 원고가 너무 큽니다. 회차를 나누어 공유해 주세요".into(),
        ));
    }
    Ok(snapshot)
}
