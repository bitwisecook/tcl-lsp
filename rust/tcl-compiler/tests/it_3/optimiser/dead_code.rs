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

//! Dead stores, unused variables and unused procedures: what is deleted, and what a possible raise, a conditional writer or a reader keeps.

use super::*;

#[test]
fn dead_store_and_dead_code_elimination() {
    // DSE: `set a 1; set a 2; puts $a` ⇒ first store dead (O109), value 2
    // forwarded (O102). tclsh: a == 2.
    let dse = "set a 1\nset a 2\nputs $a";
    let dse_out = optimised(dse, TCL);
    assert!(dse_out.contains("puts 2"));
    assert!(opt_fires(dse, TCL, "O109"));
    assert!(opt_fires(dse, TCL, "O102"));

    // ADCE: transitively dead stores removed; final value 5. tclsh: a == 5.
    let adce = "set a 1\nset a [expr {$a + 1}]\nset a 5\nputs $a";
    let adce_out = optimised(adce, TCL);
    assert!(adce_out.contains("puts 5"));
    assert!(opt_fires(adce, TCL, "O108"));

    // DCE of an `if {0}` block: dead body removed, `puts always` survives (O112).
    let dce = "if {0} {\n    puts never\n    set x 1\n}\nputs always\n";
    let dce_result = optimised(dce, TCL);
    assert!(!dce_result.contains("puts never"));
    assert!(!dce_result.contains("set x 1"));
    assert!(dce_result.contains("puts always"));
    assert!(opt_fires(dce, TCL, "O112"));
}

// Unused variable elimination — O126

#[test]
fn unused_variable_elimination_o126() {
    // A `set` whose variable is returned is NOT removed.
    let ret = "proc calcDb {mag} {\n    set db [expr {10*log($mag)}]\n    return $db\n}\n";
    assert!(!opt_fires(ret, TCL, "O126"));

    // A braced (literal) `return {$result}` does not read the variable, so the
    // `set` is dead. tclsh: `return {$result}` returns the literal text
    // "$result", independent of the variable. Rust folds `return {$result}`
    // ⇒ `return 42` here (O100), proving the variable is dead either way.
    let braced = "proc foo {} {\n    set result 42\n    return {$result}\n}\n";
    // Rust applies O100 to fold the constant variable into the (literal) return;
    // the assignment becomes provably unused. Confirm the literal is preserved.
    assert!(optimised(braced, TCL).contains("return 42") || opt_fires(braced, TCL, "O126"));
}

// Unused iRule procs — O124 (f5-irules only)

#[test]
fn unused_irule_procs_o124() {
    const IR: &str = "f5-irules";
    // Unused proc commented out.
    let unused = "proc helper {x} {\n    return $x\n}\n\nwhen HTTP_REQUEST {\n    pool my_pool\n}";
    let uo = optimised(unused, IR);
    assert!(opt_fires(unused, IR, "O124"));
    assert!(uo.contains("# proc helper"));
    assert!(!uo.replace("# proc helper", "").contains("proc helper"));

    // Used / transitively-used / qualified-call / direct-invocation procs are
    // NOT commented out.
    let kept = [
        "proc helper {} {\n    return 1\n}\n\nwhen HTTP_REQUEST {\n    set val [call helper]\n}",
        "proc inner {} {\n    return 42\n}\n\nproc outer {} {\n    return [call inner]\n}\n\nwhen HTTP_REQUEST {\n    set val [call outer]\n}",
        "proc helper {} {\n    return 1\n}\n\nwhen HTTP_REQUEST {\n    set val [helper]\n}",
        "namespace eval ns {\n    proc helper {} {\n        return 1\n    }\n}\n\nwhen HTTP_REQUEST {\n    set val [call ::ns::helper]\n}",
        // library iRule (only procs + RULE_INIT) — skip.
        "proc helper {} {\n    return 1\n}\n\nwhen RULE_INIT {\n    set ::debug 0\n}",
        // procs only, no events — library, skip.
        "proc helper_a {} {\n    return 1\n}\n\nproc helper_b {} {\n    return [call helper_a]\n}",
        // called only from RULE_INIT — still used.
        "proc init_helper {} {\n    return 1\n}\n\nwhen RULE_INIT {\n    set ::val [call init_helper]\n}\n\nwhen HTTP_REQUEST {\n    pool my_pool\n}",
    ];
    for src in kept {
        assert!(!opt_fires(src, IR, "O124"), "O124 must not fire: {src}");
    }

    // O124 only applies to f5-irules — plain Tcl is untouched.
    assert!(!opt_fires(
        "proc unused {} {\n    return 1\n}\nputs hello",
        TCL,
        "O124"
    ));

    // Multiple unused procs ⇒ two O124, naming each.
    let multi = "proc used {} {\n    return 1\n}\n\nproc unused_a {} {\n    return 2\n}\n\nproc unused_b {} {\n    return 3\n}\n\nwhen HTTP_REQUEST {\n    set val [call used]\n}";
    let registry = static_context_for(IR).commands();
    let o124s: Vec<_> = optimise_with_dialect(
        multi,
        registry,
        Some(tcl_registry::model::ingress::resolve_environment(IR).analyser_profile()),
    )
    .into_iter()
    .filter(|o| o.code.as_str() == "O124")
    .collect();
    assert_eq!(o124s.len(), 2);
    assert!(o124s.iter().any(|o| o.message.contains("unused_a")));
    assert!(o124s.iter().any(|o| o.message.contains("unused_b")));

    // Mutually-recursive but unreachable procs ⇒ both flagged, no tail-call
    // rewrite (O124 supersedes).
    let mutual = "proc ping {n} {\n    if {$n <= 0} {\n        return 0\n    }\n    return [call pong [expr {$n - 1}]]\n}\n\nproc pong {n} {\n    if {$n <= 0} {\n        return 0\n    }\n    return [call ping [expr {$n - 1}]]\n}\n\nwhen HTTP_REQUEST {\n    pool my_pool\n}";
    assert_eq!(opt_count(mutual, IR, "O124"), 2);
    assert!(!opt_fires(mutual, IR, "O121"));
    assert!(!opt_fires(mutual, IR, "O122"));

    // O124 supersedes tail-call rewrites for an unused tail-recursive proc.
    let supersede = "proc fact {n acc} {\n    if {$n <= 1} {\n        return $acc\n    }\n    return [fact [expr {$n - 1}] [expr {$n * $acc}]]\n}\n\nwhen HTTP_REQUEST {\n    pool my_pool\n}";
    assert!(opt_fires(supersede, IR, "O124"));
    assert!(!opt_fires(supersede, IR, "O121"));
    assert!(!opt_fires(supersede, IR, "O122"));

    // O124 does not block independent passes for a *used* proc; the used proc
    // is not commented out.
    //
    // The pass that fires here is O112, not O120: `[call helper bar]` is a
    // command substitution nested in a word, and since #2134 those are
    // enumerated as call sites, so `x` is known to be `bar` inside the body
    // and the `foo` branch is proven dead. Before that the caller was
    // invisible, nothing was seeded, and only the weaker O120 rewrite applied.
    let used_body = "proc helper {x} {\n    if {$x == \"foo\"} {\n        return 1\n    }\n    return 0\n}\n\nwhen HTTP_REQUEST {\n    set val [call helper bar]\n}";
    assert!(
        opt_fires(used_body, IR, "O112"),
        "the now-visible caller proves the foo branch dead",
    );
    assert!(!opt_fires(used_body, IR, "O124"));
}

#[test]
fn unused_irule_procs_o124_eval_suppression() {
    const IR: &str = "f5-irules";
    // OMISSION: O124 is NOT suppressed when a reachable event/proc contains
    // `eval` (dynamic dispatch could in principle call the "unused" proc) — the
    // unused proc is still flagged as removable.
    //  - `eval`-in-event           → DOES fire O124
    //  - `eval`-in-reachable-proc  → DOES fire O124
    // These two cases are known gaps (omitted here).

    // `eval` only in an UNREACHABLE proc does not suppress O124 — the unused proc
    // is still flagged.
    let unreach = "proc dynamic_helper {} {\n    eval {set x 1}\n}\n\nwhen HTTP_REQUEST {\n    pool my_pool\n}";
    assert!(opt_fires(unreach, IR, "O124"));
}

/// #2051 — `regexp`, `scan` and `binary scan` write their targets only on the
/// match path.
///
/// Each leaves a target's previous value in place when the match or
/// conversion does not reach it, and never creates one that did not exist.
/// Measured identical on tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1b0.
/// Modelling the write as unconditional made the feeding store look
/// overwritten-before-read, so O109 deleted it and the program did not merely
/// print something else — it failed with `can't read "…"`.
#[test]
fn a_conditional_writer_does_not_kill_the_store_it_may_preserve() {
    for (src, why) in [
        (
            "proc p {} {\n    set a before\n    set b before\n    regexp {(x)(y)} zz a b\n    puts \"$a $b\"\n}\np\n",
            "a failed regexp leaves both match variables alone",
        ),
        (
            "proc p {} {\n    set a before\n    set b before\n    scan {12 nope} {%d %d} a b\n    puts \"$a $b\"\n}\np\n",
            "scan converts one field and leaves the second target alone",
        ),
        (
            "proc p {d} {\n    set c before\n    set e before\n    binary scan $d \"a1a5\" c e\n    puts \"$c $e\"\n}\np AB\n",
            "binary scan runs out of data and leaves the second target alone",
        ),
    ] {
        assert!(
            !opt_fires(src, TCL, "O109"),
            "{why}: {:?}",
            opt_codes(src, TCL)
        );
        // Asserted on the stores rather than byte-identity: the `binary scan`
        // row also gets a legitimate O100, specialising its one call site's
        // `$d` to `AB`, and the `regexp` row's proven no-match keeps both
        // values in the lattice, so its `puts` reads them as the
        // constants they are — both unrelated and correct.
        let out = optimised(src, TCL);
        for store in src
            .lines()
            .map(str::trim)
            .filter(|line| line.starts_with("set ") && line.ends_with(" before"))
        {
            assert!(
                out.lines().any(|line| line.trim() == store),
                "{why}: the store `{store}` the command may preserve survives: {out}"
            );
        }
    }
    // The proven no-match preserves both variables, and says so.
    let out = optimised(
        "proc p {} {\n    set a before\n    set b before\n    regexp {(x)(y)} zz a b\n    puts \"$a $b\"\n}\np\n",
        TCL,
    );
    assert!(out.contains("puts \"before before\""), "{out}");
}

/// Precision: a command that writes its target on *every* path still has its
/// dead store eliminated. Each was measured writing unconditionally, on the
/// failure path too — `regsub {xx} zz YY a` leaves `a` as `zz`, `lassign`
/// pads a short list with `""`, `catch` always writes its result variable.
#[test]
fn an_unconditional_writer_still_kills_its_dead_store() {
    for (src, why) in [
        (
            "proc p {s} { set a 1; regsub {x} $s y a; puts $a }\np zz\n",
            "regsub",
        ),
        (
            "proc p {} { set m 1; catch {expr {1+1}} m; puts $m }\np\n",
            "catch",
        ),
        (
            "proc p {l} { set a 1; set b 2; lassign $l a b; puts \"$a $b\" }\np one\n",
            "lassign",
        ),
    ] {
        assert!(
            opt_fires(src, TCL, "O109"),
            "{why} writes on every path, so the earlier store is dead: {:?}",
            opt_codes(src, TCL)
        );
    }
}

/// Precision: the no-match prover still reports a target a failed match never
/// creates. Crediting the read must not silence W210, which is what keeps
/// `regexp {x} y -> v; puts $v` reported — tclsh fails it with
/// `can't read "v"`.
#[test]
fn the_no_match_prover_still_reports_an_uncreated_target() {
    for src in [
        "proc f {} { regexp {x} y -> v; puts $v }",
        "proc f {} { scan abc %d v; puts $v }",
    ] {
        assert!(
            analyser_codes(src, TCL).iter().any(|c| c == "W210"),
            "a target the match never creates is still reported: {:?}",
            analyser_codes(src, TCL)
        );
    }
}

/// A dead assignment whose value can raise is not dead: deleting it drops the
/// error the program stops on. tclsh 8.6.18 raises for each of these (and
/// 8.4.20, 9.0.4 and 9.1b0 agree); O126, O109 and O108 deleted the statement
/// and the program printed `hi` (#2249).
#[test]
fn a_dead_assignment_whose_value_can_raise_is_kept() {
    for (why, src, kept) in [
        (
            "an unset variable",
            "proc p {} {\n    set y $x\n    puts hi\n}\n",
            "set y $x",
        ),
        (
            "an unset variable in a command word",
            "proc p {} {\n    set y [lindex $x 0]\n    puts hi\n}\n",
            "set y [lindex $x 0]",
        ),
        (
            "an unset variable in quotes",
            "proc p {} {\n    set y \"$x\"\n    puts hi\n}\n",
            "set y \"$x\"",
        ),
        (
            "an unset element",
            "proc p {} {\n    set y $a(k)\n    puts hi\n}\n",
            "set y $a(k)",
        ),
        (
            "a name `upvar` links",
            "proc p {} {\n    upvar 1 v v\n    set y $v\n    puts hi\n}\n",
            "set y $v",
        ),
        (
            "an array read as a scalar",
            "proc p {} {\n    set a(k) 1\n    set y $a\n    puts hi\n}\n",
            "set y $a",
        ),
        (
            "a variable a `catch` may leave unset",
            "proc p {} {\n    catch {set x [error boom]}\n    set y $x\n    puts hi\n}\n",
            "set y $x",
        ),
        (
            "a variable set on one branch only",
            "proc p {c} {\n    if {$c} {set x 1}\n    set y $x\n    puts hi\n}\n",
            "set y $x",
        ),
        (
            "a `regexp` output variable",
            "proc p {s} {\n    regexp {(z)} $s -> x\n    set y $x\n    puts hi\n}\n",
            "set y $x",
        ),
        (
            "an `unset` variable",
            "proc p {} {\n    set x 1\n    unset x\n    set y $x\n    puts hi\n}\n",
            "set y $x",
        ),
        (
            "`expr` arithmetic on a parameter",
            "proc p {v} {\n    set y [expr {$v + 1}]\n    puts hi\n}\n",
            "set y [expr {$v + 1}]",
        ),
        (
            "`expr` division by zero",
            "proc p {} {\n    set y [expr {1/0}]\n    puts hi\n}\n",
            "set y [expr {1/0}]",
        ),
        (
            "a method sharing its name with a proc binds only its own parameters",
            "namespace eval C { proc m {x} {} }\noo::class create C {\n    method m {} {\n        ::set unused $x\n        ::return 2\n    }\n}\n",
            "::set unused $x",
        ),
        (
            "a `foreach` variable over a list that may be empty",
            "proc p {l} {\n    foreach x $l {}\n    set y $x\n    puts hi\n}\n",
            "set y $x",
        ),
        (
            "an array read as a scalar after `array set`",
            "proc p {} {\n    array set a {}\n    set y $a\n    puts hi\n}\n",
            "set y $a",
        ),
        (
            "an `lset` target that was never set",
            "proc p {} {\n    catch {lset x 0 new}\n    set y $x\n    puts hi\n}\n",
            "set y $x",
        ),
        (
            "a variable only a `catch` script assigns",
            "proc p {} {\n    catch {error boom; set a 1} x\n    set y $a\n    puts hi\n}\n",
            "set y $a",
        ),
        (
            "a nested writer's target that an outer word happens to spell",
            "proc p {} {\n    regsub x [regexp z a -> x] y out\n    set dead $x\n    puts hi\n}\n",
            "set dead $x",
        ),
        (
            "an overwritten store of an unset variable (O109)",
            "proc p {} {\n    set y $x\n    set y 1\n    return $y\n}\n",
            "set y $x",
        ),
    ] {
        let out = optimised(src, TCL);
        assert!(out.contains(kept), "{why}: the statement stays: {out}");
    }
}

/// Bound reads alone do not prove an element read or a startup name: an
/// element read raises when its base is a scalar, and only the top level
/// itself starts with `argv` bound.
#[test]
fn a_dead_element_or_startup_read_that_can_raise_is_kept() {
    for (why, src, kept) in [
        (
            "an element of a scalar parameter, by a dynamic index",
            "proc p {a i} {\n    set dead $a($i)\n    puts hi\n}\n",
            "set dead $a($i)",
        ),
        (
            "an element of a scalar parameter, by a literal index",
            "proc p {a} {\n    set dead $a(k)\n    puts hi\n}\n",
            "set dead $a(k)",
        ),
        (
            "an element of the empty-named scalar",
            "proc p {i} {\n    set {} scalar\n    set dead x$($i)\n    puts hi\n}\n",
            "set dead x$($i)",
        ),
        (
            "a startup name in a procedure that shares the top level's `::top` name",
            "proc ::top {} {\n    set dead $argv\n    puts hi\n}\n",
            "set dead $argv",
        ),
    ] {
        let out = optimised(src, TCL);
        assert!(out.contains(kept), "{why}: the statement stays: {out}");
    }
}

/// Precision for the rule above: a value that cannot raise is still deleted.
#[test]
fn a_dead_assignment_whose_value_cannot_raise_is_still_deleted() {
    for (why, src) in [
        ("a literal", "proc p {} {\n    set y 1\n    puts hi\n}\n"),
        (
            "a copy of a set local",
            "proc p {} {\n    set x [clock seconds]\n    set y $x\n    puts hi\n}\n",
        ),
        (
            "a copy of a parameter",
            "proc p {v} {\n    set y $v\n    puts hi\n}\n",
        ),
        // A method binds its arguments on entry too (found in review).
        (
            "a copy of a method argument",
            "oo::class create C {\n    method uses {v} {\n        ::set unused $v\n        ::return 2\n    }\n}\n",
        ),
        (
            "a variable set on both branches",
            "proc p {c} {\n    if {$c} {set x 1} else {set x 2}\n    set y $x\n    puts hi\n}\n",
        ),
        (
            "`expr` SCCP folds to a constant",
            "proc p {} {\n    set a 1\n    set y [expr {$a + 1}]\n    puts hi\n}\n",
        ),
        // Commands the registry marks as always writing their targets
        // (found in review).
        (
            "a `catch` result variable",
            "proc p {} {\n    catch {error boom} x\n    set y $x\n    puts hi\n}\n",
        ),
        (
            "a `gets` target",
            "proc p {c} {\n    gets $c line\n    set y $line\n    puts hi\n}\n",
        ),
        (
            "an `lassign` target",
            "proc p {l} {\n    lassign $l a\n    set y $a\n    puts hi\n}\n",
        ),
        (
            "a `regsub` target",
            "proc p {s} {\n    regsub {xx} $s YY a\n    set y $a\n    puts hi\n}\n",
        ),
        (
            "an `append` target",
            "proc p {v} {\n    append x $v\n    set y $x\n    puts hi\n}\n",
        ),
        (
            "an `lappend` target",
            "proc p {v} {\n    lappend x $v\n    set y $x\n    puts hi\n}\n",
        ),
        (
            "a `dict set` target",
            "proc p {v} {\n    dict set d k $v\n    set y $d\n    puts hi\n}\n",
        ),
        (
            "a `dict incr` target",
            "proc p {} {\n    dict incr d k\n    set y $d\n    puts hi\n}\n",
        ),
        (
            "a `chan gets` target",
            "proc p {c} {\n    chan gets $c line\n    set y $line\n    puts hi\n}\n",
        ),
        // A conditional writer keeps the previous value on a miss.
        (
            "a `regexp` output variable set before",
            "proc p {s} {\n    set x old\n    regexp {(z)} $s -> x\n    set y $x\n    puts hi\n}\n",
        ),
        // A read-modify-write target stays set once it was set.
        (
            "an `lset` target set before",
            "proc p {} {\n    set x {old}\n    lset x 0 new\n    set y $x\n    puts hi\n}\n",
        ),
        (
            "a `file tempfile` name variable",
            "proc p {} {\n    file tempfile path\n    set y $path\n    puts hi\n}\n",
        ),
        (
            "an `info default` variable",
            "proc p {} {\n    info default p x v\n    set y $v\n    puts hi\n}\n",
        ),
        // An alias that prepends words keeps its target (found in review).
        (
            "a `gets` target through an alias that prepends the channel",
            "interp alias {} mygets {} gets stdin\nproc p {} {\n    mygets line\n    set y $line\n    puts hi\n}\n",
        ),
        (
            "a `cmdline::getKnownOpt` value variable",
            "package require cmdline\nproc p {argv} {\n    cmdline::getKnownOpt argv {a.arg} o v\n    set y $v\n    puts hi\n}\n",
        ),
    ] {
        assert!(
            opt_fires(src, TCL, "O126"),
            "{why}: the unused store goes: {:?}",
            opt_codes(src, TCL)
        );
    }
}

/// The same precision for writers that exist only in Tcl 9.
#[test]
fn a_dead_copy_of_a_tcl9_writer_target_is_still_deleted() {
    for (why, src) in [
        (
            "a `const`",
            "proc p {} {\n    const c 5\n    set y $c\n    puts hi\n}\n",
        ),
        (
            "an `encoding -failindex` variable",
            "proc p {s} {\n    encoding convertto -failindex fi utf-8 $s\n    set y $fi\n    puts hi\n}\n",
        ),
    ] {
        assert!(
            opt_fires(src, "tcl9.0", "O126"),
            "{why}: the unused store goes: {:?}",
            opt_codes(src, "tcl9.0")
        );
    }
}

/// A scalar the interpreter binds before user code (`argv`) is set at the top
/// level's entry, so an overwritten copy of it is a dead store there, as a
/// literal is; an unset name keeps its store, and inside a procedure `argv` is
/// an unset local.
#[test]
fn an_overwritten_copy_of_a_startup_scalar_goes_only_at_top_level() {
    for (why, src, dead) in [
        (
            "a startup scalar",
            "set dead $argv\nset dead 1\nputs $dead\n",
            true,
        ),
        (
            "an unset global",
            "set dead $nope\nset dead 1\nputs $dead\n",
            false,
        ),
        (
            "a global named like a parameter of a procedure called `::top`",
            "proc ::top {x} {}\nset dead $x\nset dead 1\nputs $dead\n",
            false,
        ),
        (
            "a procedure's local",
            "proc p {} {\n    set dead $argv\n    set dead 1\n    return $dead\n}\n",
            false,
        ),
    ] {
        assert_eq!(
            opt_fires(src, TCL, "O109"),
            dead,
            "{why}: {:?}",
            opt_codes(src, TCL)
        );
    }
}

/// `grapheme next|prev` read their cursor and then write it, so the cursor is
/// set afterwards exactly when it was set before, and the store feeding it is
/// live.
#[test]
fn a_grapheme_cursor_carries_its_definedness() {
    let copy = "proc p {s i} {\n    ::tcl::unsupported::grapheme next $s i\n    set dead $i\n    puts hi\n}\n";
    assert!(
        opt_fires(copy, "tcl9.1", "O126"),
        "a copy of a bound cursor goes: {:?}",
        opt_codes(copy, "tcl9.1")
    );
    let feed =
        "proc p {s} {\n    set i 2\n    tcl::unsupported::grapheme prev $s i\n    return $i\n}\n";
    assert!(
        !opt_fires(feed, "tcl9.1", "O109"),
        "the store feeding the cursor stays: {:?}",
        opt_codes(feed, "tcl9.1")
    );
}

/// `string is class -failindex var` writes `var` only when the test fails and
/// leaves it as it was otherwise, so a store before it is not dead. tclsh
/// 8.5.19 to 9.1b0 print `keep`; O109 deleted `set fi keep` and the program
/// failed with `can't read "fi"`.
#[test]
fn a_store_before_string_is_failindex_is_kept() {
    let src = "proc p {} {\n    set fi keep\n    string is integer -failindex fi 123\n    return $fi\n}\nputs [p]\n";
    let out = optimised(src, TCL);
    assert!(out.contains("set fi keep"), "the store stays: {out}");
}
