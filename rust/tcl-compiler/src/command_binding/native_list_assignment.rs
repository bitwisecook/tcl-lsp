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

//! Native assignment evaluates each original target after the preceding store.

use super::*;
use tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand as Operand;
use tcl_registry::native_list_operations_compilation::{
    NativeListOperationInstruction as Plan, NativeListVariableOperand as Target,
};

struct AssignmentRun<'a> {
    input: SourceCommandInput<'a>,
    base: u32,
    prepared: Box<PreparedSourceArguments>,
    converted: Option<Vec<String>>,
    list_operand: &'a Operand,
    pool: Option<literal_object_pool::SourceOrdinaryLiteralPool>,
}

impl SourceCommandBindings {
    pub(super) fn walk_original_list_assignment(
        &mut self,
        input: SourceCommandInput<'_>,
        base: u32,
        state: &mut ModuleCommandBindings,
        compiled: &compiled_invocation::CompiledInvocationSelection,
        context: &SourceExecutionContext<'_>,
    ) -> Option<SourceOutcomes> {
        if self.declaration_preview_depth != 0
            || compiled.live
            || compiled.unknown
            || compiled.compile_error
            || !compiled.named.is_empty()
            || compiled.proofs.is_empty()
        {
            return None;
        }
        let preparation = compiled.structured.as_ref()?;
        let tcl_registry::native_instruction_plan::NativeInstructionPlan::ListOperations(
            Plan::Assign { list, targets },
        ) = preparation.recipe()
        else {
            return None;
        };
        let mut run = AssignmentRun::new(input, base, list, state);
        let mut outcomes = self.evaluate_assignment_operand(list, &mut run, state, context);
        let mut entered = false;
        let mut evaluated_targets = 0;
        for (index, target) in targets.iter().enumerate() {
            let Some(normal) = outcomes.normal.take() else {
                break;
            };
            publish_source_branch(state, normal);
            let operand = target_operand(target);
            let evaluated = self.evaluate_assignment_operand(operand, &mut run, state, context);
            let name = run.operand_value(operand).map(str::to_owned);
            outcomes.join(&evaluated);
            let Some(normal) = outcomes.normal.take() else {
                break;
            };
            publish_source_branch(state, normal);
            entered = true;
            evaluated_targets += 1;
            outcomes.join(&self.store_assignment_member(
                index,
                name.as_deref(),
                &mut run,
                state,
                *context,
            ));
        }
        if let Some(normal) = outcomes.normal.take() {
            publish_source_branch(state, normal);
            let mut result = SourceOutcomes::invocation(
                state,
                tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                    tcl_registry::completion::CompletionCode::Ok,
                ),
            );
            if let Some(members) = &run.converted {
                result.normal_value = Some(Arc::new(native_result::EvaluatedSourceValue {
                    text: tcl_syntax::list::join_list(members.iter().skip(targets.len())),
                    representation: tcl_syntax::value::ValueRepresentation::List,
                    numeric: None,
                }));
            }
            outcomes.join(&result);
        }
        if entered {
            run.prepared.complete_normally &= evaluated_targets == targets.len();
            run.prepared.effective = run
                .prepared
                .written_arguments
                .iter()
                .flat_map(frozen_arguments::runtime_words)
                .collect();
            let arguments = source_argument_context_boxed(
                context,
                &run.prepared.written_arguments,
                &run.prepared.written_values,
                &run.prepared.written_representations,
                &run.prepared.written_objects,
                &run.prepared.written_method_prefixes,
                &run.prepared.written_variable_reads,
            );
            let input = SourceCommandInput {
                effective: &run.prepared.effective,
                ..input
            };
            let basis = self.record_evaluated_source_point(
                input,
                state,
                compiled,
                SourceObserverExecution::from_observed(false),
                run.prepared.complete_normally,
                &arguments,
            );
            outcomes.retain_complete_normal_evaluation();
            self.record_invocation_normal_result(basis, &outcomes);
        }
        outcomes.publish(state);
        Some(outcomes)
    }

    fn evaluate_assignment_operand(
        &mut self,
        operand: &Operand,
        run: &mut AssignmentRun<'_>,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        match operand {
            Operand::LiteralExpansion {
                original_word,
                value,
                ..
            } => {
                // This member is an authenticated parser TEXT expansion, with no runtime substitutions.
                if std::str::from_utf8(value).is_err() {
                    return opaque_source_invocation(state);
                }
                let Some(word) = run.input.words.get(*original_word) else {
                    return opaque_source_invocation(state);
                };
                let frozen = frozen_arguments::freeze_word(word, None, state, context.registry);
                if let Ok(frozen) = frozen {
                    Arc::make_mut(&mut run.prepared.written_arguments)[*original_word] = frozen;
                }
                SourceOutcomes::normal(state)
            }
            Operand::Original(index) => {
                let Ok(source) = run.input.image.try_text() else {
                    return opaque_source_invocation(state);
                };
                let prepared = self.prepare_source_argument_range(
                    source_arguments::SourceArgumentSlice {
                        source,
                        base: run.base,
                        words: run.input.words,
                        selected: *index..*index + 1,
                    },
                    state,
                    run.input.segment,
                    context,
                );
                run.prepared.complete_normally &= prepared.complete_normally && prepared.ready;
                let ready = prepared.ready;
                if ready {
                    run.replace_argument(*index, &prepared);
                }
                let mut outcomes = prepared.into_outcomes();
                if ready {
                    outcomes.join(&SourceOutcomes::normal(state));
                }
                outcomes
            }
        }
    }

    fn store_assignment_member(
        &mut self,
        index: usize,
        name: Option<&str>,
        run: &mut AssignmentRun<'_>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let closed = run.list_effects_closed(state, context);
        self.record_object_callback_effects(state, run.input.segment.span.start(), closed);
        if !closed {
            return opaque_source_invocation(state);
        }
        if run.converted.is_none() {
            let Some(list) = run.operand_value(run.list_operand) else {
                return opaque_source_invocation(state);
            };
            let Some(dialect) = state.baseline.dialect else {
                return opaque_source_invocation(state);
            };
            let rules =
                tcl_syntax::word_rules::WordValueRules::from_grammar(&dialect.lexer_grammar);
            let Ok(members) = rules.split_list(list) else {
                return SourceOutcomes::invocation(
                    state,
                    tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                        tcl_registry::completion::CompletionCode::Error,
                    ),
                );
            };
            run.converted = Some(
                members
                    .into_iter()
                    .map(std::borrow::Cow::into_owned)
                    .collect(),
            );
        }
        let Some(name) = name else {
            return opaque_source_invocation(state);
        };
        let receiver = crate::var_resolve::resolve_literal_access(
            name,
            &state.source_variables,
            false,
            context.registry,
            tcl_registry::TraceOperation::Write,
        );
        let value = run
            .converted
            .as_ref()
            .and_then(|members| members.get(index))
            .map_or("", String::as_str);
        self.walk_original_opcode_store(
            variable_observers::OriginalOpcodeStore {
                receiver: &receiver,
                reference: Some(name),
                value: Some(value),
                offset: run.input.segment.span.start(),
            },
            state,
            context,
        )
    }
}

fn target_operand(target: &Target) -> &Operand {
    match target {
        Target::Original { operand, .. } | Target::ExpandedLiteral { operand, .. } => operand,
    }
}
impl<'a> AssignmentRun<'a> {
    fn new(
        input: SourceCommandInput<'a>,
        base: u32,
        list_operand: &'a Operand,
        state: &ModuleCommandBindings,
    ) -> Self {
        let mut prepared = PreparedSourceArguments::boxed(input.words.len());
        prepared.written_arguments =
            source_effective_words(input.words, state.baseline.dialect, None).into();
        prepared.written_values.resize(input.words.len(), None);
        prepared
            .written_representations
            .resize(input.words.len(), None);
        prepared.written_objects.resize(input.words.len(), None);
        prepared
            .written_method_prefixes
            .resize(input.words.len(), None);
        prepared
            .written_variable_reads
            .resize(input.words.len(), None);
        Self {
            input,
            base,
            prepared,
            converted: None,
            list_operand,
            pool: state.ordinary_literal_pool.clone(),
        }
    }
    fn replace_argument(&mut self, index: usize, operand: &PreparedSourceArguments) {
        Arc::make_mut(&mut self.prepared.written_arguments)[index] =
            operand.written_arguments[0].clone();
        self.prepared.written_values[index] = operand.written_values[0].clone();
        self.prepared.written_representations[index] = operand.written_representations[0];
        self.prepared.written_objects[index] = operand.written_objects[0].clone();
        self.prepared.written_method_prefixes[index] = operand.written_method_prefixes[0].clone();
        self.prepared.written_variable_reads[index] = operand.written_variable_reads[0].clone();
    }
    fn operand_value<'b>(&'b self, operand: &'b Operand) -> Option<&'b str> {
        match operand {
            Operand::Original(index) => self
                .prepared
                .written_arguments
                .get(*index)?
                .as_registry_word()
                .literal(),
            Operand::LiteralExpansion { value, .. } => std::str::from_utf8(value).ok(),
        }
    }
    fn list_effects_closed(
        &self,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> bool {
        if self.converted.is_some()
            && self.pool == state.ordinary_literal_pool
            && self
                .pool
                .as_ref()
                .is_some_and(literal_object_pool::SourceOrdinaryLiteralPool::effects_current)
        {
            return true;
        }
        match self.list_operand {
            Operand::LiteralExpansion { .. } => state
                .ordinary_literal_pool
                .as_ref()
                .is_some_and(literal_object_pool::SourceOrdinaryLiteralPool::effects_current),
            Operand::Original(index) => {
                self.prepared.written_representations[*index].is_some_and(|receipt| {
                    receipt.callback_input_is_closed_for(
                        &state.source_variables,
                        tcl_registry::list_object_methods::NativeListMethod::Index,
                    )
                }) || self.operand_value(self.list_operand).is_some_and(|value| {
                    literal_object_pool::SourceOrdinaryLiteralObject::capture_original_word(
                        &self.input.words[*index],
                        value,
                        state,
                        context.config,
                    )
                    .is_some()
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_registry::native_compilation::{
        NativeCompilationContext, NativeCompilationFrame, NativeCompilationMode,
    };

    fn analyse(
        source: &str,
        entry: Option<&tcl_runtime_api::NativeCompilationEntry>,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> (SourceCommandBindings, Arc<CommandRegistry>) {
        let registry = Arc::new(CommandRegistry::build_default().project_for_profile(profile));
        let analysis = SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            &registry,
            SourceAnalysisOptions {
                native_entry: entry,
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: NativeCompilationContext {
                    frame: NativeCompilationFrame::ScriptCode,
                    mode: NativeCompilationMode::BytecodeObject,
                    catch_depth: Some(0),
                    loop_depth: 0,
                },
                ..Default::default()
            },
        );
        (analysis, registry)
    }
    #[test]
    fn original_native_assignment_projects_each_store_before_the_next_target() {
        for engine in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let profile = tcl_dialect::DialectProfile::find(engine).unwrap();
            let (_owner, entry) =
                crate::environment_ingress::captured_native_entry_with_owner(profile);
            for source in [
                "lassign {one two} first [set first]; list $first $one",
                "lassign {one two} first a([set first]); list $first $a(one)",
                "lassign {one two} first [proc lassign args {return CUSTOM}; set first]; list $first $one",
            ] {
                let (analysis, registry) = analyse(source, Some(&entry), profile);
                assert_eq!(
                    analysis
                        .final_state
                        .source_variables
                        .literal_value("first", &registry),
                    Some("one"),
                    "{engine}/{source}"
                );
                let target = if source.contains("a([") {
                    "a(one)"
                } else {
                    "one"
                };
                assert_eq!(
                    analysis
                        .final_state
                        .source_variables
                        .literal_value(target, &registry),
                    Some("two"),
                    "{engine}/{source}"
                );
            }
            let (analysis, registry) = analyse(
                "catch {lassign {one two} first [error STOP]} caught; set first",
                Some(&entry),
                profile,
            );
            assert_eq!(
                analysis
                    .final_state
                    .source_variables
                    .literal_value("first", &registry),
                Some("one"),
                "{engine}: later operand error"
            );
        }
    }
    #[test]
    fn disabled_or_unknown_native_assignment_never_borrows_a_first_store() {
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let (_owner, entry) = crate::environment_ingress::captured_native_entry_with_owner(profile);
        let source = "lassign {one two} first [set first]";
        let mut disabled = entry.clone();
        disabled.inline_compilation_disabled = true;
        for entry in [None, Some(&disabled)] {
            let (analysis, registry) = analyse(source, entry, profile);
            assert_eq!(
                analysis
                    .final_state
                    .source_variables
                    .literal_value("first", &registry),
                None
            );
            assert!(
                !analysis
                    .invocation_at_source("lassign", 0)
                    .compiled_candidates
                    .iter()
                    .any(|proof| proof.operation
                        == tcl_registry::SemanticOperationId::Intrinsic(
                            tcl_registry::IntrinsicId::ListAssign
                        ))
            );
        }
        let (analysis, registry) = analyse(
            "trace add variable first write mystery; lassign {one two} first [set first]",
            Some(&entry),
            profile,
        );
        assert_eq!(
            analysis
                .final_state
                .source_variables
                .literal_value("first", &registry),
            None
        );
    }
}
