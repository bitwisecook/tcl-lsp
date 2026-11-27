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

use tcl_lexer::Span;
use tcl_registry::CommandRegistry;
use tcl_registry::model::semantic::SemanticContext;

fn test_context() -> SemanticContext {
    SemanticContext::for_environment("tcl8.6")
}

use super::*;
use crate::executable_ir::{
    CompletionCase, CompletionId, ExecutableArgvId, ExecutableBlock, ExecutableFunctionId,
    GenericInvoke, build_linear_executable_ir, build_linear_executable_ir_with_metadata_context,
};
use crate::lowering::lower_to_ir;
use crate::registry_invocation::resolve_word_exprs;
use crate::state_ssa::CfgStateSite;
use tcl_source::words::NodeId;
use tcl_source::words::SourceSite;
use tcl_source::words::WordExpr;

fn source() -> SourceSite {
    SourceSite::source(Span::new(0, 0))
}

fn literal(text: &str) -> WordExpr {
    WordExpr::Literal {
        text: text.to_owned(),
        source: source(),
    }
}

fn resolved_invoke(
    id: ExecutableFunctionId,
    completion: CompletionId,
    words: &[&str],
) -> ExecutableInstruction {
    let words: Vec<_> = words.iter().map(|word| literal(word)).collect();
    let resolution = resolve_word_exprs(
        &CommandRegistry::build_default(),
        Some(test_context()),
        &words,
    )
    .expect("test command resolves through registry");
    ExecutableInstruction::Invoke(GenericInvoke {
        completion,
        argv: ExecutableArgvId::new(id, completion.index()),
        resolution,
        original_words: words,
        source_binding: None,
        effective_words: None,
        node: NodeId::from_path(vec![
            u32::try_from(completion.index()).expect("test completion index fits u32"),
        ]),
        source: source(),
    })
}

fn switch_function(words: &[&str], same_target: bool) -> ExecutableFunction {
    let id = ExecutableFunctionId::new(71);
    let b0 = ExecutableBlockId::new(id, 0);
    let b1 = ExecutableBlockId::new(id, 1);
    let b2 = ExecutableBlockId::new(id, 2);
    let completion = CompletionId::new(id, 0);
    ExecutableFunction::new(
        id,
        b0,
        vec![
            ExecutableBlock {
                id: b0,
                instructions: vec![resolved_invoke(id, completion, words)],
                terminator: Some(ExecutableTerminator::CompletionSwitch {
                    completion,
                    cases: vec![CompletionCase {
                        code: CompletionCode::Ok,
                        target: b1,
                    }],
                    default: if same_target { b1 } else { b2 },
                }),
            },
            ExecutableBlock {
                id: b1,
                instructions: Vec::new(),
                terminator: Some(ExecutableTerminator::ReturnCompletion(completion)),
            },
            ExecutableBlock {
                id: b2,
                instructions: Vec::new(),
                terminator: Some(ExecutableTerminator::ReturnCompletion(completion)),
            },
        ],
    )
}

fn compiled_function(source: &str, id: usize) -> ExecutableFunction {
    let registry = CommandRegistry::build_default();
    let module = lower_to_ir(source, &registry);
    build_linear_executable_ir(
        &registry,
        Some(test_context()),
        ExecutableFunctionId::new(id),
        &module.top_level,
    )
    .expect("fixture source builds valid executable IR")
}

fn compiled_native_source(source: &str, id: usize) -> ExecutableFunction {
    let fixture = crate::provider_fixtures::original_native_module(source, "tcl8.6");
    let (context, module) = &*fixture;
    build_linear_executable_ir_with_metadata_context(
        context.commands(),
        crate::registry_invocation::InvocationMetadataContext::for_module(
            context.commands(),
            module,
        ),
        ExecutableFunctionId::new(id),
        &module.top_level,
    )
    .expect("actual source-backed module builds executable IR")
}

#[test]
fn structured_expression_effects_retain_exact_source_substitution_shape() {
    for (source, expected) in [
        ("if {1 + 1} {}", false),
        ("if {[rename old new]} {}", true),
        ("switch {[rename old new]} {}", false),
        ("switch [rename old new] {}", true),
        ("foreach name {one two} {}", false),
        ("foreach name [list one two] {}", true),
    ] {
        let function = compiled_native_source(source, 403);
        let expression = function
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .find_map(|instruction| {
                if let ExecutableInstruction::EvaluateExpr { value, expr, .. } = instruction {
                    Some((*value, expr))
                } else {
                    None
                }
            })
            .expect("structured source lowering emits an expression value");
        assert_eq!(
            function
                .original_expression_command_effects(expression.0, expression.1)
                .map(crate::executable_ir::OriginalCommandEffect::into_known),
            Some(Some(expected)),
            "{source}"
        );
        let intents = executable_expr_intents_with_source(&function, expression.0, expression.1);
        assert_eq!(
            intents
                .iter()
                .any(|(_, region)| *region == WorldRegion::Any),
            expected,
            "{source}"
        );
    }
}

#[test]
fn original_word_effects_include_variable_index_commands_but_not_scalar_or_data() {
    for (source, expected) in [
        ("list $array([rename old new])", true),
        ("list prefix$array([rename old new])", true),
        ("list $array($index)", false),
        ("list $scalar", false),
        ("list {$array([rename old new])}", false),
    ] {
        let function = compiled_function(source, 401);
        let (block_index, instruction_index, value, word) = function
            .blocks
            .iter()
            .enumerate()
            .flat_map(|(block_index, block)| {
                block
                    .instructions
                    .iter()
                    .enumerate()
                    .filter_map(move |(index, instruction)| {
                        if let ExecutableInstruction::EvaluateWord { value, word, .. } = instruction
                        {
                            (word.source().span.start() > 0).then_some((
                                block_index,
                                index,
                                *value,
                                word,
                            ))
                        } else {
                            None
                        }
                    })
            })
            .next()
            .expect("actual original operand evaluation");
        assert_eq!(
            function
                .original_word_command_effects(value, word)
                .map(crate::executable_ir::OriginalCommandEffect::into_known),
            Some(Some(expected)),
            "{source}"
        );
        let planner = Planner::new(&function).unwrap();
        let input = planner
            .instruction_plan_inputs(block_index, instruction_index, 0)
            .unwrap();
        assert_eq!(input.is_some(), expected, "{source}");
    }
}

#[test]
fn changed_evaluated_word_cannot_reuse_original_source_effect_absence() {
    let function = compiled_function("list $scalar", 402);
    let (value, original) = function
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .find_map(|instruction| {
            if let ExecutableInstruction::EvaluateWord { value, word, .. } = instruction {
                (word.source().span.start() > 0).then_some((*value, word))
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(
        function
            .original_word_command_effects(value, original)
            .map(crate::executable_ir::OriginalCommandEffect::into_known),
        Some(Some(false))
    );
    let changed = tcl_source::words::WordExpr::Variable {
        spelling: "$different".to_owned(),
        source: original.source().clone(),
    };
    assert_eq!(
        function
            .original_word_command_effects(value, &changed)
            .map(crate::executable_ir::OriginalCommandEffect::into_known),
        Some(None)
    );
}

fn invoke_block_indices(function: &ExecutableFunction) -> Vec<usize> {
    function
        .blocks
        .iter()
        .enumerate()
        .filter_map(|(index, block)| {
            block
                .instructions
                .iter()
                .any(|instruction| matches!(instruction, ExecutableInstruction::Invoke(_)))
                .then_some(index)
        })
        .collect()
}

#[test]
fn may_commit_has_exact_ok_and_joined_abrupt_versions() {
    // `rename` is a registry-declared MayCommit transition. This test
    // deliberately knows no compiler-side command semantics.
    let function = switch_function(&["rename", "old", "new"], false);
    let mut planner = Planner::new(&function).expect("small CFG fits");
    planner.plan().expect("registry completion switch is valid");
    let transition = &planner.plans[0].transitions[0];
    let edges = planner.apply_block_edges(0, vec![Symbol::Initial; planner.locations.len()]);
    let exact = *transition.edge_tokens.get(&1).expect("OK edge token");
    let joined = *transition.edge_tokens.get(&2).expect("abrupt edge token");
    let location = planner.tokens[exact].location;
    assert_eq!(edges[0][location], Symbol::Token(exact));
    assert_eq!(edges[1][location], Symbol::Token(joined));
    assert!(
        planner.plans[0]
            .base
            .iter()
            .any(|token| { planner.tokens[*token].kind == WorldStateIntentKind::Use })
    );
}

#[test]
fn on_ok_transition_is_not_serialised_as_an_unconditional_world_write() {
    // `proc` declares both a command-table side-effect write and an
    // exact OnOkOnly transition. The registry coverage contract makes the
    // latter authoritative, so the definition appears only on the
    // ordinary completion edge.
    let function = switch_function(&["proc", "made", "", ""], false);
    let mut planner = Planner::new(&function).expect("small CFG fits");
    planner.plan().expect("registry completion switch is valid");
    let transition = &planner.plans[0].transitions[0];
    assert!(transition.edge_tokens.contains_key(&1));
    assert!(
        !transition.edge_tokens.contains_key(&2),
        "OnOkOnly must leave the abrupt successor unchanged"
    );
    assert!(planner.plans[0].base.iter().all(|token| {
        !matches!(
            (
                &planner.tokens[*token].kind,
                &planner.locations[planner.tokens[*token].location]
            ),
            (
                WorldStateIntentKind::Def,
                WorldRegion::Scoped {
                    kind: WorldRegionKind::CommandBindings,
                    ..
                }
            )
        )
    }));
}

#[test]
fn closed_alias_transition_has_no_unstamped_predecessor_world_barrier() {
    // `global` is closed as a direct alias operation. Unlike an
    // unstamped Invoke, its registry transition and coverage contract do
    // not fabricate an unconditional `Any` clobber before completion is
    // known.
    let function = switch_function(&["global", "shared"], false);
    let mut planner = Planner::new(&function).expect("small CFG fits");
    planner.plan().expect("registry completion switch is valid");
    assert!(planner.plans[0].base.iter().all(|token| {
        !matches!(
            (
                &planner.tokens[*token].kind,
                &planner.locations[planner.tokens[*token].location]
            ),
            (WorldStateIntentKind::Clobber, WorldRegion::Any)
        )
    }));
    let transition = &planner.plans[0].transitions[0];
    assert!(transition.edge_tokens.contains_key(&1));
    assert!(
        transition.edge_tokens.contains_key(&2),
        "global installs aliases one at a time and therefore preserves its MayCommit join"
    );
}

#[test]
fn mixed_ok_and_default_target_uses_join_not_precise_definition() {
    let function = switch_function(&["rename", "old", "new"], true);
    let mut planner = Planner::new(&function).expect("small CFG fits");
    planner.plan().expect("registry completion switch is valid");
    let transition = &planner.plans[0].transitions[0];
    let edge = planner.apply_block_edges(0, vec![Symbol::Initial; planner.locations.len()]);
    let joined = *transition
        .edge_tokens
        .get(&1)
        .expect("mixed edge join token");
    let location = planner.tokens[joined].location;
    assert_eq!(edge[0][location], Symbol::Token(joined));
}

#[test]
fn unreachable_transition_does_not_change_the_planned_state_graph() {
    let mut function = switch_function(&["rename", "old", "new"], false);
    let unreachable = ExecutableBlockId::new(function.id, 3);
    let completion = CompletionId::new(function.id, 1);
    function.blocks.push(ExecutableBlock {
        id: unreachable,
        instructions: vec![resolved_invoke(
            function.id,
            completion,
            &["proc", "dead", "", ""],
        )],
        terminator: Some(ExecutableTerminator::ReturnCompletion(completion)),
    });
    let mut planner = Planner::new(&function).expect("small CFG fits");
    planner
        .plan()
        .expect("dead transition imposes no completion-edge requirement");
    assert!(planner.locations.iter().all(|location| !matches!(
        location,
        WorldRegion::Scoped {
            kind: WorldRegionKind::CommandBindings,
            subject: WorldSubjectScope::Named(name),
            ..
        } if name == "dead"
    )));
}

#[test]
fn entry_backedge_merges_the_implicit_initial_world_state() {
    let mut function = compiled_function("rename old new\n", 72);
    let invoke = invoke_block_indices(&function)[0];
    let entry = function.entry;
    let ExecutableInstruction::Invoke(generic) = &function.blocks[invoke].instructions[0] else {
        panic!("fixture invocation block contains an invoke");
    };
    let completion = generic.completion;
    let abrupt = ExecutableBlockId::new(function.id, function.blocks.len());
    function.blocks[invoke].terminator = Some(ExecutableTerminator::CompletionSwitch {
        completion,
        cases: vec![CompletionCase {
            code: CompletionCode::Ok,
            target: entry,
        }],
        default: abrupt,
    });
    function.blocks.push(ExecutableBlock {
        id: abrupt,
        instructions: Vec::new(),
        terminator: Some(ExecutableTerminator::ReturnCompletion(completion)),
    });
    let invoke_block = block_id(invoke).expect("fixture block index fits shared BlockId");

    let world = build_executable_world_state_ssa(&function)
        .expect("entry backedge has a representable state merge");
    assert!(world.state.operations().iter().any(|operation| matches!(
        operation,
        StateOp::Phi(phi)
            if phi.includes_initial()
                && matches!(&phi.site, StateSite::Cfg(CfgStateSite {
                    block: BlockId(0),
                    position: CfgStatePosition::Phi { .. },
                }))
                && phi.incoming.contains_key(&invoke_block)
    )));
}

#[test]
fn namespace_and_trace_widening_cover_every_partition() {
    let facts = [
        StateTransitionFact {
            transition: StateTransition::Widen(tcl_registry::StateTransitionWidening {
                domains: vec![StateTransitionDomain::Namespaces],
                subject: TransitionSubject::Literal("dynamic".to_owned()),
            }),
            commit: StateTransitionCommit::OnOkOnly,
        },
        StateTransitionFact {
            transition: StateTransition::Widen(tcl_registry::StateTransitionWidening {
                domains: vec![StateTransitionDomain::CommandTraces],
                subject: TransitionSubject::Literal("dynamic".to_owned()),
            }),
            commit: StateTransitionCommit::OnOkOnly,
        },
        StateTransitionFact {
            transition: StateTransition::Widen(tcl_registry::StateTransitionWidening {
                domains: vec![StateTransitionDomain::ExecutionTraces],
                subject: TransitionSubject::Literal("dynamic".to_owned()),
            }),
            commit: StateTransitionCommit::OnOkOnly,
        },
    ];
    let locations: Vec<_> = project_transition_facts(&facts)
        .into_iter()
        .map(|intent| intent.location)
        .collect();
    assert!(locations.iter().any(|location| matches!(
        location,
        WorldRegion::Scoped {
            kind: WorldRegionKind::NamespaceLookup,
            ..
        }
    )));
    assert!(locations.iter().any(|location| matches!(
        location,
        WorldRegion::Scoped {
            kind: WorldRegionKind::NamespaceUnknown,
            ..
        }
    )));
    assert!(locations.iter().any(|location| matches!(
        location,
        WorldRegion::Scoped {
            kind: WorldRegionKind::CommandTraces,
            ..
        }
    )));
    assert!(locations.iter().any(|location| matches!(
        location,
        WorldRegion::Scoped {
            kind: WorldRegionKind::ExecutionTraces,
            ..
        }
    )));
}

#[test]
fn trace_transition_projection_keeps_command_and_execution_tables_separate() {
    let facts = [
        StateTransitionFact {
            transition: StateTransition::Trace(tcl_registry::TraceTransition::Add {
                target: tcl_registry::TraceTarget::Command(TransitionSubject::Literal(
                    "worker".to_owned(),
                )),
                operations: tcl_registry::TraceOperationSet::Known(vec![
                    tcl_registry::TraceOperation::Rename,
                ]),
                prefix: TransitionSubject::Literal("callback".to_owned()),
            }),
            commit: StateTransitionCommit::OnOkOnly,
        },
        StateTransitionFact {
            transition: StateTransition::Trace(tcl_registry::TraceTransition::Add {
                target: tcl_registry::TraceTarget::Execution(TransitionSubject::Literal(
                    "worker".to_owned(),
                )),
                operations: tcl_registry::TraceOperationSet::Known(vec![
                    tcl_registry::TraceOperation::Enter,
                ]),
                prefix: TransitionSubject::Literal("callback".to_owned()),
            }),
            commit: StateTransitionCommit::OnOkOnly,
        },
    ];
    let locations: Vec<_> = project_transition_facts(&facts)
        .into_iter()
        .map(|intent| intent.location)
        .collect();
    assert!(locations.iter().any(|location| matches!(
        location,
        WorldRegion::Scoped {
            kind: WorldRegionKind::CommandTraces,
            subject: WorldSubjectScope::Named(name),
            ..
        } if name == "worker"
    )));
    assert!(locations.iter().any(|location| matches!(
        location,
        WorldRegion::Scoped {
            kind: WorldRegionKind::ExecutionTraces,
            subject: WorldSubjectScope::Named(name),
            ..
        } if name == "worker"
    )));
}

#[test]
fn variable_trace_removal_projects_placeholder_cell_and_trace_table() {
    let facts = [StateTransitionFact {
        transition: StateTransition::Trace(tcl_registry::TraceTransition::Remove {
            target: tcl_registry::TraceTarget::Variable(TransitionSubject::Literal(
                "item".to_owned(),
            )),
            operations: tcl_registry::TraceOperationSet::Known(vec![
                tcl_registry::TraceOperation::Write,
            ]),
            prefix: TransitionSubject::Literal("callback".to_owned()),
        }),
        commit: StateTransitionCommit::OnOkOnly,
    }];
    let intents = project_transition_facts(&facts);

    for kind in [
        WorldRegionKind::VariableStore,
        WorldRegionKind::VariableTraces,
    ] {
        assert!(intents.iter().any(|intent| {
            intent.kind == WorldStateIntentKind::Use
                && matches!(
                    &intent.location,
                    WorldRegion::Scoped {
                        kind: actual,
                        subject: WorldSubjectScope::Named(name),
                        ..
                    } if *actual == kind && name == "item"
                )
        }));
        assert!(intents.iter().any(|intent| {
            intent.kind == WorldStateIntentKind::Def
                && matches!(
                    &intent.location,
                    WorldRegion::Scoped {
                        kind: actual,
                        subject: WorldSubjectScope::Named(name),
                        ..
                    } if *actual == kind && name == "item"
                )
        }));
    }
}

#[test]
fn namespace_transition_projection_preserves_regions_and_commit_policy() {
    let facts = [
        StateTransitionFact {
            transition: StateTransition::Namespace(NamespaceTransition::Ensure {
                namespace: NamespaceTransitionTarget::Named(TransitionSubject::Literal(
                    "::a::b".to_owned(),
                )),
            }),
            commit: StateTransitionCommit::MayCommitBeforeAbruptCompletion,
        },
        StateTransitionFact {
            transition: StateTransition::Namespace(NamespaceTransition::Delete {
                namespace: NamespaceTransitionTarget::Named(TransitionSubject::Literal(
                    "::gone".to_owned(),
                )),
            }),
            commit: StateTransitionCommit::MayCommitBeforeAbruptCompletion,
        },
    ];
    let intents = project_transition_facts(&facts);

    assert!(intents.iter().any(|intent| matches!(
        &intent.location,
        WorldRegion::NamespaceLineage {
            interpreter: WorldInterpreterScope::Current,
            namespace: WorldNamespaceScope::Named(namespace),
        } if namespace == "::a::b"
    ) && intent.commit
        == StateTransitionCommit::MayCommitBeforeAbruptCompletion));
    assert!(intents.iter().any(|intent| matches!(
        &intent.location,
        WorldRegion::NamespaceSubtree {
            interpreter: WorldInterpreterScope::Current,
            namespace: WorldNamespaceScope::Named(namespace),
        } if namespace == "::gone"
    ) && intent.commit
        == StateTransitionCommit::MayCommitBeforeAbruptCompletion));
}

#[test]
fn rename_target_namespace_transition_projects_its_created_lineage() {
    // The registry emits this companion fact for `rename old
    // ::created::target`; Tcl creates the target namespace if it is
    // missing.  The world-state projection must retain that fact rather
    // than treating the move as only a command-table update.
    let facts = [StateTransitionFact {
        transition: StateTransition::Namespace(NamespaceTransition::Ensure {
            namespace: NamespaceTransitionTarget::Named(TransitionSubject::Literal(
                "::created".to_owned(),
            )),
        }),
        commit: StateTransitionCommit::MayCommitBeforeAbruptCompletion,
    }];

    assert!(
        project_transition_facts(&facts)
            .iter()
            .any(|intent| matches!(
                &intent.location,
                WorldRegion::NamespaceLineage {
                    interpreter: WorldInterpreterScope::Current,
                    namespace: WorldNamespaceScope::Named(namespace),
                } if namespace == "::created"
            ) && intent.commit
                == StateTransitionCommit::MayCommitBeforeAbruptCompletion)
    );
}

#[test]
fn interpreter_transitions_project_lifecycle_policy_and_reverse_alias_safety() {
    let facts = [
        StateTransitionFact {
            transition: StateTransition::Interpreter(InterpreterTransition::Create {
                interpreter: Some(TransitionSubject::Literal("child".to_owned())),
                safety: tcl_registry::ChildInterpreterSafety::Safe,
            }),
            commit: StateTransitionCommit::OnOkOnly,
        },
        StateTransitionFact {
            transition: StateTransition::Interpreter(InterpreterTransition::Delete {
                interpreter: TransitionSubject::Literal("child".to_owned()),
            }),
            commit: StateTransitionCommit::MayCommitBeforeAbruptCompletion,
        },
    ];
    let intents = project_transition_facts(&facts);

    assert!(intents.iter().any(|intent| matches!(
        &intent.location,
        WorldRegion::InterpreterWildcard {
            interpreter: WorldInterpreterScope::Named(path),
        } if path == "child"
    )));
    assert!(intents.iter().any(|intent| matches!(
        &intent.location,
        WorldRegion::Scoped {
            kind: WorldRegionKind::InterpreterTopology,
            interpreter: WorldInterpreterScope::Named(path),
            ..
        } if path == "child"
    )));
    assert!(intents.iter().any(|intent| matches!(
        (&intent.kind, &intent.location),
        (WorldStateIntentKind::Clobber, WorldRegion::Scoped {
            kind: WorldRegionKind::CommandBindings,
            interpreter: WorldInterpreterScope::Any,
            namespace: WorldNamespaceScope::Any,
            subject: WorldSubjectScope::Wildcard,
        }) if intent.commit == StateTransitionCommit::MayCommitBeforeAbruptCompletion
    )));
}

#[test]
fn object_lifecycle_projection_keeps_dispatch_namespace_and_private_variables() {
    let facts = [StateTransitionFact {
        transition: StateTransition::ObjectDispatch(ObjectDispatchTransition::Create {
            target: ObjectDispatchTarget::Named(TransitionSubject::Literal("::object".to_owned())),
            private_namespace: ObjectPrivateNamespace::Fresh,
            kind: tcl_registry::ObjectDispatchKind::Object,
        }),
        commit: StateTransitionCommit::OnOkOnly,
    }];
    let intents = project_transition_facts(&facts);

    assert!(intents.iter().any(|intent| matches!(
        (&intent.kind, &intent.location),
        (
            WorldStateIntentKind::Def,
            WorldRegion::Scoped {
                kind: WorldRegionKind::ObjectDispatch,
                subject: WorldSubjectScope::Named(name),
                ..
            }
        ) if name == "::object"
    )));
    assert!(intents.iter().any(|intent| matches!(
        (&intent.kind, &intent.location),
        (
            WorldStateIntentKind::Def,
            WorldRegion::Scoped {
                kind: WorldRegionKind::NamespaceLookup,
                namespace: WorldNamespaceScope::Any,
                ..
            }
        )
    )));
    assert!(intents.iter().any(|intent| matches!(
        (&intent.kind, &intent.location),
        (
            WorldStateIntentKind::Clobber,
            WorldRegion::Scoped {
                kind: WorldRegionKind::VariableStore,
                namespace: WorldNamespaceScope::Any,
                ..
            }
        )
    )));
    assert!(
        intents
            .iter()
            .all(|intent| intent.commit == StateTransitionCommit::OnOkOnly)
    );
}

#[test]
fn production_missing_word_value_never_borrows_carrierless_compatibility() {
    let function = compiled_function("list $array([rename old new])", 403);
    let (actual, word) = function
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .find_map(|instruction| {
            if let ExecutableInstruction::EvaluateWord { value, word, .. } = instruction {
                (word.source().span.start() > 0).then_some((*value, word))
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(
        function
            .original_word_command_effects(actual, word)
            .map(crate::executable_ir::OriginalCommandEffect::into_known),
        Some(Some(true))
    );
    let missing = crate::executable_ir::ExecutableValueId::new(function.id, usize::MAX);
    assert_eq!(
        function
            .original_word_command_effects(missing, word)
            .map(crate::executable_ir::OriginalCommandEffect::into_known),
        Some(None)
    );
    assert!(original_word_evaluation_runs_commands(
        &function, missing, word
    ));
    // The explicit existing constructor is the only carrierless domain.
    let manual = ExecutableFunction::new(function.id, function.entry, function.blocks.clone());
    assert_eq!(
        manual
            .original_word_command_effects(actual, word)
            .map(crate::executable_ir::OriginalCommandEffect::into_known),
        None
    );
    assert!(!word_evaluation_runs_commands(word));
    assert!(!original_word_evaluation_runs_commands(
        &manual, actual, word
    ));
}
