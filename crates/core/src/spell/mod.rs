//! 맞춤법 검사: spelling checked word by word (어절) against the Korean
//! Hunspell dictionary (hunspell-dict-ko), read by the pure-Rust `spellbook`.
//!
//! The dictionary is GPL-3.0 data. It is never built into the app: it is
//! fetched once, after the writer agrees, into the app's data folder
//! (`install`), with its license files next to it. See
//! docs/spellcheck-license.md.
//!
//! What is accepted besides the dictionary's words (`Known`, `check`):
//! the project's setting card names and aliases, the project's own word list
//! and this device's, each also with particles after it ("서하는",
//! "서하에게"), and drawn-out or repeated sounds whose short form is a word
//! ("콰아앙", "두근두근두근").

mod check;
mod compound;
mod hangul;
pub mod install;
mod prepare;
pub mod words;
mod work;

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

pub use check::{Issue, Known, check_text, word_ok};
pub use work::{DocIssues, WorkCheck, check_work};

use crate::store::read_text;
use crate::{Error, Result};
use compound::Compounds;

/// Words checked so far and the answer; cleared when it grows past this.
const CACHE_LIMIT: usize = 200_000;

/// Suggestions offered for one word.
const MAX_SUGGESTIONS: usize = 6;

/// A loaded dictionary. Loading takes about half a second (release build),
/// so the app keeps one.
pub struct Speller {
    dict: spellbook::Dictionary,
    compounds: Compounds,
    cache: Mutex<HashMap<String, bool>>,
}

impl Speller {
    /// A speller from `.aff` and `.dic` text.
    pub fn new(aff: &str, dic: &str) -> Result<Self> {
        let prepared = prepare::prepare(aff, dic);
        let dict = spellbook::Dictionary::new(&prepared.aff, &prepared.dic)
            .map_err(|e| Error::Invalid(format!("맞춤법 사전을 읽을 수 없음 ({e})")))?;
        Ok(Speller {
            dict,
            compounds: Compounds::new(prepared.rules, prepared.heads),
            cache: Mutex::new(HashMap::new()),
        })
    }

    /// The speller of the dictionary in `dir` (`ko.aff`, `ko.dic`).
    pub fn open(dir: &Path) -> Result<Self> {
        let aff = read_text(&dir.join(install::AFF))?;
        let dic = read_text(&dir.join(install::DIC))?;
        Self::new(&aff, &dic)
    }

    /// Whether the dictionary knows `word` as it stands.
    pub fn knows(&self, word: &str) -> bool {
        if word.is_empty() {
            return true;
        }
        if let Some(known) = self.lock_cache().get(word) {
            return *known;
        }
        let answer = self.dict.check(word)
            || (!self.compounds.is_empty()
                && self.compounds.matches(word, &mut |flag, part| {
                    let mut marked = String::with_capacity(part.len() + 4);
                    marked.push(prepare::tail_marker(flag));
                    marked.push_str(part);
                    self.dict.check(&marked)
                }));
        let mut cache = self.lock_cache();
        if cache.len() >= CACHE_LIMIT {
            cache.clear();
        }
        cache.insert(word.to_string(), answer);
        answer
    }

    /// Words the writer may have meant, best first. Can take a second for a
    /// long word.
    pub fn suggest(&self, word: &str) -> Vec<String> {
        let mut out = Vec::new();
        self.dict.suggest(word, &mut out);
        let mut seen = std::collections::HashSet::new();
        out.retain(|s| {
            !s.is_empty()
                && s != word
                && !s.contains(|c: char| ('\u{F0000}'..).contains(&c))
                && seen.insert(s.clone())
        });
        out.truncate(MAX_SUGGESTIONS);
        out
    }

    fn lock_cache(&self) -> std::sync::MutexGuard<'_, HashMap<String, bool>> {
        self.cache.lock().unwrap_or_else(|p| p.into_inner())
    }
}

#[cfg(test)]
pub(crate) mod fixture {
    //! A small dictionary written for the tests in the shape of
    //! hunspell-dict-ko (numeric flags with aliases, particles as suffixes,
    //! compound rules for 들 and auxiliary verbs). Not the real dictionary,
    //! which is GPL data and never kept in the repository.

    use super::Speller;
    use super::hangul::to_jamo;

    pub const AFF: &str = include_str!("../../tests/fixtures/spell/ko.aff");
    pub const DIC: &str = include_str!("../../tests/fixtures/spell/ko.dic");

    pub fn speller() -> Speller {
        Speller::new(AFF, DIC).expect("fixture dictionary")
    }

    /// The fixture written as the real dictionary is: words and affixes in
    /// conjoining letters, with ICONV/OCONV tables between syllables and
    /// letters.
    pub fn jamo_speller() -> Speller {
        let mut syllables: Vec<char> = AFF
            .chars()
            .chain(DIC.chars())
            .chain("멱었다사람들은나무는새겨져".chars())
            .filter(|c| super::hangul::is_syllable(*c))
            .collect();
        syllables.sort_unstable();
        syllables.dedup();
        let mut aff = String::new();
        for line in AFF.lines() {
            if line.starts_with("SFX ") || line.starts_with("TRY ") {
                aff.push_str(&to_jamo(line));
            } else {
                aff.push_str(line);
            }
            aff.push('\n');
            if line.starts_with("LANG") {
                aff.push_str(&format!("ICONV {}\n", syllables.len()));
                for s in &syllables {
                    aff.push_str(&format!("ICONV {s} {}\n", to_jamo(&s.to_string())));
                }
                aff.push_str(&format!("OCONV {}\n", syllables.len()));
                for s in &syllables {
                    aff.push_str(&format!("OCONV {} {s}\n", to_jamo(&s.to_string())));
                }
            }
        }
        Speller::new(&aff, &to_jamo(DIC)).expect("fixture dictionary in letters")
    }
}

#[cfg(test)]
mod tests {
    use super::fixture::{jamo_speller, speller};

    #[test]
    fn verb_endings_and_particles_through_aliases() {
        for s in [speller(), jamo_speller()] {
            assert!(s.knows("먹었다"));
            assert!(s.knows("먹고"));
            assert!(s.knows("나무는"));
            assert!(s.knows("사람이"));
            assert!(!s.knows("멱었다"));
            assert!(!s.knows("사람는"));
        }
    }

    #[test]
    fn rule_compounds_take_suffixes_on_the_last_part() {
        for s in [speller(), jamo_speller()] {
            assert!(s.knows("사람들"));
            assert!(s.knows("사람들은"));
            assert!(s.knows("새겨져"));
            assert!(s.knows("새겨졌다"));
            // 들 only comes after a word.
            assert!(!s.knows("들은"));
            assert!(!s.knows("사람들는"));
        }
    }

    #[test]
    fn suggestions_are_words() {
        for s in [speller(), jamo_speller()] {
            let found = s.suggest("멱었다");
            assert!(found.iter().any(|w| w == "먹었다"), "{found:?}");
            assert!(found.iter().all(|w| !w.starts_with('\u{F000E}')));
        }
    }
}
