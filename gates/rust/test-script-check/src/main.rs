//! test-script-check — a package's own gates must reach its own files.
//!
//! ```text
//! test-script-check [--config gates.toml] [--root <dir>]
//! ```
//!
//! ```toml
//! [test-scripts]
//! # Files that, alongside package.json, mark the workspace root.
//! root_markers = ["turbo.json"]
//!
//! [test-scripts.script.test]
//! stems      = [".test", "_test", ".spec", "_spec", ".vitest"]
//! extensions = ["ts", "tsx", "js", "jsx", "mts", "cts", "mjs", "cjs"]
//! fix        = "add a \"test\" script — \"bun test\", or \"vitest run\""
//!
//! [test-scripts.script.typecheck]
//! extensions = ["ts", "tsx", "mts", "cts", "svelte"]
//! fix        = "add \"typecheck\": \"tsc --noEmit\" (svelte-check where .svelte exists)"
//! ```
//!
//! A requirement WITH `stems` asks "does this package own suites?"; one without
//! asks "does it own files of these extensions at all?". That difference is the
//! whole config surface — everything else is the same question twice.
//!
//! Exit 0 = clean. Exit 1 = a package owns files no script reaches.
//! Exit 2 = the gate could not run.

use gates_config::Config;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

mod scan;
use scan::{scan, Package, Requirement};

/// Violation lists stay readable; the fix is per-package, not per-file.
const MAX_LISTED_FILES: usize = 5;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let config_path = flag(&args, "--config").unwrap_or_else(|| "gates.toml".to_string());

    let cfg = match Config::read(&config_path) {
        Ok(Some(c)) => c,
        Ok(None) => Config::default(),
        Err(e) => {
            eprintln!("test-script-check: {config_path}: {e}");
            return ExitCode::from(2);
        }
    };

    let requirements = build_requirements(&cfg);
    if requirements.is_empty() {
        eprintln!("test-script-check: {config_path} declares no [test-scripts.script.*] table.");
        return ExitCode::from(2);
    }

    let markers = cfg.list("test-scripts.root_markers").unwrap_or_default();
    let root = match flag(&args, "--root").map(PathBuf::from) {
        Some(explicit) => explicit,
        None => match repo_root(&markers) {
            Some(root) => root,
            None => {
                eprintln!("test-script-check: could not locate the workspace root.");
                eprintln!("  Looked for package.json beside: {}", markers.join(", "));
                return ExitCode::from(2);
            }
        },
    };

    let packages = match scan(&root) {
        Ok(packages) => packages,
        Err(why) => {
            eprintln!("test-script-check: {why}");
            return ExitCode::from(2);
        }
    };
    if packages.is_empty() {
        eprintln!("test-script-check: the workspace globs matched no package.");
        return ExitCode::from(2);
    }

    let mut message = String::new();
    let mut total = 0usize;
    for requirement in &requirements {
        let offenders: Vec<&Package> = packages
            .iter()
            .filter(|p| !p.declares(&requirement.script) && !p.evidence(requirement).is_empty())
            .collect();
        if offenders.is_empty() {
            continue;
        }
        total += offenders.len();
        message.push_str(&format!(
            "\n{} package(s) own files a `{}` script would cover, and declare none:\n\n",
            offenders.len(),
            requirement.script
        ));
        for package in offenders {
            push_violation(&mut message, package, requirement);
        }
    }

    if total == 0 {
        println!(
            "test-script-check: ✓ {} package(s), {} requirement(s) satisfied",
            packages.len(),
            requirements.len()
        );
        return ExitCode::SUCCESS;
    }

    eprint!("{message}");
    eprintln!(
        "A task runner's tasks are global: it runs whatever packages DECLARE the script\n\
         and resolves the rest to nothing. These files are checked by no gate anywhere,\n\
         which looks exactly like a gate that passes."
    );
    ExitCode::from(1)
}

fn push_violation(out: &mut String, package: &Package, requirement: &Requirement) {
    let evidence = package.evidence(requirement);
    out.push_str(&format!(
        "  {} ({}) — {} file(s), reached by nothing:\n",
        package.name,
        package.dir.display(),
        evidence.len()
    ));
    for file in evidence.iter().take(MAX_LISTED_FILES) {
        out.push_str(&format!("      {}\n", file.display()));
    }
    if evidence.len() > MAX_LISTED_FILES {
        out.push_str(&format!(
            "      … and {} more\n",
            evidence.len() - MAX_LISTED_FILES
        ));
    }
    out.push_str(&format!(
        "    Fix: in {}/package.json, {}\n\n",
        package.dir.display(),
        requirement.fix
    ));
}

fn build_requirements(cfg: &Config) -> Vec<Requirement> {
    cfg.tables_under("test-scripts.script")
        .into_iter()
        .filter_map(|script| {
            let key = |k: &str| format!("test-scripts.script.{script}.{k}");
            let extensions = cfg.list(&key("extensions"))?;
            if extensions.is_empty() {
                return None;
            }
            Some(Requirement {
                stems: cfg.list(&key("stems")).unwrap_or_default(),
                extensions,
                fix: cfg
                    .string(&key("fix"))
                    .unwrap_or("declare the script")
                    .to_string(),
                script,
            })
        })
        .collect()
}

/// The workspace root carries `package.json` plus every declared marker.
fn repo_root(markers: &[String]) -> Option<PathBuf> {
    let is_root = |dir: &Path| {
        dir.join("package.json").is_file() && markers.iter().all(|m| dir.join(m).is_file())
    };

    let cwd = PathBuf::from(".");
    if is_root(&cwd) {
        return Some(cwd);
    }
    let mut dir = std::env::current_dir().ok()?;
    while !is_root(&dir) {
        if !dir.pop() {
            return None;
        }
    }
    Some(dir)
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}
