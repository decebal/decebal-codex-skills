//! layer-boundary-check — the dependency direction, enforced.
//!
//! ```text
//! layer-boundary-check [--config gates.toml]
//! ```
//!
//! ```toml
//! [layers]
//! # The dependency direction. A layer may import only layers AFTER it.
//! order  = ["presentation", "application", "infrastructure"]
//! # Layers outside the ordering that may import no other layer at all.
//! pure   = ["domain"]
//! # How an import of layer X is spelled. `{layer}` is the layer's NAME.
//! import = "use crate::{layer}"
//!
//! [layers.dir]
//! presentation   = "apps/desktop/src/presentation/"
//! application    = "apps/desktop/src/application/"
//! infrastructure = "apps/desktop/src/infrastructure/"
//! domain         = "apps/desktop/src/domain/"
//!
//! # What you cannot fix today. Absent means 0.
//! [layers.ceiling]
//! presentation_to_infrastructure = "12"
//!
//! # The single sanctioned seam across an edge, exempt from its ceiling.
//! [layers.facade]
//! presentation_to_infrastructure = "apps/desktop/src/presentation/infra.rs"
//! ```
//!
//! Exit 0 = clean. Exit 1 = a violation, or a ceiling that is now too loose.
//! Exit 2 = the gate could not run.
//!
//! ## Why beating a ceiling also fails
//!
//! A ceiling is a ratchet, and a ratchet that only ever catches increases is
//! just a permanent exemption with a number on it. When the real count drops
//! below the declared ceiling the gate fails and names the new number, so the
//! commit that removed the violation is the commit that tightens the bound.
//! Otherwise the ceiling records the worst the codebase has ever been, forever,
//! and re-opens silently the next time someone adds one back.

use gates_config::Config;
use std::collections::BTreeMap;
use std::process::{Command, ExitCode};

mod layers;
use layers::{imports_in, is_test_file, Edge, Hit, Layer, Model};

/// One layer file, attributed to exactly one layer.
struct Source {
    path: String,
    text: String,
    layer: String,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let config_path = flag(&args, "--config").unwrap_or_else(|| "gates.toml".to_string());

    let cfg = match Config::read(&config_path) {
        Ok(Some(c)) => c,
        Ok(None) => {
            eprintln!("layer-boundary-check: {config_path} not found.");
            return ExitCode::from(2);
        }
        Err(e) => {
            eprintln!("layer-boundary-check: {config_path}: {e}");
            return ExitCode::from(2);
        }
    };

    let model = match build_model(&cfg) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("layer-boundary-check: [layers]: {e}");
            return ExitCode::from(2);
        }
    };

    let tracked = match tracked_files() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("layer-boundary-check: {e}");
            return ExitCode::from(2);
        }
    };

    // Read every layer file ONCE, attributed to exactly ONE layer. A four-layer
    // model has twelve forbidden edges, so re-reading the tree per edge is
    // twelve full walks for one answer — and walking per layer instead would
    // read a file once per layer whose directory contains it, which for nested
    // directories is more than one.
    let mut sources: Vec<Source> = Vec::new();
    for path in tracked.iter().filter(|p| !is_test_file(p)) {
        let Some(layer) = model.owning_layer(path) else {
            continue;
        };
        if let Ok(text) = std::fs::read_to_string(path) {
            sources.push(Source {
                path: path.clone(),
                text,
                layer: layer.name.clone(),
            });
        }
    }
    if sources.is_empty() {
        eprintln!("layer-boundary-check: no layer file matched any [layers.dir] prefix.");
        eprintln!("  A gate that reads nothing prints the same tick as one that reads");
        eprintln!("  everything. Repoint [layers.dir] at where the layers actually live.");
        return ExitCode::from(2);
    }

    let mut breaches = 0usize;
    let mut slack: Vec<(String, usize, usize)> = Vec::new();
    let mut checked = 0usize;

    for edge in model.forbidden_edges() {
        checked += 1;
        let needle = model.import_of(&edge);
        let facade = model.facade_for(&edge);
        let hits: Vec<Hit> = sources
            .iter()
            .filter(|s| s.layer == edge.from.name)
            .filter(|s| facade != Some(s.path.as_str()))
            .flat_map(|s| imports_in(&s.path, &s.text, &needle))
            .collect();

        let ceiling = model.ceiling_for(&edge);
        if hits.len() > ceiling {
            breaches += 1;
            report_breach(&edge, &hits, ceiling, facade);
        } else if hits.len() < ceiling {
            slack.push((edge.label(), hits.len(), ceiling));
        }
    }

    for (label, actual, ceiling) in &slack {
        eprintln!(
            "\nERROR: the ceiling for {label} is {ceiling}, but only {actual} remain.\n\
             \x20 Lower it to {actual} in [layers.ceiling] — a ceiling nobody tightens stops\n\
             \x20 being a ratchet and becomes a permanent exemption, and the next violation\n\
             \x20 slips in under it without failing anything."
        );
    }

    if breaches == 0 && slack.is_empty() {
        println!(
            "layer-boundary-check: ✓ {checked} forbidden edge(s), {} file(s) read",
            sources.len()
        );
        return ExitCode::SUCCESS;
    }
    ExitCode::from(1)
}

fn report_breach(edge: &Edge, hits: &[Hit], ceiling: usize, facade: Option<&str>) {
    eprintln!(
        "\nERROR: {} — {} import(s), ceiling {}.\n",
        edge.label(),
        hits.len(),
        ceiling
    );
    for h in hits {
        eprintln!("  {}:{}  {}", h.path, h.line, h.text);
    }
    match facade {
        Some(f) => eprintln!(
            "\n  {f} is the one sanctioned seam across this edge. Route through it,\n\
             \x20 so a later migration shrinks one file instead of chasing import paths."
        ),
        None => eprintln!(
            "\n  The direction only points one way. Pass what the caller needs as a\n\
             \x20 PARAMETER, or move the shared type down into the layer both can see."
        ),
    }
}

fn build_model(cfg: &Config) -> Result<Model, String> {
    let names = cfg
        .list("layers.order")
        .ok_or_else(|| "no `order` declared".to_string())?;
    if names.len() < 2 {
        return Err("`order` needs at least two layers to have a direction".to_string());
    }
    let pure_names = cfg.list("layers.pure").unwrap_or_default();

    let resolve = |name: &String| -> Result<Layer, String> {
        let dir = cfg
            .string(&format!("layers.dir.{name}"))
            .ok_or_else(|| format!("layer `{name}` has no [layers.dir] entry"))?;
        Ok(Layer {
            name: name.clone(),
            dir: dir.to_string(),
        })
    };

    let order = names.iter().map(resolve).collect::<Result<Vec<_>, _>>()?;
    let pure = pure_names
        .iter()
        .map(resolve)
        .collect::<Result<Vec<_>, _>>()?;

    let mut ceilings = BTreeMap::new();
    for edge_key in cfg.keys_under("layers.ceiling") {
        let raw = cfg
            .string(&format!("layers.ceiling.{edge_key}"))
            .unwrap_or_default();
        let n: usize = raw
            .parse()
            .map_err(|_| format!("ceiling `{edge_key}` is not a number: {raw}"))?;
        ceilings.insert(edge_key, n);
    }

    let mut facades = BTreeMap::new();
    for edge_key in cfg.keys_under("layers.facade") {
        if let Some(path) = cfg.string(&format!("layers.facade.{edge_key}")) {
            facades.insert(edge_key, path.to_string());
        }
    }

    Ok(Model {
        order,
        pure,
        import_template: cfg
            .string("layers.import")
            .unwrap_or("use crate::{layer}")
            .to_string(),
        ceilings,
        facades,
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
