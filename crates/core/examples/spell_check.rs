//! Measures the spell checker with a real dictionary (docs/spellcheck-license.md).
//!
//! ```text
//! cargo run --release -p writer-core --example spell_check -- <dictionary folder> <text file> [reference]
//! ```
//!
//! The folder holds `ko.aff` and `ko.dic` (hunspell-dict-ko, fetched by the
//! app or by hand; never kept in this repository). Prints how long loading
//! takes, words checked per second and the words flagged. With a reference
//! file of `word true|false` lines (Hunspell's own answers), prints where the
//! two differ.

use std::time::Instant;

use writer_core::spell::{Known, Speller, check_text};

fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(dir), Some(text_file)) = (args.next(), args.next()) else {
        eprintln!("usage: spell_check <dictionary folder> <text file> [reference]");
        std::process::exit(2);
    };
    let started = Instant::now();
    let speller = Speller::open(std::path::Path::new(&dir)).expect("dictionary");
    println!("load: {:?}", started.elapsed());

    let text = std::fs::read_to_string(&text_file).expect("text file");
    let known = Known::default();
    let words = text.split_whitespace().count();
    let started = Instant::now();
    let mut flagged = Vec::new();
    for line in text.lines() {
        flagged.extend(
            check_text(&speller, &known, line)
                .into_iter()
                .map(|i| i.word),
        );
    }
    let first = started.elapsed();
    let started = Instant::now();
    for line in text.lines() {
        check_text(&speller, &known, line);
    }
    let again = started.elapsed();
    println!(
        "{words} words: {first:?} ({:.0} words/s), again with the word cache {again:?}",
        words as f64 / first.as_secs_f64()
    );
    flagged.sort();
    flagged.dedup();
    println!("flagged {}: {}", flagged.len(), flagged.join(" "));
    for word in flagged.iter().take(10) {
        let started = Instant::now();
        let found = speller.suggest(word);
        println!("  {word} → {} ({:?})", found.join(", "), started.elapsed());
    }

    if let Some(reference) = args.next() {
        let reference = std::fs::read_to_string(reference).expect("reference");
        let (mut stricter, mut looser) = (Vec::new(), Vec::new());
        for line in reference.lines() {
            let mut parts = line.split(' ');
            let (Some(word), Some(answer)) = (parts.next(), parts.next()) else {
                continue;
            };
            let hunspell = answer == "true";
            match (speller.knows(word), hunspell) {
                (false, true) => stricter.push(word),
                (true, false) => looser.push(word),
                _ => {}
            }
        }
        println!(
            "rejected here, accepted by Hunspell: {} {:?}",
            stricter.len(),
            stricter
        );
        println!(
            "accepted here, rejected by Hunspell: {} {:?}",
            looser.len(),
            looser
        );
    }
}
