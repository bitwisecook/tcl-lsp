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

//! Read-before-set (W210), dead stores (W220), variables in command position (W307) and unused or unset variables, with the control flow that decides them.

use super::*;

// W210: read of a possibly-unset variable.

#[test]
fn w210_read_before_set_on_path_merge() {
    // `y` is defined only inside the `if` arm; the read at the merge point may
    // see it unset.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc p {x} {\n    if {$x} {\n        set y 1\n    }\n    return $y\n}\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(has_code(&diags, "W210"), "{:?}", codes(&diags));
    // The diagnostic anchors on the read site (`return $y`), not the def.
    assert!(on_line(&diags, "W210").contains(&4));
}

#[test]
fn w210_use_after_unset() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set a 1\nunset a\nputs $a\n");
    assert!(has_code(&diags, "W210"), "{:?}", codes(&diags));
    assert!(on_line(&diags, "W210").contains(&2));
}

#[test]
fn w210_silent_when_set_on_all_paths() {
    // Defined on both branches → no read-before-set (must-stay-silent).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc p {x} {\n    if {$x} {\n        set y 1\n    } else {\n        set y 2\n    }\n    return $y\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

// A helper proc's `upvar` write, reached only through a `while`/`if` loop
// condition rather than a bare statement, must be visible to W210 — not
// just calls to the hardcoded builtins (`catch`/`scan`/`gets`/`regexp`).
// The tcllib idiom is `modules/cmdline/cmdline.tcl`'s `getopt`/`getKnownOpt`
// chain, `while {[set err [getopt argv $opts opt arg]]} { ... }`. tclsh
// 9.0/8.6 confirm the condition's own command substitution (including the
// upvar write) completes before the guarded body ever runs.

#[test]
fn w210_silent_for_upvar_proc_call_in_while_condition_923_idx122() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc getopt {ovar} {\n    upvar 1 $ovar opt\n    set opt value\n    return 1\n}\nwhile {[getopt opt]} {\n    puts $opt\n    break\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

#[test]
fn w210_silent_for_upvar_proc_wrapped_inside_a_while_condition_923_idx122() {
    // The exact tcllib shape: the upvar-writing call sits one bracket
    // deeper than the condition's own outermost command (`set err
    // [getopt ...]`, not a bare `[getopt ...]`).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc getopt {ovar} {\n    upvar 1 $ovar opt\n    set opt value\n    return 1\n}\nwhile {[set err [getopt opt]]} {\n    puts $opt\n    break\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

#[test]
fn w210_silent_for_upvar_proc_call_in_if_condition_923_idx122() {
    // The `if`-conditioned analogue — never "frozen" (only `for`/`while`
    // have a command-substitution-condition opaque-barrier path), so this
    // exercises the ordinary `lower_if` → `condition_out_vars` route
    // rather than the frozen-loop synthetic `<cond>` Call.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc getopt {ovar} {\n    upvar 1 $ovar opt\n    set opt value\n    return 1\n}\nif {[getopt opt]} {\n    puts $opt\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

#[test]
fn w210_still_fires_for_a_genuinely_dynamic_condition_call_923_idx122() {
    // A condition call to a proc that does *not* upvar-write its argument
    // must still warn: the check must not blanket-suppress every
    // `$var`-in-condition read, only ones a known upvar proc actually
    // populates. The proc returns 1: tclsh 8.4 to 9.1 error on the empty
    // result (`expected boolean value but got ""`) before `$opt` is read.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc doesNothing {ovar} {\n    puts \"called with $ovar\"\n    return 1\n}\nwhile {[doesNothing opt]} {\n    puts $opt\n    break\n}\n";
    assert!(has_code(&lsp.open_ready(&uri, src), "W210"));
}

// A wrapper proc that reaches an `upvar`+`uplevel` "custom control
// structure" proc through a *plain* call (not `uplevel`) does NOT propagate
// the effect to its own caller — tclsh 9.0/8.6 confirm `can't read "myf":
// no such variable` when the caller reads the variable outside any
// `uplevel`'d script argument. Reading the variable *inside* the same
// script block that wrote it is not a valid test of this, since that
// coincidentally lands in the same frame regardless of whether real
// propagation happened. The real tcllib idiom (`page::util::flow`) reaches
// its worker via `uplevel 1 [list ...]`, which genuinely does propagate one
// frame further (also tclsh 9.0-verified); soundly modelling that shape is
// not attempted here.

#[test]
fn w210_silent_for_a_plain_call_wrapper_923_idx18() {
    // tclsh 8.6 / 9.0 print `f=1 n=2`: the wrapper passes its own names
    // through, `real_worker` upvars and sets them in the wrapper's frame,
    // and `uplevel 1` runs the script there.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc real_worker {fvar nvar script} {\n    upvar 1 $fvar f\n    upvar 1 $nvar n\n    set f 1\n    set n 2\n    uplevel 1 $script\n}\nproc wrapper {fvar nvar script} {\n    real_worker $fvar $nvar $script\n}\nwrapper myf myn {\n    puts \"f=$myf n=$myn\"\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

#[test]
fn w210_silent_across_a_two_hop_plain_call_wrapper_chain_923_idx18() {
    // tclsh 8.6 / 9.0 print `f=1 n=2`, as for the single wrapper.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc real_worker {fvar nvar script} {\n    upvar 1 $fvar f\n    upvar 1 $nvar n\n    set f 1\n    set n 2\n    uplevel 1 $script\n}\nproc middle_wrapper {fvar nvar script} {\n    real_worker $fvar $nvar $script\n}\nproc outer_wrapper {fvar nvar script} {\n    middle_wrapper $fvar $nvar $script\n}\nouter_wrapper myf myn {\n    puts \"f=$myf n=$myn\"\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

#[test]
fn w210_still_fires_when_wrapper_does_not_pass_its_own_params_through_923_idx18() {
    // TN — `unrelated_wrapper` calls the known upvar proc `real_worker`
    // with literal args ("x"/"y"), not its own parameters passed through
    // unchanged. tclsh9.0/8.6-verified this genuinely errors ("can't read
    // \"myf\": no such variable"), so it must still warn.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc real_worker {fvar nvar script} {\n    upvar 1 $fvar f\n    upvar 1 $nvar n\n    set f 1\n    set n 2\n    uplevel 1 $script\n}\nproc unrelated_wrapper {a b c} {\n    real_worker x y $c\n}\nunrelated_wrapper myf myn {\n    puts \"f=$myf n=$myn\"\n}\n";
    assert!(has_code(&lsp.open_ready(&uri, src), "W210"));
}

// W307: a variable used in command position.

#[test]
fn w307_known_literal_non_command_fires() {
    // `cmd` resolves to the literal `foo` which is not a command.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "proc p {} {\n    set cmd foo\n    $cmd bar\n}\n");
    assert!(has_code(&diags, "W307"), "{:?}", codes(&diags));
}

#[test]
fn w307_silent_for_opaque_dispatch_target() {
    // `$self` is an opaque parameter (a method-dispatch idiom); with no known
    // non-command literal value, W307 must not fire.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "proc p {self} {\n    $self configure -x 1\n}\n");
    assert!(!has_code(&diags, "W307"), "{:?}", codes(&diags));
}

// W220: dead store.

#[test]
fn w220_dead_store_fires() {
    // `x` is set, then overwritten before any read → the first store is dead.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc p {} {\n    set x 1\n    set x 2\n    return $x\n}\n",
    );
    assert!(has_code(&diags, "W220"), "{:?}", codes(&diags));
    assert!(on_line(&diags, "W220").contains(&1));
}

#[test]
fn w220_silent_when_value_is_read_between() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc p {} {\n    set x 1\n    puts $x\n    set x 2\n    return $x\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W220"));
}

#[test]
fn w220_silent_for_command_name_read_nested_in_dict_for_if() {
    // `$x` is read as the command name of `$x a $key`, nested inside
    // `if {$value}` inside `dict for`. `dict for` bodies are lowered into
    // real CFG blocks, so this read is visible and `set x set` does not
    // false-fire W220.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc demo {} {\n    set x set\n    set d [dict create a true b false c true]\n    dict for {key value} $d {\n        if {$value} {\n            $x a $key\n        }\n    }\n}\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(!has_code(&diags, "W220"), "{:?}", codes(&diags));
    assert!(!has_code(&diags, "W211"), "{:?}", codes(&diags));
}

#[test]
fn w220_dead_store_inside_dict_for_body_fires() {
    // A dead store *inside* a `dict for` body is a real W220, since the
    // body is lowered into real CFG blocks rather than treated as opaque.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc demo {d} {\n    dict for {k v} $d {\n        set tmp 1\n        set tmp 2\n        puts $tmp\n    }\n}\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(has_code(&diags, "W220"), "{:?}", codes(&diags));
    assert!(on_line(&diags, "W220").contains(&2));
}

// A value word's embedded `{…}` run that survives, as ordinary literal
// content, inside a double-quoted or bareword-concatenated source word,
// must not hide the `$var` substitutions inside it from W220. The tcllib
// idiom is `modules/htmlparse/htmlparse.tcl`'s `eval "$cmd {$vroot} {} {}
// \{$html\}"`. tclsh 9.0/8.6 confirm `{$vroot}` here is an ordinary
// substitution, exactly like the bare `$cmd` beside it.

#[test]
fn w220_silent_for_a_var_wrapped_in_braces_inside_a_double_quoted_value_923_idx125() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc demo {} {\n    set a AA\n    set b BB\n    set c CC\n    set s \"prefix {$a} suffix $b $c\"\n    puts $s\n}\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(!has_code(&diags, "W220"), "{:?}", codes(&diags));
}

#[test]
fn w220_still_fires_for_a_genuinely_brace_quoted_value_923_idx125() {
    // TN — the whole value is brace-quoted (`{$a}` as ONE word, not
    // embedded inside a larger double-quoted string): Tcl performs zero
    // substitution on it at all (tclsh9.0/8.6-verified `set s {$a}` stores
    // the literal two characters `$a`), so `a` is genuinely never read and
    // must still warn.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc demo {} {\n    set a 1\n    set s {$a}\n    puts $s\n}\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(has_code(&diags, "W220"), "{:?}", codes(&diags));
}

// `itcl::class` / `snit::widget` / `snit::type` / `snit::widgetadaptor`
// bodies carry the registry's `body_kind: Structural` classification, the
// same as `oo::class`, so their body argument is excluded as a separate
// definition scope rather than scanned as ordinary value text. Without
// that classification, the class/method body's own nested braces are
// mis-read as a single non-substituting brace-quoted word, which swallows
// every `$this` / instance-variable reference along with it.

#[test]
fn w210_silent_for_this_and_instance_vars_in_an_itcl_method_body_923_idx125() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "itcl::class C {\n    variable handler\n    common registry\n    method run {} {\n        $this configure\n        $handler process\n        return $registry\n    }\n}\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(!has_code(&diags, "W210"), "{:?}", codes(&diags));
}

#[test]
fn w210_silent_for_self_and_instance_vars_in_a_snit_widget_method_body_923_idx125() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "snit::widget mywidget {\n    variable helper\n    component inner\n    method draw {} {\n        $self configure -bg white\n        $inner render\n        $helper compute\n        return $win\n    }\n}\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(!has_code(&diags, "W210"), "{:?}", codes(&diags));
}

#[test]
fn uplevel_issue_837_body_is_silent_through_server() {
    // The recursed `uplevel` body is clean Tcl and must produce no
    // diagnostics through the packaged server.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc forgetXyce {} {\n    uplevel 1 {foreach nameSpc [namespace children ::Foo] {\n        namespace forget ${nameSpc}::*\n    }}\n}\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        diags.is_empty(),
        "issue #837 repro must be silent: {:?}",
        codes(&diags)
    );
}

#[test]
fn uplevel_unbraced_substituted_body_fires_w105_through_server() {
    // Now that `uplevel`'s body is registry-known, an unbraced substituted body
    // fires W105 end-to-end, exactly as `eval` does — guiding the user to brace it.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {} {\n    uplevel 1 \"puts $x\"\n}\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(has_code(&diags, "W105"), "{:?}", codes(&diags));
}

// Clean code stays clean (cross-family negative control).

#[test]
fn clean_dataflow_has_no_diagnostics() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc p {x} {\n    set d [dict create a 1]\n    if {[dict exists $d a]} {\n        return [dict get $d a]\n    }\n    return $x\n}\n";
    assert_eq!(lsp.open_ready(&uri, src), [] as [serde_json::Value; 0]);
}

// Tailcall ends straight-line flow (FP-RBS-13).

#[test]
fn tailcall_terminated_branch_silent() {
    // `tailcall g` returns from the proc, so `return $result` is reached only
    // via the else branch where `result` is set.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {cond} {\n    if {$cond} {\n        tailcall g\n    } else {\n        set result 1\n    }\n    return $result\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

#[test]
fn non_terminating_branch_still_fires() {
    // Replace the tailcall with a completing command → `result` is maybe-unset
    // on the then-path, so W210 must fire.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {cond} {\n    if {$cond} {\n        puts hi\n    } else {\n        set result 1\n    }\n    return $result\n}\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(has_code(&diags, "W210"), "{:?}", codes(&diags));
}

// A non-empty-literal foreach runs its body (FP-RBS-17).

#[test]
fn foreach_non_empty_literal_silent() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {} {\n    foreach x {1 2 3} { set y $x }\n    puts $y\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

#[test]
fn foreach_empty_literal_still_fires() {
    // An empty literal never runs the body → `y` is unset.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {} {\n    foreach x {} { set y $x }\n    puts $y\n}\n";
    assert!(has_code(&lsp.open_ready(&uri, src), "W210"));
}

// A for whose condition is true on entry runs its body (FP-RBS-18).

#[test]
fn for_true_on_entry_silent() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {} {\n    for {set i 0} {$i < 3} {incr i} { set y $i }\n    puts $y\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

#[test]
fn for_false_on_entry_still_fires() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {} {\n    for {set i 5} {$i < 3} {incr i} { set y $i }\n    puts $y\n}\n";
    assert!(has_code(&lsp.open_ready(&uri, src), "W210"));
}

// while 1 only exits via break, where the var is set (FP-RBS-16).

#[test]
fn while1_break_set_silent() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {} {\n    while 1 { set y 1; break }\n    puts $y\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

#[test]
fn normal_while_body_defined_silent() {
    // FP-RBS-19: a non-constant `while` may run zero times, but its body
    // unconditionally sets `y`, so a read after the loop is defined whenever the
    // loop ran. Matching C Tcl (which errors only when the condition is false on
    // entry at runtime), we assume a may-run loop runs — no W210.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {n} {\n    while {$n > 0} { set y 1; incr n -1 }\n    puts $y\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

#[test]
fn foreach_accumulator_after_loop_silent() {
    // FP-RBS-19: a `lappend` accumulator built inside a dynamic multi-group
    // `foreach`, read after the loop. The body defines the accumulators on
    // every iteration, so the after-loop reads are not read-before-set.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "set data [getDataDict]\n\
        foreach time [dict get $data time] vout [dict get $data osc_out] {\n\
        \x20   lappend timeVout [list $time $vout]\n\
        }\n\
        puts $timeVout\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

#[test]
fn foreach_empty_literal_after_loop_still_fires() {
    // TP control: a provably-empty literal list runs zero times, so tclsh always
    // errors — the after-loop read must keep firing.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {} {\n    foreach x {} { lappend acc $x }\n    puts $acc\n}\n";
    assert!(has_code(&lsp.open_ready(&uri, src), "W210"));
}

// An opaque switch whose every arm exits is a terminator (FP-RBS-15).

#[test]
fn all_arms_return_makes_trailing_read_unreachable() {
    // Every arm returns, so `puts $y` is unreachable dead code → no W210.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {x} {\n    switch -glob -- $x { a* { return 1 } default { return 2 } }\n    puts $y\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

#[test]
fn no_default_switch_falls_through_fires() {
    // Without a `default` an unmatched subject falls through to the read, so the
    // switch is not a terminator and `y` is maybe-unset.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {x} {\n    switch -glob -- $x { a* { return 1 } b* { return 2 } }\n    puts $y\n}\n";
    assert!(has_code(&lsp.open_ready(&uri, src), "W210"));
}

// -- opaque switch must-define excludes non-completing arms (FP-RBS-14) --

#[test]
fn returning_arm_excluded_from_must_define() {
    // The `a*` arm returns before reaching `puts $y`; the only path that does
    // (default) sets `y` → definitely defined, no W210.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {x} {\n    switch -glob -- $x { a* { return 0 } default { set y 2 } }\n    puts $y\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

#[test]
fn break_arm_escaping_loop_still_fires() {
    // `break`/`continue` are loop-jumps, not proc-exits, so a break arm does
    // NOT define the other arm's var on the path that escapes the loop —
    // `y` is maybe-unset, W210 must fire.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {} {\n    foreach x {a} { switch -glob -- $x { a* { break } default { set y 1 } } }\n    puts $y\n}\n";
    assert!(has_code(&lsp.open_ready(&uri, src), "W210"));
}

// Omitted-arg call-site constants are poisoned (interproc).

#[test]
fn omitted_default_arg_does_not_hide_read_before_set() {
    // `p` (slot 0 omitted → default `x == 0`) leaves `y` unset, so `puts $y` is
    // a real read-before-set; the literal `1` passed by `p 1` must not be bound
    // as a constant for `x`.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc p {{x 0}} {\n    if {$x} {\n        set y 5\n    }\n    puts $y\n}\np\np 1\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(has_code(&diags, "W210"), "{:?}", codes(&diags));
    assert!(on_line(&diags, "W210").contains(&4));
}

#[test]
fn uniform_literal_arg_still_binds_silent() {
    // Every caller passes `1` at slot 0 → the constant binding holds, the
    // `if {$x}` body is provably taken, `y` is always set: stay silent.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc q {{x 0}} {\n    if {$x} {\n        set y 5\n    }\n    puts $y\n}\nq 1\nq 1\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

// regexp `-expanded` is not unconditionally literal-safe.

#[test]
fn regexp_expanded_whitespace_pattern_silent() {
    // `-expanded` ignores unescaped whitespace, so `{a b}` matches the substring
    // `ab` and writes `v`; reading `v` must not fire W210.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc g {input} {\n    regexp -expanded {a b} $input v\n    puts $v\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

#[test]
fn regexp_expanded_clean_literal_still_fires() {
    // A whitespace/`#`-free literal stays provably literal: `{x}` never matches
    // `X`, never writes `w` → reading `w` is read-before-set.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc g {} {\n    regexp -expanded {x} X w\n    puts $w\n}\n";
    assert!(has_code(&lsp.open_ready(&uri, src), "W210"));
}

// A try body throw keeps its handler exception edge.

#[test]
fn try_body_throw_keeps_handler_defs_silent() {
    // `x` is set before the only throw, so the handler always sees it (tclsh
    // prints `1`): stay silent.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {} {\n    try {\n        set x 1\n        error boom\n    } on error {} {\n        puts $x\n    }\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

#[test]
fn try_body_earlier_conditional_throw_fires() {
    // The handler is reachable from every throw point; `x` is unset on the
    // earlier `if {$c} {error a}` path, so the read is maybe-unset.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {c} {\n    try {\n        if {$c} { error a }\n        set x 1\n        error b\n    } on error {} {\n        puts $x\n    }\n}\n";
    assert!(has_code(&lsp.open_ready(&uri, src), "W210"));
}

/// TN — an unmatched literal `{` inside a *double-quoted* string does not
/// stop substitution, so the `$ns` beside it is a genuine read and the
/// `set` before it is not a dead store.
///
/// tclsh 8.6.16 and 9.0.4: `set ns "ctx"; puts "{ $ns"` prints `{ ctx` —
/// the variable really is read. This is the exact shape `pix`'s
/// `pixdoc.tcl` uses to emit generated `namespace eval` preambles. The
/// analyser's own `fires()` helper (FP-DS-13) pins this; this test drives
/// it through the server's publish path as well.
#[test]
fn an_unmatched_brace_in_a_quoted_string_keeps_the_read_923_idx64() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set ns \"ctx\"\nputs \"{ $ns\"\n");
    let diags = lsp.pull_diagnostics(&uri);
    assert!(
        !has_code(&diags, "W220"),
        "`$ns` after an unmatched `{{` in a quoted string is a real read, got: {:?}",
        codes(&diags),
    );

    // The fuller shape the finding came from: an if/elseif namespace remap
    // feeding a generated `namespace eval` preamble.
    let uri2 = unique_uri("tcl");
    lsp.open_ready(
        &uri2,
        "set fp [open /tmp/genned.tcl w]\n\
         foreach name {paths image} {\n\
         \x20   if {$name eq \"paths\"} { set ns path } elseif {$name eq \"image\"} \
         { set ns img } else { set ns other }\n\
         \x20   set preamble \"Docs for $name\"\n\
         \x20   puts $fp \"namespace eval ::pix {\\n    namespace eval $ns {\\n\
         \x20       variable _ruff_preamble $preamble\\n    }\\n}\"\n\
         }\n\
         close $fp\n",
    );
    let diags2 = lsp.pull_diagnostics(&uri2);
    assert!(
        !has_code(&diags2, "W220"),
        "the generated-preamble idiom must draw no dead-store warning, got: {:?}",
        codes(&diags2),
    );
}

#[test]
fn w300_w103_silent_when_var_provably_literal() {
    // A `$var` proven to hold a compile-time literal path/filename is a known
    // constant — no more dangerous than writing it inline — so W300/W103 are
    // suppressed. A bare parameter stays flagged (the TP control).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let silent = lsp.open_ready(
        &uri,
        "set p \"./lib.tcl\"\nsource $p\nset f \"data.txt\"\nopen $f\n",
    );
    assert!(!has_code(&silent, "W300"), "W300 FP: {silent:?}");
    assert!(!has_code(&silent, "W103"), "W103 FP: {silent:?}");

    let uri2 = unique_uri("tcl");
    let fires = lsp.open_ready(&uri2, "proc f {p} { source $p }\nproc g {f} { open $f }\n");
    assert!(has_code(&fires, "W300"), "W300 TP (param): {fires:?}");
    assert!(has_code(&fires, "W103"), "W103 TP (param): {fires:?}");
}

#[test]
fn w213_unset_narrows_to_variable_and_carries_fix() {
    // `unset xs` on a possibly-undefined var → W213 squiggled on `xs` (col
    // 20..22 of the proc body), carrying the `-nocomplain` insert fix.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "proc foo {} { unset xs }\n");
    let w213: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("W213"))
        .collect();
    assert_eq!(w213.len(), 1, "expected one W213: {diags:?}");
    assert_eq!(
        diag_range(w213[0]),
        ((0, 20), (0, 22)),
        "W213 must span only `xs`"
    );
}

#[test]
fn w214_unused_param_anchors_on_the_param_name() {
    // The squiggle must cover only the offending parameter's *name* (`unused`),
    // not the whole `proc` definition, so it aligns with go-to-definition/rename.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "proc f {unused x} { puts $x }\n");
    let w214: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("W214"))
        .collect();
    assert_eq!(w214.len(), 1, "expected exactly one W214: {diags:?}");
    assert_eq!(
        diag_range(w214[0]),
        ((0, 8), (0, 14)),
        "W214 must span only `unused`"
    );
}

#[test]
fn w214_two_unused_params_get_separate_tight_ranges() {
    // Two unused params must yield two squiggles, each on its own name — not two
    // diagnostics stacked on the whole proc definition.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "proc g {aa bb} { return 0 }\n");
    let ranges: Vec<((i64, i64), (i64, i64))> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("W214"))
        .map(diag_range)
        .collect();
    assert!(ranges.contains(&((0, 8), (0, 10))), "aa range: {ranges:?}");
    assert!(ranges.contains(&((0, 11), (0, 13))), "bb range: {ranges:?}");
}

#[test]
fn w214_nested_proc_names_its_defining_namespace_and_anchors_on_the_param() {
    // `proc a::outer` runs its body in `::a` (the namespace it is
    // *defined* in), so the `proc helper` it executes creates `::a::helper` —
    // NOT the lexical `::helper`. tclsh 9.0.4 / 8.6.16 agree:
    //
    //   namespace eval ::a {}
    //   proc a::outer {} { proc helper {x} {return $x} ; return [namespace current] }
    //   a::outer                  ;# -> ::a
    //   info commands ::helper    ;# -> {}
    //   info commands ::a::helper ;# -> ::a::helper
    //
    // Lowering must home `helper` to `::a::helper`, not lexically to
    // `::helper`, and the per-parameter name-span lookup (keyed on that FQN
    // against the analyser's `all_procs`) must find it — otherwise the
    // squiggle falls back to the whole `proc` definition. Both halves are
    // asserted here.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src =
        "namespace eval ::a {}\nproc a::outer {} {\n    proc helper {unusedp} { return 1 }\n}\n";
    let diags = lsp.open_ready(&uri, src);
    let w214: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("W214"))
        .collect();
    assert_eq!(w214.len(), 1, "expected exactly one W214: {diags:?}");
    let message = w214[0]["message"].as_str().unwrap_or_default();
    assert!(
        message.contains("'::a::helper'"),
        "W214 must name the defining-namespace FQN, got {message:?}",
    );
    // Line 2 (0-based), `    proc helper {unusedp} …` — `unusedp` starts at
    // column 17 and ends at 24.
    assert_eq!(
        diag_range(w214[0]),
        ((2, 17), (2, 24)),
        "W214 must span only `unusedp`, not the whole nested definition",
    );
}
