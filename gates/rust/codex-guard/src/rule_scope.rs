//! PreToolUse, apply_patch and Bash — deliver a path-scoped rule the first time
//! a session touches a file the rule covers.
//!
//! Codex has no path-scoped instructions: AGENTS.md loads whole or not at all.
//! A rule file whose YAML frontmatter carries `paths:` globs is instead handed to
//! the model as `additionalContext` on the first call of a session that touches
//! a matching path, and not again in that session until the history is
//! compacted: registered on `PostCompact`, the same subcommand clears the
//! session's record, because a compaction summarises the delivered text away.
//!
//! Rule sources, each a directory of `*.md`, read in this order:
//!
//! - `$CODEX_HOME/agent-rules/`, the global set. Never `$CODEX_HOME/rules/`:
//!   that is Codex's execution-policy directory.
//! - `<root>/.agents/rules/` and `<root>/.claude/rules/`, where `<root>` is the
//!   git root of the payload's `cwd`.
//!
//! A file with no `paths:` in its frontmatter is ignored here; render-agent-docs
//! inlines those into AGENTS.md. One rule reached through two directories (a
//! symlinked `.agents` → `.claude`, or the same file installed globally and in
//! the repo) is delivered once.
//!
//! Candidate paths: the `Add File`, `Update File` and `Move to` headers of a
//! patch; for Bash, up to 64 non-flag words that contain `/` or look like a file
//! name with an extension.
//!
//! Globs match the candidate relative to `<root>`; a path outside it is matched
//! as written. `**`, `*`, `?` and `{a,b}` are supported and every glob is
//! anchored at the root, including one with no `/`: `Cargo.lock` matches the
//! root lockfile only. The rule files this serves spell depth out
//! (`**/Cargo.toml` in dependency-hygiene.md), so a gitignore-style basename
//! match would widen them.
//!
//! Never denies and spawns no process. Any error exits 0 silently.

use crate::hook;
use crate::pr_guard;
use crate::session_state;
use regex::Regex;
use serde_json::Value;
use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::SystemTime;

/// Ceiling on the whole context text, in bytes; rules past it become pointers.
pub const MAX_CONTEXT: usize = 16_000;

pub const MAX_CANDIDATES: usize = 64;

const MAX_RULES_PER_DIR: usize = 200;
const MAX_RULE_BYTES: u64 = 256 * 1024;
const MAX_ROOT_DEPTH: usize = 64;

pub const HEADER: &str =
    "rule-scope: for the rest of this session, these rules govern every file their paths match.";

const OVERFLOW: &str = "Not inlined, over the size cap. Read each before editing a matching file:";

#[derive(Debug)]
pub struct Rule {
    pub path: PathBuf,
    pub stem: String,
    pub globs: Vec<Regex>,
    pub body: String,
}

impl Rule {
    pub fn matches_any(&self, keys: &[String]) -> bool {
        keys.iter()
            .any(|key| self.globs.iter().any(|g| g.is_match(key)))
    }
}

pub fn run(payload: &Value) -> ! {
    let home = session_state::codex_home();
    if hook::str_at(payload, &["hook_event_name"]) == "PostCompact" {
        forget(payload, home.as_deref());
        hook::allow();
    }
    match decide(payload, home.as_deref(), SystemTime::now()) {
        Some(context) => hook::pre_tool_use_context(&context),
        None => hook::allow(),
    }
}

/// Clear the session's delivered-rule record, so each rule is delivered again
/// the next time a call touches a matching file.
pub fn forget(payload: &Value, codex_home: Option<&Path>) {
    let Some(session) = session_state::file_name(hook::str_at(payload, &["session_id"])) else {
        return;
    };
    if let Some(home) = codex_home {
        let _ = std::fs::remove_file(home.join("rule-scope").join(session));
    }
}

/// The context text for rules this call newly matches, or `None`. Rules
/// returned are recorded as delivered before this returns.
pub fn decide(payload: &Value, codex_home: Option<&Path>, now: SystemTime) -> Option<String> {
    let command = hook::str_at(payload, &["tool_input", "command"]);
    let raw = match hook::str_at(payload, &["tool_name"]) {
        "apply_patch" => patch_paths(command),
        "Bash" => command_paths(command),
        _ => return None,
    };
    if raw.is_empty() {
        return None;
    }
    let session = session_state::file_name(hook::str_at(payload, &["session_id"]))?;
    let state = codex_home?.join("rule-scope").join(session);

    let cwd = hook::str_at(payload, &["cwd"]);
    let root = git_root(&hook::resolve_path("", cwd));
    let keys: Vec<String> = raw
        .iter()
        .filter_map(|p| match_key(p, cwd, root.as_deref()))
        .collect();

    let delivered = read_state(&state);
    let rules = load_rules(&rule_dirs(codex_home, root.as_deref()));
    let fresh: Vec<&Rule> = rules
        .iter()
        .filter(|rule| !delivered.contains(&rule.path) && rule.matches_any(&keys))
        .collect();
    if fresh.is_empty() {
        return None;
    }
    record_state(&state, &fresh, now).ok()?;
    Some(compose(&fresh))
}

/// The frontmatter and the body of a Markdown file that opens with a `---`
/// fence, or `None` when it does not.
pub fn split_frontmatter(text: &str) -> Option<(&str, &str)> {
    let text = text.trim_start_matches('\u{feff}');
    let rest = text.strip_prefix("---")?;
    let rest = rest
        .strip_prefix("\r\n")
        .or_else(|| rest.strip_prefix('\n'))?;
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        if line.trim_end() == "---" {
            return Some((&rest[..offset], &rest[offset + line.len()..]));
        }
        offset += line.len();
    }
    None
}

/// The globs of a top-level `paths:` key, as a block list of `- glob` lines,
/// an inline `[a, b]` list or a single scalar. `None` when there is no such key.
pub fn paths_globs(frontmatter: &str) -> Option<Vec<String>> {
    let mut lines = frontmatter.lines().peekable();
    while let Some(line) = lines.next() {
        let Some(value) = line.strip_prefix("paths:") else {
            continue;
        };
        let value = value.trim();
        let globs = if value.is_empty() {
            let mut items = Vec::new();
            while let Some(next) = lines.peek() {
                let item = next.trim();
                if item.is_empty() || item.starts_with('#') {
                    lines.next();
                    continue;
                }
                let Some(glob) = item.strip_prefix('-') else {
                    break;
                };
                items.push(unquote(glob.trim()).to_string());
                lines.next();
            }
            items
        } else if let Some(inner) = value.strip_prefix('[').and_then(|v| v.strip_suffix(']')) {
            split_list(inner)
                .into_iter()
                .map(|item| unquote(item.trim()).to_string())
                .collect()
        } else {
            vec![unquote(value).to_string()]
        };
        return Some(globs.into_iter().filter(|g| !g.is_empty()).collect());
    }
    None
}

fn unquote(value: &str) -> &str {
    for quote in ['"', '\''] {
        if let Some(inner) = value
            .strip_prefix(quote)
            .and_then(|v| v.strip_suffix(quote))
        {
            return inner;
        }
    }
    value
}

/// Split on commas outside quotes and outside `{...}`, so `"*.{ts,tsx}"` stays
/// one item.
fn split_list(body: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut quote: Option<char> = None;
    let mut depth = 0usize;
    let mut start = 0;
    for (i, ch) in body.char_indices() {
        match (quote, ch) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(ch),
            (None, '{') => depth += 1,
            (None, '}') => depth = depth.saturating_sub(1),
            (None, ',') if depth == 0 => {
                parts.push(&body[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&body[start..]);
    parts
}

/// An anchored regex for a glob. `**/` spans zero or more directories, a
/// trailing `/**` the directory and everything under it, `*` and `?` stay
/// within one path segment, and `{a,b}` is an alternation.
pub fn glob_regex(glob: &str) -> Option<Regex> {
    let glob = glob.trim();
    let glob = glob.strip_prefix("./").unwrap_or(glob);
    let glob = glob.trim_start_matches('/');
    if glob.is_empty() {
        return None;
    }
    let chars: Vec<char> = glob.chars().collect();
    let mut re = String::from("^");
    translate(&chars, &mut re);
    re.push('$');
    Regex::new(&re).ok()
}

fn translate(chars: &[char], re: &mut String) {
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '*' if chars.get(i + 1) == Some(&'*') => {
                let segment_start = i == 0 || chars[i - 1] == '/';
                match chars.get(i + 2) {
                    Some('/') if segment_start => {
                        re.push_str("(?:.*/)?");
                        i += 3;
                    }
                    None if segment_start && i > 0 => {
                        re.pop();
                        re.push_str("(?:/.*)?");
                        i += 2;
                    }
                    _ => {
                        re.push_str(".*");
                        i += 2;
                    }
                }
            }
            '*' => {
                re.push_str("[^/]*");
                i += 1;
            }
            '?' => {
                re.push_str("[^/]");
                i += 1;
            }
            '{' => match closing_brace(chars, i) {
                Some(end) => {
                    re.push_str("(?:");
                    for (n, alternative) in alternatives(&chars[i + 1..end]).into_iter().enumerate()
                    {
                        if n > 0 {
                            re.push('|');
                        }
                        translate(alternative, re);
                    }
                    re.push(')');
                    i = end + 1;
                }
                None => {
                    re.push_str(r"\{");
                    i += 1;
                }
            },
            c => {
                re.push_str(&regex::escape(c.encode_utf8(&mut [0; 4])));
                i += 1;
            }
        }
    }
}

fn closing_brace(chars: &[char], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (i, c) in chars.iter().enumerate().skip(open) {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

fn alternatives(chars: &[char]) -> Vec<&[char]> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    for (i, c) in chars.iter().enumerate() {
        match c {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                out.push(&chars[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&chars[start..]);
    out
}

/// The paths named by the `Add File`, `Update File` and `Move to` headers.
pub fn patch_paths(patch: &str) -> Vec<String> {
    patch
        .lines()
        .filter_map(|line| {
            line.strip_prefix("*** Add File: ")
                .or_else(|| line.strip_prefix("*** Update File: "))
                .or_else(|| line.strip_prefix("*** Move to: "))
        })
        .map(|path| path.trim().to_string())
        .filter(|path| !path.is_empty())
        .take(MAX_CANDIDATES)
        .collect()
}

/// Words of a shell command that look like paths: flags are dropped (a
/// `--flag=value` keeps its value), URLs are dropped, and a word qualifies when
/// it contains `/` or reads as a file name with an extension.
pub fn command_paths(command: &str) -> Vec<String> {
    static FILE_NAME: OnceLock<Regex> = OnceLock::new();
    let file_name = FILE_NAME.get_or_init(|| {
        Regex::new(r"^\.?[A-Za-z0-9_][A-Za-z0-9_.+-]*\.[A-Za-z][A-Za-z0-9]{0,9}$")
            .expect("valid regex")
    });
    pr_guard::segments(command)
        .into_iter()
        .flatten()
        .filter_map(|word| {
            if word.starts_with('-') {
                return word.split_once('=').map(|(_, value)| value.to_string());
            }
            Some(word)
        })
        .filter(|word| !word.contains("://"))
        .filter(|word| word.contains('/') || file_name.is_match(word))
        .take(MAX_CANDIDATES)
        .collect()
}

/// The nearest ancestor of `cwd` holding `.git`, looking no more than
/// [`MAX_ROOT_DEPTH`] levels up.
pub fn git_root(cwd: &Path) -> Option<PathBuf> {
    if !cwd.is_absolute() {
        return None;
    }
    cwd.ancestors()
        .take(MAX_ROOT_DEPTH)
        .find(|dir| dir.join(".git").exists())
        .map(Path::to_path_buf)
}

/// What a glob is matched against: `raw` relative to `root` when it resolves
/// inside it, else `raw` as written.
pub fn match_key(raw: &str, cwd: &str, root: Option<&Path>) -> Option<String> {
    let resolved = hook::resolve_path(cwd, raw);
    if let Some(relative) = root.and_then(|r| resolved.strip_prefix(r).ok()) {
        let relative = relative.to_str()?;
        return (!relative.is_empty()).then(|| relative.to_string());
    }
    Some(raw.to_string())
}

pub fn rule_dirs(codex_home: Option<&Path>, root: Option<&Path>) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(home) = codex_home {
        dirs.push(home.join("agent-rules"));
    }
    if let Some(root) = root {
        dirs.push(root.join(".agents").join("rules"));
        dirs.push(root.join(".claude").join("rules"));
    }
    dirs
}

/// Every path-scoped rule in `dirs`, directory by directory, each sorted by
/// file name. A file seen before under another path, or a rule with the same
/// name and body as an earlier one, is skipped; a rule's `path` is canonical.
pub fn load_rules(dirs: &[PathBuf]) -> Vec<Rule> {
    let mut rules: Vec<Rule> = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        let mut files: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "md"))
            .collect();
        files.sort();
        for path in files.into_iter().take(MAX_RULES_PER_DIR) {
            let small =
                std::fs::metadata(&path).is_ok_and(|m| m.is_file() && m.len() <= MAX_RULE_BYTES);
            if !small {
                continue;
            }
            let path = std::fs::canonicalize(&path).unwrap_or(path);
            if !seen.insert(path.clone()) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let Some(rule) = parse_rule(path, &text) else {
                continue;
            };
            let duplicate = rules
                .iter()
                .any(|r| r.stem == rule.stem && r.body == rule.body);
            if !duplicate {
                rules.push(rule);
            }
        }
    }
    rules
}

/// A rule from its file text, or `None` when its frontmatter has no `paths:`.
pub fn parse_rule(path: PathBuf, text: &str) -> Option<Rule> {
    let (front, body) = split_frontmatter(text)?;
    let globs = paths_globs(front)?
        .iter()
        .filter_map(|g| glob_regex(g))
        .collect();
    let stem = path.file_stem()?.to_str()?.to_string();
    Some(Rule {
        path,
        stem,
        globs,
        body: body.trim().to_string(),
    })
}

fn read_state(state: &Path) -> HashSet<PathBuf> {
    std::fs::read_to_string(state)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.is_empty())
        .map(PathBuf::from)
        .collect()
}

fn record_state(state: &Path, rules: &[&Rule], now: SystemTime) -> std::io::Result<()> {
    session_state::prepare(state, now);
    let mut file = OpenOptions::new().create(true).append(true).open(state)?;
    let lines: String = rules
        .iter()
        .map(|r| format!("{}\n", r.path.display()))
        .collect();
    file.write_all(lines.as_bytes())
}

/// The context text: [`HEADER`], then each rule in full while it fits under
/// [`MAX_CONTEXT`], then a `read <path>` line for every rule that did not.
pub fn compose(rules: &[&Rule]) -> String {
    let pointer = |rule: &Rule| format!("- read {}\n", rule.path.display());
    let mut out = format!("{HEADER}\n\n");
    let mut skipped: Vec<&Rule> = Vec::new();
    for (i, rule) in rules.iter().enumerate() {
        let section = format!(
            "### {} ({})\n\n{}\n\n",
            rule.stem,
            rule.path.display(),
            rule.body
        );
        let later = skipped
            .iter()
            .copied()
            .chain(rules[i + 1..].iter().copied());
        let pointers: usize = later.map(|r| pointer(r).len()).sum();
        let reserve = if pointers == 0 {
            0
        } else {
            OVERFLOW.len() + 1 + pointers
        };
        if out.len() + section.len() + reserve <= MAX_CONTEXT {
            out.push_str(&section);
        } else {
            skipped.push(rule);
        }
    }
    if !skipped.is_empty() {
        out.push_str(OVERFLOW);
        out.push('\n');
        for rule in skipped {
            out.push_str(&pointer(rule));
        }
    }
    let mut end = out.trim_end().len().min(MAX_CONTEXT);
    while !out.is_char_boundary(end) {
        end -= 1;
    }
    out.truncate(end);
    out
}

#[cfg(test)]
#[path = "rule_scope_tests.rs"]
mod rule_scope_tests;
