//! dev-preflight — clear the state that makes a dev launch look like a hang.
//!
//! ```text
//! dev-preflight [reap|status] [--config gates.toml] [--root <dir>]
//! ```
//!
//! ```toml
//! [dev-preflight]
//! # Binaries, relative to the checkout root. Scoping to the root is what keeps
//! # an installed copy of the same app safe.
//! binaries      = ["target/debug/my-app", "target/release/my-app"]
//! # Single-instance lock sockets, cleared when no app process remains.
//! sockets       = ["/tmp/com_example_app_si.sock"]
//! # Per-launch sockets named `<prefix><pid>.sock` in the temp dir.
//! socket_prefix = "my-app-crash-"
//! # Argv marker for a helper sibling rather than the app itself.
//! helper_marker = "--crash-monitor"
//! grace_ms      = "1500"
//! ```
//!
//! `reap` is what a dev task runs; `status` reports and kills nothing.
//!
//! Exit 0 always in `status`. `reap` exits 0 even when it finds nothing — this
//! is a pre-flight, not a gate, and a launch with nothing to clear is the normal
//! case. It exits 2 only on a config error.

use gates_config::Config;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::thread::sleep;
use std::time::Duration;

mod procs;
use procs::{select, stale_sockets, Proc};

struct Settings {
    binaries: Vec<String>,
    sockets: Vec<String>,
    socket_prefix: String,
    helper_marker: String,
    grace: Duration,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut status_only = false;
    let mut config_path = "gates.toml".to_string();
    let mut root_arg: Option<PathBuf> = None;

    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "reap" => status_only = false,
            "status" => status_only = true,
            "--config" => match rest.next() {
                Some(value) => config_path = value.clone(),
                None => return usage("--config needs a path"),
            },
            "--root" => match rest.next() {
                Some(value) => root_arg = Some(PathBuf::from(value)),
                None => return usage("--root needs a path"),
            },
            other => return usage(&format!("unknown argument `{other}`")),
        }
    }

    let cfg = match Config::read(&config_path) {
        Ok(Some(c)) => c,
        Ok(None) => Config::default(),
        Err(e) => {
            eprintln!("dev-preflight: {config_path}: {e}");
            return ExitCode::from(2);
        }
    };

    let settings = match build_settings(&cfg) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("dev-preflight: [dev-preflight]: {e}");
            return ExitCode::from(2);
        }
    };

    let root = root_arg
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_default();

    let absolute: Vec<String> = settings
        .binaries
        .iter()
        .map(|b| root.join(b).to_string_lossy().into_owned())
        .collect();

    // A failure to list processes is not fatal: this is a pre-flight, and the
    // launch that follows behaves exactly as it does with no pre-flight at all.
    let table = match Command::new("ps")
        .args(["-axo", "pid=,ppid=,command="])
        .output()
    {
        Ok(out) => String::from_utf8_lossy(&out.stdout).into_owned(),
        Err(e) => {
            eprintln!("dev-preflight: could not list processes ({e}) — skipping pre-flight");
            return ExitCode::SUCCESS;
        }
    };
    let found = select(&table, &absolute, std::process::id());

    if status_only {
        report(&found, &settings);
    } else {
        reap(&found, &settings);
    }
    ExitCode::SUCCESS
}

fn report(found: &[Proc], settings: &Settings) {
    if found.is_empty() {
        println!("dev-preflight: nothing from this checkout is running");
    }
    for p in found {
        let kind = if p.is_helper(&settings.helper_marker) {
            "helper"
        } else {
            "app"
        };
        let state = if p.is_orphan() {
            "ORPHAN (parent gone)"
        } else {
            "supervised"
        };
        println!(
            "dev-preflight: {kind} pid {} ppid {} — {state}",
            p.pid, p.ppid
        );
    }
    for socket in current_stale(found, settings) {
        println!("dev-preflight: stale socket {}", socket.display());
    }
}

fn reap(found: &[Proc], settings: &Settings) {
    if !found.is_empty() {
        // Loud on purpose: silently killing another terminal's app would be its
        // own mystery. Such an app is not usable anyway — the launch about to
        // happen would have handed off to it and exited.
        for p in found {
            let kind = if p.is_helper(&settings.helper_marker) {
                "helper"
            } else {
                "app"
            };
            println!(
                "dev-preflight: reaping leftover {kind} (pid {}, ppid {}) — it holds the \
                 single-instance lock and would make this launch exit silently",
                p.pid, p.ppid
            );
        }

        let pids: Vec<u32> = found.iter().map(|p| p.pid).collect();
        signal(&pids, "TERM");
        sleep(settings.grace);
        let survivors: Vec<u32> = pids.into_iter().filter(|pid| is_alive(*pid)).collect();
        if !survivors.is_empty() {
            signal(&survivors, "KILL");
        }
    }

    let remaining: Vec<Proc> = found.iter().filter(|p| is_alive(p.pid)).cloned().collect();
    for socket in current_stale(&remaining, settings) {
        match std::fs::remove_file(&socket) {
            Ok(()) => println!("dev-preflight: removed stale socket {}", socket.display()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => eprintln!("dev-preflight: could not remove {}: {e}", socket.display()),
        }
    }
}

fn current_stale(live: &[Proc], settings: &Settings) -> Vec<PathBuf> {
    let temp = std::env::temp_dir();
    let entries: Vec<(String, PathBuf)> = std::fs::read_dir(&temp)
        .map(|dir| {
            dir.flatten()
                .filter_map(|e| e.file_name().to_str().map(|n| (n.to_string(), e.path())))
                .collect()
        })
        .unwrap_or_default();

    stale_sockets(
        live,
        &settings.helper_marker,
        &settings.sockets,
        &entries,
        &settings.socket_prefix,
    )
    .into_iter()
    .filter(|p| Path::new(p).exists())
    .collect()
}

fn build_settings(cfg: &Config) -> Result<Settings, String> {
    let binaries = cfg
        .list("dev-preflight.binaries")
        .ok_or_else(|| "no `binaries` declared".to_string())?;
    if binaries.is_empty() {
        return Err("`binaries` is empty".to_string());
    }
    let grace_ms: u64 = cfg
        .string("dev-preflight.grace_ms")
        .unwrap_or("1500")
        .parse()
        .map_err(|_| "`grace_ms` is not a number".to_string())?;

    Ok(Settings {
        binaries,
        sockets: cfg.list("dev-preflight.sockets").unwrap_or_default(),
        socket_prefix: cfg
            .string("dev-preflight.socket_prefix")
            .unwrap_or_default()
            .to_string(),
        helper_marker: cfg
            .string("dev-preflight.helper_marker")
            .unwrap_or_default()
            .to_string(),
        grace: Duration::from_millis(grace_ms),
    })
}

fn signal(pids: &[u32], sig: &str) {
    let mut cmd = Command::new("kill");
    cmd.arg(format!("-{sig}"));
    for pid in pids {
        cmd.arg(pid.to_string());
    }
    // A pid that died between the listing and here is the normal case.
    cmd.stderr(Stdio::null());
    if let Err(e) = cmd.status() {
        eprintln!("dev-preflight: kill -{sig} failed: {e}");
    }
}

fn is_alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn usage(problem: &str) -> ExitCode {
    eprintln!("dev-preflight: {problem}");
    eprintln!("usage: dev-preflight [reap|status] [--config gates.toml] [--root <dir>]");
    ExitCode::from(2)
}
