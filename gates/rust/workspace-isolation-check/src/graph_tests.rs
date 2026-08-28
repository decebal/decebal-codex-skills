use super::{cargo_members, leaks, lock_hits};

const METADATA: &str = r#"{"packages":[
  {"manifest_path":"/repo/apps/desktop/Cargo.toml"},
  {"manifest_path":"/repo/crates/domain/Cargo.toml"}
]}"#;

#[test]
fn reads_every_member_manifest_path() {
    let members = cargo_members(METADATA).unwrap();
    assert_eq!(members.len(), 2);
    assert!(members[0].ends_with("apps/desktop/Cargo.toml"));
}

#[test]
fn malformed_metadata_is_an_error_not_an_empty_clean_result() {
    assert!(cargo_members("not json").is_err());
    assert!(cargo_members("{}").is_err());
}

#[test]
fn a_forbidden_segment_in_a_member_path_is_a_leak() {
    let members = vec!["/repo/control-plane/api/Cargo.toml".to_string()];
    assert_eq!(leaks(&members, &["/control-plane/".into()]).len(), 1);
}

#[test]
fn a_worktree_named_after_the_forbidden_thing_is_not_a_leak() {
    let members = vec!["/wt/feat-saas-control-plane/apps/desktop/Cargo.toml".to_string()];
    assert!(leaks(&members, &["/control-plane/".into()]).is_empty());
}

#[test]
fn a_clean_graph_reports_no_leak() {
    let members = cargo_members(METADATA).unwrap();
    assert!(leaks(&members, &["/control-plane/".into()]).is_empty());
}

#[test]
fn a_lockfile_naming_a_forbidden_package_is_reported_with_its_line() {
    let lock = "root\n  \"@scope/admin\": \"1.0.0\",\n";
    let hits = lock_hits(lock, &["@scope/admin".into()]);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].0, 2);
}

#[test]
fn a_clean_lockfile_reports_nothing() {
    assert!(lock_hits("\"@other/pkg\": \"1\"\n", &["@scope/admin".into()]).is_empty());
}
