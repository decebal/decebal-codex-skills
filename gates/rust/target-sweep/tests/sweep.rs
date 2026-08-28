//! The safety property, pinned: a sweep must never take a temp dir that a
//! RUNNING rustc could still own.
//!
//! Getting this wrong does not waste disk — it breaks another session's build,
//! on a machine where several compile at once. So the age floor is tested
//! directly rather than trusted.

use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

use target_sweep::{inspect, sweep};

/// A throwaway `target/`-shaped tree. Returns its root.
fn tree(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("target-sweep-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("debug/deps")).expect("create tree");
    root
}

fn dir_with_file(deps: &Path, name: &str, bytes: usize) -> std::path::PathBuf {
    let d = deps.join(name);
    fs::create_dir_all(&d).expect("mkdir");
    fs::write(d.join("scratch.o"), vec![0u8; bytes]).expect("write");
    d
}

/// The whole point. An old scratch dir goes; a young one — which a live build
/// may be writing into right now — stays.
#[test]
fn a_young_temp_dir_survives_and_an_old_one_does_not() {
    let root = tree("age");
    let deps = root.join("debug/deps");
    let old = dir_with_file(&deps, "rustcOLD", 4096);
    let young = dir_with_file(&deps, "rustcYOUNG", 4096);

    // Everything older than one hour is in scope; `young` was made just now.
    let cutoff = SystemTime::now() - Duration::from_secs(3600);
    filetime_backdate(&old, Duration::from_secs(48 * 3600));

    let findings = inspect(&root, cutoff);
    assert_eq!(
        findings.temp_dirs.len(),
        1,
        "only the backdated dir may be in scope, got {:?}",
        findings.temp_dirs
    );

    let (removed, errors) = sweep(&findings);
    assert_eq!(removed, 1, "the old dir should have gone");
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    assert!(!old.exists(), "the old temp dir survived a sweep");
    assert!(
        young.exists(),
        "a young temp dir was removed — this is the bug that breaks a live build"
    );

    let _ = fs::remove_dir_all(&root);
}

/// Only rustc's own scratch names are ever in scope. A real artifact directory
/// sitting in `deps/` must not be touched.
#[test]
fn only_rustc_scratch_names_are_swept() {
    let root = tree("names");
    let deps = root.join("debug/deps");
    let keep = dir_with_file(&deps, "some-build-output", 128);
    let take = dir_with_file(&deps, "rmetaXYZ", 128);
    filetime_backdate(&keep, Duration::from_secs(48 * 3600));
    filetime_backdate(&take, Duration::from_secs(48 * 3600));

    let findings = inspect(&root, SystemTime::now() - Duration::from_secs(3600));
    assert_eq!(findings.temp_dirs.len(), 1);
    assert!(findings.temp_dirs[0].ends_with("rmetaXYZ"));

    sweep(&findings);
    assert!(keep.exists(), "a non-scratch directory was swept");

    let _ = fs::remove_dir_all(&root);
}

/// Files in `deps/` are counted, never removed — deleting a cargo artifact
/// needs fingerprint knowledge this crate deliberately does not have.
#[test]
fn artifact_files_are_counted_and_left_alone() {
    let root = tree("files");
    let deps = root.join("debug/deps");
    let artifact = deps.join("libexample-abc123.rlib");
    fs::write(&artifact, vec![0u8; 64]).expect("write");

    let findings = inspect(&root, SystemTime::now() - Duration::from_secs(3600));
    assert_eq!(findings.dep_files, 1);
    assert!(findings.temp_dirs.is_empty());

    sweep(&findings);
    assert!(artifact.exists(), "an artifact file was removed");

    let _ = fs::remove_dir_all(&root);
}

/// Backdate a directory's mtime without pulling in a dependency: rewrite it
/// through a file operation, then set the times with `touch`, which every
/// platform this repo builds on has.
fn filetime_backdate(path: &Path, ago: Duration) {
    let when = SystemTime::now() - ago;
    let secs = when
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("after epoch")
        .as_secs();
    // `touch -t` wants [[CC]YY]MMDDhhmm[.SS]; derive it with `date`, which is
    // POSIX and present on the macOS and Linux runners.
    let stamp = std::process::Command::new("date")
        .args(["-r", &secs.to_string(), "+%Y%m%d%H%M.%S"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
    let Some(stamp) = stamp else {
        // GNU date: -d @<secs>
        let out = std::process::Command::new("date")
            .args(["-d", &format!("@{secs}"), "+%Y%m%d%H%M.%S"])
            .output()
            .expect("date");
        let stamp = String::from_utf8_lossy(&out.stdout).trim().to_string();
        run_touch(path, &stamp);
        return;
    };
    run_touch(path, &stamp);
}

fn run_touch(path: &Path, stamp: &str) {
    let status = std::process::Command::new("touch")
        .arg("-t")
        .arg(stamp)
        .arg(path)
        .status()
        .expect("touch");
    assert!(status.success(), "could not backdate {}", path.display());
}
