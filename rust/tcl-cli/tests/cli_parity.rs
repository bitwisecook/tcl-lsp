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

// `callgraph` is wired onto the interprocedural engine (`tcl-compiler`
// `InterproceduralAnalysis`, via the `ProcSummary::direct_calls` accessor) for
// nodes + proc→proc edges, and onto the analyser's command invocations for
// call-site resolution + top-level edges. The interproc-engine closure that
// flips proc→proc edges to parity has landed: the call scanner now detects
// calls nested in `[cmd …]` substitutions (`return [add …]` / `set x [f …]`),
// a resolved internal call no longer applies the callee's command-effect
// locally (so `pure` matches Python), and a global-variable write is recorded
// via `writes_global` rather than the effect-region string. The
// side-effects-classification closure has also landed: `classify_side_effects`
// now consults a command's structured `side_effects`, so a proc that calls an
// untracked-effect command (`puts`) is impure yet region-free — matching
// Python — instead of falling back to `UNKNOWN_STATE`. This fixture exercises
// real proc→proc edges (with multiple call sites), a namespace proc, an impure
// `puts`-calling proc (no `[pure]` marker, a leaf + top-level edge), and roots
// and leaves, all byte-identical.
#[test]
fn callgraph_text_matches_python() {
    let input = fixtures_dir().join("callgraph.tcl");
    assert_matches_golden(&["callgraph", input.to_str().unwrap()], "callgraph.golden");
}

#[test]
fn callgraph_json_matches_python() {
    let input = fixtures_dir().join("callgraph.tcl");
    assert_matches_golden(
        &["callgraph", "--json", input.to_str().unwrap()],
        "callgraph.json.golden",
    );
}

// `dataflow` is wired onto the same `CompilationUnit` (`interproc` summaries
// for the `proc_effects` half — `pure` / `reads` / `writes` / `has_barrier` via
// `_effect_region_str`) and the taint engine. The `taint_warnings` half now
// aggregates all five Python warning families per scope, in Python's order
// (sink-injection / setter-constraint / uri-split / path-concat /
// destructive-file), mirroring `compiler_checks::run_all_checks`; the fixture
// exercises a top-level `eval $tainted` (`T100`) and a `file delete -- $tainted`
// (`W313`). `tainted_variables` walks the per-unit lattices. The `proc_effects`
// half is at parity (closed interproc engine), incl. an impure `puts`-calling
// proc (region-free — `reads`/`writes` = `NONE`). **Remaining taint gaps**
// (documented in docs/rust-cli-port.md): no inter-procedural taint solve
// (`_solve_interprocedural_taints`), so a global written inside a proc is
// over-tainted (the version-0 global seeding) where Python is precise; and the
// `T102` option-injection check doesn't yet model ensemble subcommands /
// Python's option-scan region + message label (e.g. `file delete $tainted`
// without `--`). The fixture stays clear of both so it locks byte-for-byte.
#[test]
fn dataflow_text_matches_python() {
    let input = fixtures_dir().join("dataflow.tcl");
    assert_matches_golden(&["dataflow", input.to_str().unwrap()], "dataflow.golden");
}

#[test]
fn dataflow_json_matches_python() {
    let input = fixtures_dir().join("dataflow.tcl");
    assert_matches_golden(
        &["dataflow", "--json", input.to_str().unwrap()],
        "dataflow.json.golden",
    );
}

// `registry-dump` is wired onto the Rust command-registry snapshot
// (`tcl_registry::command_snapshot`, a faithful port of Python
// `command_registry_snapshot`). Whole-dialect byte-parity is gated by
// command-registry *data* parity: the Rust and Python registries differ on
// the `dialects` field representation (Rust uses explicit dialect sets where
// Python uses `None`, and Rust carries no `f5-bigip`/`f5-tmsh` dialect bits)
// plus scattered trait / hover-synopsis / arity / subcommand-modelling data
// divergences (see docs/rust-cli-port.md). So this golden locks the faithful
// subset — the core commands whose registry data is already byte-identical to
// Python — verifying the snapshot serialisation + field derivation while the
// data gap converges as a separate workstream.
#[test]
fn registry_dump_faithful_subset_matches_python() {
    // Core commands verified byte-identical to the Python `registry-dump`
    // entry (`dialects: null` in both, matching traits/forms/scalars/info).
    const NAMES: &[&str] = &[
        "append", "array", "break", "catch", "continue", "error", "eval", "expr", "for", "global",
        "incr", "info", "join", "lappend", "lassign", "lindex", "llength", "lrange", "proc",
        "regexp", "regsub", "return", "set", "split", "switch", "throw", "try", "unset", "uplevel",
        "upvar", "variable", "while",
    ];
    use std::collections::BTreeMap;
    use tcl_registry::snapshot::Json;

    let registry = tcl_cli_support::registry_for_dialect("tcl8.6");
    let mut obj: BTreeMap<String, Json> = BTreeMap::new();
    for name in NAMES {
        let entry = tcl_registry::command_snapshot::command_entry_json(registry, "tcl8.6", name)
            .unwrap_or_else(|| panic!("registry has no entry for {name}"));
        obj.insert((*name).to_owned(), entry);
    }
    let actual = Json::Object(obj).dumps_indent2();

    let golden_path = fixtures_dir().join("registry-dump.tcl8.6-subset.golden");
    let expected = std::fs::read_to_string(&golden_path)
        .unwrap_or_else(|e| panic!("read golden {}: {e}", golden_path.display()));
    assert_eq!(
        actual, expected,
        "registry-dump faithful-subset snapshot does not match the Python golden"
    );
}
