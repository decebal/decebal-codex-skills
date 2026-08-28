//! trophy-check — every promised test must be a REAL test.
//!
//! ```text
//! trophy-check [--config gates.toml] [--run] [--strict]
//! ```
//!
//! ```toml
//! [trophy]
//! manifest_path = "apps/desktop/Cargo.toml"
//! packages      = ["my-app"]              # empty selects --workspace
//! features      = ["my-app/test-support"]
//!
//! [trophy.entry.us-001-unit]
//! story       = "US-001"
//! layer       = "unit"
//! pattern     = "step_run & records_id"
//! block_merge = "true"
//! desc        = "A step run records its id"
//! ```
//!
//! `--run` also EXECUTES the matched tests. `--strict` makes any missing or
//! failing line fail, not only the `block_merge` ones — so the manifest can list
//! the whole plan from day one and go green as the work lands.
//!
//! Exit 0 = satisfied. Exit 1 = a promised test is missing or failing.
//! Exit 2 = the gate could not run.
//!
//! ## Why presence is the default and running is a flag
//!
//! Listing compiles the test binaries but runs nothing, which fits inside a
//! hook's time budget; running does not, for any suite worth having. Presence
//! catches the failure this exists for — a plan whose testing section was
//! satisfied by writing "tests added" — and `--run` belongs in CI, where the
//! suite is running anyway.

use gates_config::Config;
use std::collections::BTreeSet;
use std::process::ExitCode;

mod manifest;
mod nextest;
use manifest::{matches_for, Entry, TestCase};
use nextest::Scope;

#[derive(PartialEq)]
enum Status {
    Ok,
    Missing,
    Failing,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let config_path = flag(&args, "--config").unwrap_or_else(|| "gates.toml".to_string());
    let do_run = args.iter().any(|a| a == "--run");
    let strict = args.iter().any(|a| a == "--strict");

    let cfg = match Config::read(&config_path) {
        Ok(Some(c)) => c,
        Ok(None) => {
            eprintln!("trophy-check: {config_path} not found.");
            return ExitCode::from(2);
        }
        Err(e) => {
            eprintln!("trophy-check: {config_path}: {e}");
            return ExitCode::from(2);
        }
    };

    let entries = build_entries(&cfg);
    if entries.is_empty() {
        eprintln!("trophy-check: {config_path} declares no [trophy.entry.*] table.");
        eprintln!("  An empty manifest is satisfied by an empty test suite.");
        return ExitCode::from(2);
    }

    let scope = Scope {
        manifest_path: cfg
            .string("trophy.manifest_path")
            .unwrap_or("Cargo.toml")
            .to_string(),
        packages: cfg.list("trophy.packages").unwrap_or_default(),
        features: cfg.list("trophy.features").unwrap_or_default(),
    };

    let tests = match nextest::list(&scope) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("trophy-check: {e}");
            return ExitCode::from(2);
        }
    };
    if tests.is_empty() {
        eprintln!("trophy-check: the runner enumerated NO tests.");
        eprintln!("  Check `packages` / `features` — a suite that lists nothing satisfies no");
        eprintln!("  pattern, and would report every promised line as missing for one reason.");
        return ExitCode::from(2);
    }

    // Run every matched test in ONE invocation. Per-entry runs would recompile
    // nothing but would re-spawn the runner once per line, and one line's tests
    // routinely overlap another's.
    let mut matched: Vec<(usize, Vec<&TestCase>)> = Vec::new();
    for (i, entry) in entries.iter().enumerate() {
        matched.push((i, matches_for(entry, &tests)));
    }

    let failed_names = if do_run {
        let selected: BTreeSet<String> = matched
            .iter()
            .flat_map(|(_, hits)| hits.iter().map(|t| t.testcase.clone()))
            .collect();
        if selected.is_empty() {
            BTreeSet::new()
        } else {
            match nextest::run(&scope, &nextest::union_expr(&selected)) {
                Ok(outcome) if outcome.ok => BTreeSet::new(),
                Ok(outcome) => {
                    // A test can fail without its name reaching a FAIL line —
                    // a panic in a fixture, a binary that aborts. Printing the
                    // runner's own output means the operator is never left with
                    // "something failed" and no way to find out what.
                    eprintln!("{}", outcome.output.trim_end());
                    outcome.failed
                }
                Err(e) => {
                    eprintln!("trophy-check: {e}");
                    return ExitCode::from(2);
                }
            }
        }
    } else {
        BTreeSet::new()
    };

    let mut blocking = 0usize;
    let mut advisory = 0usize;
    println!(
        "trophy-check: {} promised line(s), {} test(s) enumerated\n",
        entries.len(),
        tests.len()
    );

    for (i, hits) in &matched {
        let entry = &entries[*i];
        let status = if hits.is_empty() {
            Status::Missing
        } else if hits.iter().any(|t| failed_names.contains(&t.testcase)) {
            Status::Failing
        } else {
            Status::Ok
        };

        let mark = match status {
            Status::Ok => "✓",
            Status::Missing => "MISSING",
            Status::Failing => "FAILING",
        };
        println!(
            "  {mark:<8} [{}] {} — {} ({} match(es))",
            entry.layer,
            entry.story,
            entry.desc,
            hits.len()
        );
        if status == Status::Ok {
            continue;
        }
        println!("           pattern: {}", entry.pattern);
        if entry.block_merge || strict {
            blocking += 1;
        } else {
            advisory += 1;
        }
    }

    if blocking == 0 {
        if advisory > 0 {
            println!(
                "\ntrophy-check: {advisory} line(s) not yet satisfied, none blocking. \
                 Re-run with --strict to fail on them."
            );
        }
        return ExitCode::SUCCESS;
    }

    eprintln!(
        "\nERROR: {blocking} promised test(s) are missing or failing.\n\n\
         A plan's testing section is a checklist, and a checklist is satisfied by writing\n\
         \"tests added\". Each line above names a test the runner cannot find or cannot\n\
         pass — so write it, or correct the pattern if the test exists under another name."
    );
    if !do_run {
        eprintln!("(Presence only. Re-run with --run to execute the matched tests.)");
    }
    ExitCode::from(1)
}

fn build_entries(cfg: &Config) -> Vec<Entry> {
    cfg.tables_under("trophy.entry")
        .into_iter()
        .filter_map(|id| {
            let key = |k: &str| format!("trophy.entry.{id}.{k}");
            let pattern = cfg.string(&key("pattern"))?.to_string();
            Some(Entry {
                story: cfg.string(&key("story")).unwrap_or(&id).to_string(),
                layer: cfg.string(&key("layer")).unwrap_or("test").to_string(),
                pattern,
                block_merge: cfg.string(&key("block_merge")) == Some("true"),
                desc: cfg.string(&key("desc")).unwrap_or(&id).to_string(),
                id,
            })
        })
        .collect()
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}
