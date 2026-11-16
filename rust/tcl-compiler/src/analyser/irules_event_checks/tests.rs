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

use super::*;

fn codes(source: &str) -> Vec<(String, String)> {
    let mut a = Analyser::new();
    let res = a.analyse(source, "f5-irules");
    res.diagnostics
        .iter()
        .filter(|d| d.code.as_str().starts_with("IRULE"))
        .map(|d| (d.code.to_string(), d.message.clone()))
        .collect()
}

fn has(source: &str, code: &str) -> bool {
    codes(source).iter().any(|(c, _)| c == code)
}

/// User decision 2026-10-21T00:10Z ("realm-aware command world"): the
/// rule loader defines a rule's `proc` at load time, so a document
/// supplied without an execution entry publishes it as an original
/// procedure declaration, which editor references and rename select.
#[test]
fn rule_procedure_is_an_original_declaration_of_the_fresh_rule_load() {
    let source = "proc hit {} { return hit }\nwhen HTTP_REQUEST { call hit }\n";
    let analysis = crate::analyser::Analyser::new().analyse(source, "f5-irules");
    let names = analysis
        .original_procedure_declarations()
        .map(|declaration| declaration.name().reported_full_name())
        .collect::<Vec<_>>();
    assert_eq!(names.len(), 1, "{names:?}");
    assert!(
        names[0]
            .as_deref()
            .is_some_and(|name| name.ends_with("hit")),
        "{names:?}"
    );
}

/// The fresh rule load's own `proc` is the loader builtin, so its
/// invocation keeps the original lookup editor selection needs at the
/// declaration (`proc hit` in a document supplied without an entry).
#[test]
fn rule_loader_proc_invocation_keeps_its_original_lookup() {
    let source =
        "proc hit {} { return hit }\nswitch -glob $value {\n    default {\n        hit\n    }\n}\n";
    let analysis = crate::analyser::Analyser::new().analyse(source, "f5-irules");
    let invocation = analysis
        .command_invocations
        .iter()
        .find(|invocation| invocation.range.start() == 0)
        .expect("the proc invocation");
    assert!(invocation.original_name_input.is_some());
    assert!(invocation.original_lookup.is_some());
    assert_eq!(analysis.original_procedure_declarations().count(), 1);
}

#[test]
fn irule1004_is_quiet_for_bigip_default_priority() {
    // BIG-IP gives every `when` handler priority 500 when the clause is
    // omitted, so a bare handler is valid and must not produce a hint.
    assert!(!has("when HTTP_REQUEST { set x 1 }", "IRULE1004"));
}

#[test]
fn irule1004_suppressed_with_explicit_priority() {
    assert!(!has(
        "when HTTP_REQUEST priority 100 { set x 1 }",
        "IRULE1004"
    ));
}

#[test]
fn irule1002_fires_for_unknown_event() {
    assert!(has("when BOGUS_EVENT { log local0. hi }", "IRULE1002"));
}

#[test]
fn irule1002_keeps_valid_unknown_events_but_ignores_non_braced_bodies() {
    assert!(has("when BOGUS_EVENT priority 200 {}", "IRULE1002"));
    for source in [
        "when BOGUS_EVENT bare_body",
        "when BOGUS_EVENT \"quoted body\"",
    ] {
        assert!(
            !has(source, "IRULE1002"),
            "a non-declaration must not report an event selector: {source}"
        );
    }
}

#[test]
fn unknown_event_body_is_inert_except_for_irule1002() {
    let got = codes("when BOGUS_EVENT { log local0. hi; HTTP::uri }");
    let irules: Vec<_> = got
        .iter()
        .map(|(code, _)| code.as_str())
        .filter(|code| code.starts_with("IRULE"))
        .collect();
    assert_eq!(irules, ["IRULE1002"]);
}

#[test]
fn irule1002_quiet_for_known_event() {
    assert!(!has("when HTTP_REQUEST { set x 1 }", "IRULE1002"));
}

#[test]
fn irule3102_fires_for_unnormalised_getter() {
    assert!(has("when HTTP_REQUEST { set u [HTTP::uri] }", "IRULE3102"));
}

#[test]
fn irule3102_quiet_with_normalized_flag() {
    assert!(!has(
        "when HTTP_REQUEST { set u [HTTP::uri -normalized] }",
        "IRULE3102"
    ));
}

#[test]
fn irule3102_fires_for_getter_nested_in_argument() {
    assert!(has(
        "when HTTP_REQUEST { HTTP::respond 200 content [HTTP::query] }",
        "IRULE3102"
    ));
}

#[test]
fn irule2003_fires_for_unsafe_command() {
    let cs = codes("when HTTP_REQUEST { uplevel 1 { set x 1 } }");
    assert!(cs.iter().any(|(c, _)| c == "IRULE2003"));
}

#[test]
fn irule1003_fires_for_deprecated_event() {
    assert!(has("when AUTH_SUCCESS { log local0. hi }", "IRULE1003"));
}

#[test]
fn irule1003_quiet_for_live_event() {
    assert!(!has("when HTTP_REQUEST { set x 1 }", "IRULE1003"));
}

#[test]
fn irule2101_fires_for_regexp_in_hot_event() {
    assert!(has(
        "when HTTP_REQUEST { regexp {^/a} [HTTP::uri] }",
        "IRULE2101"
    ));
}

#[test]
fn irule2101_quiet_outside_hot_event() {
    assert!(!has("when CLIENT_DATA { regexp {^/a} $x }", "IRULE2101"));
}

#[test]
fn irule5001_fires_for_log_in_hot_event() {
    assert!(has("when HTTP_REQUEST { log local0. hi }", "IRULE5001"));
}

#[test]
fn irule5001_quiet_for_log_gated_on_static_flag() {
    // The fix the message prescribes has to clear the diagnostic; a
    // check that survives its own advice teaches the reader to ignore it.
    assert!(!has(
        "when HTTP_REQUEST { if {$static::debug} { log local0. hi } }",
        "IRULE5001"
    ));
}

#[test]
fn irule5001_quiet_for_log_gated_on_braced_static_flag() {
    assert!(!has(
        "when HTTP_REQUEST { if {${static::debug}} { log local0. hi } }",
        "IRULE5001"
    ));
}

#[test]
fn irule5001_quiet_for_log_gated_on_static_existence_check() {
    assert!(!has(
        "when HTTP_REQUEST { if {[info exists static::debug]} { log local0. hi } }",
        "IRULE5001"
    ));
}

#[test]
fn irule5001_quiet_in_nested_body_under_a_static_gate() {
    // Nested bodies inherit the gate: the `log` is still reached only
    // when the flag is set.
    assert!(!has(
        "when HTTP_REQUEST { if {$static::debug} { \
             if {[HTTP::uri] eq \"/a\"} { log local0. hi } } }",
        "IRULE5001"
    ));
}

#[test]
fn irule5001_quiet_in_else_arm_of_a_static_gate() {
    // Whichever way a decision made on a debug flag goes, the `log` is
    // gated on that flag; over-suppressing is the safe direction here.
    assert!(!has(
        "when HTTP_REQUEST { if {$static::quiet} { set x 1 } else { log local0. hi } }",
        "IRULE5001"
    ));
}

#[test]
fn irule5001_quiet_for_log_gated_on_a_switch_arm() {
    assert!(!has(
        "when HTTP_REQUEST { switch $static::level { high { log local0. hi } } }",
        "IRULE5001"
    ));
}

#[test]
fn irule5001_quiet_for_flag_set_in_rule_init() {
    // The message prescribes `set debug 0` in a setup event plus
    // `if {$debug}`; that pair has to work too.
    assert!(!has(
        "when RULE_INIT { set debug 0 }\n\
             when HTTP_REQUEST { if {$::debug} { log local0. hi } }",
        "IRULE5001"
    ));
    assert!(has(
        "when RULE_INIT {set debug 0}\nwhen HTTP_REQUEST {if {$debug} {log local0. hi}}",
        "IRULE5001"
    ));
}

#[test]
fn irule5001_quiet_for_flag_set_in_a_later_setup_event() {
    // Handler order in the file must not decide the diagnostic.
    assert!(!has(
        "when HTTP_REQUEST { if {$debug} { log local0. hi } }\n\
             when CLIENT_ACCEPTED { set debug 0 }",
        "IRULE5001"
    ));
}

#[test]
fn irule5001_still_fires_under_a_condition_that_reads_no_flag() {
    // An ordinary conditional is not a debug gate.
    assert!(has(
        "when HTTP_REQUEST { if {[HTTP::uri] eq \"/a\"} { log local0. hi } }",
        "IRULE5001"
    ));
}

#[test]
fn irule5001_still_fires_after_a_sibling_gate_closes() {
    // The gate depth must unwind: a bare `log` following a gated one is
    // still ungated.
    assert!(has(
        "when HTTP_REQUEST { if {$static::debug} { log local0. gated }\n\
             log local0. bare }",
        "IRULE5001"
    ));
}

#[test]
fn irule5001_still_fires_in_a_loop_over_a_static_list() {
    // A loop selects its body by iteration, not by a decision. The
    // `log` still runs once per request per element.
    assert!(has(
        "when HTTP_REQUEST { foreach ip $static::allowlist { log local0. $ip } }",
        "IRULE5001"
    ));
}

#[test]
fn irule5001_still_fires_in_a_while_loop_on_a_static_flag() {
    assert!(has(
        "when HTTP_REQUEST { while {$static::debug} { log local0. hi } }",
        "IRULE5001"
    ));
}

#[test]
fn irule5001_quiet_for_log_gated_inside_a_loop() {
    // The gate still reaches a body nested under a loop — the loop
    // itself is not the gate, the `if` inside it is.
    assert!(!has(
        "when HTTP_REQUEST { foreach ip $static::allowlist { \
             if {$static::debug} { log local0. $ip } } }",
        "IRULE5001"
    ));
}

#[test]
fn irule5001_quiet_for_flag_set_in_a_switch_arm_of_a_setup_event() {
    // A clause list is a pattern/body sequence, not a script: read as one
    // script its patterns become command heads and the arm assignments
    // vanish, leaving the gated `log` reported.
    let source = "when CLIENT_ACCEPTED { switch $mode { \
             loud { set debug 1 } default { set debug 0 } } }\n\
             when HTTP_REQUEST { if {$debug} { log local0. hi } }";
    let mut analyser = Analyser::new();
    let result = analyser.analyse(source, "f5-irules");
    assert!(
        result
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != DiagCode::Irule5001),
        "flags: {:?}; diagnostics: {:?}",
        analyser.irules_debug_flag_names(),
        result.diagnostics
    );
}

#[test]
fn irule5001_still_fires_for_catch_naming_a_static_result_variable() {
    // `catch`'s body always runs, so it can never be a gate. Its other
    // arguments are variable names it writes, not a decision reading one.
    assert!(has(
        "when HTTP_REQUEST { catch { log local0. hi } static::err }",
        "IRULE5001"
    ));
}

#[test]
fn irule5001_gate_does_not_leak_into_a_later_event() {
    assert!(has(
        "when CLIENT_ACCEPTED { if {$static::debug} { log local0. gated } }\n\
             when HTTP_REQUEST { log local0. bare }",
        "IRULE5001"
    ));
}

#[test]
fn irule4001_fires_for_static_write_outside_rule_init() {
    assert!(has("when HTTP_REQUEST { set static::c 1 }", "IRULE4001"));
}

#[test]
fn storage_diagnostics_follow_canonical_cells_and_point_aliases() {
    for target in ["static::c", "::static::c"] {
        assert!(has(
            &format!("when HTTP_REQUEST {{set {target} 1}}"),
            "IRULE4001"
        ));
        assert!(!has(
            &format!("when HTTP_REQUEST {{set {target} 1}}"),
            "IRULE6001"
        ));
    }
    let static_alias = "when HTTP_REQUEST {upvar #0 ::static::c linked; set linked 1}";
    assert!(has(static_alias, "IRULE4001"));
    assert!(!has(static_alias, "IRULE6001"));
    assert!(has(
        "when HTTP_REQUEST {upvar #0 ::counter linked; set linked 1}",
        "IRULE6001"
    ));
    assert!(!has("when HTTP_REQUEST {set local_value 1}", "IRULE6001"));
}

#[test]
fn irule4001_suppressed_in_rule_init() {
    assert!(!has("when RULE_INIT { set static::c 1 }", "IRULE4001"));
}

#[test]
fn irule6001_fires_for_global_qualified_write() {
    assert!(has("when HTTP_REQUEST { set ::counter 0 }", "IRULE6001"));
}

#[test]
fn irule6001_fires_for_implicit_global_in_rule_init() {
    assert!(has("when RULE_INIT { set greeting hi }", "IRULE6001"));
}

#[test]
fn irule6001_fires_for_global_command() {
    let cs = codes("when HTTP_REQUEST { global shared }");
    assert!(
        cs.iter()
            .any(|(c, m)| c == "IRULE6001" && m.contains("'global shared'"))
    );
}

fn irule6001_count(source: &str) -> usize {
    codes(source)
        .iter()
        .filter(|(c, _)| c == "IRULE6001")
        .count()
}

/// The writer set is the registry's `ArgRole::VarWrite` answer for the
/// concrete call, so multi-name and switch-skipping writers — which a table
/// of one index per command cannot express — are covered.
#[test]
fn irule6001_fires_for_registry_writers_the_name_table_missed() {
    for (source, expected) in [
        ("when HTTP_REQUEST { catch {HTTP::collect} ::err }", 1),
        ("when HTTP_REQUEST {set s a; regexp {(a)} $s ::m ::one }", 2),
        (
            "when HTTP_REQUEST {set s {1 2}; scan $s {%d %d} ::v ::w }",
            2,
        ),
        ("when HTTP_REQUEST {set d AB; binary scan $d H2 ::v }", 1),
    ] {
        assert_eq!(
            irule6001_count(source),
            expected,
            "{source} must report {expected} IRULE6001; got {:?}",
            codes(source)
        );
    }
}

#[test]
fn irule6001_does_not_invent_writes_for_unavailable_core_commands() {
    for source in [
        "when HTTP_REQUEST {lassign {1 2} ::a ::b}",
        "when HTTP_REQUEST {const ::c 1}",
        "when HTTP_REQUEST {ledit ::c 0 0 x}",
        "when HTTP_REQUEST {lpop ::c}",
    ] {
        assert!(!has(source, "IRULE6001"), "{source}");
    }
}

/// `array names` declares [`ArgRole::VarRead`], not `VarWrite`, so it is not
/// read as a write (an unconditional `"array" => Some(1)` arm would be, and
/// would offer a code fix rewriting the *read* to `static::x`).
#[test]
fn irule6001_quiet_for_array_names_read() {
    assert!(!has("when HTTP_REQUEST { array names ::x }", "IRULE6001"));
    assert!(has(
        "when HTTP_REQUEST { array set ::x {a 1} }",
        "IRULE6001"
    ));
}

/// Every command a hand-written writer table would list still reports.
#[test]
fn irule6001_still_fires_for_every_name_the_old_table_listed() {
    for source in [
        "when HTTP_REQUEST { append ::c x }",
        "when HTTP_REQUEST { incr ::c }",
        "when HTTP_REQUEST { lappend ::c x }",
        "when HTTP_REQUEST { lset ::c 0 x }",
        "when HTTP_REQUEST { set ::c 0 }",
        "when HTTP_REQUEST { unset ::c }",
        "when HTTP_REQUEST { variable ::c }",
        "when HTTP_REQUEST { array set ::c {a 1} }",
        "when HTTP_REQUEST {set ch stdin; eval [list gets $ch ::c] }",
    ] {
        assert!(
            has(source, "IRULE6001"),
            "{source} must still report IRULE6001; got {:?}",
            codes(source)
        );
    }
    // The load-time refusal prevents a literal handler from reaching its
    // output binding; a constructed runtime script retains that hazard.
    let literal = "when HTTP_REQUEST {set ch stdin; gets $ch ::c}";
    assert!(has(literal, "IRULE2004"));
    assert!(!has(literal, "IRULE6001"));
    let constructed = "when HTTP_REQUEST {set ch stdin; eval [list gets $ch ::c]}";
    assert!(!has(constructed, "IRULE2004"), "{:?}", codes(constructed));
    assert!(has(constructed, "IRULE6001"), "{:?}", codes(constructed));
    // `global` keeps its own message and its own branch.
    assert!(has("when HTTP_REQUEST { global shared }", "IRULE6001"));
}

/// The `RULE_INIT` implicit-global variant reaches the same registry
/// writers, and its three carve-outs are registry facts:
/// `set var` with one argument resolves as `VarRead`, and a destroyer
/// (`unset`, `array unset`) removes a variable rather than creating an
/// implicit global.
#[test]
fn irule6001_implicit_global_in_rule_init_follows_the_registry() {
    assert_eq!(
        irule6001_count("when RULE_INIT { scan {1 2} {%d %d} a b }"),
        2
    );
    assert!(has("when RULE_INIT { catch {foo} err }", "IRULE6001"));
    assert!(has("when RULE_INIT { set greeting hi }", "IRULE6001"));
    // `set var` (one argument) is a read.
    assert!(!has("when RULE_INIT { set greeting }", "IRULE6001"));
    // Destroyers create nothing.
    assert!(!has("when RULE_INIT { unset foo }", "IRULE6001"));
    assert!(!has("when RULE_INIT { array unset foo }", "IRULE6001"));
    // `array names` is a read here too.
    assert!(!has("when RULE_INIT { array names foo }", "IRULE6001"));
    // A `static::` target is already per-TMM.
    assert!(!has("when RULE_INIT { set static::c 1 }", "IRULE6001"));
}

/// The seven-row dynamic-code table of measurements §4c: the rule
/// compiler's load-time bans scan **braced script literals** — wrapping
/// `proc` in `eval {…}` / `uplevel #0 {…}` changes nothing, and nesting
/// is not an escape hatch — while `eval`/`uplevel` themselves are fine,
/// a literal call head still fails load-time resolution (§4a), and
/// script text held in a variable escapes the scan entirely, widening
/// the realm state instead (a runtime-defined proc is a persistent
/// per-TMM global surviving across events and connections).
#[test]
fn the_measurements_4c_seven_row_dynamic_code_table() {
    // Row 1: bare `proc` in a `when` body — REJECT.
    assert!(has(
        "when RULE_INIT { proc p {} { return 1 } }",
        "IRULE5006"
    ));
    // Rows 2-3: braced `eval` / `uplevel` wrappers — REJECT with the
    // same diagnostic as the bare form (the scan is lexical). Any
    // level spelling; nesting is not an escape hatch.
    for src in [
        "when RULE_INIT { eval {proc p {} { return 1 }} }",
        "when RULE_INIT { uplevel #0 {proc p {} { return 1 }} }",
        "when RULE_INIT { uplevel {proc p {} { return 1 }} }",
        "when RULE_INIT { uplevel 1 {proc p {} { return 1 }} }",
        "when RULE_INIT { eval {eval {proc p {} { return 1 }}} }",
    ] {
        assert!(has(src, "IRULE5006"), "{src}: §4c braced-literal scan");
    }
    // Rows 4-5: `eval` / `uplevel` of a harmless braced literal —
    // ACCEPT (`eval` itself is fine; the ban is on `proc`).
    assert!(!has(
        "when RULE_INIT { eval {set static::x 1} }",
        "IRULE5006"
    ));
    assert!(!has(
        "when RULE_INIT { uplevel #0 {set static::x 1} }",
        "IRULE5006"
    ));
    // Row 6: top-level `proc` + literal call head — REJECT (§4a
    // load-time resolution: a rule proc is reachable only via `call`).
    assert!(has(
        "proc p {} { return 1 }\nwhen RULE_INIT { p }",
        "IRULE5005"
    ));
    assert!(!has(
        "proc p {} { return 1 }\nwhen RULE_INIT { call p }",
        "IRULE5005"
    ));
    // Row 7: variable-held define + call — ACCEPT, with the May
    // widening: the hidden script may define procs (persistent
    // per-TMM globals, §4c), so the realm state records a dynamic
    // provider and literal-surface checks abstain.
    let hidden = "when RULE_INIT { set s \"proc dpp {} { return persisted }\"\n  \
                      uplevel #0 $s\n  set c \"dpp\"\n  uplevel #0 $c }";
    let mut analyser = Analyser::new();
    let res = analyser.analyse(hidden, "f5-irules");
    assert!(
        res.has_dynamic_providers,
        "a dynamic uplevel at event scope must widen the realm state"
    );
    assert!(
        !res.diagnostics
            .iter()
            .any(|d| matches!(d.code.as_str(), "IRULE5005" | "IRULE5006" | "W123")),
        "the hidden define+call form is accepted: {:?}",
        res.diagnostics
    );
    // Control: a braced-literal eval body does NOT widen the realm.
    let mut analyser = Analyser::new();
    let res = analyser.analyse("when RULE_INIT { eval {set static::x 1} }", "f5-irules");
    assert!(!res.has_dynamic_providers);
}

#[test]
fn irule5006_fires_for_proc_in_nested_body() {
    assert!(has(
        "when HTTP_REQUEST { proc helper {} { return 1 } }",
        "IRULE5006"
    ));
}

#[test]
fn irule5006_quiet_for_top_level_proc() {
    assert!(!has("proc helper {} { return 1 }", "IRULE5006"));
}

#[test]
fn irule5007_fires_for_event_command_at_top_level() {
    assert!(has("HTTP::respond 200", "IRULE5007"));
}

#[test]
fn irule5007_quiet_inside_when_block() {
    assert!(!has("when HTTP_REQUEST { HTTP::respond 200 }", "IRULE5007"));
}

#[test]
fn irule5007_rejects_every_executable_top_level_form() {
    for source in ["set x 1", "puts hello", "call helper", "helper"] {
        assert!(has(source, "IRULE5007"), "missing IRULE5007 for {source:?}");
    }
}

#[test]
fn irule5007_accepts_exactly_the_registry_declaration_surface() {
    for source in [
        "when RULE_INIT {}",
        "proc helper {} {}",
        "timing on",
        "priority 100",
    ] {
        assert!(
            !has(source, "IRULE5007"),
            "unexpected IRULE5007 for {source:?}"
        );
    }
}

#[test]
fn irule4003_fires_for_cross_event_variable() {
    let src = "when HTTP_REQUEST { set token abc }\nwhen CLIENT_DATA { log local0. $token }";
    assert!(has(src, "IRULE4003"));
}

#[test]
fn irule4003_ignores_literal_variable_text() {
    assert!(!has(
        "when HTTP_REQUEST {set token abc}\nwhen CLIENT_DATA {log local0. {$token}}",
        "IRULE4003"
    ));
}

#[test]
fn irule4003_quiet_for_commented_out_second_event() {
    // The "other event" exists only in a comment, so there is no second
    // handler and no cross-event concern.  A byte scan would read the
    // comment as a `when` block and invent the hint.
    let src = "when HTTP_REQUEST { set token abc }\n# when CLIENT_DATA { log local0. $token }";
    assert!(!has(src, "IRULE4003"));
}

#[test]
fn irule4003_quiet_for_second_event_inside_a_string_literal() {
    let src = "when HTTP_REQUEST { set token abc }\n\
                   log local0. \"when CLIENT_DATA { log local0. $token }\"";
    assert!(!has(src, "IRULE4003"));
}

#[test]
fn irule5003_fires_for_ne_zero_decrement_loop() {
    assert!(has(
        "when HTTP_REQUEST { while {$count != 0} { incr count -1 } }",
        "IRULE5003"
    ));
}

#[test]
fn irule5003_fires_for_zero_ne_braced_form() {
    assert!(has(
        "when HTTP_REQUEST { while {0 ne ${n}} { incr n -2 } }",
        "IRULE5003"
    ));
}

#[test]
fn irule5003_quiet_for_greater_than_zero() {
    assert!(!has(
        "when HTTP_REQUEST { while {$x > 0} { incr x -1 } }",
        "IRULE5003"
    ));
}

#[test]
fn irule5003_quiet_when_body_decrements_other_var() {
    assert!(!has(
        "when HTTP_REQUEST { while {$count != 0} { incr other -1 } }",
        "IRULE5003"
    ));
}

/// IRULE5003 takes the loop whose iteration plan is a bare condition, and
/// a decrement the registry's integer cell update states at any depth of
/// its body: the text `incr count -1` as a `log` argument is data, which
/// the substring scan read as a decrement.
#[test]
fn irule5003_reads_the_plan_and_the_cell_update() {
    assert!(has(
        "when HTTP_REQUEST { while {$count != 0} { if {1} { incr count -1 } } }",
        "IRULE5003"
    ));
    assert!(has(
        "when HTTP_REQUEST { while {$count != 0} { ::incr count -3 } }",
        "IRULE5003"
    ));
    assert!(!has(
        "when HTTP_REQUEST { while {$count != 0} { log local0. \"incr count -1\"; set count 0 } }",
        "IRULE5003"
    ));
    assert!(!has(
        "when HTTP_REQUEST { while {$count != 0} { incr count } }",
        "IRULE5003"
    ));
}

#[test]
fn find_loop_ne_zero_handles_both_orders() {
    assert_eq!(find_loop_ne_zero("$count != 0").as_deref(), Some("count"));
    assert_eq!(find_loop_ne_zero("0 ne ${n}").as_deref(), Some("n"));
    assert_eq!(find_loop_ne_zero("$x > 0"), None);
}

#[test]
fn no_irule_checks_outside_f5_dialect() {
    let mut a = Analyser::new();
    let res = a.analyse("when AUTH_SUCCESS { set static::c 1 }", "tcl");
    assert!(
        !res.diagnostics
            .iter()
            .any(|d| d.code.as_str().starts_with("IRULE"))
    );
}

#[test]
fn collect_event_bodies_extracts_priority_form() {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let blocks = super::collect_event_bodies(
        "when HTTP_REQUEST priority 5 { body1 }",
        Some(registry),
        &crate::realm::CommandBindingRealm::default(),
    );
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].0, "HTTP_REQUEST");
    assert_eq!(blocks[0].1[0].trim(), "body1");
}

#[test]
fn collect_event_bodies_ignores_comments_and_string_literals() {
    // Neither a commented-out handler nor one quoted inside a string is a
    // command, so neither contributes an event body; a byte scan would
    // invent both.
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let source = "# when HTTP_REQUEST { log local0. $x }\n\
                      log local0. \"when LB_SELECTED { set y 1 }\"\n\
                      when CLIENT_ACCEPTED { set z 1 }\n";
    let blocks = super::collect_event_bodies(
        source,
        Some(registry),
        &crate::realm::CommandBindingRealm::default(),
    );
    let names: Vec<&str> = blocks.iter().map(|(e, _)| e.as_str()).collect();
    assert_eq!(names, vec!["CLIENT_ACCEPTED"]);
}

/// TMM accepts a handler written with no separator before its body —
/// `when {HTTP_REQUEST}{ … }` — because the `}{` sequence acts as a
/// command-word separator in the iRules grammar.  The collector must
/// segment with the *document's* dialect config to see it: segmenting with
/// `LexerConfig::default()` reads the whole line as one malformed word,
/// finds no event handler, and silently drops every handler written that
/// way from IRULE4003's index.
#[test]
fn collect_event_bodies_honours_the_tmm_ghost_separator() {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let source = "when {HTTP_REQUEST}{ set token abc }\n\
                      when {CLIENT_DATA}{ log local0. $token }\n";
    let blocks = super::collect_event_bodies(
        source,
        Some(registry),
        &crate::realm::CommandBindingRealm::default(),
    );
    let names: Vec<&str> = blocks.iter().map(|(e, _)| e.as_str()).collect();
    assert_eq!(names, vec!["HTTP_REQUEST", "CLIENT_DATA"]);
}

#[test]
fn irule4003_fires_across_ghost_separated_events() {
    let src = "when {HTTP_REQUEST}{ set token abc }\n\
                   when {CLIENT_DATA}{ log local0. $token }";
    assert!(
        has(src, "IRULE4003"),
        "the `}}{{` ghost separator must not hide the second handler; got {:?}",
        codes(src)
    );
}

#[test]
fn irule4003_fires_across_spaced_events_the_same_way() {
    // Control: the spaced spelling of the same two handlers, which the
    // default-config segmentation already saw.
    let src = "when {HTTP_REQUEST} { set token abc }\n\
                   when {CLIENT_DATA} { log local0. $token }";
    assert!(
        has(src, "IRULE4003"),
        "the spaced spelling must keep reporting; got {:?}",
        codes(src)
    );
}

#[test]
fn irule4003_ignores_invalid_nested_event_handler() {
    let src = "when HTTP_REQUEST {\n\
                       set token abc\n\
                       if {1} { when CLIENT_DATA { log local0. $token } }\n\
                   }";
    assert!(has(src, "IRULE5006"), "nested when must be rejected");
    assert!(
        !has(src, "IRULE4003"),
        "an invalid nested when must not manufacture a second event inventory"
    );
}

#[test]
fn nested_event_handler_retains_outer_event_context() {
    let src = "when HTTP_REQUEST { when CLIENT_ACCEPTED { HTTP::respond 200 } }";
    assert!(has(src, "IRULE5006"), "nested when must be rejected");
    assert_eq!(
        irule1001(src).len(),
        0,
        "the invalid nested body remains in HTTP_REQUEST context"
    );
}

#[test]
fn event_handler_inside_proc_does_not_manufacture_event_context() {
    let src = "proc target {} { return 1 }\n\
                   proc helper {} { when HTTP_REQUEST { target } }";
    assert!(has(src, "IRULE5006"), "when inside proc must be rejected");
    assert!(
        !has(src, "IRULE5005"),
        "an invalid nested handler is inert, so its data must not be \
             diagnosed as an executable direct proc dispatch"
    );
}

#[test]
fn var_referenced_in_matches_both_forms() {
    assert!(var_referenced_in("x", "log local0. $x"));
    assert!(var_referenced_in("x", "log local0. ${x}"));
    assert!(!var_referenced_in("x", "log local0. $xyz"));
    assert!(!var_referenced_in("x", "no dollar here"));
}

// IRULE1001 — command-event-validity.  Messages and severities below are
// pinned against the command-event-validity checker (run on the same
// snippets) and the registry legality matrix.

/// `(severity, message)` for every IRULE1001 diagnostic, whole-file.
fn irule1001(source: &str) -> Vec<(Severity, String)> {
    let mut a = Analyser::new();
    a.analyse(source, "f5-irules")
        .diagnostics
        .into_iter()
        .filter(|d| d.code == DiagCode::Irule1001)
        .map(|d| (d.severity, d.message))
        .collect()
}

#[test]
fn irule1001_quiet_for_legal_command_with_inferred_profile() {
    // HTTP_REQUEST infers the HTTP profile, so HTTP::respond is fully
    // satisfied — no diagnostic at all.
    assert_eq!(
        irule1001("when HTTP_REQUEST { HTTP::respond 200 content \"ok\" }"),
        [] as [(tcl_core_types::Severity, std::string::String); 0]
    );
}

#[test]
fn irule1001_warns_command_unmet_requirements() {
    // HTTP::respond in CLIENT_ACCEPTED: the L4 event provides no HTTP
    // profile, so it "may not work" with the missing-requirement reason.
    let diags = irule1001("when CLIENT_ACCEPTED { HTTP::respond 200 content \"ok\" }");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].0, Severity::Warning);
    assert_eq!(
        diags[0].1,
        "'HTTP::respond' may not work in CLIENT_ACCEPTED: requires profile FASTHTTP or HTTP."
    );
}

#[test]
fn irule1001_hints_unconfirmed_namespace_profile() {
    // SSL::cipher is legal in HTTP_REQUEST but assumes an SSL profile the
    // file doesn't confirm — an informational hint, OR-listed + sorted.
    let diags = irule1001("when HTTP_REQUEST { SSL::cipher name }");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].0, Severity::Hint);
    assert_eq!(
        diags[0].1,
        "'SSL::cipher' assumes profile CLIENTSSL or PERSIST or SERVERSSL or \
             SSL_PERSISTENCE on the virtual server."
    );
}

/// The two cells the appliance measurements single out as *"exactly
/// the mistakes an editor should catch"*
/// (`docs/design/f5/bigip-irule-parser-measurements.md` §8): the rule
/// compiler refuses `HTTP::uri` in `HTTP_RESPONSE` and `HTTP::status`
/// in `HTTP_REQUEST` at rule load, though both events carry an HTTP
/// profile. The registry model must refuse them too — the F5 conformance
/// corpus (`tcl_registry::f5::corpus`) carries both rows as `Agrees`.
#[test]
fn irule1001_warns_the_measured_http_asymmetries() {
    for (source, command, event) in [
        (
            "when HTTP_RESPONSE { HTTP::uri }",
            "HTTP::uri",
            "HTTP_RESPONSE",
        ),
        (
            "when HTTP_REQUEST { HTTP::status }",
            "HTTP::status",
            "HTTP_REQUEST",
        ),
    ] {
        let diags = irule1001(source);
        assert_eq!(diags.len(), 1, "{source}");
        assert_eq!(diags[0].0, Severity::Warning, "{source}");
        assert!(
            diags[0]
                .1
                .starts_with(&format!("'{command}' cannot be used in {event}.")),
            "{}",
            diags[0].1
        );
    }
}

/// The mirror image, and the reason the rule cannot be a blanket
/// tightening: the same §8 sweep measured `IP::server_addr` accepted
/// in every traffic event including the client-side ones (its own
/// documentation says it returns `0` before the serverside connection
/// exists), and `HTTP::uri`/`HTTP::status`/`HTTP::collect` accepted in
/// `LB_SELECTED`, which implies no HTTP profile at all. Warning on any of
/// these is a false positive.
#[test]
fn irule1001_quiet_for_the_measured_acceptances() {
    // `IP::server_addr` has no profile requirement at all, so these
    // are silent outright.
    for source in [
        "when CLIENT_ACCEPTED { IP::server_addr }",
        "when CLIENT_DATA { IP::server_addr }",
        "when HTTP_REQUEST { IP::server_addr }",
        "when CLIENT_CLOSED { IP::server_addr }",
    ] {
        assert_eq!(
            irule1001(source).len(),
            0,
            "{source}: {:?}",
            irule1001(source)
        );
    }
    // The `HTTP::*` trio in `LB_SELECTED` keeps the *informational*
    // half: the event implies no HTTP profile, so the file cannot
    // confirm the virtual server carries one, and the unconfirmed
    // -namespace-profile hint still fires. What must be gone is the
    // **warning** — the appliance loads all three there (§8), so claiming
    // otherwise would be a false positive.
    for source in [
        "when LB_SELECTED { HTTP::uri }",
        "when LB_SELECTED { HTTP::status }",
        "when LB_SELECTED { HTTP::collect }",
    ] {
        let diags = irule1001(source);
        assert!(
            diags
                .iter()
                .all(|(severity, _)| *severity == Severity::Hint),
            "{source}: {diags:?}"
        );
        assert!(
            diags
                .iter()
                .all(|(_, message)| message.contains("assumes profile")),
            "{source}: {diags:?}"
        );
    }
}

/// `SSL::cipher` is refused either side of a completed handshake and
/// in `RULE_INIT`, and accepted in the three events between (§8) — the
/// registry model follows the measured row rather than accepting it
/// everywhere.
#[test]
fn irule1001_follows_the_measured_ssl_cipher_row() {
    // The four handshake-adjacent events are a measured closed list,
    // so they read as an exclusion…
    for event in [
        "CLIENT_ACCEPTED",
        "CLIENT_DATA",
        "SERVER_CONNECTED",
        "CLIENT_CLOSED",
    ] {
        let source = format!("when {event} {{ SSL::cipher name }}");
        let diags = irule1001(&source);
        assert_eq!(diags.len(), 1, "{source}");
        assert_eq!(diags[0].0, Severity::Warning, "{source}");
        assert!(
            diags[0]
                .1
                .starts_with(&format!("'SSL::cipher' cannot be used in {event}.")),
            "{}",
            diags[0].1
        );
    }
    // …while `RULE_INIT` is refused for the ordinary reason, with the
    // reason spelled out. `LB::server` differs from `SSL::cipher` in
    // exactly one cell: that is the only one it is refused in.
    for command in ["SSL::cipher name", "LB::server pool"] {
        let source = format!("when RULE_INIT {{ {command} }}");
        let diags = irule1001(&source);
        assert_eq!(diags.len(), 1, "{source}");
        assert_eq!(diags[0].0, Severity::Warning, "{source}");
        assert!(
            diags[0]
                .1
                .contains("may not work in RULE_INIT: no active traffic flow"),
            "{}",
            diags[0].1
        );
    }
    assert_eq!(
        irule1001("when CLIENT_ACCEPTED { LB::server pool }"),
        [] as [(tcl_core_types::Severity, std::string::String); 0]
    );
}

#[test]
fn irule1001_warns_excluded_event_with_available_list() {
    // HA::status is excluded from RULE_INIT; the "Available in: …" list is
    // the first five (sorted) legal events plus a "(+N more)" tail.
    let diags = irule1001("when RULE_INIT { HA::status }");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].0, Severity::Warning);
    assert!(
        diags[0]
            .1
            .starts_with("'HA::status' cannot be used in RULE_INIT. Available in: "),
        "{}",
        diags[0].1
    );
    assert!(diags[0].1.contains("(+") && diags[0].1.ends_with("more)."));
}

#[test]
fn irule1001_hint_suppressed_by_profile_directive() {
    // A leading `# profiles: CLIENTSSL` directive confirms the SSL stack,
    // so SSL::cipher's profile hint is suppressed.
    assert_eq!(
        irule1001("# profiles: CLIENTSSL\nwhen HTTP_REQUEST { SSL::cipher name }").len(),
        0
    );
}

#[test]
fn irule1001_hint_suppressed_by_inferred_event_profile() {
    // A CLIENTSSL_HANDSHAKE handler in the file infers CLIENTSSL, which
    // covers SSL::cipher's requirement — hint suppressed.
    let src = "when CLIENTSSL_HANDSHAKE { log local0. hi }\nwhen HTTP_REQUEST { SSL::cipher name }";
    assert_eq!(
        irule1001(src),
        [] as [(tcl_core_types::Severity, std::string::String); 0]
    );
}

#[test]
fn irule1001_fp_map_configuration_is_not_limited_to_fix_message() {
    // FP: `FIX::tag map set` is configuration. F5 documents it as valid
    // in any event, unlike `FIX::tag get`; the form distinction is in the
    // registry, not this checker.
    assert_eq!(
        irule1001("when RULE_INIT { FIX::tag map set client_1 tag_map }"),
        [] as [(tcl_core_types::Severity, std::string::String); 0]
    );
}

#[test]
fn irule1001_tp_fix_message_read_rejects_an_unrelated_event() {
    // TP: the live-message read form has a deliberately closed event
    // surface. This also pins the generic `only_in` diagnostic wording.
    assert_eq!(
        irule1001("when RULE_INIT { FIX::tag get 49 }"),
        vec![(
            (Severity::Warning),
            "'FIX::tag' cannot be used in RULE_INIT. Available in: FIX_MESSAGE.".to_string()
        )]
    );
}

#[test]
fn irule1001_tn_fix_message_read_is_accepted_in_its_documented_event() {
    // TN: the same `get` form is legal at FIX_MESSAGE, whose event data
    // supplies the required FIX profile.
    assert_eq!(
        irule1001("when FIX_MESSAGE { FIX::tag get 49 }"),
        [] as [(tcl_core_types::Severity, std::string::String); 0]
    );
}

#[test]
fn irule1001_mqtt_payload_forms_follow_registry_event_contracts() {
    // The current-message forms are available at ingress, while a bare
    // collected-payload read is limited to the corresponding DATA event.
    for form in ["length", "replace bytes", "prepend bytes"] {
        assert_eq!(
            irule1001(&format!(
                "when MQTT_CLIENT_INGRESS {{ MQTT::payload {form} }}"
            ))
            .len(),
            0,
            "MQTT::payload {form} must be valid at ingress"
        );
    }
    assert_eq!(
            irule1001("when MQTT_CLIENT_INGRESS { set bytes [MQTT::payload] }"),
            vec![(
                Severity::Warning,
                "'MQTT::payload' cannot be used in MQTT_CLIENT_INGRESS. Available in: MQTT_CLIENT_DATA, MQTT_SERVER_DATA."
                    .to_string(),
            )]
        );
    assert_eq!(
        irule1001("when MQTT_CLIENT_DATA { set bytes [MQTT::payload] }").len(),
        0,
        "the bare collected-payload form is valid in MQTT_CLIENT_DATA"
    );
    assert_eq!(
        irule1001("when MQTT_CLIENT_INGRESS { MQTT::payload $form bytes }").len(),
        0,
        "a dynamic form must fall back to the profile requirement"
    );
}

#[test]
fn irule1001_fn_prevention_documented_examples_are_not_invalidated() {
    // Treating optional protocol consumers as mandatory requirements draws
    // false warnings here. The generic event-contract test covers every
    // registry example; these two data rows are kept as focused cases.
    assert_eq!(
        irule1001("when HTTP_REQUEST { active_members web_pool }"),
        [] as [(tcl_core_types::Severity, std::string::String); 0]
    );
    assert_eq!(
        irule1001("when HTTP_REQUEST { snat automap }"),
        [] as [(tcl_core_types::Severity, std::string::String); 0]
    );
}

#[test]
fn irule5007_rejects_all_executable_forms_at_top_level() {
    // Form resolution may refine an executable command's event contract,
    // but it cannot turn that command into a top-level declaration.
    assert!(has("FIX::tag map set client_1 tag_map", "IRULE5007"));
    assert!(has("FIX::tag get 49", "IRULE5007"));
}

#[test]
fn irule1001_quiet_at_top_level_without_event() {
    // No enclosing `when` block ⇒ no event context ⇒ the check never runs.
    assert_eq!(
        irule1001("HTTP::respond 200"),
        [] as [(tcl_core_types::Severity, std::string::String); 0]
    );
}

#[test]
fn irule1001_quiet_outside_f5_dialect() {
    let mut a = Analyser::new();
    let res = a.analyse("when CLIENT_ACCEPTED { HTTP::respond 200 }", "tcl");
    assert!(
        !res.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::Irule1001)
    );
}

#[test]
fn profile_info_description_or_lists_sorted_unconfirmed() {
    let profiles = profile_registry();
    // No requirement → None.
    assert_eq!(profile_info_description(&[], &[], profiles), None);
    // Confirmed by file → None.
    assert_eq!(
        profile_info_description(&["HTTP"], &["HTTP"], profiles),
        None
    );
    // Unconfirmed → sorted OR-list.
    assert_eq!(
        profile_info_description(&["HTTP", "FASTHTTP"], &[], profiles).as_deref(),
        Some("assumes profile FASTHTTP or HTTP on the virtual server")
    );
}

#[test]
fn missing_requirements_description_reports_the_unmet_stack_requirements() {
    let profiles = profile_registry();
    let events = event_registry();
    let server_req = EventRequires {
        client_side: false,
        server_side: true,
        transport: None,
        profiles: &[],
        also_in: &[],
        flow: false,
    };
    let props = events.get_props("CLIENT_ACCEPTED").expect("known event");
    assert_eq!(
        missing_requirements_description(props, &server_req, profiles),
        "no server-side connection"
    );
}
