//! Tests for the config reader. In a sibling file, not an inline module, per
//! rules/testing-gates.md.

use super::*;

fn parse(text: &str) -> Config {
    Config::parse(text).expect("parses")
}

#[test]
fn reads_top_level_and_table_strings() {
    let cfg = parse("trunk = \"main\"\n\n[worktree]\nroot = \"../agents\"\n");
    assert_eq!(cfg.string("trunk"), Some("main"));
    assert_eq!(cfg.string("worktree.root"), Some("../agents"));
    assert_eq!(cfg.string("worktree.missing"), None);
}

#[test]
fn reads_inline_and_multiline_lists() {
    let cfg =
        parse("[scan]\ninline = [\"a\", \"b\"]\nspread = [\n  \"c\",\n  \"d\",   # why d\n]\n");
    assert_eq!(cfg.list("scan.inline"), Some(vec!["a".into(), "b".into()]));
    assert_eq!(cfg.list("scan.spread"), Some(vec!["c".into(), "d".into()]));
}

#[test]
fn a_bare_string_reads_as_a_one_element_list() {
    // `roots = "src"` is what people type; making it work costs one branch and
    // removes a class of "why did my gate scan nothing" reports.
    let cfg = parse("[scan]\nroots = \"src\"\n");
    assert_eq!(cfg.list("scan.roots"), Some(vec!["src".into()]));
}

#[test]
fn comments_and_blank_lines_are_ignored_but_a_hash_inside_a_string_survives() {
    let cfg = parse("# leading\n\nkey = \"a#b\"   # trailing\n");
    assert_eq!(cfg.string("key"), Some("a#b"));
}

#[test]
fn nested_tables_flatten_to_dotted_keys() {
    let cfg =
        parse("[scope.desktop]\npaths = [\"apps/api\"]\n[scope.web]\npaths = [\"apps/web\"]\n");
    assert_eq!(
        cfg.list("scope.desktop.paths"),
        Some(vec!["apps/api".into()])
    );
    assert_eq!(cfg.tables_under("scope"), vec!["desktop", "web"]);
}

#[test]
fn a_missing_file_is_absence_not_an_error() {
    assert_eq!(Config::read("/nonexistent/gates.toml"), Ok(None));
}

// The strictness half: each of these is a mistake that a lenient reader would
// turn into a gate that silently checks less than its author believes.

#[test]
fn an_unquoted_value_is_an_error() {
    let err = Config::parse("key = 42\n").unwrap_err();
    assert_eq!(err.line, 1);
    assert!(err.message.contains("double-quoted"), "{}", err.message);
}

#[test]
fn a_line_that_is_not_a_pair_is_an_error() {
    let err = Config::parse("[table]\njust some prose\n").unwrap_err();
    assert_eq!(err.line, 2);
}

#[test]
fn an_unterminated_table_header_is_an_error() {
    assert_eq!(Config::parse("[table\n").unwrap_err().line, 1);
}

#[test]
fn an_unterminated_array_is_an_error() {
    assert!(Config::parse("[t]\nx = [\n  \"a\",\n").is_err());
}

#[test]
fn arrays_of_tables_are_rejected_rather_than_misread() {
    let err = Config::parse("[[thing]]\nname = \"x\"\n").unwrap_err();
    assert!(err.message.contains("arrays of tables"), "{}", err.message);
}

#[test]
fn a_comma_inside_a_quoted_list_element_does_not_split_it() {
    let cfg =
        Config::parse("[id-refs]\nshapes = [\"t-[0-9a-f]{4,}\", \"bd-[0-9a-z]{3,}\"]\n").unwrap();
    assert_eq!(
        cfg.list("id-refs.shapes").unwrap(),
        vec!["t-[0-9a-f]{4,}", "bd-[0-9a-z]{3,}"]
    );
}

#[test]
fn a_comma_inside_a_multiline_list_element_does_not_split_it() {
    let cfg = Config::parse("[t]\nx = [\n  \"a{1,2}\",\n  \"b\",\n]\n").unwrap();
    assert_eq!(cfg.list("t.x").unwrap(), vec!["a{1,2}", "b"]);
}

#[test]
fn keys_under_lists_one_tables_own_keys() {
    let cfg = Config::parse("[layers.ceiling]\na_to_b = \"3\"\nc_to_d = \"0\"\n").unwrap();
    assert_eq!(cfg.keys_under("layers.ceiling"), vec!["a_to_b", "c_to_d"]);
}

#[test]
fn keys_under_does_not_reach_into_a_nested_table() {
    let cfg = Config::parse("[a]\nk = \"1\"\n\n[a.b]\ndeep = \"2\"\n").unwrap();
    assert_eq!(cfg.keys_under("a"), vec!["k"]);
    assert_eq!(cfg.keys_under("a.b"), vec!["deep"]);
}

#[test]
fn keys_under_an_absent_table_is_empty_not_an_error() {
    let cfg = Config::parse("[a]\nk = \"1\"\n").unwrap();
    assert!(cfg.keys_under("nope").is_empty());
}

#[test]
fn keys_under_finds_lists_as_well_as_strings() {
    let cfg = Config::parse("[t]\nlist = [\"a\"]\nstr = \"b\"\n").unwrap();
    assert_eq!(cfg.keys_under("t"), vec!["list", "str"]);
}
