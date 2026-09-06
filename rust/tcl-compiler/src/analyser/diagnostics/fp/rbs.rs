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

//! RBS family — read-before-set / unused param / unused var (W210/W213/W214).

use super::{D, codes, fires};

// FP-RBS-01 — info exists / array exists is the test-before-use idiom

const FP_RBS_01_REPRO: &str = "\
proc maybe_get {} {
    # v is never set in this proc — the info-exists guard is the entire
    # safety: a bare `$v` here would be a hard tclsh error.
    if {[info exists v]} { return $v }
    return {}
}
";

#[test]
fn fp_rbs_01_info_exists_guard_silent() {
    // FP-RBS-01: a bare $v read inside an `info exists v` guard must NOT fire W210.
    assert!(
        !fires(FP_RBS_01_REPRO, D, "W210"),
        "FP-RBS-01: info-exists-guarded read must NOT fire W210; emitted: {:?}",
        codes(FP_RBS_01_REPRO, D)
    );
    assert!(
        !fires(FP_RBS_01_REPRO, D, "W213"),
        "FP-RBS-01: info-exists-guarded read must NOT fire W213; emitted: {:?}",
        codes(FP_RBS_01_REPRO, D)
    );
    assert!(
        !fires(FP_RBS_01_REPRO, D, "W214"),
        "FP-RBS-01: info-exists-guarded read must NOT fire W214; emitted: {:?}",
        codes(FP_RBS_01_REPRO, D)
    );
}

#[test]
fn fp_rbs_01_rooted_info_exists_guard_silent() {
    let src = "proc maybe_get {} { if {[::info exists v]} { return $v }; return {} }";
    assert!(
        !fires(src, D, "W210"),
        "rooted info-exists guard must suppress W210; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_01_bare_unguarded_read_still_fires() {
    // TP control: stripping the info-exists guard restores the W210.
    let src = "\
proc maybe_get {} {
    # No guard — bare `$v` on a never-set local IS a real tclsh error.
    return $v
}
";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-01 TP: unguarded bare $v read must fire W210; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_01_array_exists_guard_silent() {
    // FP-RBS-01: `array exists arr` guards an array the same way.
    let src = "\
proc maybe_get {} {
    if {[array exists arr]} { return [array get arr] }
    return {}
}
";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-01: array-exists-guarded read must NOT fire W210; emitted: {:?}",
        codes(src, D)
    );
    assert!(
        !fires(src, D, "W213"),
        "FP-RBS-01: array-exists-guarded read must NOT fire W213; emitted: {:?}",
        codes(src, D)
    );
    assert!(
        !fires(src, D, "W214"),
        "FP-RBS-01: array-exists-guarded read must NOT fire W214; emitted: {:?}",
        codes(src, D)
    );
}

// FP-RBS-02 — catch / regexp / scan command-sub writes are not read-before-set

const FP_RBS_02_REPRO: &str = "\
proc f {} {
    # [catch …] writes 'err' in this scope (tclsh-verified);
    # the read in the consequent must NOT be W210.
    if {[catch {operation} err]} { puts \"failed: $err\" }
}
";

#[test]
fn fp_rbs_02_catch_msg_var_silent() {
    // FP-RBS-02: $err read after `[catch {…} err]` must NOT fire W210.
    assert!(
        !fires(FP_RBS_02_REPRO, D, "W210"),
        "FP-RBS-02: catch-msg-var read must NOT fire W210; emitted: {:?}",
        codes(FP_RBS_02_REPRO, D)
    );
    assert!(
        !fires(FP_RBS_02_REPRO, D, "W213"),
        "FP-RBS-02: catch-msg-var read must NOT fire W213; emitted: {:?}",
        codes(FP_RBS_02_REPRO, D)
    );
}

#[test]
fn fp_rbs_02_unrelated_read_still_fires() {
    // TP control: an unrelated $other (no cmd-sub write target) must still fire W210.
    let src = "\
proc f {} {
    if {[catch {operation} err]} { puts \"failed: $other\" }
}
";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-02 TP: $other (not written by the catch) must still fire W210; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_02_regexp_match_var_silent() {
    // FP-RBS-02: regexp's -> match-vars are caller-scope writes too.
    let src = r#"
proc f {} {
    if {[regexp {(\w+)=(\w+)} "k=v" -> k v]} { puts "$k=$v" }
}
"#;
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-02: regexp match-var read must NOT fire W210; emitted: {:?}",
        codes(src, D)
    );
    assert!(
        !fires(src, D, "W213"),
        "FP-RBS-02: regexp match-var read must NOT fire W213; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_02_regexp_provably_no_match_fires_w210() {
    // TP: regexp {x} y — pattern provably does not match, $v is unset.
    let src = "\
proc f {} {
    regexp {x} y -> v
    puts $v
}
";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-02 TP: regexp no-match path read of $v must fire W210; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_02_regexp_with_options_does_not_fire() {
    // FP guard: regexp with switches must not fire (no-match proof cannot apply).
    for src in [
        "proc f {} { regexp -nocase {(x)} X -> v\n puts $v }",
        "proc f {} { regexp -lineanchor {^(x)} {a\nx} -> v\n puts $v }",
        "proc f {} { regexp -expanded {(x) # comment} x -> v\n puts $v }",
    ] {
        assert!(
            !fires(src, D, "W210"),
            "FP-RBS-02: regexp with options must NOT fire W210: {:?}; emitted: {:?}",
            src,
            codes(src, D)
        );
    }
}

#[test]
fn fp_rbs_02_regexp_no_match_reaches_dominated_block_fires() {
    // TP: regexp no-match followed by use in a DOMINATED block fires W210.
    let src = "proc f {} { regexp {x} y -> v\n if {1} { puts $v } }";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-02 TP: reached-by-dominator W210 must fire; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_02_regexp_in_negated_condition_fires() {
    // TP: regexp embedded in `if {![regexp ...]}` — body executes ONLY on no-match.
    let src = "proc f {} { if {![regexp {x} y -> v]} { puts $v } }";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-02 TP: no-match branch use must fire W210; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_02_regexp_in_positive_condition_success_branch_silent() {
    // FP: `if {[regexp ...]}` — body executes ONLY on match, v IS set.
    let src = "proc f {} { if {[regexp {x} y -> v]} { puts $v } }";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-02: success-branch use must NOT fire W210; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_02_regexp_nocase_silent_on_literal_match() {
    // FP: regexp -nocase {x} X — matches (case-insensitive), v is set.
    let src = "proc f {} { regexp -nocase {x} X v; puts $v }";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-02: -nocase literal match must NOT fire W210; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_02_regexp_unknown_switch_bails() {
    // FP / safety: estimator must bail on any unrecognised regexp switch.
    for opt in ["-bogus", "-about"] {
        let src = format!("proc f {{}} {{ regexp {opt} {{x}} X v; puts $v }}");
        assert!(
            !fires(&src, D, "W210"),
            "FP-RBS-02: unknown switch {opt} must conservatively NOT fire W210; emitted: {:?}",
            codes(&src, D)
        );
    }
}

#[test]
fn fp_rbs_02_regexp_safe_switch_still_fires() {
    // TP: switches that don't change match semantics must not inhibit the no-match verdict.
    for opt in [
        "-line",
        "-lineanchor",
        "-linestop",
        "-expanded",
        "-indices",
        "-inline",
        "-all",
    ] {
        let src = format!("proc f {{}} {{ regexp {opt} {{x}} X v; puts $v }}");
        assert!(
            fires(&src, D, "W210"),
            "FP-RBS-02 TP: {opt} must still allow W210 to fire; emitted: {:?}",
            codes(&src, D)
        );
    }
}

#[test]
fn fp_rbs_02_scan_provably_no_match_fires_w210() {
    // TP: scan abc %d n — format %d requires digit but input starts with 'a'; n is unset.
    let src = "\
proc f {} {
    scan abc %d n
    puts $n
}
";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-02 TP: scan no-match path read of $n must fire W210; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_02_scan_output_silent() {
    // FP: scan's output-vars are caller-scope writes too.
    let src = "\
proc f {} {
    scan \"42\" %d n
    puts $n
}
";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-02: scan output-var read must NOT fire W210; emitted: {:?}",
        codes(src, D)
    );
    assert!(
        !fires(src, D, "W213"),
        "FP-RBS-02: scan output-var read must NOT fire W213; emitted: {:?}",
        codes(src, D)
    );
}

// test_FP_dict_with_does_not_suppress_unrelated_missing_var
// (not tagged FP-RBS-NN; belongs to the dict-with precision gap documented inline)
#[test]
fn fp_rbs_dict_with_does_not_suppress_unrelated_missing_var() {
    // TP: empty literal dict; dict with unpacks no keys; $missing read is a real W210.
    let src = "\
proc f {} {
    set d {}
    dict with d {}
    puts $missing
}
";
    assert!(
        fires(src, D, "W210"),
        "dict-with on empty dict must NOT suppress unrelated $missing; emitted: {:?}",
        codes(src, D)
    );
}

// test_FP_dispatch_protocol_requires_dispatcher_evidence
// (not tagged FP-RBS-NN; inline precision gap)
#[test]
fn fp_rbs_dispatch_protocol_requires_dispatcher_evidence() {
    // TP: three helpers sharing {ctx token} with no dispatcher evidence — token unused, W214 fires.
    let src = "\
namespace eval ::n {
    proc a {ctx token} { puts $ctx }
    proc b {ctx token} { puts $ctx }
    proc c {ctx token} { puts $ctx }
}
";
    assert!(
        fires(src, D, "W214"),
        "unused 'token' in 3 peer helpers (no dispatcher evidence) must fire W214; emitted: {:?}",
        codes(src, D)
    );
}

// test_FP_call_by_name_info_exists_dynamic_target_not_caller_read
// (not tagged FP-RBS-NN; inline precision gap)
#[test]
fn fp_rbs_call_by_name_info_exists_dynamic_target_not_caller_read() {
    // TP: callee uses target as DYNAMIC_NAME_LOCAL; caller's $x must still fire W211/W220.
    let src = "\
proc maybe {target} {
    info exists $target
}
proc caller {} {
    set x 1
    maybe x
}
";
    let cs = codes(src, D);
    let fires_any = cs
        .iter()
        .any(|c| c == "W211" || c == "W220" || c == "O126" || c == "O109");
    assert!(
        fires_any,
        "caller's $x set-but-never-used must fire W211/W220/O126/O109; emitted: {cs:?}"
    );
}

// FP-RBS-03 — frozen-loop bodies (while/for with cmd-sub condition)

const FP_RBS_03_REPRO: &str = "\
proc f {fp} {
    # gets writes 'line' AND the body sets 'n' — both are body-local
    # but the frozen-loop body keeps them invisible to SSA defs.
    while {[gets $fp line] >= 0} {
        set n [string length $line]
        puts \"$line ($n chars)\"
    }
}
";

#[test]
fn fp_rbs_03_frozen_while_body_silent() {
    // FP-RBS-03: body writes (n, and line itself via cmd-sub) must not false-flag W210.
    assert!(
        !fires(FP_RBS_03_REPRO, D, "W210"),
        "FP-RBS-03: frozen-while-body reads must NOT fire W210; emitted: {:?}",
        codes(FP_RBS_03_REPRO, D)
    );
    assert!(
        !fires(FP_RBS_03_REPRO, D, "W213"),
        "FP-RBS-03: frozen-while-body reads must NOT fire W213; emitted: {:?}",
        codes(FP_RBS_03_REPRO, D)
    );
}

#[test]
fn fp_rbs_03_genuine_unset_in_body_still_fires() {
    // TP control: $missing (never written anywhere) must still fire W210.
    let src = "\
proc f {fp} {
    while {[gets $fp line] >= 0} {
        puts \"$line $missing\"
    }
}
";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-03 TP: $missing (never written anywhere) must fire W210; emitted: {:?}",
        codes(src, D)
    );
}

// FP-RBS-04 — qualified-variable aliases (local name is the tail)

const FP_RBS_04_REPRO: &str = "\
proc ::ns::get {name key} {
    # `variable ${name}::graphAttr` declares the local alias 'graphAttr';
    # the qualified form is just where the storage lives.
    variable ${name}::graphAttr
    if {![info exists graphAttr($key)]} { return \"\" }
    return $graphAttr($key)
}
";

#[test]
fn fp_rbs_04_qualified_alias_silent() {
    // FP-RBS-04: `variable ${name}::tail` declares local alias — reads must not false-flag.
    assert!(
        !fires(FP_RBS_04_REPRO, D, "W210"),
        "FP-RBS-04: qualified-alias tail read must NOT fire W210; emitted: {:?}",
        codes(FP_RBS_04_REPRO, D)
    );
    assert!(
        !fires(FP_RBS_04_REPRO, D, "W213"),
        "FP-RBS-04: qualified-alias tail read must NOT fire W213; emitted: {:?}",
        codes(FP_RBS_04_REPRO, D)
    );
}

#[test]
fn fp_rbs_04_static_qualified_alias_silent() {
    // FP: static-namespace form `variable ::ns::tail` also creates a tail-named local.
    let src = "\
proc tester {} {
    variable ::ns::children
    return $children
}
";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-04: static-namespace alias tail must NOT fire W210; emitted: {:?}",
        codes(src, D)
    );
    assert!(
        !fires(src, D, "W213"),
        "FP-RBS-04: static-namespace alias tail must NOT fire W213; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_04_unrelated_tail_still_fires() {
    // TP control: a read of a tail name NOT declared as qualified alias must still fire W210.
    let src = "\
proc ::ns::get {name key} {
    variable ${name}::graphAttr
    # 'undeclared' has no alias decl and no set — must still fire W210.
    return $undeclared($key)
}
";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-04 TP: $undeclared (no alias decl, no set) must fire W210; emitted: {:?}",
        codes(src, D)
    );
}

// FP-RBS-05 — namespace upvar alias-not-a-def

const FP_RBS_05_REPRO: &str = "\
proc tester {} {
    # tclsh: 'alias' is now the caller-scope name for ::ns::state.
    namespace upvar ::ns state alias
    return $alias
}
";

#[test]
fn fp_rbs_05_namespace_upvar_silent() {
    // FP-RBS-05: `namespace upvar ns src alias` is a real caller-scope def of `alias`.
    assert!(
        !fires(FP_RBS_05_REPRO, D, "W210"),
        "FP-RBS-05: namespace-upvar alias read must NOT fire W210; emitted: {:?}",
        codes(FP_RBS_05_REPRO, D)
    );
    assert!(
        !fires(FP_RBS_05_REPRO, D, "W213"),
        "FP-RBS-05: namespace-upvar alias read must NOT fire W213; emitted: {:?}",
        codes(FP_RBS_05_REPRO, D)
    );
}

// `namespace upvar`'s otherVar may be an ARRAY ELEMENT.  tclsh 8.6 (and
// tclNamesp.c `NamespaceUpvarCmd`: the source resolves through
// `TclObjLookupVarEx(..., createPart1=1, createPart2=1)`, so an `(index)`
// suffix is parsed — and created — exactly as for `upvar`; `TclPtrMakeUpvar`
// then links the scalar local).  Verified live:
//   % namespace eval ns {variable arr; array set arr {k 1}}
//   % namespace upvar ns arr(k) local; puts $local        → 1
//   % proc t {} {namespace upvar ns arr(new) l; set l 7}
//   % t; puts $ns::arr(new)                                → 7
const FP_RBS_05_ELEM_REPRO: &str = "\
proc tester {} {
    # tclsh: 'local' is now the caller-scope name for ::ns::arr(k).
    namespace upvar ::ns arr(k) local
    return $local
}
";

#[test]
fn fp_rbs_05_namespace_upvar_array_element_silent() {
    // FP-RBS-05: an array-element source is as real a link as a scalar one.
    assert!(
        !fires(FP_RBS_05_ELEM_REPRO, D, "W210"),
        "FP-RBS-05: element-source alias read must NOT fire W210; emitted: {:?}",
        codes(FP_RBS_05_ELEM_REPRO, D)
    );
    assert!(
        !fires(FP_RBS_05_ELEM_REPRO, D, "W213"),
        "FP-RBS-05: element-source alias read must NOT fire W213; emitted: {:?}",
        codes(FP_RBS_05_ELEM_REPRO, D)
    );
}

#[test]
fn fp_rbs_05_array_element_alias_write_only_silent() {
    // Adjacent FP guard: a write through the element-source alias lands in
    // ::ns::arr(k) — observable outside the frame (tclsh: `set l 7` above
    // creates/overwrites `ns::arr(new)`) — so it is neither a dead store
    // (W220) nor an unused variable (W211).
    let src = "proc t {} {\n    namespace upvar ::ns arr(k) local\n    set local 5\n}\n";
    assert!(
        !fires(src, D, "W220"),
        "FP-RBS-05: write through an element-source alias must NOT fire W220; emitted: {:?}",
        codes(src, D)
    );
    assert!(
        !fires(src, D, "W211"),
        "FP-RBS-05: write through an element-source alias must NOT fire W211; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_05_array_element_unrelated_read_still_fires() {
    // TP control: the silence above comes from the alias binding of `local`,
    // not from a blanket namespace-upvar suppression — an unrelated,
    // never-written `$other` in the same proc must still fire W210.
    let src = "proc tester {} {\n    namespace upvar ::ns arr(k) local\n    return $other\n}\n";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-05 TP: $other (no alias, no set) must fire W210; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_05_alias_exists_guard_is_not_constant() {
    // The `::safe::CheckInterp` guard shape (safe.tcl:109): whether an alias
    // local exists tracks the linked namespace variable — runtime state — so
    // `[info exists alias]` must not fold to a constant branch (I230).
    // tclsh 8.6: `namespace upvar ns s a; info exists a` → 1 when `ns::s`
    // exists, 0 when it does not.  Pre-fix the fold said "always false"
    // (the lowered `namespace upvar` Call carries no defs, unlike
    // `global`/`variable`/`upvar`).  Scalar and array-element sources both.
    for src in [
        "proc t {} {\n    namespace upvar ::ns state alias\n    if {[info exists alias]} { puts yes }\n}\n",
        "proc t {} {\n    namespace upvar ::ns arr(k) alias\n    if {[info exists alias]} { puts yes }\n}\n",
    ] {
        assert!(
            !fires(src, D, "I230"),
            "FP-RBS-05: alias existence guard must NOT fold to I230; emitted: {:?}",
            codes(src, D)
        );
    }
}

// FP-RBS-06 — cmd-sub write-targets inside an expr body are caller-scope writes

const FP_RBS_06_REPRO: &str = "\
proc f {sock} {
    # http.tcl:4340 pattern: the [catch …] inside [expr {…}] writes
    # 'tmp' during expr eval; the `|| $tmp` read must not be W210.
    set eof [expr {[catch {eof $sock} tmp] || $tmp}]
    return $eof
}
";

#[test]
fn fp_rbs_06_catch_inside_expr_silent() {
    // FP-RBS-06: `[expr {[catch {…} tmp] || $tmp}]` writes `tmp` during expr eval.
    assert!(
        !fires(FP_RBS_06_REPRO, D, "W210"),
        "FP-RBS-06: cmd-sub write inside expr must NOT fire W210; emitted: {:?}",
        codes(FP_RBS_06_REPRO, D)
    );
    assert!(
        !fires(FP_RBS_06_REPRO, D, "W213"),
        "FP-RBS-06: cmd-sub write inside expr must NOT fire W213; emitted: {:?}",
        codes(FP_RBS_06_REPRO, D)
    );
}

#[test]
fn fp_rbs_06_unrelated_inside_expr_still_fires() {
    // TP control: unrelated $other inside the same expr must still fire W210.
    let src = "\
proc f {sock} {
    set eof [expr {[catch {eof $sock} tmp] || $other}]
    return $eof
}
";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-06 TP: $other (not written by catch inside expr) must fire W210; emitted: {:?}",
        codes(src, D)
    );
}

// FP-RBS-07 — dynamically-named namespace eval bodies are still analysed

const FP_RBS_07_REPRO: &str = "\
# logger.tcl:1007-1016 pattern: ${service} is the enclosing proc's
# parameter; the dynamic namespace name doesn't stop the body's inner
# `proc greet` from being analysed (post-fix).
proc trace_on {service} {
    namespace eval ::logger::tree::${service} {
        proc greet {who} { return \"hello $who\" }
    }
}
";

#[test]
fn fp_rbs_07_dynamic_ns_eval_inner_param_silent() {
    // FP-RBS-07: dynamic-name namespace eval body is analysed; inner param read is silent.
    assert!(
        !fires(FP_RBS_07_REPRO, D, "W210"),
        "FP-RBS-07: dynamic-name ns-eval inner proc-param read must NOT fire W210; emitted: {:?}",
        codes(FP_RBS_07_REPRO, D)
    );
    assert!(
        !fires(FP_RBS_07_REPRO, D, "W213"),
        "FP-RBS-07: dynamic-name ns-eval inner proc-param read must NOT fire W213; emitted: {:?}",
        codes(FP_RBS_07_REPRO, D)
    );
    assert!(
        !fires(FP_RBS_07_REPRO, D, "W214"),
        "FP-RBS-07: dynamic-name ns-eval inner proc-param must NOT fire W214; emitted: {:?}",
        codes(FP_RBS_07_REPRO, D)
    );
}

#[test]
fn fp_rbs_07_static_ns_eval_still_works() {
    // FP control: static-namespace form was never broken — assert it stays silent.
    let src = "\
namespace eval ::pkg {
    proc greet {who} { return \"hello $who\" }
}
";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-07: static ns-eval inner proc-param read must NOT fire W210; emitted: {:?}",
        codes(src, D)
    );
    assert!(
        !fires(src, D, "W214"),
        "FP-RBS-07: static ns-eval inner proc-param must NOT fire W214; emitted: {:?}",
        codes(src, D)
    );
}

// FP-RBS-08 — upvar with a dynamic target (upvar 1 $name var) is a real alias-def

const FP_RBS_08_REPRO: &str = "\
proc f {name} {
    # picoirc.tcl:69 pattern: upvar 1 $context irc — aliases 'irc'
    # to whatever the caller named.  Writes + reads must be silent.
    upvar 1 $name var
    set var 99
    return $var
}
";

#[test]
fn fp_rbs_08_dynamic_upvar_target_silent() {
    // FP-RBS-08: `upvar 1 $name var` is a real alias-def even with dynamic target.
    let cs = codes(FP_RBS_08_REPRO, D);
    let rbs_or_ds = cs
        .iter()
        .any(|c| matches!(c.as_str(), "W210" | "W213" | "W214" | "W220" | "W211"));
    assert!(
        !rbs_or_ds,
        "FP-RBS-08: dynamic-target upvar alias use must NOT fire any RBS/dead-store/unused code; emitted: {cs:?}"
    );
}

#[test]
fn fp_rbs_08_unrelated_local_still_fires() {
    // TP control: an unrelated local (not aliased, not set) must still fire W210.
    let src = "\
proc f {name} {
    upvar 1 $name var
    return $unrelated
}
";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-08 TP: $unrelated (no alias, no set) must fire W210; emitted: {:?}",
        codes(src, D)
    );
}

// FP-RBS-09 — for-init / regexp captures in un-lowered switch arms

const FP_RBS_09_REPRO: &str = r#"
proc f {n} {
    switch -- $n {
        a {
            for {set j 0} {$j < 3} {incr j} { puts $j }
        }
        b {
            if {[regexp {(\w+)} "foo" -> v]} { puts $v }
        }
    }
}
"#;

#[test]
fn fp_rbs_09_for_init_in_switch_arm_silent() {
    // FP-RBS-09: `for {set j 0} …` inside a switch arm — for-init def is recovered.
    assert!(
        !fires(FP_RBS_09_REPRO, D, "W210"),
        "FP-RBS-09: for-init $j read inside switch arm must NOT fire W210; emitted: {:?}",
        codes(FP_RBS_09_REPRO, D)
    );
    assert!(
        !fires(FP_RBS_09_REPRO, D, "W213"),
        "FP-RBS-09: for-init $j read inside switch arm must NOT fire W213; emitted: {:?}",
        codes(FP_RBS_09_REPRO, D)
    );
}

#[test]
fn fp_rbs_09_regexp_capture_in_switch_arm_silent() {
    // FP-RBS-09: `regexp … -> v` capture inside a switch arm — capture-var def is recovered.
    // Note: tested via the same reproducer; guard on 'v' specifically.
    // Fires are already covered above (W210 must not fire for j).
    // Here we check the 'v' path (which is about regexp capture, not for-init).
    // Since both are in the same reproducer, the W210-silent check covers both.
    assert!(
        !fires(FP_RBS_09_REPRO, D, "W210"),
        "FP-RBS-09: regexp capture $v read inside switch arm must NOT fire W210; emitted: {:?}",
        codes(FP_RBS_09_REPRO, D)
    );
}

#[test]
fn fp_rbs_09_genuine_unset_in_arm_still_fires() {
    // TP control: $missing (never written anywhere) still fires W210 even inside a switch arm.
    let src = r"
proc f {n} {
    switch -- $n {
        a {
            for {set j 0} {$j < 3} {incr j} { puts $missing }
        }
    }
}
";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-09 TP: $missing (never written anywhere) must fire W210; emitted: {:?}",
        codes(src, D)
    );
}

// FP-RBS-10 — eval / namespace eval literal-body reads are recovered

const FP_RBS_10_REPRO: &str = "\
proc f {x} {
    # eval's braced body evaluates in *this* scope: $x is a real read of
    # the parameter, so 'x' must not be reported W214 (\"unused\").
    eval { puts $x }
}
";

#[test]
fn fp_rbs_10_eval_body_param_silent() {
    // FP-RBS-10: `eval { … $x … }` evaluates in this scope; $x is a real use of param x.
    assert!(
        !fires(FP_RBS_10_REPRO, D, "W210"),
        "FP-RBS-10: eval-body $x read must NOT fire W210; emitted: {:?}",
        codes(FP_RBS_10_REPRO, D)
    );
    assert!(
        !fires(FP_RBS_10_REPRO, D, "W213"),
        "FP-RBS-10: eval-body $x read must NOT fire W213; emitted: {:?}",
        codes(FP_RBS_10_REPRO, D)
    );
    assert!(
        !fires(FP_RBS_10_REPRO, D, "W214"),
        "FP-RBS-10: eval-body $x param must NOT fire W214; emitted: {:?}",
        codes(FP_RBS_10_REPRO, D)
    );
}

#[test]
fn fp_rbs_10_genuine_unused_still_fires() {
    // TP control: a parameter that's truly never read still fires W214.
    let src = "\
proc f {x y} {
    return $x
}
";
    assert!(
        fires(src, D, "W214"),
        "FP-RBS-10 TP: truly unused param 'y' must fire W214; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_10_namespace_eval_does_not_recover_reads() {
    // TP: `namespace eval ns { ... $x ... }` runs in ns frame, NOT caller's; W214 fires.
    let src = "\
proc g {x} {
    namespace eval ::ns { puts \"hello $x\" }
}
";
    assert!(
        fires(src, D, "W214"),
        "FP-RBS-10 TP: namespace-eval body must NOT recover caller reads; W214 must fire on x; emitted: {:?}",
        codes(src, D)
    );
}

// FP-RBS-11 — qualified-builtin loops (::foreach / ::lmap / ::for / ::while)

const FP_RBS_11_REPRO: &str = "\
proc f {dict} {
    # html.tcl:153 pattern: ::foreach is just qualified foreach.
    # Loop vars k,v and body reads must all be silent.
    ::foreach {k v} $dict { puts \"$k=$v\" }
}
";

#[test]
fn fp_rbs_11_qualified_foreach_silent() {
    // FP-RBS-11: ::foreach is recognised by loop-form recovery, so loop vars k/v don't false-flag.
    assert!(
        !fires(FP_RBS_11_REPRO, D, "W210"),
        "FP-RBS-11: qualified ::foreach must NOT fire W210; emitted: {:?}",
        codes(FP_RBS_11_REPRO, D)
    );
    assert!(
        !fires(FP_RBS_11_REPRO, D, "W213"),
        "FP-RBS-11: qualified ::foreach must NOT fire W213; emitted: {:?}",
        codes(FP_RBS_11_REPRO, D)
    );
}

#[test]
fn fp_rbs_11_qualified_for_silent() {
    // FP control: the qualified ::for form works the same way.
    let src = "\
proc f {} {
    ::for {set i 0} {$i < 3} {incr i} { puts $i }
}
";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-11: qualified ::for must NOT fire W210; emitted: {:?}",
        codes(src, D)
    );
    assert!(
        !fires(src, D, "W213"),
        "FP-RBS-11: qualified ::for must NOT fire W213; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_11_genuine_unset_in_body_still_fires() {
    // TP control: $missing (never bound anywhere) must still fire W210.
    let src = "\
proc f {dict} {
    ::foreach {k v} $dict { puts \"$k=$missing\" }
}
";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-11 TP: $missing (never bound) must still fire W210; emitted: {:?}",
        codes(src, D)
    );
}

// FP-RBS-12 — regexp/scan output-var conditional defs reach both reviewer cases (D1-4)

const FP_RBS_12_REPRO: &str = "proc f {} { regexp {x} y -> v; if {1} { puts $v } }\n";

#[test]
fn fp_rbs_12_regexp_unconditional_read_after_no_match_fires() {
    // TP (reviewer case A): regexp {x} vs literal y is no-match; if {1} always runs.
    assert!(
        fires(FP_RBS_12_REPRO, D, "W210"),
        "FP-RBS-12 TP: post-regexp unconditional read of $v must fire W210; emitted: {:?}",
        codes(FP_RBS_12_REPRO, D)
    );
}

#[test]
fn fp_rbs_12_regexp_in_negated_if_arm_fires() {
    // TP (reviewer case B): if-arm executes when regexp returns 0 (no match); $v unset.
    let src = "proc f {} { if {![regexp {x} y -> v]} { puts $v } }\n";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-12 TP: regexp output var in no-match arm must fire W210; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_12_regexp_match_arm_read_silent() {
    // TN control: `if {[regexp {x} y -> v]} { puts $v }` — read only on MATCH arm where v IS set.
    let src = "proc f {} { if {[regexp {x} y -> v]} { puts $v } }\n";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-12: regexp match-arm read of $v must NOT fire W210; emitted: {:?}",
        codes(src, D)
    );
}

// FP-RBS-13 — `tailcall` replaces the frame; code after it never runs

const FP_RBS_13_REPRO: &str = "\
proc f {cond} {
    # tailcall g replaces this frame: the `return $result` below is only
    # reached via the else branch, where result is always set.
    if {$cond} {
        tailcall g
    } else {
        set result 1
    }
    return $result
}
";

#[test]
fn fp_rbs_13_tailcall_with_args_silent() {
    // FP-RBS-13: `tailcall g` ends the proc's straight-line flow; return $result is safe.
    assert!(
        !fires(FP_RBS_13_REPRO, D, "W210"),
        "FP-RBS-13: tailcall-terminated branch must NOT fire W210; emitted: {:?}",
        codes(FP_RBS_13_REPRO, D)
    );
    assert!(
        !fires(FP_RBS_13_REPRO, D, "W213"),
        "FP-RBS-13: tailcall-terminated branch must NOT fire W213; emitted: {:?}",
        codes(FP_RBS_13_REPRO, D)
    );
}

#[test]
fn fp_rbs_13_bare_tailcall_silent() {
    // FP: bare `tailcall` (no args) is also a terminator.
    let src = "proc f {cond} {\n    if {$cond} { tailcall } else { set result 1 }\n    return $result\n}\n";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-13: bare tailcall is a terminator; must NOT fire W210; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_13_non_terminating_branch_still_fires() {
    // TP control: replace tailcall with a normal command; result is maybe-unset.
    let src = "proc f {cond} {\n    if {$cond} { puts hi } else { set result 1 }\n    return $result\n}\n";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-13 TP: non-terminating then-branch leaves 'result' maybe-unset; W210 must fire; emitted: {:?}",
        codes(src, D)
    );
}

// FP-RBS-14 — opaque-switch arm that can't complete normally is excluded from must-define

const FP_RBS_14_REPRO: &str = "\
proc f {x} {
    # the a* arm returns, so it never reaches `puts $y`; the only path that
    # does (default) sets y -> y is definitely defined.
    switch -glob $x {
        a* { return 0 }
        default { set y 2 }
    }
    puts $y
}
";

#[test]
fn fp_rbs_14_returning_arm_silent() {
    // FP-RBS-14: the `a*` arm returns; every path reaching `puts $y` assigns y.
    assert!(
        !fires(FP_RBS_14_REPRO, D, "W210"),
        "FP-RBS-14: returning switch arm must NOT drop 'y' from must-define; emitted: {:?}",
        codes(FP_RBS_14_REPRO, D)
    );
}

#[test]
fn fp_rbs_14_erroring_arm_silent() {
    // FP: an arm that always errors likewise cannot complete normally.
    let src = "proc f {x} {\n    switch -glob $x { a* { error bad } default { set y 2 } }\n    puts $y\n}\n";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-14: erroring switch arm must NOT drop 'y' from must-define; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_14_omitting_arm_still_fires() {
    // TP control: an arm that completes without assigning y leaves it maybe-unset.
    let src = "proc f {x} {\n    switch -glob $x { a* { set z 9 } default { set y 2 } }\n    puts $y\n}\n";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-14 TP: omitting arm leaves 'y' maybe-unset; W210 must fire; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_14_break_arm_escaping_loop_still_fires() {
    // TP (Codex P1): break in opaque-switch arm does NOT define var on the path escaping the loop.
    let src =
        "proc f {} { foreach x {a} { switch -glob $x {a* {break} default {set y 1}} }; puts $y }\n";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-14 TP: break-arm escaping the loop leaves 'y' unset; W210 must fire; emitted: {:?}",
        codes(src, D)
    );
}

// FP-RBS-15 — opaque switch whose every arm exits is itself a terminator

const FP_RBS_15_REPRO: &str = "\
proc f {x} {
    # every arm returns, so control never reaches `puts $y`:
    # the switch is a terminator and the read is dead code.
    switch -glob $x {
        a* { return 1 }
        default { return 2 }
    }
    puts $y
}
";

#[test]
fn fp_rbs_15_all_arms_return_dead_read_silent() {
    // FP-RBS-15: every arm returns; `puts $y` is unreachable dead code — no W210.
    assert!(
        !fires(FP_RBS_15_REPRO, D, "W210"),
        "FP-RBS-15: all-arms-return switch makes trailing read unreachable; must NOT fire W210; emitted: {:?}",
        codes(FP_RBS_15_REPRO, D)
    );
}

#[test]
fn fp_rbs_15_all_arms_error_or_tailcall_silent() {
    // FP: error and tailcall arms are non-completing too; all-exiting switch is still a terminator.
    let src = "proc f {x} {\n    switch -glob $x {\n        a* { error a }\n        b* { tailcall g }\n        default { error d }\n    }\n    puts $y\n}\n";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-15: all-arms-exit switch makes trailing read unreachable; must NOT fire W210; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_15_no_default_falls_through_fires() {
    // TP control: without a default an unmatched subject falls through; W210 must fire.
    let src =
        "proc f {x} {\n    switch -glob $x { a* { return 1 } b* { return 2 } }\n    puts $y\n}\n";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-15 TP: no-default switch falls through; W210 must fire; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_15_one_completing_arm_fires() {
    // TP control: one arm that completes lets the switch fall through with y unset.
    let src = "proc f {x} {\n    switch -glob $x { a* { return 1 } default { set z 9 } }\n    puts $y\n}\n";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-15 TP: completing-arm switch falls through; W210 must fire; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_15_all_break_switch_not_a_proc_terminator() {
    // TP (Codex P1): a switch whose arms all `break` jumps to loop exit, not proc return.
    let src = "proc f {x} { foreach i {1 2 3} { switch -glob $x {a* {break} default {break}} }; puts $y }\n";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-15 TP: all-break switch is a loop jump, not a proc return; W210 must fire; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_15_while1_break_in_opaque_switch_exits_loop() {
    // TP (Codex P1 / C3): while 1 whose only exit is break inside opaque switch — post-loop read is reachable.
    let src = "proc f {x} { while 1 { switch -glob $x {a* {break} default {break}} }; puts $y }\n";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-15 TP: break-in-opaque-switch must make while-1 exit reachable; W210 must fire; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_15_continue_in_opaque_switch_loop_silent() {
    // FP control: benign opaque-switch continue arm; var set before switch every iteration.
    let src = "proc f {} { foreach x {1 2 3} { set y 1; switch -glob $x {a* {continue} default {}} }; puts $y }\n";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-15: var set before opaque-switch continue arm stays defined; must NOT fire W210; emitted: {:?}",
        codes(src, D)
    );
}

// FP-RBS-16 — phi operand on a dead loop-exit edge (`while 1` + `break`)

const FP_RBS_16_REPRO: &str = "\
proc f {} {
    # while 1 runs the body >=1 time; the only exit is the break, where y is
    # already set -> $y is always defined here.
    while 1 { set y 1; break }
    puts $y
}
";

#[test]
fn fp_rbs_16_while1_break_set_silent() {
    // FP-RBS-16: `while 1` dead cond-exit edge; y is set before every break.
    assert!(
        !fires(FP_RBS_16_REPRO, D, "W210"),
        "FP-RBS-16: dead cond-exit edge operand must NOT fire W210; emitted: {:?}",
        codes(FP_RBS_16_REPRO, D)
    );
}

#[test]
fn fp_rbs_16_while1_conditional_break_silent() {
    // FP: `while 1 { set result [compute]; if {[ok $result]} break }`.
    let src = "proc f {} {\n    while 1 {\n        set result [compute]\n        if {[ok $result]} break\n    }\n    return $result\n}\n";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-16: result is set before every conditional break; must NOT fire W210; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_16_normal_while_body_defined_silent() {
    // FP-RBS-19 (#756): a non-constant `while` may run zero times, but its body
    // unconditionally sets `y`, so a read after the loop is defined whenever the
    // loop ran. Matching C Tcl (which errors only when the condition is false on
    // entry at runtime, not merely when it could be), we assume a may-run loop
    // runs — the after-loop read is not read-before-set.
    let src = "proc f {n} {\n    while {$n > 0} { set y 1; incr n -1 }\n    puts $y\n}\n";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-19: may-run while whose body defines 'y' must NOT fire W210 after the loop; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_16_partial_break_def_still_fires() {
    // TP control: only ONE of two break paths sets y; live exit edge carries unset origin.
    let src = "proc f {c} {\n    while 1 {\n        if {$c} { set y 1; break }\n        if {!$c} break\n    }\n    puts $y\n}\n";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-16 TP: a break path that leaves 'y' unset must fire W210; emitted: {:?}",
        codes(src, D)
    );
}

// FP-RBS-17 — foreach over a non-empty literal runs its body at least once

const FP_RBS_17_REPRO: &str = "\
proc f {} {
    # the list is a non-empty literal, so the body runs >=1 time and y is set
    # before puts reads it.
    foreach x {1 2 3} { set y $x }
    puts $y
}
";

#[test]
fn fp_rbs_17_foreach_literal_silent() {
    // FP-RBS-17: foreach over non-empty literal list always runs; y is defined after loop.
    assert!(
        !fires(FP_RBS_17_REPRO, D, "W210"),
        "FP-RBS-17: non-empty-literal foreach defines 'y'; must NOT fire W210; emitted: {:?}",
        codes(FP_RBS_17_REPRO, D)
    );
}

#[test]
fn fp_rbs_17_loop_var_after_foreach_silent() {
    // FP: loop variable is set on every iteration; defined after non-empty-literal foreach.
    let src = "proc f {} { foreach x {a b c} { } ; puts $x }\n";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-17: loop var defined after non-empty foreach; must NOT fire W210; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_17_empty_literal_still_fires() {
    // TP control: foreach over an empty literal never runs; y is unset.
    let src = "proc f {} { foreach x {} { set y $x } ; puts $y }\n";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-17 TP: empty-list foreach leaves 'y' unset; W210 must fire; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_17_dynamic_list_body_defined_silent() {
    // FP-RBS-19 (#756): a foreach over a *dynamic* list may be empty, but its
    // body unconditionally sets `y`, so a read after the loop is defined
    // whenever the loop ran. Matching C Tcl (which errors only when the list is
    // actually empty at runtime), we assume a may-run loop runs — this is the
    // exact accumulator idiom the reporter hit (`foreach … { lappend acc … }`).
    let src = "proc f {items} { foreach x $items { set y $x } ; puts $y }\n";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-19: dynamic-list foreach whose body defines 'y' must NOT fire W210 after the loop; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_17_continue_before_set_still_fires() {
    // TP control: continue before set can skip the def on the last iteration.
    let src = "proc f {} { foreach x {1} { if {$x==1} continue; set y $x } ; puts $y }\n";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-17 TP: continue-before-set may skip 'y'; W210 must fire; emitted: {:?}",
        codes(src, D)
    );
}

// FP-RBS-18 — for whose condition is true on entry runs its body at least once

const FP_RBS_18_REPRO: &str = "\
proc f {} {
    # 0 < 3 is true on entry, so the body runs >=1 time and y is set.
    for {set i 0} {$i < 3} {incr i} { set y $i }
    puts $y
}
";

#[test]
fn fp_rbs_18_for_true_on_entry_silent() {
    // FP-RBS-18: for whose condition is true on entry runs body at least once; y is defined.
    assert!(
        !fires(FP_RBS_18_REPRO, D, "W210"),
        "FP-RBS-18: for true-on-entry defines 'y'; must NOT fire W210; emitted: {:?}",
        codes(FP_RBS_18_REPRO, D)
    );
}

#[test]
fn fp_rbs_18_false_on_entry_still_fires() {
    // TP control: for whose condition is false on entry runs zero times; y is unset.
    let src = "proc f {} { for {set i 5} {$i < 3} {incr i} { set y $i } ; puts $y }\n";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-18 TP: false-on-entry for runs zero times; W210 must fire; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_18_unknown_bound_body_defined_silent() {
    // FP-RBS-19 (#756): a `for` with an unknown bound may run zero times, but
    // its body unconditionally sets `y`, so a read after the loop is defined
    // whenever the loop ran. Matching C Tcl, we assume a may-run loop runs.
    let src = "proc f {n} { for {set i 0} {$i < $n} {incr i} { set y $i } ; puts $y }\n";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-19: unknown-bound for whose body defines 'y' must NOT fire W210 after the loop; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_18_break_before_set_still_fires() {
    // TP control: break before def exits with y unset.
    let src =
        "proc f {} { for {set i 0} {$i < 3} {incr i} { if {$i==0} break; set y $i } ; puts $y }\n";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-18 TP: break-before-set exits with 'y' unset; W210 must fire; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_18_stale_const_init_body_defined_silent() {
    // A stale const from the for-init (`set i $n` overwrites `set i 0`) leaves
    // the loop may-run (not guaranteed-nonempty; the rotation-soundness of that
    // classification is covered by the CFG-builder tests). Under FP-RBS-19
    // (#756) the after-loop read is still silent: the body unconditionally sets
    // `y`, so a may-run loop is assumed to run, matching C Tcl.
    let src = "proc f {n} { for {set i 0; set i $n} {$i < 3} {incr i} { set y $i }; puts $y }\n";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-19: may-run for whose body defines 'y' must NOT fire W210 after the loop; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_18_incr_in_init_provably_empty_fires() {
    // TP: `for {set i 0; incr i 5} {$i < 3} …` resolves the loop var to `i = 5`,
    // so `5 < 3` is false — SCCP proves the body dead (a *provably* zero-trip
    // loop that always errors in tclsh), and W210 still fires. The incr also
    // invalidates the rotation constant (see the CFG-shape test
    // `cfg::for_rotation_requires_a_non_stale_constant_init`); unlike a dynamic
    // may-run bound (FP-RBS-19), this one is statically empty, not merely maybe.
    let src = "proc f {} { for {set i 0; incr i 5} {$i < 3} {incr i} { set y $i }; puts $y }\n";
    assert!(
        fires(src, D, "W210"),
        "provably-empty incr-in-init for leaves y unset; W210 must fire; emitted: {:?}",
        codes(src, D)
    );
}

// FP-RBS-19 (#756) — a may-run loop whose body defines a variable is assumed
// to run: an after-loop read is not read-before-set

// The reporter's exact pattern (issue #756): a multi-group `foreach` over
// dynamic (`dict get`) lists building `lappend` accumulators, read after the
// loop. tclsh errors only when the iterator lists are actually empty at
// runtime; on real data the accumulators are defined, so the after-loop reads
// are not read-before-set.
const FP_RBS_19_REPRO: &str = "\
set data [getDataDict]
foreach time [dict get $data time] vout [dict get $data osc_out] {
    lappend timeVout [list $time $vout]
    lappend timeImeas [list $time]
}
puts $timeVout
puts $timeImeas
";

#[test]
fn fp_rbs_19_reporter_foreach_accumulator_silent() {
    assert!(
        !fires(FP_RBS_19_REPRO, D, "W210"),
        "FP-RBS-19: dynamic-foreach `lappend` accumulator read after the loop must NOT fire W210; emitted: {:?}",
        codes(FP_RBS_19_REPRO, D)
    );
}

#[test]
fn fp_rbs_19_provably_empty_literal_still_fires() {
    // TP control: a *provably* empty literal list runs zero times, so tclsh
    // always errors — the after-loop read must keep firing.
    let src = "foreach x {} { lappend acc $x }\nputs $acc\n";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-19 TP: provably-empty foreach leaves 'acc' unset; W210 must fire; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_19_conditional_def_in_body_still_fires() {
    // TP control: the body defines `y` only under an inner `if`, so even when
    // the loop runs `y` may be unset — a genuine read-before-set that fires.
    let src = "proc f {items} { foreach x $items { if {$x > 0} { set y $x } }\n puts $y }\n";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-19 TP: conditionally-defined body var may be unset after the loop; W210 must fire; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_19_first_iteration_in_body_read_still_fires() {
    // TP control: a read *inside* the loop body before the body's own set is a
    // genuine first-iteration read-before-set — the suppression is confined to
    // reads reached after the loop.
    let src = "proc f {items} { foreach x $items { puts $y; set y $x } }\n";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-19 TP: in-body read before the set must still fire; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_19_nested_loop_accumulator_silent() {
    // Nested accumulator (`foreach r $rows { foreach c $cols { lappend cells … } }`):
    // the loop-entry-only-undef analysis converges over nesting, so the inner
    // loop's defined-on-exit result makes the outer loop entry-only too — the
    // after-loop read is silent.
    let src = "proc f {rows cols} { foreach r $rows { foreach c $cols { lappend cells [list $r $c] } }\n return $cells }\n";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-19: nested-loop accumulator read after the loops must NOT fire W210; emitted: {:?}",
        codes(src, D)
    );
    // TP control: the inner body defines `y` only conditionally, so even when
    // both loops run `y` may be unset after them — still fires.
    let tp = "proc f {rows cols} { foreach r $rows { foreach c $cols { if {$c} { set y 1 } } }\n puts $y }\n";
    assert!(
        fires(tp, D, "W210"),
        "FP-RBS-19 TP: conditionally-defined var in a nested loop may be unset; W210 must fire; emitted: {:?}",
        codes(tp, D)
    );
}

#[test]
fn fp_rbs_19_while_and_for_may_run_silent() {
    // A dynamic `while` and an unknown-bound `for` are both may-run loops whose
    // bodies define the variable — silent after the loop, like the foreach case.
    for src in [
        "proc f {n} { while {$n > 0} { set y $n; incr n -1 }\n puts $y }\n",
        "proc f {n} { for {set i 0} {$i < $n} {incr i} { set y $i }\n puts $y }\n",
    ] {
        assert!(
            !fires(src, D, "W210"),
            "FP-RBS-19: may-run loop whose body defines 'y' must be silent after the loop; emitted: {:?}",
            codes(src, D)
        );
    }
}

// call-by-name DYNAMIC_NAME_LOCAL trait tests

#[test]
fn fp_rbs_callbyname_scan_target_not_caller_alias() {
    // TP: scan $target uses value as local dynamic name; caller's x must still fire W211/W220.
    let src = "proc maybe {target} { scan 42 %d $target }\nproc caller {} { set x 1; maybe x }";
    let cs = codes(src, D);
    assert!(
        cs.iter().any(|c| c == "W211" || c == "W220"),
        "scan $target — caller's x must still fire W211/W220; emitted: {cs:?}"
    );
}

#[test]
fn fp_rbs_callbyname_regexp_target_not_caller_alias() {
    // TP: regexp $target uses value as local dynamic name; caller's x must still fire W211/W220.
    let src =
        "proc maybe {target} { regexp {(.)} a -> $target }\nproc caller {} { set x 1; maybe x }";
    let cs = codes(src, D);
    assert!(
        cs.iter().any(|c| c == "W211" || c == "W220"),
        "regexp $target — caller's x must still fire W211/W220; emitted: {cs:?}"
    );
}

#[test]
fn fp_rbs_callbyname_regsub_target_not_caller_alias() {
    // TP: regsub $target uses value as local dynamic name; caller's x must still fire W211/W220.
    let src = "proc maybe {target} { regsub a a b $target }\nproc caller {} { set x 1; maybe x }";
    let cs = codes(src, D);
    assert!(
        cs.iter().any(|c| c == "W211" || c == "W220"),
        "regsub $target — caller's x must still fire W211/W220; emitted: {cs:?}"
    );
}

#[test]
fn fp_rbs_callbyname_lassign_target_not_caller_alias() {
    // TP: lassign $target uses value as local dynamic name; caller's x must still fire W211/W220.
    let src = "proc maybe {target} { lassign {1} $target }\nproc caller {} { set x 1; maybe x }";
    let cs = codes(src, D);
    assert!(
        cs.iter().any(|c| c == "W211" || c == "W220"),
        "lassign $target — caller's x must still fire W211/W220; emitted: {cs:?}"
    );
}

#[test]
fn fp_rbs_callbyname_upvar_alias_still_suppresses() {
    // TP control: when callee uses `upvar $target v`, caller's x IS aliased — must NOT fire W211/W220.
    let src = "proc maybe {target} { upvar 1 $target v\n scan 42 %d v }\nproc caller {} { set x 1; maybe x }";
    let cs = codes(src, D);
    let fires_any = cs.iter().any(|c| c == "W211" || c == "W220");
    assert!(
        !fires_any,
        "upvar-aliased callee write must continue to suppress caller W211/W220; emitted: {cs:?}"
    );
}

// FP-RBS-20 — a condition-embedded variable writer defines its target
// (issue #923 audit idx 49)
//
// `condition_command_out_vars` used to hardcode `catch` / `scan` / `gets` /
// `regexp`, so the ubiquitous `if {[set VAR EXPR] rel N} {…$VAR…}` idiom
// (ticklecharts utils.tcl:1068-1071) read as read-before-set.  It now asks
// the registry for the call's `ArgRole::VarWrite` positions, so every command
// the registry knows writes an out-var answers.
//
// Oracle (tclsh 9.0.4 and 8.6.14, identical):
//   proc bar {lst} { if {[set idx [lsearch $lst foo]] > -1} { puts $idx } }
//   bar {a foo b}                                    → 1
//   proc loopy {} { set i 0; while {[incr i] < 3} { puts $i } }
//   loopy                                            → 1 2
//   proc lass {lst} { lassign $lst a b; puts "$a-$b" }
//   lass {1 2}                                       → 1-2
//   proc binscan {} { binary scan "AB" H2H2 hi lo; puts "$hi $lo" }
//   binscan                                          → 41 42

#[test]
fn fp_rbs_20_set_in_if_condition_defines_its_target() {
    let src = "proc bar {lst} { if {[set idx [lsearch $lst foo]] > -1} { puts $idx } }\n";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-20: `if {{[set idx …]}}` defines idx; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_20_incr_in_while_condition_defines_its_target() {
    let src = "proc loopy {} { while {[incr j] < 3} { puts $j } }\n";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-20: `while {{[incr j] …}}` defines j; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_20_lassign_in_condition_defines_its_targets() {
    let src = "proc la {lst} { if {[lassign $lst a b] eq {}} { puts $a$b } }\n";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-20: `lassign` writes a and b; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_20_binary_scan_in_condition_defines_its_targets() {
    let src = "proc bs {d} { if {[binary scan $d H2H2 hi lo] == 2} { puts $hi$lo } }\n";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-20: `binary scan` writes hi and lo; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_20_previously_hardcoded_four_still_silent() {
    // Regression net for the deleted name list: catch / scan / gets / regexp
    // must keep the recognition the registry query replaced.
    for src in [
        "proc c {} { if {[catch {error x} msg]} { puts $msg } }\n",
        "proc s {t} { if {[scan $t {%d %d} a b] == 2} { puts $a$b } }\n",
        "proc g {fp} { while {[gets $fp line] >= 0} { puts $line } }\n",
        "proc r {s} { if {[regexp {(a)(b)} $s m a b]} { puts $m$a$b } }\n",
    ] {
        assert!(
            !fires(src, D, "W210"),
            "FP-RBS-20: registry query must cover the old hardcoded four; \
{src} emitted: {:?}",
            codes(src, D)
        );
    }
}

#[test]
fn fp_rbs_20_genuine_read_before_set_still_fires() {
    // TP control: a condition with no variable writer at all leaves the body
    // read genuinely undefined.
    let src = "proc t {} { if {[string length foo] > 0} { puts $undefinedvar } }\n";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-20 TP: an unwritten body read must still fire W210; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_20_condition_unset_is_not_a_definition() {
    // TN control: `unset` carries `ArgRole::VarWrite` on the name it removes,
    // so the registry harvest must exclude `Traits::DESTROYS_VARIABLE` —
    // otherwise the harvest would claim `gone` is defined and swallow a real
    // warning.  tclsh 9.0.4 / 8.6.14 agree the read is an error:
    //   proc t {} { if {[catch {unset gone}]} { puts $gone } }
    //   catch {t} err → 1, err → `can't read "gone": no such variable`
    let src = "proc t {} { if {[catch {unset gone}]} { puts $gone } }\n";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-20 TN: a condition `unset` must not manufacture a def; \
emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_20_substituted_command_head_harvests_nothing() {
    // TN control (PR #1076 review, P2): the head word's *content* spelling
    // drops the `$`, so `[$set length foo]` used to resolve as the builtin
    // `set` and harvest `length` as a definition — silencing a genuine
    // warning.  Which command runs is run-time data, so nothing may be
    // claimed about what it writes.
    //
    // tclsh 9.0.4 / 8.6.14 (identical): calling `f string` executes
    // `string length foo`, which defines nothing —
    //   catch {f string} err → 1, err → `can't read "length": no such variable`
    let src = "proc f {set} { if {[$set length foo]} { puts $length } }\n";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-20 TN: a substituted head must not harvest its lookalike \
builtin's out-vars; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_20_literal_head_named_like_the_variable_still_harvests() {
    // TP control for the pair above: a *literal* `set` head keeps the
    // recognition — the guard must key on substitution, not on the spelling.
    let src = "proc f {} { if {[set length foo]} { puts $length } }\n";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-20: a literal `set` head still defines its target; \
emitted: {:?}",
        codes(src, D)
    );
}

// Issue #1078 — a brace-quoted `$`-bearing name is a *literal* variable name
//
// Oracle, tclsh 9.0.4 and 8.6.14, byte-identical:
//
//   set {$n} v ; set {$n}            -> v
//   info exists {$n} / info exists n -> 1 / 0     (two distinct variables)
//   set n other ; set {$n}           -> v         (unaffected)
//   set {$m} 1 ; set m               -> can't read "m": no such variable
//   set ${$n}                        -> can't read "v"  (`${$n}` read `$n`)
//   set {a b} v2 ; set x ${a b}      -> v2
//   set i 5 ; set {arr($i)} 1
//     array names arr                -> {$i}      (the literal key)
//     info exists arr(5)             -> 0
//   unset {$n} ; info exists {$n} / n -> 0 / 1
//
// These are the acceptance measurements recorded when PR #1076 reverted its
// one-parameter attempt (`docs/design/compiler/ssa-construction.md`).

/// The **missed true positive** the reverted #1076 attempt produced: with the
/// def landing on the wrong variable, `set {$n} 1; puts $n` stopped warning
/// even though tclsh errors `can't read "n": no such variable`.
#[test]
fn issue_1078_braced_write_does_not_define_the_plain_name() {
    let src = "proc f {} { set {$n} 1; puts $n }\n";
    assert!(
        fires(src, D, "W210"),
        "#1078 TP: `set {{$n}} 1` defines `$n`, so the bare `$n` read is still \
read-before-set (tclsh: `can't read \"n\"`); emitted: {:?}",
        codes(src, D)
    );
}

/// The **pre-existing false positive** #1078 was filed for: a lone
/// `set {$n} 1` reads nothing at all, yet the naming layer recorded a read of
/// `n` and fired `W210 Variable 'n' is read before it is set` on code that
/// never mentions `n`.
#[test]
fn issue_1078_braced_write_alone_is_not_a_read_of_the_plain_name() {
    let src = "proc f {} { set {$n} 1 }\n";
    assert!(
        !fires(src, D, "W210"),
        "#1078 FP: `set {{$n}} 1` substitutes nothing and reads nothing; \
emitted: {:?}",
        codes(src, D)
    );
    assert!(
        fires(src, D, "W211"),
        "#1078: the write is a genuine unused-variable hint; emitted: {:?}",
        codes(src, D)
    );
    let unused: Vec<String> = crate::analyser::Analyser::new()
        .analyse(src, D)
        .diagnostics
        .iter()
        .filter(|d| d.code.to_string() == "W211")
        .map(|d| d.message.clone())
        .collect();
    assert!(
        unused.iter().any(|m| m.contains("'$n'")),
        "#1078: the hint must name `$n`, not `n`; got {unused:?}"
    );
}

/// The **two fresh false `W220`s** the reverted #1076 attempt produced.  The
/// braced verdict must match the identically-shaped plain-named control
/// exactly — both silent, because `[set …]` really does read the store.
#[test]
fn issue_1078_braced_double_store_matches_the_plain_control() {
    let braced = "proc f {} { set {$n} 1; set {$n} 2; return [set {$n}] }\n";
    let plain = "proc f {} { set n 1; set n 2; return [set n] }\n";
    assert!(
        !fires(braced, D, "W220"),
        "#1078: `[set {{$n}}]` reads the store, so neither assignment is dead \
(tclsh returns 2); emitted: {:?}",
        codes(braced, D)
    );
    assert_eq!(
        codes(braced, D),
        codes(plain, D),
        "#1078: the braced spelling must draw exactly the plain spelling's \
verdicts"
    );
}

/// `unset {$n}` destroys the literal `$n` — it neither reads nor destroys `n`,
/// so no `W213 Variable 'n' may not exist`.
#[test]
fn issue_1078_braced_unset_does_not_touch_the_plain_name() {
    let src = "proc f {} { set {$n} 1; unset {$n}; return ok }\n";
    assert!(
        !fires(src, D, "W213"),
        "#1078: `unset {{$n}}` destroys the variable it just set; emitted: {:?}",
        codes(src, D)
    );
    assert!(
        !fires(src, D, "W210"),
        "#1078: …and reads nothing; emitted: {:?}",
        codes(src, D)
    );
}

/// TN control for the family: the *unbraced* substituted spellings must keep
/// behaving as dynamic names — the #1076 barrier still has to fire.  Same
/// shape as FP-DS-14's control, restated here so the #1078 changes cannot
/// quietly erase the barrier they sit on top of.
#[test]
fn issue_1078_unbraced_substituted_target_still_behaves_dynamically() {
    // `[set $p]` may observe *any* store, so the read barrier blinds the
    // dead-store pass for the whole function.
    let dynamic = "proc f {p} { set b [set $p]\n set a 1\n set a 2\n return \"$a$b\" }\n";
    assert!(
        !fires(dynamic, D, "W220"),
        "#1076 barrier: a computed read target must still blind W220; \
emitted: {:?}",
        codes(dynamic, D)
    );
    // TP control: brace the name and the barrier clears, so the genuine dead
    // store is reported again.
    let braced = "proc f {p} { set b [set {$p}]\n set a 1\n set a 2\n return \"$a$b\" }\n";
    assert!(
        fires(braced, D, "W220"),
        "#1078: a brace-quoted target computes nothing, so the unrelated dead \
store is still reported; emitted: {:?}",
        codes(braced, D)
    );
}

/// The `${…}` **read** form reaches the same cell the brace-quoted write
/// created: `set {$n} 1; return ${$n}` is a complete write/read pair with
/// nothing to report, and it must not be read as touching `n`.
#[test]
fn issue_1078_brace_reference_form_reads_the_literal_name() {
    let src = "proc f {} { set {$n} 1; return ${$n} }\n";
    assert!(
        codes(src, D).is_empty(),
        "#1078: `${{$n}}` reads the `{{$n}}` cell — nothing to report; \
emitted: {:?}",
        codes(src, D)
    );
}

/// A brace-quoted **array element** keeps its literal key, and a space-bearing
/// braced name is an ordinary variable — both must key on their own spelling.
#[test]
fn issue_1078_braced_element_and_spaced_names_key_on_their_own_spelling() {
    let elem = "proc f {} { set {arr($i)} 1; return [set {arr($i)}] }\n";
    assert!(
        codes(elem, D).is_empty(),
        "#1078: `{{arr($i)}}` writes and reads one literal element; emitted: {:?}",
        codes(elem, D)
    );
    let spaced = "proc f {} { set {a b} 1; return [set {a b}] }\n";
    assert!(
        codes(spaced, D).is_empty(),
        "#1078: `{{a b}}` is a legal name written and read; emitted: {:?}",
        codes(spaced, D)
    );
}

// Issues #1142 / #1237 — a braced word is data, not a script the call site
// substitutes.  The SSA use is *classified* (`ssa::UseClass`) rather than
// dropped: liveness keeps honouring it (the text may be evaluated later),
// read-before-set does not.
//
// Oracle, tclsh 8.6.14:
//
//   puts {$y}                       -> $y          (y undefined; no error)
//   puts {\n set y 1\n return $y\n} -> the literal three lines
//   proc f {} { return {$y} }; f    -> $y
//   catch {puts $y} m; set m        -> can't read "y": no such variable

/// #1237 — a braced data argument reads nothing, so no `W210`.
#[test]
fn issue_1237_braced_data_argument_is_not_a_read() {
    for src in [
        "proc f {} { puts {$y} }\n",
        "proc f {} { puts {\n    set y 1\n    return $y\n} }\n",
        "proc f {} { return {$y} }\n",
    ] {
        assert!(
            !fires(src, D, "W210"),
            "#1237: a braced word performs no substitution; {src} emitted: {:?}",
            codes(src, D)
        );
    }
}

/// #1142 — the generic half: an un-hooked / unknown definer's brace-shaped
/// trailing argument carries no role information, so a `$y` in it that the
/// same word `set`s is that body's own local, whichever frame the body ends up
/// running in — never a read-before-set of the enclosing scope. The
/// unknown-command hint stays: abstaining about the *body* says nothing about
/// the *name*.
#[test]
fn issue_1142_unknown_definer_body_is_not_read_before_set() {
    let src = "proc f {} { mydefiner ::foo::bar {optlist} {\n    set y 1\n    return $y\n} }\n";
    assert!(
        !fires(src, D, "W210"),
        "#1142: an un-hooked definer body's own local is not read-before-set; \
emitted: {:?}",
        codes(src, D)
    );
    assert!(
        fires(src, D, "W123"),
        "#1142: the unknown command itself is still reported; emitted: {:?}",
        codes(src, D)
    );
}

/// TN control for #1142 — a **free** read in an unclassified braced word (one
/// nothing sets) is still a read: the word may be a script, and a wrapper that
/// hands it to an `uplevel`-ing worker runs it in this frame, where tclsh
/// errors on the unset name. Only a registry-*described* command's braced
/// word (`puts {$y}`) is known data.
#[test]
fn issue_1142_unclassified_braced_word_free_read_still_fires() {
    let src = "proc f {} { mywrapper {puts $notset} }\n";
    assert!(
        fires(src, D, "W210"),
        "#1142 TN: an unclassified braced word may run in this frame; \
emitted: {:?}",
        codes(src, D)
    );
}

/// TN control — a **quoted** word really is substituted, so the genuine
/// read-before-set survives.  This is the measurement that separates
/// "classified use" from "stop scanning braced words".
#[test]
fn issue_1237_quoted_word_still_fires_w210() {
    for src in [
        "proc f {} { puts \"$y\" }\n",
        "proc f {} { puts $y }\n",
        "proc f {} { return \"$y\" }\n",
    ] {
        assert!(
            fires(src, D, "W210"),
            "#1237 TN: a substituted read must still fire; {src} emitted: {:?}",
            codes(src, D)
        );
    }
}

/// FP guard — a braced word whose role has the callee evaluate it **in this
/// frame** is a real read: `expr`'s expression and `if`'s condition/body both
/// substitute against the caller's variables.
#[test]
fn issue_1237_in_frame_braced_roles_still_read() {
    for src in [
        "proc f {} { expr {$p + $q} }\n",
        "proc f {} { if {$p} { puts hi } }\n",
        "proc f {} { catch {puts $p} m }\n",
    ] {
        assert!(
            fires(src, D, "W210"),
            "#1237 FP guard: an Expr/Body-role braced word is evaluated in \
this frame; {src} emitted: {:?}",
            codes(src, D)
        );
    }
}

/// FP guard — the use itself must **survive**, classified: dropping it would
/// resurrect `W211 set but never used` / `W220 never read` on a variable whose
/// only mention is inside a braced word that may be evaluated later (a
/// deferred callback, an `eval`, an unknown definer).  This is the guard rail
/// that rules out the "skip the scan" shortcut, restated for a scalar
/// alongside `w220_braced_literal_arg_is_not_a_read`'s array element.
#[test]
fn issue_1237_quoted_use_still_keeps_the_variable_live() {
    let src = "proc f {} { set x 1; puts {$x} }\n";
    assert!(
        !fires(src, D, "W211") && !fires(src, D, "W220"),
        "#1237: a quoted mention keeps the store live; emitted: {:?}",
        codes(src, D)
    );
    // TP control: with no mention of any kind the dead store is reported.
    let unmentioned = "proc f {} { set x 1; puts {$y} }\n";
    assert!(
        fires(unmentioned, D, "W211") || fires(unmentioned, D, "W220"),
        "#1237 TP: an unmentioned store is still dead; emitted: {:?}",
        codes(unmentioned, D)
    );
}

// Issue #1260 — a `$var` inside a **braced** `foreach` / `lmap` / `dict for`
// value word is a literal list element, not a substitution.
//
// tclsh-proof (tclsh 8.6.14 — the only interpreter available in this
// container):
//
//   foreach n {a $b c} { puts $n }          -> a / $b / c  (b undefined)
//   dict for {k v} {a $b} { puts "$k=$v" }  -> a=$b        (b undefined)
//
// Braces are the only thing separating the literal spelling from the
// substituting one, and the IR's `list_arg` stores the word's *content*, so
// `ForeachIterator::list_braced` is what the CFG's synthetic loop-header call
// carries the fact on.

/// #1260 FP — a braced value word reads nothing, at top level and inside a
/// proc, for every loop that lowers to `Statement::Foreach`.
#[test]
fn issue_1260_braced_loop_value_word_is_not_a_read() {
    for src in [
        "foreach n {a $b c} { }\n",
        "lmap n {a $b c} { }\n",
        "proc p {} { foreach n {a $b c} { } }\n",
        "proc p {} { lmap n {a $b c} { } }\n",
        "dict for {k v} {a $b} { }\n",
        "dict map {k v} {a $b} { }\n",
        // Multi-group: the braced group stays quiet alongside a
        // substituting one whose own read is satisfied.
        "proc p {l} { foreach x {a $b c} y $l { } }\n",
    ] {
        assert!(
            !fires(src, D, "W210"),
            "#1260: a braced loop value word substitutes nothing; {src} emitted: {:?}",
            codes(src, D)
        );
    }
}

/// #1260 TP — the substituting spellings of the same word must still fire.
/// This is what separates "carry the braced flag" from "stop scanning the
/// loop's value word".
#[test]
fn issue_1260_substituted_loop_value_word_still_fires() {
    for src in [
        "foreach n \"a $b c\" { }\n",
        "foreach n $b { }\n",
        "lmap n \"a $b c\" { }\n",
        "proc p {} { foreach n \"a $b c\" { } }\n",
        "proc p {} { foreach n $b { } }\n",
        "dict for {k v} $b { }\n",
    ] {
        assert!(
            fires(src, D, "W210"),
            "#1260 TP: a substituted loop value word is a real read; {src} emitted: {:?}",
            codes(src, D)
        );
    }
}

/// #1260 TN — the braced word still keeps the mentioned store *live*: the use
/// is classified (`UseClass::Quoted`), not dropped, so W211/W220 must not
/// resurrect on a variable whose only mention is inside the braced list. Same
/// guard rail as `issue_1237_quoted_use_still_keeps_the_variable_live`.
#[test]
fn issue_1260_braced_loop_value_word_keeps_the_store_live() {
    let src = "proc p {} { set x 1; foreach n {$x} { } }\n";
    assert!(
        !fires(src, D, "W211") && !fires(src, D, "W220"),
        "#1260: a quoted mention keeps the store live; emitted: {:?}",
        codes(src, D)
    );
    // TP control: with no mention of any kind the dead store is reported.
    let unmentioned = "proc p {} { set x 1; foreach n {$y} { } }\n";
    assert!(
        fires(unmentioned, D, "W211") || fires(unmentioned, D, "W220"),
        "#1260 TP: an unmentioned store is still dead; emitted: {:?}",
        codes(unmentioned, D)
    );
}

// Issue #1266 — a read collapsed out of an **opaque** `switch` arm keeps its
// `UseClass`.
//
// A non-lowered `switch` (`-glob` / `-regexp`, or `-exact` with a
// fall-through arm) keeps its arms as one opaque CFG statement, so those arm
// bodies are the only scripts that reach SSA un-lowered. `ssa::switch_reads`
// collected them through `free_reads_in_script` / `reads_in_script` as a bare
// name set and merged the lot into `ClassifiedUses::substituted`, so every
// brace-quoted DATA word inside an arm was seen as a *substituted* read and
// drew a false W210 — while the identical spelling outside an arm is
// correctly quiet, because the lowered path classifies it `UseClass::Quoted`
// and the W210 emitter skips a quoted use.
//
// The walk now threads `ClassifiedUses` end to end, so an arm body is
// classified exactly as the same script would be when lowered.
//
// tclsh-proof (tclsh 9.0.4, the interpreter available in this container; the
// issue reports the same on 8.6.14):
//
//   proc f {z} { switch -glob $z { a* { foreach n {a $b c} {puts $n} } } }
//   f abc                                     -> a / $b / c   (b undefined)
//   proc f {z} { switch -glob $z { a* { puts {$b} } } };  f abc -> $b
//   proc f {z} { switch $z { a - b { puts {$q} } } };     f b   -> $q
//   proc f {z} { switch -regexp $z { a.* { puts {$b} } default { puts {$c} } } }
//   f zz                                      -> $c
//   proc f {z} { set x 1; switch -glob $z { a* { foreach n {$x} {puts $n} } } }
//   f abc                                     -> $x   (the literal two chars)
//   proc f {z} { switch -glob $z { a* { puts $b } } }
//   f abc                    -> can't read "b": no such variable

/// #1266 FP — a braced data word inside an opaque arm substitutes nothing, in
/// every arm shape that keeps the `switch` opaque and at every nesting depth
/// the collapsed walk descends.
#[test]
fn issue_1266_braced_data_word_in_opaque_switch_arm_is_not_a_read() {
    for src in [
        // The two spellings named on the issue.
        "proc f {z} { switch -glob $z { a* { foreach n {a $b c} {} } } }\n",
        "proc f {z} { switch -glob $z { a* { puts {$b} } } }\n",
        // -regexp keeps it opaque too, and the default body is walked by the
        // same helper as an arm body.
        "proc f {z} { switch -regexp $z { a.* { puts {$b} } } }\n",
        "proc f {z} { switch -regexp $z { a.* { } default { puts {$b} } } }\n",
        // -exact stays opaque when an arm falls through.
        "proc f {z} { switch $z { a - b { puts {$q} } } }\n",
        // Structured statements inside an arm are walked by `reads_in_stmt`,
        // not lowered — each recursion must carry the classification.
        "proc f {z} { switch -glob $z { a* { if {1} { puts {$b} } } } }\n",
        "proc f {z} { switch -glob $z { a* { if {0} { } else { puts {$b} } } } }\n",
        "proc f {z} { switch -glob $z { a* { while {[incr i] < 2} { puts {$b} } } } }\n",
        "proc f {z} { switch -glob $z { a* { for {set i 0} {$i < 1} {incr i} { puts {$b} } } } }\n",
        "proc f {z} { switch -glob $z { a* { catch { puts {$b} } m } } }\n",
        "proc f {z} { switch -glob $z { a* { try { puts {$b} } finally { puts {$c} } } } }\n",
        // A nested opaque switch inside an opaque arm.
        "proc f {z} { switch -glob $z { a* { switch -glob $z { c* { puts {$b} } } } } }\n",
        // Every `Statement::Foreach` value word, including a multi-group call.
        "proc f {z} { switch -glob $z { a* { foreach n {a $b c} m {$d} {} } } }\n",
        "proc f {z} { switch -glob $z { a* { lmap n {a $b c} {} } } }\n",
        // A braced `return` value is literal in an arm exactly as outside one.
        "proc f {z} { switch -glob $z { a* { return {$b} } } }\n",
    ] {
        assert!(
            !fires(src, D, "W210"),
            "#1266: a braced word inside an opaque switch arm substitutes nothing; \
{src} emitted: {:?}",
            codes(src, D)
        );
    }
}

/// #1266 TP — the substituting spellings of the same words inside an opaque
/// arm must still fire. This is what separates "classify the arm's reads"
/// from "stop collecting them".
#[test]
fn issue_1266_substituted_word_in_opaque_switch_arm_still_fires() {
    for src in [
        "proc f {z} { switch -glob $z { a* { puts $b } } }\n",
        "proc f {z} { switch -glob $z { a* { foreach n \"a $b c\" {} } } }\n",
        "proc f {z} { switch -glob $z { a* { foreach n $b {} } } }\n",
        "proc f {z} { switch -glob $z { a* { if {$b} { } } } }\n",
        "proc f {z} { switch -glob $z { a* { return $b } } }\n",
        "proc f {z} { switch -regexp $z { a.* { } default { puts $b } } }\n",
        "proc f {z} { switch $z { a - b { puts $q } } }\n",
    ] {
        assert!(
            fires(src, D, "W210"),
            "#1266 TP: a substituted word inside an opaque switch arm is a real read; \
{src} emitted: {:?}",
            codes(src, D)
        );
    }
}

/// #1266 TN / guard rail — the classification must be *threaded*, never
/// dropped. The naive fix (omitting the name from `reads_in_script`) was
/// rejected in #1260 precisely because a dropped name loses its **liveness**
/// use as well, resurrecting a false W211/W220 on a store whose only mention
/// is the braced word. Same guard rail as
/// `issue_1237_quoted_use_still_keeps_the_variable_live` and
/// `issue_1260_braced_loop_value_word_keeps_the_store_live`, now inside an
/// opaque arm.
#[test]
fn issue_1266_quoted_arm_mention_keeps_the_store_live() {
    for src in [
        "proc p {z} { set x 1; switch -glob $z { a* { foreach n {$x} {} } } }\n",
        "proc p {z} { set x 1; switch -glob $z { a* { puts {$x} } } }\n",
        "proc p {z} { set x 1; switch -glob $z { a* { if {1} { puts {$x} } } } }\n",
        "proc p {z} { set x 1; switch -regexp $z { a.* { } default { puts {$x} } } }\n",
        "proc p {z} { set x 1; switch $z { a - b { puts {$x} } } }\n",
    ] {
        assert!(
            !fires(src, D, "W211") && !fires(src, D, "W220"),
            "#1266: a quoted mention inside an opaque arm keeps the store live; \
{src} emitted: {:?}",
            codes(src, D)
        );
    }
    // TP control: with no mention of any kind the dead store is still reported.
    for unmentioned in [
        "proc p {z} { set x 1; switch -glob $z { a* { foreach n {$y} {} } } }\n",
        "proc p {z} { set x 1; switch -glob $z { a* { puts {$y} } } }\n",
    ] {
        assert!(
            fires(unmentioned, D, "W211") || fires(unmentioned, D, "W220"),
            "#1266 TP: an unmentioned store is still dead; {unmentioned} emitted: {:?}",
            codes(unmentioned, D)
        );
    }
}

/// #1266 FN guard — the whole point is that an opaque arm agrees with the
/// lowered path. A plain `-exact` `switch` with no fall-through lowers its
/// arms into ordinary CFG blocks; the `-glob` spelling of the same body must
/// produce the same verdict, braced and substituted alike.
#[test]
fn issue_1266_opaque_arm_agrees_with_the_lowered_path() {
    for (lowered, opaque) in [
        (
            "proc f {z} { switch $z { a { puts {$b} } } }\n",
            "proc f {z} { switch -glob $z { a* { puts {$b} } } }\n",
        ),
        (
            "proc f {z} { switch $z { a { puts $b } } }\n",
            "proc f {z} { switch -glob $z { a* { puts $b } } }\n",
        ),
        (
            "proc f {z} { switch $z { a { foreach n {a $b c} {} } } }\n",
            "proc f {z} { switch -glob $z { a* { foreach n {a $b c} {} } } }\n",
        ),
        (
            "proc p {z} { set x 1; switch $z { a { foreach n {$x} {} } } }\n",
            "proc p {z} { set x 1; switch -glob $z { a* { foreach n {$x} {} } } }\n",
        ),
    ] {
        for code in ["W210", "W211", "W220"] {
            assert_eq!(
                fires(lowered, D, code),
                fires(opaque, D, code),
                "#1266: {code} must agree between the lowered arm ({lowered}) \
and the opaque one ({opaque}); {:?} vs {:?}",
                codes(lowered, D),
                codes(opaque, D)
            );
        }
    }
}

/// #1266 — the same collapse covered the arm **patterns**, which the
/// canonical single-braced `{pat body …}` block also carries literally.
///
/// tclsh 9.0.4: `proc f {z} { switch -glob $z { $a* { puts hit } default {
/// puts miss } } }` prints `hit` for `f {$a}` and `miss` for `f zz`, with `a`
/// undefined throughout — the pattern matched the two literal characters
/// `$a`. Supplied as separate words the pattern *does* substitute, and the
/// lowered `-exact` path already classified its patterns this way, so this is
/// the same lowered-vs-opaque parity gap.
#[test]
fn issue_1266_braced_switch_pattern_is_not_a_read() {
    for src in [
        "proc f {z} { switch -glob $z { $a* { } } }\n",
        "proc f {z} { switch -regexp $z { $a.* { } } }\n",
        "proc f {z} { switch $z { $a - b { } } }\n",
    ] {
        assert!(
            !fires(src, D, "W210"),
            "#1266: a pattern in the braced arm block is literal; {src} emitted: {:?}",
            codes(src, D)
        );
    }
    // TP — patterns supplied as separate words really do substitute.
    let words = "proc f {z} { switch -glob -- $z $a* { } }\n";
    assert!(
        fires(words, D, "W210"),
        "#1266 TP: a separate-word pattern substitutes; emitted: {:?}",
        codes(words, D)
    );
    // TN guard rail — the quoted pattern still keeps the store live.
    let live = "proc p {z} { set x 1; switch -glob $z { $x* { } } }\n";
    assert!(
        !fires(live, D, "W211") && !fires(live, D, "W220"),
        "#1266: a quoted pattern mention keeps the store live; emitted: {:?}",
        codes(live, D)
    );
}

// Issue #923 audit idx 24 / issue #1019 — a variable assigned indirectly in
// an outer frame by `uplevel`.
//
// Every claim below is pinned on tclsh 9.0.4 and 8.6.16, byte-identical.
// The three scripts print `99`, so the reads are not read-before-set:
//
//   proc setInCaller {var} { uplevel 1 [list set $var 99] }
//   proc setInCallerPlain {var} { uplevel 1 set $var 99 }
//   proc setUp2 {var} { uplevel 2 [list set $var 99] }
//
// driven respectively by `proc useIt {} {setInCaller answer; return $answer}`,
// the same with `setInCallerPlain`, and by the two-frame chain
// `proc middle {} {setUp2 answer}` / `proc outer {} {middle; return $answer}`.

/// FP — the one-hop list-built spelling. Already silent before idx 24's fix;
/// pinned here because nothing else locks it in.
#[test]
fn idx24_uplevel_one_list_built_set_is_not_read_before_set() {
    let src = "\
proc setInCaller {var} {
    uplevel 1 [list set $var 99]
}
proc useIt {} {
    setInCaller answer
    return $answer
}
";
    assert!(
        !fires(src, D, "W210"),
        "idx 24: `uplevel 1 [list set $var …]` defines the caller's variable; emitted: {:?}",
        codes(src, D)
    );
}

/// FP — the one-hop word-concatenated spelling (`uplevel 1 set $var 99`),
/// which `uplevel` joins into the same script.
#[test]
fn idx24_uplevel_one_concatenated_set_is_not_read_before_set() {
    let src = "\
proc setInCaller {var} {
    uplevel 1 set $var 99
}
proc useIt {} {
    setInCaller answer
    return $answer
}
";
    assert!(
        !fires(src, D, "W210"),
        "idx 24: `uplevel 1 set $var …` defines the caller's variable; emitted: {:?}",
        codes(src, D)
    );
}

/// FP — the multi-frame shape idx 24 was still broken on: the writing proc is
/// reached through an *ordinary* call, and its `uplevel 2` lands in that
/// caller's caller.
#[test]
fn idx24_uplevel_two_reaches_through_a_plain_call() {
    let src = "\
proc setUp2 {var} {
    uplevel 2 [list set $var 99]
}
proc middle {} {
    setUp2 answer
}
proc outer {} {
    middle
    return $answer
}
";
    assert!(
        !fires(src, D, "W210"),
        "idx 24: `uplevel 2` writes the plain caller's caller; emitted: {:?}",
        codes(src, D)
    );
}

/// FP — the same, with a literal braced body instead of a built list. The
/// `UpFrame` arm used to drop every level but 1 outright.
#[test]
fn idx24_uplevel_two_literal_body_reaches_through_a_plain_call() {
    let src = "\
proc setUp2 {} {
    uplevel 2 {set answer 99}
}
proc middle {} {
    setUp2
}
proc outer {} {
    middle
    return $answer
}
";
    assert!(
        !fires(src, D, "W210"),
        "idx 24: a literal `uplevel 2` body writes the plain caller's caller; emitted: {:?}",
        codes(src, D)
    );
}

/// FP — a level word nothing static can place (`uplevel [expr {[info level] -
/// $lvl}] …`, the real corpus spelling) is the same unknown-frame case.
#[test]
fn idx24_uplevel_with_a_computed_level_is_not_read_before_set() {
    let src = "\
proc setInCaller {var lvl} {
    uplevel [expr {[info level] - $lvl}] [list set $var 99]
}
proc useIt {} {
    setInCaller answer 1
    return $answer
}
";
    assert!(
        !fires(src, D, "W210"),
        "idx 24: a computed uplevel level cannot be placed, so widen; emitted: {:?}",
        codes(src, D)
    );
}

/// TP control — with the `uplevel` gone, nothing assigns the caller's
/// variable and W210 must still fire. Without this the FP arms above would
/// pass just as well against a W210 that had been switched off.
#[test]
fn idx24_tp_without_the_uplevel_the_read_is_still_unset() {
    let src = "\
proc setInCaller {var} {
    set $var 99
}
proc useIt {} {
    setInCaller answer
    return $answer
}
";
    assert!(
        fires(src, D, "W210"),
        "idx 24 TP: a callee-local `set` never reaches the caller; emitted: {:?}",
        codes(src, D)
    );
}

/// TP control — a **level-1** effect is not transitive through a plain call.
/// tclsh 9.0.4 / 8.6.16: `real_worker`'s `upvar 1` reaches `wrapper`'s frame,
/// so the outermost caller really does get `can't read "a": no such
/// variable`. This is the guard rail that keeps the new one-hop composition
/// from degenerating into "any caller of any frame-crossing proc is silent".
#[test]
fn idx24_tp_level_one_does_not_propagate_through_a_plain_call() {
    let src = "\
proc worker {v} {
    upvar 1 $v x
    set x 1
}
proc wrapper {v} {
    worker $v
}
proc caller {} {
    wrapper a
    return $a
}
";
    assert!(
        fires(src, D, "W210"),
        "idx 24 TP: a level-1 upvar stops at the wrapper's own frame; emitted: {:?}",
        codes(src, D)
    );
}

/// TP control — `uplevel #0` is the global frame and `uplevel 0` the callee's
/// own, so neither reaches a caller and neither may silence the read.
#[test]
fn idx24_tp_global_and_own_frame_levels_do_not_silence_the_caller() {
    for level in ["#0", "0"] {
        let src = format!(
            "proc setSomewhere {{var}} {{\n    uplevel {level} [list set $var 99]\n}}\n\
             proc useIt {{}} {{\n    setSomewhere answer\n    return $answer\n}}\n"
        );
        assert!(
            fires(&src, D, "W210"),
            "idx 24 TP: `uplevel {level}` never touches the caller's frame; emitted: {:?}",
            codes(&src, D)
        );
    }
}

/// FP — W212 must not read the literal word `set` inside a `[list …]`-built
/// script as a directly-written `set $var` name/value confusion. The
/// substitution happens in the *building* frame and is the whole idiom.
#[test]
fn idx24_w212_does_not_fire_on_a_list_built_variable_name_word() {
    let src = "proc setInCaller {var} {\n    uplevel 1 [list set $var 99]\n}\n";
    assert!(
        !fires(src, D, "W212"),
        "idx 24: a list-built name word is pre-substituted, not a confusion; emitted: {:?}",
        codes(src, D)
    );
}

/// TP control — the directly-written spelling is exactly what W212 is for,
/// and still fires.
#[test]
fn idx24_tp_w212_still_fires_on_a_directly_written_name_word() {
    for src in [
        "proc f {var} { set $var 1 }\n",
        "proc f {} { set counter 1\n incr $countr }\n",
    ] {
        assert!(
            fires(src, D, "W212"),
            "idx 24 TP: a written `$` in a name position is still W212; {src} emitted: {:?}",
            codes(src, D)
        );
    }
}

/// TN guard rail (#1237 / #1260 family) — widening a caller's frame must not
/// cost a variable its liveness: a store the `uplevel`-written name later
/// feeds must not resurface as a dead store (W220) or an unused variable
/// (W211).
#[test]
fn idx24_widening_keeps_the_callers_own_stores_live() {
    let src = "\
proc setUp2 {var} {
    uplevel 2 [list set $var 99]
}
proc middle {} {
    setUp2 answer
}
proc outer {} {
    set answer 0
    middle
    return $answer
}
";
    assert!(
        !fires(src, D, "W220") && !fires(src, D, "W211"),
        "idx 24: the seeded store is read after the widened call; emitted: {:?}",
        codes(src, D)
    );
}

/// FP — the same W212 carve-out on the canonical caller-frame-injection
/// idiom the `tclopt` corpus is built from (issue #923 audit idx 38): a
/// `[list set $varName $value]` built inside a `foreach`, where **both**
/// words are substitutions.
///
/// Oracle (tclsh 9.0.4 and 8.6.16): the script below prints `1 1 1`, so all
/// three names really are materialised in `Qfrac2`'s frame.
#[test]
fn idx38_w212_does_not_fire_on_a_list_built_set_of_a_looped_name() {
    let src = "\
proc NewArrays {varNames value} {
    foreach varName $varNames {
        uplevel 1 [list set $varName $value]
    }
}
proc Qfrac2 {} {
    NewArrays {rdiagArray acnormArray waArray} 1
    return [list $rdiagArray $acnormArray $waArray]
}
";
    assert!(
        !fires(src, D, "W212"),
        "idx 38: `[list set $varName $value]` is the idiom, not a confusion; emitted: {:?}",
        codes(src, D)
    );
    assert!(
        !fires(src, D, "W210"),
        "idx 38: the three names are materialised in the caller's frame; emitted: {:?}",
        codes(src, D)
    );
}

/// TP control for the shape above — with the `NewArrays` call removed
/// nothing materialises the names and all three reads are genuinely unset
/// (tclsh: `can't read "rdiagArray": no such variable`).
#[test]
fn idx38_tp_without_the_injection_call_the_reads_are_unset() {
    let src = "\
proc Qfrac2 {} {
    return [list $rdiagArray $acnormArray $waArray]
}
";
    assert!(
        fires(src, D, "W210"),
        "idx 38 TP: nothing writes those names; emitted: {:?}",
        codes(src, D)
    );
}

/// TP control for the W212 carve-out itself, in the *same* proc shape: a
/// `set $varName $value` written directly into the loop body — no `list`
/// builder, no deferred script — is exactly the name/value confusion the
/// code exists for and must still fire.
#[test]
fn idx38_tp_w212_still_fires_on_a_directly_written_looped_name() {
    let src = "\
proc NewArrays {varNames value} {
    foreach varName $varNames {
        set $varName $value
    }
}
";
    assert!(
        fires(src, D, "W212"),
        "idx 38 TP: a written `$` in a name position is still W212; emitted: {:?}",
        codes(src, D)
    );
}

// FP-RBS-21 — a barriered structured loop still binds its literal loop
// variables (issue #1380)
//
// A structured loop whose words carry a `{*}` expansion cannot be lowered to
// an `IRForeach` — the argv shape is unknowable — so it stays a
// `Statement::Barrier`.  Its body still runs in the caller's frame and is
// scanned for reads, so without the barrier contributing its
// `ArgRole::LoopVarList` names as defs, every read of a loop variable inside
// that body read as read-before-set.
//
// `foreach` showed this before #1380 (it was one of the nine names the old
// expansion barrier knew); `lmap` / `dict for` / `array for` / `foreachLine`
// were silent only because they fabricated an `IRForeach` over the
// *un-expanded* words — the wrong IR, which the same issue removed.
//
// Oracle (tclsh 9.0.4, identical in 8.6.16 — `{*}` exists from 8.5):
//   set spec {{a b}}
//   foreach i {*}$spec {puts $i}          → a
//                                           b
//   lmap i {*}$spec {string toupper $i}   → A B

#[test]
fn fp_rbs_21_expanded_loops_bind_their_literal_loop_variables() {
    for src in [
        "set spec {{a b}}\nforeach i {*}$spec {puts $i}\n",
        "set spec {{a b}}\nlmap i {*}$spec {puts $i}\n",
        "set spec {{a 1}}\ndict for {k v} {*}$spec {puts $k$v}\n",
    ] {
        assert!(
            !fires(src, D, "W210"),
            "FP-RBS-21: a barriered loop still binds its literal loop vars; \
             src {src:?} emitted: {:?}",
            codes(src, D)
        );
    }
}

#[test]
fn fp_rbs_21_tp_a_genuinely_unset_read_in_an_expanded_loop_body_still_fires() {
    // TP control: binding the loop variable must not silence an unrelated
    // undefined name in the same body.
    let src = "set spec {{a b}}\nlmap i {*}$spec {puts $nope}\n";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-21 TP: an unset non-loop name still fires; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_21_tp_unexpanded_loops_are_unchanged() {
    // TP control: the ordinary (lowered) forms keep both halves — the loop
    // variable is bound, an unrelated undefined name still fires.
    let quiet = "foreach i {a b} {puts $i}\n";
    assert!(
        !fires(quiet, D, "W210"),
        "FP-RBS-21 TP: a plain foreach binds its loop var; emitted: {:?}",
        codes(quiet, D)
    );
    let loud = "foreach i {a b} {puts $nope}\n";
    assert!(
        fires(loud, D, "W210"),
        "FP-RBS-21 TP: a plain foreach still reports an unset read; emitted: {:?}",
        codes(loud, D)
    );
}

// FP-RBS-21b — a *brace-quoted* var-list word binds the names it literally
// spells, `$` and `[` included.
//
// The barrier harvest used to test the reconstructed word text for `$` / `[`,
// which conflates "the word's value contains a dollar" with "the word
// substitutes".  A braced word substitutes nothing, so `{$x}` is a
// one-element list naming the variable `$x` — a legal Tcl name that the byte
// test dropped from the def set (PR #1481 review of issue #1380).
//
// Oracle (tclsh 8.6.16 and 9.0.4 both print `1`):
//   set spec {{1}}
//   foreach {{$x}} {*}$spec {puts ${$x}}   → 1

#[test]
fn fp_rbs_21b_braced_var_list_binds_a_literal_dollar_name() {
    let src = "set spec {{1}}\nforeach {{$x}} {*}$spec {puts ${$x}}\n";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-21b: a braced var-list word binds the name it literally spells; \
         emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_21b_tp_a_substituted_var_list_still_harvests_nothing() {
    // TP control: a var-list word that genuinely substitutes names nothing
    // statically, so a body read of the bound name still reports.
    let src = "set spec {{1}}\nset names {x}\nforeach $names {*}$spec {puts $x}\n";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-21b TP: a substituted var-list contributes no def; emitted: {:?}",
        codes(src, D)
    );
}
