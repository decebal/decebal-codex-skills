//! A reader for the TOML subset `gates.toml` uses.
//!
//! # The subset
//!
//! ```toml
//! # comment
//! top_level = "value"
//!
//! [table]
//! string = "value"
//! list = ["a", "b"]
//! multiline = [
//!   "a",
//!   "b",   # trailing commas and per-line comments are fine
//! ]
//!
//! [table.nested]
//! key = "value"
//! ```
//!
//! Values are strings and arrays of strings. Numbers, booleans, dates, inline
//! tables and arrays-of-tables are NOT supported, and a line that looks like one
//! is a parse error rather than a skipped line — see [`Config::parse`].
//!
//! That strictness is the point. A reader that ignores what it does not
//! understand turns a typo'd key into a gate that silently checks nothing, which
//! is indistinguishable from a gate that passes.
//!
//! # Strings are VERBATIM
//!
//! There is no escape processing: the text between the quotes is the value. So a
//! regex is written the way it would be inside a raw string —
//! `patterns = ["use\s+keyring"]`, NOT `"use\\s+keyring"`.
//!
//! This is worth stating loudly because the failure is silent and TOML habit
//! points the wrong way. Doubling the backslash out of habit yields the literal
//! two characters `\` `\`, the regex compiles fine, and it matches nothing — a
//! gate that reports every file clean because its pattern can never fire. A
//! quote is the one character a value therefore cannot contain, and asking for
//! one is an error rather than a truncation.

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

/// One parsed config. Keys are `"table.key"`, or bare `"key"` at top level.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Config {
    values: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Str(String),
    List(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl Config {
    /// Read and parse a file. A missing file is `Ok(None)` — a repo without a
    /// `gates.toml` runs the gates on their defaults; a MALFORMED one is an
    /// error, because that is a mistake rather than an absence.
    pub fn read(path: impl AsRef<Path>) -> Result<Option<Config>, ParseError> {
        match std::fs::read_to_string(path.as_ref()) {
            Ok(text) => Config::parse(&text).map(Some),
            Err(_) => Ok(None),
        }
    }

    pub fn parse(text: &str) -> Result<Config, ParseError> {
        let mut values = BTreeMap::new();
        let mut table = String::new();
        let mut lines = text.lines().enumerate();

        while let Some((idx, raw)) = lines.next() {
            let lineno = idx + 1;
            let line = strip_comment(raw).trim().to_string();
            if line.is_empty() {
                continue;
            }

            if let Some(rest) = line.strip_prefix('[') {
                let name = rest.strip_suffix(']').ok_or_else(|| ParseError {
                    line: lineno,
                    message: format!("unterminated table header: {line}"),
                })?;
                if name.starts_with('[') {
                    return Err(ParseError {
                        line: lineno,
                        message: "arrays of tables are not supported".into(),
                    });
                }
                table = name.trim().to_string();
                continue;
            }

            let (key, rhs) = line.split_once('=').ok_or_else(|| ParseError {
                line: lineno,
                message: format!("not a key/value pair: {line}"),
            })?;
            let key = key.trim();
            if key.is_empty() {
                return Err(ParseError {
                    line: lineno,
                    message: "empty key".into(),
                });
            }
            let full = if table.is_empty() {
                key.to_string()
            } else {
                format!("{table}.{key}")
            };
            let rhs = rhs.trim();

            let value = if rhs.starts_with('[') {
                // An array may run to the end of the line or over several.
                let mut buf = rhs.to_string();
                while !buf.trim_end().ends_with(']') {
                    let (_, next) = lines.next().ok_or_else(|| ParseError {
                        line: lineno,
                        message: "unterminated array".into(),
                    })?;
                    buf.push(' ');
                    buf.push_str(strip_comment(next).trim());
                }
                Value::List(parse_list(&buf, lineno)?)
            } else {
                Value::Str(parse_string(rhs, lineno)?)
            };
            values.insert(full, value);
        }
        Ok(Config { values })
    }

    /// A string value, or `None` when the key is absent or holds a list.
    pub fn string(&self, key: &str) -> Option<&str> {
        match self.values.get(key) {
            Some(Value::Str(s)) => Some(s),
            _ => None,
        }
    }

    /// A list value. A single string reads as a one-element list, so
    /// `roots = "src"` and `roots = ["src"]` both work.
    pub fn list(&self, key: &str) -> Option<Vec<String>> {
        match self.values.get(key) {
            Some(Value::List(v)) => Some(v.clone()),
            Some(Value::Str(s)) => Some(vec![s.clone()]),
            None => None,
        }
    }

    /// Every table directly under `prefix`, in file order by name. Used where
    /// the config declares a set of things (scopes, tiers) rather than a fixed
    /// list of keys.
    pub fn tables_under(&self, prefix: &str) -> Vec<String> {
        let head = format!("{prefix}.");
        let mut names: Vec<String> = self
            .values
            .keys()
            .filter_map(|k| k.strip_prefix(&head))
            .filter_map(|rest| rest.split_once('.').map(|(name, _)| name.to_string()))
            .collect();
        names.sort();
        names.dedup();
        names
    }

    /// The leaf keys directly under `table`, sorted. Where [`Config::tables_under`]
    /// answers "which sub-tables exist", this answers "which keys does this one
    /// table hold" — for a table whose KEY NAMES are the data, so no gate can
    /// enumerate them in advance.
    pub fn keys_under(&self, table: &str) -> Vec<String> {
        let head = format!("{table}.");
        let mut keys: Vec<String> = self
            .values
            .keys()
            .filter_map(|k| k.strip_prefix(&head))
            .filter(|rest| !rest.contains('.'))
            .map(str::to_string)
            .collect();
        keys.sort();
        keys.dedup();
        keys
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// Drop a trailing `#` comment, respecting quotes so a `#` inside a string
/// survives.
fn strip_comment(line: &str) -> &str {
    let mut in_string = false;
    for (i, ch) in line.char_indices() {
        match ch {
            '"' => in_string = !in_string,
            '#' if !in_string => return &line[..i],
            _ => {}
        }
    }
    line
}

fn parse_string(raw: &str, line: usize) -> Result<String, ParseError> {
    let trimmed = raw.trim();
    let inner = trimmed
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .ok_or_else(|| ParseError {
            line,
            message: format!("only double-quoted strings are supported, got: {trimmed}"),
        })?;
    if inner.contains('"') {
        return Err(ParseError {
            line,
            message: "escaped quotes inside a string are not supported".into(),
        });
    }
    Ok(inner.to_string())
}

fn parse_list(raw: &str, line: usize) -> Result<Vec<String>, ParseError> {
    let body = raw
        .trim()
        .strip_prefix('[')
        .and_then(|s| s.trim_end().strip_suffix(']'))
        .ok_or_else(|| ParseError {
            line,
            message: format!("malformed array: {raw}"),
        })?;
    let mut out = Vec::new();
    for item in split_outside_quotes(body) {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        out.push(parse_string(item, line)?);
    }
    Ok(out)
}

/// Split on commas that are NOT inside a quoted string.
///
/// A plain `split(',')` truncates every element holding a comma, and the regex
/// quantifier `{4,}` is exactly that shape — so `shapes = ["t-[0-9a-f]{4,}"]`,
/// the form this config documents, failed to parse at all.
fn split_outside_quotes(body: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut in_string = false;
    let mut start = 0;
    for (i, ch) in body.char_indices() {
        match ch {
            '"' => in_string = !in_string,
            ',' if !in_string => {
                parts.push(&body[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&body[start..]);
    parts
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod lib_tests;
