//! vendor-attribution-check — a vendored dependency must carry its attribution.
//!
//! ```text
//! vendor-attribution-check [--config gates.toml]
//! ```
//!
//! ```toml
//! [vendoring]
//! roots  = ["third_party/"]
//! notice = "third_party/NOTICE"
//! ```
//!
//! Per vendored package — a directory under a root holding a `Cargo.toml` or a
//! `package.json` — three things must be true:
//!
//! 1. an upstream `LICENSE` / `LICENCE` / `COPYING` file sits beside the manifest,
//! 2. the manifest declares a `license` (or `license-file`),
//! 3. the package's name appears in the NOTICE.
//!
//! Exit 0 = clean. Exit 1 = a package is missing one of the three.
//! Exit 2 = the gate could not run, INCLUDING "no vendored package found" — a
//! `third_party/` that moved must not read as a repo that vendors nothing.

use gates_config::Config;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

mod manifest;
use manifest::{is_license_file, kind_of, license, name, Kind};

struct Package {
    dir: PathBuf,
    manifest: PathBuf,
    kind: Kind,
}

struct Failure {
    package: String,
    problems: Vec<String>,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let config_path = flag(&args, "--config").unwrap_or_else(|| "gates.toml".to_string());

    let cfg = match Config::read(&config_path) {
        Ok(Some(c)) => c,
        Ok(None) => Config::default(),
        Err(e) => {
            eprintln!("vendor-attribution-check: {config_path}: {e}");
            return ExitCode::from(2);
        }
    };

    let roots = cfg
        .list("vendoring.roots")
        .unwrap_or_else(|| vec!["third_party/".to_string()]);
    let notice_path = cfg
        .string("vendoring.notice")
        .unwrap_or("third_party/NOTICE")
        .to_string();

    let notice = match std::fs::read_to_string(&notice_path) {
        Ok(text) => text,
        Err(e) => {
            eprintln!("vendor-attribution-check: cannot read {notice_path}: {e}");
            eprintln!("  The NOTICE is where attribution is RETAINED. Without it there is");
            eprintln!("  nothing to check a vendored package's name against.");
            return ExitCode::from(2);
        }
    };

    let mut packages = Vec::new();
    for root in &roots {
        collect(Path::new(root), &mut packages);
    }
    if packages.is_empty() {
        eprintln!(
            "vendor-attribution-check: no vendored package under: {}",
            roots.join(", ")
        );
        eprintln!("  Either the vendoring root moved — repoint [vendoring] roots — or nothing");
        eprintln!("  is vendored any more, in which case delete the root and this config.");
        return ExitCode::from(2);
    }

    let mut failures = Vec::new();
    for package in &packages {
        let Ok(text) = std::fs::read_to_string(&package.manifest) else {
            failures.push(Failure {
                package: package.manifest.display().to_string(),
                problems: vec!["manifest is unreadable".to_string()],
            });
            continue;
        };
        let declared = name(&text, package.kind);
        let label = declared
            .clone()
            .unwrap_or_else(|| package.dir.display().to_string());

        let mut problems = Vec::new();
        if !has_license_file(&package.dir) {
            problems.push(format!(
                "no LICENSE file beside {}",
                package.manifest.display()
            ));
        }
        if license(&text, package.kind).is_none() {
            problems.push(format!(
                "{} declares no `license`",
                package.manifest.display()
            ));
        }
        match &declared {
            None => problems.push(format!(
                "{} declares no package name, so it cannot be matched to the NOTICE",
                package.manifest.display()
            )),
            Some(n) if !notice.contains(n.as_str()) => {
                problems.push(format!("not recorded in {notice_path}"));
            }
            Some(_) => {}
        }

        if !problems.is_empty() {
            failures.push(Failure {
                package: label,
                problems,
            });
        }
    }

    if failures.is_empty() {
        println!(
            "vendor-attribution-check: ✓ {} vendored package(s), all attributed",
            packages.len()
        );
        return ExitCode::SUCCESS;
    }

    eprintln!(
        "\nERROR: {} of {} vendored package(s) are missing attribution.\n",
        failures.len(),
        packages.len()
    );
    for failure in &failures {
        eprintln!("  {}", failure.package);
        for problem in &failure.problems {
            eprintln!("      - {problem}");
        }
    }
    eprintln!(
        "\nVendoring copies someone else's code in without a package manager recording\n\
         where it came from, so the attribution has to be carried by hand — and nothing\n\
         else fails when it is not."
    );
    ExitCode::from(1)
}

fn has_license_file(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    entries.flatten().any(|e| {
        e.file_name()
            .to_str()
            .is_some_and(|n| is_license_file(n) && e.path().is_file())
    })
}

/// Every directory under `dir` holding a recognised manifest.
///
/// A vendored package's own dependencies are not descended into: the nested copy
/// belongs to its parent's upstream, and flagging it would demand a NOTICE entry
/// for something this repo never chose to vendor.
fn collect(dir: &Path, out: &mut Vec<Package>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut subdirs = Vec::new();
    let mut found = None;
    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = entry.file_name();
        let Some(file_name) = file_name.to_str() else {
            continue;
        };
        if path.is_dir() {
            if !matches!(file_name, "node_modules" | "target" | "vendor" | ".git") {
                subdirs.push(path);
            }
            continue;
        }
        if let Some(kind) = kind_of(file_name) {
            found = Some(Package {
                dir: dir.to_path_buf(),
                manifest: path,
                kind,
            });
        }
    }
    if let Some(package) = found {
        out.push(package);
        return;
    }
    subdirs.sort();
    for sub in subdirs {
        collect(&sub, out);
    }
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}
