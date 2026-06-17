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

/// Run the built `tcl` binary from `dir` (so relative input paths resolve and
/// the diff headers carry stable bare filenames), returning stdout. Unlike
/// [`run_tcl`] this does not assert success — `diff` exits 1 when the sides
/// differ.
fn run_tcl_in(dir: &std::path::Path, args: &[&str]) -> Vec<u8> {
    Command::new(env!("CARGO_BIN_EXE_tcl"))
        .current_dir(dir)
        .args(args)
        .output()
        .expect("failed to spawn tcl binary")
        .stdout
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

// `diff` compares two sources at the AST / IR / CFG layers. The **AST layer**
// is a byte-parity port: it segments each side (`tcl-compiler` segmenter),
// resolves subcommands + ranges into the canonical `_serialise_command_ast`
// JSON, and emits a `difflib.unified_diff`-faithful diff
// (`tcl_cli_support::difflib`). The IR/CFG layers depend on the IR/SSA
// serialiser (`tooling/cli/serialise.py`) and are not ported yet (requesting
// them is a clear error), so these goldens lock the AST layer. The tests run
// the binary from the fixtures dir with bare filenames so the `ast:<name>`
// headers + `leftInput` stay stable; the JSON's absolute `leftDocuments` path
// is normalised to a `__FIXTURES__` placeholder (as the golden was captured).
#[test]
fn diff_ast_text_matches_python() {
    let fx = fixtures_dir();
    let actual = run_tcl_in(
        &fx,
        &["diff", "diff-left.tcl", "diff-right.tcl", "--show", "ast"],
    );
    let expected = std::fs::read(fx.join("diff.ast.golden")).expect("read diff.ast.golden");
    assert_eq!(
        String::from_utf8_lossy(&actual),
        String::from_utf8_lossy(&expected),
    );
}

#[test]
fn diff_ast_json_matches_python() {
    let fx = fixtures_dir();
    let actual = String::from_utf8(run_tcl_in(
        &fx,
        &[
            "diff",
            "diff-left.tcl",
            "diff-right.tcl",
            "--show",
            "ast",
            "--json",
        ],
    ))
    .expect("utf8 stdout");
    let normalised = actual.replace(fx.to_string_lossy().as_ref(), "__FIXTURES__");
    let expected = std::fs::read_to_string(fx.join("diff.ast.json.golden"))
        .expect("read diff.ast.json.golden");
    assert_eq!(normalised, expected);
}

// `diff --show ir` — the **IR layer** is a byte-parity port of the `ir` half of
// `tooling/cli/serialise.py` (`_serialise_ir` / `_serialise_script` + the
// `stmt_*` / `preview` helpers) via the new `tcl-cli` `serialise` module: the
// `tcl-compiler` `CompilationUnit` IR is rendered with the same statement kinds,
// summaries, colour classes, control-flow children, and span-derived ranges
// (incl. `widen_for_highlight` brace widening). The CFG layer still needs the
// SSA serialiser. These fixtures carry procs + `if`/`for`/`expr` so the IR
// rendering is exercised.
#[test]
fn diff_ir_text_matches_python() {
    let fx = fixtures_dir();
    let actual = run_tcl_in(
        &fx,
        &[
            "diff",
            "diff-ir-left.tcl",
            "diff-ir-right.tcl",
            "--show",
            "ir",
        ],
    );
    let expected = std::fs::read(fx.join("diff.ir.golden")).expect("read diff.ir.golden");
    assert_eq!(
        String::from_utf8_lossy(&actual),
        String::from_utf8_lossy(&expected),
    );
}

#[test]
fn diff_ir_json_matches_python() {
    let fx = fixtures_dir();
    let actual = String::from_utf8(run_tcl_in(
        &fx,
        &[
            "diff",
            "diff-ir-left.tcl",
            "diff-ir-right.tcl",
            "--show",
            "ir",
            "--json",
        ],
    ))
    .expect("utf8 stdout");
    let normalised = actual.replace(fx.to_string_lossy().as_ref(), "__FIXTURES__");
    let expected =
        std::fs::read_to_string(fx.join("diff.ir.json.golden")).expect("read diff.ir.json.golden");
    assert_eq!(normalised, expected);
}

// `diff --show cfg` — the **CFG layer** rides on the CFG/SSA engine-parity work
// (trailing exit block, opaque glob/regexp/fall-through switches, semi-pruned
// phi placement, SCCP live-in-root seeding), so `{preSsa, postSsa}` now matches
// the Python CLI byte-for-byte. The fixtures carry a proc, an `if`/`expr`, and a
// glob `switch`, with every variable read (no dead stores) so the documented
// `_NO_PARITY` `analysis.deadStores` sub-field (Rust O109 vs Python liveness)
// is `[]` on both sides and does not perturb the diff.
#[test]
fn diff_cfg_text_matches_python() {
    let fx = fixtures_dir();
    let actual = run_tcl_in(
        &fx,
        &[
            "diff",
            "diff-cfg-left.tcl",
            "diff-cfg-right.tcl",
            "--show",
            "cfg",
        ],
    );
    let expected = std::fs::read(fx.join("diff.cfg.golden")).expect("read diff.cfg.golden");
    assert_eq!(
        String::from_utf8_lossy(&actual),
        String::from_utf8_lossy(&expected),
    );
}

#[test]
fn diff_cfg_json_matches_python() {
    let fx = fixtures_dir();
    let actual = String::from_utf8(run_tcl_in(
        &fx,
        &[
            "diff",
            "diff-cfg-left.tcl",
            "diff-cfg-right.tcl",
            "--show",
            "cfg",
            "--json",
        ],
    ))
    .expect("utf8 stdout");
    let normalised = actual.replace(fx.to_string_lossy().as_ref(), "__FIXTURES__");
    let expected = std::fs::read_to_string(fx.join("diff.cfg.json.golden"))
        .expect("read diff.cfg.json.golden");
    assert_eq!(normalised, expected);
}

// Wider cfg coverage: a multi-proc script with if/elseif/else, foreach, and
// while — every variable read (no dead stores) so the `_NO_PARITY` deadStores
// sub-field is `[]` on both sides and the diff stays byte-identical.
#[test]
fn diff_cfg2_text_matches_python() {
    let fx = fixtures_dir();
    let actual = run_tcl_in(
        &fx,
        &[
            "diff",
            "diff-cfg2-left.tcl",
            "diff-cfg2-right.tcl",
            "--show",
            "cfg",
        ],
    );
    let expected = std::fs::read(fx.join("diff.cfg2.golden")).expect("read diff.cfg2.golden");
    assert_eq!(
        String::from_utf8_lossy(&actual),
        String::from_utf8_lossy(&expected),
    );
}

#[test]
fn diff_cfg2_json_matches_python() {
    let fx = fixtures_dir();
    let actual = String::from_utf8(run_tcl_in(
        &fx,
        &[
            "diff",
            "diff-cfg2-left.tcl",
            "diff-cfg2-right.tcl",
            "--show",
            "cfg",
            "--json",
        ],
    ))
    .expect("utf8 stdout");
    let normalised = actual.replace(fx.to_string_lossy().as_ref(), "__FIXTURES__");
    let expected = std::fs::read_to_string(fx.join("diff.cfg2.json.golden"))
        .expect("read diff.cfg2.json.golden");
    assert_eq!(normalised, expected);
}

// `dataflow` is wired onto the same `CompilationUnit` (`interproc` summaries
// for the `proc_effects` half — `pure` / `reads` / `writes` / `has_barrier` via
// `_effect_region_str`) and the taint engine. The `taint_warnings` half now
// aggregates all five Python warning families per scope, in Python's order
// (sink-injection / setter-constraint / uri-split / path-concat /
// destructive-file), mirroring `compiler_checks::run_all_checks`. The
// sink-injection family is reconciled with Python's per-statement order and
// labels: `T102` option-injection now resolves the option-terminator profile
// (`resolve_option_terminator`) so ensemble subcommands report a compound
// label (`file delete`) and the option-scan region filters positions, and
// `T103` (regex injection) fires for tainted `regexp`/`regsub` patterns. The
// fixture exercises `eval $tainted` (`T100`), `file delete $tainted`
// (`T102` + `W313`), and `regexp $tainted …` (`T103` + `T102`), in Python's
// order. `tainted_variables` walks the per-unit lattices, ordered by SSA
// definition site (matching Python's `analysis.taints` iteration) and
// skipping version-0 entries — a `(global, 0)` slot is only ever tainted by
// the conservative cross-proc global-write seeding, which Python's per-unit
// analysis never surfaces (so `proc save {v} { set ::store $v }` no longer
// reports `::store`). The `proc_effects` half is at parity (closed interproc
// engine), incl. an impure `puts`-calling proc (region-free) and a
// global-writing proc. **Remaining taint gap** (documented in
// docs/rust-cli-port.md): no inter-procedural taint *solve*
// (`_solve_interprocedural_taints`), so a tainted argument flowing into a
// proc parameter and then into a sink inside that proc is not yet warned
// (cross-proc entry-taint). The fixture exercises a global-writing proc but
// no cross-proc entry-taint sink, so it locks byte-for-byte.
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

// `diagram` ports `tooling/diagram/extract.py` (`extract_diagram_data`) over
// the lowered IR — now byte-parity since lowering/IR reached parity. The only
// registry dependency is the `DIAGRAM_ACTION` trait
// (`CommandRegistry::is_diagram_action`). The fixture is an f5-irules script
// exercising the faithful subset: multiple `when` events (canonical firing
// order + priority + multiplicity), `switch` with a fall-through arm, an
// `if`/`else` with conditions and notable (`[` command-substitution) assigns,
// `foreach`, `catch`, `try`/`on error`/`finally`, action commands (`pool`,
// `node`, `HTTP::respond`, `log`, …), a `proc` call, and a regular procedure.
#[test]
fn diagram_text_matches_python() {
    let input = fixtures_dir().join("diagram.irule");
    assert_matches_golden(
        &["diagram", "--dialect", "f5-irules", input.to_str().unwrap()],
        "diagram.golden",
    );
}

#[test]
fn diagram_json_matches_python() {
    let input = fixtures_dir().join("diagram.irule");
    assert_matches_golden(
        &[
            "diagram",
            "--dialect",
            "f5-irules",
            "--json",
            input.to_str().unwrap(),
        ],
        "diagram.json.golden",
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
