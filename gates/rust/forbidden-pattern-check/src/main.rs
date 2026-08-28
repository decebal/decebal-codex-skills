//! forbidden-pattern-check — named patterns must not appear under named paths.
//!
//! ```text
//! forbidden-pattern-check [--config gates.toml] [--rule <name>]
//! ```
//!
//! Config — one table per rule, every key optional except `patterns`:
//!
//! ```toml
//! [forbidden.keychain]
//! roots      = ["apps/desktop/src/"]
//! extensions = ["rs"]
//! patterns   = ["use\s+keyring", "keyring::Entry::new"]
//! allow      = ["apps/desktop/src/infrastructure/external/secrets.rs"]
//! why        = "Secret storage goes to the disk vault, not the OS keychain."
//!
//! [forbidden.opaque-blob-routes]
//! files    = ["api/src/http/vault.rs"]
//! patterns = ["decrypt", "argon2", "Aes256Gcm"]
//! why      = "These routes carry ciphertext only. No server-side key exists."
//! ```
//!
//! Patterns are VERBATIM — `gates-config` does no escape processing, so write
//! `"use\s+keyring"`, never `"use\\s+keyring"`. The doubled form compiles to a
//! pattern that can never fire, and the gate then calls every file clean.
//!
//! `skip_tests = "true"` drops test files from a rule. It is OFF by default:
//! a ban on a credential API or a crypto primitive means it in tests too. Turn
//! it on for a rule about LAYERING, where a fixture writing a temp file is not
//! the breach the rule is looking for.
//!
//! `roots` selects by directory prefix, `files` by exact path; a rule may use
//! either or both. `allow` exempts a path — migration code, or the one file that
//! IS the authority. Comment lines are skipped unless `include_comments = "true"`,
//! because a doc comment naming the banned symbol is usually explaining the ban.
//!
//! Exit 0 = clean. Exit 1 = a violation. Exit 2 = the gate could not run —
//! which INCLUDES a rule that selected no files.
//!
//! That last case is the whole reason this is one gate rather than the two shell
//! scripts it replaces. Both of those printed `… not found — skipping` and
//! exited 0 when their guarded paths had moved, so a rename silently retired the
//! rule and nothing said so (rules/testing-gates.md).

use gates_config::Config;
use regex::Regex;
use std::process::{Command, ExitCode};

mod scan;
use scan::{scan, Hit, Rule};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let config_path = flag(&args, "--config").unwrap_or_else(|| "gates.toml".to_string());
    let only = flag(&args, "--rule");

    let cfg = match Config::read(&config_path) {
        Ok(Some(c)) => c,
        Ok(None) => {
            eprintln!("forbidden-pattern-check: {config_path} not found.");
            return ExitCode::from(2);
        }
        Err(e) => {
            eprintln!("forbidden-pattern-check: {config_path}: {e}");
            return ExitCode::from(2);
        }
    };

    let names = cfg.tables_under("forbidden");
    if names.is_empty() {
        eprintln!("forbidden-pattern-check: {config_path} declares no [forbidden.*] rule.");
        return ExitCode::from(2);
    }

    let mut rules = Vec::new();
    for name in names {
        if only.as_deref().is_some_and(|want| want != name) {
            continue;
        }
        match build_rule(&cfg, &name) {
            Ok(rule) => rules.push(rule),
            Err(e) => {
                eprintln!("forbidden-pattern-check: [forbidden.{name}]: {e}");
                return ExitCode::from(2);
            }
        }
    }
    if rules.is_empty() {
        eprintln!("forbidden-pattern-check: --rule matched no declared rule.");
        return ExitCode::from(2);
    }

    let tracked = match tracked_files() {
        Ok(files) => files,
        Err(e) => {
            eprintln!("forbidden-pattern-check: {e}");
            return ExitCode::from(2);
        }
    };

    let mut failed = false;
    let mut starved = Vec::new();
    for rule in &rules {
        let selected: Vec<&String> = tracked.iter().filter(|p| rule.selects(p)).collect();
        if selected.is_empty() {
            starved.push(rule.name.clone());
            continue;
        }

        let mut hits: Vec<Hit> = Vec::new();
        for path in &selected {
            let Ok(content) = std::fs::read_to_string(path) else {
                continue;
            };
            hits.extend(scan(path, &content, rule));
        }

        if hits.is_empty() {
            println!(
                "forbidden-pattern-check: ✓ {} ({} file(s) clean)",
                rule.name,
                selected.len()
            );
            continue;
        }

        failed = true;
        eprintln!(
            "\nERROR [{}]: {} forbidden occurrence(s) in {} scanned file(s).\n",
            rule.name,
            hits.len(),
            selected.len()
        );
        for h in &hits {
            eprintln!("  {}:{}  {}", h.path, h.line, h.text);
        }
        if !rule.why.is_empty() {
            eprintln!("\n  {}", rule.why);
        }
        if !rule.allow.is_empty() {
            eprintln!(
                "  Exempt today: {}. Add to `allow` only for code with an end date.",
                rule.allow.join(", ")
            );
        }
    }

    if !starved.is_empty() {
        eprintln!(
            "\nforbidden-pattern-check: rule(s) selected NO files: {}",
            starved.join(", ")
        );
        eprintln!(
            "  A rule that reads nothing is indistinguishable from one that passes, so this\n\
             \x20 is a failure rather than a skip. Either the guarded paths moved — repoint\n\
             \x20 `roots`/`files` — or the rule is obsolete and belongs deleted, not stranded."
        );
        return ExitCode::from(2);
    }

    if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn build_rule(cfg: &Config, name: &str) -> Result<Rule, String> {
    let key = |k: &str| format!("forbidden.{name}.{k}");

    let raw = cfg
        .list(&key("patterns"))
        .ok_or_else(|| "no `patterns` declared".to_string())?;
    if raw.is_empty() {
        return Err("`patterns` is empty".to_string());
    }
    let mut patterns = Vec::new();
    for p in &raw {
        patterns.push(Regex::new(p).map_err(|e| format!("bad pattern `{p}`: {e}"))?);
    }

    let roots = cfg.list(&key("roots")).unwrap_or_default();
    let files = cfg.list(&key("files")).unwrap_or_default();
    if roots.is_empty() && files.is_empty() {
        return Err("declare at least one of `roots` or `files`".to_string());
    }

    Ok(Rule {
        name: name.to_string(),
        roots,
        files,
        extensions: cfg.list(&key("extensions")).unwrap_or_default(),
        patterns,
        allow: cfg.list(&key("allow")).unwrap_or_default(),
        include_comments: cfg.string(&key("include_comments")) == Some("true"),
        skip_tests: cfg.string(&key("skip_tests")) == Some("true"),
        why: cfg.string(&key("why")).unwrap_or_default().to_string(),
    })
}

/// Tracked files only: another session's untracked work-in-progress is not this
/// repo's committed source, and CI reads a clean checkout.
fn tracked_files() -> Result<Vec<String>, String> {
    let out = Command::new("git")
        .args(["ls-files"])
        .output()
        .map_err(|e| format!("could not run git: {e}"))?;
    if !out.status.success() {
        return Err("`git ls-files` failed — not a repo?".to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_string)
        .collect())
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}
