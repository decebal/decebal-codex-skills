//! check-test-hangs — no unbounded real blocking I/O in test files.
//!
//! ```text
//! check-test-hangs [--config gates.toml]
//! ```
//!
//! Tiers come from the config, because not every tree deserves the same rule:
//!
//! ```toml
//! [test-hangs.backend]
//! paths  = ["apps/api/", "crates/"]
//! checks = ["sockets", "sleeps"]
//!
//! [test-hangs.services]
//! paths  = ["services/"]
//! checks = ["sleeps"]
//! ```
//!
//! The second tier is the interesting one. A suite that stands up real test
//! servers on ephemeral ports BY DESIGN would turn the socket check into an
//! allowlist-marking exercise with no bug behind it — but nothing about that
//! justifies a real `thread::sleep`. Splitting the tiers keeps the sleep rule
//! where a 40ms timing margin once failed CI on a loaded runner and blocked a
//! deploy.
//!
//! Exit 0 = clean. Exit 1 = an unmarked unbounded blocking-I/O test. Exit 2 =
//! the gate could not run, which is never reported as a pass.

use gates_config::Config;
use regex::Regex;
use std::process::{Command, ExitCode};

mod scan;
use scan::{
    is_test_file, scan, Violation, ALLOW_MARKER, DEFAULT_SLEEP_PATTERN, DEFAULT_SOCKET_PATTERN,
};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let config_path = flag(&args, "--config").unwrap_or_else(|| "gates.toml".to_string());

    let cfg = match Config::read(&config_path) {
        Ok(Some(c)) => c,
        Ok(None) => {
            eprintln!("check-test-hangs: no {config_path} — nothing declared to scan.");
            eprintln!(
                "  A gate that scans nothing reports the same ✓ as one that scans everything."
            );
            return ExitCode::from(2);
        }
        Err(e) => {
            eprintln!("check-test-hangs: {config_path}: {e}");
            return ExitCode::from(2);
        }
    };

    let tiers = cfg.tables_under("test-hangs");
    if tiers.is_empty() {
        eprintln!("check-test-hangs: {config_path} declares no [test-hangs.*] tier.");
        return ExitCode::from(2);
    }

    let sockets = Regex::new(
        cfg.string("test-hangs-socket-pattern")
            .unwrap_or(DEFAULT_SOCKET_PATTERN),
    )
    .expect("socket pattern compiles");
    let sleeps = Regex::new(
        cfg.string("test-hangs-sleep-pattern")
            .unwrap_or(DEFAULT_SLEEP_PATTERN),
    )
    .expect("sleep pattern compiles");

    let tracked = match tracked_files() {
        Some(f) => f,
        None => {
            eprintln!("check-test-hangs: `git ls-files` failed — not a repo?");
            return ExitCode::from(2);
        }
    };

    let mut violations: Vec<Violation> = Vec::new();
    let mut scanned = 0usize;

    for tier in &tiers {
        let paths = cfg
            .list(&format!("test-hangs.{tier}.paths"))
            .unwrap_or_default();
        let checks = cfg
            .list(&format!("test-hangs.{tier}.checks"))
            .unwrap_or_else(|| vec!["sockets".into(), "sleeps".into()]);
        let mut patterns: Vec<&Regex> = Vec::new();
        if checks.iter().any(|c| c == "sockets") {
            patterns.push(&sockets);
        }
        if checks.iter().any(|c| c == "sleeps") {
            patterns.push(&sleeps);
        }

        for file in tracked
            .iter()
            .filter(|f| is_test_file(f))
            .filter(|f| paths.iter().any(|p| f.starts_with(p.as_str())))
        {
            let content = match std::fs::read_to_string(file) {
                Ok(c) => c,
                Err(_) => continue,
            };
            scanned += 1;
            for pattern in &patterns {
                violations.extend(scan(file, &content, pattern));
            }
        }
    }

    // Scanning zero files is the failure this gate is most likely to have and
    // least likely to notice: a moved directory turns it into a no-op that
    // prints a tick. Treat it as a configuration error.
    if scanned == 0 {
        eprintln!(
            "check-test-hangs: matched NO test files across {} tier(s).",
            tiers.len()
        );
        eprintln!(
            "  Either a source root moved, or the [test-hangs.*] paths in {config_path} are wrong."
        );
        return ExitCode::from(2);
    }

    if violations.is_empty() {
        println!("check-test-hangs: ✓ {scanned} test file(s), no unbounded real blocking I/O");
        return ExitCode::SUCCESS;
    }

    eprintln!("ERROR: a test does real blocking I/O without a bound.\n");
    violations.sort_by(|a, b| (&a.path, a.line).cmp(&(&b.path, b.line)));
    violations.dedup();
    for v in &violations {
        eprintln!("  {}:{}  {}", v.path, v.line, v.text);
    }
    eprintln!(
        "\nA real socket accept/connect or a blocking sleep can hang forever and burn a\n\
         runner. Per occurrence:\n\
         \x20 • I/O over a socket  -> drive it over an in-memory duplex pair\n\
         \x20 • time-based logic   -> run the test under virtual time (start_paused /\n\
         \x20                         fake timers), never a wall-clock sleep\n\
         \x20 • an unbounded await -> wrap it in a timeout\n\
         \x20 • the socket IS the unit under test (ECONNREFUSED, port-in-use, real HTTP\n\
         \x20   framing) -> keep it bounded (ephemeral 127.0.0.1:0 + a read timeout, or a\n\
         \x20   timeout wrapper) and add on, or directly above, the line:\n\
         \x20       // {ALLOW_MARKER} <why a real socket, and why it cannot hang>\n\
         \nFull policy: rules/testing-gates.md"
    );
    ExitCode::from(1)
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

/// Tracked files only — matching CI's clean-checkout behaviour, and skipping
/// another session's untracked work-in-progress.
fn tracked_files() -> Option<Vec<String>> {
    let out = Command::new("git").args(["ls-files"]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(str::to_string)
            .collect(),
    )
}
