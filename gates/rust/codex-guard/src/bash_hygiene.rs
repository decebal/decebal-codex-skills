//! PreToolUse — one command per Bash call.
//!
//! Blocks compound commands, command substitution and combined redirects, and
//! rewrites the one form that is repairable without changing what runs.
//!
//! Quote-stripping is non-nested and ignores heredoc bodies and escaped quotes.
//! Do not grow it into a parser: the failure mode of this approximation is a
//! false NEGATIVE — a violation that slips through — never a bad block.

use crate::hook;
use serde_json::Value;

/// What a command violates, in the order the remediation is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// Two commands where there should be one.
    Split,
    /// An inner command that should be its own call.
    Inner,
    /// Output capture the harness already does better.
    Capture,
    /// A temp path outside the session's scratch directory.
    Scratch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    pub label: &'static str,
    pub class: Class,
}

/// Strip single- then double-quoted regions, so a `;` inside a commit message
/// is data rather than a separator. Non-nested by design.
pub fn strip_quoted(cmd: &str) -> String {
    let once = strip_pairs(cmd, '\'');
    strip_pairs(&once, '"')
}

fn strip_pairs(s: &str, quote: char) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(open) = rest.find(quote) {
        match rest[open + 1..].find(quote) {
            Some(rel_close) => {
                out.push_str(&rest[..open]);
                rest = &rest[open + 1 + rel_close + 1..];
            }
            // An unpaired quote: keep the remainder verbatim rather than
            // swallowing it, or an unterminated string would hide everything
            // after it from every check below.
            None => break,
        }
    }
    out.push_str(rest);
    out
}

/// Every violation in a command, in remediation order.
///
/// `scratch_dir` opts in the temp-path rule: unset means no opinion on where
/// temp files go.
pub fn violations(cmd: &str, scratch_dir: Option<&str>) -> Vec<Violation> {
    let s = strip_quoted(cmd);
    let mut v = Vec::new();
    let mut push = |label: &'static str, class: Class| v.push(Violation { label, class });

    if s.contains("&&") {
        push("'&&' chain", Class::Split);
    }
    if s.contains("||") {
        push("'||' chain", Class::Split);
    }
    if s.contains(';') {
        push("';' separator", Class::Split);
    }
    // A newline separates two commands exactly as `;` does, and the permission
    // matcher treats it the same way. A heredoc body is one command's data, so
    // a command carrying one is exempt.
    if !s.contains("<<") && s.contains('\n') {
        push("newline separator", Class::Split);
    }
    if s.contains("$(") {
        push("$() substitution", Class::Inner);
    }
    if s.contains("<(") || s.contains(">(") {
        push("<() process substitution", Class::Inner);
    }
    if s.contains('`') {
        push("backtick substitution", Class::Inner);
    }
    if s.contains("| tee") || s.contains("|tee") {
        push("'| tee' output capture", Class::Capture);
    }
    if s.contains("&>") {
        push("'&>' combined redirect", Class::Capture);
    }
    if s.contains("2>&1") {
        push("'2>&1' stderr merge", Class::Capture);
    }

    if scratch_dir.is_some() {
        // Keep the boundaries — a bare `/tmp` match hits `/tmpfs/` and `mytmp/`.
        for path in ["/tmp", "/private/tmp"] {
            if has_bare_path(&s, path) {
                push(
                    if path == "/tmp" {
                        "'/tmp/' temp path"
                    } else {
                        "'/private/tmp/' temp path"
                    },
                    Class::Scratch,
                );
            }
        }
    }
    v
}

/// `path` appearing as a whole token or directory prefix, not as a substring.
fn has_bare_path(s: &str, path: &str) -> bool {
    let bytes = s.as_bytes();
    let mut from = 0;
    while let Some(rel) = s[from..].find(path) {
        let at = from + rel;
        let before_ok = at == 0 || matches!(bytes[at - 1], b' ' | b'\t' | b'=');
        let after = at + path.len();
        let after_ok = after >= bytes.len() || matches!(bytes[after], b'/' | b' ' | b'\t' | b'\n');
        if before_ok && after_ok {
            return true;
        }
        from = at + 1;
    }
    false
}

/// A trailing `2>&1` with no other redirect is repairable without changing what
/// runs — the harness captures both streams anyway — so rewrite rather than
/// spend a round trip.
///
/// Piped or otherwise redirected forms are left alone: there the merge decides
/// which stream the next stage reads, and dropping it would change behaviour.
pub fn repairable_rewrite(cmd: &str, v: &[Violation]) -> Option<String> {
    if v.len() != 1 || v[0].class != Class::Capture {
        return None;
    }
    let trimmed = cmd.trim_end();
    if !trimmed.ends_with("2>&1") {
        return None;
    }
    let head = &trimmed[..trimmed.len() - 4];
    if !head.ends_with(' ') && !head.ends_with('\t') {
        return None;
    }
    if cmd.matches('>').count() != 1 {
        return None;
    }
    Some(head.trim_end().to_string())
}

/// The remediation, carrying only the lines the violations earned.
///
/// Explaining all four classes when one fired is ~70 wasted tokens, and this
/// text is billed to the model.
pub fn remediation(v: &[Violation], scratch_dir: Option<&str>) -> String {
    let labels: Vec<&str> = v.iter().map(|x| x.label).collect();
    let mut out = format!("Blocked by bash-hygiene: {}\n", labels.join(" "));
    let has = |c: Class| v.iter().any(|x| x.class == c);
    if has(Class::Split) {
        out.push_str(
            "Split into separate Bash calls, or use a tool flag (-C, --cwd, --manifest-path, -p).\n",
        );
    }
    if has(Class::Inner) {
        out.push_str("Run the inner command as its own Bash call and use its result.\n");
    }
    if has(Class::Capture) {
        out.push_str("Remove redundant capture — Codex keeps command output from both streams.\n");
    }
    if has(Class::Scratch) {
        if let Some(dir) = scratch_dir {
            out.push_str(&format!("Use {dir} for scratch files.\n"));
        }
    }
    out
}

pub fn run(payload: &Value) -> ! {
    if hook::str_at(payload, &["tool_name"]) != "Bash" {
        hook::allow();
    }
    let cmd = hook::str_at(payload, &["tool_input", "command"]);
    if cmd.is_empty() {
        hook::allow();
    }
    let scratch = std::env::var("CODEX_SCRATCH_DIR")
        .ok()
        .filter(|s| !s.is_empty());
    let v = violations(cmd, scratch.as_deref());
    if v.is_empty() {
        hook::allow();
    }
    if let Some(fixed) = repairable_rewrite(cmd, &v) {
        let empty = Value::Null;
        let input = payload.get("tool_input").unwrap_or(&empty);
        hook::rewrite_command(input, &fixed);
    }
    hook::block(&remediation(&v, scratch.as_deref()))
}

#[cfg(test)]
#[path = "bash_hygiene_tests.rs"]
mod bash_hygiene_tests;
