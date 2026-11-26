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

//! End-to-end W113, W128, W210, W214, W216, W220 and W307 cases through `analyse`.

use super::*;

#[test]
fn w113_silent_on_overridable_library_procs() {
    // FP-STY-13: the script-defined Tcl *library* procedures
    // (`unknown`, `history`, `auto_*`, `parray`, `pkg_mkIndex`,
    // `tcl_*` word helpers) are documented as user-replaceable
    // overlays, not C built-ins — redefining one must not fire
    // W113.
    let mut a = Analyser::new();
    for name in [
        "unknown",
        "history",
        "auto_execok",
        "auto_load",
        "auto_mkindex",
        "auto_qualify",
        "auto_reset",
        "parray",
        "pkg_mkIndex",
        "tcl_findLibrary",
        "tcl_wordBreakAfter",
        "tcl_endOfWord",
    ] {
        let r = a.analyse(&format!("proc {name} args {{ return }}\n"), "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == DiagCode::W113),
            "W113 should be silent on overridable library proc {name:?}",
        );
    }
    // TP control: a genuine C built-in still fires.
    let r = a.analyse("proc set {} {}\n", "tcl");
    assert!(
        r.diagnostics.iter().any(|d| d.code == DiagCode::W113),
        "W113 must still fire when redefining a C built-in (`set`)",
    );
}

#[test]
fn w307_array_set_const_element_suppression() {
    // TN: `array set state {-command puts}; $state(-command) hi` — the
    // literal element value `puts` is a known command, so the callback-key
    // heuristic must not fire W307. Harvested from the `array set` literal.
    let mut a = Analyser::new();
    let has_w307 = a
        .analyse(
            "proc f {} { array set state {-command puts}; $state(-command) hi }\n",
            "tcl8.6",
        )
        .diagnostics
        .iter()
        .any(|d| d.code == DiagCode::W307);
    assert!(
        !has_w307,
        "array-element holding a known command must not fire W307",
    );
    // TP control: a non-command literal value still fires W307.
    let mut b = Analyser::new();
    let has_w307 = b
        .analyse(
            "proc f {} { array set state {-command notACommand}; $state(-command) hi }\n",
            "tcl8.6",
        )
        .diagnostics
        .iter()
        .any(|d| d.code == DiagCode::W307);
    assert!(
        has_w307,
        "array-element holding a non-command must still fire W307",
    );
}

/// Helper: the RBS-family diagnostic codes (W210 read-before-set, W213
/// unset-no-complain, W214 unused-param) emitted for `src`.
fn rbs_codes(src: &str) -> Vec<String> {
    Analyser::new()
        .analyse(src, "tcl8.6")
        .diagnostics
        .iter()
        .filter(|d| matches!(d.code.as_str(), "W210" | "W213" | "W214"))
        .map(|d| d.code.to_string())
        .collect()
}

#[test]
fn w220_must_alias_kill_same_array_element() {
    // FP-DS-06: two writes to the *same* literal-key array element with no
    // intervening read of it make the first dead — the later-version read
    // `$a(k)` reads the second write, not the first.  Must-alias kill
    // overrides the element-observed suppression.
    let mut a = Analyser::new();
    assert!(
        a.analyse(
            "proc f {} {\n    set a(k) 1\n    set a(k) 2\n    return $a(k)\n}",
            "tcl8.6"
        )
        .diagnostics
        .iter()
        .any(|d| d.code == DiagCode::W220 || d.code == DiagCode::O109),
        "two writes to a(k) with no intervening read: first is dead",
    );
    // Control: different keys are independent — no dead store.
    let mut b = Analyser::new();
    assert!(
        !b.analyse(
            "proc f {} {\n    set a(k) 1\n    set a(j) 2\n    return $a(k)\n}",
            "tcl8.6"
        )
        .diagnostics
        .iter()
        .any(|d| d.code == DiagCode::W220 || d.code == DiagCode::O109),
        "writes to different array elements are not dead stores",
    );
    // Control: an intervening read of the same element cancels the kill.
    let mut c = Analyser::new();
    assert!(
        !c.analyse(
            "proc f {} {\n    set a(k) 1\n    puts $a(k)\n    set a(k) 2\n    return $a(k)\n}",
            "tcl8.6",
        )
        .diagnostics
        .iter()
        .any(|d| d.code == DiagCode::W220 || d.code == DiagCode::O109),
        "an intervening read of a(k) keeps the first write live",
    );
}

#[test]
fn dead_store_recovers_rmw_read_in_command_sub() {
    // FP-DS-01: `[incr i $j]` buried in a substitution reads `i`'s prior
    // value, so the feeding `set i 0` is alive — no dead-store (W220/O109)
    // or unused (W211/O126).
    let mut a = Analyser::new();
    let codes: Vec<_> = a
            .analyse(
                "proc f {} {\n    set i 0\n    foreach j {1 2 3} { lappend r [incr i $j] }\n    return $r\n}\n",
                "tcl",
            )
            .diagnostics
            .iter()
            .filter(|d| matches!(d.code.as_str(), "W220" | "W211" | "O109" | "O126"))
            .map(|d| d.code.to_string())
            .collect();
    assert_eq!(
        codes.len(),
        0,
        "RMW read in cmd-sub keeps set i 0 alive: {codes:?}"
    );
    // TP control: no read-modify-write of `i` — the first assignment is
    // truly dead, so a dead-store hint must still fire.
    let mut b = Analyser::new();
    assert!(
        b.analyse("proc f {} { set i 0\n set i 5\n return $i }", "tcl")
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W220 || d.code == DiagCode::O109),
        "a genuine dead store must still fire",
    );
}

#[test]
fn w210_dynamic_target_upvar_alias_read_is_silent() {
    // `upvar 1 $varName local` aliases `local` to the caller
    // variable named by `$varName` — the standard Tcl pass-by-reference
    // idiom. Reading `$local` is not read-before-set: it errors only when
    // the *caller* variable is missing, exactly the runtime condition that
    // would make the *literal*-target twin (`upvar 1 caller local`) error
    // too. tclsh 8.6/9.0 confirm the two forms are semantically identical,
    // so the analyser treats them alike — both silent.
    let codes = rbs_codes("proc foo {varName} {\n  upvar 1 $varName local\n  puts $local\n}\n");
    assert!(
        !codes.iter().any(|c| c == "W210"),
        "dynamic-target upvar alias read must be silent: {codes:?}"
    );
    // The literal-target twin is silent too.
    let codes = rbs_codes("proc foo {} {\n  upvar 1 caller local\n  puts $local\n}\n");
    assert!(
        !codes.iter().any(|c| c == "W210"),
        "literal-target upvar: {codes:?}"
    );
    // TP control: an *unrelated* local — not the alias, never set — is not a
    // scope alias, so it still fires W210.
    let codes = rbs_codes("proc foo {varName} {\n  upvar 1 $varName local\n  puts $unrelated\n}\n");
    assert!(
        codes.iter().any(|c| c == "W210"),
        "unrelated non-alias local must still fire W210: {codes:?}"
    );
    // An `[info exists local]` guard suppresses the read (always did).
    let codes = rbs_codes(
        "proc foo {varName} {\n  upvar 1 $varName local\n  if {[info exists local]} { puts $local }\n}\n",
    );
    assert!(
        !codes.iter().any(|c| c == "W210"),
        "guarded dynamic upvar: {codes:?}"
    );
    // A *write* through the alias is observable (goes to the caller var),
    // so it is not a dead store even with a dynamic target.
    let mut a = Analyser::new();
    assert!(
        !a.analyse(
            "proc foo {v} {\n  upvar 1 $v local\n  set local 5\n}\n",
            "tcl8.6"
        )
        .diagnostics
        .iter()
        .any(|d| d.code == DiagCode::W220),
        "write through a dynamic upvar alias is observable, not a dead store",
    );
}

#[test]
fn w210_gets_in_while_condition_writes_loop_var() {
    // FP-RBS-03: `[gets $fp line]` in the while condition writes `line`;
    // the body's `$line` reads must not fire W210.
    let codes = rbs_codes(
        "proc f {fp} {\n  while {[gets $fp line] >= 0} {\n    set n [string length $line]\n    puts \"$line ($n chars)\"\n  }\n}\n",
    );
    assert_eq!(codes.len(), 0, "gets-in-condition writes line: {codes:?}");
}

#[test]
fn w210_catch_in_expr_writes_result_var() {
    // FP-RBS-06: `[catch {…} tmp]` inside `[expr {…}]` writes `tmp`; the
    // same-expression `|| $tmp` read must not fire W210.
    let codes = rbs_codes(
        "proc f {sock} {\n  set eof [expr {[catch {eof $sock} tmp] || $tmp}]\n  return $eof\n}\n",
    );
    assert_eq!(codes.len(), 0, "catch-in-expr writes tmp: {codes:?}");
}

#[test]
fn w214_namespace_eval_body_does_not_recover_caller_param() {
    // FP-RBS-10: a `namespace eval ::ns {…}` body runs in the namespace
    // frame, so `$x` there is NOT a read of the caller's parameter `x` —
    // W214 (unused param) must still fire.  The `eval {…}` form runs in
    // the caller frame, so its `$x` read DOES recover the param (no W214).
    let mut a = Analyser::new();
    assert!(
        a.analyse(
            "proc g {x} {\n  namespace eval ::ns { puts \"hello $x\" }\n}\n",
            "tcl8.6"
        )
        .diagnostics
        .iter()
        .any(|d| d.code == DiagCode::W214),
        "namespace-eval body must not recover the caller's param read",
    );
    let mut b = Analyser::new();
    assert!(
        !b.analyse("proc f {x} {\n  eval { puts $x }\n}\n", "tcl8.6")
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W214),
        "eval body runs in the caller frame; $x read recovers the param",
    );
}

#[test]
fn w307_in_method_body_dispatch_fires_and_suppresses() {
    // TclOO method bodies are now walked in a `Method` scope, so their
    // `[cmd] method` dispatch sites are recorded for W307.
    // TP: `[format notACommand] run` returns a string, not an object.
    let mut a = Analyser::new();
    let fires = a
        .analyse(
            "oo::class create C { method m {} { [format notACommand] run } }",
            "tcl8.6",
        )
        .diagnostics
        .iter()
        .any(|d| d.code == DiagCode::W307);
    assert!(fires, "in-method [format ...] dispatch must fire W307");

    // Control: `[D new] run` where D is a known class returns an Object —
    // suppressed (and `run` resolves on D, so no W308 either).
    let mut b = Analyser::new();
    let fires = b
        .analyse(
            "oo::class create D { method run {} { return ok } }\n\
                 oo::class create C { method m {} { [D new] run } }\n",
            "tcl8.6",
        )
        .diagnostics
        .iter()
        .any(|d| d.code == DiagCode::W307);
    assert!(
        !fires,
        "in-method [D new] dispatch on known class must not fire W307"
    );
}

#[test]
fn w307_my_method_literal_vs_object_return() {
    // TP: `[my plain] run` where `plain` returns a literal — the return is
    // a plain string, so the outer dispatch fires W307.
    let mut a = Analyser::new();
    let fires = a
        .analyse(
            "oo::class create C { method plain {} { return notACommand }\n\
                 method m {} { [my plain] run } }",
            "tcl",
        )
        .diagnostics
        .iter()
        .any(|d| d.code == DiagCode::W307);
    assert!(fires, "[my plain] returning a literal must fire W307");

    // Control: `[my obj] run` where `obj` returns `[D new]` — object
    // handle, suppressed.
    let mut b = Analyser::new();
    let fires = b
        .analyse(
            "oo::class create D { method run {} { return ok } }\n\
                 oo::class create C { method obj {} { return [D new] }\n\
                 method m {} { [my obj] run } }",
            "tcl",
        )
        .diagnostics
        .iter()
        .any(|d| d.code == DiagCode::W307);
    assert!(!fires, "[my obj] returning an object must not fire W307");
}

#[test]
fn method_body_params_and_instance_vars_not_flagged() {
    // Walking method bodies must not false-fire read-before-set / unused
    // on the method's formal parameters or the class's instance variables.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create C {\n\
             variable count\n\
             method add {n} { incr count $n; return $count }\n\
             }\n",
        "tcl",
    );
    let offenders: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| matches!(d.code.as_str(), "W210" | "W214"))
        .map(|d| d.code.to_string())
        .collect();
    assert_eq!(
        offenders.len(),
        0,
        "method params / instance vars must not fire W210/W214: {offenders:?}",
    );
}

#[test]
fn w307_namespaced_ensemble_composed_name_resolution() {
    // TN: `${ns}::dowork` where `ns` is the const `mypkg` and
    // `::mypkg::dowork` is a known proc — the composed name resolves, so
    // no W307.
    let mut a = Analyser::new();
    let has = a
        .analyse(
            "namespace eval ::mypkg { proc dowork {arg} {} }\n\
                 proc f {} { set ns mypkg; ${ns}::dowork arg }\n",
            "tcl",
        )
        .diagnostics
        .iter()
        .any(|d| d.code == DiagCode::W307);
    assert!(!has, "resolvable namespaced ensemble must not fire W307");
    // TP control: composed name with no known proc still fires.
    let mut b = Analyser::new();
    let has = b
        .analyse(
            "proc f {} { set ns mypkg; ${ns}::unknownproc arg }\n",
            "tcl",
        )
        .diagnostics
        .iter()
        .any(|d| d.code == DiagCode::W307);
    assert!(
        has,
        "unresolvable composed namespaced ensemble must fire W307"
    );
}

#[test]
fn w307_interproc_dict_with_const_element_suppression() {
    // TN: `dict with d { $cmd hi }` where the caller passes `{cmd puts}` —
    // the dict-with unpacks `cmd` to the known command `puts`, so the
    // body's `$cmd hi` dispatch must not fire W307.  The literal is
    // harvested from `d`'s call-site-propagated v0 SCCP const.
    let mut a = Analyser::new();
    let has_w307 = a
        .analyse(
            "proc f {d} { dict with d { $cmd hi } }\nf {cmd puts}\n",
            "tcl8.6",
        )
        .diagnostics
        .iter()
        .any(|d| d.code == DiagCode::W307);
    assert!(
        !has_w307,
        "dict-with-unpacked known command must not fire W307"
    );
    // TP control: a non-command unpacked value still fires.
    let mut b = Analyser::new();
    let has_w307 = b
        .analyse(
            "proc f {d} { dict with d { $cmd hi } }\nf {cmd notACommand}\n",
            "tcl8.6",
        )
        .diagnostics
        .iter()
        .any(|d| d.code == DiagCode::W307);
    assert!(
        has_w307,
        "dict-with-unpacked non-command must still fire W307"
    );
}

#[test]
fn w307_known_class_new_var_suppression() {
    // TN: `set x [C new]; $x run` where C is a known oo::class — `x` holds
    // an Object of class C, so the `$x run` dispatch resolves through the
    // method check (run exists on C) instead of firing W307.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create C { method run {} { return ok } }\n\
             proc f {} { set x [C new]; $x run }\n",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W307),
        "var assigned from a known-class constructor must not fire W307: {:?}",
        r.diagnostics,
    );
}

#[test]
fn w216_brace_then_paren_name_vs_value_positions() {
    // FP-STY-12: the braced indirect-array idiom `${name}(idx)` in a
    // *variable-name* position (`set`/`unset`/`incr`/`append`/`lappend`/
    // `vwait`/`info exists`) is the legitimate "array name held in a
    // scalar" access — neither W216 nor W212 fires.
    let mut a = Analyser::new();
    for src in [
        "set token ::http::1\nset ${token}(status) eof\n",
        "set token ::http::1\ninfo exists ${token}(-pipeline)\n",
        "set tok ::http::1\nunset ${tok}(socketcoro)\n",
        "set token ::http::1\nvwait ${token}(status)\n",
        "set arr ::state\nincr ${arr}(n)\n",
        "set arr ::state\nappend ${arr}(buf) x\n",
        "set arr ::state\nlappend ${arr}(list) item\n",
    ] {
        let r = a.analyse(src, "tcl");
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W216 || d.code == DiagCode::W212),
            "indirect-array idiom must not fire W216/W212: {src:?} -> {:?}",
            r.diagnostics,
        );
    }
    // TP control: in a *value* position `${arr}(x)` is a broken read for
    // `$arr(x)` — W216 still fires.
    for src in ["puts ${arr}(x)\n", "set y ${arr}(x)\n"] {
        let r = a.analyse(src, "tcl");
        assert!(
            r.diagnostics.iter().any(|d| d.code == DiagCode::W216),
            "value-position ${{arr}}(x) must fire W216: {src:?} -> {:?}",
            r.diagnostics,
        );
    }
    // TP control: bare `set $x` / `set ${x}` (no `(idx)` suffix) is the
    // dynamic-name foot-gun, not the indirect idiom — W212 still fires.
    for src in ["set $x v\n", "set ${x} v\n"] {
        let r = a.analyse(src, "tcl");
        assert!(
            r.diagnostics.iter().any(|d| d.code == DiagCode::W212),
            "bare dynamic-name must still fire W212: {src:?} -> {:?}",
            r.diagnostics,
        );
    }
}

#[test]
fn call_by_name_var_not_flagged_unused_or_dead() {
    // A caller-local passed *by name* to a proc that upvar-writes
    // its param (`fill tag`) must not be flagged W211 (unused) or
    // W220 (dead store).
    let mut a = Analyser::new();
    let r = a.analyse(
        "proc ::fill {vn} { upvar 1 $vn v\nset v 1 }\n\
             proc ::f {} { set tag init\nset tag x\nfill tag }\n",
        "tcl8.6",
    );
    assert!(
        r.diagnostics.iter().all(|d| {
            !((d.code == DiagCode::W211 || d.code == DiagCode::W220) && d.message.contains("tag"))
        }),
        "call-by-name var `tag` must not be flagged unused/dead, got {:?}",
        r.diagnostics,
    );

    // Negative control: a genuinely unused local is still flagged
    // (the suppression didn't disable W211 wholesale).
    let mut a2 = Analyser::new();
    let r2 = a2.analyse("proc ::g {} { set unused 1\nputs hi }\n", "tcl8.6");
    assert!(
        r2.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W211 && d.message.contains("unused")),
        "a genuinely unused var should still be flagged, got {:?}",
        r2.diagnostics,
    );
}

#[test]
fn dynamic_name_write_does_not_dead_store_the_name_var() {
    // `set $p 1` writes the variable *named by* `$p` and *reads* `p`
    // (verified against tclsh 8.4–9.0).  It must not produce a spurious
    // W220 dead-store on the parameter `p` itself.
    for body in ["set $p 1", "set ${p} 1", "set $p [expr {1}]"] {
        let mut a = Analyser::new();
        let r = a.analyse(&format!("proc ::f {{p}} {{ {body} }}\n"), "tcl");
        assert!(
            r.diagnostics
                .iter()
                .all(|d| !(matches!(d.code.as_str(), "W211" | "W220" | "W214")
                    && d.message.contains("'p'"))),
            "dynamic-name write `{body}` wrongly flagged `p`, got {:?}",
            r.diagnostics,
        );
    }
}

#[test]
fn dynamic_name_out_var_does_not_suppress_caller_dead_store() {
    // A param used only as a *dynamic name* out-var (`scan`/`lassign`/
    // `regexp`/`regsub` target, or `set $p`) names a callee-local
    // variable — it does NOT write back to the caller's frame, so a
    // caller's literal arg is genuinely dead and must still fire W211 /
    // W220.
    let cases = [
        "proc maybe {target} { scan 42 %d $target }",
        "proc maybe {target} { lassign {1} $target }",
        "proc maybe {target} { regexp {(.)} a -> $target }",
        "proc maybe {target} { regsub a a b $target }",
        "proc maybe {target} { set $target 1 }",
    ];
    for callee in cases {
        let src = format!("{callee}\nproc caller {{}} {{ set x 1; maybe x }}\n");
        let mut a = Analyser::new();
        let r = a.analyse(&src, "tcl");
        assert!(
            r.diagnostics
                .iter()
                .any(|d| (d.code == DiagCode::W211 || d.code == DiagCode::W220)
                    && d.message.contains("'x'")),
            "caller's dead `x` must still fire for `{callee}`, got {:?}",
            r.diagnostics,
        );
    }
}

#[test]
fn upvar_writeback_still_suppresses_caller_dead_store() {
    // Control: a genuine `upvar`-write-back callee DOES consume the
    // caller's variable, so the dead-store is correctly suppressed.
    let mut a = Analyser::new();
    let r = a.analyse(
        "proc fill {outvar} { upvar $outvar out\nset out 42 }\n\
             proc top {} { set result {}\nfill result\nreturn $result }\n",
        "tcl",
    );
    assert!(
        r.diagnostics
            .iter()
            .all(|d| !((d.code == DiagCode::W211 || d.code == DiagCode::W220)
                && d.message.contains("result"))),
        "upvar write-back must suppress the caller dead-store, got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_w128_fires_on_call_to_renamed_command() {
    // A builtin renamed/deleted away earlier in the file → the
    // later call falls through to `unknown` → W128.
    let mut a = Analyser::new();
    let r = a.analyse("rename string {}\nstring toupper x\n", "tcl");
    let w128: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W128)
        .collect();
    assert_eq!(w128.len(), 1, "expected one W128, got {:?}", r.diagnostics);

    // Negative: a normal builtin call with no rename → no W128.
    let mut a2 = Analyser::new();
    let r2 = a2.analyse("string toupper x\n", "tcl");
    assert!(
        r2.diagnostics.iter().all(|d| d.code != DiagCode::W128),
        "unexpected W128: {:?}",
        r2.diagnostics,
    );

    // Negative: an ordinary unknown external command (never rebound)
    // resolves opaque but must not fire W128.
    let mut a3 = Analyser::new();
    let r3 = a3.analyse("someunknowncmd a b\n", "tcl");
    assert!(
        r3.diagnostics.iter().all(|d| d.code != DiagCode::W128),
        "unexpected W128: {:?}",
        r3.diagnostics,
    );
}

#[test]
fn w214_skips_parameters_a_channel_protocol_fixes() {
    // The host appends the accepted channel, address and port, and a
    // refchan call's method, channel and arguments; the handler cannot drop
    // them, so only a parameter outside those words is reported.
    for (src, expected) in [
        ("proc acc {s a p} {puts $s}\nsocket -server acc 8080\n", 0),
        (
            "proc rc {cmd id args} {puts $cmd}\nset c [chan create read rc]\n",
            0,
        ),
        (
            "proc xf {cmd id args} {puts $cmd}\nchan push stdout xf\n",
            0,
        ),
        (
            "proc done {n {err {}}} {puts $n}\nfcopy stdin stdout -command done\n",
            0,
        ),
        ("proc acc {s a p} {puts $s}\nacc 1 2 3\n", 2),
        (
            "proc acc {first s a p} {puts $first}\nsocket -server {acc 1} 8080\n",
            0,
        ),
        (
            "proc acc {first s a p extra} {puts $first}\nsocket -server {acc 1} 8080\n",
            1,
        ),
    ] {
        let unused = rbs_codes(src).iter().filter(|code| *code == "W214").count();
        assert_eq!(unused, expected, "{src}");
    }
}
