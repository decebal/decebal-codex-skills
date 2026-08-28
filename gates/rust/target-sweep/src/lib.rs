//! Reclaiming a `target/` directory without forcing a cold rebuild.
//!
//! The safety property this exists to hold: a temp dir younger than the age
//! floor may belong to a RUNNING rustc, and removing it breaks that build.
//! `sweep` therefore takes a cutoff and never widens it, and `tests/sweep.rs`
//! pins that a young dir survives.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Scratch directories rustc leaves under `deps/` when it is killed.
const TEMP_PREFIXES: [&str; 2] = ["rustc", "rmeta"];

/// Artifacts untouched for this long are what `cargo sweep` would reclaim. Only
/// reported here — deleting a cargo artifact needs fingerprint knowledge this
/// binary deliberately does not have.
pub const STALE_REPORT_DAYS: u64 = 7;

pub struct Findings {
    pub temp_dirs: Vec<PathBuf>,
    pub temp_bytes: u64,
    pub dep_files: usize,
    pub stale_files: usize,
}

/// What one `target/` directory is holding: the rustc scratch dirs older than
/// `cutoff` (removable), and how many `deps/` artifacts exist versus how many
/// have gone untouched long enough for `cargo sweep` to want them.
pub fn inspect(target: &Path, cutoff: SystemTime) -> Findings {
    let mut findings = Findings {
        temp_dirs: Vec::new(),
        temp_bytes: 0,
        dep_files: 0,
        stale_files: 0,
    };

    // Temp dirs live under <target>/<profile>/deps/. Walk the profiles rather
    // than assuming `debug`, so a release or custom profile is swept too.
    let Ok(profiles) = fs::read_dir(target) else {
        return findings;
    };
    for profile in profiles.flatten() {
        let deps = profile.path().join("deps");
        let Ok(entries) = fs::read_dir(&deps) else {
            continue;
        };
        let stale_cutoff =
            SystemTime::now() - Duration::from_secs(STALE_REPORT_DAYS * 24 * 60 * 60);
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(meta) = entry.metadata() else { continue };
            let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);

            if meta.is_dir() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if TEMP_PREFIXES.iter().any(|p| name.starts_with(p)) && modified < cutoff {
                    findings.temp_bytes += dir_size(&path);
                    findings.temp_dirs.push(path);
                }
            } else {
                findings.dep_files += 1;
                if modified < stale_cutoff {
                    findings.stale_files += 1;
                }
            }
        }
    }
    findings
}

pub fn dir_size(path: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    entries
        .flatten()
        .map(|e| match e.metadata() {
            Ok(m) if m.is_dir() => dir_size(&e.path()),
            Ok(m) => m.len(),
            Err(_) => 0,
        })
        .sum()
}

pub fn human(bytes: u64) -> String {
    const GIB: u64 = 1024 * 1024 * 1024;
    const MIB: u64 = 1024 * 1024;
    if bytes >= GIB {
        format!("{:.1} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{} MiB", bytes / MIB)
    } else {
        format!("{bytes} B")
    }
}

/// Remove every temp dir in `findings`, returning how many went.
///
/// A dir that vanished between the walk and the remove is a build finishing
/// normally, not a failure.
pub fn sweep(findings: &Findings) -> (usize, Vec<String>) {
    let mut removed = 0;
    let mut errors = Vec::new();
    for dir in &findings.temp_dirs {
        match fs::remove_dir_all(dir) {
            Ok(()) => removed += 1,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => errors.push(format!("{}: {e}", dir.display())),
        }
    }
    (removed, errors)
}
