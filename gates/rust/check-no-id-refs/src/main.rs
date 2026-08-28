//! check-no-id-refs — no task-tracker ids in source code.
//!
//! ```text
//! check-no-id-refs [--config gates.toml]
//! ```
//!
//! Config, all optional:
//!
//! ```toml
//! [id-refs]
//! roots      = ["apps/", "crates/", "packages/"]
//! shapes     = ["t-[0-9a-f]{4,}", "bd-[0-9a-z]{3,}"]
//! extensions = ["rs", "ts", "svelte"]
//! ```
//!
//! With no `[id-refs] roots`, every tracked source file is read.
//!
//! Exit 0 = clean. Exit 1 = an id in a comment. Exit 2 = the gate could not run.

use gates_config::Config;
use std::process::{Command, ExitCode};

mod scan;
use scan::{has_scanned_extension, pattern, scan, Hit, DEFAULT_EXTENSIONS, DEFAULT_ID_SHAPES};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let config_path = flag(&args, "--config").unwrap_or_else(|| "gates.toml".to_string());
    let cfg = match Config::read(&config_path) {
        Ok(Some(c)) => c,
        Ok(None) => Config::default(),
        Err(e) => {
            eprintln!("check-no-id-refs: {config_path}: {e}");
            return ExitCode::from(2);
        }
    };

    let shapes = cfg
        .list("id-refs.shapes")
        .unwrap_or_else(|| DEFAULT_ID_SHAPES.iter().map(|s| s.to_string()).collect());
    let extensions = cfg
        .list("id-refs.extensions")
        .unwrap_or_else(|| DEFAULT_EXTENSIONS.iter().map(|s| s.to_string()).collect());
    let roots = cfg.list("id-refs.roots").unwrap_or_default();

    let re = match pattern(&shapes) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("check-no-id-refs: bad id shape in {config_path}: {e}");
            return ExitCode::from(2);
        }
    };

    // Tracked files only: another session's untracked work-in-progress is not
    // this repo's committed source, and CI reads a clean checkout.
    let tracked = match Command::new("git").args(["ls-files"]).output() {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(str::to_string)
            .collect::<Vec<_>>(),
        _ => {
            eprintln!("check-no-id-refs: `git ls-files` failed — not a repo?");
            return ExitCode::from(2);
        }
    };

    let mut hits: Vec<Hit> = Vec::new();
    let mut scanned = 0usize;
    for path in tracked
        .iter()
        .filter(|p| has_scanned_extension(p, &extensions))
        .filter(|p| roots.is_empty() || roots.iter().any(|r| p.starts_with(r.as_str())))
    {
        let Ok(content) = std::fs::read_to_string(path) else {
            continue;
        };
        scanned += 1;
        hits.extend(scan(path, &content, &re));
    }

    // Scanning nothing is the failure this gate is least likely to notice.
    if scanned == 0 {
        eprintln!("check-no-id-refs: matched NO source files.");
        eprintln!(
            "  Either a root moved, or [id-refs] roots/extensions in {config_path} are wrong."
        );
        return ExitCode::from(2);
    }

    if hits.is_empty() {
        println!("check-no-id-refs: ✓ {scanned} source file(s), no tracker ids");
        return ExitCode::SUCCESS;
    }

    eprintln!("ERROR: task-tracker ids are forbidden in source code.\n");
    for h in &hits {
        eprintln!("  {}:{}  {}", h.path, h.line, h.text);
    }
    eprintln!(
        "\nThe id means nothing to a reader without that tracker, and ids churn.\n\
         Per occurrence:\n\
         \x20 • encodes a real decision   -> write or extend a decision record, cite it, drop the id\n\
         \x20 • pure provenance           -> delete it; git history has it\n\
         \x20 • paired id + record number -> drop the id half, keep the record\n\
         \nRule: rules/git-discipline.md"
    );
    ExitCode::from(1)
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}
