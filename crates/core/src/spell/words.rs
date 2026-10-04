//! Word lists the writer keeps (사용자 사전): one per project, in the
//! project folder so it goes along to other devices, and one per device for
//! words used in every work.
//!
//! ```text
//! {"words": ["서하네", "콰르릉"]}
//! ```

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::store::{atomic_write, read_text};
use crate::{Error, Result};

/// The project's list (이 작품 사전), next to `project.json`.
pub const PROJECT_WORDS: &str = "spelling.json";

/// Longest word a list takes.
const MAX_WORD_CHARS: usize = 40;

#[derive(Debug, Default, Serialize, Deserialize)]
struct WordFile {
    #[serde(default)]
    words: Vec<String>,
}

pub fn project_words_file(root: &Path) -> PathBuf {
    root.join(PROJECT_WORDS)
}

/// The words in the list at `path`; none when there is no list yet.
pub fn read(path: &Path) -> Result<Vec<String>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = read_text(path)?;
    let file: WordFile =
        serde_json::from_str(&text).map_err(|e| Error::format(path, e.to_string()))?;
    Ok(tidy(file.words))
}

fn write(path: &Path, words: Vec<String>) -> Result<Vec<String>> {
    let words = tidy(words);
    let file = WordFile {
        words: words.clone(),
    };
    let text = serde_json::to_string_pretty(&file).expect("plain JSON");
    atomic_write(path, format!("{text}\n").as_bytes())?;
    Ok(words)
}

/// Sorted, without blanks or doubles.
fn tidy(words: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = words
        .into_iter()
        .map(|w| w.trim().to_string())
        .filter(|w| !w.is_empty())
        .collect();
    out.sort();
    out.dedup();
    out
}

/// Adds `word` to the list at `path` and returns the list.
pub fn add(path: &Path, word: &str) -> Result<Vec<String>> {
    let word = word.trim();
    if word.is_empty() || word.chars().any(char::is_whitespace) {
        return Err(Error::Invalid(
            "띄어 쓰지 않은 낱말 하나만 넣을 수 있음".into(),
        ));
    }
    if word.chars().count() > MAX_WORD_CHARS {
        return Err(Error::Invalid("낱말이 너무 김".into()));
    }
    let mut words = read(path)?;
    if words.iter().any(|w| w == word) {
        return Ok(words);
    }
    words.push(word.to_string());
    write(path, words)
}

/// Takes `word` out of the list at `path` and returns the list.
pub fn remove(path: &Path, word: &str) -> Result<Vec<String>> {
    let words = read(path)?;
    if !words.iter().any(|w| w == word) {
        return Ok(words);
    }
    write(path, words.into_iter().filter(|w| w != word).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_keep_words_sorted_and_single() {
        let dir = tempfile::tempdir().unwrap();
        let path = project_words_file(dir.path());
        assert!(read(&path).unwrap().is_empty());
        add(&path, "콰르릉").unwrap();
        add(&path, " 서하네 ").unwrap();
        let words = add(&path, "콰르릉").unwrap();
        assert_eq!(words, vec!["서하네", "콰르릉"]);
        assert_eq!(read(&path).unwrap(), words);
        assert_eq!(remove(&path, "서하네").unwrap(), vec!["콰르릉"]);
    }

    #[test]
    fn one_word_at_a_time() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("words.json");
        assert!(add(&path, "두 낱말").is_err());
        assert!(add(&path, "  ").is_err());
    }
}
