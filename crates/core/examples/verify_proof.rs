//! Checks a creation proof file (증명자료.json) made with a 창작 과정
//! 증명서: the journals' hash chains, each time stamp's signature and root,
//! and, when manuscript files are given, whether their fingerprints are in
//! the record (docs/creation-proof.md §6). Works offline.
//!
//! ```bash
//! cargo run -p writer-core --example verify_proof -- 증명자료.json [manuscript files …]
//! ```
//!
//! Exits with 0 when everything checks out, 1 when something does not, 2
//! when a file cannot be read.

use std::path::Path;
use std::process::ExitCode;

use writer_core::proof;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(bundle_path) = args.first() else {
        eprintln!("쓰는 법: verify_proof <증명자료.json> [원고 파일 …]");
        return ExitCode::from(2);
    };
    let bundle = match std::fs::read(bundle_path) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("{bundle_path}을(를) 읽을 수 없습니다: {e}");
            return ExitCode::from(2);
        }
    };
    let mut files = Vec::new();
    for path in &args[1..] {
        match std::fs::read(path) {
            Ok(bytes) => {
                let name = Path::new(path)
                    .file_name()
                    .map_or_else(|| path.clone(), |n| n.to_string_lossy().into_owned());
                files.push((name, bytes));
            }
            Err(e) => {
                eprintln!("{path}을(를) 읽을 수 없습니다: {e}");
                return ExitCode::from(2);
            }
        }
    }
    let verdict = proof::verify(&bundle, &files);
    print!("{}", verdict.text());
    println!("\n{}", proof::NOT_ABOUT_AI);
    if verdict.ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
