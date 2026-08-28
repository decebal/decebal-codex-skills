//! Sibling test file (rules/testing-gates.md — no inline test modules).

use super::*;

fn sockets() -> Regex {
    Regex::new(DEFAULT_SOCKET_PATTERN).unwrap()
}
fn sleeps() -> Regex {
    Regex::new(DEFAULT_SLEEP_PATTERN).unwrap()
}

#[test]
fn flags_an_unbounded_socket_bind() {
    let v = scan(
        "a_tests.rs",
        "let l = TcpListener::bind(\"127.0.0.1:0\").unwrap();\n",
        &sockets(),
    );
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].line, 1);
}

#[test]
fn flags_a_blocking_thread_sleep() {
    assert_eq!(
        scan(
            "a_tests.rs",
            "thread::sleep(Duration::from_secs(1));\n",
            &sleeps()
        )
        .len(),
        1
    );
}

#[test]
fn a_marker_on_the_line_allows_it() {
    let src = "let l = TcpListener::bind(\"127.0.0.1:0\").unwrap(); // test-hang-allow: bounded\n";
    assert!(scan("a_tests.rs", src, &sockets()).is_empty());
}

#[test]
fn a_marker_on_the_line_above_allows_it() {
    let src = "// test-hang-allow: ephemeral one-shot mock, read timeout 2s\nlet l = TcpListener::bind(\"127.0.0.1:0\").unwrap();\n";
    assert!(scan("a_tests.rs", src, &sockets()).is_empty());
}

#[test]
fn a_marker_two_lines_above_does_not_allow_it() {
    // Directly above, or on the line. Any wider and a marker drifts away from
    // what it excuses and starts covering code nobody read.
    let src = "// test-hang-allow: stale\n\nlet l = TcpListener::bind(\"127.0.0.1:0\").unwrap();\n";
    assert_eq!(scan("a_tests.rs", src, &sockets()).len(), 1);
}

#[test]
fn a_prose_mention_in_a_comment_is_not_io() {
    let src = "// Drive it over a duplex instead of TcpStream::connect.\n *  thread::sleep is banned here.\n";
    assert!(scan("a_tests.rs", src, &sockets()).is_empty());
    assert!(scan("a_tests.rs", src, &sleeps()).is_empty());
}

#[test]
fn tokio_time_sleep_is_not_flagged() {
    // Virtual time is the recommended shape; flagging it would teach the wrong
    // lesson and the regex cannot tell paused from real anyway.
    assert!(scan("a_tests.rs", "tokio::time::sleep(d).await;\n", &sleeps()).is_empty());
}

#[test]
fn recognises_the_dedicated_test_file_shapes() {
    assert!(is_test_file("src/foo_tests.rs"));
    assert!(is_test_file("src/foo_test.rs"));
    assert!(is_test_file("src/foo/tests.rs"));
    assert!(is_test_file("tests/integration.rs"));
    assert!(is_test_file("crates/x/tests/a/b.rs"));
}

#[test]
fn production_and_vendored_files_are_out_of_scope() {
    assert!(!is_test_file("src/server.rs"));
    assert!(!is_test_file("target/debug/build/x/tests.rs"));
    assert!(!is_test_file("README.md"));
}
