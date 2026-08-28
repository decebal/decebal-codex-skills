//! The trophy manifest and its pattern DSL.
//!
//! ## Pattern DSL
//!
//! `pattern` is a substring matcher over a test's full name
//! (`<binary-id>::<testcase>`):
//!
//! * groups separated by `&` — ALL must match (AND)
//! * alternatives within a group separated by `|` — ANY matches (OR)
//! * an alternative matches when it is a SUBSTRING of the full name
//! * surrounding parens and whitespace are ignored, so `a & (b|c)` == `a & b|c`
//!
//! Substring rather than regex on purpose: the promised line is written by
//! whoever wrote the plan, and a half-remembered test name should match the test
//! that exists rather than fail on an anchor nobody meant to type.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub id: String,
    pub story: String,
    pub layer: String,
    pub pattern: String,
    /// A missing test here fails the gate even without `--strict`.
    pub block_merge: bool,
    pub desc: String,
}

/// A test the runner enumerated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestCase {
    /// `<binary-id>::<testcase>` — what patterns match against.
    pub full: String,
    /// The bare testcase name, for building a run filter.
    pub testcase: String,
}

pub fn pattern_matches(pattern: &str, full: &str) -> bool {
    let groups: Vec<&str> = pattern.split('&').collect();
    if groups.iter().all(|g| clean_group(g).is_empty()) {
        return false;
    }
    groups.iter().all(|group| {
        let alts: Vec<&str> = group
            .split('|')
            .map(clean)
            .filter(|a| !a.is_empty())
            .collect();
        alts.is_empty() || alts.iter().any(|alt| full.contains(alt))
    })
}

fn clean(token: &str) -> &str {
    token.trim().trim_matches(|c| c == '(' || c == ')').trim()
}

fn clean_group(group: &str) -> String {
    group.split('|').map(clean).collect::<Vec<_>>().join("")
}

pub fn matches_for<'a>(entry: &Entry, tests: &'a [TestCase]) -> Vec<&'a TestCase> {
    tests
        .iter()
        .filter(|t| pattern_matches(&entry.pattern, &t.full))
        .collect()
}

#[cfg(test)]
#[path = "manifest_tests.rs"]
mod manifest_tests;
