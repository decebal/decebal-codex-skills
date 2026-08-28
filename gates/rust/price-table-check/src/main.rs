//! price-table-check — a shipped price table, against a published reference.
//!
//! ```text
//! price-table-check --ours <file.json> --reference <file.json> [--config gates.toml]
//! ```
//!
//! ```toml
//! [price-table]
//! ours_map        = "models"
//! ours_input      = "input_per_million"
//! ours_output     = "output_per_million"
//! reference_map   = ""
//! reference_input = "input_cost_per_token"
//! reference_output = "output_cost_per_token"
//! # Multiplier that brings the reference into our units. Per-token -> per-million.
//! reference_scale = "1000000"
//! tolerance       = "0.0001"
//! # Ids whose price legitimately differs — a negotiated rate, a bundled model.
//! allow           = ["local/bundled"]
//! ```
//!
//! Exit 0 = agree. Exit 1 = a price disagrees. Exit 2 = the gate could not run.
//!
//! ## What it deliberately does not fail on
//!
//! Only the INTERSECTION is compared. A model the reference has not published
//! yet says nothing about whether our price is right, and failing on it would
//! break the gate every time a vendor ships — so those ids are printed and
//! counted, never failed. The gate exists to catch a price that is wrong, not to
//! keep a reference exhaustive.

use gates_config::Config;
use std::process::ExitCode;

mod prices;
use prices::{disagreements, extract, uncovered, Shape, Table};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let config_path = flag(&args, "--config").unwrap_or_else(|| "gates.toml".to_string());

    let (Some(ours_path), Some(reference_path)) =
        (flag(&args, "--ours"), flag(&args, "--reference"))
    else {
        eprintln!("usage: price-table-check --ours <file.json> --reference <file.json>");
        eprintln!("                         [--config gates.toml]");
        return ExitCode::from(2);
    };

    let cfg = match Config::read(&config_path) {
        Ok(Some(c)) => c,
        Ok(None) => Config::default(),
        Err(e) => {
            eprintln!("price-table-check: {config_path}: {e}");
            return ExitCode::from(2);
        }
    };

    let ours_shape = Shape {
        map_path: cfg.string("price-table.ours_map").unwrap_or("").to_string(),
        input_key: cfg
            .string("price-table.ours_input")
            .unwrap_or("input")
            .to_string(),
        output_key: cfg
            .string("price-table.ours_output")
            .unwrap_or("output")
            .to_string(),
        scale: 1.0,
    };
    let scale = match number(&cfg, "price-table.reference_scale", 1.0) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("price-table-check: {e}");
            return ExitCode::from(2);
        }
    };
    let reference_shape = Shape {
        map_path: cfg
            .string("price-table.reference_map")
            .unwrap_or("")
            .to_string(),
        input_key: cfg
            .string("price-table.reference_input")
            .unwrap_or("input")
            .to_string(),
        output_key: cfg
            .string("price-table.reference_output")
            .unwrap_or("output")
            .to_string(),
        scale,
    };
    let tolerance = match number(&cfg, "price-table.tolerance", 0.0) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("price-table-check: {e}");
            return ExitCode::from(2);
        }
    };
    let allow = cfg.list("price-table.allow").unwrap_or_default();

    let ours = match load(&ours_path, &ours_shape) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("price-table-check: ours ({ours_path}): {e}");
            return ExitCode::from(2);
        }
    };
    let theirs = match load(&reference_path, &reference_shape) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("price-table-check: reference ({reference_path}): {e}");
            return ExitCode::from(2);
        }
    };

    if ours.is_empty() {
        eprintln!("price-table-check: our table holds NO priced entries.");
        eprintln!("  Check `ours_map` / `ours_input` / `ours_output` — an empty table agrees");
        eprintln!("  with every reference, so this would pass while comparing nothing.");
        return ExitCode::from(2);
    }
    if theirs.is_empty() {
        eprintln!("price-table-check: the reference holds NO priced entries.");
        eprintln!("  Check `reference_map` / the key names and the `reference_scale` units.");
        return ExitCode::from(2);
    }

    let missing = uncovered(&ours, &theirs);
    let found = disagreements(&ours, &theirs, tolerance, &allow);
    let compared = ours.len() - missing.len();

    if compared == 0 {
        eprintln!("price-table-check: no id appears in BOTH tables.");
        eprintln!("  The two documents key on different id shapes, so nothing was compared.");
        return ExitCode::from(2);
    }

    if found.is_empty() {
        println!("price-table-check: ✓ {compared} price(s) agree with the reference");
        if !missing.is_empty() {
            println!(
                "price-table-check: {} id(s) the reference does not price: {}",
                missing.len(),
                missing.join(", ")
            );
        }
        return ExitCode::SUCCESS;
    }

    eprintln!(
        "\nERROR: {} of {compared} compared price(s) disagree with the reference.\n",
        found.len()
    );
    eprintln!("  {:<34}{:>12}{:>12}", "id", "ours", "reference");
    for d in &found {
        eprintln!(
            "  {:<34}{:>12}{:>12}   input",
            d.id, d.ours.input, d.theirs.input
        );
        eprintln!(
            "  {:<34}{:>12}{:>12}   output",
            "", d.ours.output, d.theirs.output
        );
    }
    eprintln!(
        "\nA wrong price looks exactly like a right one in place, and every test that\n\
         reads the table agrees with it. Correct the table, or add the id to `allow`\n\
         when the difference is real — a negotiated rate is not drift."
    );
    ExitCode::from(1)
}

fn number(cfg: &Config, key: &str, default: f64) -> Result<f64, String> {
    match cfg.string(key) {
        None => Ok(default),
        Some(raw) => raw
            .parse()
            .map_err(|_| format!("`{key}` is not a number: {raw}")),
    }
}

fn load(path: &str, shape: &Shape) -> Result<Table, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read: {e}"))?;
    let doc: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("not valid JSON: {e}"))?;
    extract(&doc, shape)
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}
