//! What each hook injects into model context per firing.
//!
//! This was a script that PRINTED the numbers; it is a test that holds them.
//! A report tells you the payload grew after you have already shipped it, and
//! only if someone runs it — a ceiling fails the push that grew it.
//!
//! The ceilings are generous on purpose. They catch a hook that starts pasting a
//! rule book into every turn, not a reworded sentence.

use crate::bash_hygiene;
use crate::comment_hygiene;
use crate::infra::{self, Decision};
use serde_json::json;

/// Rough token estimate at 4 bytes per token, matching the shell script's `/4`.
fn tokens(bytes: usize) -> usize {
    bytes.div_ceil(4)
}

fn assert_under(label: &str, text: &str, ceiling_tokens: usize) {
    let t = tokens(text.len());
    assert!(
        t <= ceiling_tokens,
        "{label}: {} B ~{t} tok, over the {ceiling_tokens} tok ceiling:\n{text}",
        text.len()
    );
}

#[test]
fn comment_hygiene_context_stays_small_as_comments_multiply() {
    let markers = comment_hygiene::markers_for("a.ts").expect("ts markers");

    let one = comment_hygiene::comment_lines("// set up the listener\nconst a = 1;", markers);
    let text = comment_hygiene::context_for("/x/a.ts", &one);
    assert_under("comment-hygiene, 1 comment", &text, 80);

    let five = comment_hygiene::comment_lines(
        "// one\n// two\n// three\n// four\n// five\nconst a = 1;",
        markers,
    );
    let text_five = comment_hygiene::context_for("/x/a.ts", &five);
    assert_under("comment-hygiene, 5 comments", &text_five, 90);

    // Five comments cost barely more than one: fixed instruction dominates.
    let growth = text_five.len() - text.len();
    assert!(growth < 60, "per-comment growth was {growth} B");
}

#[test]
fn a_bash_hygiene_block_is_a_line_not_a_lecture() {
    // Exit 2 puts this on stderr, which IS billed to the model.
    let v = bash_hygiene::violations("ls && pwd", None);
    assert_under(
        "bash-hygiene, one violation",
        &bash_hygiene::remediation(&v, None),
        45,
    );
}

#[test]
fn a_deny_reason_is_billed_and_stays_short() {
    let payload = json!({"tool_input": {"command": "gcloud run deploy cdn"}, "cwd": "/tmp"});
    let Decision::Deny(reason) = infra::decide(&payload) else {
        panic!("expected a deny");
    };
    assert_under("infra-guard deny reason", &reason, 40);
}

#[test]
fn a_confirmation_reason_stays_small() {
    // PreToolUse cannot prompt in Codex; run() turns this classification into a
    // denial with a short confirmation-oriented explanation.
    let payload = json!({"tool_input": {"command": "sudo launchctl list"}, "cwd": "/tmp"});
    let Decision::Ask(reason) = infra::decide(&payload) else {
        panic!("expected an ask");
    };
    assert_under("infra-guard ask reason", &reason, 40);
}

#[test]
fn a_clean_call_costs_nothing() {
    // The common case by a wide margin: no output, no tokens.
    assert!(bash_hygiene::violations("ls -la", None).is_empty());
    let payload = json!({"tool_input": {"command": "ls -la"}, "cwd": "/tmp"});
    assert_eq!(infra::decide(&payload), Decision::Allow);
}
