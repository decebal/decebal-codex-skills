//! Per-session state files under `$CODEX_HOME`, shared by the hooks that must
//! remember something between tool calls of one session.
//!
//! Every state file is named after the payload's `session_id`, so a stale one
//! is never read again once its session ends. They are pruned when a new one is
//! created, never on the hot path of an existing session.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// A state file untouched for this long belongs to a session nobody resumes.
pub const MAX_AGE: Duration = Duration::from_secs(14 * 24 * 60 * 60);

/// Directory entries examined per prune, so a directory that grew without bound
/// cannot make one hook firing slow.
pub const PRUNE_SCAN_LIMIT: usize = 500;

/// Longer ids are cut: the name must stay under every filesystem's limit.
const MAX_NAME: usize = 128;

/// `$CODEX_HOME`, else `$HOME/.codex`; `None` when neither is set.
pub fn codex_home() -> Option<PathBuf> {
    codex_home_from(std::env::var_os("CODEX_HOME"), std::env::var_os("HOME"))
}

pub fn codex_home_from(codex_home: Option<OsString>, home: Option<OsString>) -> Option<PathBuf> {
    if let Some(dir) = codex_home.filter(|d| !d.is_empty()) {
        return Some(PathBuf::from(dir));
    }
    home.filter(|h| !h.is_empty())
        .map(|h| PathBuf::from(h).join(".codex"))
}

/// `session_id` reduced to `[A-Za-z0-9_-]`, or `None` when nothing is left.
/// The id arrives in the payload, so it never reaches a path unsanitised.
pub fn file_name(session_id: &str) -> Option<String> {
    let name: String = session_id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .take(MAX_NAME)
        .collect();
    (!name.is_empty()).then_some(name)
}

/// Delete regular files in `dir` last modified before `now - MAX_AGE`, looking
/// at no more than [`PRUNE_SCAN_LIMIT`] entries. Every error is ignored: pruning
/// is housekeeping, and a hook must not fail over it.
pub fn prune_stale(dir: &Path, now: SystemTime) {
    let Some(cutoff) = now.checked_sub(MAX_AGE) else {
        return;
    };
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.take(PRUNE_SCAN_LIMIT).flatten() {
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        let stale = meta.is_file() && meta.modified().is_ok_and(|m| m < cutoff);
        if stale {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// Create `path`'s directory, pruning it first when `path` does not exist yet.
/// Returns whether the file is new.
pub fn prepare(path: &Path, now: SystemTime) -> bool {
    let fresh = !path.exists();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
        if fresh {
            prune_stale(dir, now);
        }
    }
    fresh
}

#[cfg(test)]
pub fn set_mtime(path: &Path, when: SystemTime) {
    use std::fs::File;
    File::options()
        .write(true)
        .open(path)
        .and_then(|f| f.set_modified(when))
        .expect("set mtime");
}

#[cfg(test)]
#[path = "session_state_tests.rs"]
mod session_state_tests;
