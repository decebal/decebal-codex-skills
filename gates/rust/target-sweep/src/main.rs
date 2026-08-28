//! Reclaim `target/` without forcing a cold rebuild.
//!
//! Usage:
//!   target-sweep                 report only, nothing removed
//!   target-sweep --sweep         remove orphaned temp dirs older than --age
//!   target-sweep --age <days>    minimum age to remove, default 1
//!   target-sweep --path <dir>    a target dir to inspect; repeatable
//!
//! With no `--path`, it inspects `target/` plus any `*/target/` one level down
//! (a second cargo workspace in a subdirectory), relative to the current
//! directory.
//!
//! The decisions live in the library next door so they can be tested; this file
//! is argument parsing and printing.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use target_sweep::{human, inspect, sweep, STALE_REPORT_DAYS};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let do_sweep = args.iter().any(|a| a == "--sweep");
    let age_days = flag_value(&args, "--age")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(1);

    // `--age 0` would put a running rustc's scratch in scope. Refusing is the
    // whole safety property; see the module docs in lib.rs.
    if age_days == 0 {
        eprintln!(
            "target-sweep: --age 0 is refused. A temp dir younger than a day may belong to a\n\
             running rustc, and removing it breaks that build. Use --age 1 or more."
        );
        std::process::exit(2);
    }

    let paths = target_paths(&args);
    if paths.is_empty() {
        eprintln!(
            "target-sweep: no target directory found. Run from the repo root, or pass --path."
        );
        std::process::exit(1);
    }

    let cutoff = SystemTime::now() - Duration::from_secs(age_days * 24 * 60 * 60);
    let mut reclaimable = 0u64;
    let mut removed_total = 0usize;
    let mut stale_total = 0usize;

    for path in &paths {
        let findings = inspect(path, cutoff);
        println!("{}", path.display());
        println!(
            "  orphaned temp dirs older than {age_days}d: {} ({})",
            findings.temp_dirs.len(),
            human(findings.temp_bytes)
        );
        if findings.dep_files > 0 {
            println!(
                "  deps/ artifacts: {} total, {} untouched for {STALE_REPORT_DAYS}d",
                findings.dep_files, findings.stale_files
            );
        }
        reclaimable += findings.temp_bytes;
        stale_total += findings.stale_files;

        if do_sweep {
            let (removed, errors) = sweep(&findings);
            removed_total += removed;
            for e in errors {
                eprintln!("  could not remove {e}");
            }
        }
    }

    if do_sweep {
        println!(
            "\nremoved {removed_total} temp dir(s), about {}",
            human(reclaimable)
        );
    } else if reclaimable > 0 {
        println!("\n{} reclaimable — re-run with --sweep", human(reclaimable));
    }

    if stale_total > 0 {
        println!(
            "\n{stale_total} artifact(s) untouched for {STALE_REPORT_DAYS}d. Cargo never collects\n\
             these; `cargo sweep --time {STALE_REPORT_DAYS}` removes them while keeping what the\n\
             current build needs (cargo install cargo-sweep)."
        );
    }
}

fn flag_value(args: &[String], name: &str) -> Option<String> {
    let i = args.iter().position(|a| a == name)?;
    args.get(i + 1).cloned()
}

/// Every `--path`, or the two this repo has when run from its root.
fn target_paths(args: &[String]) -> Vec<PathBuf> {
    let explicit: Vec<PathBuf> = args
        .iter()
        .enumerate()
        .filter(|(_, a)| a.as_str() == "--path")
        .filter_map(|(i, _)| args.get(i + 1))
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .collect();
    if !explicit.is_empty() {
        return explicit;
    }
    // `target/`, plus a second workspace's `target/` one level down. Discovered
    // rather than hard-coded: a repo with `services/api/` or `admin/` gets its
    // build dir swept without anyone editing this list.
    let mut found: Vec<PathBuf> = Vec::new();
    let root = PathBuf::from("target");
    if root.is_dir() {
        found.push(root);
    }
    if let Ok(entries) = std::fs::read_dir(".") {
        let mut nested: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path().join("target"))
            .filter(|p| p.is_dir())
            .collect();
        nested.sort();
        found.extend(nested);
    }
    found
}
