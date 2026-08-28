//! The scan itself. Takes file CONTENT, so it is testable without a filesystem.

use regex::Regex;

/// The escape hatch, on the offending line or the one directly above it.
pub const ALLOW_MARKER: &str = "test-hang-allow:";

/// A real socket constructor doing I/O — the anchor for "this test touches the
/// network or a real IPC endpoint".
pub const DEFAULT_SOCKET_PATTERN: &str = r"(TcpListener|TcpStream|UnixListener|UnixStream|LocalSocketListener|LocalSocketStream)::(bind|connect)";

/// A blocking thread sleep. Deliberately NOT `tokio::time::sleep`: no regex can
/// tell a `start_paused` sleep (correct, and the shape the convention teaches)
/// from a real-time one. That residual belongs to the runner's slow-timeout.
pub const DEFAULT_SLEEP_PATTERN: &str = r"(std::)?thread::sleep";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    pub path: String,
    pub line: usize,
    pub text: String,
}

/// Scan one file's content for `pattern`, honouring comments and the marker.
pub fn scan(path: &str, content: &str, pattern: &Regex) -> Vec<Violation> {
    let lines: Vec<&str> = content.lines().collect();
    let mut out = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        if !pattern.is_match(line) {
            continue;
        }
        // A prose mention of `TcpStream::connect` in a doc line is not real I/O.
        if is_comment(line) {
            continue;
        }
        if line.contains(ALLOW_MARKER) {
            continue;
        }
        if idx > 0 && lines[idx - 1].contains(ALLOW_MARKER) {
            continue;
        }
        out.push(Violation {
            path: path.to_string(),
            line: idx + 1,
            text: line.trim().to_string(),
        });
    }
    out
}

fn is_comment(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("//") || t.starts_with("/*") || t.starts_with('*')
}

/// Is this path a dedicated test file? Production code legitimately owns real
/// sockets, so the gate never reads it — which is only safe where new tests
/// cannot live inline (see rules/testing-gates.md).
pub fn is_test_file(path: &str) -> bool {
    if !path.ends_with(".rs") {
        return false;
    }
    if path
        .split('/')
        .any(|seg| matches!(seg, "node_modules" | "target" | "dist" | "build"))
    {
        return false;
    }
    let name = path.rsplit('/').next().unwrap_or(path);
    let stem = name.trim_end_matches(".rs");
    // `tests/` as a path segment, including the first one — `tests/x.rs` is the
    // conventional integration-test location and reading only `/tests/` misses it.
    let under_tests_dir = path.split('/').rev().skip(1).any(|seg| seg == "tests");
    stem == "tests" || stem.ends_with("_test") || stem.ends_with("_tests") || under_tests_dir
}

#[cfg(test)]
#[path = "scan_tests.rs"]
mod scan_tests;
