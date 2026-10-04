//! Reworks the Korean Hunspell dictionary in memory so `spellbook` reads it
//! as Hunspell does. The files on disk are never changed.
//!
//! Two differences matter for hunspell-dict-ko:
//!
//! 1. Flag aliases (`AF`) with numeric flags (`FLAG num`). Hunspell looks the
//!    number after a word's `/` up in the `AF` table whatever the flag type;
//!    spellbook 0.4 reads it as a flag itself when flags are numeric, so no
//!    verb ending or particle is found ("먹었다", "사랑을" rejected). Here
//!    every alias is written out in full and the `AF` table dropped.
//! 2. Compound rules (`COMPOUNDRULE`). Hunspell lets the last word of a
//!    rule compound carry suffixes ("사람" + "들은", "새겨" + "져");
//!    spellbook only matches bare dictionary words. So the rules are matched
//!    here instead (`super::compound`): the words that may start or sit
//!    inside a rule compound are kept in a table with their rule flags, and
//!    each word that may end one is added to the dictionary a second time
//!    behind a marker character standing for its flag ("\u{F000E}들" for
//!    flag 14), without its compound-only flag, so spellbook checks its
//!    suffixed forms ("\u{F000E}들은").

use std::collections::{BTreeSet, HashMap};

use super::hangul::to_jamo;

/// One part of a compound rule: a flag, and how often it may come.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Repeat {
    Once,
    /// `?`
    Maybe,
    /// `*`
    Any,
}

pub type Rule = Vec<(u32, Repeat)>;

/// The dictionary text for spellbook and what compound matching needs.
pub struct Prepared {
    pub aff: String,
    pub dic: String,
    pub rules: Vec<Rule>,
    /// Bare words (Hangul written in letters, `hangul::to_jamo`) with the
    /// rule flags they carry.
    pub heads: HashMap<String, Vec<u32>>,
}

/// The marker put before a word that may end a rule compound with `flag`.
pub fn tail_marker(flag: u32) -> char {
    char::from_u32(0xF0000 + (flag & 0xFFFF)).unwrap_or('\u{F0000}')
}

pub fn prepare(aff: &str, dic: &str) -> Prepared {
    let aliases = aliases(aff);
    let expand = |flags: &str| -> String {
        match flags.trim().parse::<usize>() {
            Ok(i) if i >= 1 && i <= aliases.len() => aliases[i - 1].clone(),
            _ => flags.to_string(),
        }
    };
    let numeric = directive(aff, "FLAG") == Some("num");

    let mut aff_out = String::with_capacity(aff.len());
    for line in aff.lines() {
        if line.starts_with("AF ") {
            continue;
        }
        if !aliases.is_empty() && (line.starts_with("SFX ") || line.starts_with("PFX ")) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 5
                && let Some((add, flags)) = parts[3].split_once('/')
            {
                let mut owned: Vec<String> = parts.iter().map(|s| (*s).to_string()).collect();
                owned[3] = format!("{add}/{}", expand(flags));
                aff_out.push_str(&owned.join(" "));
                aff_out.push('\n');
                continue;
            }
        }
        aff_out.push_str(line);
        aff_out.push('\n');
    }

    let mut entries: Vec<(String, String)> = Vec::new();
    for (i, line) in dic.lines().enumerate() {
        if i == 0 && line.trim().parse::<usize>().is_ok() {
            // The word count; written again below with the added words.
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }
        let (word, flags) = split_entry(line);
        let flags = flags.map(&expand).unwrap_or_default();
        entries.push((word.to_string(), flags));
    }

    let rules = if numeric { rules(aff) } else { Vec::new() };
    let mut heads: HashMap<String, Vec<u32>> = HashMap::new();
    let mut tails: Vec<(String, String)> = Vec::new();
    if !rules.is_empty() {
        let in_rules: BTreeSet<u32> = rules.iter().flatten().map(|(f, _)| *f).collect();
        let ends = rule_ends(&rules);
        let only_in_compound: Option<u32> =
            directive(aff, "ONLYINCOMPOUND").and_then(|f| f.parse().ok());
        let forbidden: Option<u32> = directive(aff, "FORBIDDENWORD").and_then(|f| f.parse().ok());
        for (word, flags) in &entries {
            let flags: Vec<u32> = flags
                .split(',')
                .filter_map(|f| f.trim().parse().ok())
                .collect();
            if forbidden.is_some_and(|f| flags.contains(&f)) {
                continue;
            }
            let ruled: Vec<u32> = flags
                .iter()
                .copied()
                .filter(|f| in_rules.contains(f))
                .collect();
            if ruled.is_empty() {
                continue;
            }
            heads.entry(to_jamo(word)).or_default().extend(&ruled);
            let rest: Vec<String> = flags
                .iter()
                .filter(|f| !in_rules.contains(f) && Some(**f) != only_in_compound)
                .map(|f| f.to_string())
                .collect();
            for flag in ruled.iter().filter(|f| ends.contains(f)) {
                tails.push((format!("{}{word}", tail_marker(*flag)), rest.join(",")));
            }
        }
    }

    let mut dic_out = String::with_capacity(dic.len() + tails.len() * 40);
    dic_out.push_str(&(entries.len() + tails.len()).to_string());
    dic_out.push('\n');
    for (word, flags) in entries.iter().chain(tails.iter()) {
        dic_out.push_str(word);
        if !flags.is_empty() {
            dic_out.push('/');
            dic_out.push_str(flags);
        }
        dic_out.push('\n');
    }
    Prepared {
        aff: aff_out,
        dic: dic_out,
        rules,
        heads,
    }
}

/// The value of a one-line directive such as `FLAG num`.
fn directive<'a>(aff: &'a str, name: &str) -> Option<&'a str> {
    aff.lines().find_map(|l| {
        let mut p = l.split_whitespace();
        (p.next() == Some(name)).then(|| p.next()).flatten()
    })
}

/// The `AF` aliases in order (alias 1 first). The first `AF` line gives
/// their count, so a lone number further down ("AF 15") is an alias.
fn aliases(aff: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut count: Option<usize> = None;
    for line in aff.lines() {
        let Some(rest) = line.strip_prefix("AF ") else {
            continue;
        };
        match count {
            None => count = Some(rest.trim().parse().unwrap_or(0)),
            Some(n) if out.len() < n => out.push(rest.trim().to_string()),
            Some(_) => {}
        }
    }
    out
}

/// A `.dic` line split into word and flags. A `/` escaped with `\` belongs
/// to the word; anything after a tab or a space (morphology) is dropped.
fn split_entry(line: &str) -> (&str, Option<&str>) {
    let line = line.split(['\t', ' ']).next().unwrap_or(line);
    let bytes = line.as_bytes();
    for (i, b) in bytes.iter().enumerate() {
        if *b == b'/' && (i == 0 || bytes[i - 1] != b'\\') {
            return (&line[..i], Some(&line[i + 1..]));
        }
    }
    (line, None)
}

/// Compound rules written with numeric flags: `(12)*(12)(13)`.
fn rules(aff: &str) -> Vec<Rule> {
    let mut out = Vec::new();
    for line in aff.lines() {
        let mut parts = line.split_whitespace();
        if parts.next() != Some("COMPOUNDRULE") {
            continue;
        }
        let Some(text) = parts.next() else { continue };
        if !text.starts_with('(') {
            // The count line, or a rule in another flag style.
            continue;
        }
        let mut rule = Rule::new();
        let mut rest = text;
        while let Some(open) = rest.find('(') {
            let Some(close) = rest[open..].find(')') else {
                break;
            };
            let Ok(flag) = rest[open + 1..open + close].parse() else {
                break;
            };
            rest = &rest[open + close + 1..];
            let repeat = if rest.starts_with('?') {
                Repeat::Maybe
            } else if rest.starts_with('*') {
                Repeat::Any
            } else {
                Repeat::Once
            };
            rule.push((flag, repeat));
        }
        if !rule.is_empty() {
            out.push(rule);
        }
    }
    out
}

/// Flags that can stand last in a rule.
fn rule_ends(rules: &[Rule]) -> BTreeSet<u32> {
    let mut out = BTreeSet::new();
    for rule in rules {
        for (flag, repeat) in rule.iter().rev() {
            out.insert(*flag);
            if *repeat == Repeat::Once {
                break;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliases_are_written_out() {
        let aff = "FLAG num\nAF 2\nAF 3,4\nAF 5\nSFX 3 Y 1\nSFX 3 0 s/2 .\n";
        let dic = "2\ncat/1\ndog/2\n";
        let p = prepare(aff, dic);
        assert!(!p.aff.contains("AF "));
        assert!(p.aff.contains("SFX 3 0 s/5 ."));
        assert_eq!(p.dic, "2\ncat/3,4\ndog/5\n");
    }

    #[test]
    fn a_lone_alias_line_is_not_taken_for_the_count() {
        let aff = "FLAG num\nAF 3\nAF 1,2\nAF 15\nAF 7\n";
        assert_eq!(aliases(aff), vec!["1,2", "15", "7"]);
    }

    #[test]
    fn rules_and_their_ends() {
        let aff = "COMPOUNDRULE 3\nCOMPOUNDRULE (12)*(12)(13)\nCOMPOUNDRULE (26)?(25)?(24)?\nCOMPOUNDRULE (16)*(14)?\n";
        let rules = rules(aff);
        assert_eq!(
            rules[0],
            vec![(12, Repeat::Any), (12, Repeat::Once), (13, Repeat::Once)]
        );
        let ends: Vec<u32> = rule_ends(&rules).into_iter().collect();
        assert_eq!(ends, vec![13, 14, 16, 24, 25, 26]);
    }

    #[test]
    fn compound_words_are_kept_for_matching() {
        let aff = "FLAG num\nONLYINCOMPOUND 1\nCOMPOUNDRULE 1\nCOMPOUNDRULE (9)(14)\n\
                   SFX 30 Y 1\nSFX 30 0 i .\n";
        let dic = "3\nman/9\nul/1,14,30\nfree/30\n";
        let p = prepare(aff, dic);
        assert_eq!(p.heads.get("man"), Some(&vec![9]));
        assert_eq!(p.heads.get("ul"), Some(&vec![14]));
        assert!(!p.heads.contains_key("free"));
        // The tail copy keeps its suffixes but not "only in compounds".
        assert!(p.dic.contains(&format!("\n{}ul/30\n", tail_marker(14))));
        assert!(p.dic.starts_with("4\n"));
    }
}
