//! codex-guard — the guard-rail hooks as one binary.
//!
//! ```text
//! codex-guard infra-guard        # PreToolUse, Bash
//! codex-guard bash-hygiene       # PreToolUse, Bash
//! codex-guard comment-hygiene    # PostToolUse, apply_patch
//! codex-guard trust '<command>'  # vet a wrapper chain, record the judgement
//! ```
//!
//! Each hook subcommand reads one JSON payload on stdin. Exit 0 with no output
//! is an allow, which is the common case and costs the model nothing.
//!
//! An unparseable payload allows. A guard that crashed on a surprising field
//! shape would fail closed on every tool call, and that blast radius is worse
//! than the miss.

mod bash_hygiene;
mod comment_hygiene;
mod hook;
mod infra;
mod trust;

#[cfg(test)]
#[path = "payload_size_tests.rs"]
mod payload_size_tests;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let sub = args.first().map(String::as_str).unwrap_or("");

    if sub == "trust" {
        return trust::run(args.get(1).map(String::as_str).unwrap_or(""));
    }

    let hookish = matches!(sub, "infra-guard" | "bash-hygiene" | "comment-hygiene");
    if !hookish {
        eprintln!("usage: codex-guard <infra-guard|bash-hygiene|comment-hygiene|trust>");
        return ExitCode::from(2);
    }

    let Some(payload) = hook::read_payload() else {
        return ExitCode::SUCCESS;
    };

    match sub {
        "infra-guard" => infra::run(&payload),
        "bash-hygiene" => bash_hygiene::run(&payload),
        "comment-hygiene" => comment_hygiene::run(&payload),
        _ => unreachable!("checked above"),
    }
}
