//! Per-session state: the file name a session id may become, and pruning.

use super::{codex_home_from, file_name, prepare, prune_stale, set_mtime, MAX_AGE};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("codex-guard-state-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn aged(path: &Path, age: Duration) {
    std::fs::write(path, "x").unwrap();
    set_mtime(path, SystemTime::now() - age);
}

#[test]
fn a_session_id_keeps_only_path_safe_characters() {
    assert_eq!(
        file_name("0199a1b2-7c3d-4e5f-8a9b-0c1d2e3f4a5b").as_deref(),
        Some("0199a1b2-7c3d-4e5f-8a9b-0c1d2e3f4a5b")
    );
    assert_eq!(file_name("../../etc/passwd").as_deref(), Some("etcpasswd"));
    assert_eq!(file_name("a b/c\\d_e").as_deref(), Some("abcd_e"));
    assert_eq!(file_name("../"), None);
    assert_eq!(file_name(""), None);
    assert_eq!(file_name(&"x".repeat(500)).map(|n| n.len()), Some(128));
}

#[test]
fn codex_home_prefers_the_env_var_then_falls_back_to_home() {
    assert_eq!(
        codex_home_from(Some("/opt/codex".into()), Some("/home/u".into())),
        Some(PathBuf::from("/opt/codex"))
    );
    assert_eq!(
        codex_home_from(Some("".into()), Some("/home/u".into())),
        Some(PathBuf::from("/home/u/.codex"))
    );
    assert_eq!(codex_home_from(None, None), None);
}

#[test]
fn pruning_removes_only_files_past_the_age_limit() {
    let dir = scratch("prune");
    let old = dir.join("old.jsonl");
    let fresh = dir.join("fresh.jsonl");
    aged(&old, MAX_AGE + Duration::from_secs(3600));
    aged(&fresh, Duration::from_secs(60));
    std::fs::create_dir_all(dir.join("a-directory")).unwrap();

    prune_stale(&dir, SystemTime::now());

    assert!(!old.exists());
    assert!(fresh.exists());
    assert!(dir.join("a-directory").exists());
}

#[test]
fn only_a_new_state_file_prunes_its_directory() {
    let dir = scratch("prepare");
    let old = dir.join("old");
    aged(&old, MAX_AGE + Duration::from_secs(3600));
    let existing = dir.join("existing");
    std::fs::write(&existing, "").unwrap();

    assert!(!prepare(&existing, SystemTime::now()));
    assert!(old.exists(), "an existing session's call must not prune");

    assert!(prepare(&dir.join("new"), SystemTime::now()));
    assert!(!old.exists());
}

#[test]
fn preparing_a_path_creates_its_directory() {
    let dir = scratch("mkdir").join("nested").join("deeper");
    assert!(prepare(&dir.join("s1"), SystemTime::now()));
    assert!(dir.is_dir());
}
