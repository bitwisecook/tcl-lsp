// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Differential tests for the native `tcl` CLI.
//!
//! Each test runs the built `tcl` binary on a committed fixture and asserts its
//! stdout matches the captured golden fixtures. This locks byte-for-byte output
//! for the verbs asserted below; regenerate the `.golden`
//! files if intended behaviour changes.
//!
//! Verbs gated here: `format`, `minify`, `minify --compact`. (Verbs whose
//! engine does not yet match the goldens byte-for-byte — e.g. `diag`/`validate`
//! via the analyser — are intentionally not asserted here.)

use std::path::PathBuf;
use std::process::Command;

use regex::Regex;

/// Collapse the fixtures directory's absolute path to a stable token.
///
/// Verbs invoked with an absolute fixture path (`minimize`, `symbols`) echo it
/// back in their output, and the goldens bake whatever absolute prefix the
/// capture machine had — `/home/user/...` in the agent sandbox the goldens were
/// generated in, vs `/home/runner/...` (CI) or `/Users/...` (dev).  Normalising
/// both sides keeps the assertion portable; the diff-based tests below
/// already neutralise paths by running from the fixtures dir with bare names.
fn normalise_fixture_path(s: &str) -> String {
    let re = Regex::new(r#"/[^\s"]*?/rust/tcl-cli/tests/fixtures/"#).unwrap();
    re.replace_all(s, "__FIXTURES__/").into_owned()
}

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
        normalise_fixture_path(&String::from_utf8_lossy(&actual)),
        normalise_fixture_path(&String::from_utf8_lossy(&expected)),
        "output for `tcl {}` does not match {golden}",
        args.join(" ")
    );
}

#[test]
fn minify() {
    let input = fixtures_dir().join("greet.tcl");
    assert_matches_golden(&["minify", input.to_str().unwrap()], "greet.minify.golden");
}

#[test]
fn format() {
    let input = fixtures_dir().join("greet.tcl");
    assert_matches_golden(&["format", input.to_str().unwrap()], "greet.format.golden");
}

#[test]
fn minify_compact() {
    let input = fixtures_dir().join("greet.tcl");
    assert_matches_golden(
        &["minify", "--compact", input.to_str().unwrap()],
        "greet.minify-compact.golden",
    );
}

#[test]
fn command_info_text() {
    assert_matches_golden(&["command-info", "string"], "command-info.string.golden");
}

#[test]
fn command_info_json() {
    assert_matches_golden(
        &["command-info", "string", "--json"],
        "command-info.string.json.golden",
    );
}

#[test]
fn highlight_ansi() {
    let input = fixtures_dir().join("greet.tcl");
    assert_matches_golden(
        &["highlight", "--colour", input.to_str().unwrap()],
        "greet.highlight-ansi.golden",
    );
}

#[test]
fn highlight_html() {
    let input = fixtures_dir().join("greet.tcl");
    assert_matches_golden(
        &["highlight", "--format", "html", input.to_str().unwrap()],
        "greet.highlight-html.golden",
    );
}

// `symbols` is wired onto the analyser scope tree. The analyser still diverges
// in places (e.g. explicit `::`-qualified proc names report the simple
// name, and some implicitly-created variables aren't recorded), so these
// goldens use a fixture that exercises the matching subset — namespaces,
// nested procs, namespace variables, params, and iRules `when` events — and
// lock the wiring + JSON/text serialisation shape byte-for-byte.
#[test]
fn symbols_text() {
    let input = fixtures_dir().join("symbols.tcl");
    assert_matches_golden(&["symbols", input.to_str().unwrap()], "symbols.golden");
}

#[test]
fn symbols_json() {
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
fn symbolgraph_text() {
    let input = fixtures_dir().join("symbols.tcl");
    assert_matches_golden(
        &["symbolgraph", input.to_str().unwrap()],
        "symbolgraph.golden",
    );
}

#[test]
fn symbolgraph_json() {
    let input = fixtures_dir().join("symbols.tcl");
    assert_matches_golden(
        &["symbolgraph", "--json", input.to_str().unwrap()],
        "symbolgraph.json.golden",
    );
}

// `callgraph` is wired onto the interprocedural engine (`tcl-compiler`
// `InterproceduralAnalysis`, via the `ProcSummary::direct_calls` accessor) for
// nodes + proc→proc edges, and onto the analyser's command invocations for
// call-site resolution + top-level edges. The interproc-engine handling of
// proc→proc edges is asserted against the captured golden output: the call
// scanner detects
// calls nested in `[cmd …]` substitutions (`return [add …]` / `set x [f …]`),
// a resolved internal call no longer applies the callee's command-effect
// locally (so `pure` is computed accordingly), and a global-variable write is
// recorded
// via `writes_global` rather than the effect-region string. The
// side-effects classification also matches: `classify_side_effects`
// now consults a command's structured `side_effects`, so a proc that calls an
// untracked-effect command (`puts`) is impure yet region-free — as the golden
// expects — instead of falling back to `UNKNOWN_STATE`. This fixture exercises
// real proc→proc edges (with multiple call sites), a namespace proc, an impure
// `puts`-calling proc (no `[pure]` marker, a leaf + top-level edge), and roots
// and leaves, all byte-identical.
#[test]
fn callgraph_text() {
    let input = fixtures_dir().join("callgraph.tcl");
    assert_matches_golden(&["callgraph", input.to_str().unwrap()], "callgraph.golden");
}

#[test]
fn callgraph_json() {
    let input = fixtures_dir().join("callgraph.tcl");
    assert_matches_golden(
        &["callgraph", "--json", input.to_str().unwrap()],
        "callgraph.json.golden",
    );
}

// `diff` compares two sources at the AST / IR / CFG layers. The **AST layer**
// is byte-for-byte: it segments each side (`tcl-compiler` segmenter),
// resolves subcommands + ranges into the canonical `_serialise_command_ast`
// JSON, and emits a `difflib.unified_diff`-faithful diff
// (`tcl_cli_support::difflib`). These goldens lock the AST layer. The tests run
// the binary from the fixtures dir with bare filenames so the `ast:<name>`
// headers + `leftInput` stay stable; the JSON's absolute `leftDocuments` path
// is normalised to a `__FIXTURES__` placeholder (as the golden was captured).
#[test]
fn diff_ast_text() {
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
fn diff_ast_json() {
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

// `diff --show ir` — the **IR layer** is byte-for-byte with the captured IR
// fixtures via the `tcl-cli` `serialise` module: the
// `tcl-compiler` `CompilationUnit` IR is rendered with the same statement kinds,
// summaries, colour classes, control-flow children, and span-derived ranges
// (incl. `widen_for_highlight` brace widening). The CFG layer still needs the
// SSA serialiser. These fixtures carry procs + `if`/`for`/`expr` so the IR
// rendering is exercised.
#[test]
fn diff_ir_text() {
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
fn diff_ir_json() {
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

// `diff --show cfg` — the **CFG layer** rides on the CFG/SSA engine work
// (trailing exit block, opaque glob/regexp/fall-through switches, semi-pruned
// phi placement, SCCP live-in-root seeding), so `{preSsa, postSsa}` now matches
// the captured golden output byte-for-byte. The fixtures carry a proc, an
// `if`/`expr`, and a
// glob `switch`, with every variable read (no dead stores) so the deliberately
// uncompared `analysis.deadStores` sub-field (`O109` vs the liveness pass)
// is `[]` on both sides and does not perturb the diff.
#[test]
fn diff_cfg_text() {
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
fn diff_cfg_json() {
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
// while — every variable read (no dead stores) so the deliberately uncompared
// deadStores sub-field is `[]` on both sides and the diff stays byte-identical.
#[test]
fn diff_cfg2_text() {
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
fn diff_cfg2_json() {
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
// aggregates all five warning families per scope, in a fixed order
// (sink-injection / setter-constraint / uri-split / path-concat /
// destructive-file), mirroring `compiler_checks::run_all_checks`. The
// sink-injection family is reconciled with the expected per-statement order and
// labels: `T102` option-injection now resolves the option-terminator profile
// (`resolve_option_terminator`) so ensemble subcommands report a compound
// label (`file delete`) and the option-scan region filters positions, and
// `T103` (regex injection) fires for tainted `regexp`/`regsub` patterns. The
// fixture exercises `eval $tainted` (`T100`), `file delete $tainted`
// (`T102` + `W313`), and `regexp $tainted …` (`T103` + `T102`), in pass
// order. `tainted_variables` walks the per-unit lattices, ordered by SSA
// definition site (matching `analysis.taints` iteration) and
// skipping version-0 entries — a `(global, 0)` slot is only ever tainted by
// the conservative cross-proc global-write seeding, which the per-unit
// analysis never surfaces (so `proc save {v} { set ::store $v }` no longer
// reports `::store`). The `proc_effects` half is complete (closed interproc
// engine), incl. an impure `puts`-calling proc (region-free) and a
// global-writing proc. **Remaining taint gap**: no inter-procedural taint *solve*
// (`_solve_interprocedural_taints`), so a tainted argument flowing into a
// proc parameter and then into a sink inside that proc is not yet warned
// (cross-proc entry-taint). The fixture exercises a global-writing proc but
// no cross-proc entry-taint sink, so it locks byte-for-byte.
#[test]
fn dataflow_text() {
    let input = fixtures_dir().join("dataflow.tcl");
    assert_matches_golden(&["dataflow", input.to_str().unwrap()], "dataflow.golden");
}

#[test]
fn dataflow_json() {
    let input = fixtures_dir().join("dataflow.tcl");
    assert_matches_golden(
        &["dataflow", "--json", input.to_str().unwrap()],
        "dataflow.json.golden",
    );
}

// `diagram` extracts flow data (`extract_diagram_data`) over
// the lowered IR, byte-for-byte with the captured golden output. The only
// registry dependency is the `DIAGRAM_ACTION` trait
// (`CommandRegistry::is_diagram_action`). The fixture is an f5-irules script
// exercising the matching subset: multiple `when` events (canonical firing
// order + priority + multiplicity), `switch` with a fall-through arm, an
// `if`/`else` with conditions and notable (`[` command-substitution) assigns,
// `foreach`, `catch`, `try`/`on error`/`finally`, action commands (`pool`,
// `node`, `HTTP::respond`, `log`, …), a `proc` call, and a regular procedure.
#[test]
fn diagram_text() {
    let input = fixtures_dir().join("diagram.irule");
    assert_matches_golden(
        &["diagram", "--dialect", "f5-irules", input.to_str().unwrap()],
        "diagram.golden",
    );
}

#[test]
fn diagram_json() {
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

// `help` (`docs`) searches the bundled KCS help database — an FTS5 index built
// at compile time by `build.rs` from the
// committed `docs/kcs/features/*.md`, then embedded via `include_bytes!`. The
// query layer (`search_help` / `list_features`) runs
// over `rusqlite`'s bundled SQLite (FTS5). The rendered text + catalogue match
// the captured golden output byte-for-byte (locked by
// `help_search_text` /
// `help_catalogue_dialect`).
//
// The one field that is NOT cross-environment-stable is the raw BM25 `rank`
// float in `--json`: it is computed by whatever SQLite version is linked
// (rusqlite bundles its own, and that drifts across
// rusqlite versions), so the low-order digits diverge on some corpora. The
// thing that actually matters for the help feature is *document retrieval* —
// which KCS documents come back for a query — so the JSON test asserts the
// retrieved document set (by `file`) plus the JSON shape, and deliberately
// does not pin the `rank` float or the result ordering. Regenerate the text /
// catalogue goldens (and, if the corpus changes, the file set) from
// `docs/kcs`.
#[test]
fn help_search_text() {
    assert_matches_golden(&["help", "taint"], "help-taint.golden");
}

#[test]
fn help_search_json_retrieves_expected_documents() {
    let actual = run_tcl(&["help", "taint", "--json"]);
    let golden_path = fixtures_dir().join("help-taint.json.golden");
    let expected = std::fs::read(&golden_path)
        .unwrap_or_else(|e| panic!("read golden {}: {e}", golden_path.display()));
    assert_eq!(
        help_result_files(&actual),
        help_result_files(&expected),
        "`tcl help taint --json` retrieved a different set of KCS documents"
    );
}

/// Parse `tcl help … --json` output and return the retrieved documents' `file`
/// names, sorted — so retrieval is compared as a *set*, independent of the
/// BM25 rank order. Also sanity-checks the JSON shape (array of objects, each
/// carrying a `name` and a numeric `rank`) so a malformed payload still fails;
/// the `rank` value itself is intentionally not compared (see the note above).
fn help_result_files(json: &[u8]) -> Vec<String> {
    let value: serde_json::Value =
        serde_json::from_slice(json).expect("help --json output is valid JSON");
    let array = value
        .as_array()
        .expect("help --json output is a JSON array");
    let mut files: Vec<String> = array
        .iter()
        .map(|entry| {
            let obj = entry
                .as_object()
                .expect("each help result is a JSON object");
            assert!(
                obj.get("name")
                    .and_then(serde_json::Value::as_str)
                    .is_some(),
                "help result is missing a `name`"
            );
            assert!(
                obj.get("rank")
                    .and_then(serde_json::Value::as_f64)
                    .is_some(),
                "help result is missing a numeric `rank`"
            );
            obj.get("file")
                .and_then(serde_json::Value::as_str)
                .expect("help result is missing a `file`")
                .to_owned()
        })
        .collect();
    files.sort();
    files
}

#[test]
fn help_catalogue_dialect() {
    assert_matches_golden(
        &["help", "--dialect", "f5-irules"],
        "help-catalogue-irules.golden",
    );
}

// `registry-dump` is wired onto the command-registry snapshot
// (`tcl_registry::command_snapshot`, matching the
// `command_registry_snapshot`). Whole-dialect byte-for-byte output is gated by
// command-registry *data* completeness: the snapshots differ on
// the `dialects` field representation (the snapshot uses explicit dialect sets
// where the golden uses `None`, and carries no `f5-bigip`/`f5-tmsh` dialect bits)
// plus scattered trait / hover-synopsis / arity / subcommand-modelling data
// divergences. So this golden locks the matching
// subset — the core commands whose registry data is already byte-identical to
// the golden — verifying the snapshot serialisation + field derivation.
//
// One entry now diverges from the retired Python `registry-dump` on purpose:
// `error` carries `is_language_keyword: true` (issue #904). It raises an
// exception exactly as `throw` does, and every Tcl grammar that has a function
// category agrees it is not one — so `catch { error boom }` no longer paints its
// two halves in different colours. The golden records the new truth, not the old
// parity.
#[test]
fn registry_dump_faithful_subset() {
    // Core commands verified byte-identical to `registry-dump`
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
        "registry-dump faithful-subset snapshot does not match the golden"
    );
}

#[test]
fn explore_json_emits_the_contract_keys() {
    let out = run_tcl(&["explore", "--source", "set x 1\nputs $x", "--json"]);
    let value: serde_json::Value =
        serde_json::from_slice(&out).expect("explore --json must emit valid JSON");
    let obj = value.as_object().expect("top-level object");
    // A representative spread of views is present.
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

// `find-legacy` runs the
// analyser over the combined input, keeps the six convertible codes (W100,
// W104, W110, W304, IRULE2001, IRULE5001), and reports each with its
// modernisation hint. The firing condition, span, message, and severity match
// the captured golden output byte-for-byte; the only accepted divergence is the
// emission *order* of the underlying diagnostics (the diagnostics are sorted by
// source
// position, whereas they are emitted in pass order). The
// fixtures below are deliberately single-pattern-per-line, so source order and
// pass order coincide and the goldens match exactly (verified
// at capture time); they are captured from the Rust binary per the
// ordering-divergence guardrail. `.irule` fixtures run under `f5-irules` — the
// realistic dialect; analysing an iRule under `tcl8.6` is a degenerate
// wrong-dialect run (the `when` body is an opaque string there, so Rust's
// dialect-aware body recursion intentionally skips it).
#[test]
fn find_legacy_tcl_text() {
    let input = fixtures_dir().join("find-legacy.tcl");
    assert_matches_golden(
        &["find-legacy", input.to_str().unwrap()],
        "find-legacy.tcl.golden",
    );
}

#[test]
fn find_legacy_tcl_json() {
    let input = fixtures_dir().join("find-legacy.tcl");
    assert_matches_golden(
        &["find-legacy", input.to_str().unwrap(), "--json"],
        "find-legacy.tcl.json.golden",
    );
}

#[test]
fn find_legacy_irule_text() {
    let input = fixtures_dir().join("find-legacy.irule");
    assert_matches_golden(
        &[
            "find-legacy",
            "--dialect",
            "f5-irules",
            input.to_str().unwrap(),
        ],
        "find-legacy.irule.golden",
    );
}

#[test]
fn find_legacy_irule_json() {
    let input = fixtures_dir().join("find-legacy.irule");
    assert_matches_golden(
        &[
            "find-legacy",
            "--dialect",
            "f5-irules",
            input.to_str().unwrap(),
            "--json",
        ],
        "find-legacy.irule.json.golden",
    );
}

#[test]
fn find_legacy_empty_text() {
    assert_matches_golden(
        &["find-legacy", "--source", "set x 1"],
        "find-legacy-empty.golden",
    );
}

#[test]
fn find_legacy_empty_json() {
    assert_matches_golden(
        &["find-legacy", "--source", "set x 1", "--json"],
        "find-legacy-empty.json.golden",
    );
}

// `minimize` delta-debugs the input down to the smallest snippet that
// still fires CODE, then verify-gated-renames identifiers to short `a b c …`
// names. The diagnostic CODE is the trailing positional. The
// engine is membership-only ("does CODE fire?"), so the analyser's accepted
// diagnostic-ordering divergence is irrelevant. These tcl goldens reduce to a
// single well-formed line whose analysis matches the golden exactly, so they are
// byte-identical to the captured golden output (verified at capture time) even
// though they
// are captured from the Rust binary per the project guardrail. NB: ddmin can
// also explore *brace-unbalanced* reductions, on which the two analysers
// legitimately differ (e.g. one fires IRULE2001 on a lone
// `if {[matchclass …]} {` whose `if` body is unterminated, where the other's
// segmentation does not descend); the reduced output still reproduces CODE
// (the verb's invariant), so the divergence is in *how far* ddmin reduces, not
// correctness — see the `minimize_reduced_output_still_fires` property test.
// The fixtures stay on well-formed-reducing tcl to keep
// the goldens faithful.
#[test]
fn minimize_w100_text() {
    let input = fixtures_dir().join("minimize.tcl");
    assert_matches_golden(
        &["minimize", input.to_str().unwrap(), "W100"],
        "minimize.w100.golden",
    );
}

#[test]
fn minimize_w100_json() {
    let input = fixtures_dir().join("minimize.tcl");
    assert_matches_golden(
        &["minimize", input.to_str().unwrap(), "W100", "--json"],
        "minimize.w100.json.golden",
    );
}

#[test]
fn minimize_w100_norename() {
    let input = fixtures_dir().join("minimize.tcl");
    assert_matches_golden(
        &["minimize", input.to_str().unwrap(), "W100", "--no-rename"],
        "minimize.w100-norename.golden",
    );
}

#[test]
fn minimize_w211_text() {
    let input = fixtures_dir().join("minimize.tcl");
    assert_matches_golden(
        &["minimize", input.to_str().unwrap(), "W211"],
        "minimize.w211.golden",
    );
}

// When CODE fires on no input, the verb prints `CODE does not fire on any
// input.` to stderr and exits 1.
#[test]
fn minimize_missing_code_errors() {
    let input = fixtures_dir().join("minimize.tcl");
    let output = Command::new(env!("CARGO_BIN_EXE_tcl"))
        .args(["minimize", input.to_str().unwrap(), "ZZZ999"])
        .output()
        .expect("failed to spawn tcl binary");
    assert_eq!(output.status.code(), Some(1), "exit code");
    assert!(output.stdout.is_empty(), "no stdout on the no-fire path");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "ZZZ999 does not fire on any input.\n",
    );
}

// Property test (the task's explicit requirement): the reduced reproducer must
// still fire CODE. We assert it two ways — the verb's own `reproduces` flag in
// the JSON, and independently by re-running `tcl diag` on the reduced source
// and confirming CODE is present. This is the invariant that makes a minimised
// snippet a valid bug-report repro regardless of how far ddmin reduced.
#[test]
fn minimize_reduced_output_still_fires() {
    let input = fixtures_dir().join("minimize.tcl");
    let out = run_tcl(&["minimize", input.to_str().unwrap(), "W100", "--json"]);
    let value: serde_json::Value =
        serde_json::from_slice(&out).expect("minimize --json must emit valid JSON");
    let items = value.as_array().expect("top-level array");
    assert!(!items.is_empty(), "W100 fires, so there is a result");
    for item in items {
        assert_eq!(
            item["reproduces"],
            serde_json::Value::Bool(true),
            "the verb reports the reduction reproduces"
        );
        let reduced = item["source"].as_str().expect("source string");
        // Independently confirm the analyser still fires W100 on the reduced
        // snippet via `tcl diag --json`. `diag` exits 1 when it finds a
        // problem-severity diagnostic (W100 is an error), so we read its stdout
        // directly rather than through the success-asserting `run_tcl`.
        let diag = Command::new(env!("CARGO_BIN_EXE_tcl"))
            .args(["diag", "--source", reduced, "--json"])
            .output()
            .expect("failed to spawn tcl binary")
            .stdout;
        let report: serde_json::Value =
            serde_json::from_slice(&diag).expect("diag --json must emit valid JSON");
        let fires = report
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|f| f["diagnostics"].as_array())
            .flatten()
            .any(|d| d["code"] == serde_json::Value::String("W100".to_owned()));
        assert!(fires, "reduced source {reduced:?} must still fire W100");
    }
}

#[test]
fn minify_symbol_map_written_for_plain_minify() {
    // `tcl minify --symbol-map FILE` without `--compact`/`--aggressive` must
    // still create the map file (an empty / identity map), not silently skip
    // it — otherwise a later `unminify-error` fails on a missing path
    // (issue 198).
    let input = fixtures_dir().join("greet.tcl");
    let tmp = std::env::temp_dir().join(format!(
        "tcl-cli-symmap-{}-{}.txt",
        std::process::id(),
        line!()
    ));
    let _ = std::fs::remove_file(&tmp);
    let _ = run_tcl(&[
        "minify",
        "--symbol-map",
        tmp.to_str().unwrap(),
        input.to_str().unwrap(),
    ]);
    assert!(
        tmp.exists(),
        "plain minify must still write the requested --symbol-map file"
    );
    let _ = std::fs::remove_file(&tmp);
}
