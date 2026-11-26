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

//! What a statement leaves in the values: opaque `switch` arms, callback writes, nested writes, a raise in the middle and unseen calls.

use tcl_registry::value_transfer::NumericValue;

use super::*;

#[test]
fn typed_element_base_keeps_literal_name_data_and_distinct_root_symbols() {
    let mut ssa = bare_ssa();
    let ordinary = ssa.intern_var("array");
    let sigil = ssa.intern_var("$array");
    let braced_sigil = ssa.intern_var("${array}");
    let empty = ssa.intern_var("");
    let nul = ssa.intern_var("array\0tail");
    let statement = |name: &str, name_braced| Statement::AssignConst {
        span: Span::new(0, 0),
        name: name.to_owned(),
        name_braced,
        value: "VALUE".into(),
        value_span: None,
    };
    for (name, root) in [
        ("array(key)", ordinary),
        ("array()", ordinary),
        ("$array(key)", sigil),
        ("${array}()", braced_sigil),
        ("(key)", empty),
        ("()", empty),
        ("array\0tail(key)", nul),
    ] {
        assert_eq!(
            typed_element_write_base(&statement(name, true), &ssa),
            Some(root),
            "{name:?}"
        );
    }
    for name in [
        "array",
        "$array",
        "",
        "array(key",
        "array(key)tail",
        ")",
        "array\0tail",
    ] {
        assert_eq!(
            typed_element_write_base(&statement(name, true), &ssa),
            None,
            "{name:?}"
        );
    }
    assert_eq!(
        typed_element_write_base(&statement("$array(key)", false), &ssa),
        None
    );
    assert_ne!(ordinary, sigil);
    assert_ne!(ordinary, braced_sigil);
}

/// A `switch` the flow graph keeps as one statement leaves each name its
/// arms write with no constant value, and a name only an arm binds may be
/// unbound afterwards: tclsh 8.4 to 9.1 print `0` and `1` for `proc f {s} {
/// set go 1; switch -glob -- $s { q* { set go 0; set fresh 1 } }; puts $go;
/// puts $fresh }` called with `q1`, and fail on `$fresh` with `zz`.
#[test]
fn an_opaque_switch_arm_leaves_what_it_writes_without_a_constant_or_a_settled_existence() {
    let registry = registry();
    for arm in ["-glob", "-nocase", "-regexp"] {
        let cu = crate::compilation_unit::CompilationUnit::build_for(
            &format!(
                "proc f {{s}} {{ set go 1; switch {arm} -- $s {{ q* {{ set go 0; set fresh 1 }} }}; puts $go; puts $fresh }}"
            ),
            &registry,
            false,
        );
        let f = cu.function("::f").expect("procedure analysed");
        assert_eq!(last_value(f, "go"), LatticeValue::Overdefined, "{arm}");
        assert_eq!(puts_reads(f, "fresh"), [Existence::MayBound], "{arm}");
    }
    // A name no arm writes keeps its constant.
    let cu = crate::compilation_unit::CompilationUnit::build_for(
        "proc f {s} { set go 1; switch -glob -- $s { q* { set other 0 } }; puts $go }",
        &registry,
        false,
    );
    let f = cu.function("::f").expect("procedure analysed");
    assert_eq!(last_value(f, "go"), LatticeValue::Const(ConstValue::int(1)));
}

/// What a command an arm runs does to the frame is as real as a write the
/// arm makes itself: a callee that writes the caller's name through
/// `upvar`, and a command that may write any name (`namespace eval`,
/// `dict with`), leave the name with no constant, while a callee that
/// writes only a global leaves a local alone.
#[test]
fn an_opaque_switch_arm_that_runs_a_writer_or_a_barrier_leaves_the_name_unsettled() {
    let registry = registry();
    for source in [
        "proc zero {v} {upvar 1 $v x; set x 0}\nproc f {s} { set go 1; switch -glob -- $s { q* { zero go } }; puts $go }",
        "proc f {s} { set go 1; switch -glob -- $s { q* { dict with d { set go 0 } } }; puts $go }",
        "proc f {s} { set go 1; switch -glob -- $s { q* { eval $s } }; puts $go }",
    ] {
        let cu = crate::compilation_unit::CompilationUnit::build_for(source, &registry, false);
        let f = cu.function("::f").expect("procedure analysed");
        assert_eq!(last_value(f, "go"), LatticeValue::Overdefined, "{source}");
    }
    let cu = crate::compilation_unit::CompilationUnit::build_for(
        "set go 1\nswitch -glob -- [gets stdin] { q* { namespace eval :: {set go 0} } }\nputs $go\n",
        &registry,
        false,
    );
    let f = cu.function("::top").expect("top level analysed");
    assert_eq!(last_value(f, "go"), LatticeValue::Overdefined);
    let cu = crate::compilation_unit::CompilationUnit::build_for(
        "proc hit {} {global hits; incr hits}\nproc f {s} { set go 1; switch -glob -- $s { q* { hit } }; puts $go }",
        &registry,
        false,
    );
    let f = cu.function("::f").expect("procedure analysed");
    assert_eq!(last_value(f, "go"), LatticeValue::Const(ConstValue::int(1)));
}

/// The names a callback script writes — `after`, `after idle`, a
/// variable trace's callback, and a procedure named as a callback — are
/// externally mutable in every function, the top level and a procedure
/// alike, and a callback that writes a computed name makes every name so.
#[test]
fn the_names_a_callback_writes_are_never_constant() {
    let registry = registry();
    for source in [
        "set go 1\nafter idle {set ::go 0}\nputs $go\n",
        "set go 1\nafter 100 { set go 0 }\nputs $go\n",
        "set go 1\ntrace add variable x write { set ::go 0 ;# }\nputs $go\n",
        "proc tick {} { set ::go 0 }\nset go 1\nafter 100 tick\nputs $go\n",
        "set go 1\nafter 100 { set $name 0 }\nputs $go\n",
    ] {
        let cu = crate::compilation_unit::CompilationUnit::build_for(source, &registry, false);
        let f = cu.function("::top").expect("top level analysed");
        assert_eq!(last_value(f, "go"), LatticeValue::Overdefined, "{source}");
    }
    let cu = crate::compilation_unit::CompilationUnit::build_for(
        "after 100 { set go 0 }\nproc p {} { global go; set go 1; puts $go }\n",
        &registry,
        false,
    );
    let f = cu.function("::p").expect("procedure analysed");
    assert_eq!(last_value(f, "go"), LatticeValue::Overdefined);
    let cu = crate::compilation_unit::CompilationUnit::build_for(
        "set unrelated 0\nafter 100 { set go 0 }\nputs $unrelated\n",
        &registry,
        false,
    );
    let f = cu.function("::top").expect("top level analysed");
    assert_eq!(
        last_value(f, "unrelated"),
        LatticeValue::Const(ConstValue::int(0))
    );
}

fn last_values(source: &str, name: &str, names: &[&str]) -> Vec<LatticeValue> {
    let registry = registry();
    let cu = crate::compilation_unit::CompilationUnit::build_for(source, &registry, false);
    let f = cu.function(name).expect("function analysed");
    names
        .iter()
        .map(|variable| last_value(f, variable))
        .collect()
}

/// An expression, the function that holds it and the final value of each
/// of two places it leaves.
type NestedWriteCase = (&'static str, &'static str, [(&'static str, i64); 2]);

/// A statement's substitutions run in the engine's left-to-right order
/// under one ordered state, so the writes they make are the definitions
/// the statement's embedded call makes and the host takes the result:
/// over `x` = 1, `$x + [incr x] + $x` is 5 and leaves `x` at 2. A branch
/// the engine never reaches writes nothing, and the place keeps what it
/// held. The host is an assignment of an expression, an expression
/// statement, or an assignment of one `expr` substitution, whose quoted
/// operand is substituted first; at the top level and in a procedure.
#[test]
fn a_nested_write_is_the_definition_its_embedded_call_makes_and_the_host_takes_the_result() {
    let int = |n: i64| LatticeValue::Const(ConstValue::int(n));
    let cases: [NestedWriteCase; 10] = [
        ("$x + [incr x] + $x", "::p", [("r", 5), ("x", 2)]),
        ("$y + [incr x] + $y", "::p", [("r", 12), ("x", 2)]),
        ("[set y] + [incr x]", "::p", [("r", 7), ("x", 2)]),
        ("0 && [incr x]", "::p", [("r", 0), ("x", 1)]),
        ("$x + [set x 10] + $x", "::p", [("r", 21), ("x", 10)]),
        ("[incr x] + [incr x]", "::p", [("r", 5), ("x", 3)]),
        ("$x ? [incr x] : [incr x 10]", "::p", [("r", 2), ("x", 2)]),
        (
            "!$x ? [incr x] : [incr x 10]",
            "::p",
            [("r", 11), ("x", 11)],
        ),
        (
            "[expr {$x + [incr x]}] + [incr x]",
            "::p",
            [("r", 6), ("x", 3)],
        ),
        ("$x + [incr x] + $x", "::top", [("r", 5), ("x", 2)]),
    ];
    for (expr, function, expected) in cases {
        let names = expected.map(|(name, _)| name);
        let want: Vec<LatticeValue> = expected.map(|(_, value)| int(value)).to_vec();
        let source = if function == "::top" {
            format!("set x 1\nset y 5\nset r [expr {{{expr}}}]\n")
        } else {
            format!("proc p {{}} {{set x 1; set y 5; set r [expr {{{expr}}}]; return $x}}\n")
        };
        assert_eq!(last_values(&source, function, &names), want, "{source}");
    }
    let statement = "proc p {} {set x 1; expr {$x + [incr x] + $x}; return $x}\n";
    assert_eq!(
        last_values(statement, "::p", &["x"]),
        [int(2)],
        "{statement}"
    );
    let quoted = "proc p {} {set x 1; set r [expr \"$x + [incr x]\"]; return $x}\n";
    assert_eq!(
        last_values(quoted, "::p", &["r", "x"]),
        [int(3), int(2)],
        "{quoted}"
    );
    let skipped = "proc p {} {set x 1; set r [expr {$x + [incr x] + (0 && [foo])}]; return $r}\n";
    assert_eq!(last_values(skipped, "::p", &["r"]), [int(3)], "{skipped}");
    let preserved = "proc p {} {set x 1; set r [expr {0 && [incr x]}]; return $x}\n";
    let registry = registry();
    let cu = crate::compilation_unit::CompilationUnit::build_for(preserved, &registry, false);
    let f = cu.function("::p").expect("analysed");
    let x = f.ssa.var_symbol("x").expect("x");
    assert_eq!(
        f.sccp.preserved.get(&(x, 2)),
        Some(&1),
        "the call leaves `x` as it was where no write ran"
    );
}

/// The writes a word's own command makes are the command's, ahead of its
/// own: `string length [append s bc]` in an expression, or as the value
/// of the statement, leaves `s` at `abc`, and `incr x [incr x]` reads `x`
/// after the inner `incr` ran, so over `x` = 1 it is 4.
#[test]
fn a_write_a_commands_word_makes_is_a_write_of_the_statement() {
    let text = |value: &str| LatticeValue::Const(ConstValue::String(value.into()));
    let int = |n: i64| LatticeValue::Const(ConstValue::int(n));
    let appended = "proc p {} {set s a; set r [expr {[string length [append s bc]] + [string length $s]}]; return $s}\n";
    assert_eq!(
        last_values(appended, "::p", &["r", "s"]),
        [int(6), text("abc")]
    );
    let first = "proc p {} {set s a; set r [expr {[string length $s] + [string length [append s bc]]}]; return $s}\n";
    assert_eq!(
        last_values(first, "::p", &["r", "s"]),
        [int(4), text("abc")]
    );
    let command = "proc p {} {set s a; set r [string length [append s bc]]; return $s}\n";
    assert_eq!(
        last_values(command, "::p", &["r", "s"]),
        [int(3), text("abc")]
    );
    let reads_after = "proc p {} {set x 1; set r [expr {[incr x [incr x]]}]; return $x}\n";
    assert_eq!(
        last_values(reads_after, "::p", &["r", "x"]),
        [int(4), int(4)]
    );
    let value = "proc p {} {set x 1; set r [incr x [incr x]]; return $x}\n";
    assert_eq!(last_values(value, "::p", &["r", "x"]), [int(4), int(4)]);
    let braced = "proc p {} {set x 1; set r [expr {$x} + [incr x]]; return $x}\n";
    assert_eq!(last_values(braced, "::p", &["r", "x"]), [int(4), int(2)]);
    let created = "proc p {} {unset -nocomplain u; set r [expr {[string length [append u [set u 3]]]}]; return $u}\n";
    assert_eq!(last_values(created, "::p", &["r", "u"]), [int(2), int(33)]);
    let inside =
        "proc p {} {set s a; set r [expr {[string length [append s [append s b]]]}]; return $s}\n";
    assert_eq!(
        last_values(inside, "::p", &["r", "s"]),
        [int(4), text("abab")]
    );
}

/// A statement the engine evaluates for each of several possible inputs
/// leaves each place the join of what every member left: `$c ? [incr x] :
/// 7` writes `x` for the input that takes the branch and leaves it alone
/// for the other, so `x` is 1 or 2 and the result 7 or 2.
#[test]
fn a_write_only_some_inputs_make_is_joined_over_the_members() {
    let source = "proc p {n} {set x 1; if {$n} {set c 0} else {set c 1}; \
                      set r [expr {$c ? [incr x] : 7}]; return $x}\n";
    let ints = |value: &LatticeValue| -> Vec<i64> {
        let LatticeValue::ConstSet(members) = value else {
            panic!("a set of constants: {value:?}");
        };
        let mut ints: Vec<i64> = members
            .iter()
            .map(|member| match member {
                ConstValue::Number(NumericValue::Int(n)) => *n,
                other => panic!("an integer: {other:?}"),
            })
            .collect();
        ints.sort_unstable();
        ints
    };
    let values = last_values(source, "::p", &["r", "x"]);
    assert_eq!(
        (ints(&values[0]), ints(&values[1])),
        (vec![2, 7], vec![1, 2])
    );
}

/// An error in the middle of an expression ends the statement with the
/// writes the expression's commands had made. Inside the body of a
/// `catch` the handler is thrown to with them — `x` is 2 after `catch {set
/// r [expr {[incr x] + [error mid]}]}`, and `r` is never set — and where
/// no handler is thrown to the statement is not evaluated at all.
#[test]
fn an_error_in_the_middle_leaves_the_writes_made_before_it_inside_a_catch() {
    let int = |n: i64| LatticeValue::Const(ConstValue::int(n));
    let in_proc = "proc p {} {set x 1; catch {set r [expr {[incr x] + [error mid]}]}; return $x}\n";
    assert_eq!(last_values(in_proc, "::p", &["x"]), [int(2)]);
    let statement = "proc p {} {set x 1; catch {expr {[incr x] + [error mid]}}; return $x}\n";
    assert_eq!(last_values(statement, "::p", &["x"]), [int(2)]);
}

/// What the statement's evaluation cannot own, or is not an evaluation of,
/// keeps the conservative answer: a write to a global, a command the
/// module defines, an error in the middle outside any handler, a host that
/// is a call or a condition, whose substitutions run under the
/// effect-free policy, and a value that is two substitutions rather than
/// one command. A loop carries the write round, so neither the first
/// iteration's value nor the last's is a constant.
#[test]
fn what_the_state_cannot_own_keeps_the_conservative_answer() {
    let widened = |source: &str, names: &[&str]| {
        let values = last_values(source, "::p", names);
        assert!(
            values
                .iter()
                .all(|value| *value == LatticeValue::Overdefined),
            "{source}: {values:?}"
        );
    };
    widened(
        "proc p {} {set x 1; set r [expr {$x + [incr ::g]}]; return $x}\n",
        &["r"],
    );
    widened(
        "proc bump {} {upvar 1 x x; incr x}\nproc p {} {set x 1; set r [expr {[bump] + $x}]; return $x}\n",
        &["r", "x"],
    );
    widened(
        "proc p {} {set x 1; set r [expr {[incr x] + [error mid]}]; return $x}\n",
        &["x"],
    );
    widened(
        "proc p {} {set x 1; puts [expr {$x + [incr x]}]; return $x}\n",
        &["x"],
    );
    widened(
        "proc p {} {set x 1; if {[incr x] > 1} {puts a}; return $x}\n",
        &["x"],
    );
    widened(
        "proc p {} {set r [expr {[incr x] + 1}]; return $x}\n",
        &["x"],
    );
    // A place no statement reads or defines ahead of the call has no value
    // to keep where the expression leaves it alone.
    widened(
        "proc p {} {set r [expr {0 && [set y 3]}]; return 1}\n",
        &["y"],
    );
    // A module that defines `expr` runs its own.
    for source in [
        "proc expr {args} {return 9}\nproc p {} {set x 1; set r [expr \"$x + [incr x]\"]; return $x}\n",
        "proc expr {args} {return 9}\nproc p {} {set x 1; set r [expr {$x + [incr x]}]; return $x}\n",
        "proc expr {args} {return 9}\nproc p {} {set x 1; expr {$x + [incr x]}; return $x}\n",
    ] {
        widened(source, &["x"]);
    }
    // A value that is two substitutions is not one command.
    widened(
        "proc p {} {set x 1; set r [expr {1}][incr x]; return $x}\n",
        &["x"],
    );
    widened(
        "proc p {} {set x 1; set r [incr x][expr {1}]; return $x}\n",
        &["x"],
    );
    widened(
        "proc p {} {set x 0; for {set i 0} {$i < 3} {incr i} {set r [expr {[incr x] * 2}]}; return $x}\n",
        &["r", "x"],
    );
}

/// A call to a command the module cannot see leaves each name live past it
/// a fresh version with no constant value — a plain top-level name, which
/// is the global name, and a procedure's local, which the callee reaches
/// through `upvar 1` — while the version the call holds keeps its value for
/// a read before the call, and a definition after the call is decided
/// again. A name the call follows only in a procedure the module defines
/// keeps its constant.
#[test]
fn an_unseen_call_widens_the_versions_it_leaves_and_no_earlier_or_later_one() {
    let registry = registry();
    let cu = crate::compilation_unit::CompilationUnit::build_for(
        "set x 1\nputs $x\nfoo\nputs $x\nset y 2\nset x 3\nputs $x$y\n",
        &registry,
        false,
    );
    let top = cu.function("::top").expect("top level analysed");
    let symbol = top.ssa.var_symbol("x").expect("x");
    assert!(top.ssa.is_observed_by_unseen_call(symbol, 1));
    assert!(!top.ssa.is_observed_by_unseen_call(symbol, 2));
    assert_eq!(top.ssa.binding_version(symbol, 2), 1);
    assert_eq!(
        top.sccp.values.get(&(symbol, 1)),
        Some(&LatticeValue::Const(ConstValue::int(1)))
    );
    assert_eq!(
        top.sccp.values.get(&(symbol, 2)),
        Some(&LatticeValue::Overdefined)
    );
    assert_eq!(
        last_value(top, "y"),
        LatticeValue::Const(ConstValue::int(2))
    );
    assert_eq!(
        last_value(top, "x"),
        LatticeValue::Const(ConstValue::int(3))
    );
    let cu = crate::compilation_unit::CompilationUnit::build_for(
        "proc p {} { set x 1; foo; puts $x }\n",
        &registry,
        false,
    );
    let p = cu.function("::p").expect("procedure analysed");
    assert_eq!(last_value(p, "x"), LatticeValue::Overdefined);
    let cu = crate::compilation_unit::CompilationUnit::build_for(
        "proc foo {} { puts hi }\nset x 1\nfoo\nputs $x\n",
        &registry,
        false,
    );
    let top = cu.function("::top").expect("top level analysed");
    assert_eq!(
        last_value(top, "x"),
        LatticeValue::Const(ConstValue::int(1))
    );
    // The `::` spelling is externally mutable whatever the call.
    let cu = crate::compilation_unit::CompilationUnit::build_for(
        "set ::x 1\nputs $::x\n",
        &registry,
        false,
    );
    let f = cu.function("::top").expect("top level analysed");
    assert_eq!(last_value(f, "::x"), LatticeValue::Overdefined);
}
