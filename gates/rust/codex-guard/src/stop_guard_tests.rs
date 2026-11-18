use super::{decision, needs_attention};
use serde_json::json;

#[test]
fn incomplete_review_without_a_pr_requests_continuation() {
    let payload = json!({"stop_hook_active": false, "last_assistant_message": "Review remains incomplete. UI gate rejects seven changed files until source-bound review evidence is complete. Remaining pages and states, full WCAG audit, authenticated callback and journal still need verification. No ReadyPlayDays PR opened yet."});
    assert_eq!(decision(&payload)["decision"], "block");
}

#[test]
fn continued_turn_can_report_a_real_blocker() {
    let payload = json!({"stop_hook_active": true, "last_assistant_message": "No PR yet: user permission is required."});
    assert_eq!(decision(&payload), json!({}));
}

#[test]
fn completed_and_missing_messages_do_not_block() {
    assert_eq!(
        decision(
            &json!({"stop_hook_active": false, "last_assistant_message": "PR is open and checks passed."})
        ),
        json!({})
    );
    assert_eq!(decision(&json!({"stop_hook_active": false})), json!({}));
    assert_eq!(
        decision(&json!({"last_assistant_message": "No PR yet."})),
        json!({})
    );
}

#[test]
fn work_without_a_pr_or_with_unpushed_changes_needs_attention() {
    assert!(needs_attention(&[], false, true, false));
    assert!(needs_attention(
        &[json!({"state":"OPEN"})],
        false,
        true,
        true
    ));
    assert!(!needs_attention(
        &[json!({"state":"OPEN"})],
        false,
        true,
        false
    ));
    assert!(!needs_attention(
        &[json!({"state":"MERGED"})],
        true,
        true,
        true
    ));
}
