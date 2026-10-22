//! Small files that outlive one process: refusal counts (src/review.rs) and,
//! for the `writer` command, when the guide was last read. They live in
//! `WRITER_MCP_STATE_DIR` or the system's temporary folder; losing them only
//! starts over.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// After this long the `writer` command asks for the guide again.
const GUIDE_FRESH_SECS: u64 = 12 * 60 * 60;

pub fn dir() -> PathBuf {
    std::env::var_os("WRITER_MCP_STATE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("writer-mcp"))
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Writes `text` to `path` whole or not at all. Best effort.
pub fn write(path: &Path, text: &str) {
    let Some(parent) = path.parent() else { return };
    let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
    let _ = std::fs::create_dir_all(parent)
        .and_then(|()| std::fs::write(&tmp, text))
        .and_then(|()| std::fs::rename(&tmp, path));
}

fn guide_file() -> PathBuf {
    dir().join("guide-read")
}

/// Notes that the guide was just read from the command line.
pub fn remember_guide() {
    write(&guide_file(), &now().to_string());
}

/// The guide was read from the command line in the last twelve hours.
pub fn guide_fresh() -> bool {
    std::fs::read_to_string(guide_file())
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .is_some_and(|at| now().saturating_sub(at) < GUIDE_FRESH_SECS)
}
