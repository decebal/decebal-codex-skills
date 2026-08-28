//! render-agent-docs — one manifest, generated AGENTS.md files.
//!
//! ```text
//! render-agent-docs                        # write every target
//! render-agent-docs --check                # fail if any target is stale
//! render-agent-docs --manifest <path>      # default: agent-docs.toml
//! ```
//!
//! The manifest:
//!
//! ```toml
//! rules     = ["git-discipline", "evidence-discipline", "timeouts"]
//! rules_dir = "rules"
//! overlay   = "docs/agent-overlay.md"   # this repo's own stack, commands, boundaries
//!
//! [targets.agents]
//! path  = "AGENTS.md"
//! title = "AGENTS.md — Acme instructions"
//! ```
//!
//! `--check` is the gate. Wire it into the pre-push hook: a target edited by
//! hand, or a rule added to the manifest and never rendered, fails the push
//! instead of drifting quietly.
//!
//! Exit 0 = written, or (under `--check`) every target current.
//! Exit 1 = a target is stale. Exit 2 = the manifest could not be used.

use gates_config::Config;
use std::path::Path;
use std::process::ExitCode;

mod render;
use render::{load_rules, render, Manifest};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let check = args.iter().any(|a| a == "--check");
    let manifest_path = flag(&args, "--manifest").unwrap_or_else(|| "agent-docs.toml".to_string());

    let cfg = match Config::read(&manifest_path) {
        Ok(Some(c)) => c,
        Ok(None) => {
            eprintln!("render-agent-docs: no {manifest_path}");
            return ExitCode::from(2);
        }
        Err(e) => {
            eprintln!("render-agent-docs: {manifest_path}: {e}");
            return ExitCode::from(2);
        }
    };

    let manifest = Manifest::from_config(&cfg);
    if manifest.targets.is_empty() {
        eprintln!("render-agent-docs: {manifest_path} declares no [targets.*]");
        return ExitCode::from(2);
    }

    let rules = match load_rules(&manifest) {
        Ok(rules) => rules,
        Err(e) => {
            eprintln!("render-agent-docs: rule {e}");
            return ExitCode::from(2);
        }
    };

    // A missing overlay is an error, not an empty string: rendering every target
    // as a bare rules list would look like a successful run and silently delete
    // the repo's own instructions.
    let overlay = match &manifest.overlay {
        Some(path) => match std::fs::read_to_string(path) {
            Ok(text) => Some(text),
            Err(e) => {
                eprintln!("render-agent-docs: overlay {path}: {e}");
                return ExitCode::from(2);
            }
        },
        None => None,
    };

    let mut stale: Vec<String> = Vec::new();
    for target in &manifest.targets {
        let wanted = render(target, overlay.as_deref(), &rules);
        let current = std::fs::read_to_string(&target.path).ok();

        if check {
            if current.as_deref() != Some(wanted.as_str()) {
                stale.push(target.path.clone());
            }
            continue;
        }
        if current.as_deref() == Some(wanted.as_str()) {
            println!("  = {} (unchanged)", target.path);
            continue;
        }
        if let Some(dir) = Path::new(&target.path).parent() {
            if !dir.as_os_str().is_empty() {
                let _ = std::fs::create_dir_all(dir);
            }
        }
        if let Err(e) = std::fs::write(&target.path, &wanted) {
            eprintln!("render-agent-docs: write {}: {e}", target.path);
            return ExitCode::from(2);
        }
        println!("  ✓ {}", target.path);
    }

    if !stale.is_empty() {
        eprintln!(
            "render-agent-docs: {} target(s) differ from the manifest:",
            stale.len()
        );
        for p in &stale {
            eprintln!("  {p}");
        }
        eprintln!(
            "\nA hand-edited target drifts silently — that is how one repo's AGENTS.md came\n\
             to point at a rules directory that never existed. Re-render:\n\
             \x20   render-agent-docs --manifest {manifest_path}"
        );
        return ExitCode::from(1);
    }
    if check {
        println!(
            "render-agent-docs: ✓ {} target(s) current",
            manifest.targets.len()
        );
    }
    ExitCode::SUCCESS
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}
