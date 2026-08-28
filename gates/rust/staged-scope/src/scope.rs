//! Path → scope classification. Pure: no git, no filesystem, so it is testable.

use gates_config::Config;
use std::collections::BTreeSet;

/// One named scope and the paths that belong to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    pub name: String,
    pub patterns: Vec<String>,
}

/// The classifier: the declared scopes plus the paths that legitimately run no
/// gate at all.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rules {
    pub scopes: Vec<Scope>,
    pub inert: Vec<String>,
}

impl Rules {
    /// Read `[scope.<name>] paths = [...]` plus `[gates] inert = [...]`.
    ///
    /// A config with no scopes yields no rules, and `classify` then reports
    /// everything as `unknown` — the loud failure. A gate that quietly matched
    /// nothing would be the silent one.
    pub fn from_config(cfg: &Config) -> Rules {
        let scopes = cfg
            .tables_under("scope")
            .into_iter()
            .map(|name| {
                let patterns = cfg.list(&format!("scope.{name}.paths")).unwrap_or_default();
                Scope { name, patterns }
            })
            .filter(|s| !s.patterns.is_empty())
            .collect();
        Rules {
            scopes,
            inert: cfg.list("gates.inert").unwrap_or_default(),
        }
    }

    /// The scopes a change set touches, sorted and deduped.
    ///
    /// `unknown` is emitted for any path matching neither a scope nor the inert
    /// list. It is a scope name like any other so a caller cannot forget to
    /// handle it.
    pub fn classify(&self, files: &[String]) -> Vec<String> {
        let mut out: BTreeSet<String> = BTreeSet::new();
        for file in files {
            let mut matched = false;
            for scope in &self.scopes {
                if scope.patterns.iter().any(|p| matches(p, file)) {
                    out.insert(scope.name.clone());
                    matched = true;
                }
            }
            if !matched && !self.inert.iter().any(|p| matches(p, file)) {
                out.insert("unknown".to_string());
            }
        }
        out.into_iter().collect()
    }

    /// The paths that matched nothing — what a caller prints alongside
    /// `unknown` so the fix is obvious.
    pub fn unmatched<'a>(&self, files: &'a [String]) -> Vec<&'a String> {
        files
            .iter()
            .filter(|f| {
                !self
                    .scopes
                    .iter()
                    .any(|s| s.patterns.iter().any(|p| matches(p, f)))
                    && !self.inert.iter().any(|p| matches(p, f))
            })
            .collect()
    }
}

/// Three pattern shapes, chosen so a config is readable without a regex:
///
/// * ends with `/` — a directory prefix (`apps/api/` matches `apps/api/src/x.rs`)
/// * contains `*`  — a glob, where `*` matches any run of characters including `/`
/// * otherwise     — an exact path (`Cargo.toml` matches only the root manifest)
///
/// The glob check comes FIRST: `apps/*/src/` ends with `/` and is still a glob,
/// and reading it as a literal prefix would match nothing while looking correct.
pub fn matches(pattern: &str, path: &str) -> bool {
    if pattern.contains('*') {
        return glob(pattern, path);
    }
    if pattern.ends_with('/') {
        return path.starts_with(pattern);
    }
    pattern == path
}

/// `*` matches any run of characters. Written iteratively rather than
/// recursively so a pathological pattern cannot blow the stack.
fn glob(pattern: &str, path: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    let first = parts.first().copied().unwrap_or("");
    let last = if parts.len() > 1 {
        parts.last().copied().unwrap_or("")
    } else {
        ""
    };

    if !path.starts_with(first) || !path.ends_with(last) {
        return false;
    }
    // The window the middle segments must fit inside. `first` and `last` can
    // overlap on a short path (`*` matching nothing), which is not a match.
    let mut cursor = first.len();
    let end = path.len().saturating_sub(last.len());
    if cursor > end {
        return false;
    }

    for part in parts.iter().skip(1).take(parts.len().saturating_sub(2)) {
        if part.is_empty() {
            continue;
        }
        match path.get(cursor..end).and_then(|rest| rest.find(*part)) {
            Some(at) => cursor += at + part.len(),
            None => return false,
        }
    }
    cursor <= end
}

#[cfg(test)]
#[path = "scope_tests.rs"]
mod scope_tests;
