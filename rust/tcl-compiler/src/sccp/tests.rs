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

use crate::interprocedural::test_body_identity::fixture_id_if_present;

use super::*;

/// Whitespace inside a Tcl word is data, so the lattice keeps the exact
/// spelling (#2052).
///
/// Trimming corrupted every consumer: `set p { again}` reached the lattice
/// as `again`, so `append s $p` was rewritten to `append s again` and the
/// program printed `helloagain` where tclsh 9.0.4 prints `hello again`.
/// `string length $p` folded to 5 against the true 6.
#[test]
fn a_literal_keeps_its_surrounding_whitespace() {
    assert_eq!(
        parse_literal_value(" again"),
        ConstValue::String(" again".to_owned()),
    );
    assert_eq!(
        parse_literal_value("trailing "),
        ConstValue::String("trailing ".to_owned()),
    );
    // An integer with whitespace around it is not the integer: rendering
    // it back as `5` would drop the space just as surely.
    assert_eq!(
        parse_literal_value(" 5"),
        ConstValue::String(" 5".to_owned()),
    );
    // The bare integer still folds.
    assert_eq!(parse_literal_value("5"), ConstValue::Int(5));
    assert_eq!(parse_literal_value("-17"), ConstValue::Int(-17));
}
use crate::cfg::{Block, BlockId, Function, Terminator};
use crate::expr_ast::ExprNode;

/// The Tcl 8.6 registry: a fixture here is release-invariant unless it
/// names another release.
fn registry() -> CommandRegistry {
    CommandRegistry::build_default().project_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    )
}

/// [`evaluate_fixture_def_with_folds`] under a module that mutates no command
/// binding — the stance the shared per-unit lattice takes for an
/// untouched file ([`FoldTrust::ObservedBindings`] over
/// [`crate::command_binding::ModuleCommandMutations`]'s `Default`).
///
/// Plain [`evaluate_fixture_def`] holds no whole-module command view at all and
/// therefore folds no builtin command substitution, so a fold-arm test
/// has to state the trust fact it folds under.
fn evaluate_pristine<S: std::hash::BuildHasher>(
    stmt: &SsaStatement,
    values: &HashMap<ValueKey, LatticeValue, S>,
    ssa: &SsaFunction,
    policy: FoldPolicy,
) -> LatticeValue {
    evaluate_fixture_def_with_folds(
        stmt,
        values,
        ssa,
        policy,
        Some(BuiltinFoldInputs {
            source_metadata_input: None,
            registry: &registry(),
            mutations: &crate::command_binding::ModuleCommandMutations::default(),
            dialect: None,
            defining_class: None,
            registry_engine: false,
            trust: FoldTrust::ObservedBindings,
            proven_pure_parameters: false,
        }),
        tcl_lexer::LexerConfig::default(),
    )
}

/// Convenience wrapper over [`sccp`] for tests with no `Module` in
/// hand — no traced variables, no dynamic variable trace.
fn sccp_no_traces(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    param_constants: Option<&HashMap<(String, crate::ssa::Version), LatticeValue>>,
    policy: FoldPolicy,
) -> SccpResult {
    sccp(
        cfg,
        ssa,
        param_constants,
        policy,
        TraceInputs {
            source_metadata_input: None,
            registry: &registry(),
            traced_variables: &BTreeSet::new(),
            has_dynamic_variable_trace: false,
            deferred_writes: &crate::ir::NO_DEFERRED_WRITES,
            analysis_context: None,
            existence: None,
        },
    )
}

/// [`sccp_no_traces`] under a module that mutates no command binding —
/// the shared per-unit lattice's stance for an untouched file, as
/// [`evaluate_pristine`] states it for one statement. Without the trust
/// fact no resolved command answers, so a test of a route states it.
fn sccp_pristine(cfg: &CfgFunction, ssa: &SsaFunction, policy: FoldPolicy) -> SccpResult {
    let registry = registry();
    sccp_with_builtin_folds(
        cfg,
        ssa,
        None,
        policy,
        &HashSet::new(),
        TraceInputs {
            source_metadata_input: None,
            registry: &registry,
            traced_variables: &BTreeSet::new(),
            has_dynamic_variable_trace: false,
            deferred_writes: &crate::ir::NO_DEFERRED_WRITES,
            analysis_context: None,
            existence: None,
        },
        Some(BuiltinFoldInputs {
            source_metadata_input: None,
            registry: &registry,
            mutations: &crate::command_binding::ModuleCommandMutations::default(),
            dialect: None,
            defining_class: None,
            registry_engine: false,
            trust: FoldTrust::ObservedBindings,
            proven_pure_parameters: false,
        }),
    )
}

/// Intern `name` and insert a fresh block; returns its [`BlockId`].
fn block(f: &mut Function, name: &str) -> BlockId {
    let id = f.intern_block(name);
    f.blocks.insert(id, Block::new(name));
    id
}

fn id_of(f: &Function, name: &str) -> BlockId {
    f.block_id(name).expect("interned")
}

fn goto(target: BlockId) -> Terminator {
    Terminator::Goto { target, span: None }
}

fn branch(cond: ExprNode, tt: BlockId, ft: BlockId) -> Terminator {
    Terminator::Branch {
        condition: cond,
        true_target: tt,
        false_target: ft,
        span: None,
        condition_base: None,
    }
}

fn literal(text: &str) -> ExprNode {
    ExprNode::Literal {
        text: text.into(),
        start: 0,
        end: u32::try_from(text.len()).unwrap_or(0),
    }
}

// Join.

#[test]
fn join_unknown_absorbs() {
    let c = LatticeValue::Const(ConstValue::Int(7));
    assert_eq!(join(&LatticeValue::Unknown, &c), c);
    assert_eq!(join(&c, &LatticeValue::Unknown), c);
}

#[test]
fn join_overdefined_is_absorbing() {
    let c = LatticeValue::Const(ConstValue::Int(7));
    assert_eq!(
        join(&LatticeValue::Overdefined, &c),
        LatticeValue::Overdefined
    );
    assert_eq!(
        join(&c, &LatticeValue::Overdefined),
        LatticeValue::Overdefined
    );
}

#[test]
fn join_identical_const_stays_const() {
    let c = LatticeValue::Const(ConstValue::Int(7));
    assert_eq!(join(&c, &c), c);
}

#[test]
fn join_distinct_consts_widens_to_set() {
    let a = LatticeValue::Const(ConstValue::Int(1));
    let b = LatticeValue::Const(ConstValue::Int(2));
    let merged = join(&a, &b);
    match merged {
        LatticeValue::ConstSet(ref vs) => {
            assert_eq!(vs.len(), 2);
        }
        other => panic!("expected ConstSet, got {other:?}"),
    }
}

#[test]
fn join_existing_set_absorbs_member() {
    let a = LatticeValue::ConstSet(vec![ConstValue::Int(1), ConstValue::Int(2)]);
    let b = LatticeValue::Const(ConstValue::Int(1));
    // Adding a member already in the set returns `old` unchanged.
    assert_eq!(join(&a, &b), a);
}

#[test]
fn join_widens_large_sets_to_overdefined() {
    let big: Vec<ConstValue> = (0..MAX_CONSTSET_SIZE)
        .map(|i| ConstValue::Int(i64::try_from(i).unwrap()))
        .collect();
    let a = LatticeValue::ConstSet(big);
    let b = LatticeValue::Const(ConstValue::Int(999));
    assert_eq!(join(&a, &b), LatticeValue::Overdefined);
}

#[test]
fn set_value_tracks_change() {
    let mut values: HashMap<ValueKey, LatticeValue> = HashMap::new();
    let key: ValueKey = (Symbol(0), 1);
    assert!(set_value(
        &mut values,
        key,
        &LatticeValue::Const(ConstValue::Int(1))
    ));
    assert!(!set_value(
        &mut values,
        key,
        &LatticeValue::Const(ConstValue::Int(1))
    ));
    assert!(set_value(
        &mut values,
        key,
        &LatticeValue::Const(ConstValue::Int(2))
    ));
}

// Predecessors + cfg_order.

#[test]
fn predecessors_simple_chain() {
    let mut f = Function::new("::top", "a");
    let a = f.entry;
    let b = block(&mut f, "b");
    f.blocks.get_mut(&a).unwrap().terminator = Some(goto(b));
    f.blocks.get_mut(&b).unwrap().terminator = Some(Terminator::Return {
        value: None,
        value_word: None,
        span: None,
        expr: None,
        expr_base: None,
        tokens: None,
        braced: false,
    });
    let p = compute_predecessors(&f);
    assert!(p.get(&b).unwrap().contains(&a));
    assert!(p.get(&a).is_none_or(HashSet::is_empty));
}

#[test]
fn cfg_order_starts_at_entry() {
    let mut f = Function::new("::top", "entry");
    let entry = f.entry;
    let t = block(&mut f, "t");
    let e = block(&mut f, "e");
    let join = block(&mut f, "join");
    f.blocks.get_mut(&entry).unwrap().terminator = Some(branch(literal("1"), t, e));
    f.blocks.get_mut(&t).unwrap().terminator = Some(goto(join));
    f.blocks.get_mut(&e).unwrap().terminator = Some(goto(join));
    f.blocks.get_mut(&join).unwrap().terminator = Some(Terminator::Return {
        value: None,
        value_word: None,
        span: None,
        expr: None,
        expr_base: None,
        tokens: None,
        braced: false,
    });
    let order = cfg_order(&f);
    assert_eq!(order[0], entry);
    // join must appear after both branches.
    let join_pos = order.iter().position(|b| *b == join).unwrap();
    let t_pos = order.iter().position(|b| *b == t).unwrap();
    let e_pos = order.iter().position(|b| *b == e).unwrap();
    assert!(join_pos > t_pos);
    assert!(join_pos > e_pos);
}

/// A static-body `uplevel 0` is an `UpFrame`, not a generic
/// barrier, and its body creates a local in the frame whose existence
/// query follows. The existence fold must never answer that the local is
/// absent. Since the binding walk follows a braced literal `uplevel`
/// script into the frame it selects, the query is decided: tclsh 8.4.20,
/// 8.6.18, 9.0.4 and 9.1.0 all print `yes` for `puts [f]`.
#[test]
fn existence_fold_abstains_for_a_live_upframe() {
    let registry = registry();
    let cu = crate::compilation_unit::CompilationUnit::build_for(
        "proc f {} { uplevel 0 { set created 1 }; if {[info exists created]} { return yes } else { return no } }",
        &registry,
        false,
    );
    let f = cu.function("::f").expect("procedure analysed");
    assert!(
        f.cfg
            .blocks
            .values()
            .flat_map(|block| block.statements.iter())
            .any(|statement| matches!(statement, Statement::UpFrame { .. })),
        "the regression requires the static uplevel lowering path"
    );
    assert!(
        f.sccp.constant_branches.iter().all(|branch| branch.value),
        "UpFrame defines `created`, so info exists must not fold to absent: {:?}",
        f.sccp.constant_branches
    );
}

/// `set x 1; unset x; info exists x` decides 0 inside the fixed point:
/// the query reads the rung, the branch is an `Applied` fact,
/// and its true arm is unreachable. tclsh 8.4 to 9.1 print `no`.
#[test]
fn set_unset_info_exists_decides_zero() {
    let registry = registry();
    let cu = crate::compilation_unit::CompilationUnit::build_for(
        "proc f {} { set x 1; unset x; if {[info exists x]} { puts yes } else { puts no } }",
        &registry,
        false,
    );
    let f = cu.function("::f").expect("procedure analysed");
    let decided = f
        .sccp
        .constant_branches
        .iter()
        .find(|branch| branch.condition == "[info exists x]")
        .unwrap_or_else(|| panic!("the query decides: {:?}", f.sccp.constant_branches));
    assert!(!decided.value);
    assert_eq!(decided.kind, BranchFactKind::Applied);
    let dead = f.cfg.block_id(&decided.not_taken_target).expect("the arm");
    assert!(!f.sccp.executable_blocks.contains(&dead));
}

/// The existence facts each `puts` reading `name` finds, in source
/// order.
fn puts_reads(f: &crate::compilation_unit::FunctionUnit, name: &str) -> Vec<Existence> {
    let symbol = f.ssa.var_symbol(name).expect("the place is an SSA symbol");
    let mut reads: Vec<(u32, Existence)> = f
        .sccp
        .existence_reads
        .iter()
        .filter(|((_, _, read), _)| *read == symbol)
        .filter_map(|(&(block, index, _), &fact)| {
            let statement = f
                .cfg
                .blocks
                .get(&block)?
                .statements
                .get(usize::try_from(index).ok()?)?;
            matches!(statement, Statement::Call { command, .. }
                    if command.trim_start_matches("::") == "puts")
            .then(|| (statement.span().start(), fact))
        })
        .collect();
    reads.sort_by_key(|&(start, _)| start);
    reads.into_iter().map(|(_, fact)| fact).collect()
}

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

#[test]
fn existence_fold_abstains_for_nested_upframe_alias_unset_mutation() {
    // `uplevel 0` evaluates in this procedure's frame. The nested body
    // aliases that frame's parameter and unsets it, so Tcl observes the else
    // branch. The no-uplevel twin proves the branch is otherwise foldable;
    // this is a mutation test of the exact fact the fold must block.
    let registry = registry();
    let stable = crate::compilation_unit::CompilationUnit::build_for(
        "proc f {local} { if {[info exists local]} { return yes } else { return no } }",
        &registry,
        false,
    );
    assert!(
        !stable
            .function("::f")
            .expect("control procedure analysed")
            .sccp
            .constant_branches
            .is_empty(),
        "the unmutated control must be foldable"
    );

    let mutated = crate::compilation_unit::CompilationUnit::build_for(
        "proc f {local} { uplevel 0 { uplevel 0 { upvar 0 local alias; unset alias } }; if {[info exists local]} { return yes } else { return no } }",
        &registry,
        false,
    );
    let f = mutated.function("::f").expect("mutated procedure analysed");
    let outer = f
        .cfg
        .blocks
        .values()
        .flat_map(|block| block.statements.iter())
        .find_map(|statement| match statement {
            Statement::UpFrame { body, .. } => Some(body),
            _ => None,
        })
        .expect("outer literal uplevel must lower to UpFrame");
    assert!(
        outer
            .statements
            .iter()
            .any(|statement| matches!(statement, Statement::UpFrame { .. })),
        "the nested literal uplevel must remain an UpFrame in its parent body"
    );
    assert!(
        f.sccp.constant_branches.is_empty(),
        "nested upframe/upvar/unset may remove local: {:?}",
        f.sccp.constant_branches
    );
}

#[test]
fn upframe_body_models_upvar_alias_unset_in_the_caller_frame() {
    // tclsh: `proc caller {} {set caller_value 1; f; info exists
    // caller_value}; proc f {} {uplevel 1 {upvar 0 caller_value alias;
    // unset alias}}; caller` returns 0. The literal UpFrame body therefore
    // carries a real caller-frame mutation, not merely a control-flow
    // wrapper, and is a separate vector from the nested local-frame test.
    let registry =
        registry().project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let cu = crate::compilation_unit::CompilationUnit::build_for(
        "proc f {} { uplevel 1 { upvar 0 caller_value alias; unset alias } }",
        &registry,
        false,
    );
    let f = cu.function("::f").expect("procedure analysed");
    let body = f
        .cfg
        .blocks
        .values()
        .flat_map(|block| block.statements.iter())
        .find_map(|statement| match statement {
            Statement::UpFrame { body, .. } => Some(body),
            _ => None,
        })
        .expect("literal caller-frame body must lower to UpFrame");
    assert!(body.statements.iter().any(|statement| {
        matches!(statement, Statement::Call { command, .. } if command == "upvar")
    }));
    assert!(body.statements.iter().any(|statement| {
        matches!(statement, Statement::Call { command, .. } if command == "unset")
    }));
}

// Driver.

use crate::ir::Statement;
use crate::ssa::{SsaBlock, SsaStatement};
use tcl_lexer::Span;

/// A block-less SSA function used purely as a variable-name interner for
/// the hand-built statement / lattice tests.
fn bare_ssa() -> SsaFunction {
    SsaFunction::trivial("::top", BlockId(0), vec!["entry".into()])
}

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

fn assign_const_stmt(ssa: &mut SsaFunction, name: &str, value: &str, ver: u32) -> SsaStatement {
    let mut defs = HashMap::new();
    defs.insert(ssa.intern_var(name), ver);
    SsaStatement {
        statement: Statement::AssignConst {
            span: Span::new(0, 0),
            name: name.into(),
            name_braced: false,
            value: value.into(),
            value_span: None,
        },
        uses: HashMap::new(),
        defs,
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    }
}

fn empty_ssa_block(name: &str) -> SsaBlock {
    SsaBlock {
        name: name.into(),
        phis: Vec::new(),
        statements: Vec::new(),
        entry_versions: HashMap::new(),
        exit_versions: HashMap::new(),
    }
}

/// Build an [`SsaFunction`] over `f`'s interner with the given per-name SSA
/// blocks; any CFG block not listed gets an empty SSA block.
fn make_ssa(f: &Function, named: Vec<(&str, SsaBlock)>) -> SsaFunction {
    let mut ssa = SsaFunction::trivial("::top", f.entry, f.block_names().to_vec());
    let mut provided: std::collections::HashSet<BlockId> = std::collections::HashSet::new();
    for (name, blk) in named {
        let id = id_of(f, name);
        provided.insert(id);
        ssa.blocks.insert(id, blk);
    }
    for id in f.blocks.keys() {
        if !provided.contains(id) {
            ssa.blocks.insert(*id, empty_ssa_block(f.block_name(*id)));
        }
    }
    ssa
}

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
    assert!(
        sccp_process_statements(
            &mut values,
            &block,
            &ssa,
            StatementInputs {
                block: BlockId(0),
                escaping: &escaping,
                has_dynamic_variable_trace: false,
                clobbers: None,
                catches: None,
            },
            &LatticeDriver::detached(None, FoldPolicy::default()),
            None,
        )
        .changed
    );
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

/// A call to a command the module cannot see may create or destroy a plain
/// top-level name as it may a `::` one, and a procedure's local through
/// `upvar 1`, so `[info exists]` after it decides nothing; a call the
/// module resolves leaves the local's existence decided.
#[test]
fn an_unseen_call_leaves_no_name_with_a_settled_existence() {
    let registry =
        registry().project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    // #2427: a bare undefined command no longer widens the command table; an autoload index entry in view keeps this device unbounded.
    for (source, function) in [
        (
            "set ::auto_index(foo) {source lib.tcl}\nset x 1\nfoo\nif {[info exists x]} { puts a } else { puts b }\n",
            "::top",
        ),
        (
            "set ::auto_index(foo) {source lib.tcl}\nfoo\nif {[info exists never_set]} { puts a } else { puts b }\n",
            "::top",
        ),
        (
            "set ::auto_index(foo) {source lib.tcl}\nproc p {} { set x 1; foo; if {[info exists x]} { puts a } else { puts b } }\n",
            "::p",
        ),
    ] {
        let cu = crate::compilation_unit::CompilationUnit::build_for(source, &registry, false);
        let f = cu.function(function).expect("function analysed");
        assert!(f.sccp.constant_branches.is_empty(), "{source}");
    }
    let cu = crate::compilation_unit::CompilationUnit::build_for(
        "proc foo {} { puts hi }\nproc p {} { set x 1; foo; if {[info exists x]} { puts a } else { puts b } }\n",
        &registry,
        false,
    );
    let f = cu.function("::p").expect("procedure analysed");
    let decided = f
        .sccp
        .constant_branches
        .iter()
        .find(|branch| branch.condition == "[info exists x]")
        .expect("the local's existence is decided");
    assert!(decided.value);
}

#[test]
fn sccp_marks_entry_executable_and_propagates_const() {
    // entry: set x 42
    let mut f = Function::new("::top", "entry");
    let entry = f.entry;
    f.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Return {
        value: None,
        value_word: None,
        span: None,
        expr: None,
        expr_base: None,
        tokens: None,
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
        value: None,
        value_word: None,
        span: None,
        expr: None,
        expr_base: None,
        tokens: None,
        braced: false,
    });
    f.blocks.get_mut(&e).unwrap().terminator = Some(Terminator::Return {
        value: None,
        value_word: None,
        span: None,
        expr: None,
        expr_base: None,
        tokens: None,
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
        value: None,
        value_word: None,
        span: None,
        expr: None,
        expr_base: None,
        tokens: None,
        braced: false,
    });
    f.blocks.get_mut(&e).unwrap().terminator = Some(Terminator::Return {
        value: None,
        value_word: None,
        span: None,
        expr: None,
        expr_base: None,
        tokens: None,
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
        value: None,
        value_word: None,
        span: None,
        expr: None,
        expr_base: None,
        tokens: None,
        braced: false,
    });
    f.blocks.get_mut(&e).unwrap().terminator = Some(Terminator::Return {
        value: None,
        value_word: None,
        span: None,
        expr: None,
        expr_base: None,
        tokens: None,
        braced: false,
    });
    let ssa = make_ssa(&f, vec![]);

    let r = sccp_no_traces(&f, &ssa, None, FoldPolicy::default());
    assert!(r.executable_blocks.contains(&t));
    assert!(r.executable_blocks.contains(&e));
    assert_eq!(r.constant_branches, [] as [crate::sccp::ConstantBranch; 0]);
}

#[test]
fn evaluate_def_assign_const_produces_int_or_string() {
    let mut ssa = bare_ssa();
    let s_int = assign_const_stmt(&mut ssa, "x", "42", 1);
    assert_eq!(
        evaluate_fixture_def(
            &s_int,
            &HashMap::new(),
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Const(ConstValue::Int(42))
    );
    let s_str = assign_const_stmt(&mut ssa, "x", "hello", 1);
    assert_eq!(
        evaluate_fixture_def(
            &s_str,
            &HashMap::new(),
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Const(ConstValue::String("hello".into()))
    );
}

#[test]
fn evaluate_def_assign_expr_folds_with_lattice() {
    // `set x [expr {$a + 3}]` with `a` the constant 2 settles 5.
    assert_eq!(
        settled(
            "tcl8.6",
            "proc p {} {set a 2; set x [expr {$a + 3}]; return $x}",
            "x"
        ),
        LatticeValue::Const(ConstValue::Int(5))
    );
}

/// SCCP folds the iRules word operators when — and only when — the
/// policy says the dialect has them.
///
/// Evaluating through a dialect-blind entry point (a bare
/// `octal: Option<bool>`) leaves `FoldOps::is_irules` `false`, so every
/// word operator silently declines while the `eq` control on the same
/// shape folds, because plain Tcl shares it.
#[test]
fn evaluate_def_folds_irules_word_operator_only_under_an_irules_policy() {
    let mut ssa = bare_ssa();
    let subject = ssa.intern_var("s");
    let out = ssa.intern_var("hit");
    let mut uses = HashMap::new();
    uses.insert(subject, 1);
    let mut defs = HashMap::new();
    defs.insert(out, 1);

    let expr = ExprNode::Binary {
        op: crate::expr_ast::BinOp::Contains,
        left: Box::new(ExprNode::Var {
            text: "$s".into(),
            name: "s".into(),
            start: 0,
            end: 2,
        }),
        right: Box::new(ExprNode::String {
            text: "cd".into(),
            start: 3,
            end: 7,
        }),
    };
    let stmt_ssa = SsaStatement {
        statement: Statement::AssignExpr {
            span: Span::new(0, 0),
            name: "hit".into(),
            name_braced: false,
            expr,
            command_binding: Some(tcl_runtime_api::CommandBindingIdentity::new("expr", "expr")),
            expr_base: None,
            fallback_value: "[expr {$s contains \"cd\"}]".into(),
        },
        uses,
        defs,
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    };
    let mut values = HashMap::new();
    values.insert(
        (subject, 1),
        LatticeValue::Const(ConstValue::String("abcde".into())),
    );

    let irules = FoldPolicy::for_profile(Some(true), Some(tcl_dialect::DialectProfile::irules()));
    assert_eq!(
        evaluate_fixture_def(
            &stmt_ssa,
            &values,
            &ssa,
            irules,
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Const(ConstValue::Int(1)),
        "`$s contains \"cd\"` with $s = abcde must fold to 1 under an iRules policy"
    );
    // TN: without the dialect fact the fold is declined, not guessed.
    assert_eq!(
        evaluate_fixture_def(
            &stmt_ssa,
            &values,
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Overdefined,
        "a dialect-blind policy must decline the word-operator fold"
    );
}

/// A fused `set r [expr {TEXT}]` under `profile`'s expression grammar.
fn fused_expr_stmt(
    ssa: &mut SsaFunction,
    text: &str,
    profile: Option<&'static tcl_dialect::DialectProfile>,
) -> SsaStatement {
    let mut defs = HashMap::new();
    defs.insert(ssa.intern_var("r"), 1);
    SsaStatement {
        statement: Statement::AssignExpr {
            span: Span::new(0, 0),
            name: "r".into(),
            name_braced: false,
            expr: crate::expr_parser::parse_expr_for_profile(text, profile),
            command_binding: Some(tcl_runtime_api::CommandBindingIdentity::new("expr", "expr")),
            expr_base: None,
            fallback_value: format!("[expr {{{text}}}]"),
        },
        uses: HashMap::new(),
        defs,
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    }
}

/// `expr` answers its full value, not only a number: a string result is
/// the result, and a string that reads as a number under the target's
/// grammar is that number's canonical form. Oracle, tclsh 8.4 to 9.1:
/// `expr {"x"}` is `x`, `expr {1 ? "yes" : "no"}` is `yes`, `expr
/// {"true"}` is `true`, `expr {" 5 "}` is 5, `expr {"0x10"}` is 16,
/// `expr {"1.50"}` is 1.5 and `expr {1/0}` raises; `expr {"010"}` is 8 up
/// to 8.6 and 10 from 9.0; `expr {1 << 70}` is
/// `1180591620717411303424` from 8.5 and 0 under 8.4, so a target that
/// does not widen declines it.
#[test]
fn a_fused_expression_answers_its_full_value() {
    let text = |s: &str| LatticeValue::Const(ConstValue::String(s.to_owned()));
    let int = |i: i64| LatticeValue::Const(ConstValue::Int(i));
    let under = |name: &str| {
        let profile = tcl_dialect::DialectProfile::find(name).expect(name);
        (
            Some(profile),
            FoldPolicy::for_profile(
                crate::tcl_expr_eval::leading_zero_is_octal(profile),
                Some(profile),
            ),
        )
    };
    let cases: [(&str, &str, LatticeValue); 14] = [
        ("tcl8.4", r#""x""#, text("x")),
        ("f5-irules", r#""x""#, text("x")),
        ("tcl9.0", r#"1 ? "yes" : "no""#, text("yes")),
        ("tcl8.6", r#""true""#, text("true")),
        ("tcl8.6", r#"" 5 ""#, int(5)),
        ("tcl9.0", r#""0x10""#, int(16)),
        (
            "tcl8.6",
            r#""1.50""#,
            LatticeValue::Const(ConstValue::Float(1.5)),
        ),
        ("tcl8.6", r#""010""#, int(8)),
        ("tcl9.0", r#""010""#, int(10)),
        ("tcl8.6", r#""08""#, text("08")),
        ("tcl9.0", "1/0", LatticeValue::Overdefined),
        ("tcl8.6", "2 ** 64", text("18446744073709551616")),
        ("tcl9.0", "[string length abc] * 2", int(6)),
        ("tcl9.0", "0 && [error never]", int(0)),
    ];
    for (dialect, expression, want) in cases {
        let (profile, policy) = under(dialect);
        let mut ssa = bare_ssa();
        let stmt = fused_expr_stmt(&mut ssa, expression, profile);
        assert_eq!(
            evaluate_pristine(&stmt, &HashMap::new(), &ssa, policy),
            want,
            "{dialect}: expr {{{expression}}}"
        );
    }
    // No leading-zero rule at all: the two readings of `010` disagree.
    let mut ssa = bare_ssa();
    let stmt = fused_expr_stmt(&mut ssa, r#""010""#, None);
    assert_eq!(
        evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::from_octal(None)),
        LatticeValue::Overdefined
    );
}

/// A shift past the wide boundary is the release's own answer. tclsh
/// 8.4.20 prints 0 for `puts [expr {1 << 70}]`, where the count runs off
/// its fixed-width integer; tclsh 8.5 to 9.1 print
/// 1180591620717411303424. The `f5-irules` row waits on BIG-IP check 10
/// and on handler bodies lowering with plain names.
#[test]
fn a_wide_shift_settles_what_each_release_prints() {
    let big = LatticeValue::Const(ConstValue::String("1180591620717411303424".into()));
    for (dialect, want) in [
        ("tcl8.4", LatticeValue::Const(ConstValue::Int(0))),
        ("tcl8.5", big.clone()),
        ("tcl8.6", big.clone()),
        ("tcl9.0", big.clone()),
        ("tcl9.1", big),
    ] {
        assert_eq!(
            settled(
                dialect,
                "proc p {} {set r [expr {1 << 70}]; return $r}",
                "r"
            ),
            want,
            "{dialect}"
        );
    }
}

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
    values.insert((x, 1), LatticeValue::Const(ConstValue::Int(5)));
    assert_eq!(
        evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
        LatticeValue::Const(ConstValue::Int(6))
    );
}

#[test]
fn evaluate_def_incr_integer_literal_amount() {
    let mut ssa = bare_ssa();
    let stmt = incr_stmt(&mut ssa, "x", Some("10"), 1, 2);
    let x = ssa.var_symbol("x").unwrap();
    let mut values = HashMap::new();
    values.insert((x, 1), LatticeValue::Const(ConstValue::Int(3)));
    assert_eq!(
        evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
        LatticeValue::Const(ConstValue::Int(13))
    );
}

#[test]
fn evaluate_def_incr_negative_literal_amount() {
    let mut ssa = bare_ssa();
    let stmt = incr_stmt(&mut ssa, "x", Some("-2"), 1, 2);
    let x = ssa.var_symbol("x").unwrap();
    let mut values = HashMap::new();
    values.insert((x, 1), LatticeValue::Const(ConstValue::Int(10)));
    assert_eq!(
        evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
        LatticeValue::Const(ConstValue::Int(8))
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
    values.insert((x, 1), LatticeValue::Const(ConstValue::Int(6)));
    values.insert((y, 1), LatticeValue::Const(ConstValue::Int(4)));
    assert_eq!(
        evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
        LatticeValue::Const(ConstValue::Int(10))
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
    values.insert((x, 1), LatticeValue::Const(ConstValue::Int(5)));
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
        LatticeValue::Const(ConstValue::Int(6))
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
    values.insert((x, 1), LatticeValue::Const(ConstValue::Int(1)));
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

fn folds_for(dialect: &str) -> BuiltinFoldInputs<'static> {
    use std::sync::OnceLock;
    static MUTATIONS: OnceLock<crate::command_binding::ModuleCommandMutations> = OnceLock::new();
    let environment = tcl_registry::model::ingress::resolve_environment(dialect);
    BuiltinFoldInputs {
        source_metadata_input: None,
        registry: tcl_registry::model::ingress::static_context_for(dialect).commands(),
        mutations: MUTATIONS.get_or_init(Default::default),
        dialect: Some(environment.analyser_profile()),
        defining_class: None,
        registry_engine: false,
        trust: FoldTrust::ObservedBindings,
        proven_pure_parameters: false,
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
    let text = |value: &str| LatticeValue::Const(ConstValue::String(value.to_owned()));
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
        LatticeValue::ConstSet(members.into_iter().map(ConstValue::Int).collect())
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
    let int = |value: i64| LatticeValue::Const(ConstValue::Int(value));
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
    values.insert((n, 1), LatticeValue::Const(ConstValue::Int(1)));
    assert_eq!(
        evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
        LatticeValue::Overdefined,
        "never the pending bottom a join would launder"
    );
    // The statement-position update holds its target's use and folds.
    let incr = incr_stmt(&mut ssa, "n", None, 1, 2);
    assert_eq!(
        evaluate_pristine(&incr, &values, &ssa, FoldPolicy::default()),
        LatticeValue::Const(ConstValue::Int(2))
    );
}

/// `[set x]` in value position reads `x` through the cell-write route;
/// `[set x 10]` writes storage a value position cannot land
/// (`EffectFreeOnly`), so its host widens.
#[test]
fn evaluate_def_set_read_in_value_position_folds() {
    // tclsh 8.6.18 and 9.0.4 print `<hello> <10 10> <CHANGED>` for a
    // nested read, a nested write, and a nested read of a place whose
    // read trace replaces it.
    let text = |value: &str| LatticeValue::Const(ConstValue::String(value.to_owned()));
    assert_eq!(
        settled(
            "tcl8.6",
            "proc p {} {set x hello; set r [set x]; return $r}",
            "r"
        ),
        text("hello")
    );
    assert_eq!(
        settled(
            "tcl8.6",
            "proc p {} {set x hello; set r [set x 10]; return $r}",
            "r"
        ),
        LatticeValue::Overdefined
    );
    assert_eq!(
        settled(
            "tcl8.6",
            "proc t args {}\nproc p {} {set x hello; trace add variable x read t; set r [set x]; return $r}",
            "r"
        ),
        LatticeValue::Overdefined
    );
    assert_eq!(
        settled("tcl8.6", "proc p {x} {set r [set x]; return $r}", "r"),
        LatticeValue::Overdefined
    );
}

/// `[string range "a\tb" 0 1]` in value position reads its words as Tcl
/// substitutes them: `a` and a tab, where the raw spelling gave `a\`. A
/// braced word keeps its backslash, and a braced `$x` is text, not a
/// read (tclsh 8.4 to 9.1 print `a` and a tab, `a\`, and `$x`).
#[test]
fn string_range_in_value_position_reads_cooked_escapes() {
    let text = |value: &str| LatticeValue::Const(ConstValue::String(value.to_owned()));
    let fold = |word: &str| {
        settled(
            "tcl8.6",
            &format!("proc p {{}} {{set xy abc; set r [string range {word} 0 1]; return $r}}"),
            "r",
        )
    };
    assert_eq!(fold("\"a\\tb\""), text("a\t"));
    assert_eq!(fold("{a\\tb}"), text("a\\"));
    assert_eq!(fold("{$xy}"), text("$x"), "the braced word is text");
}

/// `[string range …]` in value position runs the registry's route: the
/// index numerals read under the target's grammar. `f5-irules` reads
/// them as its declared 8.4 base does (ruling 8; tclsh 8.4 prints
/// `ijkl`), and the lenient `tcl` profile, naming no release, declines.
#[test]
fn evaluate_def_assign_value_folds_string_range_under_the_release() {
    let under =
        |dialect: &str| settled_body(dialect, "set s [string range abcdefghijkl 010 end]", "s");
    assert_eq!(
        under("tcl8.6"),
        LatticeValue::Const(ConstValue::String("ijkl".into()))
    );
    assert_eq!(
        under("tcl9.0"),
        LatticeValue::Const(ConstValue::String("kl".into()))
    );
    assert_eq!(
        under("f5-irules"),
        LatticeValue::Const(ConstValue::String("ijkl".into()))
    );
    assert_eq!(under("tcl"), LatticeValue::Overdefined);
    assert_eq!(
        settled_body("tcl8.6", "set t [string range { a } 0 end]", "t"),
        LatticeValue::Const(ConstValue::String(" a ".into()))
    );
}

/// The run records each statement's route and answer for the
/// Explorer, at the fixed point.
#[test]
fn sccp_records_a_route_explanation_per_statement() {
    let registry = registry();
    let cu = crate::compilation_unit::CompilationUnit::build_for(
        "proc p {} {\n    set n 1\n    incr n\n    append s x\n    set r [string range abc 0 1]\n    return $n\n}\n",
        &registry,
        false,
    );
    let fu = fixture_id_if_present(&cu.procedures, "::p")
        .and_then(|id| cu.procedures.get(&id))
        .expect("proc");
    let explained: Vec<(String, String)> = fu
        .sccp
        .explanations
        .iter()
        .map(|e| (e.command.clone(), e.answer.clone()))
        .collect();
    assert!(
        explained.contains(&("incr".to_owned(), "evaluated".to_owned())),
        "{explained:?}"
    );
    // The existence rung proves `s` unbound, and `append` creates its
    // cell in every release, so the absent cell evaluates
    // (`the_absent_cell_release_table`: `append s foo` is `foo` under
    // tclsh 8.4 to 9.1).
    assert!(
        explained.contains(&("append".to_owned(), "evaluated".to_owned())),
        "an append over an unbound cell creates it: {explained:?}"
    );
    assert!(
        fu.sccp
            .explanations
            .iter()
            .any(|e| e.command == "string" && e.route.starts_with("direct string-range")),
        "{:?}",
        fu.sccp.explanations
    );
}

#[test]
fn lattice_reference_shapes_keep_actual_grammar_and_refuse_compound_or_array_names() {
    let mut ssa = bare_ssa();
    let mut uses = HashMap::new();
    let mut values = HashMap::new();
    for name in ["a", "a:b", "café", "", "scalar(open", "arr(key)", "a{b}c"] {
        let symbol = ssa.intern_var(name);
        uses.insert(symbol, 1);
        values.insert(
            (symbol, 1),
            LatticeValue::Const(ConstValue::String("one two".into())),
        );
    }
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let config = tcl_lexer::LexerConfig::for_dialect(profile);
        let rules = tcl_syntax::word_rules::WordValueRules::from_config(&config);
        for source in ["$a", "${}", "${scalar(open}"] {
            assert!(
                resolve_simple_var_ref(source, &uses, &values, &ssa, config).is_some(),
                "{profile}: {source}"
            );
            assert_eq!(
                resolve_foreach_list_via_lattice(source, &uses, &values, &ssa, rules, config),
                Some(vec!["one".into(), "two".into()]),
                "{profile}: {source}"
            );
        }
        for source in ["$a:b", "${arr(key)}", "$arr(key)", "$a$b"] {
            assert_eq!(
                resolve_simple_var_ref(source, &uses, &values, &ssa, config),
                None,
                "{profile}: {source}"
            );
            assert_eq!(
                resolve_foreach_list_via_lattice(source, &uses, &values, &ssa, rules, config),
                None,
                "{profile}: {source}"
            );
        }
        assert_eq!(
            resolve_simple_var_ref("$café", &uses, &values, &ssa, config).is_some(),
            profile == "jim"
        );
        assert_eq!(
            resolve_simple_var_ref("${a{b}c}", &uses, &values, &ssa, config).is_some(),
            matches!(profile, "tcl9.0" | "tcl9.1")
        );
    }
}

#[test]
fn resolve_simple_var_ref_accepts_bare_and_braced() {
    let mut ssa = bare_ssa();
    let x = ssa.intern_var("x");
    let mut uses = HashMap::new();
    uses.insert(x, 1);
    let mut values = HashMap::new();
    values.insert((x, 1), LatticeValue::Const(ConstValue::Int(7)));
    assert_eq!(
        resolve_simple_var_ref(
            "$x",
            &uses,
            &values,
            &ssa,
            tcl_lexer::LexerConfig::default()
        ),
        Some(LatticeValue::Const(ConstValue::Int(7)))
    );
    assert_eq!(
        resolve_simple_var_ref(
            "${x}",
            &uses,
            &values,
            &ssa,
            tcl_lexer::LexerConfig::default()
        ),
        Some(LatticeValue::Const(ConstValue::Int(7)))
    );
    assert_eq!(
        resolve_simple_var_ref(
            "$y",
            &uses,
            &values,
            &ssa,
            tcl_lexer::LexerConfig::default()
        ),
        None
    );
    assert_eq!(
        resolve_simple_var_ref(
            "plain",
            &uses,
            &values,
            &ssa,
            tcl_lexer::LexerConfig::default()
        ),
        None
    );
}

// Foreach constset extraction.

fn foreach_stmt(ssa: &mut SsaFunction, var: &str, list: &str, new_ver: u32) -> SsaStatement {
    let mut defs = HashMap::new();
    defs.insert(ssa.intern_var(var), new_ver);
    SsaStatement {
        statement: Statement::Call {
            span: Span::new(0, 0),
            command: "foreach".into(),
            canonical_command: None,
            args: vec![list.into()],
            defs: vec![var.into()],
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            // The CFG builder marks the synthetic loop header it emits
            // with its iterator-group sizes; the driver keys the header
            // layout on that typed fact, never on the command's name.
            foreach_groups: Some(vec![1]),
        },
        uses: HashMap::new(),
        defs,
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    }
}

#[test]
fn extract_foreach_elements_literal_list() {
    // `list_text` is already delimiter-stripped by the segmenter, matching
    // the shape `foreach v {a b c}`'s `list_arg` is built in: the word's
    // own `{…}` is gone, leaving plain whitespace-separated text.
    assert_eq!(
        extract_foreach_elements("a b c", tcl_syntax::word_rules::WordValueRules::TCL),
        Some(vec!["a".into(), "b".into(), "c".into()])
    );
}

#[test]
fn extract_foreach_elements_rejects_substitutions() {
    assert_eq!(
        extract_foreach_elements("$lst", tcl_syntax::word_rules::WordValueRules::TCL),
        None
    );
    assert_eq!(
        extract_foreach_elements("[list a b c]", tcl_syntax::word_rules::WordValueRules::TCL),
        None
    );
}

#[test]
fn extract_foreach_elements_splits_nested_braces_as_tcl_list() {
    // `a {b c} d` is a three-element Tcl list — `a`, `b c`, `d` — not the
    // four whitespace runs `a`, `{b`, `c}`, `d`. A naive `split_ascii_whitespace`
    // corrupted the CONSTSET; the list-aware split fixes it.
    assert_eq!(
        extract_foreach_elements("a {b c} d", tcl_syntax::word_rules::WordValueRules::TCL),
        Some(vec!["a".into(), "b c".into(), "d".into()])
    );
    // Backslash-escaped whitespace groups an element too: `a\ b c` is two
    // elements `a b` and `c`.
    assert_eq!(
        extract_foreach_elements("a\\ b c", tcl_syntax::word_rules::WordValueRules::TCL),
        Some(vec!["a b".into(), "c".into()])
    );
}

#[test]
fn extract_foreach_elements_empty_list_returns_empty() {
    assert_eq!(
        extract_foreach_elements("", tcl_syntax::word_rules::WordValueRules::TCL),
        Some(Vec::new())
    );
}

// `list_text` already has its outer
// `foreach`-word delimiter removed by the segmenter (see
// `Statement::Foreach::list_arg`'s construction in
// `lowering::structured`), so a value that itself starts with `{` / `"`
// is a *nested* list element, not a delimiter to peel a second time.
#[test]
fn extract_foreach_elements_single_element_braced_list_not_double_stripped() {
    // Source `foreach v {{a b c}}`: the segmenter strips the word's own
    // outer `{…}`, leaving the single-element list `{a b c}` as
    // `list_text`. Splitting it as a Tcl list yields the one element
    // `a b c`, not the three whitespace-separated words `a`, `b`, `c`
    // a second brace-strip-then-split would wrongly produce.
    assert_eq!(
        extract_foreach_elements("{a b c}", tcl_syntax::word_rules::WordValueRules::TCL),
        Some(vec!["a b c".into()])
    );
}

#[test]
fn extract_foreach_elements_two_braced_elements_not_double_stripped() {
    // Source `foreach v {{a b} {c d}}`: `list_text` is `{a b} {c d}`.
    // The pre-fix double-strip peeled the text's own outer `{`/`}` too,
    // yielding the corrupted split `a`, `b}`, `{c`, `d` (lenient split of
    // `a b} {c d`). The correct split is the two nested-list elements.
    assert_eq!(
        extract_foreach_elements("{a b} {c d}", tcl_syntax::word_rules::WordValueRules::TCL),
        Some(vec!["a b".into(), "c d".into()])
    );
}

#[test]
fn extract_foreach_elements_quoted_word_already_delimiter_stripped() {
    // Source `foreach v "a b c"`: the segmenter strips the word's own
    // `"…"` delimiters the same way it strips `{…}`, so `list_text` is
    // plain `a b c` — no quote handling is needed (or wanted) here.
    assert_eq!(
        extract_foreach_elements("a b c", tcl_syntax::word_rules::WordValueRules::TCL),
        Some(vec!["a".into(), "b".into(), "c".into()])
    );
}

#[test]
fn evaluate_def_foreach_literal_list_folds_constset() {
    // `list` mirrors `list_arg` as the segmenter hands it: source
    // `foreach v {1 2 3}`'s outer `{…}` is already stripped.
    let mut ssa = bare_ssa();
    let stmt = foreach_stmt(&mut ssa, "v", "1 2 3", 1);
    let result = evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default());
    match result {
        LatticeValue::ConstSet(ref vs) => {
            assert_eq!(vs.len(), 3);
            assert!(vs.contains(&ConstValue::Int(1)));
            assert!(vs.contains(&ConstValue::Int(3)));
        }
        other => panic!("expected ConstSet, got {other:?}"),
    }
}

#[test]
fn evaluate_def_foreach_list_cmd_subst_folds_constset() {
    // `foreach v [list a b c]` folds through the declared `list` route to the
    // same element CONSTSET as the braced-literal form.
    let mut ssa = bare_ssa();
    let stmt = foreach_stmt(&mut ssa, "v", "[list a b c]", 1);
    let result = evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default());
    match result {
        LatticeValue::ConstSet(ref vs) => {
            assert_eq!(vs.len(), 3);
            assert!(vs.contains(&ConstValue::String("a".into())));
            assert!(vs.contains(&ConstValue::String("c".into())));
        }
        other => panic!("expected ConstSet, got {other:?}"),
    }
}

#[test]
fn evaluate_def_foreach_single_element_folds_const() {
    // `list` mirrors `list_arg` for source `foreach v {only}`: the
    // segmenter's delimiter strip leaves the bare word `only`.
    let mut ssa = bare_ssa();
    let stmt = foreach_stmt(&mut ssa, "v", "only", 1);
    assert_eq!(
        evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()),
        LatticeValue::Const(ConstValue::String("only".into()))
    );
}

#[test]
fn evaluate_def_foreach_nested_braced_elements_folds_constset() {
    // Source `foreach v {{a b} {c d}}` hands `list_arg` = `{a b} {c d}`
    // (only the word's own outer `{…}` is stripped by the segmenter).
    // Peeling a *second* level of bracing off this already
    // delimiter-stripped text would fold `v` to the corrupted CONSTSET
    // {"a", "b}"} instead of the two list elements.
    let mut ssa = bare_ssa();
    let stmt = foreach_stmt(&mut ssa, "v", "{a b} {c d}", 1);
    let result = evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default());
    match result {
        LatticeValue::ConstSet(ref vs) => {
            assert_eq!(vs.len(), 2);
            assert!(vs.contains(&ConstValue::String("a b".into())));
            assert!(vs.contains(&ConstValue::String("c d".into())));
            assert!(!vs.contains(&ConstValue::String("b}".into())));
        }
        other => panic!("expected ConstSet, got {other:?}"),
    }
}

#[test]
fn evaluate_def_foreach_via_lattice_var() {
    let mut ssa = bare_ssa();
    let mut stmt = foreach_stmt(&mut ssa, "v", "$lst", 1);
    let lst = ssa.intern_var("lst");
    stmt.uses.insert(lst, 1);
    let mut values = HashMap::new();
    values.insert(
        (lst, 1),
        LatticeValue::Const(ConstValue::String("a b c".into())),
    );
    let result = evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default());
    match result {
        LatticeValue::ConstSet(ref vs) => assert_eq!(vs.len(), 3),
        other => panic!("expected ConstSet, got {other:?}"),
    }
}

#[test]
fn evaluate_def_foreach_unbound_var_widens() {
    let mut ssa = bare_ssa();
    let mut stmt = foreach_stmt(&mut ssa, "v", "$lst", 1);
    let lst = ssa.intern_var("lst");
    stmt.uses.insert(lst, 1);
    // Empty lattice — var not bound.
    let result = evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default());
    assert_eq!(result, LatticeValue::Overdefined);
}

#[test]
fn evaluate_def_foreach_multi_var_binds_each_its_elements() {
    // Two binders over one list: each takes the elements it is
    // assigned, so over `a b` the first binder, `v`, holds `a`.
    assert_eq!(
        settled("tcl8.6", "proc p {} {foreach {v w} {a b} {puts $v$w}}", "v"),
        LatticeValue::Const(ConstValue::String("a".into()))
    );
}

// AssignValue + command-substitution folding.

fn assign_value_stmt(ssa: &mut SsaFunction, name: &str, value: &str, ver: u32) -> SsaStatement {
    let mut defs = HashMap::new();
    defs.insert(ssa.intern_var(name), ver);
    SsaStatement {
        statement: Statement::AssignValue {
            span: Span::new(0, 0),
            name: name.into(),
            name_braced: false,
            value: value.into(),
            value_needs_backsubst: false,
            tokens: None,
        },
        uses: HashMap::new(),
        defs,
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    }
}

#[test]
fn evaluate_def_assign_value_plain_literal() {
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "x", "hello", 1);
    assert_eq!(
        evaluate_fixture_def(
            &stmt,
            &HashMap::new(),
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Const(ConstValue::String("hello".into()))
    );
}

#[test]
fn evaluate_def_assign_value_integer_literal() {
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "x", "42", 1);
    assert_eq!(
        evaluate_fixture_def(
            &stmt,
            &HashMap::new(),
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Const(ConstValue::Int(42))
    );
}

#[test]
fn evaluate_def_assign_value_resolves_var_ref() {
    let mut ssa = bare_ssa();
    let mut stmt = assign_value_stmt(&mut ssa, "y", "$x", 1);
    let x = ssa.intern_var("x");
    stmt.uses.insert(x, 1);
    let mut values = HashMap::new();
    values.insert((x, 1), LatticeValue::Const(ConstValue::Int(7)));
    assert_eq!(
        evaluate_fixture_def(
            &stmt,
            &values,
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Const(ConstValue::Int(7))
    );
}

#[test]
fn evaluate_def_assign_value_folds_list_cmd() {
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "x", "[list a b c]", 1);
    let result = evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default());
    match result {
        LatticeValue::Const(ConstValue::String(s)) => assert_eq!(s, "a b c"),
        other => panic!("expected Const(String), got {other:?}"),
    }
}

#[test]
fn evaluate_def_assign_value_folds_llength_literal() {
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "n", "[llength {a b c d}]", 1);
    assert_eq!(
        evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()),
        LatticeValue::Const(ConstValue::Int(4))
    );
}

#[test]
fn evaluate_def_assign_value_folds_string_length() {
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "n", "[string length \"hello\"]", 1);
    assert_eq!(
        evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()),
        LatticeValue::Const(ConstValue::Int(5))
    );
}

// The fold arms answer to the command-binding trust fact.

/// The whole-module mutation summary for `src`.
fn mutations_for(src: &str) -> crate::command_binding::ModuleCommandMutations {
    let reg = registry();
    let ir = crate::lowering::lower_to_ir(src, &reg);
    crate::command_binding::scan_module_command_mutations(&ir, &reg)
}

/// Evaluate `stmt` under the whole-module trust fact `mutations`.
fn evaluate_under_unit(
    stmt: &SsaStatement,
    ssa: &SsaFunction,
    registry: &CommandRegistry,
    mutations: &crate::command_binding::ModuleCommandMutations,
) -> LatticeValue {
    evaluate_fixture_def_with_folds(
        stmt,
        &HashMap::new(),
        ssa,
        FoldPolicy::default(),
        Some(BuiltinFoldInputs {
            source_metadata_input: None,
            registry,
            mutations,
            dialect: None,
            defining_class: None,
            registry_engine: false,
            trust: FoldTrust::WholeModule,
            proven_pure_parameters: false,
        }),
        tcl_lexer::LexerConfig::default(),
    )
}

/// The `[list …]` / `[format …]` arms run ahead of the registry engine, so
/// they must apply its trust gate themselves — otherwise a unit that
/// renamed `list` still gets the builtin's answer.
///
/// tclsh 8.6.16 / 9.0.4 (identical): after `rename list mylist`, evaluating
/// `list a b c` raises rather than returning `a b c`.
#[test]
fn list_arm_declines_once_the_unit_renames_list() {
    let reg = registry();
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "x", "[list a b c]", 1);

    let untouched = mutations_for("set y 1\n");
    assert_eq!(
        evaluate_under_unit(&stmt, &ssa, &reg, &untouched),
        LatticeValue::Const(ConstValue::String("a b c".into())),
        "an untouched `list` still folds"
    );

    let renamed = mutations_for("rename list mylist\n");
    assert_eq!(
        evaluate_under_unit(&stmt, &ssa, &reg, &renamed),
        LatticeValue::Overdefined,
        "a renamed `list` must not fold to the builtin's answer"
    );
}

/// The same gate, on an arm reached through the head-word check rather
/// than through its own early return — and per name: renaming `llength`
/// must not cost `list` its fold.
#[test]
fn head_word_arms_decline_only_for_the_name_the_unit_touched() {
    let reg = registry();
    let mut ssa = bare_ssa();
    let count = assign_value_stmt(&mut ssa, "n", "[llength {a b c d}]", 1);
    let listing = assign_value_stmt(&mut ssa, "x", "[list a b c]", 2);

    let renamed = mutations_for("rename llength myll\n");
    assert_eq!(
        evaluate_under_unit(&count, &ssa, &reg, &renamed),
        LatticeValue::Overdefined,
        "a renamed `llength` must not fold to the builtin's answer"
    );
    assert_eq!(
        evaluate_under_unit(&listing, &ssa, &reg, &renamed),
        LatticeValue::Const(ConstValue::String("a b c".into())),
        "`list` is untouched by a `rename llength` and keeps its fold"
    );
}

/// Evaluate `stmt` under `mutations` with the shared per-unit lattice's
/// stance ([`FoldTrust::ObservedBindings`]).
fn evaluate_under_lattice_stance(
    stmt: &SsaStatement,
    ssa: &SsaFunction,
    registry: &CommandRegistry,
    mutations: &crate::command_binding::ModuleCommandMutations,
) -> LatticeValue {
    evaluate_fixture_def_with_folds(
        stmt,
        &HashMap::new(),
        ssa,
        FoldPolicy::default(),
        Some(BuiltinFoldInputs {
            source_metadata_input: None,
            registry,
            mutations,
            dialect: None,
            defining_class: None,
            registry_engine: false,
            trust: FoldTrust::ObservedBindings,
            proven_pure_parameters: false,
        }),
        tcl_lexer::LexerConfig::default(),
    )
}

/// A user `proc` shadowing a builtin is a *named* takeover, so both
/// stances decline it. This is the root of #2164: the shared per-unit
/// lattice took no trust fact at all and answered `3` for a
/// `[llength {a b c}]` the module resolves to a proc returning 99, which
/// the optimiser then published as `return 3` beside its own `set v 99`.
///
/// tclsh 8.4.20 / 8.5.19 / 8.6.18 / 9.0.4 / 9.1b0 (unanimous): with
/// `proc llength {l} {return 99}` in scope, `[llength {a b c}]` is 99.
#[test]
fn both_stances_decline_a_builtin_a_proc_shadows() {
    let reg = registry();
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "n", "[llength {a b c}]", 1);

    let shadowed = mutations_for("proc llength {l} { return 99 }\n");
    assert_eq!(
        evaluate_under_lattice_stance(&stmt, &ssa, &reg, &shadowed),
        LatticeValue::Overdefined,
        "the lattice must not answer with builtin semantics for a shadowed name"
    );
    assert_eq!(
        evaluate_under_unit(&stmt, &ssa, &reg, &shadowed),
        LatticeValue::Overdefined,
        "nor may the rewrite stance"
    );

    // Positive control: the same statement, the same two stances, a module
    // that shadows nothing — both fold, so the declines above are the
    // shadow's doing and not a dead arm.
    let untouched = mutations_for("set y 1\n");
    assert_eq!(
        evaluate_under_lattice_stance(&stmt, &ssa, &reg, &untouched),
        LatticeValue::Const(ConstValue::Int(3)),
    );
    assert_eq!(
        evaluate_under_unit(&stmt, &ssa, &reg, &untouched),
        LatticeValue::Const(ConstValue::Int(3)),
    );
}

/// Where the two stances deliberately differ: a command head an autoload
/// index in view may define raises the summary's unbounded `dynamic` top,
/// which names no subject. The rewrite stance declines on it (a rewrite
/// must be certain); the lattice stance keeps folding, or a single
/// unknown library call would cost a file every constant it has.
#[test]
fn only_the_rewrite_stance_declines_on_the_unbounded_top() {
    let reg = registry();
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "n", "[llength {a b c}]", 1);

    let opaque = mutations_for("someUnknownLibraryCall x\n");
    assert!(
        opaque.has_dynamic_mutation(),
        "an unresolved head is expected to raise the unbounded top"
    );
    assert!(
        opaque.observed_binding_is_the_builtin("llength"),
        "but it names no subject, so `llength`'s observed binding is intact"
    );
    assert_eq!(
        evaluate_under_lattice_stance(&stmt, &ssa, &reg, &opaque),
        LatticeValue::Const(ConstValue::Int(3)),
    );
    assert_eq!(
        evaluate_under_unit(&stmt, &ssa, &reg, &opaque),
        LatticeValue::Overdefined,
        "a fold that becomes a rewrite declines on the unbounded top"
    );
}

/// A `rename` whose **subject** the scan cannot name distrusts every
/// builtin, where an unresolved command *head* does not (#2168).
///
/// Both raise the unbounded top, which is why gating the lattice on the
/// whole of it was rejected in #2164 — it would take the fold in
/// [`only_the_rewrite_stance_declines_on_the_unbounded_top`] with it. The
/// distinction is whether something was definitely rebound: `rename $a {}`
/// moved *some* command, so no name is claimable; `someUnknownLibraryCall
/// x` moved nothing.
///
/// tclsh 8.6.18 and 9.0.4 both print 99 for the repro on #2168, where the
/// optimiser rewrote the body to `return 3` — and its own output literally
/// read `rename llength {}`, having constant-propagated the operands in
/// the same run.
#[test]
fn an_unnameable_rename_subject_distrusts_where_an_unknown_head_does_not() {
    let reg = registry();
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "n", "[llength {a b c}]", 1);

    let computed = mutations_for(
        "set a llength
rename $a {}
",
    );
    assert!(
        computed.has_dynamic_mutation(),
        "a computed rename raises the unbounded top"
    );
    assert!(
        !computed.observed_binding_is_the_builtin("llength"),
        "and, unlike an unknown head, it leaves no name claimable"
    );
    assert_eq!(
        evaluate_under_lattice_stance(&stmt, &ssa, &reg, &computed),
        LatticeValue::Overdefined,
        "the lattice must not fold a builtin a computed rename may have moved"
    );

    // Positive control, and the coverage #2164 measured: an unresolved
    // head still raises the top yet still folds, so this change cannot
    // have been bought by gating on `dynamic`.
    let opaque = mutations_for(
        "someUnknownLibraryCall x
",
    );
    assert!(opaque.has_dynamic_mutation());
    assert_eq!(
        evaluate_under_lattice_stance(&stmt, &ssa, &reg, &opaque),
        LatticeValue::Const(ConstValue::Int(3)),
    );
}

/// The same rule for a body the source-recursive walk never sees. A proc
/// installed through an alias prefix is recovered only by the closed
/// command lattice, so its unnameable rename subject reaches the optimiser
/// through `mutation_projection` or not at all (#2168).
///
/// Measured end to end before this was joined: the program below prints
/// **99**, and `tcl optimise` rewrote it into one printing **3** on tclsh
/// 8.6.18 and 9.0.4.
#[test]
fn an_unnameable_rename_subject_inside_an_alias_defined_body_also_distrusts() {
    let reg = registry();
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "n", "[llength {a b c}]", 1);

    // `makep {} BODY` is `proc p {} BODY`; BODY is never walked as source.
    let aliased = mutations_for(
        "proc mylen {l} { return 99 }
interp alias {} makep {} proc p
makep {} {set a llength; set b mylen; rename $a {}; rename $b $a}
p
",
    );
    assert!(
        !aliased.observed_binding_is_the_builtin("llength"),
        "the projection is the only witness that the subject was unnameable"
    );
    assert_eq!(
        evaluate_under_lattice_stance(&stmt, &ssa, &reg, &aliased),
        LatticeValue::Overdefined,
        "folding here rewrites a program meaning 99 into one meaning 3"
    );
}

#[test]
fn string_length_fold_counts_in_the_selected_dialects_character_model() {
    // U+1D11E written in a UTF-8 source file: tclsh 9.0 and 9.1 decode
    // the source as UTF-8 and count one scalar; 8.4 to 8.6 decode it in
    // the system encoding and print 4. The registry's route answers
    // under the dialect's release and declines where the source's
    // decoding is ambiguous — 8.x and no selected release.
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "n", "[string length \"\u{1D11E}\"]", 1);
    let fold = |dialect: &str| {
        let folds = folds_for(dialect);
        evaluate_fixture_def_with_folds(
            &stmt,
            &HashMap::new(),
            &ssa,
            FoldPolicy::from_registry(folds.registry),
            Some(folds),
            tcl_lexer::LexerConfig::default(),
        )
    };
    assert_eq!(fold("tcl9.0"), LatticeValue::Const(ConstValue::Int(1)));
    assert_eq!(fold("tcl8.6"), LatticeValue::Overdefined);
    assert_eq!(
        evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()),
        LatticeValue::Overdefined,
        "no selected release leaves the source's decoding ambiguous"
    );

    // A string both models count identically still folds with no selected
    // release — declining those would drop an ordinary optimisation.
    let mut ssa = bare_ssa();
    let ascii = assign_value_stmt(&mut ssa, "n", "[string length \"hello\"]", 1);
    assert_eq!(
        evaluate_pristine(
            &ascii,
            &HashMap::new(),
            &ssa,
            FoldPolicy::for_profile(Some(false), None)
        ),
        LatticeValue::Const(ConstValue::Int(5))
    );
}

#[test]
fn evaluate_def_assign_value_folds_expr_cmd_subst() {
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "x", "[expr {1 + 2}]", 1);
    assert_eq!(
        evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()),
        LatticeValue::Const(ConstValue::Int(3))
    );
}

#[test]
fn evaluate_def_assign_value_folds_format_literal() {
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "s", "[format \"%d-%d\" 1 2]", 1);
    match evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()) {
        LatticeValue::Const(ConstValue::String(s)) => assert_eq!(s, "1-2"),
        other => panic!("expected Const(String), got {other:?}"),
    }
}

#[test]
fn quoted_expr_with_string_var_does_not_fold() {
    // `set r [expr "$a == $b"]` with a="alpha", b="beta": Tcl substitutes
    // the values textually before parsing, so `expr "alpha == beta"`
    // errors (`invalid bareword`). The fold must bail rather than treat
    // the strings as operands and return 0.
    let mut ssa = bare_ssa();
    let mut stmt = assign_value_stmt(&mut ssa, "r", "[expr \"$a == $b\"]", 1);
    let a = ssa.intern_var("a");
    let b = ssa.intern_var("b");
    stmt.uses.insert(a, 1);
    stmt.uses.insert(b, 1);
    let mut values = HashMap::new();
    values.insert(
        (a, 1),
        LatticeValue::Const(ConstValue::String("alpha".into())),
    );
    values.insert(
        (b, 1),
        LatticeValue::Const(ConstValue::String("beta".into())),
    );
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
}

#[test]
fn evaluate_def_assign_value_unknown_cmd_widens() {
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "x", "[nonexistent_fold args]", 1);
    assert_eq!(
        evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()),
        LatticeValue::Overdefined
    );
}

#[test]
fn evaluate_def_assign_value_llength_via_lattice_var() {
    let mut ssa = bare_ssa();
    let mut stmt = assign_value_stmt(&mut ssa, "n", "[llength $lst]", 1);
    let lst = ssa.intern_var("lst");
    stmt.uses.insert(lst, 1);
    let mut values = HashMap::new();
    values.insert(
        (lst, 1),
        LatticeValue::Const(ConstValue::String("a b c".into())),
    );
    assert_eq!(
        evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
        LatticeValue::Const(ConstValue::Int(3))
    );
}

#[test]
fn parse_literal_value_prefers_int() {
    assert_eq!(parse_literal_value("42"), ConstValue::Int(42));
    assert_eq!(parse_literal_value("-5"), ConstValue::Int(-5));
    assert_eq!(parse_literal_value("0"), ConstValue::Int(0));
    assert_eq!(
        parse_literal_value("hello"),
        ConstValue::String("hello".into())
    );
    // Leading-zero and non-canonical integer forms do not round-trip, so
    // they stay strings (lets SCCP apply the per-dialect leading-zero rule).
    assert_eq!(parse_literal_value("08"), ConstValue::String("08".into()));
    assert_eq!(parse_literal_value("010"), ConstValue::String("010".into()));
    assert_eq!(parse_literal_value("+5"), ConstValue::String("+5".into()));
    assert_eq!(parse_literal_value("-0"), ConstValue::String("-0".into()));
}

#[test]
fn cfg_order_appends_unreachable_blocks() {
    let mut f = Function::new("::top", "entry");
    let entry = f.entry;
    let dead = block(&mut f, "dead");
    f.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Return {
        value: None,
        value_word: None,
        span: None,
        expr: None,
        expr_base: None,
        tokens: None,
        braced: false,
    });
    f.blocks.get_mut(&dead).unwrap().terminator = Some(Terminator::Return {
        value: None,
        value_word: None,
        span: None,
        expr: None,
        expr_base: None,
        tokens: None,
        braced: false,
    });
    let order = cfg_order(&f);
    assert!(order.contains(&entry));
    assert!(order.contains(&dead));
}

fn cu(src: &str) -> crate::compilation_unit::CompilationUnit {
    crate::compilation_unit::CompilationUnit::build_for(src, &registry(), false)
}

/// A condition resolves a nested pure command through the driver's
/// services: with `acc` the constant `foobar`, `[string length $acc] ==
/// 6` is true (tclsh 8.4 to 9.1 take the branch); with `acc` a
/// parameter nothing is known and the branch stays open.
#[test]
fn evaluate_branch_resolves_a_nested_command() {
    let known = "proc ::p {} {\n set acc foobar\n if {[string length $acc] == 6} { return 1 } else { return 0 }\n}";
    let unit = cu(known);
    let f = unit.function("::p").unwrap();
    let result = sccp_pristine(&f.cfg, &f.ssa, FoldPolicy::default());
    assert!(
        result
            .constant_branches
            .iter()
            .any(|branch| branch.value && branch.condition.contains("string length")),
        "{:?}",
        result.constant_branches
    );
    let open =
        "proc ::p {acc} {\n if {[string length $acc] == 6} { return 1 } else { return 0 }\n}";
    let unit = cu(open);
    let f = unit.function("::p").unwrap();
    let result = sccp_pristine(&f.cfg, &f.ssa, FoldPolicy::default());
    assert!(
        result.constant_branches.is_empty(),
        "{:?}",
        result.constant_branches
    );
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
    let r = sccp_no_traces(&fu.cfg, &fu.ssa, None, FoldPolicy::default());
    let cb = r
        .constant_branches
        .iter()
        .find(|cb| cb.condition.contains("$i == 10"))
        .expect("post-loop branch must fold via the static-loop summary");
    assert!(cb.value, "i == 10 after the loop, so the branch is true");

    // Body side effects are simulated too: j accumulates to 5.
    let ca = cu(
        "proc ::a {} { set j 0\n for {set k 5} {$k > 0} {incr k -1} { incr j }\n if {$j == 5} { return yes } else { return no } }",
    );
    let fa = ca.function("::a").unwrap();
    let ra = sccp_no_traces(&fa.cfg, &fa.ssa, None, FoldPolicy::default());
    let cba = ra
        .constant_branches
        .iter()
        .find(|cb| cb.condition.contains("$j == 5"))
        .expect("accumulator branch must fold via the static-loop summary");
    assert!(cba.value, "j == 5 after the loop");

    // A loop with an unknown (parameter) bound cannot be summarised, so the
    // post-loop branch stays unfolded (conservative).
    let cq = cu(
        "proc ::q {n} { for {set i 0} {$i < $n} {incr i} {}\n if {$i == 10} { return yes } else { return no } }",
    );
    let fq = cq.function("::q").unwrap();
    let rq = sccp_no_traces(&fq.cfg, &fq.ssa, None, FoldPolicy::default());
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
    let r = sccp_pristine(&fu.cfg, &fu.ssa, FoldPolicy::default());
    let v = fu.ssa.var_symbol("v").expect("v must be an SSA symbol");
    let const_sets: Vec<_> = r
        .values
        .iter()
        .filter(|((sym, _), _)| *sym == v)
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
fn sccp_widens_global_aliased_var_to_overdefined() {
    // A `global`-aliased variable is shared mutable state: SCCP must not
    // fold a constant through it, so the `if {$g == 5}` branch stays
    // unresolved (both arms executable, no constant branch). The matching
    // *local* program does fold — proving the widening is what makes the
    // difference, not an unrelated failure to evaluate.
    let global_src =
        "proc ::p {} { global g\n set g 5\n if {$g == 5} { return 1 } else { return 0 } }";
    let local_src = "proc ::p {} { set x 5\n if {$x == 5} { return 1 } else { return 0 } }";

    let cg = cu(global_src);
    let fg = cg.function("::p").unwrap();
    let rg = sccp_no_traces(&fg.cfg, &fg.ssa, None, FoldPolicy::default());
    assert!(
        rg.constant_branches.is_empty(),
        "global var must not fold a constant branch"
    );

    let cl = cu(local_src);
    let fl = cl.function("::p").unwrap();
    let rl = sccp_no_traces(&fl.cfg, &fl.ssa, None, FoldPolicy::default());
    assert!(
        !rl.constant_branches.is_empty(),
        "local var should still fold the constant branch"
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
    assert!(
        r.constant_branches.is_empty(),
        "a value reachable through an UpFrame must not fold a constant branch, got {:?}",
        r.constant_branches,
    );
}

/// The value the solver gives the last definition of `name` in `f`.
fn last_value(f: &crate::compilation_unit::FunctionUnit, name: &str) -> LatticeValue {
    let symbol = f
        .ssa
        .var_symbol(name)
        .expect("the variable is an SSA symbol");
    f.sccp
        .values
        .iter()
        .filter(|((sym, _), _)| *sym == symbol)
        .max_by_key(|((_, version), _)| *version)
        .map(|(_, value)| value.clone())
        .expect("a definition")
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
    assert_eq!(last_value(f, "go"), LatticeValue::Const(ConstValue::Int(1)));
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
    assert_eq!(last_value(f, "go"), LatticeValue::Const(ConstValue::Int(1)));
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
        LatticeValue::Const(ConstValue::Int(0))
    );
}

/// The values the solver gives the last definitions of `names` in the
/// function `name` of `source`.
/// The lattice value of `name`'s last definition in `::p` of `source`
/// under `dialect`. A statement the lowering did not write has no source
/// metadata, so these read the value a real unit settles.
fn settled(dialect: &str, source: &str, name: &str) -> LatticeValue {
    let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
        source, registry, false, dialect,
    );
    last_value(unit.function("::p").expect("::p is analysed"), name)
}

/// The lattice value of version `version` of `name` in `::p` of `source`
/// under `dialect`, for a name a later arm or merge defines again.
fn settled_at(dialect: &str, source: &str, name: &str, version: u32) -> LatticeValue {
    let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
        source, registry, false, dialect,
    );
    let function = unit.function("::p").expect("::p is analysed");
    let symbol = function
        .ssa
        .var_symbol(name)
        .expect("the variable is an SSA symbol");
    function
        .sccp
        .values
        .get(&(symbol, version))
        .cloned()
        .expect("the version is defined")
}

/// The value the semantic facts hold for `name`'s last definition in
/// `::p` of `source` under `dialect`, and whether the execution lattice,
/// which rewrites read with no further check, holds a constant there.
fn value_and_constant(dialect: &str, source: &str, name: &str) -> (LatticeValue, bool) {
    let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
        source, registry, false, dialect,
    );
    let function = unit.function("::p").expect("::p is analysed");
    let symbol = function
        .ssa
        .var_symbol(name)
        .expect("the variable is an SSA symbol");
    let key = function
        .sccp
        .values
        .keys()
        .filter(|(sym, _)| *sym == symbol)
        .max_by_key(|(_, version)| *version)
        .copied()
        .expect("a definition");
    let semantic = function
        .semantic_values()
        .and_then(|facts| facts.contents(key).cloned())
        .unwrap_or(LatticeValue::Overdefined);
    (
        semantic,
        matches!(function.sccp.values.get(&key), Some(LatticeValue::Const(_))),
    )
}

fn assert_value_and_constant_rows<const N: usize>(
    rows: [(&str, &str, LatticeValue, bool, &str); N],
) {
    for (dialect, body, value, constant, printed) in rows {
        let source = format!("proc p {{}} {{{body}; return $r}}");
        assert_eq!(
            value_and_constant(dialect, &source, "r"),
            (value, constant),
            "{dialect}: {body} (tclsh prints {printed})"
        );
    }
}

/// One row per input: the value the semantic facts hold, and whether the
/// execution lattice holds a constant. A rewrite may erase an expression
/// only on an execution constant; a value alone is knowledge. The last
/// column is what tclsh prints for `proc p {} {BODY; return $r}; p` under
/// that release (8.4.20, 8.6.18, 9.0.4).
#[test]
fn a_fused_expression_has_a_value_and_maybe_an_execution_constant() {
    let int = |value: i64| LatticeValue::Const(ConstValue::Int(value));
    let text = |value: &str| LatticeValue::Const(ConstValue::String(value.to_owned()));
    let float = |value: f64| LatticeValue::Const(ConstValue::Float(value));
    let none = LatticeValue::Overdefined;
    let rows: [(&str, &str, LatticeValue, bool, &str); 16] = [
        // A comparison of two string variables is false, but the numeric
        // comparison may change the operands' representations.
        (
            "tcl8.6",
            "set a alpha; set b beta; set r [expr {$a == $b}]",
            int(0),
            false,
            "0",
        ),
        // A quoted word is substituted before the expression is parsed.
        (
            "tcl8.6",
            "set a 3; set b 4; set r [expr \"$a + $b\"]",
            int(7),
            false,
            "7",
        ),
        (
            "tcl8.6",
            "set a {1 + 1}; set r [expr \"$a * 2\"]",
            int(3),
            false,
            "3",
        ),
        (
            "tcl8.6",
            "set a {1 + 1}; set r [expr {$a * 2}]",
            none.clone(),
            false,
            "can't use non-numeric string as operand of \"*\"",
        ),
        (
            "tcl9.0",
            "set r [expr {1 ? \"yes\" : \"no\"}]",
            text("yes"),
            true,
            "yes",
        ),
        (
            "tcl8.6",
            "set r [expr {\"true\"}]",
            text("true"),
            false,
            "true",
        ),
        ("tcl8.6", "set r [expr {\" 5 \"}]", int(5), false, "5"),
        ("tcl9.0", "set r [expr {\"0x10\"}]", int(16), false, "16"),
        (
            "tcl8.6",
            "set r [expr {\"1.50\"}]",
            float(1.5),
            false,
            "1.5",
        ),
        ("tcl8.6", "set r [expr {\"010\"}]", int(8), false, "8"),
        ("tcl9.0", "set r [expr {\"010\"}]", int(10), false, "10"),
        (
            "tcl9.0",
            "set r [expr {1/0}]",
            none,
            false,
            "divide by zero",
        ),
        (
            "tcl8.6",
            "set r [expr {2 ** 64}]",
            text("18446744073709551616"),
            true,
            "18446744073709551616",
        ),
        (
            "tcl9.0",
            "set r [expr {0 && [error never]}]",
            int(0),
            false,
            "0",
        ),
        // A double from an executed operator keeps its operation under
        // 8.4, whose spelling follows `tcl_precision`.
        (
            "tcl8.4",
            "set r [expr {1 / 4.0}]",
            float(0.25),
            false,
            "0.25",
        ),
        (
            "tcl8.6",
            "set r [expr {1 / 4.0}]",
            float(0.25),
            true,
            "0.25",
        ),
    ];
    assert_value_and_constant_rows(rows);
}

/// [`settled`] for a body: a procedure in a Tcl release, an event handler
/// under `f5-irules`, where a procedure is only run through `call`.
fn settled_body(dialect: &str, body: &str, name: &str) -> LatticeValue {
    if dialect != "f5-irules" {
        return settled(dialect, &format!("proc p {{}} {{{body}}}"), name);
    }
    let source = format!("when RULE_INIT {{{body}}}");
    let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
        &source, registry, false, dialect,
    );
    last_value(
        unit.function("::when::RULE_INIT")
            .expect("the handler is analysed"),
        name,
    )
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
    let int = |n: i64| LatticeValue::Const(ConstValue::Int(n));
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
    let text = |value: &str| LatticeValue::Const(ConstValue::String(value.to_owned()));
    let int = |n: i64| LatticeValue::Const(ConstValue::Int(n));
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
                ConstValue::Int(n) => *n,
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
    let int = |n: i64| LatticeValue::Const(ConstValue::Int(n));
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
        Some(&LatticeValue::Const(ConstValue::Int(1)))
    );
    assert_eq!(
        top.sccp.values.get(&(symbol, 2)),
        Some(&LatticeValue::Overdefined)
    );
    assert_eq!(
        last_value(top, "y"),
        LatticeValue::Const(ConstValue::Int(2))
    );
    assert_eq!(
        last_value(top, "x"),
        LatticeValue::Const(ConstValue::Int(3))
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
        LatticeValue::Const(ConstValue::Int(1))
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

/// A contradiction drops the state of the loop whose state the settled
/// run contradicts and keeps every other loop's: the run is made again
/// with the other loop's refinements alone, and the result records that
/// loop alone. Two loops leave to blocks 3 and 7; the second run
/// contradicts the first's state.
#[test]
fn a_contradiction_drops_only_its_own_loop() {
    let record = |end: u32| EnumeratedLoop {
        exit_block: BlockId(end),
        span: Span::new(0, 0),
        iterations: 1,
        exit: tcl_registry::value_transfer::ExitRule::Exhaustion,
        published: Vec::new(),
    };
    let refinement = |end: u32| EdgeRefinement {
        edge: (BlockId(end - 1), BlockId(end)),
        key: (Symbol(0), 1),
        domain: FactDomain::ExactValue,
        fact: FactView::Exact(ExactValue::int(1), None),
        evidence: DependencyEvidence::default(),
        loop_exit: true,
    };
    let mut rounds = Vec::new();
    let result = drive(|round| {
        let published = match round {
            Round::First => None,
            Round::Published(exits) => Some(
                exits
                    .iter()
                    .map(|refinement| refinement.edge.1)
                    .collect::<Vec<_>>(),
            ),
        };
        rounds.push(published.clone());
        match published {
            None => Solved::Enumerated(
                vec![record(3), record(7)],
                vec![refinement(3), refinement(7)],
            ),
            Some(ends) if ends.contains(&BlockId(3)) => {
                Solved::Contradicted(HashSet::from([BlockId(3)]))
            }
            Some(_) => Solved::Settled(Box::default()),
        }
    });
    assert_eq!(
        rounds,
        [
            None,
            Some(vec![BlockId(3), BlockId(7)]),
            Some(vec![BlockId(7)])
        ]
    );
    assert_eq!(
        result
            .loop_enumerations
            .iter()
            .map(|record| record.exit_block)
            .collect::<Vec<_>>(),
        [BlockId(7)]
    );
}
