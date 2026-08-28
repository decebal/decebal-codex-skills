//! The scan. Takes file CONTENT, so it is testable without a filesystem.

use regex::Regex;

/// Id shapes, as alternatives spliced into the comment-anchored pattern below.
/// Defaults cover the two trackers this came from; override in config.
pub const DEFAULT_ID_SHAPES: &[&str] = &[r"t-[0-9a-f]{4,}", r"bd-[0-9a-z]{3,}"];

/// Source extensions to read. Markdown is deliberately absent: decision records,
/// plans and the rule files themselves cite ids on purpose.
pub const DEFAULT_EXTENSIONS: &[&str] =
    &["rs", "ts", "tsx", "js", "mjs", "cjs", "svelte", "go", "py"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub path: String,
    pub line: usize,
    pub text: String,
}

/// Anchored on a comment opener — `//`, `/*`, `<!--`, or a doc-continuation `*`
/// at the start of a line — so only PROSE mentions match.
pub fn pattern(shapes: &[String]) -> Result<Regex, regex::Error> {
    let alts = shapes.join("|");
    Regex::new(&format!(r"(//|/\*|<!--|^\s*\*).*\b({alts})\b"))
}

pub fn scan(path: &str, content: &str, re: &Regex) -> Vec<Hit> {
    content
        .lines()
        .enumerate()
        .filter(|(_, line)| re.is_match(line))
        .map(|(idx, line)| Hit {
            path: path.to_string(),
            line: idx + 1,
            text: line.trim().to_string(),
        })
        .collect()
}

pub fn has_scanned_extension(path: &str, extensions: &[String]) -> bool {
    if path.split('/').any(|seg| {
        matches!(
            seg,
            "node_modules" | "target" | "dist" | "build" | "bindings"
        )
    }) {
        return false;
    }
    match path.rsplit_once('.') {
        Some((_, ext)) => extensions.iter().any(|e| e == ext),
        None => false,
    }
}

#[cfg(test)]
#[path = "scan_tests.rs"]
mod scan_tests;
