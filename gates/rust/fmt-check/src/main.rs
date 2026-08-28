//! fmt-check — whole-repo `rustfmt --check`, without cargo.
//!
//! A formatting check compiles nothing, so it should not wait on cargo's locks:
//! `target/.cargo-lock` per worktree, and `~/.cargo/.package-cache`, which is
//! one lock for the whole machine that per-worktree `target/` does not isolate.
//! This binary reads the manifests itself and drives `rustfmt` directly.
//!
//! Coverage is deliberately whole-repo and must stay that way: pre-commit
//! formats only STAGED paths, so a file nobody touches would otherwise sit
//! unformatted forever while every push stays green. Every TRACKED `*.rs` file
//! is checked, including ones no crate's `mod` tree reaches — which `cargo fmt`
//! walks from crate roots and therefore misses.
//!
//! Every crate is checked under ITS OWN edition. The tree mixes 2021 and 2024,
//! and the difference is not cosmetic — `let` chains do not even parse as 2021.
//!
//! Run from anywhere inside the repo:
//!   fmt-check                 # check every tracked *.rs file
//!   fmt-check --print-plan    # print `<edition>\t<path>` for every file
//!   fmt-check --jobs 4        # cap the rustfmt worker pool

mod manifest;
mod plan;
mod runner;

#[cfg(test)]
mod manifest_tests;
#[cfg(test)]
mod plan_tests;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const USAGE: &str = "\
usage: fmt-check [--print-plan] [--jobs N] [--rustfmt PATH]

  --print-plan   print `<edition>\\t<path>` for every file that would be
                 checked, and exit without running rustfmt
  --jobs N       size of the rustfmt worker pool (default: CPU count, max 8)
  --rustfmt PATH rustfmt binary to use (default: $RUSTFMT, else `rustfmt`)
";

struct Options {
    print_plan: bool,
    jobs: usize,
    rustfmt: String,
}

fn main() -> ExitCode {
    let options = match parse_args(std::env::args().skip(1)) {
        Ok(Some(options)) => options,
        Ok(None) => {
            print!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Err(message) => {
            eprintln!("fmt-check: {message}\n\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };

    let root = match repo_root() {
        Ok(root) => root,
        Err(message) => {
            eprintln!("fmt-check: {message}");
            return ExitCode::FAILURE;
        }
    };

    let manifests = match manifest::discover(&root) {
        Ok(manifests) => manifests,
        Err(e) => {
            eprintln!(
                "fmt-check: could not read manifests under {}: {e}",
                root.display()
            );
            return ExitCode::FAILURE;
        }
    };
    let editions = match manifest::resolve(&manifests) {
        Ok(editions) => editions,
        Err(errors) => {
            eprintln!("fmt-check: could not determine the edition of every crate:");
            for error in errors {
                eprintln!("  {error}");
            }
            return ExitCode::FAILURE;
        }
    };

    let files = match tracked_rust_files(&root) {
        Ok(files) => files,
        Err(message) => {
            eprintln!("fmt-check: {message}");
            return ExitCode::FAILURE;
        }
    };

    let plan = plan::build(&files, &editions);
    if !plan.unowned.is_empty() {
        eprintln!(
            "fmt-check: {} tracked Rust file(s) sit under no Cargo package, so no \
             edition applies to them:",
            plan.unowned.len()
        );
        for file in &plan.unowned {
            eprintln!("  {}", file.display());
        }
        return ExitCode::FAILURE;
    }

    if options.print_plan {
        for (edition, files) in &plan.groups {
            for file in files {
                println!("{edition}\t{}", file.display());
            }
        }
        return ExitCode::SUCCESS;
    }

    let outcome = runner::check_all(&options.rustfmt, &plan.groups, &root, options.jobs);
    report(&plan, &outcome, &editions);
    if outcome.is_clean() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn report(plan: &plan::Plan, outcome: &runner::Outcome, editions: &manifest::CrateEditions) {
    let total: usize = plan.groups.values().map(Vec::len).sum();
    let spread = plan
        .groups
        .iter()
        .map(|(edition, files)| format!("{edition}: {}", files.len()))
        .collect::<Vec<_>>()
        .join(", ");
    println!(
        "fmt-check: {total} files across {} crates ({spread})",
        editions.len()
    );

    for error in &outcome.errors {
        eprintln!("{}", error.trim_end());
    }

    if outcome.unformatted.is_empty() {
        if outcome.errors.is_empty() {
            println!("fmt-check: all formatted");
        }
        return;
    }

    println!();
    println!("{} file(s) are not formatted:", outcome.unformatted.len());
    let mut by_edition: BTreeMap<&str, Vec<&PathBuf>> = BTreeMap::new();
    for file in &outcome.unformatted {
        let edition = plan
            .groups
            .iter()
            .find(|(_, files)| files.contains(file))
            .map(|(edition, _)| edition.as_str())
            .unwrap_or("2021");
        by_edition.entry(edition).or_default().push(file);
    }
    for (edition, files) in &by_edition {
        for file in files {
            println!("  {}", file.display());
        }
        let paths = files
            .iter()
            .map(|f| f.display().to_string())
            .collect::<Vec<_>>()
            .join(" ");
        println!("  fix: rustfmt --edition {edition} {paths}");
    }
}

fn parse_args<I: Iterator<Item = String>>(args: I) -> Result<Option<Options>, String> {
    let default_jobs = std::thread::available_parallelism()
        .map(|n| n.get().min(8))
        .unwrap_or(4);
    let mut options = Options {
        print_plan: false,
        jobs: default_jobs,
        rustfmt: std::env::var("RUSTFMT").unwrap_or_else(|_| "rustfmt".to_string()),
    };
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => return Ok(None),
            "--print-plan" => options.print_plan = true,
            "--jobs" => {
                let value = args.next().ok_or("--jobs needs a number")?;
                options.jobs = value.parse().map_err(|_| format!("bad --jobs: {value}"))?;
            }
            "--rustfmt" => {
                options.rustfmt = args.next().ok_or("--rustfmt needs a path")?;
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(Some(options))
}

fn repo_root() -> Result<PathBuf, String> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| format!("could not run git: {e}"))?;
    if !output.status.success() {
        return Err("not inside a git repository".to_string());
    }
    let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(PathBuf::from(root))
}

/// Tracked `*.rs` files, repo-relative. Tracked rather than walked so build
/// output and untracked scratch files can never enter the check.
fn tracked_rust_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["ls-files", "-z", "--", "*.rs"])
        .output()
        .map_err(|e| format!("could not run `git ls-files`: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "`git ls-files` failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .split('\0')
        .filter(|entry| !entry.is_empty())
        .map(PathBuf::from)
        .collect())
}
