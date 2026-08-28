use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn audit_fixture_runs_through_binary() {
    let output = Command::new(env!("CARGO_BIN_EXE_codex-seo"))
        .args([
            "audit",
            "--input",
            fixture("page.html").to_str().unwrap(),
            "--base-url",
            "https://example.com/guide",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["page"]["canonical"], "https://example.com/guide");
    assert_eq!(report["page"]["json_ld_types"][0], "Article");
    assert_eq!(report["coverage"]["static_html_score"], 100);
}

#[test]
fn sitemap_fixture_runs_through_binary() {
    let output = Command::new(env!("CARGO_BIN_EXE_codex-seo"))
        .args([
            "sitemap",
            "--input",
            fixture("sitemap.xml").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["valid"], true);
    assert_eq!(report["location_count"], 2);
}
