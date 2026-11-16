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

//! Iteration plans: loop headers, lockstep lists, bounds and steps, and the keys a `dict` body binds.

use super::*;

/// The loops' source layout answers an iteration plan: one binder
/// per name of the var-list word, padded past the list's end, over the one
/// list, the body in the caller's frame with `break` and `continue`
/// absorbed, nothing bound on the zero-iteration path, and the empty string
/// (`foreach`) or the body's results (`lmap`) as the result. A var-list the
/// analysis does not know names no binders; an empty one is the command's
/// error.
#[test]
fn the_source_layout_answers_an_iteration_plan() {
    use tcl_registry::FrameLevel;
    use tcl_registry::value_transfer::{
        Binder, BinderName, BindingKind, BodyPlan, CompletionProtocol, ExitRule, IterationPlan,
        LoopResult,
    };
    let reg = CommandRegistry::build_default();
    for (name, result) in [
        ("foreach", LoopResult::Empty),
        ("lmap", LoopResult::Collected),
    ] {
        let resolved = resolve_semantics(reg.get(name).expect(name), None, None);
        let semantics = resolved.semantics().expect("declared");
        let words = |var_list| {
            TestInputs::new(
                name,
                vec![
                    literal(var_list, None),
                    literal("1 10 2 20", None),
                    literal("puts $a", Some(ArgRole::Body)),
                ],
            )
        };
        let PlanAnswer::Iterate(plan) = semantics.structure(&words("a b")) else {
            panic!("{name}: no iteration plan");
        };
        assert_eq!(
            plan,
            IterationPlan {
                binders: ["a", "b"]
                    .map(|binder| Binder {
                        name: BinderName::Declared(binder.to_owned()),
                        kind: BindingKind::Scalar,
                    })
                    .to_vec(),
                iterable: IterableKind::List(OperandId(1)),
                body: Some(BodyPlan {
                    body: OperandId(2),
                    frame: FrameLevel::Relative(0),
                }),
                exit: ExitRule::Exhaustion,
                zero_iterations_bind: false,
                completion: CompletionProtocol::Absorb(&[
                    tcl_registry::completion::CompletionCode::Break,
                    tcl_registry::completion::CompletionCode::Continue,
                ]),
                result,
            },
            "{name}"
        );
        assert_eq!(
            semantics.structure(&words("")),
            PlanAnswer::Declined(DeclineReason::WrongRepresentation),
            "{name}: an empty var-list"
        );
        let mut unknown = words("a b");
        unknown
            .operands
            .insert(0, FactView::Top(DeclineReason::NotExact));
        assert_eq!(
            semantics.structure(&unknown),
            PlanAnswer::Declined(DeclineReason::NotExact),
            "{name}: an unknown var-list"
        );
    }
}

/// Several var-list and list pairs step their lists in lockstep: the plan
/// binds each group's names in order, the next of its binders from its own
/// list, over the lists in the order the words give them, with the body the
/// last word.
#[test]
fn several_lists_step_in_lockstep() {
    use tcl_registry::FrameLevel;
    use tcl_registry::value_transfer::{BinderName, BodyPlan};
    let reg = CommandRegistry::build_default();
    for name in ["foreach", "lmap"] {
        let resolved = resolve_semantics(reg.get(name).expect(name), None, None);
        let semantics = resolved.semantics().expect("declared");
        let lockstep = TestInputs::new(
            name,
            vec![
                literal("a", None),
                literal("1 2", None),
                literal("b", None),
                literal("3 4", None),
                literal("puts $a$b", Some(ArgRole::Body)),
            ],
        );
        let PlanAnswer::Iterate(plan) = semantics.structure(&lockstep) else {
            panic!("{name}: no lockstep plan");
        };
        assert_eq!(
            plan.binders
                .iter()
                .map(|binder| binder.name.clone())
                .collect::<Vec<_>>(),
            ["a", "b"].map(|name| BinderName::Declared(name.to_owned())),
            "{name}"
        );
        assert_eq!(
            plan.iterable,
            IterableKind::Lockstep(vec![
                ListGroup {
                    binders: 1,
                    list: OperandId(1),
                },
                ListGroup {
                    binders: 1,
                    list: OperandId(3),
                },
            ]),
            "{name}: two lists in lockstep"
        );
        assert_eq!(
            plan.body,
            Some(BodyPlan {
                body: OperandId(4),
                frame: FrameLevel::Relative(0),
            }),
            "{name}"
        );
    }
}

/// The counted and the conditional loop name the words their bound and step
/// are read from: `for start test next body` iterates `Counted` over the start
/// script, the condition and the step script, `while test body` over the
/// condition alone, each with the body run in the caller's frame, `break` and
/// `continue` absorbed, a false condition as the exit, nothing bound per
/// pass, and the empty string as the result. Another word count is the
/// command's error, and the CFG's loop header, a branch, asks neither.
#[test]
fn the_loop_plans_name_their_bound_and_step() {
    use tcl_registry::FrameLevel;
    use tcl_registry::value_transfer::{
        BodyPlan, CompletionProtocol, ExitRule, IterationPlan, LoopResult,
    };
    let reg = CommandRegistry::build_default();
    let absorbed = CompletionProtocol::Absorb(&[
        tcl_registry::completion::CompletionCode::Break,
        tcl_registry::completion::CompletionCode::Continue,
    ]);
    let plan_of = |name: &str, words: Vec<_>| {
        let resolved = resolve_semantics(reg.get(name).expect(name), None, None);
        let semantics = resolved.semantics().expect("declared");
        semantics.structure(&TestInputs::new(name, words))
    };
    let counted = plan_of(
        "for",
        vec![
            literal("set i 0", Some(ArgRole::Body)),
            literal("$i < 10", Some(ArgRole::Expr)),
            literal("incr i 2", Some(ArgRole::Body)),
            literal("puts $i", Some(ArgRole::Body)),
        ],
    );
    assert_eq!(
        counted,
        PlanAnswer::Iterate(IterationPlan {
            binders: Vec::new(),
            iterable: IterableKind::Counted {
                init: OperandId(0),
                condition: OperandId(1),
                next: OperandId(2),
            },
            body: Some(BodyPlan {
                body: OperandId(3),
                frame: FrameLevel::Relative(0),
            }),
            exit: ExitRule::FalseCondition,
            zero_iterations_bind: false,
            completion: absorbed.clone(),
            result: LoopResult::Empty,
        })
    );
    let conditional = plan_of(
        "while",
        vec![
            literal("$i < 10", Some(ArgRole::Expr)),
            literal("incr i", Some(ArgRole::Body)),
        ],
    );
    assert_eq!(
        conditional,
        PlanAnswer::Iterate(IterationPlan {
            binders: Vec::new(),
            iterable: IterableKind::Condition(OperandId(0)),
            body: Some(BodyPlan {
                body: OperandId(1),
                frame: FrameLevel::Relative(0),
            }),
            exit: ExitRule::FalseCondition,
            zero_iterations_bind: false,
            completion: absorbed,
            result: LoopResult::Empty,
        })
    );
    assert_eq!(
        plan_of("while", vec![literal("1", Some(ArgRole::Expr))]),
        PlanAnswer::Declined(DeclineReason::WrongRepresentation)
    );
    for name in ["for", "while"] {
        let resolved = resolve_semantics(reg.get(name).expect(name), None, None);
        let semantics = resolved.semantics().expect("declared");
        let binders = Vec::new();
        let mut header = TestInputs::new(name, vec![literal("1", None)]);
        header.view.layout = InvocationLayout::LoopHeader { binders: &binders };
        assert_eq!(
            semantics.structure(&header),
            PlanAnswer::NoStructure,
            "{name}"
        );
    }
}

/// The structural plan `dict SUB` (or its `::tcl::dict::` spelling when
/// `qualified`) answers over `words`, with `d` holding `prior` when it is
/// not empty.
fn dict_body_plan(
    reg: &CommandRegistry,
    sub: &str,
    qualified: bool,
    words: &[(&'static str, Option<ArgRole>)],
    prior: &str,
) -> PlanAnswer {
    let dict = reg.get("dict").expect("dict");
    let (semantics, command) = if qualified {
        let spec = reg
            .get(if sub == "with" {
                "::tcl::dict::with"
            } else {
                "::tcl::dict::update"
            })
            .expect("the qualified spelling");
        (resolve_semantics(spec, None, None), spec.name)
    } else {
        (
            resolve_semantics(dict, Some(dict.subcommand(sub).expect(sub)), None),
            "dict",
        )
    };
    let semantics = semantics.semantics().expect("a body plan");
    let mut operands: Vec<OperandView<'static>> = Vec::new();
    if !qualified {
        operands.push(literal(if sub == "with" { "with" } else { "update" }, None));
    }
    operands.extend(words.iter().map(|&(text, role)| literal(text, role)));
    let mut inputs = TestInputs::new(command, operands);
    if !qualified {
        inputs.view.argument_offset = 1;
    }
    if !prior.is_empty() {
        inputs.prior.insert(
            "d".to_owned(),
            FactView::Exact(ExactValue::from_literal(prior), None),
        );
    }
    semantics.structure(&inputs)
}

/// `dict with` and `dict update` are structural plans, under both
/// spellings: the binders are a projection on body entry — the proven keys
/// of the dictionary for `dict with` (`set d {a 1}; dict with d {incr a;
/// set result done}` binds `a`; tclsh 8.5 to 9.1 answer `done` and leave
/// `d` as `a 2`), a key path's nested dictionary's keys, and the declared
/// variables for `dict update` — the body runs in the caller's frame, the
/// bound keys are written back into the dictionary operand, and the body's
/// completion is the command's. A dictionary the analysis does not know
/// names no binders, so `dict with` declines; a path key it lacks, or a
/// value that is no dictionary, is the command's error.
#[test]
fn dict_with_binds_the_proven_keys() {
    use tcl_registry::FrameLevel;
    use tcl_registry::value_transfer::{
        Binder, BinderName, BindingKind, BodyPlan, CompletionProtocol, Reconcile,
    };
    let reg = CommandRegistry::build_default();
    let plan_of = |sub, qualified, words: &[(&'static str, Option<ArgRole>)], prior| {
        dict_body_plan(&reg, sub, qualified, words, prior)
    };
    let declared = |names: &[&str]| -> Vec<Binder> {
        names
            .iter()
            .map(|name| Binder {
                name: BinderName::Declared((*name).to_owned()),
                kind: BindingKind::Scalar,
            })
            .collect()
    };
    let body = |at: usize| BodyPlan {
        body: OperandId(at),
        frame: FrameLevel::Relative(0),
    };
    let var = ("d", Some(ArgRole::VarWrite));
    let script = ("incr a; set result done", Some(ArgRole::Body));
    for qualified in [false, true] {
        let offset = usize::from(!qualified);
        assert_eq!(
            plan_of("with", qualified, &[var, script], "a 1"),
            PlanAnswer::Body {
                binders: declared(&["a"]),
                body: body(offset + 1),
                reconcile: Reconcile::WriteBackKeys(OperandId(offset)),
                completion: CompletionProtocol::TclBody,
            },
            "qualified: {qualified}"
        );
        assert_eq!(
            plan_of(
                "with",
                qualified,
                &[var, ("x", None), script],
                "x {a 1 b 2 a 3} y 4"
            ),
            PlanAnswer::Body {
                binders: declared(&["a", "b"]),
                body: body(offset + 2),
                reconcile: Reconcile::WriteBackKeys(OperandId(offset)),
                completion: CompletionProtocol::TclBody,
            },
            "qualified: {qualified}: a key path"
        );
        assert_eq!(
            plan_of("with", qualified, &[var, script], ""),
            PlanAnswer::Declined(DeclineReason::NotExact),
            "qualified: {qualified}: an unknown dictionary"
        );
        assert_eq!(
            plan_of("with", qualified, &[var, ("z", None), script], "x 1"),
            PlanAnswer::Declined(DeclineReason::WrongRepresentation),
            "qualified: {qualified}: a path key the dictionary lacks"
        );
        assert_eq!(
            plan_of(
                "update",
                qualified,
                &[
                    var,
                    ("k", None),
                    ("v", None),
                    ("j", None),
                    ("w", None),
                    script
                ],
                ""
            ),
            PlanAnswer::Body {
                binders: [offset + 2, offset + 4]
                    .map(|at| Binder {
                        name: BinderName::Operand(OperandId(at)),
                        kind: BindingKind::Scalar,
                    })
                    .to_vec(),
                body: body(offset + 5),
                reconcile: Reconcile::WriteBackKeys(OperandId(offset)),
                completion: CompletionProtocol::TclBody,
            },
            "qualified: {qualified}: dict update"
        );
    }
}
