//! staged-scope — which gates does this change actually need?
//!
//! ```text
//! staged-scope                    # scopes from the index (staged files)
//! staged-scope --range <ref>      # scopes from `git diff <ref>..HEAD`
//! staged-scope --config <path>    # default: gates.toml at the repo root
//! ```
//!
//! Prints one scope name per line, sorted and deduped, for the caller to switch
//! on. Two outputs are special:
//!
//! * nothing at all — no gate can be affected by this change.
//! * `unknown`      — a path matched no scope and no inert entry. The caller
//!   must expand that to "run everything"; the unmatched paths are printed on
//!   stderr so the config gets fixed.
//!
//! A modified `.rs` file whose only change is comments, doc comments, whitespace
//! or formatting is dropped before classification, via `rust-effective-diff` —
//! so rewording a module doc-comment does not drag in the heavy compile gates.
//!
//! One exception, and it is not academic: if the LINE COUNT changed, the file is
//! kept. A file-size ratchet reads source as TEXT, counting `///` and `//!` as
//! ordinary lines, so a purely additive doc-comment edit cannot change behaviour
//! and CAN push a file past its ceiling. Dropped here, such a change emitted no
//! scope, ran no gate, and put a red ratchet on the trunk for the next person.

use gates_config::Config;
use std::process::{Command, ExitCode};

mod scope;
use scope::Rules;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let range = flag(&args, "--range");
    let config_path = flag(&args, "--config").unwrap_or_else(|| "gates.toml".to_string());

    let cfg = match Config::read(&config_path) {
        Ok(Some(c)) => c,
        Ok(None) => {
            eprintln!("staged-scope: no {config_path} — every path will read as `unknown`.");
            Config::default()
        }
        Err(e) => {
            eprintln!("staged-scope: {config_path}: {e}");
            return ExitCode::from(2);
        }
    };
    let rules = Rules::from_config(&cfg);

    let mut files = match changed_files(range.as_deref()) {
        Some(f) => f,
        None => {
            eprintln!("staged-scope: git failed — not a repo?");
            return ExitCode::from(2);
        }
    };
    if files.is_empty() {
        return ExitCode::SUCCESS;
    }

    drop_comment_only_rust(&mut files, range.as_deref());
    if files.is_empty() {
        return ExitCode::SUCCESS;
    }

    let unmatched = rules.unmatched(&files);
    if !unmatched.is_empty() {
        eprintln!(
            "staged-scope: {} path(s) match no scope and no inert entry — running everything:",
            unmatched.len()
        );
        for p in &unmatched {
            eprintln!("  {p}");
        }
        eprintln!("  Add them to a [scope.*] paths list or to [gates] inert in {config_path}.");
    }
    for name in rules.classify(&files) {
        println!("{name}");
    }
    ExitCode::SUCCESS
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn changed_files(range: Option<&str>) -> Option<Vec<String>> {
    let raw = match range {
        Some(r) => git(&[
            "diff",
            "--name-only",
            "--diff-filter=ACMR",
            &format!("{r}..HEAD"),
        ])?,
        None => git(&["diff", "--cached", "--name-only", "--diff-filter=ACMR"])?,
    };
    Some(
        raw.lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(str::to_string)
            .collect(),
    )
}

/// Remove `.rs` files whose change is comment/doc/whitespace-only AND left the
/// line count unchanged.
///
/// Conservative by construction: any step that cannot be completed — no
/// classifier on PATH, a blob that will not read, an added or renamed file with
/// no old version — KEEPS the file. The wrong direction here runs a gate that
/// was not needed; the other direction skips one that was.
fn drop_comment_only_rust(files: &mut Vec<String>, range: Option<&str>) {
    if !classifier_available() {
        return;
    }
    let old_ref = range.unwrap_or("HEAD").to_string();
    files.retain(|f| {
        if !f.ends_with(".rs") {
            return true;
        }
        let old = match git(&["show", &format!("{old_ref}:{f}")]) {
            Some(t) => t,
            None => return true,
        };
        let new = match range {
            Some(_) => git(&["show", &format!("HEAD:{f}")]),
            None => git(&["show", &format!(":{f}")]),
        };
        let new = match new {
            Some(t) if !t.is_empty() => t,
            _ => return true,
        };
        match effective(&old, &new) {
            Some(false) => old.lines().count() != new.lines().count(),
            // effective, or the classifier could not answer
            _ => true,
        }
    });
}

/// Is `rust-effective-diff` on PATH? Run with no arguments it exits 2 (usage),
/// which is fine — the question is whether it SPAWNS. A missing binary is an
/// `Err` from `output()`, and the caller then keeps every file.
fn classifier_available() -> bool {
    Command::new("rust-effective-diff").output().is_ok()
}

/// `Some(true)` = the change affects compilation, `Some(false)` = it does not,
/// `None` = the classifier could not be run.
fn effective(old: &str, new: &str) -> Option<bool> {
    let dir = std::env::temp_dir().join(format!("staged-scope-{}", std::process::id()));
    std::fs::create_dir_all(&dir).ok()?;
    let old_path = dir.join("old.rs");
    let new_path = dir.join("new.rs");
    std::fs::write(&old_path, old).ok()?;
    std::fs::write(&new_path, new).ok()?;
    let status = Command::new("rust-effective-diff")
        .arg(&old_path)
        .arg(&new_path)
        .status()
        .ok();
    let _ = std::fs::remove_dir_all(&dir);
    match status?.code()? {
        0 => Some(true),
        1 => Some(false),
        _ => None,
    }
}
