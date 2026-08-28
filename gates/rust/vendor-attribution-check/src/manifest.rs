//! Reading a vendored package's name and declared licence out of its manifest.
//!
//! Two ecosystems, one shape, and neither needs a parser: both spell the two
//! fields on their own line, and a vendored manifest is a copy of an upstream
//! one rather than something hand-golfed. Pulling in a TOML and a JSON parser to
//! read two strings would double the workspace's dependency count
//! (rules/dependency-hygiene.md).

/// What kind of manifest a filename denotes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Cargo,
    Npm,
}

pub fn kind_of(file_name: &str) -> Option<Kind> {
    match file_name {
        "Cargo.toml" => Some(Kind::Cargo),
        "package.json" => Some(Kind::Npm),
        _ => None,
    }
}

/// The package name, or `None` when the manifest does not declare one.
pub fn name(text: &str, kind: Kind) -> Option<String> {
    match kind {
        Kind::Cargo => toml_string(text, "name"),
        Kind::Npm => json_string(text, "name"),
    }
}

/// The declared licence, or `None`.
pub fn license(text: &str, kind: Kind) -> Option<String> {
    match kind {
        Kind::Cargo => toml_string(text, "license").or_else(|| {
            // A crate may point at a file instead of naming an SPDX expression.
            toml_string(text, "license-file")
        }),
        Kind::Npm => json_string(text, "license"),
    }
}

/// `name = "value"` at the start of a line, ignoring `[table]` nesting — a
/// vendored crate's `[package]` block comes first, so the first match is its own.
fn toml_string(text: &str, key: &str) -> Option<String> {
    for line in text.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix(key) else {
            continue;
        };
        let rest = rest.trim_start();
        let Some(rest) = rest.strip_prefix('=') else {
            continue;
        };
        if let Some(value) = quoted(rest.trim()) {
            return Some(value);
        }
    }
    None
}

/// `"key": "value"` — the top-level form both fields take in a package manifest.
fn json_string(text: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    for line in text.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix(needle.as_str()) else {
            continue;
        };
        let rest = rest.trim_start();
        let Some(rest) = rest.strip_prefix(':') else {
            continue;
        };
        let rest = rest.trim().trim_end_matches(',');
        if let Some(value) = quoted(rest) {
            return Some(value);
        }
    }
    None
}

fn quoted(raw: &str) -> Option<String> {
    let inner = raw.strip_prefix('"')?;
    let end = inner.find('"')?;
    let value = &inner[..end];
    if value.is_empty() {
        return None;
    }
    Some(value.to_string())
}

/// Does a directory entry name look like the upstream licence text?
pub fn is_license_file(file_name: &str) -> bool {
    let upper = file_name.to_ascii_uppercase();
    upper.starts_with("LICENSE") || upper.starts_with("LICENCE") || upper.starts_with("COPYING")
}

#[cfg(test)]
#[path = "manifest_tests.rs"]
mod manifest_tests;
