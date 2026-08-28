//! CLI: `rust-effective-diff <old-file> <new-file>`
//!
//! Reads two versions of a Rust source file and exits:
//!   0  — the change is code-effective (run the Rust gates)
//!   1  — the change is comment/doc/whitespace/formatting-only (safe to skip)
//!   2  — usage error
//!
//! Exit 0 (effective) is the safe default: any read error, missing file, or
//! non-Rust input makes the caller run the gates rather than skip them.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: rust-effective-diff <old-file> <new-file>");
        return ExitCode::from(2);
    }
    let old = std::fs::read_to_string(&args[1]);
    let new = std::fs::read_to_string(&args[2]);
    match (old, new) {
        (Ok(old), Ok(new)) => {
            if rust_effective_diff::is_effective(&old, &new) {
                ExitCode::from(0)
            } else {
                ExitCode::from(1)
            }
        }
        // Could not read a version — be conservative, report effective.
        _ => ExitCode::from(0),
    }
}
