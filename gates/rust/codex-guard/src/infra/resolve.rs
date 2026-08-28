//! Wrapper-chain resolution: a blocked command hidden one layer down is still a
//! blocked command.
//!
//! FOLLOWS project-local text scripts reached by static resolution —
//! `[cd <dir> &&] bash|sh|source|. <path>`, `./<path>.sh`, `<dir>/<path>.sh`,
//! `make [-C <dir>] <target>`, `$(MAKE) <target>`, and
//! `[cd <dir> &&] bun|npm|pnpm run <script>` with the `--cwd`/`--prefix`/`-C`
//! forms.
//!
//! `-C` and `--cwd` matter more than they look. Any convention that prefers them
//! over `cd <dir> && …` routes every command through the flag form, so a
//! resolver blind to it is blind to the common case.
//!
//! STOPS at installed binaries, anything outside the repo, `node_modules`,
//! `dist`, `build`, non-text files, and anything already visited. Bounded by
//! depth, file count and file size.
//!
//! Known limits, both covered by the literal tier and the trust cache: a script
//! referencing a sibling by a path relative to a `cd`'d subdir is not resolved
//! (the `cd <dir> && bash <path>` form itself is), and a dynamic command naming
//! no infra tool is a blind spot.

use super::classify::{cached, classify, Verdict, INFRA_TOOL_RE, MUTATE_RE, READONLY_RE};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// One thing a chain reaches.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Ref {
    Script(String),
    /// `(dir, target)` — the dir may be empty.
    Make(String, String),
    /// `(dir, script name)` — the dir may be empty.
    Node(String, String),
}

impl Ref {
    /// How the reference names itself in a decision reason.
    fn label(&self) -> String {
        match self {
            Ref::Script(p) => p.clone(),
            Ref::Make(d, t) => format!("{d}::{t}"),
            Ref::Node(d, n) => format!("{d}::{n}"),
        }
    }

    /// How the reference names itself in the hashed unit list.
    fn unit_name(&self, resolved: &str) -> String {
        match self {
            Ref::Script(_) => resolved.to_string(),
            Ref::Make(d, t) => format!("make:{d}:{t}"),
            Ref::Node(d, n) => format!("node:{d}:{n}"),
        }
    }
}

#[derive(Debug, Default)]
pub struct Scan {
    pub verdict: Option<Verdict>,
    pub reason: String,
    /// The chain touches an infra tool or prod token in a mutating form, with no
    /// clean pattern match — a dynamic command, most likely.
    pub risk: bool,
    /// `"<sha256>  <name>"` per resolved unit, for the trust hash.
    pub units: Vec<String>,
    pub files: Vec<String>,
}

pub struct Limits {
    pub max_depth: usize,
    pub max_files: usize,
    pub max_bytes: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_depth: 5,
            max_files: 40,
            max_bytes: 524_288,
        }
    }
}

impl Limits {
    /// Depth from the environment. A non-numeric value falls back to the default
    /// rather than to 0, so a typo cannot silently drop the deep layer.
    pub fn from_env() -> Limits {
        let mut l = Limits::default();
        if let Ok(v) = std::env::var("INFRA_GUARD_DEPTH") {
            l.max_depth = v.parse().unwrap_or(5);
        }
        if let Ok(v) = std::env::var("INFRA_GUARD_MAX_FILES") {
            l.max_files = v.parse().unwrap_or(40);
        }
        if let Ok(v) = std::env::var("INFRA_GUARD_MAX_BYTES") {
            l.max_bytes = v.parse().unwrap_or(524_288);
        }
        l
    }
}

pub fn sha256_hex(text: &str) -> String {
    let mut h = Sha256::new();
    h.update(text.as_bytes());
    format!("{:x}", h.finalize())
}

/// Every wrapper reference in a block of text, deduped and ordered.
pub fn extract_refs(text: &str) -> BTreeSet<Ref> {
    let mut out = BTreeSet::new();

    let cd_script = cached(r"cd +([^ ;&|]+) +&& +(?:bash|sh|source|\.) +([^ ;&|<>]+\.sh)");
    for c in cd_script.captures_iter(text) {
        out.insert(Ref::Script(format!("{}/{}", &c[1], &c[2])));
    }
    for c in cached(r"(?:^|[^a-z])(?:bash|sh|source|\.) +([^ ;&|<>]+\.sh)").captures_iter(text) {
        out.insert(Ref::Script(c[1].to_string()));
    }
    for c in cached(r"(?:^|[[:space:]])([^ ;&|<>()]*\.sh)").captures_iter(text) {
        out.insert(Ref::Script(c[1].to_string()));
    }
    for c in cached(r"(?:^|[^a-z])source +([^ ;&|<>]+)").captures_iter(text) {
        out.insert(Ref::Script(c[1].to_string()));
    }

    for c in cached(r"(?:^|[^a-z])make +-C +([^ ;&|]+) +([a-zA-Z0-9_.:][a-zA-Z0-9_.:-]*)")
        .captures_iter(text)
    {
        out.insert(Ref::Make(c[1].to_string(), c[2].to_string()));
    }
    // `-C` is excluded here so the flag is not read as the target.
    for c in cached(r"(?:^|[^a-z])make +([a-zA-Z0-9_.:][a-zA-Z0-9_.:-]*)").captures_iter(text) {
        out.insert(Ref::Make(String::new(), c[1].to_string()));
    }
    for c in cached(r"[$][({]MAKE[)}] +([a-zA-Z0-9_.:-]+)").captures_iter(text) {
        out.insert(Ref::Make(String::new(), c[1].to_string()));
    }

    for c in
        cached(r"cd +([^ ;&|]+) +&& +(?:bun|npm|pnpm) +run +([a-zA-Z0-9:_.-]+)").captures_iter(text)
    {
        out.insert(Ref::Node(c[1].to_string(), c[2].to_string()));
    }
    for c in cached(r"(?:bun|npm|pnpm) +(?:--cwd|--prefix|-C) +([^ ;&|]+) +run +([a-zA-Z0-9:_.-]+)")
        .captures_iter(text)
    {
        out.insert(Ref::Node(c[1].to_string(), c[2].to_string()));
    }
    for c in cached(r"(?:^|[^&] )(?:bun|npm|pnpm) +run +([a-zA-Z0-9:_.-]+)").captures_iter(text) {
        out.insert(Ref::Node(String::new(), c[1].to_string()));
    }
    out
}

fn is_project_local(p: &Path, root: &Path) -> bool {
    if !p.starts_with(root) {
        return false;
    }
    let s = p.to_string_lossy();
    !["/node_modules/", "/.git/", "/dist/", "/build/"]
        .iter()
        .any(|seg| s.contains(seg))
}

/// Text, by the same test `grep -I` uses: no NUL byte in the content.
fn is_text(bytes: &[u8]) -> bool {
    !bytes.contains(&0)
}

fn strip_comments(text: &str) -> String {
    text.lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Read-only invocations are deliberately not a risk signal. Flagging every
/// mention of an infra tool made ordinary wrappers prompt, which is what got the
/// deep layer switched off rather than fixed.
pub fn risky_lines(text: &str, prod_re: Option<&str>) -> bool {
    let tokens = match prod_re {
        Some(p) if !p.is_empty() => format!("(?i)({INFRA_TOOL_RE}|{p})"),
        _ => format!("(?i)({INFRA_TOOL_RE})"),
    };
    let tool = cached(&tokens);
    let mutate = cached(&format!("(?i){MUTATE_RE}"));
    let readonly = cached(&format!("(?i){READONLY_RE}"));
    text.lines()
        .any(|l| tool.is_match(l) && mutate.is_match(l) && !readonly.is_match(l))
}

fn resolve_script(p: &str, root: &Path, cwd: &Path) -> Option<PathBuf> {
    let bare = p.strip_prefix("./").unwrap_or(p);
    [PathBuf::from(p), root.join(bare), cwd.join(bare)]
        .into_iter()
        .find(|cand| cand.is_file())
}

/// The recipe body for one make target.
///
/// `-C` must be honoured: `make -C sub build` runs `sub/Makefile`, and reading
/// the root Makefile instead resolves a different recipe or none at all.
fn make_recipe(target: &str, root: &Path, cwd: &Path, dir: &str) -> Option<String> {
    let candidates = [
        root.join(dir).join("Makefile"),
        cwd.join(dir).join("Makefile"),
        root.join(dir).join("makefile"),
        root.join("Makefile"),
    ];
    for mf in candidates {
        let Ok(text) = std::fs::read_to_string(&mf) else {
            continue;
        };
        let mut body = Vec::new();
        let mut inside = false;
        for line in text.lines() {
            if !inside {
                let head = line.split(':').next().unwrap_or("");
                if head.trim_end() == target && line.contains(':') {
                    inside = true;
                }
                continue;
            }
            if let Some(rest) = line.strip_prefix('\t') {
                body.push(rest);
            } else if !line.is_empty() {
                break;
            }
        }
        if !body.is_empty() {
            return Some(body.join("\n"));
        }
    }
    None
}

fn node_script(name: &str, root: &Path, cwd: &Path, dir: &str) -> Option<String> {
    let candidates = [
        root.join(dir).join("package.json"),
        cwd.join(dir).join("package.json"),
        cwd.join("package.json"),
        root.join("package.json"),
    ];
    for pj in candidates {
        let Ok(text) = std::fs::read_to_string(&pj) else {
            continue;
        };
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue;
        };
        if let Some(body) = json
            .get("scripts")
            .and_then(|s| s.get(name))
            .and_then(|v| v.as_str())
        {
            if !body.is_empty() {
                return Some(body.to_string());
            }
        }
    }
    None
}

/// Walk the chain. Scans the FULL tree with no early exit, so `units` is
/// complete for hashing even when a deny is already decided.
pub fn scan_tree(
    initial: &str,
    root: &Path,
    cwd: &Path,
    limits: &Limits,
    prod_re: Option<&str>,
) -> Scan {
    let mut scan = Scan::default();
    let mut seen: BTreeSet<Ref> = BTreeSet::new();
    // Breadth-first, so the reason names the SHALLOWEST reference that fired.
    // A depth-first walk reports the same verdict via a deeper file, which reads
    // as a different finding to whoever has to review the chain.
    let mut work: std::collections::VecDeque<(usize, Ref)> =
        extract_refs(initial).into_iter().map(|r| (1, r)).collect();
    let mut nfiles = 0usize;

    while let Some((depth, r)) = work.pop_front() {
        if !seen.insert(r.clone()) {
            continue;
        }
        if depth > limits.max_depth {
            continue;
        }
        if nfiles >= limits.max_files {
            break;
        }

        let (content, unit_name) = match &r {
            Ref::Script(p) => {
                let Some(rp) = resolve_script(p, root, cwd) else {
                    continue;
                };
                if !is_project_local(&rp, root) {
                    continue;
                }
                let Ok(bytes) = std::fs::read(&rp) else {
                    continue;
                };
                if !is_text(&bytes) || bytes.len() as u64 > limits.max_bytes {
                    continue;
                }
                let Ok(text) = String::from_utf8(bytes) else {
                    continue;
                };
                nfiles += 1;
                let name = rp.to_string_lossy().to_string();
                scan.files.push(name.clone());
                (text, r.unit_name(&name))
            }
            Ref::Make(dir, target) => match make_recipe(target, root, cwd, dir) {
                Some(body) => (body, r.unit_name("")),
                None => continue,
            },
            Ref::Node(dir, name) => match node_script(name, root, cwd, dir) {
                Some(body) => (body, r.unit_name("")),
                None => continue,
            },
        };

        scan.units
            .push(format!("{}  {}", sha256_hex(&content), unit_name));

        let code = strip_comments(&content);
        if !scan.risk && risky_lines(&code, prod_re) {
            scan.risk = true;
        }

        if let Some(v) = classify(&code.to_lowercase()) {
            let reason = format!("{} (in {})", reason_of(&v), r.label());
            match (&v, &scan.verdict) {
                (Verdict::Deny(_), Some(Verdict::Deny(_))) => {}
                (Verdict::Deny(_), _) => {
                    scan.reason = reason;
                    scan.verdict = Some(v);
                }
                (Verdict::Ask(_), None) => {
                    scan.reason = reason;
                    scan.verdict = Some(v);
                }
                _ => {}
            }
        }

        for next in extract_refs(&code) {
            work.push_back((depth + 1, next));
        }
    }
    scan
}

fn reason_of(v: &Verdict) -> &str {
    match v {
        Verdict::Deny(r) | Verdict::Ask(r) => r,
    }
}

/// Combined hash over the sorted set of resolved unit hashes.
///
/// Covers script content, make-recipe text and node-script text, so editing a
/// Makefile recipe busts the trust even when no script file changed.
pub fn tree_hash(units: &[String]) -> Option<String> {
    if units.is_empty() {
        return None;
    }
    let sorted: BTreeSet<&String> = units.iter().collect();
    let mut joined = String::new();
    for u in sorted {
        joined.push_str(u);
        joined.push('\n');
    }
    Some(sha256_hex(&joined))
}

pub fn kosher_path() -> PathBuf {
    if let Ok(p) = std::env::var("INFRA_GUARD_KOSHER") {
        return PathBuf::from(p);
    }
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join(".codex/infra-guard-kosher.txt")
}

pub fn is_kosher(hash: &str) -> bool {
    if hash.is_empty() {
        return false;
    }
    let Ok(text) = std::fs::read_to_string(kosher_path()) else {
        return false;
    };
    text.lines().any(|l| l.starts_with(&format!("{hash}  ")))
}

#[cfg(test)]
#[path = "resolve_tests.rs"]
mod resolve_tests;
