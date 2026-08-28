//! The allow / rewrite / block matrix, ported case-for-case from the shell
//! version it replaces (rules/testing-gates.md — sibling file, no inline module).

use super::*;

/// What the hook would do, without the process exit.
#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    Allow,
    Rewrite(String),
    Block,
}

fn decide(cmd: &str) -> Verdict {
    decide_with(cmd, None)
}

fn decide_with(cmd: &str, scratch: Option<&str>) -> Verdict {
    let v = violations(cmd, scratch);
    if v.is_empty() {
        return Verdict::Allow;
    }
    match repairable_rewrite(cmd, &v) {
        Some(fixed) => Verdict::Rewrite(fixed),
        None => Verdict::Block,
    }
}

#[test]
fn plain_command_is_allowed() {
    assert_eq!(decide("make build"), Verdict::Allow);
}

#[test]
fn a_semicolon_inside_quotes_is_data() {
    assert_eq!(decide("git commit -m 'a; b'"), Verdict::Allow);
}

#[test]
fn a_heredoc_keeps_its_newlines() {
    assert_eq!(decide("cat <<EOF\nbody\nEOF"), Verdict::Allow);
}

#[test]
fn chains_and_separators_block() {
    assert_eq!(decide("ls && pwd"), Verdict::Block);
    assert_eq!(decide("ls || pwd"), Verdict::Block);
    assert_eq!(decide("ls ; pwd"), Verdict::Block);
    assert_eq!(decide("ls\npwd"), Verdict::Block);
}

#[test]
fn substitutions_block() {
    assert_eq!(decide("diff <(ls) <(ls)"), Verdict::Block);
    assert_eq!(decide("echo $(date)"), Verdict::Block);
    assert_eq!(decide("echo `date`"), Verdict::Block);
}

#[test]
fn a_trailing_stderr_merge_is_rewritten() {
    assert_eq!(
        decide("gh pr view 1 2>&1"),
        Verdict::Rewrite("gh pr view 1".into())
    );
}

#[test]
fn a_stderr_merge_that_changes_what_the_next_stage_reads_blocks() {
    // Piped or file-redirected, the merge decides which stream flows on, so
    // dropping it would change behaviour rather than tidy the call.
    assert_eq!(decide("make build 2>&1 | tail -5"), Verdict::Block);
    assert_eq!(decide("make build > log 2>&1"), Verdict::Block);
    assert_eq!(decide("ls 2>&1 && pwd"), Verdict::Block);
}

#[test]
fn a_clean_command_produces_no_output() {
    assert_eq!(decide("ls -la"), Verdict::Allow);
}

#[test]
fn the_scratch_rule_is_opt_in() {
    assert_eq!(decide("ls /tmp/x"), Verdict::Allow);
    assert_eq!(decide_with("ls /tmp/x", Some("/scratch")), Verdict::Block);
}

#[test]
fn a_temp_path_matches_only_on_a_boundary() {
    assert_eq!(decide_with("ls /tmpfs/x", Some("/scratch")), Verdict::Allow);
    assert_eq!(decide_with("ls mytmp/x", Some("/scratch")), Verdict::Allow);
    assert_eq!(
        decide_with("ls /private/tmp/x", Some("/scratch")),
        Verdict::Block
    );
}

#[test]
fn an_unpaired_quote_does_not_hide_the_rest_of_the_line() {
    // Swallowing from the quote to end-of-string would make every violation
    // after it invisible — a false negative that reads as a clean command.
    assert_eq!(decide("echo 'unterminated && pwd"), Verdict::Block);
}

#[test]
fn remediation_names_only_the_classes_that_fired() {
    let v = violations("ls && pwd", None);
    let text = remediation(&v, None);
    assert!(text.contains("Split into separate Bash calls"));
    assert!(!text.contains("run_in_background"), "{text}");
    assert!(!text.contains("inner command"), "{text}");
}

#[test]
fn remediation_covers_every_class_that_fired() {
    let v = violations("echo $(date) && cat x | tee y", None);
    let text = remediation(&v, None);
    assert!(text.contains("Split into separate Bash calls"));
    assert!(text.contains("inner command"));
    assert!(text.contains("keeps command output"));
}

#[test]
fn quote_stripping_removes_single_then_double() {
    assert_eq!(strip_quoted("a 'b;c' d \"e;f\" g"), "a  d  g");
}
