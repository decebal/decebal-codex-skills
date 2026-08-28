//! PostToolUse — hand back the comment lines an edit added.
//!
//! Advisory only. It never blocks: a blocking exit surfaces as an error the user
//! has to dismiss, for a finding only the assistant needs to act on.

use crate::hook;
use serde_json::Value;

/// Comment openers per language. `#` opens a comment in shell and Python but
/// names a private field in TypeScript, so the marker set is chosen by
/// extension rather than tried universally.
pub fn markers_for(path: &str) -> Option<&'static [&'static str]> {
    const SLASH: &[&str] = &["//", "/*", "*"];
    const HASH: &[&str] = &["#"];
    const PUG: &[&str] = &["//-", "//"];
    let ext = path.rsplit('.').next()?;
    match ext {
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" => Some(SLASH),
        "rs" | "go" | "java" | "kt" | "swift" => Some(SLASH),
        "c" | "h" | "cpp" | "hpp" | "cc" | "css" | "scss" => Some(SLASH),
        "pug" => Some(PUG),
        "py" | "rb" | "sh" | "bash" | "zsh" => Some(HASH),
        _ => None,
    }
}

/// Doc comments are API surface, not the narration this targets.
const DOC_OPENERS: &[&str] = &["#!", "///", "//!", "/**"];

const MAX_LINES: usize = 40;

/// Comment lines in `text`, capped at [`MAX_LINES`].
///
/// Line-leading markers only, so a URL or a `//` inside a string literal is not
/// flagged. Deliberately unnumbered: an offset into the edit reads as a file
/// line number and is not one.
pub fn comment_lines(text: &str, markers: &[&str]) -> Vec<String> {
    text.lines()
        .filter(|line| {
            let t = line.trim_start();
            markers.iter().any(|m| t.starts_with(m))
                && !DOC_OPENERS.iter().any(|d| t.starts_with(d))
        })
        .take(MAX_LINES)
        .map(str::to_string)
        .collect()
}

/// The message. The three tests themselves live in AGENTS.md, which is already
/// in context — restating them here costs ~130 tokens per firing, and
/// `additionalContext` persists in the transcript, so the same block would be
/// re-billed on every later turn.
pub fn context_for(path: &str, comments: &[String]) -> String {
    format!(
        "comment-hygiene: {} comment line(s) added to {}.\n\n{}\n\nKeep comments that explain a contract, risk, or non-obvious reason. Delete narration that restates the code. Fix this turn.",
        comments.len(),
        path,
        comments.join("\n"),
    )
}

/// Extract comments from files touched by a Codex `apply_patch` command.
///
/// Codex sends the complete patch in `tool_input.command`.
pub fn patch_comments(patch: &str) -> Vec<(String, Vec<String>)> {
    let mut findings = Vec::new();
    let mut path: Option<String> = None;
    let mut added = String::new();

    let flush = |path: &mut Option<String>, added: &mut String, out: &mut Vec<_>| {
        let Some(current) = path.take() else {
            added.clear();
            return;
        };
        if let Some(markers) = markers_for(&current) {
            let comments = comment_lines(added, markers);
            if !comments.is_empty() {
                out.push((current, comments));
            }
        }
        added.clear();
    };

    for line in patch.lines() {
        if let Some(next) = line
            .strip_prefix("*** Update File: ")
            .or_else(|| line.strip_prefix("*** Add File: "))
        {
            flush(&mut path, &mut added, &mut findings);
            path = Some(next.trim().to_string());
            continue;
        }
        if line.starts_with("*** Delete File: ") || line == "*** End Patch" {
            flush(&mut path, &mut added, &mut findings);
            continue;
        }
        if path.is_some() && line.starts_with('+') && !line.starts_with("+++") {
            added.push_str(&line[1..]);
            added.push('\n');
        }
    }
    flush(&mut path, &mut added, &mut findings);
    findings
}

pub fn run(payload: &Value) -> ! {
    if hook::str_at(payload, &["tool_name"]) != "apply_patch" {
        hook::allow();
    }
    let patch = hook::str_at(payload, &["tool_input", "command"]);
    let findings = patch_comments(patch);
    if findings.is_empty() {
        hook::allow();
    }
    let context = findings
        .iter()
        .map(|(path, comments)| context_for(path, comments))
        .collect::<Vec<_>>()
        .join("\n\n");
    hook::additional_context(&context)
}

#[cfg(test)]
#[path = "comment_hygiene_tests.rs"]
mod comment_hygiene_tests;
