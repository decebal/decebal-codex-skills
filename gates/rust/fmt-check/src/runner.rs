//! Batched, parallel `rustfmt --check` invocation.
//!
//! One process per chunk of files rather than one per file: 2,564 files become
//! a handful of processes. Chunks are bounded so the argument list stays well
//! inside `ARG_MAX`, and they run across a small worker pool because rustfmt is
//! single-threaded per process.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

/// Files per rustfmt process.
pub const CHUNK: usize = 400;

/// What the whole run produced.
#[derive(Debug, Default)]
pub struct Outcome {
    /// Files whose on-disk text differs from formatted text.
    pub unformatted: Vec<PathBuf>,
    /// rustfmt could not process something — a parse error, a missing `mod`
    /// target, a broken toolchain. Distinct from `unformatted`: a check that
    /// could not run is not a check that passed.
    pub errors: Vec<String>,
}

impl Outcome {
    pub fn is_clean(&self) -> bool {
        self.unformatted.is_empty() && self.errors.is_empty()
    }
}

/// One rustfmt process worth of work.
struct Job<'a> {
    edition: &'a str,
    files: &'a [PathBuf],
}

/// Runs `rustfmt --check` over every group under its own edition.
pub fn check_all(
    rustfmt: &str,
    groups: &BTreeMap<String, Vec<PathBuf>>,
    root: &Path,
    workers: usize,
) -> Outcome {
    let jobs: Vec<Job<'_>> = groups
        .iter()
        .flat_map(|(edition, files)| {
            files.chunks(CHUNK).map(move |files| Job {
                edition: edition.as_str(),
                files,
            })
        })
        .collect();

    let queue = Mutex::new(jobs.into_iter());
    let collected = Mutex::new(Outcome::default());
    let workers = workers.max(1);

    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| loop {
                let Some(job) = queue.lock().expect("queue poisoned").next() else {
                    return;
                };
                let result = run_chunk(rustfmt, job.edition, job.files, root);
                let mut outcome = collected.lock().expect("outcome poisoned");
                match result {
                    Ok((listed, stderr)) => {
                        outcome.unformatted.extend(listed);
                        if !stderr.trim().is_empty() {
                            outcome.errors.push(stderr);
                        }
                    }
                    Err(e) => outcome.errors.push(format!(
                        "failed to run `{rustfmt}` (edition {}): {e}",
                        job.edition
                    )),
                }
            });
        }
    });

    let mut outcome = collected.into_inner().expect("outcome poisoned");
    outcome.unformatted.sort();
    outcome.unformatted.dedup();
    outcome.errors.sort();
    outcome.errors.dedup();
    outcome
}

fn run_chunk(
    rustfmt: &str,
    edition: &str,
    files: &[PathBuf],
    root: &Path,
) -> io::Result<(Vec<PathBuf>, String)> {
    let output = Command::new(rustfmt)
        .current_dir(root)
        .arg("--check")
        .arg("--files-with-diff")
        .arg("--edition")
        .arg(edition)
        // Every file is named explicitly, so recursion into `mod` children would
        // only re-check what another argument already covers — and it is the one
        // way a file could be formatted under a neighbouring crate's edition.
        .arg("--config")
        .arg("skip_children=true")
        .args(files)
        .output()?;
    let listed = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| relative(line, root))
        .collect();
    Ok((listed, String::from_utf8_lossy(&output.stderr).into_owned()))
}

/// rustfmt echoes absolute paths; the report is easier to act on relative to
/// the repo root.
fn relative(line: &str, root: &Path) -> PathBuf {
    let path = Path::new(line);
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}
