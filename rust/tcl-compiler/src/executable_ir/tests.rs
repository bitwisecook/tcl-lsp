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

use super::*;

use tcl_lexer::Span;
use tcl_registry::{
    ArgRole, CompletionDescriptor, InvocationWordKind, OwnedSubcommandResolution,
    SemanticOperationId, TclType,
};

fn source(span: u32) -> SourceSite {
    SourceSite::source(Span::new(span, span + 1))
}

fn literal(text: &str, span: u32) -> WordExpr {
    WordExpr::Literal {
        text: text.to_owned(),
        source: source(span),
    }
}

fn substitution(text: &str, span: u32) -> WordExpr {
    WordExpr::CommandSubstitution {
        spelling: text.to_owned(),
        source: source(span),
    }
}

fn tokens(words: Vec<WordExpr>) -> crate::ir::CommandTokens {
    let argv_texts = words.iter().map(WordExpr::legacy_text).collect::<Vec<_>>();
    crate::ir::CommandTokens {
        argv: words.iter().map(|word| word.source().span).collect(),
        argv_texts,
        word_exprs: words,
        argv_kinds: Vec::new(),
        single_token_word: Vec::new(),
        all_tokens: Vec::new(),
        expand_word: None,
        synthetic: None,
        evaluated_body: None,
        source_binding: None,
        nested_bindings: Vec::new(),
        variable_accesses: Vec::new(),
        hosted_taint_context: None,
        source_bpf_command: None,
    }
}

fn call(words: Vec<WordExpr>) -> Statement {
    let command = words[0].legacy_text();
    let args = words.iter().skip(1).map(WordExpr::legacy_text).collect();
    Statement::Call {
        span: Span::new(0, 20),
        command,
        canonical_command: None,
        args,
        defs: Vec::new(),
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: Some(tokens(words)),
        foreach_groups: None,
    }
}

fn test_context() -> SemanticContext {
    SemanticContext::for_environment("tcl8.6")
}

/// The catalogue projected for the release [`test_context`] selects.
fn test_registry() -> CommandRegistry {
    CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap())
}

fn find_invoke(function: &ExecutableFunction) -> &GenericInvoke {
    function
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .find_map(|instruction| match instruction {
            ExecutableInstruction::Invoke(invoke) => Some(invoke),
            _ => None,
        })
        .expect("generic invocation")
}

#[test]
fn supplied_executable_metadata_keeps_availability_inside_nested_regions() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Fixed IR metadata control, without a Native entry or execution claim.
    use std::sync::Arc;
    let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let mut registry = baseline
        .commands()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let mut puts = registry.get("puts").unwrap().clone();
    puts.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
    registry.insert(puts);
    let current = Arc::new(baseline.with_command_store(Arc::new(registry)));
    let registry = current.commands();
    let older = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(Arc::clone(registry)),
    );
    let call = call(vec![literal("puts", 0), literal("VALUE", 5)]);
    let script = Script::from_statements(vec![
        Statement::If {
            span: Span::new(0, 20),
            clauses: vec![crate::ir::IfClause {
                condition: ExprNode::Literal {
                    text: "1".into(),
                    start: 0,
                    end: 1,
                },
                condition_span: Span::new(0, 1),
                body: Script::from_statements(vec![call.clone()]),
                body_span: Span::new(0, 20),
                condition_base: None,
            }],
            else_body: None,
            else_span: None,
        },
        call,
    ]);
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    for (availability, expected) in [(Arc::clone(&current), true), (older, false)] {
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            availability,
            tcl_lexer::LexerConfig::for_dialect("tcl8.6"),
        );
        let context = InvocationMetadataContext::for_analysis_input(registry, &input);
        let function = build_linear_executable_ir_with_metadata_context(
            registry,
            context,
            ExecutableFunctionId::new(398),
            &script,
        )
        .expect("complete supplied metadata");
        function.validate().unwrap();
        let calls: Vec<_> = function
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .filter_map(|instruction| match instruction {
                ExecutableInstruction::Invoke(call) => Some(call),
                _ => None,
            })
            .collect();
        assert_eq!(calls.len(), 2);
        assert!(calls.iter().any(|call| call.node.path().len() > 1));
        assert!(
            calls.iter().all(
                |call| matches!(call.resolution, InvocationResolution::Resolved(_)) == expected
            )
        );
    }
}

#[test]
fn supplied_executable_metadata_refuses_missing_and_foreign_owners() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let current = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let foreign = tcl_registry::model::ingress::static_context_for("tcl9.1");
    let script = Script::from_statements(vec![call(vec![literal("puts", 0), literal("VALUE", 5)])]);
    assert_ne!(
        current.commands().snapshot().semantic_key(),
        foreign.commands().snapshot().semantic_key()
    );
    for context in [None, Some(InvocationMetadataContext::from(foreign))] {
        assert_eq!(
            build_linear_executable_ir_with_metadata_context(
                current.commands(),
                context,
                ExecutableFunctionId::new(397),
                &script,
            ),
            Err(ExecutableMetadataBuildDecline::ContextUnavailable)
        );
    }
}

#[test]
fn explicit_unknown_dispatch_keeps_runtime_words_without_builtin_facts() {
    let registry = CommandRegistry::build_default();
    let mut statement = call(vec![literal("set", 0), literal("x", 4), literal("1", 6)]);
    let Statement::Call {
        tokens: Some(tokens),
        ..
    } = &mut statement
    else {
        unreachable!();
    };
    tokens.source_binding = Some(crate::command_binding::SourceInvocationBinding::unknown());
    let function = build_linear_executable_ir(
        &registry,
        Some(test_context()),
        ExecutableFunctionId::new(399),
        &Script::from_statements(vec![statement]),
    )
    .expect("unknown dispatch remains executable");
    function.validate().expect("valid runtime invocation");
    let invocation = find_invoke(&function);
    assert!(matches!(
        invocation.resolution,
        InvocationResolution::Unresolved(_)
    ));
    assert!(invocation.effective_words.is_none());
    assert_eq!(invocation.original_words[0].legacy_text(), "set");
    assert!(!invocation.registry_specialisation_arguments_exact());
}

#[test]
fn alias_prefix_facts_do_not_rewrite_runtime_argument_evaluation() {
    let registry = test_registry();
    let module = crate::lowering::lower_to_ir(
        "interp alias {} prefixed {} list inserted\nprefixed $value",
        &registry,
    );
    let statement = module
        .top_level
        .statements
        .last()
        .expect("alias invocation")
        .clone();
    let function = build_linear_executable_ir(
        &registry,
        Some(test_context()),
        ExecutableFunctionId::new(400),
        &Script::from_statements(vec![statement]),
    )
    .expect("resolved alias remains executable");
    function.validate().expect("valid runtime invocation");
    let invocation = find_invoke(&function);
    assert!(matches!(
        invocation.resolution,
        InvocationResolution::Resolved(_)
    ));
    assert_eq!(invocation.original_words.len(), 2);
    assert_eq!(invocation.original_words[0].legacy_text(), "prefixed");
    let effective = invocation
        .effective_words
        .as_ref()
        .expect("proved alias arguments");
    assert_eq!(effective.words[0].legacy_text(), "::list");
    assert_eq!(effective.words[1].legacy_text(), "inserted");
    assert_eq!(effective.words.len(), 3);
    assert!(!invocation.registry_specialisation_arguments_exact());
    assert_eq!(
        instructions(&function)
            .iter()
            .filter(|instruction| matches!(instruction, ExecutableInstruction::EvaluateWord { .. }))
            .count(),
        2,
        "only written words are evaluated"
    );
}

#[test]
fn completed_cell_names_keep_literal_sigils_empty_roots_and_elements() {
    for (value, root, element) in [
        ("", "", false),
        ("$name", "$name", false),
        ("[name]", "[name]", false),
        ("a($key)", "a", true),
        ("($key)", "", true),
        ("()", "", true),
        ("a(b)c", "a(b)c", false),
    ] {
        assert_eq!(
            CellReference::from_name_data(value),
            CellReference::Named {
                name: root.to_owned(),
                element,
            }
        );
    }
}

fn build(source: &str, id: usize) -> ExecutableFunction {
    let registry = test_registry();
    let module = crate::lowering::lower_to_ir(source, &registry);
    let function = build_linear_executable_ir(
        &registry,
        Some(test_context()),
        ExecutableFunctionId::new(id),
        &module.top_level,
    )
    .expect("structured control remains executable");
    function.validate().expect("valid executable IR");
    function
}

fn instructions(function: &ExecutableFunction) -> Vec<&ExecutableInstruction> {
    function
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .collect()
}

fn structured_regions(function: &ExecutableFunction) -> Vec<&StructuredRegion> {
    instructions(function)
        .into_iter()
        .filter_map(|instruction| match instruction {
            ExecutableInstruction::CompleteStructuredRegion(region) => Some(region),
            _ => None,
        })
        .collect()
}

fn region_block(function: &ExecutableFunction) -> ExecutableBlockId {
    function
        .blocks
        .iter()
        .find(|block| {
            block.instructions.iter().any(|instruction| {
                matches!(
                    instruction,
                    ExecutableInstruction::CompleteStructuredRegion(_)
                )
            })
        })
        .expect("structured region")
        .id
}

#[test]
fn if_becomes_branch_edges_joining_at_the_region_completion() {
    let function = build("if {$enabled} {puts on} else {puts off}", 200);
    assert!(
        instructions(&function).iter().all(|instruction| !matches!(
            instruction,
            ExecutableInstruction::ExecuteOpaqueRegion(_)
        )),
        "an `if` no longer needs an opaque compatibility barrier"
    );
    let conditions: Vec<_> = instructions(&function)
        .into_iter()
        .filter(|instruction| {
            matches!(
                instruction,
                ExecutableInstruction::EvaluateExpr {
                    expr: ExecutableExpr::Condition { .. },
                    ..
                }
            )
        })
        .collect();
    assert_eq!(conditions.len(), 1, "one clause, one condition");
    let branches = function
        .blocks
        .iter()
        .filter(|block| matches!(block.terminator, Some(ExecutableTerminator::Branch { .. })))
        .count();
    assert_eq!(branches, 1);
    let regions = structured_regions(&function);
    assert_eq!(regions.len(), 1);
    assert_eq!(regions[0].kind, StructuredRegionKind::Conditional);
    assert_eq!(regions[0].descriptor, LoweringHookId::If);
    // Both arms converge on the block that produces the region completion.
    let join = region_block(&function);
    let predecessors = function
            .blocks
            .iter()
            .filter(|block| {
                matches!(block.terminator, Some(ExecutableTerminator::Goto(target)) if target == join)
            })
            .count();
    assert_eq!(predecessors, 2, "then and else arms both join");
}

#[test]
fn while_loop_has_an_explicit_back_edge_to_its_header() {
    let function = build("while {$more} {puts tick}", 201);
    let header = function
        .blocks
        .iter()
        .find(|block| {
            matches!(
                block.instructions.first(),
                Some(ExecutableInstruction::EvaluateExpr {
                    expr: ExecutableExpr::Condition { .. },
                    ..
                })
            )
        })
        .expect("loop header")
        .id;
    let back_edges = function
        .blocks
        .iter()
        .filter(|block| {
            block.id.index() > header.index()
                && matches!(
                    block.terminator,
                    Some(ExecutableTerminator::Goto(target)) if target == header
                )
        })
        .count();
    assert_eq!(back_edges, 1, "the loop body branches back to the header");
    assert_eq!(
        structured_regions(&function)[0].kind,
        StructuredRegionKind::ConditionLoop
    );
}

#[test]
fn break_and_continue_are_routed_to_loop_targets_not_out_of_the_function() {
    let function = build("while {1} {break}", 202);
    let exit = region_block(&function);
    let routed = function
        .blocks
        .iter()
        .filter_map(|block| match &block.terminator {
            Some(ExecutableTerminator::CompletionSwitch { cases, .. }) => Some(cases),
            _ => None,
        })
        .filter(|cases| {
            cases
                .iter()
                .any(|case| case.code == CompletionCode::Break && case.target == exit)
        })
        .count();
    assert!(
        routed > 0,
        "a completion of code TCL_BREAK inside the body reaches the loop exit"
    );
    assert!(
        function
            .blocks
            .iter()
            .filter_map(|block| match &block.terminator {
                Some(ExecutableTerminator::CompletionSwitch { cases, .. }) => Some(cases),
                _ => None,
            })
            .any(|cases| cases
                .iter()
                .any(|case| case.code == CompletionCode::Continue)),
        "the loop body also names an explicit continue target"
    );
}

#[test]
fn every_non_ok_completion_leaves_the_function_at_top_level() {
    let function = build("puts hello", 203);
    for block in &function.blocks {
        let Some(ExecutableTerminator::CompletionSwitch { cases, default, .. }) = &block.terminator
        else {
            continue;
        };
        assert_eq!(
            cases.len(),
            1,
            "outside a loop only the OK edge is named; every other code unwinds"
        );
        assert!(cases[0].code.is_ok());
        let unwind = function
            .blocks
            .iter()
            .find(|candidate| candidate.id == *default)
            .expect("unwind block");
        assert!(matches!(
            unwind.terminator,
            Some(ExecutableTerminator::ReturnCompletion(_))
        ));
    }
}

#[test]
fn for_loop_continues_through_its_next_script() {
    let function = build("for {set i 0} {$i < 4} {incr i} {puts $i}", 204);
    let continue_targets: BTreeSet<_> = function
        .blocks
        .iter()
        .filter_map(|block| match &block.terminator {
            Some(ExecutableTerminator::CompletionSwitch { cases, .. }) => Some(cases),
            _ => None,
        })
        .flat_map(|cases| cases.iter())
        .filter(|case| case.code == CompletionCode::Continue)
        .map(|case| case.target)
        .collect();
    assert_eq!(continue_targets.len(), 1);
    let target = *continue_targets.iter().next().expect("continue target");
    // The `for` continue target runs the loop's `next` script, so it is not
    // the header itself.
    let next_block = function
        .blocks
        .iter()
        .find(|block| block.id == target)
        .expect("continue block");
    assert!(matches!(
        next_block.instructions.first(),
        Some(ExecutableInstruction::ExecuteLowered(LoweredOperation {
            descriptor: LoweringHookId::Incr,
            ..
        }))
    ));
}

#[test]
fn foreach_becomes_a_list_cursor_loop_that_binds_its_variables() {
    let function = build(
        "set pairs [gets stdin]\nforeach {a b} $pairs {puts $a}",
        205,
    );
    let cursor = instructions(&function)
        .into_iter()
        .find_map(|instruction| match instruction {
            ExecutableInstruction::IterateLists { groups, .. } => Some(groups),
            _ => None,
        })
        .expect("list-cursor loop header");
    assert_eq!(cursor.len(), 1);
    assert_eq!(
        cursor[0].variables,
        vec![
            CellReference::Named {
                name: "a".to_owned(),
                element: false
            },
            CellReference::Named {
                name: "b".to_owned(),
                element: false
            },
        ]
    );
    assert_eq!(
        structured_regions(&function)[0].kind,
        StructuredRegionKind::CursorLoop
    );
}

#[test]
fn dict_iteration_keeps_its_opaque_region() {
    let function = build("dict for {k v} $d {puts $k}", 206);
    assert!(
        instructions(&function).iter().any(|instruction| matches!(
            instruction,
            ExecutableInstruction::ExecuteOpaqueRegion(_)
        )),
        "a dict cursor is not the Tcl-list cursor this projection models"
    );
}

#[test]
fn catch_joins_its_abrupt_edge_and_writes_the_result_cells() {
    let function = build("catch {error boom} msg opts", 207);
    let joins = instructions(&function)
        .into_iter()
        .filter(|instruction| matches!(instruction, ExecutableInstruction::JoinCompletion { .. }))
        .count();
    assert_eq!(joins, 1, "every abrupt edge out of the body joins once");
    let written: Vec<_> = instructions(&function)
        .into_iter()
        .filter_map(|instruction| match instruction {
            ExecutableInstruction::WriteCompletionCell { payload, cell, .. } => {
                Some((*payload, cell.clone()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        written,
        vec![
            (
                CompletionPayload::Result,
                CellReference::Named {
                    name: "msg".to_owned(),
                    element: false
                }
            ),
            (
                CompletionPayload::Options,
                CellReference::Named {
                    name: "opts".to_owned(),
                    element: false
                }
            ),
        ]
    );
    assert_eq!(
        structured_regions(&function)[0].kind,
        StructuredRegionKind::Catch
    );
}

#[test]
fn try_routes_handlers_by_completion_class() {
    let function = build("try {risky} on error {m} {puts $m} finally {cleanup}", 208);
    let joined = instructions(&function)
        .into_iter()
        .find_map(|instruction| match instruction {
            ExecutableInstruction::JoinCompletion { completion, .. } => Some(*completion),
            _ => None,
        })
        .expect("try joins its abrupt edges");
    let cases = function
        .blocks
        .iter()
        .find_map(|block| match &block.terminator {
            Some(ExecutableTerminator::CompletionSwitch {
                completion, cases, ..
            }) if *completion == joined => Some(cases.clone()),
            _ => None,
        })
        .expect("the joined completion selects a handler by its code");
    assert_eq!(
        cases.iter().map(|case| case.code).collect::<Vec<_>>(),
        vec![CompletionCode::Error]
    );
    assert_eq!(
        structured_regions(&function)[0].kind,
        StructuredRegionKind::Try
    );
}

#[test]
fn switch_becomes_a_decision_tree_with_shared_fallthrough_bodies() {
    let function = build(
        "set mode [gets stdin]\nswitch $mode {a - b {puts ab} c {puts c}}",
        209,
    );
    let patterns: Vec<_> = instructions(&function)
        .into_iter()
        .filter_map(|instruction| match instruction {
            ExecutableInstruction::MatchPattern { pattern, .. } => Some(pattern.text.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(patterns, vec!["a", "b", "c"]);
    // `a -` falls through, so both `a` and `b` branch to the same body.
    let targets: Vec<_> = function
        .blocks
        .iter()
        .filter_map(|block| match &block.terminator {
            Some(ExecutableTerminator::Branch { then_target, .. }) => Some(*then_target),
            _ => None,
        })
        .collect();
    assert_eq!(targets.len(), 3);
    assert_eq!(targets[0], targets[1], "a fallthrough arm shares one body");
    assert_ne!(targets[1], targets[2]);
    assert_eq!(
        structured_regions(&function)[0].kind,
        StructuredRegionKind::Switch
    );
}

#[test]
fn uplevel_and_inlined_blocks_stay_opaque() {
    let function = build("uplevel 1 {set outer 1}", 210);
    let region = instructions(&function)
        .into_iter()
        .find_map(|instruction| match instruction {
            ExecutableInstruction::ExecuteOpaqueRegion(region) => Some(region),
            _ => None,
        })
        .expect("an inlined uplevel body is not straight-line splice-able yet");
    assert_eq!(region.descriptor, Some(LoweringHookId::Uplevel));
}

#[test]
fn constant_assignment_has_an_exact_footprint_and_cannot_fail() {
    let function = build("set counter 0", 211);
    let operation = instructions(&function)
        .into_iter()
        .find_map(|instruction| match instruction {
            ExecutableInstruction::ExecuteLowered(operation) => Some(operation),
            _ => None,
        })
        .expect("lowered assignment");
    assert_eq!(
        operation.footprint.writes,
        vec![CellReference::Named {
            name: "counter".to_owned(),
            element: false
        }]
    );
    assert_eq!(
        operation.footprint.reads,
        [] as [crate::executable_ir::CellReference; 0]
    );
    assert!(operation.footprint.is_bounded());
    assert_eq!(operation.footprint.completion, vec![CompletionCode::Ok]);
}

#[test]
fn expression_assignment_reads_exactly_its_operand_variables() {
    // A fresh tclsh has no `a` or `b`; reading them would raise first.
    let function = build(
        "set a [gets stdin]\nset b [gets stdin]\nset total [expr {$a + $b}]",
        212,
    );
    let operation = instructions(&function)
        .into_iter()
        .find_map(|instruction| match instruction {
            ExecutableInstruction::ExecuteLowered(operation)
                if operation.footprint.writes
                    == [CellReference::Named {
                        name: "total".to_owned(),
                        element: false,
                    }] =>
            {
                Some(operation)
            }
            _ => None,
        })
        .expect("lowered expression assignment");
    assert_eq!(
        operation.footprint.reads,
        vec![
            CellReference::Named {
                name: "a".to_owned(),
                element: false
            },
            CellReference::Named {
                name: "b".to_owned(),
                element: false
            },
        ]
    );
    assert!(operation.footprint.is_bounded());
    assert_eq!(
        operation.footprint.completion,
        vec![CompletionCode::Ok, CompletionCode::Error]
    );
}

#[test]
fn increment_reads_before_it_writes_and_a_command_operand_is_unbounded() {
    let bounded = build("incr n 2", 213);
    let operation = instructions(&bounded)
        .into_iter()
        .find_map(|instruction| match instruction {
            ExecutableInstruction::ExecuteLowered(operation) => Some(operation),
            _ => None,
        })
        .expect("lowered increment");
    assert_eq!(operation.footprint.writes, operation.footprint.reads);
    assert!(operation.footprint.is_bounded());

    let unbounded = build(
        "proc step {} {return 2}; set total [expr {[step] + 1}]",
        214,
    );
    let operation = instructions(&unbounded)
        .into_iter()
        .find_map(|instruction| match instruction {
            ExecutableInstruction::ExecuteLowered(operation) => Some(operation),
            _ => None,
        })
        .expect("lowered expression assignment");
    assert!(
        operation.footprint.runs_commands,
        "a command substitution in the operand is not bounded by a cell footprint"
    );
    assert!(!operation.footprint.is_bounded());
}

#[test]
fn source_footprints_use_original_name_data_and_refuse_detached_carriers() {
    for (source, expected) in [
        (r"set a\x62 1", "ab"),
        ("set {$name} 1", "$name"),
        ("set {} 1", ""),
    ] {
        let mut function = build(source, 223);
        let operation = function
            .blocks
            .iter_mut()
            .flat_map(|block| &mut block.instructions)
            .find_map(|instruction| match instruction {
                ExecutableInstruction::ExecuteLowered(operation) => Some(operation),
                _ => None,
            })
            .expect("genuine original lowered source assignment");
        assert_eq!(
            operation.footprint.writes,
            vec![CellReference::Named {
                name: expected.to_owned(),
                element: false
            }]
        );
        let original = operation
            .original_tokens
            .take()
            .expect("Script-issued original command");
        let detached = lowered_operation_footprint(&operation.statement, None);
        assert_eq!(detached.writes, vec![CellReference::Computed]);
        assert!(!detached.is_bounded());
        operation.original_tokens = Some(original);
        assert!(function.validate().is_ok());
    }
}

#[test]
fn world_word_source_metadata_retains_full_input_not_borrowed_availability() {
    let fixture =
        crate::provider_fixtures::original_native_module("list $array([rename old new])", "tcl8.6");
    let (context, module) = &*fixture;
    let actual = InvocationMetadataContext::for_module(context.commands(), module).unwrap();
    let availability_only = InvocationMetadataContext::from(context.as_ref());
    for (metadata, expected) in [(actual, Some(Some(true))), (availability_only, Some(None))] {
        let function = build_linear_executable_ir_with_metadata_context(
            context.commands(),
            Some(metadata),
            ExecutableFunctionId::new(1740),
            &module.top_level,
        )
        .unwrap();
        let (value, word) = instructions(&function)
            .into_iter()
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
                .original_word_command_effects(value, word)
                .map(crate::executable_ir::OriginalCommandEffect::into_known),
            expected
        );
    }
}

fn original_expression_source(
    source: &str,
    dialect: &str,
) -> (
    crate::provider_fixtures::OriginalNativeModuleFixture,
    ExecutableFunction,
) {
    let fixture = crate::provider_fixtures::original_native_module(source, dialect);
    let (context, module) = &*fixture;
    let function = build_linear_executable_ir_with_metadata_context(
        context.commands(),
        crate::registry_invocation::InvocationMetadataContext::for_module(
            context.commands(),
            module,
        ),
        ExecutableFunctionId::new(225),
        &module.top_level,
    )
    .expect("actual retained source expression under live Native owner");
    (fixture, function)
}

fn expression_operation(function: &ExecutableFunction) -> &LoweredOperation {
    instructions(function)
        .into_iter()
        .find_map(|instruction| match instruction {
            ExecutableInstruction::ExecuteLowered(operation)
                if matches!(operation.statement, Statement::ExprEval { .. }) =>
            {
                Some(operation)
            }
            _ => None,
        })
        .expect("actual lowered expression source operation")
}

#[test]
fn cursor_loop_names_use_original_list_values_and_refuse_changed_receivers() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        let (_source_owner, function) = original_expression_source(
            "foreach {$literal {a b} a($key) {}} {1 2 3 4} {set result 1}",
            dialect,
        );
        function.validate().expect("authentic cursor source");
        let groups = instructions(&function)
            .into_iter()
            .find_map(|instruction| {
                if let ExecutableInstruction::IterateLists { groups, .. } = instruction {
                    Some(groups)
                } else {
                    None
                }
            })
            .expect("actual list iteration");
        assert_eq!(
            groups[0].variables,
            vec![
                CellReference::Named {
                    name: "$literal".to_owned(),
                    element: false
                },
                CellReference::Named {
                    name: "a b".to_owned(),
                    element: false
                },
                CellReference::Named {
                    name: "a".to_owned(),
                    element: true
                },
                CellReference::Named {
                    name: String::new(),
                    element: false
                },
            ],
            "{dialect}"
        );
    }
    let fixture = crate::provider_fixtures::original_native_module(
        "foreach {$literal} X {set result 1}",
        "tcl9.0",
    );
    let (context, module) = &*fixture;
    let registry = context.commands();
    let statement = &module.top_level.statements[0];
    let Statement::Foreach { iterators, .. } = statement else {
        panic!("actual loop")
    };
    assert!(original_list_iterator_names(registry, statement, iterators).is_some());
    let mut detached = statement.clone();
    if let Statement::Foreach {
        raw_tokens: Some(tokens),
        ..
    } = &mut detached
    {
        tokens.source_binding = None;
    }
    assert!(original_list_iterator_names(registry, &detached, iterators).is_none());
    let mut changed = iterators.clone();
    changed[0].vars[0] = "OTHER".to_owned();
    assert!(original_list_iterator_names(registry, statement, &changed).is_none());
}

#[test]
fn file_line_analysis_cannot_be_emitted_as_a_list_cursor() {
    let registry = CommandRegistry::build_default();
    let module =
        crate::lowering::lower_to_ir("foreachLine {a b} input.txt {set result 1}", &registry);
    let Statement::Foreach { iterators, .. } = &module.top_level.statements[0] else {
        panic!("actual file-line analysis shape");
    };
    assert_eq!(iterators[0].vars, vec!["a b"]);
    assert!(matches!(
        build_linear_executable_ir(
            &registry,
            Some(test_context()),
            ExecutableFunctionId::new(226),
            &module.top_level
        ),
        Err(SourceCompatibilityDecline::UnsupportedStatement {
            kind: "unowned or non-list iteration",
            ..
        })
    ));
}

#[test]
fn expression_footprints_keep_original_reference_form_and_index_reads() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        for (source, expected, unbounded, commands) in [
            ("expr {${a(k)}}", vec![("a", true)], false, false),
            ("expr {${a($k)}}", vec![("a", true)], false, false),
            (
                "expr {$a($k)}",
                vec![("a", true), ("k", false)],
                true,
                false,
            ),
            (
                "expr {$a($b($k))}",
                vec![("a", true), ("b", true), ("k", false)],
                true,
                false,
            ),
            ("expr {$a([index])}", vec![("a", true)], true, true),
            ("expr {${a([index])}}", vec![("a", true)], false, false),
            ("expr {${$name}}", vec![("$name", false)], false, false),
            ("expr {${}}", vec![("", false)], false, false),
            ("expr {${a(b)c}}", vec![("a(b)c", false)], false, false),
            (r#"expr {"pre$k"}"#, vec![("k", false)], false, false),
            ("expr {{pre$k}}", vec![], false, false),
            (
                r#"expr {"$a($k)"}"#,
                vec![("a", true), ("k", false)],
                true,
                false,
            ),
        ] {
            let (_source_owner, function) = original_expression_source(source, dialect);
            function.validate().expect("unchanged authentic footprint");
            let operation = expression_operation(&function);
            let Statement::ExprEval {
                expr, expr_base, ..
            } = &operation.statement
            else {
                panic!("actual source expression");
            };
            assert!(
                crate::registry_invocation::original_lowered_expression_syntax(
                    expr,
                    *expr_base,
                    operation.original_tokens.as_deref().unwrap(),
                )
                .unwrap()
                .1
                .is_some(),
                "{dialect}: {source}: selected original expression and recipe"
            );
            let expected = expected
                .into_iter()
                .map(|(name, element)| CellReference::Named {
                    name: name.to_owned(),
                    element,
                })
                .collect::<Vec<_>>();
            assert_eq!(operation.footprint.reads, expected, "{dialect}: {source}");
            assert_eq!(
                operation.footprint.reads_unbounded, unbounded,
                "{dialect}: {source}"
            );
            assert_eq!(
                operation.footprint.runs_commands, commands,
                "{dialect}: {source}"
            );
        }
    }
}

#[test]
fn expression_footprints_refuse_missing_or_changed_whole_source_syntax() {
    let (_source_owner, function) = original_expression_source("expr {$a($k)}", "tcl8.6");
    let operation = expression_operation(&function);
    let mut detached = operation.original_tokens.as_deref().unwrap().clone();
    detached.source_binding = None;
    for tokens in [None, Some(&detached)] {
        let footprint = lowered_operation_footprint(&operation.statement, tokens);
        assert_eq!(footprint.reads, vec![CellReference::Computed]);
        assert!(footprint.reads_unbounded);
    }
    let mut changed = operation.original_tokens.as_deref().unwrap().clone();
    changed.argv_texts[1] = "OTHER".to_owned();
    let footprint = lowered_operation_footprint(&operation.statement, Some(&changed));
    assert_eq!(footprint.reads, vec![CellReference::Computed]);
    assert!(footprint.reads_unbounded);

    let (_other_owner, other) = original_expression_source("expr {$other}", "tcl8.6");
    let mut changed_ast = operation.statement.clone();
    if let (Statement::ExprEval { expr, .. }, Statement::ExprEval { expr: other, .. }) =
        (&mut changed_ast, &expression_operation(&other).statement)
    {
        *expr = other.clone();
    } else {
        panic!("authentic expression operations");
    }
    let footprint = lowered_operation_footprint(&changed_ast, operation.original_tokens.as_deref());
    assert_eq!(footprint.reads, vec![CellReference::Computed]);
    assert!(footprint.reads_unbounded);

    let (_foreign_owner, foreign) =
        original_expression_source("set prefix 1; expr {$a($k)}", "tcl8.6");
    let footprint = lowered_operation_footprint(
        &operation.statement,
        expression_operation(&foreign).original_tokens.as_deref(),
    );
    assert_eq!(footprint.reads, vec![CellReference::Computed]);
    assert!(footprint.reads_unbounded);
}

#[test]
fn a_retained_footprint_that_disagrees_with_its_statement_is_rejected() {
    let registry = test_registry();
    let module = crate::lowering::lower_to_ir("set counter 0", &registry);
    let function_id = ExecutableFunctionId::new(215);
    let mut function = build_linear_executable_ir(
        &registry,
        Some(test_context()),
        function_id,
        &module.top_level,
    )
    .expect("lowered assignment");
    for block in &mut function.blocks {
        for instruction in &mut block.instructions {
            if let ExecutableInstruction::ExecuteLowered(operation) = instruction {
                operation.footprint.writes = vec![CellReference::Computed];
            }
        }
    }
    assert!(matches!(
        function.validate(),
        Err(ExecutableIrValidationError::OperationFootprintMismatch { .. })
    ));
}

#[test]
fn a_structured_region_still_isolates_the_statements_around_it() {
    let function = build(
        "set total 0\nforeach item {1 2} {incr total}\nreturn $total",
        216,
    );
    let descriptors: Vec<_> = instructions(&function)
        .into_iter()
        .filter_map(|instruction| match instruction {
            ExecutableInstruction::ExecuteLowered(operation) => Some(operation.descriptor),
            ExecutableInstruction::CompleteStructuredRegion(region) => Some(region.descriptor),
            _ => None,
        })
        .collect();
    assert!(descriptors.contains(&LoweringHookId::Set));
    assert!(descriptors.contains(&LoweringHookId::Foreach));
    assert!(descriptors.contains(&LoweringHookId::Return));
}

#[test]
fn unknown_foreach_conversion_preserves_generic_later_dispatch() {
    let function = build(
        "set items [gets stdin]\nset total 0\nforeach item $items {incr total $item}\nreturn $total",
        217,
    );
    assert!(instructions(&function).iter().any(|instruction| matches!(
        instruction,
        ExecutableInstruction::CompleteStructuredRegion(StructuredRegion {
            descriptor: LoweringHookId::Foreach,
            ..
        })
    )));
    assert!(instructions(&function).iter().any(|instruction| matches!(
        instruction,
        ExecutableInstruction::Invoke(invoke)
            if invoke.original_words.first().is_some_and(|word| word.legacy_text() == "return")
    )));
    assert!(!instructions(&function).iter().any(|instruction| matches!(
        instruction,
        ExecutableInstruction::ExecuteLowered(LoweredOperation {
            descriptor: LoweringHookId::Return,
            ..
        })
    )));
}

#[test]
fn substitutions_complete_before_generic_invocation() {
    let script = Script::from_statements(vec![call(vec![
        literal("puts", 0),
        substitution("[make-message]", 5),
    ])]);
    let function = build_linear_executable_ir(
        &CommandRegistry::build_default(),
        Some(test_context()),
        ExecutableFunctionId::new(1),
        &script,
    )
    .expect("a literal command head and structured substitution are supported");
    function.validate().expect("valid executable IR");

    let mut evaluation = None;
    let mut invocation = None;
    for (block_index, block) in function.blocks.iter().enumerate() {
        for instruction in &block.instructions {
            match instruction {
                ExecutableInstruction::EvaluateWord {
                    word: WordExpr::CommandSubstitution { .. },
                    ..
                } => evaluation = Some(block_index),
                ExecutableInstruction::Invoke(_) => invocation = Some(block_index),
                _ => {}
            }
        }
    }
    assert!(evaluation < invocation);
}

#[test]
fn expansion_failure_switches_before_invocation() {
    let expanded = WordExpr::Expand {
        source: source(5),
        word: Box::new(substitution("[not-a-list]", 8)),
    };
    let script = Script::from_statements(vec![call(vec![literal("puts", 0), expanded])]);
    let function = build_linear_executable_ir(
        &CommandRegistry::build_default(),
        Some(test_context()),
        ExecutableFunctionId::new(2),
        &script,
    )
    .expect("expansion is retained as an explicit operation");
    function.validate().expect("valid executable IR");

    let expansion_block = function
        .blocks
        .iter()
        .find(|block| {
            matches!(
                block.instructions.first(),
                Some(ExecutableInstruction::ExpandWord { .. })
            )
        })
        .expect("expansion block");
    let ExecutableTerminator::CompletionSwitch { default, .. } =
        expansion_block.terminator.as_ref().expect("terminator")
    else {
        panic!("an expansion error must dispatch before invocation");
    };
    let failure = function
        .blocks
        .iter()
        .find(|block| block.id == *default)
        .expect("failure block");
    assert!(matches!(
        failure.terminator,
        Some(ExecutableTerminator::ReturnCompletion(_))
    ));
}

#[test]
fn compatibility_builder_carries_complete_registry_facts() {
    let script =
        Script::from_statements(vec![call(vec![literal("incr", 0), literal("counter", 5)])]);
    let function = build_linear_executable_ir(
        &CommandRegistry::build_default(),
        Some(test_context()),
        ExecutableFunctionId::new(7),
        &script,
    )
    .expect("literal registry command");
    let InvocationResolution::Resolved(facts) = &find_invoke(&function).resolution else {
        panic!("literal registry command must resolve");
    };

    assert_eq!(facts.canonical_command, "incr");
    assert_ne!(facts.operation, SemanticOperationId::Invoke);
    assert_eq!(
        facts.completion,
        CompletionDescriptor::exact(&[CompletionCode::Ok, CompletionCode::Error])
    );
    assert!(facts.effects.requires_world_barrier());
    assert_eq!(facts.return_type, Some(TclType::Int));
    assert_eq!(facts.arg_roles, vec![(0, ArgRole::VarWrite)]);
    assert!(facts.arg_roles_complete);
}

#[test]
fn ordinary_lowered_operations_form_one_executable_sequence() {
    let registry = test_registry();
    let module = crate::lowering::lower_to_ir(
        "set counter 0\nincr counter\nexpr {$counter + 1}",
        &registry,
    );
    let function = build_linear_executable_ir(
        &registry,
        Some(test_context()),
        ExecutableFunctionId::new(70),
        &module.top_level,
    )
    .expect("already-lowered operations remain executable");
    function.validate().expect("valid executable operation CFG");

    let descriptors: Vec<_> = function
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .filter_map(|instruction| match instruction {
            ExecutableInstruction::ExecuteLowered(operation) => Some(operation.descriptor),
            _ => None,
        })
        .collect();
    assert_eq!(
        descriptors,
        [
            LoweringHookId::Set,
            LoweringHookId::Incr,
            LoweringHookId::Expr
        ]
    );
}

#[test]
fn braced_array_element_assignment_keeps_exact_target_payload() {
    let registry = test_registry();
    let module = crate::lowering::lower_to_ir("set {totals($key)} 41", &registry);
    let function = build_linear_executable_ir(
        &registry,
        Some(test_context()),
        ExecutableFunctionId::new(71),
        &module.top_level,
    )
    .expect("array-element assignments use the shared set operation");
    let operation = function
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .find_map(|instruction| match instruction {
            ExecutableInstruction::ExecuteLowered(operation) => Some(operation),
            _ => None,
        })
        .expect("lowered assignment");
    assert_eq!(operation.descriptor, LoweringHookId::Set);
    assert!(matches!(
        &operation.statement,
        Statement::AssignConst {
            name,
            name_braced: true,
            value,
            ..
        } if name == "totals($key)" && value == "41"
    ));
}

#[test]
fn structured_region_does_not_decline_following_invocation() {
    let registry = CommandRegistry::build_default();
    let module =
        crate::lowering::lower_to_ir("if {$enabled} {puts enabled}\nputs finished", &registry);
    let function = build_linear_executable_ir(
        &registry,
        Some(test_context()),
        ExecutableFunctionId::new(72),
        &module.top_level,
    )
    .expect("structured control is isolated, not a whole-function decline");
    function
        .validate()
        .expect("valid structured-region sequence");

    let region = function
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .find_map(|instruction| match instruction {
            ExecutableInstruction::CompleteStructuredRegion(region) => Some(region),
            _ => None,
        })
        .expect("if region");
    assert_eq!(region.descriptor, LoweringHookId::If);
    assert_eq!(region.kind, StructuredRegionKind::Conditional);
    assert_eq!(region.node.path(), &[0]);
    // The body's own invocation is now a node *inside* the region, and the
    // statement after the region keeps its own top-level node.
    let invoked: Vec<_> = function
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .filter_map(|instruction| match instruction {
            ExecutableInstruction::Invoke(invoke) => Some(invoke.node.path().to_vec()),
            _ => None,
        })
        .collect();
    assert!(
        invoked
            .iter()
            .any(|path| path.first() == Some(&0) && path.len() > 1)
    );
    assert!(invoked.contains(&vec![1]));
}

#[test]
fn corpus_counter_keeps_facts_around_closed_foreach_region() {
    let registry = test_registry();
    let module = crate::lowering::lower_to_ir(
        "set total 0\nforeach item {1 2} {incr total}\nreturn $total",
        &registry,
    );
    let function = build_linear_executable_ir(
        &registry,
        Some(test_context()),
        ExecutableFunctionId::new(73),
        &module.top_level,
    )
    .expect("ordinary real-job loop shape remains executable");
    function.validate().expect("valid accumulator CFG");

    let instructions: Vec<_> = function
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .collect();
    assert!(instructions.iter().any(|instruction| matches!(
        instruction,
        ExecutableInstruction::ExecuteLowered(LoweredOperation {
            descriptor: LoweringHookId::Set,
            ..
        })
    )));
    assert!(instructions.iter().any(|instruction| matches!(
        instruction,
        ExecutableInstruction::CompleteStructuredRegion(StructuredRegion {
            descriptor: LoweringHookId::Foreach,
            kind: StructuredRegionKind::CursorLoop,
            ..
        })
    )));
    assert!(instructions.iter().any(|instruction| matches!(
        instruction,
        ExecutableInstruction::ExecuteLowered(LoweredOperation {
            descriptor: LoweringHookId::Return,
            ..
        })
    )));
}

#[test]
fn compatibility_builder_preserves_dynamic_subcommand_outcomes() {
    let subcommand = WordExpr::Variable {
        spelling: "$operation".to_owned(),
        source: source(7),
    };
    let script = Script::from_statements(vec![call(vec![
        literal("string", 0),
        subcommand.clone(),
        literal("text", 18),
    ])]);
    let function = build_linear_executable_ir(
        &CommandRegistry::build_default(),
        Some(test_context()),
        ExecutableFunctionId::new(8),
        &script,
    )
    .expect("a dynamic subcommand leaves a generic registry invocation");
    let InvocationResolution::Resolved(facts) = &find_invoke(&function).resolution else {
        panic!("the literal command head must resolve");
    };

    assert!(matches!(
        &facts.subcommand,
        OwnedSubcommandResolution::Indeterminate {
            word_kind: InvocationWordKind::Dynamic
        }
    ));
    assert_eq!(facts.operation, SemanticOperationId::Invoke);
    assert_eq!(find_invoke(&function).original_words[1], subcommand);
}

#[test]
fn compatibility_builder_preserves_ambiguous_subcommand_outcomes() {
    let script = Script::from_statements(vec![call(vec![
        literal("string", 0),
        literal("i", 7),
        literal("text", 9),
    ])]);
    let function = build_linear_executable_ir(
        &CommandRegistry::build_default(),
        Some(test_context()),
        ExecutableFunctionId::new(9),
        &script,
    )
    .expect("an ambiguous subcommand leaves a generic registry invocation");
    let InvocationResolution::Resolved(facts) = &find_invoke(&function).resolution else {
        panic!("the literal command head must resolve");
    };

    assert!(matches!(
        &facts.subcommand,
        OwnedSubcommandResolution::Ambiguous { spelling } if spelling == "i"
    ));
    assert_eq!(facts.operation, SemanticOperationId::Invoke);
}

#[test]
fn compatibility_builder_retains_expansion_for_registry_resolution() {
    let expanded = WordExpr::Expand {
        source: source(5),
        word: Box::new(WordExpr::Variable {
            spelling: "$arguments".to_owned(),
            source: source(8),
        }),
    };
    let script = Script::from_statements(vec![call(vec![literal("incr", 0), expanded.clone()])]);
    let function = build_linear_executable_ir(
        &CommandRegistry::build_default(),
        Some(test_context()),
        ExecutableFunctionId::new(10),
        &script,
    )
    .expect("expanded arguments remain a generic invocation");
    let InvocationResolution::Resolved(facts) = &find_invoke(&function).resolution else {
        panic!("the literal command head must resolve");
    };

    assert_eq!(facts.form, None);
    assert_eq!(find_invoke(&function).original_words[1], expanded);
    let entries = function
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .find_map(|instruction| match instruction {
            ExecutableInstruction::BuildArgv { entries, .. } => Some(entries),
            _ => None,
        })
        .expect("argv build");
    assert!(matches!(entries[1], ArgvEntry::Expanded(_)));
}

#[test]
fn compatibility_builder_evaluates_each_source_word_once() {
    let repeated = WordExpr::Variable {
        spelling: "$message".to_owned(),
        source: source(5),
    };
    let repeated_again = WordExpr::Variable {
        spelling: "$message".to_owned(),
        source: source(15),
    };
    let script = Script::from_statements(vec![call(vec![
        literal("puts", 0),
        repeated,
        repeated_again,
    ])]);
    let function = build_linear_executable_ir(
        &CommandRegistry::build_default(),
        Some(test_context()),
        ExecutableFunctionId::new(11),
        &script,
    )
    .expect("simple generic invocation");
    let evaluated: Vec<_> = function
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .filter_map(|instruction| match instruction {
            ExecutableInstruction::EvaluateWord { value, .. } => Some(*value),
            _ => None,
        })
        .collect();
    let entries = function
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .find_map(|instruction| match instruction {
            ExecutableInstruction::BuildArgv { entries, .. } => Some(entries),
            _ => None,
        })
        .expect("argv build");
    let entry_values: Vec<_> = entries.iter().copied().map(ArgvEntry::value).collect();

    assert_eq!(evaluated.len(), 3);
    assert_eq!(entry_values, evaluated);
    assert_eq!(
        entry_values.iter().copied().collect::<BTreeSet<_>>().len(),
        entry_values.len()
    );
}

#[test]
fn completion_switch_accepts_a_custom_completion_code() {
    let function_id = ExecutableFunctionId::new(3);
    let block0 = ExecutableBlockId::new(function_id, 0);
    let block1 = ExecutableBlockId::new(function_id, 1);
    let block2 = ExecutableBlockId::new(function_id, 2);
    let block3 = ExecutableBlockId::new(function_id, 3);
    let block4 = ExecutableBlockId::new(function_id, 4);
    let block5 = ExecutableBlockId::new(function_id, 5);
    let word_completion = CompletionId::new(function_id, 0);
    let argv_completion = CompletionId::new(function_id, 1);
    let completion = CompletionId::new(function_id, 2);
    let argv = ExecutableArgvId::new(function_id, 0);
    let value = ExecutableValueId::new(function_id, 0);
    let site = source(0);
    let invocation = GenericInvoke {
        completion,
        argv,
        resolution: InvocationResolution::Unresolved(
            OwnedInvocationResolutionUnresolved::UnknownLiteralHead {
                spelling: "fixture".to_owned(),
            },
        ),
        original_words: vec![literal("fixture", 0)],
        source_binding: None,
        effective_words: None,
        node: NodeId::from_path(vec![0]),
        source: site.clone(),
    };
    let function = ExecutableFunction::new(
        function_id,
        block0,
        vec![
            ExecutableBlock {
                id: block0,
                instructions: vec![ExecutableInstruction::EvaluateWord {
                    value,
                    completion: word_completion,
                    word: literal("fixture", 0),
                    node: NodeId::from_path(vec![0]),
                    source: site,
                }],
                terminator: Some(ExecutableTerminator::CompletionSwitch {
                    completion: word_completion,
                    cases: vec![CompletionCase {
                        code: CompletionCode::Ok,
                        target: block1,
                    }],
                    default: block4,
                }),
            },
            ExecutableBlock {
                id: block1,
                instructions: vec![ExecutableInstruction::BuildArgv {
                    argv,
                    completion: argv_completion,
                    entries: vec![ArgvEntry::Value(value)],
                }],
                terminator: Some(ExecutableTerminator::CompletionSwitch {
                    completion: argv_completion,
                    cases: vec![CompletionCase {
                        code: CompletionCode::Ok,
                        target: block2,
                    }],
                    default: block5,
                }),
            },
            ExecutableBlock {
                id: block2,
                instructions: vec![ExecutableInstruction::Invoke(invocation)],
                terminator: Some(ExecutableTerminator::CompletionSwitch {
                    completion,
                    cases: vec![CompletionCase {
                        code: CompletionCode::Other(91),
                        target: block3,
                    }],
                    default: block3,
                }),
            },
            ExecutableBlock {
                id: block3,
                instructions: Vec::new(),
                terminator: Some(ExecutableTerminator::ReturnCompletion(completion)),
            },
            ExecutableBlock {
                id: block4,
                instructions: Vec::new(),
                terminator: Some(ExecutableTerminator::ReturnCompletion(word_completion)),
            },
            ExecutableBlock {
                id: block5,
                instructions: Vec::new(),
                terminator: Some(ExecutableTerminator::ReturnCompletion(argv_completion)),
            },
        ],
    );
    function
        .validate()
        .expect("custom Tcl codes are valid switch arms");
}

#[test]
fn value_availability_uses_cfg_dominance_not_block_vector_order() {
    let function_id = ExecutableFunctionId::new(6);
    let block0 = ExecutableBlockId::new(function_id, 0);
    let block1 = ExecutableBlockId::new(function_id, 1);
    let block2 = ExecutableBlockId::new(function_id, 2);
    let block3 = ExecutableBlockId::new(function_id, 3);
    let value = ExecutableValueId::new(function_id, 0);
    let word_completion = CompletionId::new(function_id, 0);
    let argv = ExecutableArgvId::new(function_id, 0);
    let argv_completion = CompletionId::new(function_id, 1);
    let node = NodeId::from_path(vec![0]);
    let function = ExecutableFunction::new(
        function_id,
        block0,
        vec![
            ExecutableBlock {
                id: block0,
                instructions: Vec::new(),
                terminator: Some(ExecutableTerminator::Goto(block2)),
            },
            ExecutableBlock {
                id: block1,
                instructions: vec![ExecutableInstruction::BuildArgv {
                    argv,
                    completion: argv_completion,
                    entries: vec![ArgvEntry::Value(value)],
                }],
                terminator: Some(ExecutableTerminator::ReturnCompletion(argv_completion)),
            },
            ExecutableBlock {
                id: block2,
                instructions: vec![ExecutableInstruction::EvaluateWord {
                    value,
                    completion: word_completion,
                    word: literal("fixture", 0),
                    node,
                    source: source(0),
                }],
                terminator: Some(ExecutableTerminator::CompletionSwitch {
                    completion: word_completion,
                    cases: vec![CompletionCase {
                        code: CompletionCode::Ok,
                        target: block1,
                    }],
                    default: block3,
                }),
            },
            ExecutableBlock {
                id: block3,
                instructions: Vec::new(),
                terminator: Some(ExecutableTerminator::ReturnCompletion(word_completion)),
            },
        ],
    );
    function
        .validate()
        .expect("the definition dominates its use despite vector order");
}

#[test]
fn generic_invoke_retains_original_words_as_non_executable_provenance() {
    let variable = WordExpr::Variable {
        spelling: "$message".to_owned(),
        source: source(5),
    };
    let script = Script::from_statements(vec![call(vec![literal("puts", 0), variable.clone()])]);
    let function = build_linear_executable_ir(
        &CommandRegistry::build_default(),
        Some(test_context()),
        ExecutableFunctionId::new(4),
        &script,
    )
    .expect("simple call");
    let invoke = find_invoke(&function);
    assert_eq!(invoke.original_words, vec![literal("puts", 0), variable]);
    assert!(matches!(
        &invoke.resolution,
        InvocationResolution::Resolved(_)
    ));
}

#[test]
fn compatibility_builder_retains_dynamic_command_heads_as_unresolved() {
    let head = WordExpr::Variable {
        spelling: "$command".to_owned(),
        source: source(0),
    };
    let script = Script::from_statements(vec![call(vec![head.clone(), literal("x", 9)])]);
    let function = build_linear_executable_ir(
        &CommandRegistry::build_default(),
        Some(test_context()),
        ExecutableFunctionId::new(5),
        &script,
    )
    .expect("a computed head remains a generic invocation");
    let invoke = find_invoke(&function);
    assert!(matches!(
        &invoke.resolution,
        InvocationResolution::Unresolved(OwnedInvocationResolutionUnresolved::ComputedHead {
            word_kind: InvocationWordKind::Dynamic,
        })
    ));
    assert_eq!(invoke.original_words, vec![head, literal("x", 9)]);
}
