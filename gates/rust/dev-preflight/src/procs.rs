//! Process-table parsing and the stale-socket rule. Text in, values out.

use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proc {
    pub pid: u32,
    pub ppid: u32,
    pub command: String,
}

impl Proc {
    /// A helper sibling rather than the app itself.
    pub fn is_helper(&self, marker: &str) -> bool {
        !marker.is_empty() && self.command.contains(marker)
    }

    /// Reparented to the init process — nothing is supervising it.
    ///
    /// Reported for context only. Reaping does not depend on it, because an app
    /// left running by a still-open dev shell blocks the next launch just as hard
    /// as an orphan does.
    pub fn is_orphan(&self) -> bool {
        self.ppid <= 1
    }
}

/// `  1234   567 /path/to/binary --flag` -> `Proc`.
///
/// `ps` right-pads both numeric columns, so the fields are separated by RUNS of
/// spaces. Split on the first run each time and keep the remainder verbatim —
/// the command itself contains spaces.
pub fn parse_ps_line(line: &str) -> Option<Proc> {
    let (pid, rest) = line.trim_start().split_once(char::is_whitespace)?;
    let (ppid, command) = rest.trim_start().split_once(char::is_whitespace)?;
    Some(Proc {
        pid: pid.parse().ok()?,
        ppid: ppid.parse().ok()?,
        command: command.trim().to_string(),
    })
}

/// Processes running one of `binaries`, excluding this process.
pub fn select(table: &str, binaries: &[String], me: u32) -> Vec<Proc> {
    table
        .lines()
        .filter_map(parse_ps_line)
        .filter(|p| p.pid != me)
        .filter(|p| binaries.iter().any(|b| p.command.contains(b.as_str())))
        .collect()
}

/// Sockets no live process owns.
///
/// A lock socket is stale exactly when no non-helper process remains: the plugin
/// that owns it usually recovers from a stale file on its own, but leaving it
/// behind makes a `status` report lie about what is running.
pub fn stale_sockets(
    live: &[Proc],
    helper_marker: &str,
    lock_sockets: &[String],
    temp_entries: &[(String, PathBuf)],
    socket_prefix: &str,
) -> Vec<PathBuf> {
    let mut stale = Vec::new();

    let app_running = live.iter().any(|p| !p.is_helper(helper_marker));
    if !app_running {
        for socket in lock_sockets {
            stale.push(PathBuf::from(socket));
        }
    }

    if socket_prefix.is_empty() {
        return stale;
    }
    let live_pids: HashSet<u32> = live.iter().map(|p| p.pid).collect();
    for (name, path) in temp_entries {
        let Some(rest) = name.strip_prefix(socket_prefix) else {
            continue;
        };
        let Some(pid) = rest
            .strip_suffix(".sock")
            .and_then(|p| p.parse::<u32>().ok())
        else {
            continue;
        };
        if !live_pids.contains(&pid) {
            stale.push(path.clone());
        }
    }
    stale
}

#[cfg(test)]
#[path = "procs_tests.rs"]
mod procs_tests;
