//! PreToolUse, apply_patch — refuse to create a numbered prompt whose number
//! was never claimed.
//!
//! `.prompts/` is gitignored, so every worktree holds its own near-empty copy. A
//! session that picks a number by looking at the directory it is standing in
//! sees almost nothing and lands on one the main checkout already issued.
//! `prompt-id alloc` exists to remove that: it takes a lock on the NUMBER, under
//! `<store>/.numbers/`, before anything is written.
//!
//! Codex sends the whole patch in `tool_input.command`. Every `*** Add File:` and
//! `*** Move to:` target is checked; `*** Update File:` never is, because revising
//! a prompt issues no number.
//!
//! Fails open in every direction that is not the exact damage: a target that
//! already exists, a store with no `.numbers/` at all (a repo that never adopted
//! the allocator), a path that is not a direct child of `.prompts/`, a rename
//! that keeps the number of a prompt already in the same store, an unreadable
//! payload.

use crate::hook;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// A prompt path this guard has an opinion about: the store it belongs to, and
/// the number it carries.
pub struct Numbered {
    pub store: PathBuf,
    pub number: String,
}

/// The store and number a path carries, when it is a numbered prompt at the top
/// level of a `.prompts/` store.
///
/// Only a DIRECT child qualifies. `.prompts/completed/NNN-x.md` is an archive
/// move, and `.prompts/NNN-topic/NNN-topic.md` is a stage inside a directory
/// whose number was already claimed when the directory was made; neither is a
/// new allocation.
pub fn parse(path: &Path) -> Option<Numbered> {
    let name = path.file_name()?.to_str()?;
    if !name.ends_with(".md") {
        return None;
    }
    let store = path.parent()?;
    if store.file_name()?.to_str()? != ".prompts" {
        return None;
    }
    let number = leading_number(name)?;
    Some(Numbered {
        store: store.to_path_buf(),
        number,
    })
}

/// `NNN` from `NNN-rest`, zero-padded to three to match how the claim is
/// written, so `0208` and `208` resolve to the same claim rather than to two.
fn leading_number(name: &str) -> Option<String> {
    let digits: String = name.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() || !name[digits.len()..].starts_with('-') {
        return None;
    }
    let number = digits.parse::<u32>().ok()?;
    Some(format!("{number:03}"))
}

/// One file the patch writes a new name for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub path: String,
    /// The `*** Update File:` path a `*** Move to:` renames.
    pub moved_from: Option<String>,
}

/// Every `*** Add File:` and `*** Move to:` path in a patch, in order.
pub fn patch_targets(patch: &str) -> Vec<Target> {
    let mut targets = Vec::new();
    let mut updating: Option<&str> = None;
    for line in patch.lines() {
        if let Some(path) = line.strip_prefix("*** Add File: ") {
            updating = None;
            targets.push(Target {
                path: path.trim().to_string(),
                moved_from: None,
            });
        } else if let Some(path) = line.strip_prefix("*** Update File: ") {
            updating = Some(path.trim());
        } else if let Some(path) = line.strip_prefix("*** Move to: ") {
            targets.push(Target {
                path: path.trim().to_string(),
                moved_from: updating.map(str::to_string),
            });
        } else if line.starts_with("*** Delete File: ") || line == "*** End Patch" {
            updating = None;
        }
    }
    targets
}

/// The unclaimed number `target` would issue, or `None` to allow it.
pub fn unclaimed(cwd: &str, target: &Target) -> Option<String> {
    let path = hook::resolve_path(cwd, &target.path);
    let found = parse(&path)?;
    if path.exists() {
        return None;
    }
    let claims = found.store.join(".numbers");
    if !claims.is_dir() || claims.join(&found.number).exists() {
        return None;
    }
    let keeps_number = target.moved_from.as_deref().is_some_and(|from| {
        let from = hook::resolve_path(cwd, from);
        let name = from.file_name().and_then(|n| n.to_str()).unwrap_or("");
        from.starts_with(&found.store) && leading_number(name).as_ref() == Some(&found.number)
    });
    (!keeps_number).then_some(found.number)
}

/// The message. It names the missing claim and the one command that fixes it:
/// a refusal that does not say what to run instead just gets worked around.
///
/// This is a `deny` reason, so every byte is billed to the model's context.
/// Held under a ceiling by `payload_size_tests.rs`.
pub fn reason(numbers: &[String]) -> String {
    let claim = match numbers {
        [one] => format!(
            "{one} is unallocated: .prompts/.numbers/{one} does not exist, so another session can still be handed it"
        ),
        many => format!(
            "{} are unallocated: none has a claim in .prompts/.numbers/, so another session can still be handed them",
            many.join(", ")
        ),
    };
    format!(
        "prompt-id: {claim}.\n\
         Allocate with `prompt-id alloc <slug>` (the prompt-id tool from \
         github.com/decebal/decebal-claude-skills, gates/rust/prompt-id) and write to the path it prints."
    )
}

/// The deny reason for a payload, or `None` to allow it.
pub fn decide(payload: &Value) -> Option<String> {
    if hook::str_at(payload, &["tool_name"]) != "apply_patch" {
        return None;
    }
    let cwd = hook::str_at(payload, &["cwd"]);
    let mut numbers: Vec<String> = Vec::new();
    for target in patch_targets(hook::str_at(payload, &["tool_input", "command"])) {
        if let Some(number) = unclaimed(cwd, &target) {
            if !numbers.contains(&number) {
                numbers.push(number);
            }
        }
    }
    (!numbers.is_empty()).then(|| reason(&numbers))
}

pub fn run(payload: &Value) -> ! {
    match decide(payload) {
        Some(message) => hook::deny(&message),
        None => hook::allow(),
    }
}

#[cfg(test)]
#[path = "prompt_number_tests.rs"]
mod prompt_number_tests;
