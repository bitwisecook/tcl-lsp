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

//! Workspace `executeCommand` handlers, end-to-end against the packaged server.
//! These drive the real `workspace/executeCommand` dispatch the editors' command
//! palette entries invoke. Each declared parameter is bound positionally, so the
//! argument arrays here mirror the handler signatures exactly.

use crate::common::{Lsp, unique_uri};

use serde_json::{Value, json};

/// The `set` of `code` strings in an `applied` array.
fn applied_codes(result: &Value) -> std::collections::BTreeSet<String> {
    result
        .get("applied")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|e| e.get("code").and_then(Value::as_str).map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// A result's `source` string (empty if absent).
fn source(result: &Value) -> &str {
    result.get("source").and_then(Value::as_str).unwrap_or("")
}

// -- TestDocumentTransforms ----------------------------------------------

#[test]
fn fix_all_safe_issues_braces_a_substitution_free_expression() {
    // TP: nothing substitutes in `abs(-2)`, so `expr` receives the same
    // string braced or not — the class of repair the bulk pass exists for.
    // The assertion is on the returned *source*, not on the `applied` code
    // list, so a fix that reports itself but rewrites the wrong bytes fails.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set n [expr abs(-2)]\n");
    let result = lsp.execute_command("tcl-lsp.fixAllSafeIssues", json!([uri]));
    assert!(!result.is_null());
    assert_eq!(source(&result), "set n [expr {abs(-2)}]\n");
    assert_eq!(
        applied_codes(&result),
        std::collections::BTreeSet::from(["W100".to_owned()])
    );
}

#[test]
fn fix_all_safe_issues_leaves_a_substituted_expression_alone() {
    // FP, and the reason issue #1195 was filed.  Under C Tcl 9.0.3 this
    // program prints `5`: `$a` substitutes to the string `$x`, and `expr`
    // substitutes *that* to 3.  Bracing makes it an error, so the bulk pass
    // must not do it — the individually-named "Brace expr for safety and
    // performance" action still offers it.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "set a {$x}\nset x 3\nset b 2\nputs [expr $a + $b]\n";
    lsp.open_ready(&uri, src);
    let result = lsp.execute_command("tcl-lsp.fixAllSafeIssues", json!([uri]));
    assert!(!result.is_null());
    assert_eq!(source(&result), src, "the source must come back untouched");
    assert!(!applied_codes(&result).contains("W100"), "{result:?}");
}

#[test]
fn fix_all_safe_issues_leaves_a_numeric_string_comparison_alone() {
    // FP: `expr {"1" == "01"}` is 1 (numeric coercion) and
    // `expr {"1" eq "01"}` is 0 (string comparison).  W110's advice is
    // sound; applying it unattended changes results.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "puts [expr {\"1\" == \"01\"}]\n";
    lsp.open_ready(&uri, src);
    let result = lsp.execute_command("tcl-lsp.fixAllSafeIssues", json!([uri]));
    assert!(!result.is_null());
    assert_eq!(source(&result), src);
    assert!(!applied_codes(&result).contains("W110"), "{result:?}");
}

#[test]
fn fix_all_safe_issues_reports_the_safety_class_it_applied() {
    // The `applied` entries name *why* each fix qualified, so a caller need
    // not trust the command's name for that.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set n [expr abs(-2)]\n");
    let result = lsp.execute_command("tcl-lsp.fixAllSafeIssues", json!([uri]));
    let applied = result.get("applied").and_then(Value::as_array).unwrap();
    assert!(!applied.is_empty());
    for entry in applied {
        assert_eq!(
            entry.get("safety").and_then(Value::as_str),
            Some("semantics-equivalent"),
            "{entry:?}"
        );
    }
}

#[test]
fn fix_all_safe_issues_respects_a_disabled_diagnostic() {
    // TN: a diagnostic the user turned off is never analysed, so its fixes
    // cannot be applied in bulk either.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "set n [expr abs(-2)]\n";
    lsp.apply_configuration_settle(
        json!({ "diagnostics": { "W100": false } }),
        &uri,
        |config| {
            config["disabled_diagnostics"]
                .as_array()
                .is_some_and(|codes| codes.iter().any(|code| code == "W100"))
        },
    );
    lsp.open_ready(&uri, src);
    let result = lsp.execute_command("tcl-lsp.fixAllSafeIssues", json!([uri]));
    assert!(!result.is_null());
    assert_eq!(source(&result), src);
}

#[test]
fn minify_strips_comments() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "# comment\nset   x    42\n\nputs $x\n");
    let result = lsp.execute_command("tcl-lsp.minifyDocument", json!([uri, false, false, false]));
    assert!(!result.is_null());
    assert!(
        !source(&result).contains("# comment"),
        "{}",
        source(&result)
    );
    assert!(source(&result).contains("set x 42"), "{}", source(&result));
    let minified = result
        .get("minifiedLength")
        .and_then(Value::as_i64)
        .unwrap();
    let original = result
        .get("originalLength")
        .and_then(Value::as_i64)
        .unwrap();
    assert!(minified < original);
}

#[test]
fn optimise_returns_optimisation_offers() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "puts [llength [list a b c]]\n");
    let result = lsp.execute_command("tcl-lsp.optimiseDocument", json!([uri, "full"]));
    assert!(!result.is_null());
    assert!(source(&result).contains("puts 3"), "{}", source(&result));
}

#[test]
fn optimise_document_preserves_set_only_tk_profile() {
    // Tk is a valid additive dialect surface, but it is intentionally absent
    // from the catalog. The optimiser must receive its typed profile rather
    // than `None`, whose unknown-dialect fallback would offer Tcl 8.6's
    // `tailcall` rewrite for this recursive Tk script.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tk");
    lsp.open_ready_lang(&uri, "proc recurse {} { recurse }\n", "tk");
    let result = lsp.execute_command("tcl-lsp.optimiseDocument", json!([uri, "full"]));
    let codes = result
        .get("optimisations")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| item.get("code").and_then(Value::as_str))
        .collect::<std::collections::BTreeSet<_>>();
    assert!(
        !codes.contains("O121"),
        "Tk must not offer tailcall: {result:?}"
    );
    assert!(source(&result).contains("recurse"), "{result:?}");
    assert!(!source(&result).contains("tailcall"), "{result:?}");
}

#[test]
fn optimise_document_does_not_forward_across_a_variable_trace() {
    // Regression for a confirmed silent miscompile: `tcl-lsp.optimiseDocument`
    // (`profile: "full"`) previously rewrote this to `puts 5`, dropping the
    // `trace add variable` read-handler's `puts "trace fired"` side effect —
    // and, for a write trace, the literal text at the `set` isn't even
    // guaranteed to be the runtime value. tclsh: prints "trace fired" then
    // "5" — the read of `$x` must survive as a real runtime variable access
    // so the trace keeps firing.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc onread {name1 name2 op} {\n    puts \"trace fired\"\n}\nproc setup {} {\n    trace add variable ::x read onread\n}\nsetup\nset x 5\nputs $x\n";
    lsp.open_ready(&uri, src);
    let result = lsp.execute_command("tcl-lsp.optimiseDocument", json!([uri, "full"]));
    assert!(!result.is_null());
    assert!(
        source(&result).contains("puts $x"),
        "trace-guarded read must survive the optimiser unchanged, got: {}",
        source(&result)
    );
}

#[test]
fn optimise_document_does_not_eliminate_a_branch_guarded_by_a_cross_procedural_trace() {
    // Regression for O107 (unreachable-code elimination): SCCP used to have
    // no notion of variable traces at all, so it proved `if {$x}` constant
    // (`x` is `1` at every call) and O107 deleted the "unreachable" `else`
    // body's `puts no` — silently losing the trace-firing read of `$x` the
    // same way the O102 forward above did, but through the DCE path
    // instead. The trace is installed by a *called* proc (`setup`), not
    // lexically between the `set` and the `if`, so only the whole-module
    // `Module::traced_variables` fact (not a same-function scan) catches
    // it. tclsh: prints "trace fired" then "yes" — `else { puts no }` never
    // runs, but the compiler cannot prove that statically, so it must
    // survive in the rewritten source.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc onread {name1 name2 op} {\n    puts \"trace fired\"\n}\nproc setup {} {\n    trace add variable ::x read onread\n}\nset x 1\nsetup\nif {$x} {\n    puts yes\n} else {\n    puts no\n}\n";
    lsp.open_ready(&uri, src);
    let result = lsp.execute_command("tcl-lsp.optimiseDocument", json!([uri, "full"]));
    assert!(!result.is_null());
    assert!(
        source(&result).contains("puts no"),
        "trace-guarded else branch must survive the optimiser unchanged, got: {}",
        source(&result)
    );
    let fired_o107 = result
        .get("optimisations")
        .and_then(Value::as_array)
        .is_some_and(|arr| arr.iter().any(|o| o.get("code") == Some(&json!("O107"))));
    assert!(!fired_o107, "expected no O107, got: {result:?}");
}

#[test]
fn minify_preserves_switch_hash_pattern_arm() {
    // Issue #1197: a braced `switch` case list is a Tcl LIST, not a script —
    // `#` is an ordinary pattern there, never a comment.  tclsh 9.0.4:
    // `switch # { # {puts matched} default {puts default} }` prints
    // `matched`; the pre-fix minifier deleted the `#` arm and the minified
    // script printed `default`.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "switch # {\n    # {puts matched}\n    default {puts default}\n}\n";
    lsp.open_ready(&uri, src);
    let result = lsp.execute_command("tcl-lsp.minifyDocument", json!([uri, false, false, false]));
    assert!(!result.is_null());
    let out = source(&result);
    assert!(
        out.contains("# {puts matched}"),
        "the `#` arm must survive: {out}"
    );
    assert!(out.contains("default {puts default}"), "{out}");
}

#[test]
fn minify_default_tier_adds_no_alias_variables() {
    // Issue #1194: the default tier must stay frame-transparent — the former
    // template deduplication injected a `set a {…}` preamble that clobbered
    // any live variable `a` (observable via `puts [set a]`, traces, and
    // `info vars`).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "puts \"very long dynamic value $x here\"\nputs \"very long dynamic value $x here\"\nputs \"very long dynamic value $x here\"\nputs [set a]\n";
    lsp.open_ready(&uri, src);
    let result = lsp.execute_command("tcl-lsp.minifyDocument", json!([uri, false, false, false]));
    assert!(!result.is_null());
    let out = source(&result);
    assert!(!out.contains("subst"), "no template aliasing: {out}");
    assert!(!out.starts_with("set "), "no alias preamble: {out}");
    assert!(out.contains("puts [set a]"), "{out}");
}

#[test]
fn minify_compact_preserves_public_proc_names_and_array_keys() {
    // Issues #1192/#1193: non-isolated compact keeps procedure names (public
    // command identities — `info procs`, `rename`, external callers) and
    // never rewrites array member keys (Tcl data — `array get` observes
    // them).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc longprocedure {} {\n    set arr(longmember) 1\n    return [array get arr]\n}\nputs [info procs longprocedure]\nputs [longprocedure]\n";
    lsp.open_ready(&uri, src);
    let result = lsp.execute_command("tcl-lsp.minifyDocument", json!([uri, true, false, false]));
    assert!(!result.is_null());
    let out = source(&result);
    assert!(out.contains("proc longprocedure"), "{out}");
    assert!(out.contains("info procs longprocedure"), "{out}");
    assert!(out.contains("arr(longmember)"), "{out}");
}

#[test]
fn minify_preserves_switch_braced_quoted_pattern_closers() {
    // Issue #540: a braced `{a b}` / quoted `"c d"` pattern's end was derived one
    // char short, so the minifier dropped the closing `}` / `"` and re-emitted a
    // truncated, unbalanced pattern. The minified document must keep both patterns
    // intact and stay balanced.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src =
        "switch $x {\n  {a b} { puts one }\n  \"c d\" { puts two }\n  default { puts def }\n}\n";
    lsp.open_ready(&uri, src);
    let result = lsp.execute_command("tcl-lsp.minifyDocument", json!([uri, false, false, false]));
    assert!(!result.is_null());
    let out = source(&result);
    assert!(out.contains("{a b}"), "{out}");
    assert!(out.contains("\"c d\""), "{out}");
    assert_eq!(out.matches('{').count(), out.matches('}').count(), "{out}");
}

// -- TestRegistryLookups -------------------------------------------------

#[test]
fn describe_event_known() {
    let mut lsp = Lsp::tcl();
    let data = lsp.execute_command("tcl-lsp.describeIruleEvent", json!(["HTTP_REQUEST"]));
    assert_eq!(data.get("known"), Some(&Value::Bool(true)));
    assert!(
        data.get("validCommandCount")
            .and_then(Value::as_i64)
            .unwrap()
            >= 1
    );
}

#[test]
fn describe_event_unknown() {
    let mut lsp = Lsp::tcl();
    let data = lsp.execute_command("tcl-lsp.describeIruleEvent", json!(["NOT_A_REAL_EVENT"]));
    assert_eq!(data.get("known"), Some(&Value::Bool(false)));
    assert_eq!(
        data.get("validCommandCount").and_then(Value::as_i64),
        Some(0)
    );
}

#[test]
fn describe_command() {
    let mut lsp = Lsp::tcl();
    let data = lsp.execute_command("tcl-lsp.describeIruleCommand", json!(["HTTP::uri"]));
    assert_eq!(data.get("found"), Some(&Value::Bool(true)));
    assert_eq!(
        data.get("command").and_then(Value::as_str),
        Some("HTTP::uri")
    );
}

#[test]
fn list_irule_events_nonempty() {
    let mut lsp = Lsp::tcl();
    let data = lsp.execute_command("tcl-lsp.listIruleEvents", json!([]));
    let events: Vec<&str> = data
        .get("events")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    assert!(events.contains(&"HTTP_REQUEST"), "{events:?}");
}

// -- TestDiagramAndConfig ------------------------------------------------

#[test]
fn diagram_extracts_irule_events() {
    let mut lsp = Lsp::tcl();
    let source = "proc helper {x} {\n    if {$x} { return ok }\n}\nwhen HTTP_REQUEST {\n    if {[HTTP::uri] eq \"/\"} { pool web }\n}\n";
    let result = lsp.execute_command("tcl-lsp.diagramData", json!([source]));
    assert!(!result.is_null());
    let names: Vec<&str> = result
        .get("events")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|e| e.get("name").and_then(Value::as_str))
                .collect()
        })
        .unwrap_or_default();
    assert!(names.contains(&"HTTP_REQUEST"), "{names:?}");
    let event = result["events"]
        .as_array()
        .and_then(|events| events.first())
        .expect("event payload");
    assert!(event.get("priority").is_some(), "{event}");
    assert!(
        event.get("multiplicity").and_then(Value::as_str).is_some(),
        "{event}"
    );
    assert!(
        event["flow"]
            .as_array()
            .is_some_and(|flow| !flow.is_empty()),
        "{event}"
    );
    let procedures = result["procedures"].as_array().expect("procedures payload");
    assert_eq!(procedures.len(), 1, "{procedures:?}");
    assert_eq!(procedures[0]["name"], "helper");
    assert_eq!(procedures[0]["params"], json!(["x"]));
    assert!(
        procedures[0]["flow"]
            .as_array()
            .is_some_and(|flow| !flow.is_empty())
    );
}

#[test]
fn diagram_data_serialises_completion_contract_for_clients() {
    let mut lsp = Lsp::tcl();
    let source = r"
        proc paths {} {
            return stop
            set after [clock seconds]
        }
    ";
    let result = lsp.execute_command("tcl-lsp.diagramData", json!([source]));
    let return_node = &result["procedures"][0]["flow"][0];
    assert_eq!(return_node["kind"], "return", "{result}");
    assert_eq!(return_node["completion"], "return", "{result}");
}

#[test]
fn effective_config_shape() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x 1\n");
    let data = lsp.execute_command("tcl-lsp.getEffectiveConfig", json!([uri]));
    assert!(data.is_object());
    assert!(data.get("dialect").is_some());
}

// -- TestCommandSurface --------------------------------------------------

/// Commands advertised in `executeCommandProvider` that every conforming backend
/// must expose.
const CORE_COMMANDS: &[&str] = &[
    "tcl-lsp.optimiseDocument",
    "tcl-lsp.minifyDocument",
    "tcl-lsp.fixAllSafeIssues",
    "tcl-lsp.getEffectiveConfig",
    "tcl-lsp.listSubcommands",
    "tcl-lsp.describeIruleEvent",
    "tcl-lsp.describeIruleCommand",
    "tcl-lsp.listIruleEvents",
    "tcl-lsp.diagramData",
];

#[test]
fn core_commands_are_advertised() {
    let lsp = Lsp::tcl();
    let advertised: std::collections::BTreeSet<String> = lsp
        .initialize_result()
        .get("capabilities")
        .and_then(|c| c.get("executeCommandProvider"))
        .and_then(|p| p.get("commands"))
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    let missing: Vec<&str> = CORE_COMMANDS
        .iter()
        .copied()
        .filter(|c| !advertised.contains(*c))
        .collect();
    assert!(
        missing.is_empty(),
        "executeCommandProvider missing core commands: {missing:?}"
    );
}

#[test]
fn minify_compact_round_trip() {
    // Compact renaming shortens identifiers and reports the reverse map, so a name
    // in the minified source resolves back to the original.  Proc renaming
    // needs `isolated` (the 4th argument): a proc name is a public command
    // identity the non-isolated tier must preserve (issue #1193).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "proc addNumbers {a b} { return [expr {$a + $b}] }\naddNumbers 1 2\n",
    );
    let result = lsp.execute_command("tcl-lsp.minifyDocument", json!([uri, true, false, true]));
    assert!(!result.is_null());
    let minified = result
        .get("minifiedLength")
        .and_then(Value::as_i64)
        .unwrap();
    let original = result
        .get("originalLength")
        .and_then(Value::as_i64)
        .unwrap();
    assert!(minified < original);
    assert!(
        !source(&result).contains("addNumbers"),
        "{}",
        source(&result)
    );
    // The minified source must still map back to the original name. The server
    // may report `symbolMap` as an object or as a formatted string, so accept a
    // key on the object and a substring in the string.
    let symbol_map = result.get("symbolMap").cloned().unwrap_or(Value::Null);
    let maps_back = match &symbol_map {
        Value::Object(_) => symbol_map.get("addNumbers").is_some(),
        Value::String(s) => s.contains("addNumbers"),
        _ => false,
    };
    assert!(maps_back, "{symbol_map}");
}

#[test]
fn list_subcommands_shape() {
    let mut lsp = Lsp::tcl();
    let data = lsp.execute_command("tcl-lsp.listSubcommands", json!(["string"]));
    assert_eq!(data.get("command").and_then(Value::as_str), Some("string"));
    let names: std::collections::BTreeSet<&str> = data
        .get("subcommands")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|s| s.get("name").and_then(Value::as_str))
                .collect()
        })
        .unwrap_or_default();
    for expected in ["length", "range", "map"] {
        assert!(
            names.contains(expected),
            "missing {expected:?} in {names:?}"
        );
    }
}

#[test]
fn list_subcommands_unknown_is_empty() {
    let mut lsp = Lsp::tcl();
    let data = lsp.execute_command(
        "tcl-lsp.listSubcommands",
        json!(["definitely_not_a_command_zzz"]),
    );
    assert_eq!(
        data.get("subcommands").and_then(Value::as_array),
        Some(&Vec::new())
    );
}

#[test]
fn list_known_packages_shape() {
    let mut lsp = Lsp::tcl();
    let data = lsp.execute_command("tcl-lsp.listKnownPackages", json!([]));
    assert!(data.get("packages").is_some_and(Value::is_array), "{data}");
}

#[test]
fn suggest_packages_for_symbol_shape() {
    let mut lsp = Lsp::tcl();
    let data = lsp.execute_command("tcl-lsp.suggestPackagesForSymbol", json!(["json::write"]));
    assert_eq!(
        data.get("symbol").and_then(Value::as_str),
        Some("json::write")
    );
    assert!(
        data.get("suggestions").is_some_and(Value::is_array),
        "{data}"
    );
}

#[test]
fn export_config_writes_a_file() {
    let mut lsp = Lsp::tcl();
    let data = lsp.execute_command("tcl-lsp.exportConfig", json!([]));
    assert_eq!(data.get("success"), Some(&Value::Bool(true)));
    let path = data.get("path").and_then(Value::as_str).unwrap_or("");
    assert!(!path.is_empty(), "{data}");
}
