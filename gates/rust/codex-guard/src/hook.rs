//! The hook envelope: what arrives on stdin, and what a decision looks like on
//! stdout.
//!
//! Every subcommand reads one JSON object from stdin and writes at most one JSON
//! object to stdout. Silence is an allow — the common case, and the cheapest,
//! because a hook that prints nothing costs the model nothing.

use serde_json::{json, Value};
use std::io::Read;
use std::path::{Component, Path, PathBuf};

/// Read and parse the hook payload.
///
/// A payload that will not parse is `None` rather than an error: a guard that
/// crashes on a surprising field shape would fail CLOSED on every tool call, and
/// the blast radius of that is worse than the miss.
pub fn read_payload() -> Option<Value> {
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf).ok()?;
    serde_json::from_str(&buf).ok()
}

/// A string field, or `""`. Mirrors `jq -r '.a.b // ""'`.
pub fn str_at<'a>(v: &'a Value, path: &[&str]) -> &'a str {
    let mut cur = v;
    for key in path {
        match cur.get(key) {
            Some(next) => cur = next,
            None => return "",
        }
    }
    cur.as_str().unwrap_or("")
}

/// `path` against the payload's `cwd`, with `.` and `..` folded lexically: a
/// file a patch adds does not exist yet, so it cannot be canonicalised.
pub fn resolve_path(cwd: &str, path: &str) -> PathBuf {
    let joined = if Path::new(path).is_absolute() || cwd.is_empty() {
        PathBuf::from(path)
    } else {
        Path::new(cwd).join(path)
    };
    let mut out = PathBuf::new();
    for part in joined.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

/// `deny` — the tool call does not run, and the reason reaches the model.
pub fn deny(reason: &str) -> ! {
    emit_decision("deny", reason);
    std::process::exit(0)
}

fn emit_decision(decision: &str, reason: &str) {
    let out = json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": decision,
            "permissionDecisionReason": reason,
        }
    });
    println!("{out}");
}

/// Replace the tool input, keeping every sibling field.
///
/// The whole `tool_input` object is replaced, so a rewrite that rebuilt it from
/// scratch would silently drop `description`, `timeout`, and anything else the
/// caller set. Clone and patch, never rebuild.
pub fn rewrite_command(tool_input: &Value, command: &str) -> ! {
    let mut input = tool_input.clone();
    if let Some(obj) = input.as_object_mut() {
        obj.insert("command".into(), json!(command));
    }
    let out = json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "allow",
            "updatedInput": input,
        }
    });
    println!("{out}");
    std::process::exit(0)
}

/// Advisory context for a PostToolUse hook.
///
/// Never a blocking exit: a block surfaces as an error the USER has to dismiss,
/// for a finding only the assistant needs to act on.
pub fn additional_context(context: &str) -> ! {
    println!("{}", context_output("PostToolUse", context));
    std::process::exit(0)
}

/// Model context for a `PreToolUse` hook, with no decision: the call runs.
///
/// Codex marks a hook failed for `permissionDecision: "allow"` without
/// `updatedInput`, so the key must be absent rather than set to allow.
pub fn pre_tool_use_context(context: &str) -> ! {
    println!("{}", context_output("PreToolUse", context));
    std::process::exit(0)
}

pub fn context_output(event: &str, context: &str) -> Value {
    json!({
        "hookSpecificOutput": {
            "hookEventName": event,
            "additionalContext": context,
        }
    })
}

/// Nothing to say. Exit 0, print nothing.
pub fn allow() -> ! {
    std::process::exit(0)
}

/// Block a Bash call, with the remediation on stderr.
///
/// Exit 2 is what makes stderr reach the model rather than the user, so every
/// line here is billed to the context window — which is why the callers pick
/// which remediation lines to print rather than always printing all of them.
pub fn block(message: &str) -> ! {
    eprint!("{message}");
    std::process::exit(2)
}
