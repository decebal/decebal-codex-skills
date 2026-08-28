//! contract-set-drift — two halves of a contract must declare the same SETS.
//!
//! ```text
//! contract-set-drift --left <file.json> --right <file.json> \
//!                    [--config gates.toml] [--rule <name>]
//! ```
//!
//! ```toml
//! [contract-drift.web]
//! left_label  = "api"
//! right_label = "bundle"
//! # Dotted path to the map of entries, in each document.
//! left_map    = "features"
//! right_map   = "contract.features"
//! # Within one entry, where its set lives. Empty = the entry IS the array.
//! set_key     = "operations"
//! why         = "The reading side gates on exact set equality."
//! ```
//!
//! Both documents are read from DISK. Fetch them however you like — `curl` in
//! the hook, an artifact in CI — and hand this the files. A gate that opens its
//! own socket cannot run offline and cannot be tested without one.
//!
//! Exit 0 = the sets agree. Exit 1 = drift. Exit 2 = the gate could not run.

use gates_config::Config;
use std::process::ExitCode;

mod compare;
use compare::{compare, extract, Drift, Presence, Sets};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let config_path = flag(&args, "--config").unwrap_or_else(|| "gates.toml".to_string());

    let (Some(left_path), Some(right_path)) = (flag(&args, "--left"), flag(&args, "--right"))
    else {
        eprintln!("usage: contract-set-drift --left <file.json> --right <file.json>");
        eprintln!("                          [--config gates.toml] [--rule <name>]");
        return ExitCode::from(2);
    };

    let cfg = match Config::read(&config_path) {
        Ok(Some(c)) => c,
        Ok(None) => Config::default(),
        Err(e) => {
            eprintln!("contract-set-drift: {config_path}: {e}");
            return ExitCode::from(2);
        }
    };

    let rule = match flag(&args, "--rule") {
        Some(name) => name,
        None => match cfg.tables_under("contract-drift").first() {
            Some(only) => only.clone(),
            None => {
                eprintln!("contract-set-drift: {config_path} declares no [contract-drift.*] rule.");
                return ExitCode::from(2);
            }
        },
    };
    let key = |k: &str| format!("contract-drift.{rule}.{k}");
    let left_label = cfg.string(&key("left_label")).unwrap_or("left").to_string();
    let right_label = cfg
        .string(&key("right_label"))
        .unwrap_or("right")
        .to_string();
    let set_key = cfg.string(&key("set_key")).unwrap_or_default().to_string();
    let why = cfg.string(&key("why")).unwrap_or_default().to_string();

    let left = match load(
        &left_path,
        cfg.string(&key("left_map")).unwrap_or(""),
        &set_key,
    ) {
        Ok(sets) => sets,
        Err(e) => {
            eprintln!("contract-set-drift: {left_label} ({left_path}): {e}");
            return ExitCode::from(2);
        }
    };
    let right = match load(
        &right_path,
        cfg.string(&key("right_map")).unwrap_or(""),
        &set_key,
    ) {
        Ok(sets) => sets,
        Err(e) => {
            eprintln!("contract-set-drift: {right_label} ({right_path}): {e}");
            return ExitCode::from(2);
        }
    };

    if left.is_empty() && right.is_empty() {
        eprintln!("contract-set-drift: both documents declared NO entries.");
        eprintln!("  Two empty maps compare equal, so this would pass while checking nothing.");
        eprintln!("  Check `left_map` / `right_map` against the documents' actual shape.");
        return ExitCode::from(2);
    }

    let drift = compare(&left, &right);
    if drift.is_empty() {
        println!(
            "contract-set-drift: ✓ {rule} — {} entr(ies) agree between {left_label} and {right_label}",
            left.len()
        );
        return ExitCode::SUCCESS;
    }

    eprintln!(
        "\nERROR [{rule}]: {} entr(ies) differ between {left_label} and {right_label}.\n",
        drift.len()
    );
    for d in &drift {
        report(d, &left_label, &right_label);
    }
    eprintln!(
        "\n{} is behind: {}\n{} is behind: {}",
        left_label,
        blame(&drift, Drift::blames_left),
        right_label,
        blame(&drift, Drift::blames_right)
    );
    if !why.is_empty() {
        eprintln!("\n{why}");
    }
    eprintln!(
        "A version field cannot see this — both sides can carry the same number while\n\
         their sets diverge. Only the sets are compared, so only the sets can lie."
    );
    ExitCode::from(1)
}

fn report(d: &Drift, left_label: &str, right_label: &str) {
    match d.presence {
        Presence::LeftOnly => eprintln!("  {} — present in {left_label} only", d.entry),
        Presence::RightOnly => eprintln!("  {} — present in {right_label} only", d.entry),
        Presence::Both => eprintln!("  {}", d.entry),
    }
    if !d.missing_from_left.is_empty() {
        eprintln!(
            "      {left_label} does not declare: {}",
            d.missing_from_left
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    if !d.missing_from_right.is_empty() {
        eprintln!(
            "      {right_label} does not declare: {}",
            d.missing_from_right
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
}

fn blame(drift: &[Drift], which: fn(&Drift) -> bool) -> String {
    let names: Vec<&str> = drift
        .iter()
        .filter(|d| which(d))
        .map(|d| d.entry.as_str())
        .collect();
    if names.is_empty() {
        "nothing".to_string()
    } else {
        names.join(", ")
    }
}

fn load(path: &str, map_path: &str, set_key: &str) -> Result<Sets, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read: {e}"))?;
    let doc: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("not valid JSON: {e}"))?;
    extract(&doc, map_path, set_key)
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}
