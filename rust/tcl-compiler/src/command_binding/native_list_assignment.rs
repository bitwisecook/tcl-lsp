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

use super::{
    Arc, ModuleCommandBindings, PreparedSourceArguments, SourceArgumentReceipts,
    SourceCommandBindings, SourceCommandInput, SourceExecutionContext, SourceObserverExecution,
    SourceOutcomes, compiled_invocation, frozen_arguments, literal_object_pool, native_result,
    opaque_source_invocation, original_name_value, publish_source_branch,
    source_argument_context_boxed, source_arguments, source_effective_words, variable_observers,
};
use tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand as Operand;
use tcl_registry::native_list_operations_compilation::{
    NativeListOperationInstruction as Plan, NativeListVariableOperand as Target,
};

struct AssignmentRun<'a> {
    input: SourceCommandInput<'a>,
    base: u32,
    prepared: Box<PreparedSourceArguments>,
    converted: Option<Vec<original_name_value::OriginalProducedNameValue>>,
    conversion_operation: Option<original_name_value::OriginalProducedNameValue>,
    list_operand: &'a Operand,
    pool: Option<literal_object_pool::SourceOrdinaryLiteralPool>,
}

#[derive(Clone, Copy)]
struct AssignmentProgress {
    entered: bool,
    evaluated_targets: usize,
    target_count: usize,
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
        #[cfg(test)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_ASSIGNMENT").is_some() {
            eprintln!(
                "ORIGINAL_ASSIGNMENT offset={} stage=selection preview={} live={} unknown={} error={} named={} proofs={} structured={}",
                input.segment.span.start(),
                self.declaration_preview_depth,
                compiled.live,
                compiled.unknown,
                compiled.compile_error,
                compiled.named.len(),
                compiled.proofs.len(),
                compiled.structured.is_some(),
            );
        }
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
        #[cfg(test)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_ASSIGNMENT").is_some() {
            eprintln!(
                "ORIGINAL_ASSIGNMENT offset={} stage=list normal={} complete={} targets={}",
                input.segment.span.start(),
                outcomes.normal.is_some(),
                run.prepared.complete_normally,
                targets.len(),
            );
        }
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
        Some(self.finish_list_assignment(
            &mut run,
            state,
            compiled,
            context,
            outcomes,
            AssignmentProgress {
                entered,
                evaluated_targets,
                target_count: targets.len(),
            },
        ))
    }

    fn finish_list_assignment(
        &mut self,
        run: &mut AssignmentRun<'_>,
        state: &mut ModuleCommandBindings,
        compiled: &compiled_invocation::CompiledInvocationSelection,
        context: &SourceExecutionContext<'_>,
        mut outcomes: SourceOutcomes,
        progress: AssignmentProgress,
    ) -> SourceOutcomes {
        let input = run.input;
        if let Some(normal) = outcomes.normal.take() {
            publish_source_branch(state, normal);
            let mut result = SourceOutcomes::invocation(
                state,
                tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                    tcl_registry::completion::CompletionCode::Ok,
                ),
            );
            if let Some(members) = &run.converted
                && let Some(operation) = &run.conversion_operation
            {
                let remainder = members.get(progress.target_count..).unwrap_or_default();
                result.normal_name_value =
                    original_name_value::OriginalProducedNameValue::list_result(
                        remainder, operation,
                    )
                    .filter(|value| value.is_current(&state.source_variables))
                    .map(Arc::new);
                let text = result
                    .normal_name_value
                    .as_deref()
                    .and_then(|value| ascii_value(value.bytes()))
                    .map(str::to_owned);
                result.normal_value = text.map(|text| {
                    Arc::new(native_result::EvaluatedSourceValue {
                        text,
                        representation: tcl_syntax::value::ValueRepresentation::List,
                        numeric: None,
                    })
                });
            }
            outcomes.join(&result);
        }
        if progress.entered {
            run.prepared.complete_normally &= progress.evaluated_targets == progress.target_count;
            run.prepared.effective = run
                .prepared
                .written_arguments
                .iter()
                .flat_map(frozen_arguments::runtime_words)
                .collect();
            let argument_context = source_argument_context_boxed(
                context,
                SourceArgumentReceipts {
                    arguments: &run.prepared.written_arguments,
                    values: &run.prepared.written_values,
                    name_values: &run.prepared.written_name_values,
                    representations: &run.prepared.written_representations,
                    objects: &run.prepared.written_objects,
                    prefixes: &run.prepared.written_method_prefixes,
                    reads: &run.prepared.written_variable_reads,
                },
            );
            let arguments = argument_context.context();
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
        outcomes
    }

    fn evaluate_assignment_operand(
        &mut self,
        operand: &Operand,
        run: &mut AssignmentRun<'_>,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        match operand {
            Operand::LiteralExpansion { original_word, .. } => {
                // This member is an authenticated parser TEXT expansion, with no runtime substitutions.
                let Some(word) = run.input.words.get(*original_word) else {
                    return opaque_source_invocation(state);
                };
                let Some(name_value) =
                    super::original_name_value::capture_word(word, state, context.config)
                else {
                    return opaque_source_invocation(state);
                };
                let Ok(frozen) = frozen_arguments::freeze_word(
                    word,
                    None,
                    Some(&name_value),
                    state,
                    context.registry,
                ) else {
                    return SourceOutcomes::invocation(
                        state,
                        tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                            tcl_registry::completion::CompletionCode::Error,
                        ),
                    );
                };
                Arc::make_mut(&mut run.prepared.written_arguments)[*original_word] = frozen;
                run.prepared.written_name_values[*original_word] = Some(Arc::new(name_value));
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

    fn assignment_conversion_failure(
        &mut self,
        index: usize,
        run: &mut AssignmentRun<'_>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceOutcomes> {
        #[cfg(not(test))]
        let _ = index;
        let closed = run.list_effects_closed(state, context);
        #[cfg(test)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_ASSIGNMENT").is_some() {
            eprintln!(
                "ORIGINAL_ASSIGNMENT offset={} target={} stage=conversion effects={} converted={} pool={}",
                run.input.segment.span.start(),
                index,
                closed,
                run.converted.is_some(),
                state
                    .ordinary_literal_pool
                    .as_ref()
                    .is_some_and(literal_object_pool::SourceOrdinaryLiteralPool::effects_current),
            );
        }
        self.record_object_callback_effects(state, run.input.segment.span.start(), closed);
        if !closed {
            return Some(opaque_source_invocation(state));
        }
        if run.converted.is_none() {
            let Some(list) = run.operand_input(run.list_operand, state, context.config) else {
                return Some(opaque_source_invocation(state));
            };
            let Some(children) = list.original_list_elements() else {
                return Some(
                    if tcl_syntax::list::split_native_list_bytes(
                        list.bytes(),
                        list.policy().string_protocol(),
                    )
                    .is_err()
                    {
                        SourceOutcomes::invocation(
                            state,
                            tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                                tcl_registry::completion::CompletionCode::Error,
                            ),
                        )
                    } else {
                        opaque_source_invocation(state)
                    },
                );
            };
            run.conversion_operation =
                original_name_value::OriginalProducedNameValue::from_source_input(
                    &list,
                    &state.source_variables,
                );
            run.converted = children
                .iter()
                .map(|child| {
                    original_name_value::OriginalProducedNameValue::from_source_input(
                        child,
                        &state.source_variables,
                    )
                })
                .collect();
            if run.conversion_operation.is_none() || run.converted.is_none() {
                return Some(opaque_source_invocation(state));
            }
        }
        None
    }

    fn store_assignment_member(
        &mut self,
        index: usize,
        name: Option<&str>,
        run: &mut AssignmentRun<'_>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        if let Some(failure) = self.assignment_conversion_failure(index, run, state, context) {
            return failure;
        }
        let argument_context = source_argument_context_boxed(
            &context,
            SourceArgumentReceipts {
                arguments: &run.prepared.written_arguments,
                values: &run.prepared.written_values,
                name_values: &run.prepared.written_name_values,
                representations: &run.prepared.written_representations,
                objects: &run.prepared.written_objects,
                prefixes: &run.prepared.written_method_prefixes,
                reads: &run.prepared.written_variable_reads,
            },
        );
        let arguments = argument_context.context();
        let Some(compilation) = arguments.original_variable_compilation else {
            #[cfg(test)]
            if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_ASSIGNMENT").is_some() {
                eprintln!(
                    "ORIGINAL_ASSIGNMENT offset={} target={} stage=missing-compilation",
                    run.input.segment.span.start(),
                    index
                );
            }
            return opaque_source_invocation(state);
        };
        let Some(target) =
            super::original_variable_compilation::original_compiler_list_assignment_operand(
                compilation,
                index,
                &state.source_variables,
            )
        else {
            #[cfg(test)]
            if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_ASSIGNMENT").is_some() {
                eprintln!(
                    "ORIGINAL_ASSIGNMENT offset={} target={} stage=missing-target",
                    run.input.segment.span.start(),
                    index
                );
            }
            return opaque_source_invocation(state);
        };
        let receiver = target.resolve(
            &state.source_variables,
            context.registry,
            false,
            tcl_registry::TraceOperation::Write,
        );
        let name_value = run
            .converted
            .as_ref()
            .and_then(|members| members.get(index))
            .cloned()
            .or_else(|| {
                original_name_value::OriginalProducedNameValue::list_assignment_empty(
                    run.conversion_operation.as_ref()?,
                )
            });
        let value = name_value
            .as_ref()
            .and_then(|value| ascii_value(value.bytes()));
        #[cfg(test)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_ASSIGNMENT").is_some() {
            eprintln!(
                "ORIGINAL_ASSIGNMENT offset={} target={} stage=store receiver={receiver:?} value={} current={} normal-write={}",
                run.input.segment.span.start(),
                index,
                name_value.is_some(),
                name_value
                    .as_ref()
                    .is_some_and(|value| value.is_current(&state.source_variables)),
                state
                    .source_variables
                    .original_normal_value_write(&receiver, context.registry)
                    .is_some(),
            );
        }
        self.walk_original_opcode_store(
            variable_observers::OriginalOpcodeStore {
                receiver: &receiver,
                reference: name,
                value,
                name_value: name_value.as_ref(),
                offset: run.input.segment.span.start(),
            },
            state,
            context,
        )
    }
}

// The logical compatibility table retains ASCII only. Exact native bytes
// travel separately and never borrow this presentation as a name producer.
fn ascii_value(bytes: &[u8]) -> Option<&str> {
    bytes
        .iter()
        .all(u8::is_ascii)
        .then(|| std::str::from_utf8(bytes).ok())
        .flatten()
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
        prepared.written_name_values.resize(input.words.len(), None);
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
            conversion_operation: None,
            list_operand,
            pool: state.ordinary_literal_pool.clone(),
        }
    }
    fn replace_argument(&mut self, index: usize, operand: &PreparedSourceArguments) {
        Arc::make_mut(&mut self.prepared.written_arguments)[index] =
            operand.written_arguments[0].clone();
        self.prepared.written_values[index].clone_from(&operand.written_values[0]);
        self.prepared.written_name_values[index].clone_from(&operand.written_name_values[0]);
        self.prepared.written_representations[index] = operand.written_representations[0];
        self.prepared.written_objects[index].clone_from(&operand.written_objects[0]);
        self.prepared.written_method_prefixes[index]
            .clone_from(&operand.written_method_prefixes[0]);
        self.prepared.written_variable_reads[index].clone_from(&operand.written_variable_reads[0]);
    }
    fn operand_input(
        &self,
        operand: &Operand,
        state: &ModuleCommandBindings,
        config: tcl_lexer::LexerConfig,
    ) -> Option<crate::signature_scan::scope::SignatureSourceNameInput> {
        use crate::signature_scan::scope::{SignatureSourceNameInput, SignatureSourceNameValue};
        match operand {
            Operand::Original(index) => {
                let value = self.prepared.written_name_values.get(*index)?.as_deref()?;
                value.is_current(&state.source_variables).then(|| {
                    SignatureSourceNameInput::OriginalValue(
                        SignatureSourceNameValue::from_original_produced_value(value),
                    )
                })
            }
            Operand::LiteralExpansion {
                original_word,
                value_span,
                ..
            } => {
                let word = original_name_value::original_native_word(
                    self.input.words.get(*original_word)?,
                    state,
                    config,
                )?;
                let policy = state
                    .source_variables
                    .execution_name_policy?
                    .native_recipe()?;
                let parent = SignatureSourceNameInput::OriginalValue(
                    SignatureSourceNameValue::from_original_static_word(
                        &word,
                        tcl_syntax::word_rules::WordValueRules::from_config(&config),
                        policy,
                    )?,
                );
                let mut children = parent
                    .original_list_elements_with_source_spans()?
                    .into_iter()
                    .filter(|(_, span)| *span == Some(*value_span));
                let child = children.next()?.0;
                children.next().is_none().then_some(child)
            }
        }
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
    use crate::command_binding::{SourceAnalysisOptions, SourceInvocationBinding};
    use tcl_registry::CommandRegistry;
    use tcl_registry::native_compilation::{
        NativeCompilationContext, NativeCompilationFrame, NativeCompilationMode,
    };

    fn analyse(
        source: &str,
        entry: Option<&tcl_runtime_api::NativeCompilationEntry>,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> (SourceCommandBindings, Arc<CommandRegistry>) {
        analyse_in_compilation(
            source,
            entry,
            profile,
            NativeCompilationContext {
                frame: NativeCompilationFrame::ScriptCode,
                mode: NativeCompilationMode::BytecodeObject,
                catch_depth: Some(0),
                loop_depth: 0,
            },
        )
    }

    fn analyse_in_compilation(
        source: &str,
        entry: Option<&tcl_runtime_api::NativeCompilationEntry>,
        profile: &'static tcl_dialect::DialectProfile,
        compilation: NativeCompilationContext,
    ) -> (SourceCommandBindings, Arc<CommandRegistry>) {
        let registry = Arc::new(CommandRegistry::build_default().project_for_profile(profile));
        let analysis = SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            &registry,
            SourceAnalysisOptions {
                native_entry: entry,
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: compilation,
                ..Default::default()
            },
        );
        (analysis, registry)
    }
    fn retained_argument_test_context<'a>(
        binding: &'a SourceInvocationBinding,
        registry: &'a CommandRegistry,
        selected: &'a compiled_invocation::CompiledInvocationSelection,
        config: tcl_lexer::LexerConfig,
    ) -> SourceExecutionContext<'a> {
        SourceExecutionContext {
            realm: tcl_dialect::model::InvocationRealm::RuleLoader,
            compilation: NativeCompilationContext {
                frame: NativeCompilationFrame::ScriptCode,
                mode: NativeCompilationMode::BytecodeObject,
                catch_depth: Some(0),
                loop_depth: 0,
            },
            compilation_snapshot: None,
            selected_compilation: selected.admission.as_ref(),
            original_variable_compilation: None,
            namespace: &binding.lookup_namespace,
            namespace_key: Some(&binding.lookup_namespace_key),
            config,
            registry,
            depth: 0,
            frame: &binding.variable_frame,
            invocation_offset: 0,
            variable_read_owner: None,
            original_written_projection: None,
            written_arguments: None,
            written_values: None,
            written_name_values: None,
            written_representations: None,
            written_objects: None,
            written_method_prefixes: None,
            written_variable_reads: None,
            expression_source: None,
        }
    }

    #[test]
    fn original_argument_context_borrows_its_evaluator_owned_compilation() {
        // naming.source.recursive-driver-state-transport
        // docs/design/analysis/name-resolution-proofs/recursive-driver-state-transport.md
        let source = "lassign {one two} first second";
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let (_owner, entry) = crate::environment_ingress::captured_native_entry_with_owner(profile);
        let (analysis, registry) = analyse(source, Some(&entry), profile);
        let binding = analysis.invocation_at_source("lassign", 0);
        // These are the retained selected receipts, not a reconstructed plan.
        let selected = compiled_invocation::CompiledInvocationSelection {
            admission: binding.native_compilation_admission,
            structured: binding.native_structured_preparation.clone(),
            original_words: binding.original_compiler_words.clone(),
            ..Default::default()
        };
        let state = &binding.lookup_state.as_ref().unwrap().state;
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        let mut context = retained_argument_test_context(&binding, &registry, &selected, config);
        let compilation = original_name_value::OriginalSourceVariableCompilation::from_selected(
            &selected, state, context,
        )
        .expect("actual selected lassign preparation");
        context.original_variable_compilation = Some(&compilation);
        let input = binding.original_retained_written_name_input(1).unwrap();
        let produced = original_name_value::OriginalProducedNameValue::from_source_input(
            &input,
            &state.source_variables,
        )
        .unwrap();
        let name_values = [None, Some(Arc::new(produced)), None, None];
        let arguments = [];
        let holder = source_argument_context_boxed(
            &context,
            SourceArgumentReceipts {
                arguments: &arguments,
                values: &[],
                name_values: &name_values,
                representations: &[],
                objects: &[],
                prefixes: &[],
                reads: &[],
            },
        );
        let view = holder.context();
        let owned = holder.compilation.as_ref().unwrap();
        assert!(std::ptr::eq(
            view.original_variable_compilation.unwrap(),
            owned
        ));
        assert!(std::ptr::eq(
            owned.selection(),
            selected.admission.as_ref().unwrap()
        ));
        assert_eq!(owned.site(), compilation.site());
        assert_eq!(owned.config(), compilation.config());
        assert_eq!(
            owned.evaluated_original_word(1).unwrap().bytes(),
            b"one two"
        );
        assert!(compilation.evaluated_original_word(1).is_none());
        assert!(std::ptr::eq(
            view.written_name_values.unwrap(),
            name_values.as_slice()
        ));
        let truncated = source_argument_context_boxed(
            &context,
            SourceArgumentReceipts {
                arguments: &arguments,
                values: &[],
                name_values: &name_values[..2],
                representations: &[],
                objects: &[],
                prefixes: &[],
                reads: &[],
            },
        );
        assert!(truncated.context().original_variable_compilation.is_none());
        context.config.strict_quoting = !context.config.strict_quoting;
        assert!(
            original_name_value::OriginalSourceVariableCompilation::from_selected(
                &selected, state, context,
            )
            .is_none()
        );
        // The recursive Copy descriptor must stay below the checked argument
        // transport budget without changing depth or stack configuration.
        assert!(std::mem::size_of::<SourceExecutionContext<'_>>() <= 256);
    }

    fn assert_ordered_original_assignment_stores(
        engine: &str,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::native_compilation::NativeCompilationEntry,
    ) {
        for source in [
            "lassign {one two} first [set first]; list $first $one",
            "lassign {one two} first a([set first]); list $first $a(one)",
            "lassign {one two} first [proc lassign args {return CUSTOM}; set first]; list $first $one",
        ] {
            let (analysis, registry) = analyse(source, Some(entry), profile);
            assert_eq!(
                analysis
                    .final_state
                    .source_variables
                    .literal_value("first", &registry),
                Some("one"),
                "{engine}/{source}"
            );
            let last = u32::try_from(source.rfind("list").unwrap()).unwrap();
            let call = analysis.invocation_at_source("list", last);
            assert_eq!(
                call.original_retained_written_name_input(1)
                    .unwrap()
                    .bytes(),
                b"one"
            );
            assert_eq!(
                call.original_retained_written_name_input(2)
                    .unwrap()
                    .bytes(),
                b"two"
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
    }

    #[test]
    fn original_native_assignment_projects_each_store_before_the_next_target() {
        // Implementation contract: naming.variable.original-set-read-completion
        // docs/design/analysis/name-resolution-proofs/original-set-read-completion.md
        for engine in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let profile = tcl_dialect::DialectProfile::find(engine).unwrap();
            let (_owner, entry) =
                crate::environment_ingress::captured_native_entry_with_owner(profile);
            assert_ordered_original_assignment_stores(engine, profile, &entry);
            let source = "lassign {one} first second; list $first $second";
            let (analysis, registry) = analyse(source, Some(&entry), profile);
            assert_eq!(
                analysis
                    .final_state
                    .source_variables
                    .literal_value("second", &registry),
                Some("")
            );
            let call = analysis.invocation_at_source(
                "list",
                u32::try_from(source.rfind("list").unwrap()).unwrap(),
            );
            assert_eq!(
                call.original_retained_written_name_input(1)
                    .unwrap()
                    .bytes(),
                b"one"
            );
            assert_eq!(
                call.original_retained_written_name_input(2)
                    .unwrap()
                    .bytes(),
                b""
            );
            let source = "lassign {one two} n\\uD800 a(k); list [set n\\uD800] $a(k)";
            let (analysis, _) = analyse(source, Some(&entry), profile);
            let call = analysis.invocation_at_source(
                "list",
                u32::try_from(source.rfind("list").unwrap()).unwrap(),
            );
            assert_eq!(
                call.original_retained_written_name_input(1)
                    .unwrap()
                    .bytes(),
                b"one"
            );
            assert_eq!(
                call.original_retained_written_name_input(2)
                    .unwrap()
                    .bytes(),
                b"two"
            );
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
        // No captured entry with an explicit authored BytecodeObject context
        // still selects the source compiler recipe; it is not an unknown mode.
        let (authored, registry) = analyse(source, None, profile);
        assert_eq!(
            authored
                .final_state
                .source_variables
                .literal_value("first", &registry),
            Some("one")
        );
        for (entry, compilation) in [
            (None, NativeCompilationContext::default()),
            (
                Some(&disabled),
                NativeCompilationContext {
                    frame: NativeCompilationFrame::ScriptCode,
                    mode: NativeCompilationMode::BytecodeObject,
                    catch_depth: Some(0),
                    loop_depth: 0,
                },
            ),
        ] {
            let (analysis, registry) = analyse_in_compilation(source, entry, profile, compilation);
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
