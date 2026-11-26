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

//! `incr`, `append`, `lappend` and `dict incr`: the value a cell update leaves, and the trust and plan it needs.

use super::*;

// evaluate_fixture_def for Incr.

fn incr_stmt(
    ssa: &mut SsaFunction,
    name: &str,
    amount: Option<&str>,
    old_ver: u32,
    new_ver: u32,
) -> SsaStatement {
    let sym = ssa.intern_var(name);
    let mut uses = HashMap::new();
    uses.insert(sym, old_ver);
    let mut defs = HashMap::new();
    defs.insert(sym, new_ver);
    SsaStatement {
        statement: Statement::Incr {
            span: Span::new(0, 0),
            name: name.into(),
            name_braced: false,
            amount: amount.map(String::from),
            amount_braced: false,
            safe_on_uninit: false,
        },
        uses,
        defs,
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    }
}

#[test]
fn evaluate_def_incr_default_amount() {
    // x@1 = Const(Int(5)); `incr x` → x@2 = Const(Int(6)).
    let mut ssa = bare_ssa();
    let stmt = incr_stmt(&mut ssa, "x", None, 1, 2);
    let x = ssa.var_symbol("x").unwrap();
    let mut values = HashMap::new();
    values.insert((x, 1), LatticeValue::Const(ConstValue::int(5)));
    assert_eq!(
        evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
        LatticeValue::Const(ConstValue::int(6))
    );
}

#[test]
fn evaluate_def_incr_integer_literal_amount() {
    let mut ssa = bare_ssa();
    let stmt = incr_stmt(&mut ssa, "x", Some("10"), 1, 2);
    let x = ssa.var_symbol("x").unwrap();
    let mut values = HashMap::new();
    values.insert((x, 1), LatticeValue::Const(ConstValue::int(3)));
    assert_eq!(
        evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
        LatticeValue::Const(ConstValue::int(13))
    );
}

#[test]
fn evaluate_def_incr_negative_literal_amount() {
    let mut ssa = bare_ssa();
    let stmt = incr_stmt(&mut ssa, "x", Some("-2"), 1, 2);
    let x = ssa.var_symbol("x").unwrap();
    let mut values = HashMap::new();
    values.insert((x, 1), LatticeValue::Const(ConstValue::int(10)));
    assert_eq!(
        evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
        LatticeValue::Const(ConstValue::int(8))
    );
}

#[test]
fn evaluate_def_incr_var_ref_amount() {
    // `incr x $y` where $y resolves to 4.
    let mut ssa = bare_ssa();
    let mut stmt = incr_stmt(&mut ssa, "x", Some("$y"), 1, 2);
    let x = ssa.var_symbol("x").unwrap();
    let y = ssa.intern_var("y");
    stmt.uses.insert(y, 1);
    let mut values = HashMap::new();
    values.insert((x, 1), LatticeValue::Const(ConstValue::int(6)));
    values.insert((y, 1), LatticeValue::Const(ConstValue::int(4)));
    assert_eq!(
        evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
        LatticeValue::Const(ConstValue::int(10))
    );
}

#[test]
fn evaluate_def_incr_unknown_base_propagates_unknown() {
    let mut ssa = bare_ssa();
    let stmt = incr_stmt(&mut ssa, "x", None, 1, 2);
    let values = HashMap::new();
    // No entry for x@1 → base is Unknown → result Unknown.
    assert_eq!(
        evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
        LatticeValue::Unknown
    );
}

/// Without the whole-module trust fact there is no evidence a head still
/// denotes its registry command, so no resolved command answers — the
/// typed `incr` included; the same statement under the pristine stance
/// folds (#2164).
#[test]
fn evaluate_def_without_the_trust_fact_answers_for_no_command() {
    let mut ssa = bare_ssa();
    let stmt = incr_stmt(&mut ssa, "x", None, 1, 2);
    let x = ssa.var_symbol("x").unwrap();
    let mut values = HashMap::new();
    values.insert((x, 1), LatticeValue::Const(ConstValue::int(5)));
    assert_eq!(
        evaluate_fixture_def(
            &stmt,
            &values,
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Overdefined
    );
    assert_eq!(
        evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
        LatticeValue::Const(ConstValue::int(6))
    );
}

#[test]
fn evaluate_def_incr_overdefined_base_widens() {
    let mut ssa = bare_ssa();
    let stmt = incr_stmt(&mut ssa, "x", None, 1, 2);
    let x = ssa.var_symbol("x").unwrap();
    let mut values = HashMap::new();
    values.insert((x, 1), LatticeValue::Overdefined);
    assert_eq!(
        evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
        LatticeValue::Overdefined
    );
}

#[test]
fn evaluate_def_incr_non_integer_amount_widens() {
    let mut ssa = bare_ssa();
    let stmt = incr_stmt(&mut ssa, "x", Some("2.5"), 1, 2);
    let x = ssa.var_symbol("x").unwrap();
    let mut values = HashMap::new();
    values.insert((x, 1), LatticeValue::Const(ConstValue::int(1)));
    assert_eq!(
        evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
        LatticeValue::Overdefined
    );
}

/// A `Call` whose resolved plan is a cell update, in the shape the
/// lowering gives `append` / `lappend`: the target is the first
/// argument, a def, and a read of its own prior version.
fn cell_update_call(
    ssa: &mut SsaFunction,
    command: &str,
    args: &[&str],
    old_ver: u32,
    new_ver: u32,
) -> SsaStatement {
    let target = ssa.intern_var(args[0]);
    let mut uses = HashMap::new();
    uses.insert(target, old_ver);
    let mut defs = HashMap::new();
    defs.insert(target, new_ver);
    SsaStatement {
        statement: Statement::Call {
            span: Span::new(0, 0),
            command: command.into(),
            canonical_command: None,
            args: args.iter().map(|a| (*a).to_owned()).collect(),
            defs: vec![args[0].to_owned()],
            reads: Vec::new(),
            reads_own_defs: true,
            safe_on_uninit: true,
            tokens: None,
            foreach_groups: None,
        },
        uses,
        defs,
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    }
}

/// `append s " world"` over `s@1 = hello`: the registry's cell append
/// writes `hello world`, byte-exact, and the def takes the store.
#[test]
fn evaluate_def_append_call_folds_through_the_cell_update() {
    assert_eq!(
        settled(
            "tcl8.6",
            "proc p {} {set s hello; append s \" world\" !; return $s}",
            "s"
        ),
        LatticeValue::Const(ConstValue::String("hello world!".into()))
    );
}

/// `append s $p` reads the piece from the lattice, exactly: `set p
/// { again}` keeps its leading space (#2052).
#[test]
fn evaluate_def_append_var_piece_reads_the_lattice_exactly() {
    // tclsh 8.6.18 and 9.0.4 print `<hello again> <a b {c d}>
    // <helloCHANGED>` for these three bodies: a read trace may replace
    // the piece, so a traced piece settles nothing.
    let text = |value: &str| LatticeValue::Const(ConstValue::String(value.into()));
    assert_eq!(
        settled(
            "tcl8.6",
            "proc p {} {set s hello; set p { again}; append s $p; return $s}",
            "s"
        ),
        text("hello again")
    );
    assert_eq!(
        settled(
            "tcl8.6",
            "proc p {} {set l {a b}; set e {c d}; lappend l $e; return $l}",
            "l"
        ),
        text("a b {c d}")
    );
    assert_eq!(
        settled(
            "tcl8.6",
            "proc t args {}\nproc p {} {set s hello; set q x; trace add variable q read t; append s $q; return $s}",
            "s"
        ),
        LatticeValue::Overdefined
    );
    assert_eq!(
        settled(
            "tcl8.6",
            "proc p {q} {set s hello; append s $q; return $s}",
            "s"
        ),
        LatticeValue::Overdefined
    );
}

/// `lappend l c {d e}` over `l@1 = a b`: the list append renders
/// canonically; over a value that is not a list it is the program's
/// error, so the def widens.
#[test]
fn evaluate_def_lappend_call_renders_the_canonical_list() {
    assert_eq!(
        settled(
            "tcl8.6",
            "proc p {} {set l {a b}; lappend l c {d e}; return $l}",
            "l"
        ),
        LatticeValue::Const(ConstValue::String("a b c {d e}".into()))
    );
    // tclsh 8.6.18: `set l "\{"; lappend l c` raises `unmatched open
    // brace in list`.
    assert_eq!(
        settled(
            "tcl8.6",
            "proc p {} {set l \"\\{\"; lappend l c; return $l}",
            "l"
        ),
        LatticeValue::Overdefined
    );
    // A prior nothing proves is never a manufactured empty list.
    assert_eq!(
        settled("tcl8.6", "proc p {l} {lappend l c; return $l}", "l"),
        LatticeValue::Overdefined
    );
}

/// `dict incr d a` over `d@1 = a 1 b 2`: the keyed update rewrites the
/// dictionary the variable holds, and the call's one definition takes
/// it (tclsh 8.5 to 9.1 give `a 2 b 2`). A prior the lattice cannot
/// prove leaves the definition unknown or widened, never an empty
/// dictionary.
#[test]
fn evaluate_def_dict_incr_writes_the_dictionary() {
    assert_eq!(
        settled(
            "tcl8.6",
            "proc p {} {set d {a 1 b 2}; dict incr d a; return $d}",
            "d"
        ),
        LatticeValue::Const(ConstValue::String("a 2 b 2".into()))
    );
    assert_eq!(
        settled("tcl8.6", "proc p {d} {dict incr d a; return $d}", "d"),
        LatticeValue::Overdefined
    );
}

/// A generic call with a def but no declared plan keeps the
/// conservative answer.
#[test]
fn evaluate_def_call_without_a_plan_widens() {
    let mut ssa = bare_ssa();
    let stmt = cell_update_call(&mut ssa, "gets", &["chan", "line"], 1, 2);
    let chan = ssa.var_symbol("chan").unwrap();
    let mut values = HashMap::new();
    values.insert(
        (chan, 1),
        LatticeValue::Const(ConstValue::String("stdin".into())),
    );
    assert_eq!(
        evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
        LatticeValue::Overdefined
    );
}

/// The correlated finite-set limit: one finite input evaluates per
/// member; two distinct finite inputs decline; two reads of one SSA
/// value are one distinct input.
#[test]
fn evaluate_def_incr_lifts_over_one_finite_input() {
    let ints = |members: [i64; 2]| {
        LatticeValue::ConstSet(members.into_iter().map(ConstValue::int).collect())
    };
    assert_eq!(
        settled_at(
            "tcl8.6",
            "proc p {c} {if {$c} {set x 1} else {set x 2}; incr x 10; return $x}",
            "x",
            2
        ),
        ints([11, 12])
    );
    assert_eq!(
        settled_at(
            "tcl8.6",
            "proc p {c} {if {$c} {set x 1; set a 10} else {set x 2; set a 20}; incr x $a; return $x}",
            "x",
            2
        ),
        LatticeValue::Overdefined,
        "two distinct finite inputs are correlated"
    );
    // `incr x $x`: the step reads the target's own version.
    assert_eq!(
        settled_at(
            "tcl8.6",
            "proc p {c} {if {$c} {set x 1} else {set x 2}; incr x $x; return $x}",
            "x",
            2
        ),
        ints([2, 4])
    );
}

/// The release rules reach the lattice through the registry's route:
/// a leading-zero base reads as octal up to 8.6 and decimal from 9.0,
/// and declines under a profile naming no release (the lenient `tcl`);
/// past the wide boundary 8.5 onward widens, 8.4 and an unnamed release
/// decline. `f5-irules` reads as its declared 8.4 base does (ruling 8):
/// tclsh 8.4 prints 9 for `set x 010; incr x`, and -8 past the wide
/// boundary, which no model computes.
#[test]
fn evaluate_def_incr_reads_the_base_under_the_targets_release() {
    let under =
        |dialect: &str, base: &str| settled_body(dialect, &format!("set x {base}; incr x"), "x");
    let int = |value: i64| LatticeValue::Const(ConstValue::int(value));
    assert_eq!(under("tcl8.4", "010"), int(9));
    assert_eq!(under("tcl8.6", "010"), int(9));
    assert_eq!(under("tcl9.0", "010"), int(11));
    assert_eq!(under("f5-irules", "010"), int(9));
    assert_eq!(under("tcl", "010"), LatticeValue::Overdefined);

    let max = "9223372036854775807";
    assert_eq!(
        under("tcl8.6", max),
        LatticeValue::Const(ConstValue::String("9223372036854775808".into()))
    );
    assert_eq!(under("tcl8.4", max), LatticeValue::Overdefined);
    assert_eq!(under("f5-irules", max), LatticeValue::Overdefined);
    assert_eq!(under("tcl", max), LatticeValue::Overdefined);
    // A whitespace-padded step is an integer in every release.
    assert_eq!(settled_body("tcl8.6", "set x 1; incr x { 5}", "x"), int(6));
}

/// `set x [incr n]`: the host statement's uses do not hold `n` (its read
/// sits on the synthetic call ahead of the host), so the prior store is
/// a permanent miss and the definition widens. Reading version 0 found
/// no value and answered a pending that never resolved, and a join
/// then took the other arm's constant for `x` (`f 1` prints 2 under
/// 8.4 to 9.1, where `tcl opt` had rewritten `puts $x` to `puts 5`).
#[test]
fn a_missing_use_declines_the_prior_store() {
    let mut ssa = bare_ssa();
    let n = ssa.intern_var("n");
    let stmt = assign_value_stmt(&mut ssa, "x", "[incr n]", 1);
    assert!(!stmt.uses.contains_key(&n), "the host holds no use of n");
    let mut values = HashMap::new();
    values.insert((n, 1), LatticeValue::Const(ConstValue::int(1)));
    assert_eq!(
        evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
        LatticeValue::Overdefined,
        "never the pending bottom a join would launder"
    );
    // The statement-position update holds its target's use and folds.
    let incr = incr_stmt(&mut ssa, "n", None, 1, 2);
    assert_eq!(
        evaluate_pristine(&incr, &values, &ssa, FoldPolicy::default()),
        LatticeValue::Const(ConstValue::int(2))
    );
}
