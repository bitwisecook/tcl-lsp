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

//! Selecting the operation for one command, with its namespace operands and format strings.

use super::{
    unavailable, ArenaInstruction, Builder, NamespaceBindingInstruction, NamespaceOperand,
    NamespaceOperation, Operation, SwitchOperation, WordInstruction,
};
use std::collections::HashMap;
use tcl_lexer::{ExecutablePart, NativeWord};
use tcl_registry::native_compilation::NativeCompilationSelection;
use tcl_registry::native_compiler_words::{NativeCompiledListRecipe, NativeCompilerWords};
use tcl_registry::native_instruction_plan::{native_instruction_plan, NativeInstructionPlan};

impl Builder<'_> {
    pub(super) fn operation(
        &mut self,
        words: &[NativeWord],
        depth: u32,
    ) -> Result<Option<Operation>, tcl_cmd_core::CmdError> {
        let captured = NativeCompilerWords::capture(words, self.stamp.source_protocol)
            .map_err(|_| unavailable("native body original compiler words"))?;
        if captured
            .shapes()
            .first()
            .and_then(|shape| shape.compiler_head(Some(self.interp.native_invocation_dialect())))
            != Some(true)
        {
            return Ok(Some(Operation::Invoke));
        }
        let Some(head) = captured.literal(0) else {
            return Ok(Some(Operation::Invoke));
        };
        // A release-hidden installed root is absent from the real callable
        // surface. Its retained stock compiler metadata cannot select a hook.
        let generation = self
            .interp
            .resolve_dispatchable_with_generation(self.stamp.namespace, head)
            .and_then(|(_, generation)| generation);
        let Some(generation) = generation else {
            return Ok(Some(Operation::Invoke));
        };
        let dialect = self.interp.native_invocation_dialect();
        let mut arguments_from = 1;
        let mut selected_prerequisite = None;
        let mut selected_plan = None;
        let (identity, spec) = if let Some(compiler) = self
            .interp
            .native_ensemble_compiler_configuration(generation)
        {
            use tcl_registry::native_ensemble::NativeEnsembleWorkerSelection;
            use tcl_runtime_api::native_compilation::NativeCommandCompilerPrerequisite;
            let Some(public) = self
                .interp
                .native_compilation_binding_at(self.stamp.namespace, head)?
            else {
                return Ok(None);
            };
            if public.has_execution_trace {
                return Ok(Some(Operation::Invoke));
            }
            let configuration = compiler
                .ensemble
                .as_ref()
                .expect("actual ensemble configuration");
            let selector =
                tcl_registry::native_ensemble::original_ensemble_selector(&captured, dialect)
                    .map_err(|_| unavailable("native original ensemble selector geometry"))?;
            let selection = tcl_registry::native_ensemble::select_worker_from_original_target(
                configuration,
                selector.as_ref().and_then(|word| word.literal.as_deref()),
                selector.as_ref().map(|word| word.shape),
                Some(dialect),
                |namespace, member, position| {
                    let original = self.interp.native_ensemble_original_target(
                        generation,
                        member.as_bytes(),
                        position,
                    )?;
                    self.interp
                        .native_compilation_binding_from_original(namespace as usize, original)
                        .ok()
                        .flatten()
                },
            );
            let (member, worker) = match selection {
                NativeEnsembleWorkerSelection::Generic => return Ok(Some(Operation::Invoke)),
                NativeEnsembleWorkerSelection::Unknown => return Ok(None),
                NativeEnsembleWorkerSelection::Worker { member, binding } => (member, binding),
            };
            let prerequisite = NativeCommandCompilerPrerequisite {
                interpreter: self.stamp.interpreter,
                lookup_namespace_token: self.stamp.namespace as u64,
                invocation_word: head.into(),
                slot: public.slot,
                namespace_token: public.namespace_token,
                token: public.token,
                implementation_generation: public.implementation_generation,
                compiler,
                selected_worker: Some((*worker).clone()),
                nested_compilers: Vec::new(),
                guard: tcl_runtime_api::CommandBindingGuard::BeforeArguments,
            };
            let registry = crate::environment::store_for_profile(self.interp.dialect_profile());
            let replacements = [member.as_bytes().to_vec()];
            use tcl_registry::native_selected_worker::{
                OriginalSelectedWorkerCompilation, OriginalSelectedWorkerInvocation,
            };
            let selected =
                tcl_registry::native_selected_worker::compile_original_selected_worker_path(
                    registry,
                    &worker,
                    OriginalSelectedWorkerInvocation {
                        words: &captured,
                        operand_from: 2,
                        replacements: &replacements,
                        dialect,
                        context: self.context,
                    },
                    prerequisite,
                    |ensemble, configuration, member, shape| {
                        tcl_registry::native_ensemble::select_worker_from_original_target(
                            configuration,
                            member,
                            shape,
                            Some(dialect),
                            |namespace, member, position| {
                                let original = self.interp.native_ensemble_original_target(
                                    ensemble.token,
                                    member.as_bytes(),
                                    position,
                                )?;
                                self.interp
                                    .native_compilation_binding_from_original(
                                        namespace as usize,
                                        original,
                                    )
                                    .ok()
                                    .flatten()
                            },
                        )
                    },
                );
            let prerequisite = selected.prerequisite;
            let worker = selected.worker;
            match selected.compilation {
                OriginalSelectedWorkerCompilation::PublicGeneric => {
                    return Ok(Some(Operation::Invoke));
                }
                OriginalSelectedWorkerCompilation::Unavailable => return Ok(None),
                OriginalSelectedWorkerCompilation::Named(recipe) => {
                    return self
                        .named_operation(&captured, recipe, Some(&prerequisite), depth)
                        .map(|operation| Some(Operation::NamedInvocation(operation)));
                }
                OriginalSelectedWorkerCompilation::Operation {
                    spec,
                    selection,
                    plan,
                } => {
                    arguments_from = selected.operand_from;
                    selected_prerequisite = Some(prerequisite);
                    selected_plan = Some((selection, *plan));
                    let identity = worker
                        .compiler
                        .as_ref()
                        .ok_or_else(|| unavailable("selected worker compiler"))?
                        .registry_identity
                        .as_bytes()
                        .to_vec();
                    (identity, spec)
                }
            }
        } else {
            let recipe = self
                .interp
                .namespaces
                .borrow()
                .native_compiler_recipe(generation);
            match recipe {
                Some(
                    crate::namespace::NativeCompilerRecipe::Absent
                    | crate::namespace::NativeCompilerRecipe::ProcedureNoOp,
                ) => return Ok(Some(Operation::Invoke)),
                Some(crate::namespace::NativeCompilerRecipe::Registered { registration, spec }) => {
                    let identity = core::str::from_utf8(&registration)
                        .map_err(|_| unavailable("native body compiler registration identity"))?;
                    let registry =
                        crate::environment::store_for_profile(self.interp.dialect_profile());
                    let original = registry
                        .native_compilation_for_registration(identity, dialect)
                        .filter(|original| *original == spec);
                    if original.is_none() {
                        return Ok(None);
                    }
                    let Some(selected) = registry.native_compilation_for_original_registration(
                        identity,
                        &captured,
                        arguments_from,
                        dialect,
                    ) else {
                        return Ok(None);
                    };
                    (registration, selected)
                }
                Some(crate::namespace::NativeCompilerRecipe::Unknown) | None => return Ok(None),
            }
        };
        let dialect = self.interp.native_invocation_dialect();
        std::str::from_utf8(&identity)
            .map_err(|_| unavailable("native body registered compiler identity"))?;
        // Private workers belong to the captured compiler recipe. Callable
        // origin following supplies only that separately checked dependency.
        let target_generation = self
            .interp
            .native_compiler_target(self.stamp.namespace, head)
            .map(|(_, _, generation, _)| generation)
            .unwrap_or(generation);
        if let Some(path) = spec
            .implementation_prerequisites(dialect)
            .filter(|_| selected_prerequisite.is_none())
        {
            if !self
                .interp
                .native_implementation_path_holds(target_generation, &path)
            {
                return Ok(Some(Operation::Invoke));
            }
        }
        let observed = {
            let traces = self.interp.traces.borrow();
            !traces.step_active.is_empty()
                || traces.cmd_traces.iter().any(|trace| {
                    trace.token == Some(generation)
                        && trace.ops & crate::cmd_trace::ops::EXEC_ANY != 0
                })
        };
        if observed {
            return Ok(Some(Operation::Invoke));
        }
        let selection = selected_plan.as_ref().map_or_else(
            || spec.select_native_words(&captured, arguments_from, Some(dialect), self.context),
            |(selection, _)| *selection,
        );
        if let Some(kind) = spec.namespace_binding_kind() {
            let recipe = tcl_registry::native_namespace_binding_compilation::compile_native_namespace_bindings(
                &captured, arguments_from, self.stamp.physical, self.context, kind,
            ).map_err(|_| unavailable("native namespace binding compiler words"))?;
            if recipe.outcome == tcl_registry::native_namespace_binding_compilation::NativeNamespaceBindingOutcome::Unknown {
                return Ok(None);
            }
            return self
                .namespace_operation(&captured, recipe, depth)
                .map(|operation| Some(Operation::NamespaceBindings(operation)));
        }
        if matches!(
            spec.grammar,
            tcl_registry::native_compilation::NativeCompilationGrammar::Conditional
                | tcl_registry::native_compilation::NativeCompilationGrammar::ForLoop
                | tcl_registry::native_compilation::NativeCompilationGrammar::Catch
                | tcl_registry::native_compilation::NativeCompilationGrammar::WhileLoop
        ) {
            let recipe = tcl_registry::native_control_instructions::native_control_instruction(
                spec.grammar,
                &captured,
                arguments_from,
                dialect,
                self.context,
            )
            .map_err(|_| unavailable("native original control compiler"))?;
            if recipe.outcome
                == tcl_registry::native_control_compilation::NativeControlOutcome::Generic
            {
                let _prepared =
                    self.prepare_control_steps(&captured, &recipe.preparations, depth)?;
                self.validate_control_boolean_probes(&captured, &recipe.preparations)?;
                return Ok(Some(Operation::Invoke));
            }
            return self
                .control_operation(&captured, recipe, depth)
                .map(|operation| {
                    Some(if operation.recipe.outcome
                    == tcl_registry::native_control_compilation::NativeControlOutcome::Generic {
                    Operation::Invoke
                } else { Operation::Control(Box::new(operation)) })
                });
        }
        if spec.grammar == tcl_registry::native_compilation::NativeCompilationGrammar::Try {
            let recipe = tcl_registry::native_try_compilation::compile_native_try(
                &captured,
                arguments_from,
                self.stamp.physical,
                self.context,
            )
            .map_err(|_| unavailable("native original try compiler"))?;
            return self.try_operation(&captured, recipe, depth).map(Some);
        }
        if spec.grammar == tcl_registry::native_compilation::NativeCompilationGrammar::Foreach {
            let recipe = tcl_registry::native_each_compilation::compile_native_each(
                &captured,
                arguments_from,
                self.stamp.physical,
                self.context,
                spec.each_collection()
                    .ok_or_else(|| unavailable("native iterator collection policy"))?,
            )
            .map_err(|_| unavailable("native original iterator compiler"))?;
            return self.each_operation(&captured, recipe, depth).map(Some);
        }
        if let tcl_registry::native_compilation::NativeCompilationGrammar::Array {
            command, ..
        } = spec.grammar
        {
            if selection == NativeCompilationSelection::Generic {
                let recipe = tcl_registry::native_array_compilation::native_array_compilation(
                    &captured,
                    arguments_from,
                    command,
                    self.stamp.physical,
                    self.context,
                )
                .ok_or_else(|| unavailable("original declined Array preparation"))?;
                return self.array_operation(&captured, recipe, depth).map(Some);
            }
        }
        if selection == NativeCompilationSelection::Generic {
            if let Some(preparations) =
                tcl_registry::native_instruction_plan::original_dictionary_preparations(
                    spec,
                    &captured,
                    arguments_from,
                    self.stamp.physical,
                    self.context,
                )
            {
                let preparations = preparations
                    .map_err(|_| unavailable("native dictionary declined original preparation"))?;
                self.prepare_control_steps(&captured, &preparations, depth)?;
            }
            return Ok(Some(Operation::Invoke));
        }
        if !matches!(
            selection,
            NativeCompilationSelection::Inline { .. }
                | NativeCompilationSelection::NamedInvocation { .. }
        ) {
            return Ok(None);
        }
        let plan = match selected_plan {
            Some((_, plan)) => plan,
            None => match native_instruction_plan(
                spec,
                selection,
                &captured,
                arguments_from,
                dialect,
                self.context,
            ) {
                Ok(plan) => plan,
                Err(_) => return Ok(None),
            },
        };
        Ok(Some(match plan {
            NativeInstructionPlan::NamespaceString(recipe) => Operation::NamespaceString(
                self.namespace_string_operation(&captured, recipe, depth)?,
            ),
            NativeInstructionPlan::Upvar(recipe) => {
                Operation::Upvar(self.upvar_operation(&captured, recipe, depth)?)
            }
            NativeInstructionPlan::InfoExists(recipe) => {
                Operation::InfoExists(self.info_exists_operation(&captured, recipe, depth)?)
            }
            NativeInstructionPlan::ListOperations(
                tcl_registry::native_list_operations_compilation::NativeListOperationInstruction::Set {
                    ..
                },
            ) => return Ok(None),
            NativeInstructionPlan::ListOperations(recipe) => {
                Operation::ListOperations(self.list_operation(&captured, recipe, depth)?)
            }
            NativeInstructionPlan::ListIndex(recipe) => {
                Operation::ListIndex(self.list_index_operation(&captured, recipe, depth)?)
            }
            NativeInstructionPlan::Array(recipe) => {
                self.array_operation(&captured, recipe, depth)?
            }
            NativeInstructionPlan::Introspection(recipe) => {
                Operation::Introspection(self.introspection_operation(&captured, recipe, depth)?)
            }
            NativeInstructionPlan::MathOperator(recipe) => {
                Operation::MathOperator(self.mathop_operation(&captured, recipe, depth)?)
            }
            NativeInstructionPlan::Scalar(recipe) => {
                Operation::Scalar(self.scalar_operation(&captured, recipe, depth)?)
            }
            NativeInstructionPlan::StringTrim(recipe) => {
                Operation::StringTrim(self.string_trim_operation(&captured, recipe, depth)?)
            }
            NativeInstructionPlan::StringRange(recipe) => {
                if matches!(
                    recipe.operation,
                    tcl_registry::native_string_range_compilation::NativeStringRangeOperation::Indices { .. }
                ) {
                    return Ok(None);
                }
                Operation::StringRange(self.string_range_operation(&captured, &recipe, depth)?)
            }
            NativeInstructionPlan::StringOperand(recipe) => {
                Operation::StringOperand(self.string_operand_operation(&captured, recipe, depth)?)
            }
            NativeInstructionPlan::StringMatch(recipe) => {
                Operation::StringMatch(self.string_match_operation(&captured, recipe, depth)?)
            }
            NativeInstructionPlan::Coroutine(recipe) => {
                Operation::Coroutine(self.coroutine_operation(&captured, recipe, depth)?)
            }
            NativeInstructionPlan::Error(recipe) => {
                // Throw's type-check stage has no runtime body step; generic
                // dispatch runs it.
                if recipe.steps.iter().any(|step| {
                    matches!(
                        step,
                        tcl_registry::native_error_compilation::NativeErrorStep::Pop
                            | tcl_registry::native_error_compilation::NativeErrorStep::ThrowTypeCheck
                    )
                }) {
                    return Ok(None);
                }
                Operation::Error(self.error_operation(&captured, recipe, depth)?)
            }
            NativeInstructionPlan::DictionaryScope(recipe) => {
                self.dictionary_scope_operation(&captured, recipe, depth)?
            }
            NativeInstructionPlan::DictionaryMutation(recipe) => Operation::DictionaryMutation(
                self.dictionary_mutation_operation(&captured, recipe, depth)?,
            ),
            NativeInstructionPlan::GenericPreparation(steps) => {
                self.prepare_control_steps(&captured, &steps, depth)?;
                Operation::Invoke
            }
            NativeInstructionPlan::DictionaryLookup(recipe) => Operation::DictionaryLookup(
                self.dictionary_lookup_operation(&captured, recipe, depth)?,
            ),
            NativeInstructionPlan::NamedInvocation(recipe) => Operation::NamedInvocation(
                self.named_operation(&captured, recipe, selected_prerequisite.as_ref(), depth)?,
            ),
            NativeInstructionPlan::Expression(recipe) => Operation::Expression(Box::new(
                self.expression_operation(&captured, recipe, depth)?,
            )),
            NativeInstructionPlan::Break => Operation::Break,
            NativeInstructionPlan::Continue => Operation::Continue,
            NativeInstructionPlan::Try(_) => {
                return Err((unavailable("native try preparation entry")).into());
            }
            NativeInstructionPlan::Control(_) => {
                return Err((unavailable("native control preparation entry")).into());
            }
            NativeInstructionPlan::Each(_) => {
                return Err((unavailable("native iterator preparation entry")).into());
            }
            NativeInstructionPlan::NamespaceBindings(_) => {
                return Err((unavailable("native namespace binding preparation entry")).into());
            }
            NativeInstructionPlan::TclOoHelper(recipe) => {
                use tcl_registry::native_tcloo_compilation::NativeTclOoInstruction as Helper;
                let operands = match &recipe {
                    Helper::Next { words, .. } => words.as_slice(),
                    Helper::ObjectInfo { operand, .. } => std::slice::from_ref(operand),
                    Helper::SelfObject | Helper::SelfNamespace => &[],
                };
                let literals = operands.iter().map(|word|match word {
                        tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand::LiteralExpansion{value,..}=>Some(self.literals.intern_bytes(value)),
                        _=>None,
                    }).collect();
                Operation::TclOoHelper(recipe, literals)
            }
            NativeInstructionPlan::Switch(recipe) => Operation::Switch(SwitchOperation {
                recipe,
                patterns: Vec::new(),
                subject_literal: None,
                empty: None,
            }),
            NativeInstructionPlan::Unset(recipe) => {
                Operation::Unset(self.unset_operation(&captured, recipe, depth)?)
            }
            NativeInstructionPlan::Load { target, .. } => {
                Operation::Load(self.target(target, depth)?)
            }
            NativeInstructionPlan::Store {
                target, value_word, ..
            } => Operation::Store(self.target(target, depth)?, value_word),
            NativeInstructionPlan::Increment {
                target,
                amount_word,
                immediate,
                ..
            } => Operation::Increment(self.target(target, depth)?, amount_word, immediate),
            NativeInstructionPlan::Append { target, recipe, .. } => {
                Operation::Append(self.target(target, depth)?, recipe)
            }
            NativeInstructionPlan::List(recipe) => {
                let index = match &recipe {
                    NativeCompiledListRecipe::EmptyString => Some(self.literals.intern_bytes(b"")),
                    NativeCompiledListRecipe::PrivateConstant { members } => Some(
                        self.literals
                            .register_private_constant_list(members, self.stamp.source_protocol),
                    ),
                    NativeCompiledListRecipe::DynamicElements { .. } => None,
                };
                Operation::List(recipe, index)
            }
            NativeInstructionPlan::FormatStringConcat(recipe) => {
                self.format_string_operation(&captured, recipe, depth)?
            }
            NativeInstructionPlan::Concat(recipe) => {
                use tcl_registry::native_instruction_plan::{
                    NativeConcatInstruction, NativeConcatLiteralAllocation,
                };
                match recipe {
                    NativeConcatInstruction::Literal { bytes, allocation } => {
                        let index = match allocation {
                            NativeConcatLiteralAllocation::Registered => {
                                self.literals.intern_bytes(&bytes)
                            }
                            NativeConcatLiteralAllocation::PrivateEmpty => {
                                self.literals.register_unshared(&bytes)
                            }
                            NativeConcatLiteralAllocation::PrivateString => {
                                self.literals.register_private_concat_string(&bytes)
                            }
                        };
                        Operation::Concat {
                            literal: Some(index),
                            operands: Vec::new(),
                            prepared_words: HashMap::new(),
                        }
                    }
                    NativeConcatInstruction::Operands(operands) => {
                        let mut prepared_words = HashMap::new();
                        let operands = operands
                            .iter()
                            .map(|operand| {
                                self.namespace_operand(
                                    &captured,
                                    operand,
                                    &mut prepared_words,
                                    depth,
                                )
                            })
                            .collect::<Result<Vec<_>, _>>()?;
                        Operation::Concat {
                            literal: None,
                            operands,
                            prepared_words,
                        }
                    }
                }
            }
            NativeInstructionPlan::ReturnOptions(recipe) => {
                Operation::SelectedReturn(recipe, None, None)
            }
            NativeInstructionPlan::Uplevel(recipe) => {
                let default = recipe
                    .level_word
                    .is_none()
                    .then(|| self.literals.intern_bytes(b"1"));
                Operation::Uplevel(recipe, default)
            }
            // These plans decline a native body artifact, as an unavailable plan does.
            NativeInstructionPlan::Regexp(_)
            | NativeInstructionPlan::Substitution(_)
            | NativeInstructionPlan::ClockRead(_)
            | NativeInstructionPlan::StringMap(_)
            | NativeInstructionPlan::StringReplace(_)
            | NativeInstructionPlan::StringInsert(_)
            | NativeInstructionPlan::ListReplace(_)
            | NativeInstructionPlan::DictionaryEach(_)
            | NativeInstructionPlan::SyntaxError { .. }
            | NativeInstructionPlan::Constant { .. }
            | NativeInstructionPlan::DictionaryCreate(_)
            | NativeInstructionPlan::StringIs(_) => return Ok(None),
        }))
    }

    pub(super) fn namespace_word(
        &mut self,
        captured: &NativeCompilerWords<'_>,
        index: usize,
        command_head: bool,
        depth: u32,
    ) -> Result<WordInstruction, tcl_cmd_core::CmdError> {
        let original = captured
            .original_words()
            .get(index)
            .ok_or_else(|| unavailable("native namespace original word index"))?;
        let literal = if command_head {
            self.command_literal(captured)?
        } else {
            captured
                .literal(index)
                .map(|bytes| self.literals.intern_bytes(bytes))
        };
        let arena = if literal.is_none() {
            self.arena(original.executable_parts(), depth)?
        } else {
            ArenaInstruction {
                original: original.executable_parts().clone(),
                texts: HashMap::new(),
                locals: HashMap::new(),
                roots: HashMap::new(),
                syntax: HashMap::new(),
            }
        };
        Ok(WordInstruction {
            original: original.clone(),
            literal,
            arena,
        })
    }

    fn format_string_operation(
        &mut self,
        captured: &NativeCompilerWords<'_>,
        recipe: tcl_registry::native_instruction_plan::NativeFormatStringInstruction,
        depth: u32,
    ) -> Result<Operation, tcl_cmd_core::CmdError> {
        use tcl_registry::native_instruction_plan::{
            NativeFormatLiteralAllocation as Allocation, NativeFormatStringPiece as Piece,
        };
        let mut prepared_words = HashMap::new();
        let mut operands = Vec::new();
        for piece in recipe.pieces {
            operands.push(match piece {
                Piece::Literal(bytes) => NamespaceOperand::Literal(match recipe.literal_allocation {
                    Allocation::Registered => self.literals.intern_bytes(&bytes),
                    Allocation::PrivateString => self.literals.register_private_concat_string(&bytes),
                }),
                Piece::Value(word) => self.namespace_operand(captured, &tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand::Original(word), &mut prepared_words, depth)?,
            });
        }
        Ok(Operation::FormatStringConcat {
            operands,
            prepared_words,
        })
    }

    pub(super) fn namespace_operand(
        &mut self,
        captured: &NativeCompilerWords<'_>,
        operand: &tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand,
        prepared_words: &mut HashMap<usize, WordInstruction>,
        depth: u32,
    ) -> Result<NamespaceOperand, tcl_cmd_core::CmdError> {
        use tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand;
        match operand {
            NativeCompilerWordOperand::Original(index) => {
                if !prepared_words.contains_key(index) {
                    let word = self.namespace_word(captured, *index, *index == 0, depth)?;
                    prepared_words.insert(*index, word);
                }
                Ok(NamespaceOperand::Original(*index))
            }
            NativeCompilerWordOperand::LiteralExpansion { value, .. } => {
                Ok(NamespaceOperand::Literal(self.literals.intern_bytes(value)))
            }
        }
    }

    /// Withdraw executable child publication from a declined compiler attempt.
    /// Its literal allocations and local declarations remain in this builder.
    fn rollback_word_publication(&mut self, original: &NativeWord) {
        for component in original.executable_parts().all_parts() {
            if let ExecutablePart::Command { body } = component.part {
                self.scripts
                    .retain(|region, _| region.start() < body.start() || region.end() > body.end());
            }
        }
    }

    fn namespace_operation(
        &mut self,
        captured: &NativeCompilerWords<'_>,
        recipe: tcl_registry::native_namespace_binding_compilation::NativeNamespaceBindingCompilation,
        depth: u32,
    ) -> Result<NamespaceOperation, tcl_cmd_core::CmdError> {
        use tcl_registry::native_namespace_binding_compilation::{
            NativeNamespaceBindingOutcome, NativeNamespaceBindingVisit,
        };
        let mut prepared_words = HashMap::new();
        let mut locals = HashMap::new();
        let mut operands = HashMap::new();
        let mut prefix = None;
        for visit in &recipe.visits {
            match visit {
                NativeNamespaceBindingVisit::Literal(value) => {
                    prefix = Some(self.literals.intern_bytes(value));
                }
                NativeNamespaceBindingVisit::DeclareLocal(name) => {
                    let protocol = self
                        .interp
                        .native_invocation_dialect()
                        .native_compiled_variable_protocol()
                        .ok_or_else(|| {
                            unavailable("native namespace local declaration protocol")
                        })?;
                    locals.insert(name.clone(), self.lvt.intern_native(protocol, name));
                }
                NativeNamespaceBindingVisit::Word(operand) => {
                    let value =
                        self.namespace_operand(captured, operand, &mut prepared_words, depth)?;
                    operands.insert(operand.clone(), value);
                }
            }
        }
        let mut bindings = Vec::new();
        let generic = if recipe.outcome == NativeNamespaceBindingOutcome::Generic {
            let projected =
                tcl_registry::native_compiler_word_projection::project_native_compiler_words(
                    captured,
                    self.stamp.physical,
                )
                .map_err(|_| unavailable("native namespace fallback parser words"))?;
            let mut generic = Vec::with_capacity(projected.len());
            for word in projected {
                if let tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand::Original(index) = &word.operand {
                    if prepared_words.contains_key(index) {
                        self.rollback_word_publication(&captured.original_words()[*index]);
                        let prepared = self.namespace_word(captured, *index, *index == 0, depth)?;
                        prepared_words.insert(*index, prepared);
                    }
                }
                generic.push(self.namespace_operand(
                    captured,
                    &word.operand,
                    &mut prepared_words,
                    depth,
                )?);
            }
            Some(generic)
        } else {
            for binding in &recipe.bindings {
                bindings.push(NamespaceBindingInstruction {
                    name: operands
                        .remove(&binding.name)
                        .ok_or_else(|| unavailable("native namespace name preparation"))?,
                    slot: *locals
                        .get(&binding.local)
                        .ok_or_else(|| unavailable("native namespace local preparation"))?,
                    local: binding.local.clone(),
                    value: binding
                        .value
                        .as_ref()
                        .map(|value| {
                            operands
                                .remove(value)
                                .ok_or_else(|| unavailable("native namespace value preparation"))
                        })
                        .transpose()?,
                });
            }
            None
        };
        let namespace = if generic.is_none() {
            recipe
                .namespace
                .as_ref()
                .map(|operand| {
                    operands
                        .remove(operand)
                        .ok_or_else(|| unavailable("native original namespace operand preparation"))
                })
                .transpose()?
        } else {
            None
        };
        let empty = generic.is_none().then(|| self.literals.intern_bytes(b""));
        Ok(NamespaceOperation {
            kind: recipe.kind,
            bindings,
            generic,
            prepared_words,
            prefix,
            namespace,
            empty,
        })
    }
}
