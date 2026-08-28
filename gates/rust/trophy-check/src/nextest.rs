//! Shell out to `cargo nextest` and parse its output.
//!
//! `list` enumerates; `run` executes a filtered subset. Nothing here links the
//! crate under test — it only spawns `cargo`.

use crate::manifest::TestCase;
use serde_json::Value;
use std::collections::BTreeSet;
use std::process::Command;

/// Which packages and features to enumerate under.
pub struct Scope {
    pub manifest_path: String,
    /// Empty selects `--workspace`.
    pub packages: Vec<String>,
    pub features: Vec<String>,
}

impl Scope {
    fn base_args(&self) -> Vec<String> {
        let mut args = vec![
            "nextest".to_string(),
            "--manifest-path".to_string(),
            self.manifest_path.clone(),
        ];
        if self.packages.is_empty() {
            args.push("--workspace".to_string());
        } else {
            for p in &self.packages {
                args.push("-p".to_string());
                args.push(p.clone());
            }
        }
        if !self.features.is_empty() {
            args.push("--features".to_string());
            args.push(self.features.join(","));
        }
        args
    }
}

pub fn list(scope: &Scope) -> Result<Vec<TestCase>, String> {
    let mut args = scope.base_args();
    args.insert(1, "list".to_string());
    args.push("--message-format".to_string());
    args.push("json".to_string());

    let out = Command::new("cargo")
        .args(&args)
        .output()
        .map_err(|e| format!("failed to spawn `cargo nextest list`: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "`cargo nextest list` failed ({}).\n{}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    parse_list(&String::from_utf8_lossy(&out.stdout))
}

pub fn parse_list(json: &str) -> Result<Vec<TestCase>, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("parse nextest json: {e}"))?;
    let suites = v
        .get("rust-suites")
        .and_then(Value::as_object)
        .ok_or("nextest json has no `rust-suites`")?;

    let mut tests = Vec::new();
    for suite in suites.values() {
        let binary_id = suite
            .get("binary-id")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let Some(cases) = suite.get("testcases").and_then(Value::as_object) else {
            continue;
        };
        for name in cases.keys() {
            tests.push(TestCase {
                full: format!("{binary_id}::{name}"),
                testcase: name.clone(),
            });
        }
    }
    Ok(tests)
}

pub struct RunOutcome {
    pub ok: bool,
    pub failed: BTreeSet<String>,
    pub output: String,
}

/// `cargo nextest run … -E <expr>`. Reuses the binaries `list` already built.
pub fn run(scope: &Scope, expr: &str) -> Result<RunOutcome, String> {
    let mut args = scope.base_args();
    args.insert(1, "run".to_string());
    args.push("-E".to_string());
    args.push(expr.to_string());
    // No colour, so the FAIL-line parse below is stable.
    args.push("--color".to_string());
    args.push("never".to_string());

    let out = Command::new("cargo")
        .args(&args)
        .output()
        .map_err(|e| format!("failed to spawn `cargo nextest run`: {e}"))?;
    let mut combined = String::from_utf8_lossy(&out.stdout).into_owned();
    combined.push_str(&String::from_utf8_lossy(&out.stderr));
    Ok(RunOutcome {
        ok: out.status.success(),
        failed: parse_failed(&combined),
        output: combined,
    })
}

/// nextest failure statuses that name a test on their line.
const FAIL_MARKERS: &[&str] = &[
    "FAIL", "TIMEOUT", "ABORT", "SIGSEGV", "SIGABRT", "LEAK", "XPASS",
];

/// `    FAIL [   0.031s] app::binid module::test_name` — the testcase is the
/// last whitespace token.
pub fn parse_failed(output: &str) -> BTreeSet<String> {
    let mut failed = BTreeSet::new();
    for line in output.lines() {
        let mut tokens = line.split_whitespace();
        let Some(first) = tokens.next() else { continue };
        if FAIL_MARKERS.contains(&first) {
            if let Some(last) = line.split_whitespace().last() {
                failed.insert(last.to_string());
            }
        }
    }
    failed
}

pub fn union_expr(testcases: &BTreeSet<String>) -> String {
    testcases
        .iter()
        .map(|t| format!("test(={t})"))
        .collect::<Vec<_>>()
        .join(" | ")
}

#[cfg(test)]
#[path = "nextest_tests.rs"]
mod nextest_tests;
