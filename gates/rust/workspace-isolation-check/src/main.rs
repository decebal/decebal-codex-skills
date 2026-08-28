//! workspace-isolation-check — one half of a repo must not enter the other's
//! build graph.
//!
//! ```text
//! workspace-isolation-check [--config gates.toml] [--rule <name>]
//! ```
//!
//! ```toml
//! [isolation.control-plane]
//! # The build graph that must stay clean.
//! manifest      = "apps/desktop/Cargo.toml"
//! # Path segments no workspace MEMBER may sit under. Keep the slashes.
//! forbid_paths  = ["/control-plane/"]
//! # Lockfiles that must not resolve the other half's packages.
//! lockfiles     = ["bun.lock"]
//! forbid_names  = ["@acme-control-plane/"]
//! why           = "The desktop build must not pay for the server's dependencies."
//! ```
//!
//! Exit 0 = clean. Exit 1 = a leak. Exit 2 = the gate could not run.
//!
//! ## Why a declared lockfile that is absent fails
//!
//! `lockfiles` names a file the rule depends on reading. If it is gone the rule
//! covers less than it claims, and the difference between "checked, clean" and
//! "did not check" is invisible in the output — so it is exit 2, not a tick.

use gates_config::Config;
use std::process::{Command, ExitCode};

mod graph;
use graph::{cargo_members, leaks, lock_hits};

struct Rule {
    name: String,
    manifest: Option<String>,
    forbid_paths: Vec<String>,
    lockfiles: Vec<String>,
    forbid_names: Vec<String>,
    why: String,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let config_path = flag(&args, "--config").unwrap_or_else(|| "gates.toml".to_string());
    let only = flag(&args, "--rule");

    let cfg = match Config::read(&config_path) {
        Ok(Some(c)) => c,
        Ok(None) => {
            eprintln!("workspace-isolation-check: {config_path} not found.");
            return ExitCode::from(2);
        }
        Err(e) => {
            eprintln!("workspace-isolation-check: {config_path}: {e}");
            return ExitCode::from(2);
        }
    };

    let names = cfg.tables_under("isolation");
    if names.is_empty() {
        eprintln!("workspace-isolation-check: {config_path} declares no [isolation.*] rule.");
        return ExitCode::from(2);
    }

    let mut failed = false;
    let mut ran = 0usize;

    for name in names {
        if only.as_deref().is_some_and(|want| want != name) {
            continue;
        }
        let rule = build_rule(&cfg, &name);
        if rule.manifest.is_none() && rule.lockfiles.is_empty() {
            eprintln!(
                "workspace-isolation-check: [isolation.{name}] declares neither `manifest` nor `lockfiles`."
            );
            return ExitCode::from(2);
        }
        ran += 1;

        if let Some(manifest) = &rule.manifest {
            match check_cargo(&rule, manifest) {
                Ok(true) => {}
                Ok(false) => failed = true,
                Err(e) => {
                    eprintln!("workspace-isolation-check [{}]: {e}", rule.name);
                    return ExitCode::from(2);
                }
            }
        }

        for lockfile in &rule.lockfiles {
            let Ok(text) = std::fs::read_to_string(lockfile) else {
                eprintln!(
                    "workspace-isolation-check [{}]: {lockfile} is missing.",
                    rule.name
                );
                eprintln!("  The rule claims to check it, so a tick here would overstate what");
                eprintln!("  ran. Repoint `lockfiles`, or drop it from the rule.");
                return ExitCode::from(2);
            };
            let hits = lock_hits(&text, &rule.forbid_names);
            if hits.is_empty() {
                println!(
                    "workspace-isolation-check: ✓ {} — {lockfile} resolves nothing forbidden",
                    rule.name
                );
                continue;
            }
            failed = true;
            eprintln!(
                "\nERROR [{}]: {lockfile} resolves {} forbidden package reference(s).\n",
                rule.name,
                hits.len()
            );
            for (line, text) in &hits {
                eprintln!("  {lockfile}:{line}  {text}");
            }
            eprintln!(
                "\n  The root workspace globs match something they should not, or a package\n\
                 \x20 under them took a dependency on the other half."
            );
            if !rule.why.is_empty() {
                eprintln!("  {}", rule.why);
            }
        }
    }

    if ran == 0 {
        eprintln!("workspace-isolation-check: --rule matched no declared rule.");
        return ExitCode::from(2);
    }
    if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

/// `Ok(true)` = clean, `Ok(false)` = leaked, `Err` = could not run.
fn check_cargo(rule: &Rule, manifest: &str) -> Result<bool, String> {
    let out = Command::new("cargo")
        .args([
            "metadata",
            "--manifest-path",
            manifest,
            "--format-version",
            "1",
            "--no-deps",
        ])
        .output()
        .map_err(|e| format!("could not run cargo: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "`cargo metadata --manifest-path {manifest}` failed:\n{}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }

    let members = cargo_members(&String::from_utf8_lossy(&out.stdout))?;
    if members.is_empty() {
        return Err(format!("{manifest} resolved no workspace member"));
    }
    let leaked = leaks(&members, &rule.forbid_paths);
    if leaked.is_empty() {
        println!(
            "workspace-isolation-check: ✓ {} — {} member(s) of {manifest}, none forbidden",
            rule.name,
            members.len()
        );
        return Ok(true);
    }

    eprintln!(
        "\nERROR [{}]: {} manifest(s) leaked into {manifest}'s build graph.\n",
        rule.name,
        leaked.len()
    );
    for path in &leaked {
        eprintln!("  {path}");
    }
    eprintln!(
        "\n  Check that nothing in this graph imports across the boundary, and that the\n\
         \x20 root manifest's `members` does not reach past it."
    );
    if !rule.why.is_empty() {
        eprintln!("  {}", rule.why);
    }
    Ok(false)
}

fn build_rule(cfg: &Config, name: &str) -> Rule {
    let key = |k: &str| format!("isolation.{name}.{k}");
    Rule {
        name: name.to_string(),
        manifest: cfg.string(&key("manifest")).map(str::to_string),
        forbid_paths: cfg.list(&key("forbid_paths")).unwrap_or_default(),
        lockfiles: cfg.list(&key("lockfiles")).unwrap_or_default(),
        forbid_names: cfg.list(&key("forbid_names")).unwrap_or_default(),
        why: cfg.string(&key("why")).unwrap_or_default().to_string(),
    }
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}
