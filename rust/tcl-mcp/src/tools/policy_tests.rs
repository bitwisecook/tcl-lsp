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

//! The diagnostics tools, `optimize` and `code_actions` read one report
//! under the global file and the call's own arguments
//! (`docs/design/compiler/diagnostic-policy.md` § Adapters). Every test
//! resolves under a global layer of its own, parsed from INI text exactly
//! as the user's `config.ini` is — never under the machine's.

use super::*;
use tcl_lsp_core::config_ini::{Layer, settings_from_ini};

/// A read of a variable nothing set: W210.
const UNSET_READ: &str = "puts $y\n";

/// A `while` whose counter the body never touches, against a bound the
/// solver cannot decide (a parameter, so no header fact settles it): W242,
/// the one code the catalogue declares default-off. Against a literal
/// bound the header is decided true at every test and the loop draws W241
/// instead.
const UNPROVABLE_LOOP: &str =
    "proc p {n} {\n    set i 0\n    while {$i < $n} {\n        puts $i\n    }\n}\n";

/// A constant expression O101 folds; a global stays a live store.
const FOLDING: &str = "set x [expr {1 + 2}]\n";

/// The layers a call with `args` resolves under, over `section`, with
/// `global_ini` as the user's `config.ini`.
fn inputs(args: &Value, section: &str, global_ini: &str) -> PolicyInputs {
    PolicyInputs {
        global: settings_from_ini(global_ini, Layer::Global),
        invocation: invocation_layer(args, section),
    }
}

/// `analyze`'s diagnostics under `global_ini`.
fn analyzed(args: &Value, global_ini: &str) -> Vec<Value> {
    analyze_with(args, &inputs(args, "diagnostics", global_ini))["diagnostics"]
        .as_array()
        .expect("diagnostics array")
        .clone()
}

fn has_code(diagnostics: &[Value], code: &str) -> bool {
    diagnostics.iter().any(|d| d["code"] == code)
}

/// `analyze`'s `suppressed` array under `global_ini`.
fn analyze_suppressed(args: &Value, global_ini: &str) -> Vec<Value> {
    analyze_with(args, &inputs(args, "diagnostics", global_ini))["suppressed"]
        .as_array()
        .expect("suppressed array")
        .clone()
}

/// A `jim` document's diagnostics run with the shipped packs installed:
/// the generation the analysis reads at the packs' key exists, so the
/// analysis is available and Jim's own commands are neither disabled nor
/// unresolved, while the same source under Tcl 8.6 flags them.
#[test]
fn a_jim_document_is_analysed_with_the_shipped_packs() {
    let source = "loop i 0 3 { puts $i }\nsleep 0.1\n";
    let jim = json!({ "source": source, "dialect": "jim" });
    let analysed = analyse_under(source, "jim", &inputs(&jim, "diagnostics", ""));
    assert_eq!(
        analysed.findings.analysis.analysis_context_unavailable,
        None
    );
    let payload = analyze_with(&jim, &inputs(&jim, "diagnostics", ""));
    assert!(
        payload["analysis_context_unavailable"].is_null(),
        "{payload}"
    );
    let shown = analyzed(&jim, "");
    assert!(!has_code(&shown, "W002"), "{shown:?}");
    assert!(!has_code(&shown, "W123"), "{shown:?}");
    let tcl = json!({ "source": source, "dialect": "tcl8.6" });
    let control = analyzed(&tcl, "");
    assert!(has_code(&control, "W002"), "{control:?}");
}

#[test]
fn analyze_honours_an_inline_noqa() {
    let plain = json!({ "source": UNSET_READ, "dialect": "tcl9.0" });
    let shown = analyzed(&plain, "");
    assert!(
        has_code(&shown, "W210"),
        "the control reports W210: {shown:?}"
    );

    let marked = json!({
        "source": format!("# noqa: W210\n{UNSET_READ}"),
        "dialect": "tcl9.0",
    });
    let hidden = analyzed(&marked, "");
    assert!(
        !has_code(&hidden, "W210"),
        "a `# noqa` on the command silences it: {hidden:?}"
    );
}

#[test]
fn analyze_honours_disable_enable_and_the_global_file() {
    let disabled = json!({ "source": UNSET_READ, "dialect": "tcl9.0", "disable": "W210" });
    assert!(!has_code(&analyzed(&disabled, ""), "W210"));

    let plain = json!({ "source": UNSET_READ, "dialect": "tcl9.0" });
    let global = "[diagnostics]\ndisabled = W210\n";
    assert!(
        !has_code(&analyzed(&plain, global), "W210"),
        "the global file reaches the tool"
    );
    let enabled = json!({ "source": UNSET_READ, "dialect": "tcl9.0", "enable": "W210" });
    assert!(
        has_code(&analyzed(&enabled, global), "W210"),
        "`enable` sits above the global file"
    );
}

#[test]
fn analyze_seeds_the_default_off_codes_and_enable_reaches_them() {
    let plain = json!({ "source": UNPROVABLE_LOOP, "dialect": "tcl9.0" });
    let seeded = analyzed(&plain, "");
    assert!(
        !has_code(&seeded, "W242"),
        "W242 is default-off and must not fire unasked: {seeded:?}"
    );
    let enabled = json!({ "source": UNPROVABLE_LOOP, "dialect": "tcl9.0", "enable": "W242" });
    let shown = analyzed(&enabled, "");
    assert!(
        has_code(&shown, "W242"),
        "`enable: W242` reaches a default-off code: {shown:?}"
    );
}

#[test]
fn the_features_toggle_is_an_editor_setting() {
    let args = json!({ "source": UNSET_READ, "dialect": "tcl9.0" });
    let shown = analyzed(&args, "[features]\ndiagnostics = false\n");
    assert!(
        has_code(&shown, "W210"),
        "`[features] diagnostics = false` is the editor's alone: {shown:?}"
    );
}

/// The codes `review` groups under `section` for `source` in `dialect`.
fn reviewed(source: &str, dialect: &str, section: &str) -> Vec<String> {
    let args = json!({ "source": source, "dialect": dialect });
    review_with(&args, &inputs(&args, "diagnostics", ""))[section]
        .as_array()
        .expect("review section")
        .iter()
        .filter_map(|d| d["code"].as_str().map(str::to_owned))
        .collect()
}

/// #2061's three programs: the compiler checks fill `review`'s taint,
/// security and thread-safety sections, which the analyser alone never
/// could.
#[test]
fn review_reports_the_compiler_check_families() {
    let taint = reviewed("set cmd [gets stdin]\neval $cmd\n", "tcl9.0", "taint");
    assert!(taint.iter().any(|c| c == "T100"), "{taint:?}");
    let security = reviewed(
        "when HTTP_REQUEST {\n  set host [HTTP::host]\n  HTTP::respond 200 content \"<h1>$host</h1>\"\n}\n",
        "f5-irules",
        "security",
    );
    assert!(security.iter().any(|c| c == "IRULE3001"), "{security:?}");
    let thread = reviewed(
        "when RULE_INIT { set static::debug 0 }\n",
        "f5-irules",
        "thread_safety",
    );
    assert!(thread.iter().any(|c| c == "IRULE4002"), "{thread:?}");
    let untainted = reviewed("set cmd safe\neval $cmd\n", "tcl9.0", "taint");
    assert!(untainted.is_empty(), "no tainted source: {untainted:?}");
}

/// #2061's lead reproduction: on #2020's own fixture `analyze` reports
/// what `tcl diag` reports (`diag_honours_noqa_directives_the_way_the_editor_does`)
/// — each `# noqa` silences its command, analyser code and compiler-check
/// code alike, and the unmarked findings stand.
#[test]
fn analyze_honours_the_noqa_fixture_as_tcl_diag_does() {
    let fixture = include_str!("../../../tcl-cli/tests/fixtures/noqaSuppression.tcl");
    let args = json!({ "source": fixture, "dialect": "tcl9.0" });
    let shown = analyzed(&args, "");
    let mentions = |needle: &str| {
        shown
            .iter()
            .any(|d| d["message"].as_str().is_some_and(|m| m.contains(needle)))
    };
    for silenced in ["suppressedByCode", "suppressedByBareNoqa", "dictValue"] {
        assert!(!mentions(silenced), "{silenced} is silenced: {shown:?}");
    }
    for reported in ["reportedWithoutAMarker", "reportedBesideProse", "otherDict"] {
        assert!(mentions(reported), "{reported} is reported: {shown:?}");
    }
    assert!(
        has_code(&shown, "S100"),
        "the compiler checks run: {shown:?}"
    );
}

#[test]
fn analyze_reports_the_source_style_pass() {
    let trailing = "set x 1   \n";
    let plain = json!({ "source": trailing, "dialect": "tcl9.0" });
    assert!(has_code(&analyzed(&plain, ""), "W112"));
    let disabled = json!({ "source": trailing, "dialect": "tcl9.0", "disable": "W112" });
    assert!(!has_code(&analyzed(&disabled, ""), "W112"));
}

/// The loader's findings reach a `sslictcl` source, and the loader owns
/// W123 there document-wide — the overlap the editor applies.
#[test]
fn analyze_reports_the_sslictcl_loader() {
    let args = json!({
        "source": "sslictcl 1\nunknown-declaration {a b}\n",
        "dialect": "sslictcl",
    });
    let shown = analyzed(&args, "");
    assert!(has_code(&shown, "SSLIC1101"), "{shown:?}");
    assert!(!has_code(&shown, "W123"), "{shown:?}");
}

/// An O-code a compiler check emits is in the report, hidden because the
/// diagnostics tools run with the optimiser off — not missing.
#[test]
fn a_check_emitted_rewrite_is_suppressed_not_missing() {
    let source = "if {1} { set x 1 } else { set y 2 }\n";
    let args = json!({ "source": source, "dialect": "tcl9.0" });
    assert!(!has_code(&analyzed(&args, ""), "O100"));
    let report =
        analyse_under(source, "tcl9.0", &inputs(&args, "diagnostics", "")).diagnostics_report();
    assert!(
        report.suppressed().any(|(finding, reason)| {
            finding.code == tcl_compiler::compiler_checks::DiagCode::O100
                && reason == tcl_lsp_core::diagnostic_policy::Reason::OptimiserOff
        }),
        "O100 stands in the report as an `OptimiserOff` suppression: {report:?}"
    );
    // The payload's own `suppressed` array carries the same half.
    let suppressed = analyze_suppressed(&args, "");
    assert!(
        suppressed
            .iter()
            .any(|s| s["code"] == "O100" && s["reason"] == "optimiser-off"),
        "{suppressed:?}"
    );
}

/// Each diagnostics tool's payload gains a `suppressed` array
/// (`docs/design/compiler/diagnostic-policy.md` § Adapters, MCP JSON):
/// an inline `# noqa` and `disable` both explain a hidden finding, and a
/// tool's own restriction to its code set still applies to the array.
#[test]
fn the_diagnostics_tools_list_what_the_policy_hides() {
    let marked = json!({ "source": "# noqa: W210\nputs $y\n", "dialect": "tcl9.0" });
    let suppressed = analyze_suppressed(&marked, "");
    let w210 = suppressed
        .iter()
        .find(|s| s["code"] == "W210")
        .unwrap_or_else(|| panic!("no suppressed W210: {suppressed:?}"));
    assert_eq!(w210["reason"], "inline-directive", "{w210}");
    assert_eq!(w210["range"]["start"]["line"], 1, "{w210}");

    let disabled = json!({ "source": "puts $y\n", "dialect": "tcl9.0", "disable": "W210" });
    let suppressed = analyze_suppressed(&disabled, "");
    let w210 = suppressed
        .iter()
        .find(|s| s["code"] == "W210")
        .unwrap_or_else(|| panic!("no suppressed W210: {suppressed:?}"));
    assert_eq!(w210["range"], Value::Null, "{w210}");
    assert_eq!(w210["reason"], "disabled:invocation", "{w210}");

    let review_args = json!({
        "source": "set x hello\n# noqa: S100\nincr x\n",
        "dialect": "tcl9.0",
    });
    let review_suppressed =
        review_with(&review_args, &inputs(&review_args, "diagnostics", ""))["suppressed"]
            .as_array()
            .expect("suppressed array")
            .clone();
    assert!(
        !review_suppressed.iter().any(|s| s["code"] == "S100"),
        "S100 is not a review code: {review_suppressed:?}"
    );
}

/// The optimiser the diagnostics tools never run is one `suppressed`
/// entry per reason, listing its `codes`, rather than an entry per code
/// (`docs/design/compiler/diagnostic-policy.md` § Adapters); a code the
/// compiler checks or the O111 producer emit is not among them, since
/// those producers ran, and a tool's own code set restricts the list,
/// dropping the entry it empties.
#[test]
fn the_diagnostics_tools_collapse_the_optimiser_into_one_entry() {
    use tcl_compiler::compiler_checks::DiagCode;
    use tcl_lsp_core::diagnostic_policy::PRODUCED_WITHOUT_THE_OPTIMISER;

    let only_the_optimisers: Vec<&str> = DiagCode::ALL
        .iter()
        .copied()
        .filter(|code| code.is_optimisation() && !PRODUCED_WITHOUT_THE_OPTIMISER.contains(code))
        .map(DiagCode::as_str)
        .collect();
    let args = json!({ "source": UNSET_READ, "dialect": "tcl9.0" });
    let suppressed = analyze_suppressed(&args, "");
    let not_run: Vec<&Value> = suppressed
        .iter()
        .filter(|s| s.get("codes").is_some())
        .collect();
    assert_eq!(not_run.len(), 1, "{suppressed:?}");
    let entry = not_run[0];
    assert_eq!(entry["producer"], "optimiser", "{entry}");
    assert_eq!(entry["reason"], "optimiser-off", "{entry}");
    assert_eq!(
        entry["message"], "optimiser not run on this surface",
        "{entry}"
    );
    assert_eq!(entry["range"], Value::Null, "{entry}");
    assert_eq!(entry["codes"], json!(only_the_optimisers), "{entry}");
    assert!(
        !suppressed
            .iter()
            .any(|s| s["code"].as_str().is_some_and(|code| code.starts_with('O'))),
        "no O-code has an entry of its own: {suppressed:?}"
    );

    let review_suppressed = review_with(&args, &inputs(&args, "diagnostics", ""))["suppressed"]
        .as_array()
        .expect("suppressed array")
        .clone();
    assert!(
        !review_suppressed.iter().any(|s| s.get("codes").is_some()),
        "no O-code is a review code, so the entry goes: {review_suppressed:?}"
    );
}

#[test]
fn analyze_reports_the_resolved_severity() {
    let args = json!({ "source": UNSET_READ, "dialect": "tcl9.0" });
    let shown = analyzed(&args, "[diagnosticSeverity]\nW210 = error\n");
    let w210 = shown
        .iter()
        .find(|d| d["code"] == "W210")
        .unwrap_or_else(|| panic!("no W210 in {shown:?}"));
    assert_eq!(w210["severity"], "error");
}

#[test]
fn the_grouping_tools_read_the_shown_set() {
    let unmarked = "proc check {a b} {\n    set total [expr $a + $b]\n    return $total\n}\n";
    let marked = "proc check {a b} {\n    # noqa: W100\n    set total [expr $a + $b]\n    return $total\n}\n";
    let legacy = |source: &str| {
        let args = json!({ "source": source, "dialect": "tcl9.0" });
        find_legacy_with(&args, &inputs(&args, "diagnostics", ""))
    };
    let control = legacy(unmarked);
    assert!(
        control["patterns"]
            .as_array()
            .expect("patterns")
            .iter()
            .any(|p| p["code"] == "W100"),
        "the control reports W100: {control}"
    );
    let silenced = legacy(marked);
    assert!(
        silenced["patterns"]
            .as_array()
            .expect("patterns")
            .iter()
            .all(|p| p["code"] != "W100"),
        "a `# noqa: W100` silences the pattern: {silenced}"
    );

    let args = json!({ "source": marked, "dialect": "tcl9.0" });
    let grouped = validate_with(&args, &inputs(&args, "diagnostics", ""));
    let categories = grouped["categories"].as_object().expect("categories");
    assert!(
        categories.values().all(|group| {
            group["items"]
                .as_array()
                .is_none_or(|items| items.iter().all(|d| d["code"] != "W100"))
        }),
        "validate groups only the shown set: {grouped}"
    );
}

/// `optimize` under `global_ini` for `args`.
fn optimized(args: &Value, global_ini: &str) -> Value {
    optimize_with(args, &inputs(args, "optimiser", global_ini))
}

#[test]
fn optimize_applies_only_the_rewrites_the_directives_leave_shown() {
    let plain = json!({ "source": FOLDING, "dialect": "tcl9.0" });
    let folded = optimized(&plain, "");
    assert_eq!(folded["optimized_source"], "set x 3\n", "{folded}");
    assert!(
        folded["optimizations"]
            .as_array()
            .expect("optimizations")
            .iter()
            .any(|o| o["code"] == "O101"),
        "{folded}"
    );

    let marked = format!("# noqa: O101\n{FOLDING}");
    let kept = optimized(&json!({ "source": marked, "dialect": "tcl9.0" }), "");
    assert_eq!(
        kept["optimized_source"],
        marked.as_str(),
        "a `# noqa` on the command keeps the fold off: {kept}"
    );
    assert_eq!(kept["total"], 0, "{kept}");

    let whole = format!("# tcl-lsp: disable=*\n{FOLDING}");
    let untouched = optimized(&json!({ "source": whole, "dialect": "tcl9.0" }), "");
    assert_eq!(untouched["changed"], false, "{untouched}");
}

#[test]
fn optimize_honours_the_profile_the_overrides_and_the_global_file() {
    let readability = optimized(
        &json!({ "source": FOLDING, "dialect": "tcl9.0", "profile": "readability" }),
        "",
    );
    assert_eq!(
        readability["changed"], false,
        "readability turns constant folding off: {readability}"
    );
    let enabled = optimized(
        &json!({
            "source": FOLDING,
            "dialect": "tcl9.0",
            "profile": "readability",
            "enable": "O101",
        }),
        "",
    );
    assert_eq!(
        enabled["optimized_source"], "set x 3\n",
        "`enable` overrides the profile: {enabled}"
    );
    let disabled = optimized(
        &json!({ "source": FOLDING, "dialect": "tcl9.0", "disable": "O101" }),
        "",
    );
    assert_eq!(disabled["changed"], false, "{disabled}");

    let plain = json!({ "source": FOLDING, "dialect": "tcl9.0" });
    let global_off = optimized(&plain, "[optimiser]\ndisabled = O101\n");
    assert_eq!(
        global_off["changed"], false,
        "the global file reaches a rewrite: {global_off}"
    );
    let switched_off = optimized(&plain, "[optimiser]\nenabled = false\n");
    assert_eq!(
        switched_off["changed"], false,
        "the optimiser master switch reaches a rewrite: {switched_off}"
    );
}

/// The owner's ruling on the MCP surface: a named `profile` is the
/// profile in force over the global file's `[optimiser] profile`, which
/// is the default only when the call names none — an MCP `source` has no
/// project layer, so the global file is the whole of that default — and
/// the pass count is the profile in force's.
#[test]
fn optimize_a_named_profile_overrules_the_global_file() {
    let named = json!({ "source": FOLDING, "dialect": "tcl9.0", "profile": "full" });
    let plain = json!({ "source": FOLDING, "dialect": "tcl9.0" });
    let readability = "[optimiser]\nprofile = readability\n";

    let full = optimized(&named, readability);
    assert_eq!(full["optimized_source"], "set x 3\n", "{full}");
    assert_eq!(full["profile"], "full", "{full}");
    let defaulted = optimized(&plain, readability);
    assert_eq!(
        defaulted["changed"], false,
        "with no `profile` the global `readability` runs: {defaulted}"
    );
    assert_eq!(defaulted["profile"], "readability", "{defaulted}");

    let aggressive = "[optimiser]\nprofile = aggressive\n";
    let fixpoint = optimized(&plain, aggressive);
    assert_eq!(fixpoint["multi_pass"], true, "{fixpoint}");
    assert_eq!(
        fixpoint["iterations"], 2,
        "the global `aggressive` runs a second pass that finds nothing: {fixpoint}"
    );
    let single = optimized(&named, aggressive);
    assert_eq!(single["multi_pass"], false, "{single}");
    assert_eq!(
        single["iterations"], 1,
        "a named `full` is one pass over the global `aggressive`: {single}"
    );
}

/// #2062's own program: a `# noqa: O109` over a dead store keeps it
/// through `optimize`, where the unmarked control eliminates it.
#[test]
fn optimize_keeps_a_store_a_noqa_o109_marks() {
    let marked = "proc f {} {\n    # noqa: O109\n    set x 1\n    set x 2\n    return $x\n}\n";
    let kept = optimized(
        &json!({ "source": marked, "dialect": "tcl9.0", "profile": "full" }),
        "",
    );
    assert_eq!(kept["total"], 0, "{kept}");
    assert_eq!(kept["optimized_source"], marked, "{kept}");
    let plain = marked.replace("    # noqa: O109\n", "");
    let control = optimized(
        &json!({ "source": plain, "dialect": "tcl9.0", "profile": "full" }),
        "",
    );
    assert!(
        control["optimizations"]
            .as_array()
            .expect("optimizations")
            .iter()
            .any(|o| o["code"] == "O109"),
        "{control}"
    );
}

/// `code_actions` over the whole of `line` in `source`, under
/// `global_ini`.
fn actions_on_line(source: &str, line: u32, global_ini: &str) -> Vec<Value> {
    let args = json!({
        "source": source,
        "dialect": "tcl9.0",
        "start_line": line,
        "start_character": 0,
        "end_line": line,
        "end_character": 80,
    });
    code_actions_with(&args, &inputs(&args, "diagnostics", global_ini))["actions"]
        .as_array()
        .expect("actions array")
        .clone()
}

/// A lone-`\r` source is read in its analysis form by the rewrite tools,
/// as `tcl opt` and the editor read it: `optimize` and `code_actions`
/// answer for it exactly as for its `\n` twin.
#[test]
fn a_lone_cr_source_optimises_and_acts_like_its_lf_twin() {
    let lf = "proc p {} {\n    return [expr {1 + 2}]\n}\nputs [p]\n";
    let cr = lf.replace('\n', "\r");
    let opt = |source: &str| optimized(&json!({ "source": source, "dialect": "tcl9.0" }), "");
    let (from_lf, from_cr) = (opt(lf), opt(&cr));
    assert!(
        from_lf["total"].as_u64().is_some_and(|n| n > 0),
        "the `\\n` form folds: {from_lf}"
    );
    assert_eq!(from_cr["optimizations"], from_lf["optimizations"]);
    assert_eq!(from_cr["optimized_source"], from_lf["optimized_source"]);
    assert_eq!(from_cr["changed"], from_lf["changed"]);
    let full = "[optimiser]\nprofile = full\n";
    let lf_actions = actions_on_line(lf, 1, full);
    assert!(!lf_actions.is_empty(), "the `\\n` form offers actions");
    assert_eq!(actions_on_line(&cr, 1, full), lf_actions);
}

#[test]
#[allow(clippy::too_many_lines)]
fn original_code_action_candidates_retain_the_supplied_unit_and_refuse_changed_owners() {
    // naming.consumer.original-mcp-refactor-context
    // docs/design/analysis/name-resolution-proofs/original-mcp-refactor-context.md
    let args = json!({
        "source": FOLDING, "dialect": "tcl9.0",
        "start_line": 0, "start_character": 0,
        "end_line": 0, "end_character": 80,
    });
    let inputs = inputs(&args, "diagnostics", "[optimiser]\nprofile = full\n");
    let mut analysed = analyse_under(FOLDING, "tcl9.0", &inputs);
    let original = analysed.findings.analysis.clone();
    let fold = |response: &Value| {
        response["actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|action| {
                action["edits"]
                    .as_array()
                    .is_some_and(|edits| edits.iter().any(|edit| edit["new_text"] == "set x 3"))
            })
    };
    let control = code_actions_from_analysis(&args, &analysed);
    assert!(
        fold(&control),
        "actual code-action consumer offers the retained unit's fold: {control}"
    );
    assert_eq!(code_actions_with(&args, &inputs), control);
    let input = original.resolved_input.as_ref().unwrap();
    assert_eq!(
        input.borrowed_context_registry().commands().snapshot(),
        registry("tcl9.0").snapshot()
    );
    assert_eq!(input.lexer_config(), original.body_lexer_config.unwrap());
    analysed.findings.analysis.resolved_input = None;
    assert!(!fold(&code_actions_from_analysis(&args, &analysed)));
    analysed.findings.analysis = original.clone();
    analysed.findings.analysis.analysis_context_unavailable =
        Some(tcl_registry::model::OverlayMiss {
            environment: "tcl9.0".to_owned(),
            overlay: u64::MAX - 436,
        });
    assert!(!fold(&code_actions_from_analysis(&args, &analysed)));
    analysed.findings.analysis = original.clone();
    analysed
        .findings
        .analysis
        .body_lexer_config
        .as_mut()
        .unwrap()
        .strict_quoting ^= true;
    assert!(!fold(&code_actions_from_analysis(&args, &analysed)));
    let older = tcl_registry::model::resolve_environment("tcl8.4")
        .default_context_registry()
        .with_command_store(std::sync::Arc::clone(
            input.borrowed_context_registry().commands(),
        ));
    let older_input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        std::sync::Arc::new(older),
        input.lexer_config(),
    );
    assert!(std::sync::Arc::ptr_eq(
        input.borrowed_context_registry().commands(),
        older_input.borrowed_context_registry().commands()
    ));
    assert_ne!(input, &older_input);
    analysed.findings.analysis = Analyser::new()
        .with_resolved_input(older_input)
        .analyse(FOLDING, input.analyser_profile().name);
    assert!(analysed.findings.analysis.matches_original_source_image(
        &tcl_lexer::SourceImage::document(FOLDING),
        input.lexer_config()
    ));
    assert!(!fold(&code_actions_from_analysis(&args, &analysed)));
    let mut foreign_store = CommandRegistry::build_default();
    foreign_store.insert(tcl_registry::CommandSpec {
        name: "foreignMcpCandidateStoreMarker",
        ..tcl_registry::CommandSpec::DEFAULT
    });
    assert_ne!(
        foreign_store.snapshot(),
        input.borrowed_context_registry().commands().snapshot()
    );
    let foreign_context = input
        .borrowed_context_registry()
        .with_command_store(foreign_store.snapshot().shared_registry());
    let foreign_input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        std::sync::Arc::new(foreign_context),
        input.lexer_config(),
    );
    analysed.findings.analysis = Analyser::new()
        .with_resolved_input(foreign_input)
        .analyse(FOLDING, input.analyser_profile().name);
    assert!(analysed.findings.analysis.matches_original_source_image(
        &tcl_lexer::SourceImage::document(FOLDING),
        input.lexer_config()
    ));
    assert!(!fold(&code_actions_from_analysis(&args, &analysed)));
    analysed.findings.analysis = original.clone();
    analysed.analysis_text = FOLDING.replace("1 + 2", "1 + 4");
    assert!(!fold(&code_actions_from_analysis(&args, &analysed)));
    analysed.analysis_text = FOLDING.to_owned();
    assert_eq!(code_actions_from_analysis(&args, &analysed), control);
}

#[test]
fn code_actions_offer_nothing_for_a_silenced_finding() {
    let brace = |actions: &[Value]| {
        actions
            .iter()
            .any(|a| a["title"] == "Brace expr for safety and performance")
    };
    let control = actions_on_line("set a 1\nset b [expr $a + 1]\n", 1, "");
    assert!(
        brace(&control),
        "the control offers the refactor: {control:?}"
    );
    let marked = actions_on_line("set a 1\n# noqa: W100\nset b [expr $a + 1]\n", 2, "");
    assert!(
        !brace(&marked),
        "a `# noqa: W100` line offers no brace refactor: {marked:?}"
    );
    let disabled = actions_on_line(
        "set a 1\nset b [expr $a + 1]\n",
        1,
        "[diagnostics]\ndisabled = W100\n",
    );
    assert!(
        !brace(&disabled),
        "a code the global file disables offers no refactor: {disabled:?}"
    );
}

#[test]
fn code_actions_offer_a_shown_rewrite_as_a_quickfix() {
    // The fold rewrites the whole statement it proved constant.
    let fold = |actions: &[Value]| {
        actions.iter().any(|a| {
            a["kind"] == "quickfix"
                && a["edits"]
                    .as_array()
                    .is_some_and(|edits| edits.iter().any(|e| e["new_text"] == "set x 3"))
        })
    };
    let full = "[optimiser]\nprofile = full\n";
    let offered = actions_on_line(FOLDING, 0, full);
    assert!(
        fold(&offered),
        "a shown O101 rewrite is a quick-fix: {offered:?}"
    );
    let marked = actions_on_line(&format!("# noqa: O101\n{FOLDING}"), 1, full);
    assert!(
        !fold(&marked),
        "a silenced rewrite is not offered: {marked:?}"
    );
    let profile_off = actions_on_line(FOLDING, 0, "");
    assert!(
        !fold(&profile_off),
        "the default profile keeps constant folding off: {profile_off:?}"
    );
}

// The diagnostic-policy truth table's rows, run through the MCP tools
// (`docs/design/compiler/diagnostic-policy.md` § The truth table).

/// `analyze`'s `diagnostics` and `suppressed` arrays, as observations:
/// `range` is 0-based (`byte_range`), so a line is `range.start.line`
/// plus one; a gap's `range` is `null`, and a not-run entry is one
/// observation per code it lists.
fn observed_from_analyze(
    result: &Value,
) -> Vec<tcl_lsp_core::diagnostic_policy::truth_table::Observed> {
    use tcl_compiler::analyser::Severity;
    use tcl_compiler::compiler_checks::DiagCode;
    use tcl_lsp_core::diagnostic_policy::truth_table::{Observed, ObservedState};

    fn severity_of(label: &str) -> Option<Severity> {
        match label {
            "hint" => Some(Severity::Hint),
            "suggestion" => Some(Severity::Suggestion),
            "info" => Some(Severity::Info),
            "warning" => Some(Severity::Warning),
            "error" => Some(Severity::Error),
            _ => None,
        }
    }
    let line_of = |range: &Value| -> Option<u32> {
        let line = range.get("start")?.get("line")?.as_u64()?;
        Some(u32::try_from(line).ok()? + 1)
    };

    let mut observed: Vec<Observed> = result["diagnostics"]
        .as_array()
        .expect("diagnostics array")
        .iter()
        .filter_map(|d| {
            Some(Observed {
                code: d["code"].as_str()?.parse::<DiagCode>().ok()?,
                line: line_of(&d["range"]),
                state: ObservedState::Shown(d["severity"].as_str().and_then(severity_of)),
            })
        })
        .collect();
    for s in result["suppressed"].as_array().expect("suppressed array") {
        let reason = s["reason"].as_str().unwrap_or_default().to_owned();
        if let Some(codes) = s["codes"].as_array() {
            let producer = s["producer"].as_str().unwrap_or_default();
            observed.extend(codes.iter().filter_map(|code| {
                Some(Observed {
                    code: code.as_str()?.parse::<DiagCode>().ok()?,
                    line: None,
                    state: ObservedState::NotRun {
                        producer: producer.to_owned(),
                        reason: reason.clone(),
                    },
                })
            }));
        } else if let Some(code) = s["code"]
            .as_str()
            .and_then(|code| code.parse::<DiagCode>().ok())
        {
            observed.push(Observed {
                code,
                line: line_of(&s["range"]),
                state: ObservedState::Suppressed(reason),
            });
        }
    }
    observed
}

/// Every row `analyze` can realise renders its shown and suppressed
/// codes exactly as `Surface::Mcp` expects — the global layer from
/// `Row::ini`, the slot as the call's `disable` / `enable` arguments.
#[test]
fn every_row_renders_through_analyze() {
    use tcl_lsp_core::diagnostic_policy::truth_table::{ROWS, Row, Surface, check};

    let mut failures: Vec<String> = Vec::new();
    for row in ROWS.iter().filter(|row| row.runs_on(Surface::Mcp)) {
        let (text, _) = row.text();
        let flags = row.slot_flags();
        let args = json!({
            "source": text,
            "dialect": row.dialect,
            "disable": flags.disable.join(","),
            "enable": flags.enable.join(","),
        });
        let global_ini = Row::ini(row.global);
        let result = analyze_with(&args, &inputs(&args, "diagnostics", &global_ini));
        if let Err(failure) = check(row, Surface::Mcp, &observed_from_analyze(&result)) {
            failures.push(failure);
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// `code_actions` over the whole document offers a subject's fix exactly
/// when the row shows it — the same three subjects the server's
/// lightbulb pass judges (W100's brace refactor, S100's `# noqa`
/// quick-fix, O101's fold).
#[test]
fn every_row_offers_fixes_for_shown_findings_only() {
    use tcl_lsp_core::diagnostic_policy::truth_table::{
        ActionView, Observed, ObservedState, ROWS, Row, Surface, check, offered,
    };

    let mut failures: Vec<String> = Vec::new();
    for row in ROWS.iter().filter(|row| row.runs_on(Surface::McpActions)) {
        let (text, _) = row.text();
        let end_line = u32::try_from(text.lines().count()).expect("a small program");
        let flags = row.slot_flags();
        let args = json!({
            "source": text,
            "dialect": row.dialect,
            "disable": flags.disable.join(","),
            "enable": flags.enable.join(","),
            "start_line": 0,
            "start_character": 0,
            "end_line": end_line,
            "end_character": 0,
        });
        let global_ini = Row::ini(row.global);
        let result = code_actions_with(&args, &inputs(&args, "diagnostics", &global_ini));
        let views: Vec<ActionView<'_>> = result["actions"]
            .as_array()
            .expect("actions array")
            .iter()
            .map(|action| ActionView {
                title: action["title"].as_str().unwrap_or(""),
                kind: action["kind"].as_str().unwrap_or(""),
                edits: action["edits"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|edit| {
                        let line = edit["range"]["start"]["line"].as_u64()?;
                        let line = u32::try_from(line).ok()? + 1;
                        Some((line, edit["new_text"].as_str()?))
                    })
                    .collect(),
            })
            .collect();
        let observed: Vec<Observed> = row
            .expected(Surface::McpActions)
            .iter()
            .filter_map(|expect| {
                let line = expect.line?;
                Some(Observed {
                    code: expect.code,
                    line: Some(line),
                    state: ObservedState::Offered(offered(expect.code, line, &views)),
                })
            })
            .collect();
        if let Err(failure) = check(row, Surface::McpActions, &observed) {
            failures.push(failure);
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// `optimize` applies the fold exactly when the row shows it — the
/// slot's `profile` and per-code keys as the call's own arguments.
#[test]
fn every_rewrite_row_renders_through_optimize() {
    use tcl_lsp_core::diagnostic_policy::truth_table::{
        Observed, ObservedState, ROWS, Row, Surface, check,
    };

    let mut failures: Vec<String> = Vec::new();
    for row in ROWS.iter().filter(|row| row.runs_on(Surface::McpRewrite)) {
        let (text, _) = row.text();
        let flags = row.slot_flags();
        let args = json!({
            "source": text,
            "dialect": row.dialect,
            "profile": flags.profile.clone().unwrap_or_default(),
            "disable": flags.disable.join(","),
            "enable": flags.enable.join(","),
        });
        let global_ini = Row::ini(row.global);
        let result = optimized(&args, &global_ini);
        let applied = result["optimized_source"]
            .as_str()
            .is_some_and(|source| source.contains("set x 3"));
        let observed: Vec<Observed> = row
            .expected(Surface::McpRewrite)
            .iter()
            .map(|expect| Observed {
                code: expect.code,
                line: expect.line,
                state: ObservedState::Applied(applied),
            })
            .collect();
        if let Err(failure) = check(row, Surface::McpRewrite, &observed) {
            failures.push(failure);
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
