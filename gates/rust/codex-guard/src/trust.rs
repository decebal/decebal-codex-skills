//! `codex-guard trust '<command>'` — vet a wrapper chain, then record YOUR
//! judgement as a content hash.
//!
//! It does not re-judge. It records that you reviewed this exact chain, and the
//! hash covers every resolved unit — script content, make-recipe text, node
//! script text — so editing a Makefile recipe busts the trust even when no
//! script file changed. Trust never outlives the thing it was granted for.

use crate::infra::resolve;
use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

pub fn run(cmd: &str) -> ExitCode {
    if cmd.is_empty() {
        eprintln!("usage: codex-guard trust '<command>'");
        return ExitCode::from(2);
    }
    let cwd = std::env::current_dir().unwrap_or_default();
    let root = cwd
        .ancestors()
        .find(|candidate| candidate.join(".git").exists())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| cwd.clone());

    let limits = resolve::Limits::from_env();
    let scan = resolve::scan_tree(cmd, &root, &cwd, &limits, None);

    let Some(hash) = resolve::tree_hash(&scan.units) else {
        eprintln!("Nothing project-local to trust in: {cmd}");
        eprintln!(
            "(no scripts resolved under {} — nothing to hash)",
            root.display()
        );
        return ExitCode::from(1);
    };

    if resolve::is_kosher(&hash) {
        println!("Already kosher ({hash}): {cmd}");
        return ExitCode::SUCCESS;
    }

    let path = resolve::kosher_path();
    if let Some(dir) = Path::new(&path).parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let opened = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path);
    let Ok(mut file) = opened else {
        eprintln!("codex-guard trust: cannot write {}", path.display());
        return ExitCode::from(1);
    };
    if writeln!(file, "{hash}  {cmd}").is_err() {
        eprintln!("codex-guard trust: cannot write {}", path.display());
        return ExitCode::from(1);
    }

    println!("Trusted ({hash}): {cmd}");
    println!("Hashed units (file content + make/node recipe text):");
    for unit in &scan.units {
        println!("  {unit}");
    }
    if scan.risk {
        eprintln!(
            "NOTE: chain references infra tools / prod tokens — trust covers only what resolved."
        );
    }
    ExitCode::SUCCESS
}
