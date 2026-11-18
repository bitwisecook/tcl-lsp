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

//! Variable-flow checks: read before set, dead stores, unused parameters and variables, and the body scopes that must not raise them falsely.

// W214 — unused proc parameter (pure static heuristic), checks-specific angles.
//
// Empty-body signature stubs and dispatch-protocol params (≥3 peers sharing a
// signature *plus* a dispatcher) are recognised and suppressed; a `switch`
// subject counts as a use; genuinely-unused params still fire.
mod unused_proc_parameters {
    use super::super::*;

    fn w214_names(src: &str) -> std::collections::HashSet<String> {
        of_code(src, D, "W214")
            .into_iter()
            .filter_map(|(m, _, _)| m.split('\'').nth(1).map(str::to_string))
            .collect()
    }

    #[test]
    fn empty_body_stub_silences_all_params() {
        // tclsh: an empty-body proc is a valid placeholder returning "". Every
        // param is necessarily "unused" — flagging them is noise.
        assert!(w214_names("proc reverse {fa} {}\n").is_empty());
        assert!(w214_names("proc complete {fa {sink {}}} {}\n").is_empty());
    }

    #[test]
    fn real_body_unused_params_still_fire() {
        assert_eq!(
            w214_names("proc foo {x y} { puts hello }\n"),
            ["x", "y"].map(String::from).into()
        );
    }

    #[test]
    fn switch_subject_counts_as_use() {
        // A param read only as a `switch -- $col {…}` subject is used.
        let src = "proc editStartCmd {tbl row col text} {\n    switch -- $col {\n        1 { set combList columns }\n        default { return $text }\n    }\n    return $combList\n}\n";
        let names = w214_names(src);
        assert!(!names.contains("col"), "col is used as switch subject");
        // `row` (and `tbl`) are genuinely unused.
        assert!(names.contains("row"));
    }

    #[test]
    fn dispatch_protocol_with_dispatcher_suppresses() {
        // ≥3 peers sharing `{s e}` + a variable-command dispatcher (`walk`) →
        // protocol contract, params suppressed (tcllib pt::peg parser-rule shape).
        let src = "namespace eval ::g {\n    proc ALNUM {s e} { return alnum }\n    proc ALPHA {s e} { return alpha }\n    proc DIGIT {s e} { return digit }\n    proc walk {rule s e} { $rule $s $e }\n}\n";
        assert!(w214_names(src).is_empty());
    }

    #[test]
    fn two_peers_without_protocol_still_fire() {
        // Only 2 peers — not enough to infer a protocol → each unused param fires.
        let src = "namespace eval ::g {\n    proc rule_a {s e} { return alnum }\n    proc rule_b {s e} { return alpha }\n}\n";
        assert_eq!(w214_names(src), ["s", "e"].map(String::from).into());
    }

    #[test]
    fn three_peers_without_dispatcher_still_fire() {
        // Peer-count alone is insufficient: no dispatcher anywhere → just helpers.
        let src = "namespace eval ::n {\n    proc a {ctx token} { puts $ctx }\n    proc b {ctx token} { puts $ctx }\n    proc c {ctx token} { puts $ctx }\n}\n";
        assert_eq!(w214_names(src), ["token"].map(String::from).into());
    }

    /// The proc FQNs W214 names, for `src`.
    fn w214_procs(src: &str) -> std::collections::HashSet<String> {
        of_code(src, D, "W214")
            .into_iter()
            .filter_map(|(m, _, _)| m.split('\'').nth(3).map(str::to_string))
            .collect()
    }

    #[test]
    fn nested_proc_is_named_by_its_defining_namespace() {
        // Oracle (tclsh 9.0.4 / 8.6.16, identical): a proc body
        // runs in the namespace the proc is *defined* in, so `proc a::outer`'s
        // body creates `::a::helper`, not the lexical `::helper`.
        assert_eq!(
            w214_procs(
                "namespace eval ::a {}\nproc a::outer {} { proc helper {unusedp} { return 1 } }\n"
            ),
            ["::a::helper"].map(String::from).into(),
        );
        // The name word's own qualifier beats the enclosing `namespace eval`.
        assert_eq!(
            w214_procs(
                "namespace eval ::e {}\nnamespace eval ::d {\n    proc ::e::o3 {} { proc h3 {unusedz} { return 1 } }\n}\n"
            ),
            ["::e::h3"].map(String::from).into(),
        );
    }

    #[test]
    fn unqualified_and_absolute_nested_names_keep_their_fqn() {
        // TN pair — the shapes where lexical and defining namespace
        // already agreed must not move.
        assert_eq!(
            w214_procs("proc outer {} { proc inner {unusedq} { return 1 } }\n"),
            ["::inner"].map(String::from).into(),
        );
        assert_eq!(
            w214_procs(
                "namespace eval ::a {}\nproc a::outer4 {} { proc ::abs {unusedq} { return 1 } }\n"
            ),
            ["::abs"].map(String::from).into(),
        );
    }

    #[test]
    fn unique_signature_still_fires() {
        assert_eq!(
            w214_names("proc foo {x y unused} { puts $x; puts $y }\n"),
            ["unused"].map(String::from).into()
        );
    }
}

// W210 — read-before-set, checks-specific shapes that are handled correctly.
//
// These mirror tclsh ground truth and pass cleanly. (The catch-body case is
// pinned in `catch_body_defs_no_false_w210` below.)
mod read_before_set_correct {
    use super::super::*;

    fn w210_named(src: &str, var: &str) -> usize {
        of_code(src, D, "W210")
            .iter()
            .filter(|(m, _, _)| m.contains(var))
            .count()
    }

    #[test]
    fn info_exists_guard_is_not_read_before_set() {
        // tclsh: `info exists` on an unset var returns 0 (legal); it is the
        // canonical test-before-use idiom.
        assert!(!fires(
            "proc f {} { if {![info exists ns]} { set ns 1 }\n return $ns }",
            D,
            "W210"
        ));
        assert!(!fires(
            "proc f {} { set y [info exists x]\n return $y }",
            D,
            "W210"
        ));
        assert!(!fires(
            "proc f {} { if {[array exists arr]} { return [array size arr] }\n return 0 }",
            D,
            "W210"
        ));
    }

    #[test]
    fn return_reads_its_variables() {
        // tclsh: `return $x` with x unset errors `can't read "x"`.
        assert_eq!(count("proc f {} { return $x }", D, "W210"), 1);
        assert_eq!(count("proc f {} { return [expr {$x + 1}] }", D, "W210"), 1);
        // …but a set var / literal is fine.
        assert!(!fires("proc f {} { set x 1\n return $x }", D, "W210"));
        assert!(!fires("proc f {} { return 42 }", D, "W210"));
    }

    #[test]
    fn try_handler_versions_are_correct() {
        // on-error after a throw that set x sees it; a never-set var still fires.
        assert!(!fires(
            "proc f {} {\n    try {\n        set x 1\n        error boom\n    } on error {} {\n        puts $x\n    }\n}",
            D,
            "W210"
        ));
        assert_eq!(
            count(
                "proc f {} {\n    try {\n        error boom\n    } on error {} {\n        puts $y\n    }\n}",
                D,
                "W210"
            ),
            1
        );
        // An earlier conditional throw leaves x maybe-unset → still warns.
        assert_eq!(
            count(
                "proc f {c} {\n    try {\n        if {$c} { error a }\n        set x 1\n        error b\n    } on error {} {\n        puts $x\n    }\n}",
                D,
                "W210"
            ),
            1
        );
    }

    #[test]
    fn proc_written_global_suppresses_top_level_read() {
        // A proc declaring+setting a global makes a later top-level read safe
        // only when the proc may have run (via an earlier `source`). With no
        // call in between, tclsh 8.4-9.1 raise `can't read "package_name": no
        // such variable` (and likewise for `counter`), so W210 is warranted.
        assert_eq!(
            w210_named(
                "proc set_name {v} {global package_name; set package_name $v}\nsource foo.tcl\nset out $package_name\n",
                "package_name"
            ),
            0
        );
        assert_eq!(
            w210_named(
                "proc set_name {v} {set ::package_name $v}\nset out $package_name\n",
                "package_name"
            ),
            1
        );
        assert_eq!(
            w210_named(
                "proc bump {} {global counter; incr counter}\nputs $counter\n",
                "counter"
            ),
            1
        );
    }

    #[test]
    fn global_alias_without_write_and_unrelated_still_warn() {
        // `global X` with no set does not populate it.
        assert_eq!(
            w210_named(
                "proc reader {} {global package_name; return $package_name}\nset out $package_name\n",
                "package_name"
            ),
            1
        );
        // A proc writing a *different* global doesn't mask a real read.
        assert_eq!(
            w210_named(
                "proc set_name {v} {global package_name; set package_name $v}\nputs $other_var\n",
                "other_var"
            ),
            1
        );
    }

    #[test]
    fn genuine_local_read_before_set_warns() {
        assert_eq!(
            count(
                "proc f {} {\n    set kids $children\n    return $kids\n}",
                D,
                "W210"
            ),
            1
        );
        assert_eq!(count("puts $never_set", D, "W210"), 1);
    }

    #[test]
    fn qualified_variable_alias_not_read_before_set() {
        // `variable ::tcl::WordBreakRE` links the local alias by tail.
        assert!(!fires(
            "proc f {} {\n    variable ::tcl::WordBreakRE\n    regexp -indices -- $WordBreakRE(after) abc result\n    return $result\n}",
            D,
            "W210"
        ));
    }
}

// W210 — catch-body variable definitions suppress the false positive.
//
// tclsh ground truth (8.6 + 9.0): `if {![catch {set x 1}]} { puts $x }` prints
// `1` — the catch *body* defines `x`, so the read in the if-body is safe and
// W210 must NOT fire. Recognising only the `[catch {…} err]` *result-message*
// var (FP-RBS-02), and not a body-`set` of an *other* variable read after the
// catch, would false-fire W210. The command-substitution out-var recovery
// therefore also scans the catch body, and the read-before-set suppression
// scans branch conditions (not just `set x [expr …]` assignments), so these
// guard-safe reads stay unflagged.
pub(super) mod catch_body_defs_no_false_w210 {
    use super::super::*;

    #[test]
    fn catch_body_set_does_not_fire_w210() {
        // tclsh: the no-error branch ran the body to completion, so `$x` /
        // `$line` is set; W210 must NOT fire.
        assert_eq!(count("if {![catch {set x 1}]} { puts $x }", D, "W210"), 0);
        assert_eq!(
            count("if {![catch {gets stdin line}]} { puts $line }", D, "W210"),
            0
        );
    }

    #[test]
    fn genuine_read_before_set_still_warns() {
        // The control that holds either way: an unrelated unset var still fires.
        assert_eq!(count("puts $never_set", D, "W210"), 1);
    }
}

// Braced script data needs its own execution and binding context.
// Original C8.6.18/C9.0.4: these unknown-command operands read no unset variable.
mod undescribed_script_word_bindings_no_false_w210 {
    use super::super::*;

    /// The reported shape, reduced: one unknown command, one script word,
    /// three names bound three different ways.
    #[test]
    fn a_nested_set_a_loop_variable_and_a_catch_operand_are_not_read_before_set() {
        let src = "mycmd one {a case} -body {\n\
                   \x20   foreach it {1 2 3} { set last $it }\n\
                   \x20   catch {risky} err\n\
                   \x20   list $last $err $it\n\
                   }\n";
        assert_eq!(count(src, D, "W210"), 0, "{:?}", codes(src, D));
    }

    /// `tcltest`'s bare spelling is the one the reporter hit: the
    /// `tcltest::test` spelling lowers to a barrier and was always silent,
    /// while `test` — undescribed — took the word path.
    #[test]
    fn a_tcltest_body_is_silent_in_either_spelling() {
        let body = "-body {\n\
                    \x20   foreach it {1 2 3} { set last $it }\n\
                    \x20   list $last\n\
                    } -result 3\n";
        for head in ["test one {a test} ", "tcltest::test one {a test} "] {
            let src = format!("package require tcltest\n{head}{body}");
            assert_eq!(count(&src, D, "W210"), 0, "{:?}", codes(&src, D));
        }
    }

    /// The control: a name the word never binds is still a read of this
    /// frame, because an unknown command may `uplevel` the script.
    #[test]
    fn a_name_the_word_never_binds_still_warns() {
        // `mycmd` runs its script word in the caller's frame, so the read
        // happens: tclsh 8.6.18 / 9.0.4 raise `can't read "never_set"`.
        let src = "proc mycmd {args} {uplevel 1 [lindex $args end]}\n\
                   mycmd -body {\n\
                   \x20   foreach it {1 2 3} { set last $it }\n\
                   \x20   puts $never_set\n\
                   }\n";
        assert_eq!(count(src, D, "W210"), 1, "{:?}", codes(src, D));
    }

    /// A bare undefined `mycmd` raises `invalid command name` under the
    /// stock `unknown` and never runs its script word (ruling #2427; tclsh
    /// 8.6.18 / 9.0.4), so nothing inside it is a read.
    #[test]
    fn an_undefined_command_never_runs_its_script_word() {
        let src = "mycmd -body {\n\
                   \x20   puts $never_set\n\
                   }\n";
        assert_eq!(count(src, D, "W210"), 0, "{:?}", codes(src, D));
    }
}

// W210 — `unset` of a global does NOT suppress a later read (FIXED).
//
// tclsh ground truth: `proc clear {} {unset ::x}` then top-level `puts $x`
// errors `can't read "x"` — `unset` destroys, it does not initialise — so a
// real W210 is warranted. `globals_written_by_procs` now excludes `unset`
// (it removes a variable, never assigns one, unlike `set`/`global`-then-`set`),
// so the warranted top-level W210 fires instead of being suppressed.
mod unset_global_does_not_suppress_w210 {
    use super::super::*;

    fn w210_named(src: &str, var: &str) -> usize {
        of_code(src, D, "W210")
            .iter()
            .filter(|(m, _, _)| m.contains(var))
            .count()
    }

    #[test]
    fn unset_only_global_still_fires_w210() {
        // tclsh: top-level `puts $x` errors `can't read "x": no such variable`
        // (proc isn't called; `unset` never initialises). W210 is warranted.
        assert_eq!(w210_named("proc clear {} {unset ::x}\nputs $x\n", "'x'"), 1);
        assert_eq!(
            w210_named("proc clear {} {global x; unset x}\nputs $x\n", "'x'"),
            1
        );
    }
}

// W220 / W211 — dead-store & unused, checks-specific read-recovery cases.
//
// Read-modify-write reads (incr/append/lappend), reads inside command-
// substituted exprs, and reads inside braced `eval {…}` bodies must be
// recovered so the feeding assignment is not falsely dead/unused. Array
// elements are distinguished by overlap. Genuine dead stores still fire.
mod dead_store_and_unused {
    use super::super::*;

    #[test]
    fn distinct_array_element_write_not_dead() {
        // `set a(k)` read by `puts $a(k)`; the intervening `a(j)` write is not it.
        assert!(!fires(
            "proc f {} { set a(k) 1\n set a(j) 2\n puts $a(k) }",
            D,
            "W220"
        ));
        assert!(!fires(
            "proc f {} { set o(-mode) cbc\n set o(-key) k\n return [array get o] }",
            D,
            "W220"
        ));
    }

    #[test]
    fn genuinely_unused_element_and_scalar_dead_store_fire() {
        // `set a(k) 1` on an array that is never read is flagged W211 ("set but
        // never used"); the co-located dead-store W220 is deduped in favour of
        // that single, clearer hint.
        assert_eq!(count("proc f {} { set a(k) 1\n return 0 }", D, "W211"), 1);
        assert_eq!(count("proc f {} { set a(k) 1\n return 0 }", D, "W220"), 0);
        // A genuine dead store of a *used* scalar still fires a standalone W220
        // (no W211 to dedup against).
        assert_eq!(
            count("proc f {} { set x 1\n set x 2\n return $x }", D, "W220"),
            1
        );
    }

    #[test]
    fn read_modify_write_in_cmdsub_keeps_init_live() {
        let src = "proc f {} { set i 0\n foreach j {1 2 3} { lappend r [incr i $j] }\n return $r }";
        assert!(!fires(src, D, "W220"));
        assert!(!fires(src, D, "W211"));
    }

    #[test]
    fn cmdsub_expr_read_keeps_init_live() {
        assert!(!fires(
            "proc f {} { set w 5; set i 0; incr i [expr {$w}]; return $i }",
            D,
            "W220"
        ));
        assert!(!fires(
            "proc f {n} { set scale 3; return [expr {$n * $scale}] }",
            D,
            "W211"
        ));
        assert!(!fires(
            "proc f {n} { set scale 3; return [expr {$n * $scale}] }",
            D,
            "W220"
        ));
    }

    #[test]
    fn eval_braced_body_read_keeps_init_live() {
        assert!(!fires("proc f {} { set x 1; eval {puts $x} }", D, "W220"));
        assert!(!fires("proc f {} { set x 1; eval {puts $x} }", D, "W211"));
        // …but an eval body that does NOT read x leaves it unused.
        assert_eq!(count("proc f {} { set x 1; eval {puts hi} }", D, "W211"), 1);
    }

    #[test]
    fn genuine_dead_store_still_flagged() {
        assert!(fires(
            "proc f {} { set i 0\n set i 5\n return $i }",
            D,
            "W220"
        ));
        assert!(fires(
            "proc f {} { set y 1; set y 2; return $y }",
            D,
            "W220"
        ));
    }
}

// Structural-body scope isolation.
//
// An OO / snit body is STRUCTURAL — it must not contribute reads/writes to the
// enclosing proc's data flow, otherwise it would silence the proc's own W210 /
// dead-store / W214 findings.
//
// Message shape: the dead `set x 1` surfaces as **W220 ("never read")** rather
// than W211 ("set but never used") — the structural-isolation intent (all three
// findings survive) holds, so the assertion is pinned to the code set
// {W210, W214, W220}.
mod structural_body_isolation {
    use super::super::*;

    fn outer_codes(structural_call: &str) -> std::collections::HashSet<String> {
        // The structural call runs before the raising `puts $y`, and every
        // definer it uses exists: tclsh8.6.18/9.0.4 raise `invalid command
        // name` for snit/uri without `package require`, and `oo::define C`
        // raises when C does not refer to an object.
        let src = format!(
            "package require snit\npackage require uri\noo::class create C\n\
             proc outer {{p}} {{\n    set x 1\n    {structural_call}\n    puts $y\n}}\n"
        );
        codes(&src, D)
            .into_iter()
            .filter(|c| ["W210", "W211", "W214", "W220"].contains(&c.as_str()))
            .collect()
    }

    #[test]
    fn structural_bodies_do_not_leak_into_outer_scope() {
        // For each structural call, the outer proc's own findings all survive:
        // W210 (read $y never set), W214 (unused param p), and the never-read
        // `set x` (W211, which suppresses W220 per kcs-diagnostic-w220).
        for structural_call in [
            "oo::class create C { method m {} { puts $x; set y 1; set p 2 } }",
            "oo::define C { method m {} { puts $x; set y 1; set p 2 } }",
            "oo::objdefine C method m {} { puts $x; set y 1; set p 2 }",
            "snit::type T { method m {} { puts $x; set y 1; set p 2 } }",
            "snit::method T m {} { puts $x; set y 1; set p 2 }",
            "uri::register demo { puts $x; set y 1; set p 2 }",
        ] {
            let cs = outer_codes(structural_call);
            assert!(
                cs.contains("W210"),
                "missing W210 for {structural_call:?}: {cs:?}"
            );
            assert!(
                cs.contains("W214"),
                "missing W214 for {structural_call:?}: {cs:?}"
            );
            assert!(
                cs.contains("W211"),
                "missing dead-store for {structural_call:?}: {cs:?}"
            );
        }
    }
}

// namespace eval body scope (caller-frame vs child-namespace).
//
// `namespace eval ns {…}` runs in ns, NOT the caller frame, so an unqualified
// `$x` inside it is not a use of the caller's local `x` (param stays unused).
// Plain `eval {…}` DOES run in the current frame → recovers the read.
mod namespace_eval_body_scope {
    use super::super::*;

    fn w214_names(src: &str) -> std::collections::HashSet<String> {
        of_code(src, D, "W214")
            .into_iter()
            .filter_map(|(m, _, _)| m.split('\'').nth(1).map(str::to_string))
            .collect()
    }

    #[test]
    fn param_used_only_in_ns_eval_body_is_unused() {
        assert!(
            w214_names("proc g {x} { namespace eval ::ns { puts \"hello $x\" } }").contains("x")
        );
        assert!(
            w214_names("proc g {x ns} { namespace eval $ns { puts \"hello $x\" } }").contains("x")
        );
    }

    #[test]
    fn param_used_in_plain_eval_body_is_used() {
        assert!(!w214_names("proc g {x} { eval { puts \"hello $x\" } }").contains("x"));
    }

    #[test]
    fn ns_eval_dead_store_still_fires_name_arg_recovered() {
        // `set y 1` is never read: the body's `$y` runs in ::ns (tclsh8.6.18
        // falls back to the global y, 9.0.4 raises can't read "y"), so the
        // store reports W211, which suppresses W220.
        assert!(fires(
            "proc g {} { set y 1; namespace eval ::ns { puts \"$y\" } }",
            D,
            "W211"
        ));
        // …but the namespace *name* expression IS evaluated in the caller scope.
        assert!(!fires(
            "proc f {conn} { set name [format ns%s $conn]\n namespace eval $name { variable s } }",
            D,
            "W220"
        ));
    }
}

// Call-by-name suppression precision — the `DynamicNameLocal` refinement.
//
// A callee whose parameter's *value* names a CALLEE-LOCAL variable
// (`set $p 1`, `scan … $p`, `lassign … $p`, `regsub … $p`) does NOT
// consume the caller's variable when a literal name is passed (`f x`),
// so the caller's otherwise-unused `set x 1` stays a dead store
// (W211 / W220).  Only a genuine `upvar`-aliased write-back suppresses
// it.  These pin the `DYNAMIC_NAME_LOCAL` refinement; its absence would
// re-open the caller-side false negatives it guards against.
mod call_by_name_dynamic_name_local {
    use super::super::*;

    /// Each callee uses `$p` as a callee-local dynamic name; the caller's
    /// `set x 1; f x` leaves `x` unread → the dead store must still be flagged.
    /// `x` is set-once-never-read, so it surfaces as W211 ("set but never
    /// used"); the co-located dead-store W220 is deduped in favour of that
    /// single hint. The point of the test — that the callee's dynamic name does
    /// not *suppress* the caller's detection — holds via the surviving W211.
    #[test]
    fn callee_local_dynamic_name_does_not_suppress_caller_dead_store() {
        for (label, src) in [
            ("set", "proc f {p} { set $p 1 }\nproc g {} { set x 1; f x }"),
            (
                "scan",
                "proc f {p} { scan $s {%d} $p }\nproc g {} { set x 1; f x }",
            ),
            (
                "lassign",
                "proc f {p} { lassign $l $p }\nproc g {} { set x 1; f x }",
            ),
            (
                "regsub",
                "proc f {p} { regsub a $s b $p }\nproc g {} { set x 1; f x }",
            ),
        ] {
            assert!(
                fires(src, D, "W211"),
                "[{label}] callee-local dynamic name must not suppress caller W211; got {:?}",
                codes(src, D),
            );
            assert!(
                !fires(src, D, "W220"),
                "[{label}] the caller dead store is covered by W211, so the co-located \
                 W220 is deduped; got {:?}",
                codes(src, D),
            );
        }
    }

    /// A genuine caller-frame write-back through `upvar` IS call-by-name —
    /// the caller's `x` is consumed, so neither W211 nor W220 fires.
    #[test]
    fn genuine_upvar_call_by_name_suppresses_caller_dead_store() {
        let src = "proc f {p} { upvar 1 $p l; set l 5 }\nproc g {} { set x 1; f x }";
        assert!(
            !fires(src, D, "W211"),
            "genuine upvar call-by-name should suppress caller W211; got {:?}",
            codes(src, D),
        );
        assert!(
            !fires(src, D, "W220"),
            "genuine upvar call-by-name should suppress caller W220; got {:?}",
            codes(src, D),
        );
    }
}

// A procedure's condition over a formal is never constant (I230), even when
// every call in the file passes the same literal: the body is analysed from a
// generic entry, since a later `source`, a callback or an interactive session
// can call it with another value.
//   tclsh: `proc h {m} {expr {$m eq "prod"}}; list [h prod] [h dev]` ⇒ `1 0`
mod caller_uniform_literal_seed {
    use super::super::*;

    const LIB: &str = "proc helper {mode} {\n    if {$mode eq \"prod\"} { set r 1 } else { set r 2 }\n    return $r\n}\nhelper prod\n";

    #[test]
    fn agreeing_callers_leave_the_condition_open() {
        for other in ["helper prod", "set c helper; $c prod"] {
            let src = format!("{LIB}{other}\n");
            assert!(!fires(&src, D, "I230"), "`{other}`: {:?}", codes(&src, D));
        }
    }

    #[test]
    fn a_differing_caller_retracts_the_fold() {
        for other in [
            "helper dev",
            "uplevel #0 {helper dev}",
            "catch { uplevel #0 { helper dev } }",
            "set c helper; $c dev",
            "set c [gets stdin]; $c dev",
        ] {
            let src = format!("{LIB}{other}\n");
            assert!(!fires(&src, D, "I230"), "`{other}`: {:?}", codes(&src, D));
        }
    }
}
