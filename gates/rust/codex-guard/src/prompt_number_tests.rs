//! What the prompt-number guard must deny, and everything it must not.

use super::{decide, parse, patch_targets, reason, Target};
use crate::hook::resolve_path as resolve;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// A repo root holding a `.prompts/` store; returns the root.
fn repo(tag: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("codex-guard-prompts-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".prompts")).unwrap();
    root
}

fn claim(root: &Path, number: &str) {
    let claims = root.join(".prompts").join(".numbers");
    std::fs::create_dir_all(&claims).unwrap();
    std::fs::write(claims.join(number), "").unwrap();
}

fn patch_payload(cwd: &Path, patch: &str) -> Value {
    json!({"session_id": "s1", "cwd": cwd.to_str().unwrap(), "hook_event_name": "PreToolUse",
           "tool_name": "apply_patch", "tool_input": {"command": patch}, "tool_use_id": "call_1"})
}

fn add(path: &str) -> String {
    format!("*** Begin Patch\n*** Add File: {path}\n+# Prompt\n*** End Patch")
}

#[test]
fn a_numbered_prompt_at_the_store_root_is_recognised() {
    let found = parse(Path::new("/repo/.prompts/208-a-numbered-prompt.md")).unwrap();
    assert_eq!(found.number, "208");
    assert_eq!(found.store, Path::new("/repo/.prompts"));
}

#[test]
fn a_short_number_resolves_to_the_padded_claim() {
    assert_eq!(
        parse(Path::new("/repo/.prompts/8-early.md"))
            .unwrap()
            .number,
        "008"
    );
    assert_eq!(
        parse(Path::new("/repo/.prompts/0208-x.md")).unwrap().number,
        "208"
    );
}

#[test]
fn paths_this_guard_has_no_opinion_about() {
    for path in [
        "/repo/.prompts/completed/208-a-numbered-prompt.md",
        "/repo/.prompts/214-mobile-blockers/214-mobile-blockers.md",
        "/repo/.prompts/notes.md",
        "/repo/docs/208-a-numbered-prompt.md",
        "/repo/.prompts/208notahyphen.md",
        "/repo/.prompts/208-a-numbered-prompt.txt",
    ] {
        assert!(
            parse(Path::new(path)).is_none(),
            "{path} should not be claimed ground"
        );
    }
}

#[test]
fn add_and_move_headers_are_targets_and_update_alone_is_not() {
    let patch = "*** Begin Patch\n\
                 *** Add File: .prompts/208-new.md\n+x\n\
                 *** Update File: .prompts/150-old.md\n@@\n-a\n+b\n\
                 *** Update File: .prompts/151-old.md\n*** Move to: .prompts/209-renamed.md\n@@\n-a\n+b\n\
                 *** Delete File: .prompts/100-gone.md\n\
                 *** End Patch";
    assert_eq!(
        patch_targets(patch),
        vec![
            Target {
                path: ".prompts/208-new.md".into(),
                moved_from: None
            },
            Target {
                path: ".prompts/209-renamed.md".into(),
                moved_from: Some(".prompts/151-old.md".into())
            },
        ]
    );
}

#[test]
fn paths_resolve_against_the_cwd_and_fold_dot_segments() {
    assert_eq!(
        resolve("/work/widgets/src", "../.prompts/./208-x.md"),
        PathBuf::from("/work/widgets/.prompts/208-x.md")
    );
    assert_eq!(
        resolve("/work/widgets", "/elsewhere/.prompts/208-x.md"),
        PathBuf::from("/elsewhere/.prompts/208-x.md")
    );
}

#[test]
fn adding_an_unclaimed_number_is_refused_by_relative_path() {
    let root = repo("relative");
    claim(&root, "207");
    let why = decide(&patch_payload(
        &root,
        &add(".prompts/208-a-numbered-prompt.md"),
    ))
    .expect("208 was never claimed");
    assert!(why.contains("208"), "{why}");
    assert!(why.contains("prompt-id alloc <slug>"), "{why}");
    assert!(why.contains("gates/rust/prompt-id"), "{why}");
}

#[test]
fn adding_an_unclaimed_number_is_refused_by_absolute_path() {
    let root = repo("absolute");
    claim(&root, "207");
    let target = root.join(".prompts").join("208-x.md");
    let elsewhere = Path::new("/");
    assert!(decide(&patch_payload(elsewhere, &add(target.to_str().unwrap()))).is_some());
}

#[test]
fn a_claimed_number_is_allowed() {
    let root = repo("claimed");
    claim(&root, "208");
    assert!(decide(&patch_payload(
        &root,
        &add(".prompts/208-a-numbered-prompt.md")
    ))
    .is_none());
}

#[test]
fn a_move_to_an_unclaimed_number_is_refused() {
    let root = repo("move");
    claim(&root, "207");
    std::fs::write(root.join("draft.md"), "x").unwrap();
    let patch = "*** Begin Patch\n*** Update File: draft.md\n*** Move to: .prompts/208-draft.md\n@@\n-x\n+y\n*** End Patch";
    let why = decide(&patch_payload(&root, patch)).expect("a move issues a number too");
    assert!(why.contains("208"), "{why}");
}

#[test]
fn a_rename_that_keeps_the_number_in_the_same_store_is_allowed() {
    // Every prompt written before the allocator existed is unclaimed; renaming
    // one issues no number.
    let root = repo("rename");
    claim(&root, "207");
    std::fs::write(root.join(".prompts").join("150-old-name.md"), "x").unwrap();
    let patch = "*** Begin Patch\n*** Update File: .prompts/150-old-name.md\n*** Move to: .prompts/150-new-name.md\n*** End Patch";
    assert!(decide(&patch_payload(&root, patch)).is_none());
}

#[test]
fn updating_or_adding_over_an_existing_prompt_is_allowed() {
    let root = repo("existing");
    claim(&root, "207");
    std::fs::write(root.join(".prompts").join("208-existing.md"), "x").unwrap();
    let update =
        "*** Begin Patch\n*** Update File: .prompts/208-existing.md\n@@\n-x\n+y\n*** End Patch";
    assert!(decide(&patch_payload(&root, update)).is_none());
    assert!(decide(&patch_payload(&root, &add(".prompts/208-existing.md"))).is_none());
}

#[test]
fn a_store_with_no_claims_directory_is_left_alone() {
    let root = repo("unadopted");
    assert!(decide(&patch_payload(&root, &add(".prompts/001-first-ever.md"))).is_none());
}

#[test]
fn several_unclaimed_files_in_one_patch_are_named_together() {
    let root = repo("several");
    claim(&root, "207");
    let patch = "*** Begin Patch\n\
                 *** Add File: .prompts/208-a.md\n+a\n\
                 *** Add File: .prompts/207-claimed.md\n+b\n\
                 *** Add File: docs/209-not-a-prompt.md\n+c\n\
                 *** Add File: .prompts/210-b.md\n+d\n\
                 *** End Patch";
    let why = decide(&patch_payload(&root, patch)).expect("two unclaimed");
    assert!(why.contains("208, 210 are unallocated"), "{why}");
    assert!(!why.contains("207") && !why.contains("209"), "{why}");
}

#[test]
fn only_apply_patch_is_checked() {
    let root = repo("tool");
    claim(&root, "207");
    let mut payload = patch_payload(&root, &add(".prompts/208-x.md"));
    payload["tool_name"] = json!("Bash");
    assert!(decide(&payload).is_none());
}

#[test]
fn the_refusal_names_only_the_store_and_the_tool() {
    let message = reason(&["208".into()]);
    assert!(message.contains(".prompts/.numbers/208"));
    assert!(message.contains("prompt-id alloc"));
    assert!(!message.contains("/Users"), "no machine path may leak");
}
