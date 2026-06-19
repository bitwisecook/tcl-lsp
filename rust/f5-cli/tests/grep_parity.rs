//! Differential parity tests for `f5 grep` (alias `related`).
//!
//! Runs the built `f5-query` binary on the committed `bigip.conf` fixture and
//! asserts stdout matches a golden captured from
//! `python -m tooling.f5.main grep …`. The fixture is the same drift-free
//! config the other BIG-IP verbs use, so the grep reference graph inherits
//! their parity.

use std::path::PathBuf;
use std::process::Command;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Run `f5-query grep <args> <fixture>` and assert stdout equals the golden.
/// `grep` exits 0 on a match and 1 on no match, so both are accepted.
fn assert_grep_matches(args: &[&str], golden: &str) {
    let conf = fixtures_dir().join("bigip.conf");
    let mut full: Vec<String> = vec!["grep".to_owned()];
    full.extend(args.iter().map(|s| (*s).to_owned()));
    full.push(conf.to_string_lossy().into_owned());

    let output = Command::new(env!("CARGO_BIN_EXE_f5-query"))
        .args(&full)
        .output()
        .expect("failed to spawn f5-query binary");
    let code = output.status.code().unwrap_or(-1);
    assert!(
        code == 0 || code == 1,
        "f5-query {full:?} exited {code}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let expected = std::fs::read(fixtures_dir().join(golden)).expect("read golden");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&expected),
        "f5 grep output does not match the Python CLI ({golden})"
    );
}

#[test]
fn grep_literal_matches_python() {
    assert_grep_matches(&["web_pool"], "grep-literal.text.golden");
}

#[test]
fn grep_regex_matches_python() {
    assert_grep_matches(&["-e", "/Common/.*_vs"], "grep-regex.text.golden");
}

#[test]
fn grep_cidr_matches_python() {
    assert_grep_matches(&["--cidr", "10.0.0.0/8"], "grep-cidr.text.golden");
}

#[test]
fn grep_reverse_matches_python() {
    assert_grep_matches(
        &["/Common/www_vs", "--direction", "reverse"],
        "grep-reverse.text.golden",
    );
}

#[test]
fn grep_forward_matches_python() {
    assert_grep_matches(
        &["/Common/www_vs", "--direction", "forward"],
        "grep-forward.text.golden",
    );
}

#[test]
fn grep_both_matches_python() {
    assert_grep_matches(
        &["/Common/www_vs", "--direction", "both"],
        "grep-both.text.golden",
    );
}

#[test]
fn grep_max_depth_matches_python() {
    assert_grep_matches(
        &["web_pool", "--max-depth", "1"],
        "grep-maxdepth.text.golden",
    );
}

#[test]
fn grep_include_body_matches_python() {
    assert_grep_matches(&["web_pool", "--full"], "grep-full.text.golden");
}

#[test]
fn grep_json_matches_python() {
    assert_grep_matches(&["web_pool", "--json"], "grep-json.json.golden");
}

#[test]
fn grep_no_match_matches_python() {
    assert_grep_matches(&["no_such_object_xyz"], "grep-nomatch.text.golden");
}
