use super::{parse_ps_line, select, stale_sockets, Proc};
use std::path::PathBuf;

const TABLE: &str = concat!(
    " 1234     1 /repo/target/debug/my-app\n",
    " 1235  1234 /repo/target/debug/my-app --crash-monitor /tmp/x.sock\n",
    " 9999   500 /usr/bin/something-else\n",
);

fn app() -> Proc {
    Proc {
        pid: 1,
        ppid: 1,
        command: "/repo/target/debug/my-app".into(),
    }
}

fn helper() -> Proc {
    Proc {
        pid: 2,
        ppid: 1,
        command: "/repo/target/debug/my-app --crash-monitor /tmp/x.sock".into(),
    }
}

#[test]
fn parses_padded_columns_and_keeps_the_command_verbatim() {
    let p = parse_ps_line("  1234   567 /path/to/bin --a --b").unwrap();
    assert_eq!(p.pid, 1234);
    assert_eq!(p.ppid, 567);
    assert_eq!(p.command, "/path/to/bin --a --b");
}

#[test]
fn a_malformed_line_is_skipped_rather_than_panicking() {
    assert!(parse_ps_line("garbage").is_none());
    assert!(parse_ps_line("").is_none());
}

#[test]
fn selects_only_processes_running_this_checkouts_binary() {
    let found = select(TABLE, &["/repo/target/debug/my-app".into()], 0);
    assert_eq!(found.len(), 2);
}

#[test]
fn an_installed_copy_elsewhere_is_never_selected() {
    let table = " 42 1 /Applications/My.app/Contents/MacOS/my-app\n";
    assert!(select(table, &["/repo/target/debug/my-app".into()], 0).is_empty());
}

#[test]
fn this_process_is_excluded_from_its_own_reap_list() {
    assert!(select(TABLE, &["/repo/target/debug/my-app".into()], 1234).len() == 1);
}

#[test]
fn a_helper_is_told_apart_from_the_app() {
    assert!(helper().is_helper("--crash-monitor"));
    assert!(!app().is_helper("--crash-monitor"));
}

#[test]
fn an_empty_helper_marker_never_classifies_anything_as_a_helper() {
    assert!(!helper().is_helper(""));
}

#[test]
fn a_reparented_process_reads_as_an_orphan() {
    assert!(app().is_orphan());
    let supervised = Proc { ppid: 500, ..app() };
    assert!(!supervised.is_orphan());
}

#[test]
fn the_lock_socket_is_stale_only_when_no_app_process_remains() {
    let locks = vec!["/tmp/lock.sock".to_string()];
    assert!(stale_sockets(&[app()], "--crash-monitor", &locks, &[], "").is_empty());
    assert_eq!(
        stale_sockets(&[], "--crash-monitor", &locks, &[], "").len(),
        1
    );
}

#[test]
fn a_surviving_helper_does_not_keep_the_lock_socket_alive() {
    let locks = vec!["/tmp/lock.sock".to_string()];
    let stale = stale_sockets(&[helper()], "--crash-monitor", &locks, &[], "");
    assert_eq!(stale.len(), 1);
}

#[test]
fn a_per_launch_socket_is_stale_when_its_pid_is_gone() {
    let entries = vec![
        ("app-crash-2.sock".to_string(), PathBuf::from("/tmp/a")),
        ("app-crash-777.sock".to_string(), PathBuf::from("/tmp/b")),
    ];
    let stale = stale_sockets(&[helper()], "--crash-monitor", &[], &entries, "app-crash-");
    assert_eq!(stale, vec![PathBuf::from("/tmp/b")]);
}

#[test]
fn an_unrelated_temp_file_is_never_removed() {
    let entries = vec![("other.sock".to_string(), PathBuf::from("/tmp/other"))];
    assert!(stale_sockets(&[], "--crash-monitor", &[], &entries, "app-crash-").is_empty());
}
