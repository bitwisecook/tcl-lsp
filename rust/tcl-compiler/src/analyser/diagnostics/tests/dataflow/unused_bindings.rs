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

//! W211 unused variables and W214 unused parameters.

use super::*;

#[test]
fn w211_deliberately_skips_destructuring_writer_outputs() {
    // Policy pin: a command-output variable the script
    // never reads (`binary scan … rest`, `regexp … m`) is how Tcl spells
    // "ignore the remainder" — no W211.  A plain `set` of an unread
    // variable is the TP control.
    let src = "proc f {d} {
    binary scan $d H2H* type rest
    return $type
}
";
    let codes = codes_for_dialect(src, "tcl8.6");
    assert!(
        !codes.contains(&"W211".to_string()),
        "unread destructuring output must not draw W211: {codes:?}"
    );

    let src_tp = "proc g {} {
    set unread 1
    return 2
}
";
    let codes_tp = codes_for_dialect(src_tp, "tcl8.6");
    assert!(
        codes_tp.contains(&"W211".to_string()),
        "a plain unread set still draws W211: {codes_tp:?}"
    );
}

#[test]
fn body_references_param_bare_dollar() {
    assert!(body_references_param(
        "set y $x",
        "x",
        tcl_lexer::LexerConfig::default()
    ));
    assert!(body_references_param(
        "return [expr {$a + $b}]",
        "a",
        tcl_lexer::LexerConfig::default()
    ));
    assert!(body_references_param(
        "return [expr {$a + $b}]",
        "b",
        tcl_lexer::LexerConfig::default()
    ));
    assert!(body_references_param(
        "puts [list $val 1]",
        "val",
        tcl_lexer::LexerConfig::default()
    ));
}

#[test]
fn body_references_param_braced_dollar() {
    assert!(body_references_param(
        "set y ${x}",
        "x",
        tcl_lexer::LexerConfig::default()
    ));
    assert!(body_references_param(
        "puts \"got ${val}!\"",
        "val",
        tcl_lexer::LexerConfig::default()
    ));
}

#[test]
fn body_references_param_no_match_for_substring_only() {
    // ``$abc`` must not match ``ab`` (boundary check).
    assert!(!body_references_param(
        "set y $abc",
        "ab",
        tcl_lexer::LexerConfig::default()
    ));
    assert!(!body_references_param(
        "puts $foobar",
        "foo",
        tcl_lexer::LexerConfig::default()
    ));
}

#[test]
fn body_references_param_skips_backslash_escape() {
    // ``\$x`` is a literal dollar — not a substitution.
    assert!(!body_references_param(
        "puts \\$x",
        "x",
        tcl_lexer::LexerConfig::default()
    ));
}

#[test]
fn body_references_param_handles_multiple_uses() {
    assert!(body_references_param(
        "set y $x; set z $x",
        "x",
        tcl_lexer::LexerConfig::default()
    ));
}

#[test]
fn body_references_param_misses_when_unused() {
    assert!(!body_references_param(
        "puts hello",
        "x",
        tcl_lexer::LexerConfig::default()
    ));
    assert!(!body_references_param(
        "return 42",
        "y",
        tcl_lexer::LexerConfig::default()
    ));
}

#[test]
fn body_references_param_braced_with_punct_after() {
    // ``${x}foo`` is a valid substitution — boundary not
    // required inside braces.
    assert!(body_references_param(
        "set y ${x}foo",
        "x",
        tcl_lexer::LexerConfig::default()
    ));
}

#[test]
fn body_references_param_namespace_qualified() {
    // ``$ns::var`` is a qualified variable; the param name
    // is the leading identifier.  Boundary on ``::`` is
    // OK — both are part of the qualified name; the W214
    // emitter passes the bare param so this is a non-issue
    // in practice.  Test pins the boundary semantics.
    assert!(!body_references_param(
        "set y $ns::var",
        "ns",
        tcl_lexer::LexerConfig::default()
    ));
}

#[test]
fn emit_cfg_ssa_diagnostics_w214_unused_param() {
    // ``proc foo {x y} { puts $x }`` — parameter ``y`` is
    // declared but never read in the body.  W214 should
    // fire on it.  Parameter ``x`` is read, so no W214.
    let mut a = Analyser::new();
    a.emit_document_cfg_ssa_diagnostics_for_test("proc foo {x y} { puts $x }");
    let w214s: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W214)
        .collect();
    assert_eq!(
        w214s.len(),
        1,
        "expected exactly one W214 for unused param ``y``; got {:?}",
        a.result.diagnostics,
    );
    assert!(w214s[0].message.contains("'y'"));
    assert!(w214s[0].message.contains("'::foo'"));
    assert_eq!(w214s[0].severity, Severity::Hint);
}

#[test]
fn emit_cfg_ssa_diagnostics_w211_unused_variable() {
    // ``proc foo {} { set y 1 }`` — y is set, never read,
    // and there's no other version → W211 fires.
    // Top-level test would be subject to global-scope
    // assumptions, so use a proc body where the local-only
    // verdict is safe.
    let mut a = Analyser::new();
    a.emit_document_cfg_ssa_diagnostics_for_test("proc foo {} { set y 1 }");
    let w211s: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W211)
        .collect();
    assert!(
        !w211s.is_empty(),
        "W211 expected for unused var ``y`` in proc foo; got {:?}",
        a.result.diagnostics,
    );
    assert!(w211s[0].message.contains("'y'"));
    assert!(w211s[0].message.contains("set but never used"));
    assert_eq!(w211s[0].severity, Severity::Hint);
}

#[test]
fn w211_not_emitted_for_command_output_vars() {
    // `scan` / `binary scan` / `regexp -> capture` write their targets
    // via the command, not a pure `set`; Statement::Call defs are excluded
    // from W211, so unused command outputs do not fire it.
    for src in [
        "proc f {} { scan $in \"%d\" n }",
        "proc f {} { binary scan $d cu4 b1 b2 b3 b4 }",
        "proc f {} { regexp {(\\w+)} $s -> word }",
    ] {
        let mut a = Analyser::new();
        let res = a.analyse(src, "tcl");
        assert!(
            !res.diagnostics.iter().any(|d| d.code == DiagCode::W211),
            "W211 must not fire for command-output vars; got {:?} for {src:?}",
            res.diagnostics,
        );
    }
}

#[test]
fn w211_fires_once_per_variable_set_twice() {
    // A variable set twice and never read is one unused variable, reported once
    // (W211) at the earliest definition. The dead store at that same earliest
    // assignment does NOT also fire W220 — the co-located double-emit is
    // deduped in favour of the more informative W211 — but the *distinct*
    // second dead store (`set x 2`) still fires its own W220.
    let mut a = Analyser::new();
    let res = a.analyse("proc f {} { set x 1\nset x 2 }", "tcl");
    let w211: Vec<_> = res
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W211)
        .collect();
    assert_eq!(w211.len(), 1, "expected one W211 for x; got {w211:?}");
    let w220: Vec<_> = res
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W220)
        .collect();
    assert_eq!(
        w220.len(),
        1,
        "the co-located W220 is deduped; only the distinct dead store remains; got {:?}",
        res.diagnostics,
    );
    // The surviving W211 and W220 anchor at different assignments.
    assert_ne!(w211[0].span, w220[0].span, "W211 and W220 must not overlap");
}

#[test]
fn w220_deduped_against_w211_on_single_dead_assignment() {
    // `set x 1` on a never-used variable is one dead assignment: it must emit
    // exactly one hint (W211 "never used"), not both W211 and a co-located
    // W220 ("never read"). The W220 is deduped in favour of W211.
    let mut a = Analyser::new();
    let res = a.analyse("proc f {} { set x 1 }", "tcl");
    let codes: Vec<_> = res
        .diagnostics
        .iter()
        .filter(|d| matches!(d.code, DiagCode::W211 | DiagCode::W220))
        .map(|d| d.code)
        .collect();
    assert_eq!(
        codes,
        vec![DiagCode::W211],
        "single dead assignment must yield only W211; got {:?}",
        res.diagnostics,
    );
}

#[test]
fn emit_cfg_ssa_diagnostics_w211_w220_skipped_for_traced_var() {
    // A write trace makes `x` observable on every `set`, so neither
    // W211 (unused) nor W220 (dead store) may fire.  Both the 8.5+
    // `trace add variable` and 8.4 `trace variable` spellings count.
    for src in [
        "proc f {} { trace add variable x write cb; set x 1 }",
        "proc f {} { trace variable x w cb; set x 1 }",
    ] {
        let mut a = Analyser::new();
        a.emit_document_cfg_ssa_diagnostics_for_test(src);
        assert!(
            !a.result
                .diagnostics
                .iter()
                .any(|d| matches!(d.code.as_str(), "W211" | "W220")),
            "traced var must not fire W211/W220 for {src:?}; got {:?}",
            a.result.diagnostics,
        );
    }
}

#[test]
fn emit_cfg_ssa_diagnostics_w211_skipped_for_textually_referenced() {
    // ``proc foo {} { set msg hello; puts "got $msg" }`` —
    // ``msg`` is referenced inside a quoted string; the
    // textual-reference filter should suppress W211 because
    // the def-use builder doesn't track ``"$msg"`` reads.
    let mut a = Analyser::new();
    a.emit_document_cfg_ssa_diagnostics_for_test(
        "proc foo {} { set msg hello; puts \"got $msg\" }",
    );
    let w211s: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W211 && d.message.contains("'msg'"))
        .collect();
    assert_eq!(
        w211s.len(),
        0,
        "W211 must not fire on var referenced via $-interpolation; got {:?}",
        a.result.diagnostics,
    );
}

#[test]
fn emit_cfg_ssa_diagnostics_w211_skipped_for_global_aliased() {
    // ``proc foo {} { global config; set config 1 }`` —
    // ``config`` is global-aliased; the write goes to the
    // outer scope, so the local "no use" verdict is unsafe.
    // W211 must not fire.
    let mut a = Analyser::new();
    a.emit_document_cfg_ssa_diagnostics_for_test("proc foo {} { global config; set config 1 }");
    let w211s: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W211 && d.message.contains("'config'"))
        .collect();
    assert_eq!(
        w211s.len(),
        0,
        "W211 must not fire on global-aliased var; got {:?}",
        a.result.diagnostics,
    );
}

#[test]
fn w214_empty_body_stub_silent() {
    // FP-STY-08: `proc stub {a b} {}` is a signature placeholder — no
    // W214 on its necessarily-unused params.
    let mut a = Analyser::new();
    let r = a.analyse("proc stub {a b} {}", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W214),
        "W214 must be suppressed on an empty-body stub; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w214_quoted_keyword_marker_silent() {
    // FP-STY-08: a param named `"as"` is a snit-style keyword marker.
    let mut a = Analyser::new();
    let r = a.analyse("proc xyz {\"as\" v} { return $v }", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W214),
        "W214 must not fire on a quoted-keyword param; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w214_dispatch_protocol_suppresses_peer_family() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    // ≥3 peers sharing `{ctx token}` + an arity-compatible dispatcher
    // (`$cmd $ctx $token`, 2 args) — `token` is a protocol contract.
    let src = "namespace eval ::n {\n\
                   proc a {ctx token} { puts $ctx }\n\
                   proc b {ctx token} { puts $ctx }\n\
                   proc c {ctx token} { puts $ctx }\n\
                   proc dispatch {cmd ctx token} { $cmd $ctx $token }\n\
                   }\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W214 && d.message.contains("'token'")),
        "dispatch-protocol family must suppress W214 on protocol params; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w214_repeated_publication_does_not_count_as_three_protocol_peers() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    let source = "namespace eval n {
\
        proc peer {ctx token} {puts $ctx}
\
        proc peer {ctx token} {puts $ctx}
\
        proc peer {ctx token} {puts $ctx}
\
        proc dispatch {cmd ctx token} {$cmd $ctx $token}
\
    }";
    let result = Analyser::new().analyse(source, "tcl");
    assert!(
        result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == DiagCode::W214 && diagnostic.message.contains("'token'")
        }),
        "one repeated publication does not establish three peers: {:?}",
        result.diagnostics,
    );
}

#[test]
fn w214_dispatcher_in_an_independent_source_namespace_does_not_suppress_peers() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    let source = "namespace eval n {
\
        proc a {ctx token} {puts $ctx}
\
        proc b {ctx token} {puts $ctx}
\
        proc c {ctx token} {puts $ctx}
\
    }
\
    namespace eval other {
\
        proc dispatch {cmd ctx token} {$cmd $ctx $token}
\
    }";
    let result = Analyser::new().analyse(source, "tcl");
    assert!(
        result
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.code == DiagCode::W214 && diagnostic.message.contains("'token'")
            })
            .count()
            >= 3,
        "independent dispatcher namespace must retain each unused peer parameter: {:?}",
        result.diagnostics,
    );
}

#[test]
fn w214_no_dispatcher_still_fires_on_peer_family() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    // TP control: 3 peers sharing `{ctx token}` but NO dispatcher — the
    // shared shape is coincidence, so `token` still fires.
    let src = "namespace eval ::n {\n\
                   proc a {ctx token} { puts $ctx }\n\
                   proc b {ctx token} { puts $ctx }\n\
                   proc c {ctx token} { puts $ctx }\n\
                   }\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W214 && d.message.contains("'token'")),
        "without a dispatcher, an unused protocol-shaped param still fires; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w214_genuine_unused_param_still_fires() {
    // TP control: a normal unused param in a non-empty body still fires.
    let mut a = Analyser::new();
    let r = a.analyse("proc f {a b} { return $a }", "tcl");
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W214 && d.message.contains("'b'")),
        "W214 must still fire on a genuine unused param; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn emit_cfg_ssa_diagnostics_w214_skips_args_param() {
    // The variadic ``args`` is conventional and frequently
    // declared without use; W214 must not fire on it.
    let mut a = Analyser::new();
    a.emit_document_cfg_ssa_diagnostics_for_test("proc foo {x args} { puts $x }");
    assert!(
        !a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W214),
        "W214 should not fire on ``args``; got {:?}",
        a.result.diagnostics,
    );
}

/// #2268: a computed-key existence read may observe any element, so the
/// element it may observe is used; a literal key observes only its own
/// element, so another element set and never read is still unused. tclsh
/// 8.4 to 9.1 print `yes` for the first program and `no` for the second.
#[test]
fn w211_reads_an_existence_test_by_the_element_it_names() {
    let w211 = |source: &str| -> Vec<String> {
        let diagnostics = Analyser::new().analyse(source, "tcl8.6").diagnostics;
        assert!(
            diagnostics.iter().all(|d| d.code != DiagCode::W210),
            "an existence test reads nothing it can miss: {source}\n{diagnostics:?}"
        );
        diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W211)
            .map(|d| d.message.clone())
            .collect()
    };
    let computed =
        "proc p {k} { set a(x) 1; if {[info exists a($k)]} {puts yes} else {puts no} }\np x\n";
    assert_eq!(w211(computed), [] as [String; 0], "{computed}");
    let literal =
        "proc p {k} { set a(x) 1; if {[info exists a(y)]} {puts yes} else {puts no} }\np x\n";
    assert_eq!(
        w211(literal),
        ["Variable 'a(x)' is set but never used"],
        "{literal}"
    );
}
