//! Differential parity tests for the native `tcl` CLI.
//!
//! Each test runs the built `tcl` binary on a committed fixture and asserts its
//! stdout matches a golden file captured from the Python CLI
//! (`python -m tooling.tcl.main <verb> ...`). This locks byte-for-byte parity
//! for the verbs whose engines are fully ported; regenerate the `.golden`
//! files from the Python CLI if intended behaviour changes.
//!
//! Verbs gated here: `format`, `minify`, `minify --compact`. (Verbs whose
//! Rust engine is still reaching parity — e.g. `diag`/`validate` via the
//! analyser — are intentionally not asserted byte-for-byte yet.)

use std::path::PathBuf;
use std::process::Command;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Run the built `tcl` binary with `args`, returning captured stdout bytes.
fn run_tcl(args: &[&str]) -> Vec<u8> {
    let output = Command::new(env!("CARGO_BIN_EXE_tcl"))
        .args(args)
        .output()
        .expect("failed to spawn tcl binary");
    assert!(
        output.status.success(),
        "tcl {args:?} exited {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn assert_matches_golden(args: &[&str], golden: &str) {
    let fixtures = fixtures_dir();
    let golden_path = fixtures.join(golden);
    let expected = std::fs::read(&golden_path)
        .unwrap_or_else(|e| panic!("read golden {}: {e}", golden_path.display()));
    let actual = run_tcl(args);
    assert_eq!(
        String::from_utf8_lossy(&actual),
        String::from_utf8_lossy(&expected),
        "output for `tcl {}` does not match {golden}",
        args.join(" ")
    );
}

#[test]
fn minify_matches_python() {
    let input = fixtures_dir().join("greet.tcl");
    assert_matches_golden(&["minify", input.to_str().unwrap()], "greet.minify.golden");
}

#[test]
fn format_matches_python() {
    let input = fixtures_dir().join("greet.tcl");
    assert_matches_golden(&["format", input.to_str().unwrap()], "greet.format.golden");
}

#[test]
fn minify_compact_matches_python() {
    let input = fixtures_dir().join("greet.tcl");
    assert_matches_golden(
        &["minify", "--compact", input.to_str().unwrap()],
        "greet.minify-compact.golden",
    );
}

#[test]
fn command_info_text_matches_python() {
    assert_matches_golden(&["command-info", "string"], "command-info.string.golden");
}

#[test]
fn command_info_json_matches_python() {
    assert_matches_golden(
        &["command-info", "string", "--json"],
        "command-info.string.json.golden",
    );
}

#[test]
fn highlight_ansi_matches_python() {
    let input = fixtures_dir().join("greet.tcl");
    assert_matches_golden(
        &["highlight", "--colour", input.to_str().unwrap()],
        "greet.highlight-ansi.golden",
    );
}

#[test]
fn highlight_html_matches_python() {
    let input = fixtures_dir().join("greet.tcl");
    assert_matches_golden(
        &["highlight", "--format", "html", input.to_str().unwrap()],
        "greet.highlight-html.golden",
    );
}

// `symbols` is wired onto the analyser scope tree. The analyser is not yet a
// full 1:1 port (e.g. explicit `::`-qualified proc names report the simple
// name, and some implicitly-created variables aren't recorded), so these
// goldens use a fixture that exercises the faithful subset — namespaces,
// nested procs, namespace variables, params, and iRules `when` events — and
// lock the wiring + JSON/text serialisation shape byte-for-byte.
#[test]
fn symbols_text_matches_python() {
    let input = fixtures_dir().join("symbols.tcl");
    assert_matches_golden(&["symbols", input.to_str().unwrap()], "symbols.golden");
}

#[test]
fn symbols_json_matches_python() {
    let input = fixtures_dir().join("symbols.tcl");
    assert_matches_golden(
        &["symbols", "--json", input.to_str().unwrap()],
        "symbols.json.golden",
    );
}

// `symbolgraph` is wired onto the analyser scope tree + command-invocation
// records. Like `symbols` it inherits the analyser gaps (explicit
// `::`-qualified proc names report the simple name, which also skews
// `ref_count`/`proc_references`; some variable references aren't tracked), so
// the goldens use the same faithful-subset fixture and lock the scope/ref
// serialisation shape byte-for-byte.
#[test]
fn symbolgraph_text_matches_python() {
    let input = fixtures_dir().join("symbols.tcl");
    assert_matches_golden(
        &["symbolgraph", input.to_str().unwrap()],
        "symbolgraph.golden",
    );
}

#[test]
fn symbolgraph_json_matches_python() {
    let input = fixtures_dir().join("symbols.tcl");
    assert_matches_golden(
        &["symbolgraph", "--json", input.to_str().unwrap()],
        "symbolgraph.json.golden",
    );
}

#[test]
fn explore_json_emits_the_contract_keys() {
    let out = run_tcl(&["explore", "--source", "set x 1\nputs $x", "--json"]);
    let value: serde_json::Value =
        serde_json::from_slice(&out).expect("explore --json must emit valid JSON");
    let obj = value.as_object().expect("top-level object");
    // A representative spread of ported views is present.
    for key in [
        "meta",
        "ir",
        "cfgPreSsa",
        "cfgPostSsa",
        "segments",
        "asm",
        "stats",
    ] {
        assert!(obj.contains_key(key), "missing explorer key {key:?}");
    }
}

#[test]
fn explore_summary_lists_views() {
    let out = run_tcl(&["explore", "--source", "set x 1"]);
    let text = String::from_utf8(out).expect("utf-8 summary");
    assert!(text.contains("Compiler explorer summary"));
    assert!(text.contains("ir:"));
}

#[test]
fn explore_text_renders_box_drawing_trees() {
    let out = run_tcl(&[
        "explore",
        "--source",
        "set x 1\nputs $x",
        "--text",
        "--show",
        "ir",
        "--no-colour",
    ]);
    let text = String::from_utf8(out).expect("utf-8 text render");
    assert!(text.contains("=== ir ==="), "section header present");
    assert!(
        text.contains("├── ") || text.contains("└── "),
        "box-drawing tree connectors present"
    );
    assert!(!text.contains('\x1b'), "no ANSI escapes with --no-colour");
}
