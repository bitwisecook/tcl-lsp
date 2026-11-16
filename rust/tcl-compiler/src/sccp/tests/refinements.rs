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

//! Edge refinements: what an existence guard or a value comparison proves on each edge, and what ends the fact.

use super::*;

/// The existence guard refines its edges: on a may-bound place
/// the true edge of `[info exists x]` carries `Bound(Either)` and the
/// false edge `Unbound`, `!` swaps the two, and past the merge the
/// place is may-bound again. A `global` alias is externally mutable and
/// never refined: both its reads stay may-bound.
#[test]
fn the_existence_guard_refines_its_edges() {
    let registry = registry();
    let cu = crate::compilation_unit::CompilationUnit::build_for(
        "proc f {c} { if {$c} { set x 1 }; if {[info exists x]} { puts $x } else { puts none }; puts $x }\n\
             proc g {c} { if {$c} { set x 1 }; if {![info exists x]} { puts none } else { puts $x }; puts $x }\n\
             proc h {} { global x; if {[info exists x]} { puts $x } else { puts none }; puts $x }\n",
        &registry,
        false,
    );
    let h = cu.function("::h").expect("procedure analysed");
    assert!(h.sccp.refinements.is_empty(), "{:?}", h.sccp.refinements);
    assert_eq!(
        puts_reads(h, "x"),
        vec![Existence::MayBound, Existence::MayBound]
    );
    for proc in ["::f", "::g"] {
        let f = cu.function(proc).expect("procedure analysed");
        assert_eq!(
            puts_reads(f, "x"),
            vec![Existence::Bound(BindingKind::Either), Existence::MayBound],
            "{proc}: the guarded read is bound, the read past the merge may-bound"
        );
        assert_eq!(
            f.sccp.refinements.len(),
            2,
            "{proc}: {:?}",
            f.sccp.refinements
        );
        let unbound = f
            .sccp
            .refinements
            .iter()
            .find(|refinement| {
                refinement.fact == FactView::Domain(DomainFact::Existence(Existence::Unbound))
            })
            .unwrap_or_else(|| panic!("{proc}: {:?}", f.sccp.refinements));
        assert_eq!(
            f.sccp.existence_at(unbound.edge.1, unbound.key),
            Some(Existence::Unbound),
            "{proc}: the other arm holds the place unbound"
        );
    }
}

/// `C1 && C2` refines by both answers only when `C2`, which runs after
/// `C1`, changes no place: `[info exists x] && [info exists y]` binds
/// both, `[otherproc] && [info exists x]` binds `x`, and `[info exists
/// x] && [otherproc]` binds nothing — the procedure may unset `x`
/// through an alias before the edge is taken.
#[test]
fn an_impure_operand_ends_the_facts_before_it() {
    let registry = registry();
    let cu = crate::compilation_unit::CompilationUnit::build_for(
        "proc f {c} { if {$c} { set x 1; set y 1 }; if {[info exists x] && [info exists y]} { puts $x$y } }\n\
             proc g {c} { if {$c} { set x 1 }; if {[otherproc] && [info exists x]} { puts $x } }\n\
             proc h {c} { if {$c} { set x 1 }; if {[info exists x] && [otherproc]} { puts $x } }\n",
        &registry,
        false,
    );
    let bound = vec![Existence::Bound(BindingKind::Either)];
    let f = cu.function("::f").expect("procedure analysed");
    assert_eq!(puts_reads(f, "x"), bound, "::f x");
    assert_eq!(puts_reads(f, "y"), bound, "::f y");
    let g = cu.function("::g").expect("procedure analysed");
    assert_eq!(puts_reads(g, "x"), bound, "::g");
    let h = cu.function("::h").expect("procedure analysed");
    assert_eq!(puts_reads(h, "x"), vec![Existence::MayBound], "::h");
}

/// An externally mutable place is never refined: the guard on
/// `::errorInfo`, at the top level and in a procedure, and on a
/// `TclOO` instance variable's `if {[info exists x]} {return $x}` —
/// spelled with absolute heads, which keep a method body analysable
/// (its namespace is the receiver's) — leaves each read may-bound, since
/// a call to a procedure the module cannot see may unset the global or
/// the object's variable without any barrier.
#[test]
fn an_externally_mutable_place_is_never_refined() {
    let registry = registry();
    let cu = crate::compilation_unit::CompilationUnit::build_for(
        "if {[info exists ::errorInfo]} {puts $::errorInfo}\n\
             proc p {} { if {[info exists ::errorInfo]} {puts $::errorInfo} }\n\
             oo::class create C {\n variable x\n \
             method m {} { ::if {[::info exists x]} {::return $x}; ::return none }\n}\n",
        &registry,
        false,
    );
    for unit in ["::top", "::p"] {
        let f = cu.function(unit).expect("unit analysed");
        assert!(
            f.sccp.refinements.is_empty(),
            "{unit}: {:?}",
            f.sccp.refinements
        );
        assert_eq!(
            puts_reads(f, "::errorInfo"),
            vec![Existence::MayBound],
            "{unit}"
        );
    }
    let m = cu.methods.values().next().expect("the method analysed");
    assert!(m.sccp.refinements.is_empty(), "{:?}", m.sccp.refinements);
    let x = m.ssa.var_symbol("x").expect("x");
    let returned: Vec<Existence> = m
        .cfg
        .blocks
        .iter()
        .filter_map(|(&id, block)| match &block.terminator {
            Some(Terminator::Return {
                value: Some(value), ..
            }) if value == "${x}" => m.sccp.existence_at_exit(id, x),
            _ => None,
        })
        .collect();
    assert_eq!(returned, vec![Existence::MayBound]);
}

/// A `return` that leaves the procedure is a barrier that writes
/// nothing, so the values its words read keep what the frame proves:
/// `es` is `abc` at `return -code error -errorinfo $es msg`, where an
/// opaque barrier, `eval $script`, widens it.
#[test]
fn a_return_that_leaves_the_procedure_widens_nothing() {
    let registry = registry();
    let value_of_es = |body: &str| {
        let source = format!("proc r {{script}} {{\n    set es abc\n    {body}\n}}\n");
        let cu = crate::compilation_unit::CompilationUnit::build_for(&source, &registry, false);
        let r = cu.function("::r").expect("the procedure analysed");
        let es = r.ssa.var_symbol("es").expect("the variable");
        r.sccp.values.get(&(es, 1)).cloned()
    };
    assert_eq!(
        value_of_es("return -code error -errorinfo $es msg"),
        Some(LatticeValue::Const(ConstValue::String("abc".to_owned())))
    );
    assert_eq!(
        value_of_es("eval $script; return $es"),
        Some(LatticeValue::Overdefined)
    );
}

/// A refinement never survives a barrier: a local its guard
/// refines bound reads bound before `eval $script`, which lowers to a
/// barrier, and may-bound after it — the script may have unset it.
#[test]
fn a_barrier_ends_the_refinement() {
    let registry = registry();
    let cu = crate::compilation_unit::CompilationUnit::build_for(
        "proc m {script c} { if {$c} {set x 1}; if {[info exists x]} { puts $x; eval $script; puts $x } }\n",
        &registry,
        false,
    );
    let m = cu.function("::m").expect("the procedure analysed");
    assert!(
        m.cfg
            .blocks
            .values()
            .flat_map(|block| block.statements.iter())
            .any(|statement| matches!(statement, Statement::Barrier { .. })),
        "the witness needs `eval $script` to lower to a barrier"
    );
    assert_eq!(
        puts_reads(m, "x"),
        vec![Existence::Bound(BindingKind::Either), Existence::MayBound]
    );
}

/// A special variable of the initial global frame is never refined:
/// at the top level of a Tcl 8.6 script `errorCode` enters
/// may-bound — startup binds it only under 8.4 — and `if {![info exists
/// errorCode]}` refines neither edge, since the host, or any command
/// that raises, may set it without a barrier.
#[test]
fn a_negated_guard_never_refines_a_special_variable() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let cu = crate::compilation_unit::CompilationUnit::build_for_dialect(
        "if {![info exists errorCode]} { puts none } else { puts $errorCode }\nputs $errorCode\n",
        registry,
        false,
        "tcl8.6",
    );
    let top = cu.function("::top").expect("top level analysed");
    assert!(
        top.sccp.refinements.is_empty(),
        "{:?}",
        top.sccp.refinements
    );
    assert_eq!(
        puts_reads(top, "errorCode"),
        vec![Existence::MayBound, Existence::MayBound]
    );
}

/// Each refinement of `f`, as the edge it sits on (`true` for the
/// branch's true edge), the place and the fact spelled for comparison.
fn refinement_rows(f: &crate::compilation_unit::FunctionUnit) -> Vec<(bool, String, String)> {
    f.sccp
        .refinements
        .iter()
        .map(|refinement| {
            let polarity = matches!(
                f.cfg.blocks.get(&refinement.edge.0).and_then(|block| block.terminator.as_ref()),
                Some(Terminator::Branch { true_target, .. }) if *true_target == refinement.edge.1
            );
            let text = |value: &ExactValue| String::from_utf8_lossy(&value.bytes).into_owned();
            let fact = match &refinement.fact {
                FactView::Exact(value, _) => format!("= {}", text(value)),
                FactView::Finite(values, _) => format!(
                    "in {}",
                    values.iter().map(text).collect::<Vec<_>>().join(" ")
                ),
                FactView::Domain(DomainFact::Type {
                    intrep: Some(intrep),
                    ..
                }) => format!("type {intrep:?}"),
                FactView::Domain(DomainFact::Range {
                    lo: Some(lo),
                    hi: Some(hi),
                }) if lo == hi => format!("range {lo}"),
                FactView::Domain(DomainFact::Range { lo, hi }) => {
                    let bound = |bound: &Option<i64>| bound.map(|n| n.to_string());
                    format!(
                        "range {}..{}",
                        bound(lo).unwrap_or_default(),
                        bound(hi).unwrap_or_default()
                    )
                }
                FactView::Domain(DomainFact::Existence(existence)) => {
                    format!("exists {existence:?}")
                }
                other => format!("{other:?}"),
            };
            let place = f
                .sccp
                .place_name(&f.ssa, refinement.key.0)
                .unwrap_or("?")
                .to_owned();
            (polarity, place, fact)
        })
        .collect()
}

/// The refinements a case expects: its edge, place and fact.
type Rows = &'static [(bool, &'static str, &'static str)];

/// Each case of the per-shape test: the procedure, its source and the
/// refinements it expects.
const SHAPE_CASES: &[(&str, &str, Rows)] = &[
    (
        "a1",
        "proc a1 {x} {if {$x eq \"a\"} {puts $x}}",
        &[(true, "x", "= a")],
    ),
    (
        "a2",
        "proc a2 {x} {if {$x ne \"a\"} {puts $x}}",
        &[(false, "x", "= a")],
    ),
    (
        "a3",
        "proc a3 {x} {if {\"a\" eq $x} {puts $x}}",
        &[(true, "x", "= a")],
    ),
    (
        "a4",
        "proc a4 {x} {if {$x == \"abc\"} {puts $x}}",
        &[(true, "x", "= abc")],
    ),
    (
        "a5",
        "proc a5 {x} {if {$x == 1} {puts $x}}",
        &[(true, "x", "type Numeric"), (true, "x", "range 1")],
    ),
    (
        "a6",
        "proc a6 {x} {if {$x != 1.5} {puts $x}}",
        &[(false, "x", "type Numeric")],
    ),
    (
        "a7",
        "proc a7 {x} {if {$x in {a b a}} {puts $x}}",
        &[(true, "x", "in a b")],
    ),
    (
        "a8",
        "proc a8 {x} {if {$x ni {a}} {puts $x}}",
        &[(false, "x", "= a")],
    ),
    (
        "a9",
        "proc a9 {x} {if {[string is integer -strict $x]} {puts $x}}",
        &[(true, "x", "type Int")],
    ),
    (
        "a10",
        "proc a10 {x} {if {[string is integer $x]} {puts $x}}",
        &[],
    ),
    (
        "a11",
        "proc a11 {x} {if {!($x eq \"a\")} {puts $x}}",
        &[(false, "x", "= a")],
    ),
    (
        "a12",
        "proc a12 {x y} {if {$x eq \"a\" && $y eq \"b\"} {puts $x$y}}",
        &[(true, "x", "= a"), (true, "y", "= b")],
    ),
    (
        "a13",
        "proc a13 {x y} {if {$x eq \"a\" || $y eq \"b\"} {puts $x$y}}",
        &[],
    ),
    (
        "a14",
        "proc a14 {x y} {if {$x ne \"a\" || $y ne \"b\"} {puts $x$y}}",
        &[(false, "x", "= a"), (false, "y", "= b")],
    ),
    (
        "a15",
        "proc a15 {x} {if {$x eq \"a\" && [foo]} {puts $x}}",
        &[],
    ),
    (
        "a16",
        "proc a16 {x y} {if {$x} {puts $x}; if {$x eq $y} {puts $x$y}}",
        &[],
    ),
    (
        "a17",
        "proc a17 {x} {switch -- $x {a {puts $x} b {puts $x}}}",
        &[(true, "x", "= a"), (true, "x", "= b")],
    ),
    (
        "a18",
        "proc a18 {x} {switch -glob -- $x {a* {puts $x}}}",
        &[],
    ),
    ("a19", "proc a19 {} {if {$::g eq \"a\"} {puts $::g}}", &[]),
    (
        "a20",
        "proc a20 {} {global g; if {$g eq \"a\"} {puts $g}}",
        &[],
    ),
    (
        "a21",
        "proc a21 {a} {if {$a(k) eq \"z\"} {puts $a(k)}}",
        &[],
    ),
    (
        "a22",
        "proc a22 {x} {if {$x < 5} {puts $x}}",
        &[(true, "x", "range ..4"), (false, "x", "range 5..")],
    ),
    (
        "a23",
        "proc a23 {x} {if {5 <= $x} {puts $x}}",
        &[(true, "x", "range 5.."), (false, "x", "range ..4")],
    ),
    (
        "a24",
        "proc a24 {x} {if {$x > 0x10} {puts $x}}",
        &[(true, "x", "range 17.."), (false, "x", "range ..16")],
    ),
    (
        "a25",
        "proc a25 {x} {if {$x >= 010} {puts $x}}",
        &[(true, "x", "range 8.."), (false, "x", "range ..7")],
    ),
    ("a26", "proc a26 {x} {if {$x < false} {puts $x}}", &[]),
    ("a27", "proc a27 {x} {if {$x <= 2.5} {puts $x}}", &[]),
];

/// The per-shape table, row for row, under Tcl 8.6: `eq` refines its
/// true edge and `ne` its false one, either side; `==` with a literal no
/// number spells refines the value, and with a number the type and the
/// integer point, never the value; an ordered comparison with an integer
/// the half-line on each edge, either side, `010` octal under 8.6, and
/// with a boolean word, which compares as a string, or a double nothing;
/// `in` the finite set and `ni` its false edge; `string is` under
/// `-strict` the type, without it nothing;
/// `!` swaps, `&&` keeps both true-edge answers and `||` both
/// false-edge ones, and neither keeps an answer an impure operand after
/// it may undo; the flattened exact `switch` refines each arm. `$x`
/// alone, `$x eq $y`, an opaque `-glob` switch, a qualified name, an
/// alias and an element refine nothing.
#[test]
fn each_shape_refines_the_edge_its_table_row_names() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let cases = SHAPE_CASES;
    let source = cases
        .iter()
        .map(|(_, proc, _)| *proc)
        .collect::<Vec<_>>()
        .join("\n");
    let cu = crate::compilation_unit::CompilationUnit::build_for_dialect(
        &source, registry, false, "tcl8.6",
    );
    for (name, _, expected) in cases {
        let f = cu
            .function(&format!("::{name}"))
            .expect("procedure analysed");
        let mut rows = refinement_rows(f);
        rows.sort();
        let mut expected: Vec<(bool, String, String)> = expected
            .iter()
            .map(|&(polarity, place, fact)| (polarity, place.to_owned(), fact.to_owned()))
            .collect();
        expected.sort();
        assert_eq!(rows, expected, "{name}");
    }
}

/// The `puts` statements of `f`, in source order, with the block each
/// sits in.
fn puts_blocks(f: &crate::compilation_unit::FunctionUnit) -> Vec<BlockId> {
    let mut found: Vec<(u32, BlockId)> = f
        .cfg
        .blocks
        .iter()
        .flat_map(|(&id, block)| {
            block
                    .statements
                    .iter()
                    .filter(|statement| {
                        matches!(statement, Statement::Call { command, .. } if command == "puts")
                    })
                    .map(move |statement| (statement.span().start(), id))
        })
        .collect();
    found.sort_unstable();
    found.into_iter().map(|(_, id)| id).collect()
}

/// A value refinement holds wherever every executable path carries it:
/// in its arm and past a merge inside the arm, whose two paths both
/// cross the refined edge, and not past the merge of the refined arm
/// with the other one, where the version reads as its own value. A
/// barrier — `time $script` runs the script here, which may set `x`
/// without a new version — ends it, in its own block and after.
#[test]
fn a_value_refinement_holds_where_every_path_carries_it() {
    let registry = registry();
    let cu = crate::compilation_unit::CompilationUnit::build_for(
        "proc p {x c} {if {$x eq \"a\"} {if {$c} {puts 1} else {puts 2}; puts $x}; puts $x}\n\
             proc m {x script} {if {$x eq \"a\"} {puts $x; time $script; puts $x}}\n",
        &registry,
        false,
    );
    let a = LatticeValue::Const(ConstValue::String("a".to_owned()));
    let p = cu.function("::p").expect("procedure analysed");
    let x = (p.ssa.var_symbol("x").expect("x"), 0);
    let held: Vec<Option<&LatticeValue>> = puts_blocks(p)
        .into_iter()
        .map(|block| p.sccp.value_at(block, x))
        .collect();
    assert_eq!(
        held,
        vec![
            Some(&a),
            Some(&a),
            Some(&a),
            Some(&LatticeValue::Overdefined)
        ],
        "both inner arms, the inner merge, then past the outer merge"
    );
    let m = cu.function("::m").expect("procedure analysed");
    let x = (m.ssa.var_symbol("x").expect("x"), 0);
    assert!(
        m.cfg
            .blocks
            .values()
            .flat_map(|block| block.statements.iter())
            .any(|statement| matches!(statement, Statement::Barrier { .. })),
        "the witness needs `time $script` to lower to a barrier"
    );
    assert_eq!(m.sccp.refinements.len(), 1, "{:?}", m.sccp.refinements);
    for block in puts_blocks(m) {
        assert_eq!(m.sccp.value_at(block, x), Some(&LatticeValue::Overdefined));
        assert_eq!(m.sccp.refinements_in(block).count(), 0);
    }
}
