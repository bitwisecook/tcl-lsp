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

//! The solver end to end: φ joins, barriers, executable edges, constant branches, loop summaries and widening across opaque effects.

use super::*;

#[test]
fn phi_join_widens_version_zero_livein_to_overdefined() {
    // A phi merging a version-0 live-in (a proc parameter / global /
    // other caller-supplied root) with a defined-arm constant must widen
    // to Overdefined — the caller's value cannot vanish from the merge.
    // Otherwise `if {$c} { set x 5 }; if {$x == 5} {A} else {B}` folds the
    // second test always-true and deletes the else arm. Mirrors the
    // interval pass joining TOP for `inc == 0` (intervals.rs:570-574).
    let mut ssa = bare_ssa();
    let x = ssa.intern_var("x");
    let mut block = empty_ssa_block("merge");
    block.phis.push(crate::ssa::Phi {
        name: x,
        version: 2,
        incoming: HashMap::from([(BlockId(1), 0), (BlockId(2), 1)]),
    });
    // `seed_live_in_roots` pins the version-0 root Overdefined.
    let mut values: HashMap<ValueKey, LatticeValue> = HashMap::new();
    values.insert((x, 0), LatticeValue::Overdefined);
    values.insert((x, 1), LatticeValue::Const(ConstValue::Int(5)));
    assert!(sccp_process_phis(
        &mut values,
        &block,
        &[BlockId(1), BlockId(2)]
    ));
    assert_eq!(values.get(&(x, 2)), Some(&LatticeValue::Overdefined));
}

#[test]
fn barrier_own_defs_become_overdefined() {
    // A barrier (e.g. `dict for {x y} $d {}`) defines its
    // own variables. Those defs must be set Overdefined so they participate
    // in a downstream phi join; otherwise the def key is never inserted and
    // a phi merging the barrier-def with a constant folds to that constant.
    let mut ssa = bare_ssa();
    let x = ssa.intern_var("x");
    let mut block = empty_ssa_block("barrier_block");
    let mut defs: HashMap<Symbol, crate::ssa::Version> = HashMap::new();
    defs.insert(x, 2);
    block.statements.push(SsaStatement {
        statement: Statement::Barrier {
            span: Span::new(0, 0),
            reason: "dict-for".into(),
            command: "::tcl::dict::for".into(),
            canonical_command: None,
            args: vec!["d".into()],
            tokens: None,
        },
        uses: HashMap::new(),
        defs,
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    });
    let mut values: HashMap<ValueKey, LatticeValue> = HashMap::new();
    let escaping: HashSet<String> = HashSet::new();
    assert!(sccp_process_statements(
        &mut values,
        &block,
        &ssa,
        StatementInputs {
            clobbers: None,
            store: &StoreInputs {
                reference_config: Some(tcl_lexer::LexerConfig::default()),
                math_dependencies: None,
                cfg: None,
                escaping: &escaping,
                policy: FoldPolicy::default(),
                has_dynamic_variable_trace: false,
                opaque_native_accesses: false,
                folds: None,
                registry: &CommandRegistry::build_default(),
            },
        }
    ));
    assert_eq!(
        values.get(&(x, 2)),
        Some(&LatticeValue::Overdefined),
        "a barrier-defined variable must be Overdefined, not absent/Unknown"
    );
}

#[test]
fn phi_join_defaults_missing_version_zero_feed_to_overdefined() {
    // Defence in depth: even if a version-0 feed was never seeded, the
    // join must treat it as Overdefined rather than skipping it (which
    // would leave the phi holding the defined-arm constant).
    let mut ssa = bare_ssa();
    let x = ssa.intern_var("x");
    let mut block = empty_ssa_block("merge");
    block.phis.push(crate::ssa::Phi {
        name: x,
        version: 2,
        incoming: HashMap::from([(BlockId(1), 0), (BlockId(2), 1)]),
    });
    let mut values: HashMap<ValueKey, LatticeValue> = HashMap::new();
    values.insert((x, 1), LatticeValue::Const(ConstValue::Int(5)));
    assert!(sccp_process_phis(
        &mut values,
        &block,
        &[BlockId(1), BlockId(2)]
    ));
    assert_eq!(values.get(&(x, 2)), Some(&LatticeValue::Overdefined));
}

#[test]
fn sccp_marks_entry_executable_and_propagates_const() {
    // entry: set x 42
    let mut f = Function::new("::top", "entry");
    let entry = f.entry;
    f.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });
    let mut ssa = make_ssa(&f, vec![]);
    let stmt = assign_const_stmt(&mut ssa, "x", "42", 1);
    ssa.blocks.get_mut(&entry).unwrap().statements.push(stmt);
    let x = ssa.var_symbol("x").unwrap();

    let r = sccp_no_traces(&f, &ssa, None, FoldPolicy::default());
    assert!(r.executable_blocks.contains(&entry));
    assert_eq!(
        r.values.get(&(x, 1)),
        Some(&LatticeValue::Const(ConstValue::Int(42)))
    );
}

#[test]
fn sccp_constant_branch_detected_and_taken_target_marked() {
    // entry: branch on literal "1" → true → "t", false → "e"
    let mut f = Function::new("::top", "entry");
    let entry = f.entry;
    let t = block(&mut f, "t");
    let e = block(&mut f, "e");
    f.blocks.get_mut(&entry).unwrap().terminator = Some(branch(literal("1"), t, e));
    f.blocks.get_mut(&t).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });
    f.blocks.get_mut(&e).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });
    let ssa = make_ssa(&f, vec![]);

    let r = sccp_no_traces(&f, &ssa, None, FoldPolicy::default());
    assert!(r.executable_blocks.contains(&t));
    assert!(!r.executable_blocks.contains(&e));
    assert_eq!(r.constant_branches.len(), 1);
    let cb = &r.constant_branches[0];
    assert!(cb.value);
    assert_eq!(cb.taken_target, "t");
    assert_eq!(cb.not_taken_target, "e");
}

#[test]
fn sccp_false_branch_prunes_true_target() {
    let mut f = Function::new("::top", "entry");
    let entry = f.entry;
    let t = block(&mut f, "t");
    let e = block(&mut f, "e");
    f.blocks.get_mut(&entry).unwrap().terminator = Some(branch(literal("0"), t, e));
    f.blocks.get_mut(&t).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });
    f.blocks.get_mut(&e).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });
    let ssa = make_ssa(&f, vec![]);

    let r = sccp_no_traces(&f, &ssa, None, FoldPolicy::default());
    assert!(!r.executable_blocks.contains(&t));
    assert!(r.executable_blocks.contains(&e));
}

#[test]
fn sccp_unknown_branch_executes_both_targets() {
    // Var reference — lattice value defaults to Unknown → decision None.
    let mut f = Function::new("::top", "entry");
    let entry = f.entry;
    let t = block(&mut f, "t");
    let e = block(&mut f, "e");
    let cond = ExprNode::Var {
        text: "$z".into(),
        name: "z".into(),
        start: 0,
        end: 2,
    };
    f.blocks.get_mut(&entry).unwrap().terminator = Some(branch(cond, t, e));
    f.blocks.get_mut(&t).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });
    f.blocks.get_mut(&e).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });
    let ssa = make_ssa(&f, vec![]);

    let r = sccp_no_traces(&f, &ssa, None, FoldPolicy::default());
    assert!(r.executable_blocks.contains(&t));
    assert!(r.executable_blocks.contains(&e));
    assert_eq!(r.constant_branches.len(), 0);
}

#[test]
fn sccp_folds_post_loop_branch_via_static_summary() {
    // After `for {set i 0} {$i < 10} {incr i} {}` tclsh leaves `i == 10`,
    // so the following `if {$i == 10}` is statically true. SCCP cannot fold
    // a loop-carried phi, but the static-loop summary simulates the loop
    // and folds the branch. Verified against tclsh 8.4-9.0 (i == 10, and
    // the accumulator j == 5, hold after the loops).
    let c = cu(
        "proc ::p {} { for {set i 0} {$i < 10} {incr i} {}\n if {$i == 10} { return yes } else { return no } }",
    );
    let fu = c.function("::p").unwrap();
    let r = &fu.semantic_values().unwrap().result;
    let cb = r
            .constant_branches
            .iter()
            .find(|cb| cb.condition.contains("$i == 10"))
            .unwrap_or_else(|| {
                panic!(
                    "post-loop branch must fold via the static-loop summary: facts={:#?} loops={:#?} preparations={:#?} sources={:#?}",
                    fu.semantic_values(),
                    fu.cfg.loop_nodes,
                    fu.cfg.expression_preparations,
                    fu.cfg.terminator_sources
                )
            });
    assert!(cb.value, "i == 10 after the loop, so the branch is true");
    let facts = fu.semantic_values().unwrap();
    assert!(
        facts.loops.values().any(|analysis| {
            !analysis.expression_obligations.is_empty()
                && !analysis.increment_obligations.is_empty()
        }),
        "semantic loop contents retain native operations in evaluation order"
    );
    assert!(
        !fu.sccp
            .constant_branches
            .iter()
            .any(|branch| branch.condition.contains("$i == 10")),
        "numeric simulation does not erase loop or branch conversions"
    );

    // Body side effects are simulated too: j accumulates to 5.
    let ca = cu(
        "proc ::a {} { set j 0\n for {set k 5} {$k > 0} {incr k -1} { incr j }\n if {$j == 5} { return yes } else { return no } }",
    );
    let fa = ca.function("::a").unwrap();
    let ra = &fa.semantic_values().unwrap().result;
    let cba = ra
        .constant_branches
        .iter()
        .find(|cb| cb.condition.contains("$j == 5"))
        .expect("accumulator branch must fold via the static-loop summary");
    assert!(cba.value, "j == 5 after the loop");
    assert!(
        !fa.sccp
            .constant_branches
            .iter()
            .any(|branch| branch.condition.contains("$j == 5")),
        "accumulator precision is not execution-erasure proof"
    );

    // A loop with an unknown (parameter) bound cannot be summarised, so the
    // post-loop branch stays unfolded (conservative).
    let cq = cu(
        "proc ::q {n} { for {set i 0} {$i < $n} {incr i} {}\n if {$i == 10} { return yes } else { return no } }",
    );
    let fq = cq.function("::q").unwrap();
    let rq = &fq.semantic_values().unwrap().result;
    assert!(
        !rq.constant_branches
            .iter()
            .any(|cb| cb.condition.contains("$i == 10")),
        "an unknown loop bound must not fold the post-loop branch"
    );
}

#[test]
fn sccp_foreach_nested_braced_list_constset_not_corrupted() {
    // End to end: `foreach v {{a b} {c d}}` lowers `list_arg` to the
    // segmenter-stripped text `{a b} {c d}` (only the word's own outer
    // braces are gone). Peeling a *second* level of bracing off this
    // already-stripped text corrupts `v`'s CONSTSET to the two elements
    // `a` and `b}` (from the lenient split of `a b} {c d`) and wrongly
    // proves `if {$v eq "c d"}` false. It must fold to the two correct
    // elements `a b` and `c d` instead.
    let c = cu(
        "proc ::p {} { foreach v {{a b} {c d}} { if {$v eq \"c d\"} { set r yes } else { set r no } } }",
    );
    let fu = c.function("::p").unwrap();
    let r = sccp_no_traces(&fu.cfg, &fu.ssa, None, FoldPolicy::default());
    let const_sets: Vec<_> = r
        .values
        .iter()
        .filter(|((symbol, _), _)| fu.ssa.var_name(*symbol) == "v")
        .filter_map(|(_, val)| match val {
            LatticeValue::ConstSet(vs) => Some(vs.clone()),
            _ => None,
        })
        .collect();
    assert!(
        !const_sets
            .iter()
            .any(|vs| vs.contains(&ConstValue::String("b}".into()))),
        "v's CONSTSET must not contain the corrupted element \"b}}\": {const_sets:?}"
    );
    assert!(
        const_sets.iter().any(|vs| vs.len() == 2
            && vs.contains(&ConstValue::String("a b".into()))
            && vs.contains(&ConstValue::String("c d".into()))),
        "expected a CONSTSET {{\"a b\", \"c d\"}} among v's lattice values, got {const_sets:?}"
    );
}

#[test]
fn branch_deferrable_optimism() {
    let cond = ExprNode::Var {
        text: "$x".into(),
        name: "x".into(),
        start: 0,
        end: 2,
    };
    let mut ssa = bare_ssa();
    let x = ssa.intern_var("x");
    let mut sb = empty_ssa_block("b");
    sb.exit_versions.insert(x, 1);
    let mut values: HashMap<ValueKey, LatticeValue> = HashMap::new();

    // Defined operand (version 1) not yet computed → defer.
    assert!(branch_deferrable(
        &sb,
        &cond,
        &values,
        &ssa,
        tcl_dialect::LexerGrammar::default()
    ));
    values.insert((x, 1), LatticeValue::Unknown);
    assert!(branch_deferrable(
        &sb,
        &cond,
        &values,
        &ssa,
        tcl_dialect::LexerGrammar::default()
    ));

    // An `Overdefined` operand proves the condition genuinely
    // non-constant → never defer.
    values.insert((x, 1), LatticeValue::Overdefined);
    assert!(!branch_deferrable(
        &sb,
        &cond,
        &values,
        &ssa,
        tcl_dialect::LexerGrammar::default()
    ));

    // A constant operand folds via `evaluate_branch`, so the `None`
    // arm is never reached → not deferrable here.
    values.insert((x, 1), LatticeValue::Const(ConstValue::Int(1)));
    assert!(!branch_deferrable(
        &sb,
        &cond,
        &values,
        &ssa,
        tcl_dialect::LexerGrammar::default()
    ));

    // Version-0 operands (parameters / globals / live-in roots) are
    // already `Overdefined` and excluded from the deferral test.
    let mut sb0 = empty_ssa_block("b");
    sb0.exit_versions.insert(x, 0);
    assert!(!branch_deferrable(
        &sb0,
        &cond,
        &HashMap::new(),
        &ssa,
        tcl_dialect::LexerGrammar::default()
    ));
}

#[test]
fn canonical_root_cells_fold_until_a_proved_mutation_dependency() {
    let stable = cu("set a 3; set b 4; if {$a+$b == 7} {set result YES} else {set result NO}");
    assert!(
        !stable
            .top_level
            .semantic_values()
            .unwrap()
            .result
            .constant_branches
            .is_empty(),
        "root-qualified identity alone does not imply mutation; values={:?}; cells={:?}",
        stable.top_level.sccp.values,
        stable.top_level.ssa.cell_names()
    );
    assert!(
        stable.top_level.sccp.constant_branches.is_empty(),
        "known branch values do not prove retained coercion erasure"
    );
    let observed = cu(
        "set a 3; proc observer {args} {set ::a 4}; trace add variable a read observer; if {$a == 3} {set result YES} else {set result NO}",
    );
    assert!(
        observed
            .top_level
            .semantic_values()
            .unwrap()
            .result
            .constant_branches
            .is_empty(),
        "a read observer may mutate the stored value"
    );
    let mutated = cu(
        "set a 3; proc mutate {} {global a; set a 4}; mutate; if {$a == 3} {set result YES} else {set result NO}",
    );
    assert!(
        !mutated
            .top_level
            .semantic_values()
            .unwrap()
            .result
            .constant_branches
            .iter()
            .any(|branch| branch.value),
        "re-entry must never reuse the stale root value"
    );
}

#[test]
fn positioned_constants_do_not_cross_unrepresented_mutations_or_observers() {
    for source in [
        "proc f {cmd} {set x 3; $cmd; if {$x == 3} {return YES} else {return NO}}",
        "proc f {n} {set x 3; set $n 4; if {$x == 3} {return YES} else {return NO}}",
        "proc f {n} {set x 3; unset $n; if {$x == 3} {return YES} else {return NO}}",
        "proc f {n} {set x 3; trace add variable $n read observer; if {$x == 3} {return YES} else {return NO}}",
    ] {
        let unit = cu(source);
        let function = unit.function("::f").expect("procedure retained");
        assert!(
            function
                .sccp
                .constant_branches
                .iter()
                .all(|branch| !branch.value),
            "an uncertain later read must not reuse the earlier value: {source}; branches={:?}",
            function.sccp.constant_branches,
        );
    }
}

#[test]
fn later_opaque_effects_do_not_destroy_an_earlier_branch_proof() {
    let unit = cu(
        "proc f {cmd} {set x 3; if {$x == 3} {set first YES} else {set first NO}; $cmd; if {$x == 4} {set second YES} else {set second NO}}",
    );
    let function = unit.function("::f").expect("procedure retained");
    assert!(
        function.sccp.constant_branches.is_empty(),
        "unknown retained conversions prevent execution erasure"
    );
    assert!(
        function
            .semantic_values()
            .unwrap()
            .result
            .constant_branches
            .iter()
            .any(|branch| branch.value && branch.condition.contains('3')),
        "the earlier read remains precise: {:?}",
        function.semantic_values().unwrap().result.constant_branches
    );
    assert!(
        !function
            .semantic_values()
            .unwrap()
            .result
            .constant_branches
            .iter()
            .any(|branch| branch.value && branch.condition.contains('4')),
        "the later read has an unknown contents origin: {:?}",
        function.semantic_values().unwrap().result.constant_branches
    );
}

#[test]
fn global_alias_contents_are_precise_until_an_actual_reentry_dependency() {
    // Both controls are independently executed in the six-engine temporal
    // vector corpus. Sharing a cell alone does not change its contents.
    let direct = cu("proc p {} {global g; set g 5; if {$g == 5} {return YES} else {return NO}}; p");
    let function = direct.function("::p").unwrap();
    assert!(
        function.sccp.constant_branches.is_empty(),
        "alias contents alone do not erase numeric read conversions"
    );
    assert!(
        function
            .semantic_values()
            .unwrap()
            .result
            .constant_branches
            .iter()
            .any(|branch| branch.value),
        "an unobserved literal store followed immediately by its read is precise"
    );

    let mutated = cu(
        "proc mutate {} {global g; set g 6}; proc p {} {global g; set g 5; mutate; if {$g == 5} {return YES} else {return NO}}; p",
    );
    let function = mutated.function("::p").unwrap();
    assert!(
        !function
            .semantic_values()
            .unwrap()
            .result
            .constant_branches
            .iter()
            .any(|branch| branch.value),
        "the alias read must use the contents after reentry"
    );
}

/// Regression: a literal-body `uplevel #0 {…}` (the CFG shape `Statement::
/// UpFrame`) evaluates its body in the absolute global frame, which can
/// reassign any name visible there — including one with no `global`/
/// `variable`/`upvar`/`trace` declaration at all. SCCP must widen every
/// tracked value across it exactly as it already does for a plain
/// `Statement::Barrier`. Confirmed against tclsh 8.6/9.0: `set n 5;
/// uplevel #0 {set n 99}; if {$n == 5} {…}` takes the *else* branch
/// (`n` is 99), so SCCP must not fold this to a constant-true branch.
#[test]
fn sccp_widens_across_upframe_from_literal_uplevel() {
    let with_upframe =
        cu("set n 5\nuplevel #0 { set n 99 }\nif {$n == 5} { set r yes } else { set r no }\n");
    let f = with_upframe.function("::top").unwrap();
    let r = sccp_no_traces(&f.cfg, &f.ssa, None, FoldPolicy::default());
    assert_eq!(
        r.constant_branches.len(),
        0,
        "a value reachable through an UpFrame must not fold a constant branch, got {:?}",
        r.constant_branches,
    );
}
