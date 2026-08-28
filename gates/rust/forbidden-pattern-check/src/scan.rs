//! The scan. Takes file CONTENT, so it is testable without a filesystem.

use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub path: String,
    pub line: usize,
    pub text: String,
}

/// One `[forbidden.<name>]` table, resolved.
pub struct Rule {
    pub name: String,
    /// Directory prefixes to scan. Empty means "every tracked file".
    pub roots: Vec<String>,
    /// Exact paths to scan, instead of or alongside `roots`.
    pub files: Vec<String>,
    /// Extensions to read. Empty means "any extension".
    pub extensions: Vec<String>,
    pub patterns: Vec<Regex>,
    /// Paths exempt from this rule — migration code, or the one file that IS
    /// the thing being guarded.
    pub allow: Vec<String>,
    /// Read comment lines too. Off by default: a doc comment naming the banned
    /// symbol is usually explaining the ban.
    pub include_comments: bool,
    /// Skip test files. Off by default, because a ban on a credential API or a
    /// crypto primitive means it in tests too. Turn it on for a rule about
    /// LAYERING, where a fixture writing a temp file is not the breach the rule
    /// is looking for.
    pub skip_tests: bool,
    pub why: String,
}

impl Rule {
    pub fn selects(&self, path: &str) -> bool {
        if self.allow.iter().any(|a| a == path) {
            return false;
        }
        if self.skip_tests && is_test_file(path) {
            return false;
        }
        if !self.extensions.is_empty() && !has_extension(path, &self.extensions) {
            return false;
        }
        if self.files.iter().any(|f| f == path) {
            return true;
        }
        if !self.files.is_empty() && self.roots.is_empty() {
            return false;
        }
        self.roots.is_empty() || self.roots.iter().any(|r| path.starts_with(r.as_str()))
    }
}

/// A line that only mentions the symbol in prose. Covers the comment openers of
/// every language these gates run over.
pub fn is_comment(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("//")
        || t.starts_with("/*")
        || t.starts_with('*')
        || t.starts_with("<!--")
        || t.starts_with('#')
}

pub fn scan(path: &str, content: &str, rule: &Rule) -> Vec<Hit> {
    content
        .lines()
        .enumerate()
        .filter(|(_, line)| rule.include_comments || !is_comment(line))
        .filter(|(_, line)| rule.patterns.iter().any(|re| re.is_match(line)))
        .map(|(idx, line)| Hit {
            path: path.to_string(),
            line: idx + 1,
            text: line.trim().to_string(),
        })
        .collect()
}

/// A test file, under the conventions of every language these gates read.
pub fn is_test_file(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    path.split('/')
        .any(|segment| matches!(segment, "tests" | "__tests__" | "__vtests__"))
        || name == "tests.rs"
        || name.ends_with("_tests.rs")
        || name.ends_with("_test.rs")
        || name.contains(".test.")
        || name.contains(".spec.")
        || name.contains(".vitest.")
}

pub fn has_extension(path: &str, extensions: &[String]) -> bool {
    match path.rsplit_once('.') {
        Some((_, ext)) => extensions.iter().any(|e| e == ext),
        None => false,
    }
}

#[cfg(test)]
#[path = "scan_tests.rs"]
mod scan_tests;
