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

//! FP-RBS-01, 02, 06, 12 and 20: `info exists` guards and the variables a command substitution or a condition writes.

use super::*;

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
    // A known error handler does not write an unrelated caller variable.
    // An unknown `operation` could use uplevel to initialise it before
    // throwing, so that separate May case cannot donate an undef finding.
    let src = "\
proc f {} {
    if {[catch {error stop} err]} { puts \"failed: $other\" }
}
";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-02 TP: $other (not written by the catch) must still fire W210; emitted: {:?}",
        codes(src, D)
    );
    // #2427: a bare undefined command no longer widens the command table; an autoload index entry in view keeps this device unbounded.
    let unknown = "set ::auto_index(operation) {source lib.tcl}\nproc f {} {if {[catch {operation} err]} {puts $other}}";
    assert!(!fires(unknown, D, "W210"));
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
    // Unknown switches cannot establish output-variable writes.
    let src = "proc f {} { regexp -bogus {x} X v; puts $v }";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-02: an unrecognised switch must conservatively NOT fire W210; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_02_regexp_about_names_no_match_variable() {
    // TP: `-about` describes the pattern and returns before it looks at a
    // subject, ignoring every later word — so nothing writes `v` and the read
    // is genuine. Measured on tclsh 8.4.20 / 8.6.18 / 9.0.4, all three:
    //   proc f {} { regexp -about {x} y v; puts $v }; f
    //   → can't read "v": no such variable
    let src = "proc f {} { regexp -about {x} y v; puts $v }";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-02: `-about` writes no match variable, so this read is genuine; emitted: {:?}",
        codes(src, D)
    );
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

// Untagged: the dict-with precision gap.
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

// Untagged: the dispatch-protocol precision gap.
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

// Untagged: the call-by-name precision gap.
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

// FP-RBS-12 — regexp/scan output-var conditional defs reach both cases

const FP_RBS_12_REPRO: &str = "proc f {} { regexp {x} y -> v; if {1} { puts $v } }\n";

#[test]
fn fp_rbs_12_regexp_unconditional_read_after_no_match_fires() {
    // TP: regexp {x} vs literal y is no-match; if {1} always runs.
    assert!(
        fires(FP_RBS_12_REPRO, D, "W210"),
        "FP-RBS-12 TP: post-regexp unconditional read of $v must fire W210; emitted: {:?}",
        codes(FP_RBS_12_REPRO, D)
    );
}

#[test]
fn fp_rbs_12_regexp_in_negated_if_arm_fires() {
    // TP: if-arm executes when regexp returns 0 (no match); $v unset.
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

// FP-RBS-20 — a condition-embedded variable writer defines its target
//
// `condition_command_out_vars` asks the registry for the call's
// `ArgRole::VarWrite` positions, so every command the registry knows writes an
// out-var answers.  A hardcoded `catch` / `scan` / `gets` / `regexp` list
// instead reads the ubiquitous `if {[set VAR EXPR] rel N} {…$VAR…}` idiom as
// read-before-set.
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
    // catch / scan / gets / regexp must keep their recognition through the
    // registry query, without a hardcoded name list.
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

const SUBSTITUTED_COMMAND_HEAD_SOURCE: &str =
    "proc f {set} { if {[$set length foo]} { puts $length } }\n";

#[test]
fn fp_rbs_20_substituted_command_head_keeps_unknown_writers() {
    // The formal may select native string or a custom caller-cell setter.
    // Neither a definition nor an absent-variable warning follows from an
    // unresolved dispatch. Its spelling must not select native set.
    let src = SUBSTITUTED_COMMAND_HEAD_SOURCE;
    assert!(
        !fires(src, D, "W210"),
        "an unknown substituted command may initialise length; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn substituted_command_head_native_callers_have_distinct_cell_effects() {
    let script = format!(
        "{SUBSTITUTED_COMMAND_HEAD_SOURCE}\
proc initialize_length {{name value}} {{upvar 1 $name cell; set cell $value; return 1}}\n\
set status [catch {{f string}} result]\n\
puts [list original_string $status $result]\n\
set status [catch {{f initialize_length}} result]\n\
puts [list original_custom_setter $status $result]\n"
    );
    let mut references = tcl_test_support::available_tclshs()
        .into_iter()
        .map(|reference| reference.path)
        .collect::<Vec<_>>();
    let jim = if tcl_dialect::trace_env::var_os("TCL_LSP_REQUIRE_JIM_ORACLE").is_some() {
        Some(tcl_test_support::require_jimsh().expect("required current Jim"))
    } else {
        tcl_test_support::locate_jimsh().expect("valid Jim override")
    };
    if let Some(jim) = jim {
        references.push(jim.path);
    }
    assert!(!references.is_empty(), "configure the native oracle matrix");
    for reference in references {
        let result = tcl_test_support::run_script(&reference, script.as_bytes())
            .expect("native original caller script");
        assert!(result.success(), "{reference:?}: {result:?}");
        assert!(result.stderr.is_empty(), "{reference:?}: {result:?}");
        assert_eq!(
            result.stdout,
            b"original_string 1 {can't read \"length\": no such variable}\nfoo\noriginal_custom_setter 0 {}\n",
            "{reference:?}"
        );
    }
    assert!(fires(
        "proc f {} {if {[string length foo]} {puts $length}}",
        D,
        "W210"
    ));
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
