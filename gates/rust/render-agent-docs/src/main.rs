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
//! path      = "AGENTS.md"
//! title     = "AGENTS.md — Acme instructions"
//! max_bytes = "32768"                   # quoted: the manifest holds strings
//! ```
//!
//! `--check` is the gate. Wire it into the pre-push hook: a target edited by
//! hand, or a rule added to the manifest and never rendered, fails the push
//! instead of drifting quietly.
//!
//! `max_bytes` defaults to 32768, Codex's `project_doc_max_bytes`; `"0"` means
//! no cap, for `~/.codex/AGENTS.md`, which Codex reads whole. Codex applies the
//! limit to the whole root-to-cwd chain of AGENTS.md files, so this per-file
//! check is a floor, not the full check. Writing an over-cap target warns;
//! `--check` fails on it.
//!
//! A rule whose frontmatter has `paths:` is listed by name instead of inlined
//! when `rules_dir` is a directory `codex-guard rule-scope` reads
//! (`.agents/rules`, `.claude/rules`, `$CODEX_HOME/agent-rules`); rule-scope
//! then delivers it when a call touches a matching file. From any other
//! directory it is inlined, with a warning, because nothing else would deliver
//! it. `~/` expands in `rules_dir`, `overlay` and target paths.
//!
//! Exit 0 = written, or (under `--check`) every target current and within cap.
//! Exit 1 = a target is stale or over its cap. Exit 2 = the manifest could not
//! be used.

use gates_config::Config;
use std::path::PathBuf;
use std::process::ExitCode;

mod render;
use render::{
    expand_home, is_path_scoped, load_rules, over_cap, render, rule_scope_reads, Manifest,
};

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

    let manifest = match Manifest::from_config(&cfg) {
        Ok(manifest) => manifest,
        Err(e) => {
            eprintln!("render-agent-docs: {manifest_path}: {e}");
            return ExitCode::from(2);
        }
    };
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
        Some(path) => match std::fs::read_to_string(expand_home(path)) {
            Ok(text) => Some(text),
            Err(e) => {
                eprintln!("render-agent-docs: overlay {path}: {e}");
                return ExitCode::from(2);
            }
        },
        None => None,
    };

    let codex_home = std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".codex")));
    let defer_scoped = rule_scope_reads(&expand_home(&manifest.rules_dir), codex_home.as_deref());
    let scoped: Vec<&str> = rules
        .iter()
        .filter(|r| is_path_scoped(&r.text))
        .map(|r| r.name.as_str())
        .collect();
    if !defer_scoped && !scoped.is_empty() {
        eprintln!(
            "render-agent-docs: warning: inlining path-scoped rules ({}) because codex-guard \
             rule-scope does not read {}; move them to .agents/rules to deliver them on demand",
            scoped.join(", "),
            manifest.rules_dir
        );
    }

    let mut stale: Vec<String> = Vec::new();
    let mut over: Vec<String> = Vec::new();
    for target in &manifest.targets {
        let wanted = render(target, overlay.as_deref(), &rules, defer_scoped);
        let path = expand_home(&target.path);
        let current = std::fs::read_to_string(&path).ok();
        let cap = over_cap(target, &wanted);

        if check {
            if current.as_deref() != Some(wanted.as_str()) {
                stale.push(target.path.clone());
            }
            over.extend(cap);
            continue;
        }
        if let Some(message) = &cap {
            eprintln!("render-agent-docs: warning: {message}");
        }
        if current.as_deref() == Some(wanted.as_str()) {
            println!("  = {} (unchanged)", target.path);
            continue;
        }
        if let Some(dir) = path.parent() {
            if !dir.as_os_str().is_empty() {
                let _ = std::fs::create_dir_all(dir);
            }
        }
        if let Err(e) = std::fs::write(&path, &wanted) {
            eprintln!("render-agent-docs: write {}: {e}", target.path);
            return ExitCode::from(2);
        }
        println!("  ✓ {}", target.path);
    }

    for message in &over {
        eprintln!("render-agent-docs: {message}");
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
    if !over.is_empty() {
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
