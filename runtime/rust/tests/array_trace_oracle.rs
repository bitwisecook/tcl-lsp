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

//! Standalone-runtime `array` trace conformance against the pinned Tcl 9.0.4
//! interpreter and upstream `trace.test` definitions.

use std::path::{Path, PathBuf};
use std::rc::Rc;

use tcl_dialect::TclVersion;
use tcl_host_native::NativeHost;
use tcl_platform::Host;
use tcl_runtime::interp::{Code, Interp};
use tcl_test_support::{
    locate_source_tree, run_script_from_source_tree, upstream_test_definition, TclSourceTree,
};

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn tcl_9_0_4() -> Option<TclSourceTree> {
    let tree = locate_source_tree(&repository_root(), TclVersion::V9_0, None)
        .expect("locate the pinned Tcl 9.0 source tree")?;
    assert_eq!(tree.patchlevel, "9.0.4", "exact Tcl oracle pin");
    Some(tree)
}

fn run_oracle(tree: &TclSourceTree, source: &str) -> Option<String> {
    if !tree.root.join("unix/tclsh").is_file() {
        eprintln!(
            "skipping Tcl 9.0.4 oracle: {} has no built unix/tclsh",
            tree.root.display()
        );
        return None;
    }
    Some(
        run_script_from_source_tree(tree, TclVersion::V9_0, source.as_bytes())
            .expect("run Tcl 9.0.4 oracle")
            .strict_text()
            .expect("Tcl 9.0.4 oracle succeeds"),
    )
}

fn oracle_result(tree: &TclSourceTree, sheet: &str) -> Option<String> {
    run_oracle(tree, &format!("{sheet}\nputs -nonewline [set ::out]\n"))
}

fn runtime_result(sheet: &str) -> String {
    let mut interp = Interp::new();
    interp.set_runtime_version(TclVersion::V9_0);
    let code = interp.eval_str(sheet.as_bytes());
    let result = String::from_utf8(interp.result_bytes()).expect("runtime result is UTF-8");
    assert_eq!(code, Code::Ok, "runtime sheet failed: {result}\n{sheet}");
    result
}

fn runtime_tcltest_result(tree: &TclSourceTree, sheet: &str) -> Option<String> {
    let host = Rc::new(NativeHost::new());
    host.env()
        .set("TCL_LIBRARY", &tree.library_dir().to_string_lossy());
    let mut interp = Interp::with_host(host);
    interp.set_runtime_version(TclVersion::V9_0);
    assert_eq!(interp.eval_str(b"info commands if"), Code::Ok);
    if interp.result_bytes().is_empty() {
        // The deliberate no-libtommath build omits expr-driven commands such
        // as `if`; Tcl's init.tcl cannot execute in that representation tier.
        return None;
    }
    assert_eq!(
        interp.init_library(),
        Code::Ok,
        "real Tcl 9.0.4 init.tcl failed: {}",
        String::from_utf8_lossy(&interp.result_bytes())
    );
    let code = interp.eval_str(sheet.as_bytes());
    let result = String::from_utf8_lossy(&interp.result_bytes()).into_owned();
    assert_eq!(code, Code::Ok, "real tcltest run failed: {result}");
    Some(result)
}

#[test]
fn upstream_trace_1_11_through_1_14_pass_through_real_tcltest_startup() {
    let Some(tree) = tcl_9_0_4() else {
        eprintln!("skipping: Tcl 9.0.4 source tree is not installed");
        return;
    };
    let source = std::fs::read_to_string(tree.tests_dir().join("trace.test"))
        .expect("read pinned upstream trace.test");
    let definitions =
        upstream_test_definition(&source, "test trace-1.11 {", "# Basic write-tracing")
            .expect("extract upstream trace-1.11 through trace-1.14 definitions");
    let sheet = format!(
        "package require tcltest\n\
         namespace import -force ::tcltest::*\n\
         {definitions}\n\
         set ::tcl_lsp_summary [list \
             $::tcltest::numTests(Total) $::tcltest::numTests(Passed) \
             $::tcltest::numTests(Skipped) $::tcltest::numTests(Failed)]\n\
         ::tcltest::cleanupTests\n\
         set ::tcl_lsp_summary"
    );
    let oracle = run_oracle(
        &tree,
        &format!("{sheet}\nputs -nonewline $::tcl_lsp_summary"),
    );
    let Some(runtime) = runtime_tcltest_result(&tree, &sheet) else {
        eprintln!("skipping real tcltest startup: runtime has no numeric tower");
        return;
    };

    const SUMMARY: &str = "4 4 0 0";
    if let Some(oracle) = oracle {
        assert!(
            oracle.ends_with(SUMMARY),
            "unexpected Tcl 9.0.4 result: {oracle:?}"
        );
    }
    assert_eq!(
        runtime, SUMMARY,
        "unexpected standalone-runtime tcltest counters"
    );
}

#[test]
fn array_get_reads_candidates_through_the_shared_trace_contract() {
    let sheet = std::fs::read_to_string(
        repository_root().join("tests/fixtures/tcl9/array-get-read-traces.tcl"),
    )
    .expect("read shared array-get trace fixture");
    let expected = "{basic {x traced} {{whole A x read} {elem A x read}} traced {x y}} \
        {missing 0 {} {TCL READ VARNAME} 0 0} \
        {destroy 1 {can't read \"C(x)\": no such variable} {TCL READ VARNAME}} \
        {retype 1 {can't read \"D(x)\": no such element in array} {TCL READ VARNAME} scalar} \
        {boom 0 {} {TCL READ VARNAME} 1 1 1 {TCL READ VARNAME}} \
        {carried 0 after 1 1 {TCL READ VARNAME}} \
        {harddestroy 1 {can't read \"J(x)\": BOOM} {TCL READ VARNAME} 1 1 0} \
        {hardretype 1 {can't read \"JR(x)\": BOOM} {TCL READ VARNAME} 1 1 scalar} \
        {aggregate 0 {} {TCL READ VARNAME} 1 1 1 {TCL READ VARNAME}} \
        {livegroup {x X} {whole new}} {owned {11 200}} \
        {relem 0 {x NEW} 0 {x NEW}} \
        {rbase 0 {} {TCL READ VARNAME} {x NEW}} \
        {opretarget {x B}} {elemretarget {{x A} {x B}}} \
        {uplevel {x traced} {local x read}} \
        {namespace {x traced} {a x read}} \
        {calls {x traced} {x traced}}";

    assert_eq!(runtime_result(&sheet), expected);
    if let Some(oracle) = tcl_9_0_4().and_then(|tree| oracle_result(&tree, &sheet)) {
        assert_eq!(oracle, expected, "Tcl 9.0.4 array-get trace oracle");
    }
}

#[test]
fn retained_read_options_follow_completion_scopes_and_execution_trace_saves() {
    let sheet = r#"
proc BOOM {n1 n2 op} {error BOOM}
array set E {x X}
trace add variable E(x) read BOOM
set rows {}
set c [catch {array get E; set v after} m o]
lappend rows [list $c $m [dict exists $o -errorcode] [dict get $o -errorcode]]
set c [catch {array get E; list after} m o]
lappend rows [list $c $m [dict exists $o -errorcode] [dict get $o -errorcode]]
set c [catch {array get E; concat after} m o]
lappend rows [list $c $m [dict exists $o -errorcode] [dict get $o -errorcode]]
set c [catch {array get E; string cat after} m o]
lappend rows [list $c $m [dict exists $o -errorcode] [dict get $o -errorcode]]
proc LEAVE args {return -level 0 -code ok -foo callback}
trace add execution array leave LEAVE
set c [catch {array get E; list after} m o]
lappend rows [list trace $c $m [dict exists $o -foo] [dict get $o -errorcode]]
set out $rows
"#;
    let expected = "{0 after 1 {TCL READ VARNAME}} \
        {0 after 1 {TCL READ VARNAME}} \
        {0 after 1 {TCL READ VARNAME}} \
        {0 after 1 {TCL READ VARNAME}} \
        {trace 0 after 0 {TCL READ VARNAME}}";

    assert_eq!(runtime_result(sheet), expected);
    if let Some(oracle) = tcl_9_0_4().and_then(|tree| oracle_result(&tree, sheet)) {
        assert_eq!(oracle, expected, "Tcl 9.0.4 completion-state oracle");
    }
}

#[test]
#[cfg(have_tommath)]
fn control_commands_apply_the_tcl9_completion_option_scope_matrix() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/tcl9/completion-options-control.tcl");
    let mut sheet = std::fs::read_to_string(path).expect("read shared completion-options fixture");
    sheet.push_str("\nset out\n");
    let expected = "{0 1 {TCL READ VARNAME}} {0 1 {TCL READ VARNAME}} \
        {0 1 {TCL READ VARNAME}} {0 1 {TCL READ VARNAME}} \
        {0 1 {TCL READ VARNAME}} {0 0 {}} {0 0 {}} {0 0 {}} \
        {0 1 {TCL READ VARNAME}} {0 0 {}} {0 0 {}} {0 0 {}} \
        {0 0 {}} {0 0 {}} {0 0 {}}";

    assert_eq!(runtime_result(&sheet), expected);
    if let Some(oracle) = tcl_9_0_4().and_then(|tree| oracle_result(&tree, &sheet)) {
        assert_eq!(oracle, expected, "Tcl 9.0.4 completion-scope oracle");
    }
}

#[test]
fn alias_wrappers_begin_a_fresh_completion_option_scope() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/tcl9/completion-options-alias.tcl");
    let mut sheet = std::fs::read_to_string(path).expect("read shared alias-options fixture");
    sheet.push_str("\nset out\n");
    let expected = "{{list 0 inside 0} {try 0 inside 0} {eval 0 inside 0} \
        {switch 0 inside 0}} {0 inside 0} {0 value BAR}";

    assert_eq!(runtime_result(&sheet), expected);
    if let Some(oracle) = tcl_9_0_4().and_then(|tree| oracle_result(&tree, &sheet)) {
        assert_eq!(oracle, expected, "Tcl 9.0.4 alias completion-scope oracle");
    }
}

#[test]
fn interleaved_coroutines_keep_array_operation_targets_with_their_flow() {
    let sheet = r#"
proc Y {n1 n2 op} {yield $n1}
array set A {x A}
array set B {x B}
trace add variable A(x) read Y
trace add variable B(x) read Y
set ya [coroutine ca apply {{} {array get ::A}}]
set yb [coroutine cb apply {{} {array get ::B}}]
set ra [ca]
set rb [cb]
list $ya $yb $ra $rb
"#;

    assert_eq!(runtime_result(sheet), "::A ::B {x A} {x B}");
}

#[test]
fn upstream_trace_5_1_passes_through_real_tcltest_startup() {
    let Some(tree) = tcl_9_0_4() else {
        eprintln!("skipping: Tcl 9.0.4 source tree is not installed");
        return;
    };
    let source = std::fs::read_to_string(tree.tests_dir().join("trace.test"))
        .expect("read pinned upstream trace.test");
    let callback = upstream_test_definition(&source, "proc traceArray2 {", "proc traceProc {")
        .expect("extract upstream traceArray2 definition");
    let definition = upstream_test_definition(&source, "test trace-5.1 {", "test trace-5.2 {")
        .expect("extract upstream trace-5.1 definition");
    let sheet = format!(
        "package require tcltest\n\
         namespace import -force ::tcltest::*\n\
         {callback}\n\
         {definition}\n\
         set ::tcl_lsp_summary [list \
             $::tcltest::numTests(Total) $::tcltest::numTests(Passed) \
             $::tcltest::numTests(Skipped) $::tcltest::numTests(Failed)]\n\
         ::tcltest::cleanupTests\n\
         set ::tcl_lsp_summary"
    );
    let oracle = run_oracle(
        &tree,
        &format!("{sheet}\nputs -nonewline $::tcl_lsp_summary"),
    );
    let Some(runtime) = runtime_tcltest_result(&tree, &sheet) else {
        eprintln!("skipping real tcltest startup: runtime has no numeric tower");
        return;
    };

    const SUMMARY: &str = "1 1 0 0";
    if let Some(oracle) = oracle {
        assert!(
            oracle.ends_with(SUMMARY),
            "unexpected Tcl 9.0.4 result: {oracle:?}"
        );
    }
    assert_eq!(
        runtime, SUMMARY,
        "unexpected standalone-runtime tcltest counters"
    );
}

#[test]
fn validation_precedes_array_traces_at_the_same_boundaries_as_tcl() {
    let sheet = "set ::out {}\n\
        proc A {n1 n2 op} { lappend ::log [list $n1 $n2 $op] }\n\
        array set a {}\n\
        trace add variable a array A\n\
        proc probe {label script} {\n\
        \x20   set ::log {}\n\
        \x20   set code [catch {uplevel #0 $script} message]\n\
        \x20   lappend ::out [list $label $code [llength $::log] $::log]\n\
        }\n\
        probe bare {array}\n\
        probe member-only {array names}\n\
        probe names-too-many {array names a -glob * extra}\n\
        probe set-too-few {array set a}\n\
        probe set-too-many {array set a {} extra}\n\
        probe set-bad-list {array set a odd}\n\
        probe for-too-few {array for {k v} a}\n\
        probe for-bad-list {array for \\{ a {}}\n\
        probe for-bad-count {array for k a {}}\n\
        probe for-valid {array for {k v} a {}}\n\
        probe default-too-few {array default get}\n\
        probe default-too-many {array default get a x y}\n\
        probe default-bad-option {array default bogus a}\n\
        probe default-get-extra {array default get a x}\n\
        probe default-set-missing {array default set a}\n\
        probe default-prefix {array def ex a}\n\
        probe unknown {array bogus a}\n\
        set ::out";
    let expected = "{bare 1 0 {}} {member-only 1 0 {}} {names-too-many 1 0 {}} \
         {set-too-few 1 0 {}} {set-too-many 1 0 {}} \
         {set-bad-list 1 1 {{a {} array}}} {for-too-few 1 0 {}} \
         {for-bad-list 1 0 {}} {for-bad-count 1 0 {}} \
         {for-valid 0 1 {{a {} array}}} {default-too-few 1 0 {}} \
         {default-too-many 1 0 {}} {default-bad-option 1 0 {}} \
         {default-get-extra 1 1 {{a {} array}}} \
         {default-set-missing 1 1 {{a {} array}}} \
         {default-prefix 0 1 {{a {} array}}} {unknown 1 0 {}}";

    assert_eq!(runtime_result(sheet), expected);
    if let Some(oracle) = tcl_9_0_4().and_then(|tree| oracle_result(&tree, sheet)) {
        assert_eq!(oracle, expected);
    }
}

#[test]
fn validation_errors_preserve_tcl_9_structured_codes() {
    let sheet = "set ::out {}\n\
        proc probe {label script} {\n\
        \x20   catch {uplevel #0 $script} message options\n\
        \x20   lappend ::out [list $label [dict get $options -errorcode]]\n\
        }\n\
        array set a {}\n\
        probe set-bad-list {array set a odd}\n\
        probe for-bad-list {array for \\{ a {}}\n\
        probe default-bad-option {array default bogus a}\n\
        set ::out";
    let expected = "{set-bad-list {TCL ARGUMENT FORMAT}} \
         {for-bad-list {TCL VALUE LIST BRACE}} \
         {default-bad-option {TCL LOOKUP INDEX option bogus}}";

    assert_eq!(runtime_result(sheet), expected);
    if let Some(oracle) = tcl_9_0_4().and_then(|tree| oracle_result(&tree, sheet)) {
        assert_eq!(oracle, expected);
    }
}
