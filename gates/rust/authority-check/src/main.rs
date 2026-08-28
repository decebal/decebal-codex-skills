//! authority-check — one file owns a derived verdict; the rest render it.
//!
//! ```text
//! authority-check [--config gates.toml] [--rule <name>]
//! ```
//!
//! ```toml
//! [authority.tool-capability]
//! # The file that DECIDES. Exempt from `markers`, never from `retired`.
//! authority  = "apps/desktop/src/application/model_capabilities.rs"
//! # Trees that may only RENDER the verdict.
//! renderers  = ["apps/app/src/", "packages/"]
//! extensions = ["ts", "svelte"]
//! # Pieces of the authority's logic. Naming one in a renderer is re-derivation.
//! markers    = ["TOOL_CAPABLE_PREFIXES", "modelSupportsTools"]
//! # The one file allowed to name the verdict's WIRE shape — it is the contract,
//! # not a second implementation of it.
//! contract   = "apps/app/src/lib/infrastructure/tauri/llm-api.ts"
//! # Symbols retired outright. Forbidden everywhere, authority included.
//! retired    = ["disabled_workflow_only"]
//! why        = "Capability is decided once and travels as `tool_capable`."
//! ```
//!
//! Exit 0 = clean. Exit 1 = a violation. Exit 2 = the gate could not run.
//!
//! ## What exit 2 covers, and why it is not paranoia
//!
//! Three ways this gate can guard nothing while printing a tick, all of them
//! failures here: the renderer roots select no files, the authority file does
//! not exist, or the authority no longer DEFINES a marker. The last is the
//! subtle one — a ban list outliving its subject reads exactly like a clean
//! codebase, and the rename that orphaned it is invisible.

use gates_config::Config;
use std::process::{Command, ExitCode};

mod rule;
use rule::{scan, undefined_markers, Hit, Kind, Rule};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let config_path = flag(&args, "--config").unwrap_or_else(|| "gates.toml".to_string());
    let only = flag(&args, "--rule");

    let cfg = match Config::read(&config_path) {
        Ok(Some(c)) => c,
        Ok(None) => {
            eprintln!("authority-check: {config_path} not found.");
            return ExitCode::from(2);
        }
        Err(e) => {
            eprintln!("authority-check: {config_path}: {e}");
            return ExitCode::from(2);
        }
    };

    let names = cfg.tables_under("authority");
    if names.is_empty() {
        eprintln!("authority-check: {config_path} declares no [authority.*] rule.");
        return ExitCode::from(2);
    }

    let tracked = match tracked_files() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("authority-check: {e}");
            return ExitCode::from(2);
        }
    };

    let mut failed = false;
    let mut ran = 0usize;

    for name in names {
        if only.as_deref().is_some_and(|want| want != name) {
            continue;
        }
        let rule = match build_rule(&cfg, &name) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("authority-check: [authority.{name}]: {e}");
                return ExitCode::from(2);
            }
        };
        ran += 1;

        let Ok(authority_text) = std::fs::read_to_string(&rule.authority) else {
            eprintln!(
                "authority-check [{}]: the authority {} does not exist.",
                rule.name, rule.authority
            );
            eprintln!("  Repoint `authority`, or delete the rule. A rule whose subject is gone");
            eprintln!("  guards nothing and reads exactly like a clean codebase.");
            return ExitCode::from(2);
        };

        let missing = undefined_markers(&authority_text, &rule);
        if !missing.is_empty() {
            eprintln!(
                "authority-check [{}]: {} no longer defines: {}",
                rule.name,
                rule.authority,
                missing.join(", ")
            );
            eprintln!("  Either the marker was renamed — update `markers` — or the authority");
            eprintln!("  moved. Until then this rule bans a symbol nothing produces.");
            return ExitCode::from(2);
        }

        let selected: Vec<&String> = tracked
            .iter()
            .filter(|p| rule.selects(p))
            .filter(|p| !rule.is_exempt_test(p))
            .collect();
        if selected.is_empty() {
            eprintln!(
                "authority-check [{}]: `renderers` selected NO files.",
                rule.name
            );
            eprintln!("  A gate that reads nothing prints the same tick as one that reads all.");
            return ExitCode::from(2);
        }

        // The authority is read for `retired` even when it sits OUTSIDE the
        // renderer roots — that is the half of the rule it is not exempt from.
        // It is read only once when it sits inside them, which it legitimately
        // may: scanning it twice would report every retired symbol in it twice,
        // and a doubled count is the kind of wrong that reads as two problems.
        let mut hits: Vec<Hit> = Vec::new();
        for path in &selected {
            if let Ok(text) = std::fs::read_to_string(path) {
                hits.extend(scan(path, &text, &rule));
            }
        }
        if !selected.contains(&&rule.authority) {
            hits.extend(scan(&rule.authority, &authority_text, &rule));
        }

        if hits.is_empty() {
            println!(
                "authority-check: ✓ {} ({} renderer file(s) clean)",
                rule.name,
                selected.len()
            );
            continue;
        }

        failed = true;
        report(&rule, &hits, selected.len());
    }

    if ran == 0 {
        eprintln!("authority-check: --rule matched no declared rule.");
        return ExitCode::from(2);
    }
    if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn report(rule: &Rule, hits: &[Hit], scanned: usize) {
    eprintln!(
        "\nERROR [{}]: {} violation(s) across {} scanned file(s).\n",
        rule.name,
        hits.len(),
        scanned
    );
    for h in hits {
        let what = match h.kind {
            Kind::Rederived => "re-derives the authority",
            Kind::Retired => "names a retired symbol",
        };
        eprintln!("  {}:{}  `{}` — {}", h.path, h.line, h.marker, what);
    }
    eprintln!("\n  Authority: {}", rule.authority);
    if let Some(contract) = &rule.contract {
        eprintln!("  Wire contract: {contract}");
    }
    if !rule.why.is_empty() {
        eprintln!("  {}", rule.why);
    }
    eprintln!(
        "  A second copy of the verdict does not disagree loudly — it disagrees on\n\
         \x20 whichever inputs the copier did not think of, and only for those users."
    );
}

fn build_rule(cfg: &Config, name: &str) -> Result<Rule, String> {
    let key = |k: &str| format!("authority.{name}.{k}");

    let authority = cfg
        .string(&key("authority"))
        .ok_or_else(|| "no `authority` declared".to_string())?
        .to_string();
    let renderers = cfg
        .list(&key("renderers"))
        .ok_or_else(|| "no `renderers` declared".to_string())?;
    if renderers.is_empty() {
        return Err("`renderers` is empty".to_string());
    }
    let markers = cfg.list(&key("markers")).unwrap_or_default();
    let retired = cfg.list(&key("retired")).unwrap_or_default();
    if markers.is_empty() && retired.is_empty() {
        return Err("declare at least one of `markers` or `retired`".to_string());
    }

    Ok(Rule {
        name: name.to_string(),
        authority,
        renderers,
        extensions: cfg.list(&key("extensions")).unwrap_or_default(),
        markers,
        contract: cfg.string(&key("contract")).map(str::to_string),
        retired,
        why: cfg.string(&key("why")).unwrap_or_default().to_string(),
    })
}

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
