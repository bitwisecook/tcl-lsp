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

//! Conditional transfer queries over one unchanged original Logical source world.

use crate::command_binding::OriginalInvocationSourcePurpose;
use crate::ir::{CommandTokens, Module, Procedure, Script, Statement};
use crate::registry_invocation::{
    EffectiveCommandWords, InvocationMetadataContext, ResolvedStatementInvocation,
};
use tcl_lexer::LexerConfig;
use tcl_registry::CommandRegistry;

mod native;
pub(super) use native::NativeSummaryContext;

/// A selected source call and its same-owner effective operands. Neither the
/// declaration nor this conditional join establishes Native Normal completion.
pub(crate) struct ProcedureCall<'a> {
    pub(crate) id: crate::ir::ProcedureId,
    pub(crate) procedure: &'a Procedure,
    pub(crate) arguments: EffectiveCommandWords,
}

impl ProcedureCall<'_> {
    pub(crate) fn literal_arguments(&self, module: &Module) -> Option<Vec<Option<String>>> {
        if self.arguments.words.iter().any(|word| {
            matches!(
                word,
                crate::ir::WordExpr::Expand { .. } | crate::ir::WordExpr::Opaque { .. }
            )
        }) {
            return None;
        }
        Some(
            (0..self.arguments.words.len().checked_sub(1)?)
                .map(|index| {
                    self.arguments.argument_literal(
                        index,
                        module.native_lexer_config().escapes,
                        module.word_values(),
                    )
                })
                .collect(),
        )
    }
}

/// Original child words retained by the same Module and parent invocation.
/// This source inventory grants no Native entry or completion authority.
#[derive(Clone)]
pub(crate) struct OriginalSummaryScript {
    text: String,
    base: u32,
    commands: Vec<CommandTokens>,
}

impl OriginalSummaryScript {
    pub(crate) fn text(&self) -> &str {
        &self.text
    }
    pub(crate) const fn base(&self) -> u32 {
        self.base
    }
    pub(crate) fn command_at(&self, offset: u32) -> Option<&CommandTokens> {
        let mut found = self.commands.iter().filter(|tokens| {
            tokens
                .source_binding
                .as_ref()
                .and_then(|binding| binding.invocation_site())
                .is_some_and(|site| site.offset == offset)
        });
        let first = found.next()?;
        found.all(|next| next == first).then_some(first)
    }
}

/// A complete selected original expression and its existing parent point.
/// It grants only child source correspondence, never Native completion.
#[derive(Clone)]
pub(crate) struct OriginalSummaryExpression {
    parent: CommandTokens,
    base: u32,
}

/// Whether `body` runs any of its commands in another frame: an `uplevel`
/// body, or a block evaluated in another namespace.
fn runs_another_frame(
    body: &Script,
    namespace: &crate::command_binding::SourceNamespaceKey,
) -> bool {
    let Some(namespace) = namespace.display() else {
        return true;
    };
    let mut pending = vec![body];
    while let Some(script) = pending.pop() {
        for statement in &script.statements {
            match statement {
                Statement::UpFrame { .. } => return true,
                Statement::Block {
                    namespace: inner, ..
                } if *inner != namespace => return true,
                _ => pending.extend(crate::ir_helpers::nested_bodies(statement)),
            }
        }
    }
    false
}

/// The frame of a release unit's procedure body and the namespace it runs in.
struct AuthoredFrame {
    frame: crate::command_binding::SourceOriginalVariableFrame,
    namespace: crate::signature_scan::scope::SignatureNamespaceScope,
}

pub(super) struct SourceSummaryContext<'a> {
    module: &'a Module,
    registry: &'a CommandRegistry,
    config: LexerConfig,
    realm: crate::realm::CommandBindingRealm,
    /// The source model the module was read under: the Logical survey, or a
    /// release unit's authored simulation. Every original query answers only
    /// under this purpose.
    purpose: OriginalInvocationSourcePurpose,
}

impl<'a> SourceSummaryContext<'a> {
    pub(super) fn for_module(
        module: &'a Module,
        registry: &'a CommandRegistry,
        config: LexerConfig,
    ) -> Option<Self> {
        if config.normalized() != module.lexer_config.normalized()
            || module.has_dynamic_trace
            || !module.traced_commands.is_empty()
            || !module
                .retained_source_bindings
                .as_ref()?
                .matches_module(module, registry)
        {
            return None;
        }
        let purpose = module.original_invocation_source_purpose(
            registry,
            InvocationMetadataContext::for_module(registry, module)?,
        )?;
        let bindings = module
            .retained_source_bindings
            .as_deref()?
            .positioned_for_module(module, registry)?;
        let realm = match purpose {
            OriginalInvocationSourcePurpose::Logical => {
                crate::realm::original_logical_realm_from_source_bindings(bindings)?
            }
            OriginalInvocationSourcePurpose::AuthoredSimulation => {
                crate::realm::original_authored_realm_from_source_bindings(
                    bindings,
                    module.source_metadata_input.as_ref()?,
                )?
            }
            _ => return None,
        };
        Some(Self {
            module,
            registry,
            config,
            realm,
            purpose,
        })
    }

    /// Whether the procedure's body was walked in its own declared frame, the
    /// frame a summary describes: under the Logical survey always; for a
    /// release unit, only where every command of the body carries the
    /// declaration's frame, not a caller's activation or a join of several.
    pub(super) fn walked_in_declared_frame(&self, id: &crate::ir::ProcedureId) -> bool {
        if self.logical() {
            return true;
        }
        let declared = || {
            let procedure = self.procedure(id)?;
            let frame =
                crate::command_binding::SourceOriginalVariableFrame::from_authored_procedure(
                    procedure,
                    self.module.source_metadata_input.as_ref()?,
                )?;
            let commands = self.same_frame_commands(self.declared_frame_procedure(id)?)?;
            commands
                .iter()
                .all(|tokens| {
                    tokens
                        .source_binding
                        .as_ref()
                        .is_some_and(|binding| &binding.variable_frame == frame.frame())
                })
                .then_some(())
        };
        declared().is_some()
    }

    /// Whether the procedure SSA defines a local at a write through its
    /// caller link, so the summary run can read the linked place's outcome:
    /// the Logical survey's frames do; a release unit's walk stores through
    /// the link to its target instead.
    pub(super) fn reads_linked_locals(&self) -> bool {
        self.logical()
    }

    /// Whether the module was read under the Logical survey, whose
    /// navigation and variable scopes the operand queries read.
    fn logical(&self) -> bool {
        self.purpose == OriginalInvocationSourcePurpose::Logical
    }

    pub(super) const fn registry(&self) -> &'a CommandRegistry {
        self.registry
    }

    pub(super) fn procedure(&self, id: &crate::ir::ProcedureId) -> Option<&'a Procedure> {
        self.module.procedure_ids.owns(*id).then_some(())?;
        let procedure = self.module.procedures.get(id)?;
        (self
            .module
            .retained_source_bindings
            .as_ref()?
            .matches_original_procedure_summary(*id, procedure))
        .then_some(procedure)
    }

    /// The procedure as its own declared frame runs it: its body lowered
    /// again in that frame where the walk also entered it from a caller,
    /// else the module's. Its declaration is the module's procedure.
    pub(super) fn declared_frame_procedure(
        &self,
        id: &crate::ir::ProcedureId,
    ) -> Option<&'a Procedure> {
        let procedure = self.procedure(id)?;
        Some(
            self.module
                .declared_frame_bodies
                .get(id)
                .unwrap_or(procedure),
        )
    }

    pub(super) fn metadata<'w>(
        &'w self,
        tokens: &'w CommandTokens,
    ) -> Option<InvocationMetadataContext<'w>> {
        if self.logical() {
            return self.variable_operands().metadata(tokens);
        }
        let binding = tokens.source_binding.as_ref()?;
        let metadata =
            binding.original_invocation_metadata_for_module(tokens, self.module, self.registry)?;
        (binding.original_invocation_source_purpose(tokens, metadata, self.registry)?
            == self.purpose)
            .then_some(metadata)
    }

    pub(super) fn operation(&self, tokens: &CommandTokens) -> Option<ResolvedStatementInvocation> {
        if self.logical() {
            return self.variable_operands().operation(tokens);
        }
        crate::registry_invocation::original_source_model_operation_invocation_with_metadata_context(
            self.registry,
            self.metadata(tokens)?,
            tokens,
            self.purpose,
        )
    }

    /// The closed state transitions of the selected operation: the
    /// registry's declaration, or, for a release unit's authored simulation
    /// whose descriptor declares none, the authored source view's.
    pub(super) fn transitions(
        &self,
        tokens: &CommandTokens,
    ) -> Option<tcl_registry::StateTransitions> {
        let operation = self.operation(tokens)?;
        if let Some(declared) = operation.facts.state_transitions.declared() {
            return Some(declared.clone());
        }
        (!self.logical()).then_some(())?;
        self.with_operation(tokens, |operation, schema| {
            Some(
                schema
                    .authored_source_effects_and_state_transitions(operation.dialect?)
                    .1,
            )
        })
    }

    pub(super) fn with_operation<T>(
        &self,
        tokens: &CommandTokens,
        apply: impl FnOnce(
            &ResolvedStatementInvocation,
            &tcl_registry::ResolvedInvocation<'_, '_>,
        ) -> Option<T>,
    ) -> Option<T> {
        let operation = self.operation(tokens)?;
        let realm = tokens.source_binding.as_ref()?.invocation_realm()?;
        operation.with_metadata_schema(self.registry, self.metadata(tokens)?, realm, |schema| {
            apply(&operation, schema)
        })
    }

    pub(super) fn procedure_call(&self, tokens: &CommandTokens) -> Option<ProcedureCall<'a>> {
        self.metadata(tokens)?;
        let calls = if self.logical() {
            crate::registry_invocation::original_logical_procedure_calls_for_module(
                tokens,
                self.module,
                self.registry,
            )?
        } else {
            crate::registry_invocation::original_authored_procedure_calls_for_module(
                tokens,
                self.module,
                self.registry,
            )?
        };
        let mut selected: Option<ProcedureCall<'a>> = None;
        for call in calls {
            let procedure = self.procedure(&call.id())?;
            let next = ProcedureCall {
                id: call.id(),
                procedure,
                arguments: call.effective_arguments().clone(),
            };
            if let Some(previous) = &selected {
                if !std::ptr::eq(previous.procedure, next.procedure)
                    || previous.arguments != next.arguments
                {
                    return None;
                }
            } else {
                selected = Some(next);
            }
        }
        selected
    }

    fn variable_operands(
        &self,
    ) -> crate::registry_invocation::source_structure::OriginalLogicalVariableOperands<'_> {
        crate::registry_invocation::source_structure::OriginalLogicalVariableOperands::borrowing(
            self.module,
            self.registry,
            &self.realm,
        )
    }

    fn original_argument_word(
        &self,
        tokens: &CommandTokens,
        written: usize,
    ) -> Option<tcl_lexer::NativeWord> {
        if self.logical() {
            return self
                .variable_operands()
                .original_argument_word(tokens, written);
        }
        let binding = tokens.source_binding.as_ref()?;
        let site = binding.invocation_site()?;
        let config = binding.original_lexer_config_for_tokens(tokens)?;
        let original = site
            .source
            .original_lexical_command(tokens.words(), site.offset, config)?;
        let index = written.checked_add(1)?;
        let word = original.native_words().get(index)?;
        (word.span() == tokens.words().get(index)?.source().span).then(|| word.clone())
    }

    /// The release procedure whose own frame runs `tokens`, and that frame:
    /// a command of the body, of a control body it runs in place, or of a
    /// substitution in either. An `uplevel` body and a `namespace eval`
    /// block run in another frame and own none.
    fn authored_frame(&self, tokens: &CommandTokens) -> Option<AuthoredFrame> {
        let mut owner = None;
        for id in self.module.procedures.keys() {
            let (Some(procedure), Some(body)) =
                (self.procedure(id), self.declared_frame_procedure(id))
            else {
                continue;
            };
            if self
                .same_frame_commands(body)?
                .iter()
                .any(|command| command == tokens)
            {
                if owner.is_some() {
                    return None;
                }
                owner = Some(procedure);
            }
        }
        let procedure = owner?;
        let namespace = procedure.body.namespace_context.as_deref()?;
        Some(AuthoredFrame {
            frame: crate::command_binding::SourceOriginalVariableFrame::from_authored_procedure(
                procedure,
                self.module.source_metadata_input.as_ref()?,
            )?,
            namespace: crate::signature_scan::scope::SignatureNamespaceScope::C(
                namespace.source_artifact_path()?,
            ),
        })
    }

    fn same_frame_commands(&self, procedure: &Procedure) -> Option<Vec<CommandTokens>> {
        let namespace = procedure.body.namespace_context.as_deref()?.display()?;
        let mut commands = Vec::new();
        let mut pending = vec![&procedure.body];
        while let Some(script) = pending.pop() {
            for statement in &script.statements {
                if statement.synthetic_marker().is_some() {
                    continue;
                }
                let tokens = script.retained_source_tokens_for_statement(statement)?;
                self.metadata(tokens)?;
                commands.push(tokens.clone());
                commands.extend(self.children(tokens)?);
                match statement {
                    Statement::UpFrame { .. } => {}
                    Statement::Block {
                        namespace: inner, ..
                    } if *inner != namespace => {}
                    _ => pending.extend(crate::ir_helpers::nested_bodies(statement)),
                }
            }
        }
        Some(commands)
    }

    /// A release unit's argument: its original word's span and name, read
    /// under the unit's authored name policy, and the frame that runs it.
    fn authored_argument(
        &self,
        tokens: &CommandTokens,
        written: usize,
    ) -> Option<(
        tcl_lexer::Span,
        crate::signature_scan::scope::SignatureSourceNameInput,
        AuthoredFrame,
    )> {
        let frame = self.authored_frame(tokens)?;
        let word = self.original_argument_word(tokens, written)?;
        let input = self.module.source_metadata_input.as_ref()?;
        let policy = tcl_registry::InvocationDialect::of_profile(input.unit_profile())
            .authored_name_policy()?;
        let purpose =
            crate::signature_scan::original_name::OriginalSourceNamePurpose::for_analysis_input(
                input, None, policy,
            );
        let name =
            crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(purpose.key(
                &word,
                tcl_syntax::word_rules::WordValueRules::from_config(&word.config()),
            )?);
        Some((word.span(), name, frame))
    }

    /// The outer places a release procedure body writes, as the Logical
    /// inventory (`global_write_info::logical_body_source_writes`) states
    /// them: each alias the body declares onto a global or namespace
    /// variable, and each written argument naming one directly or through
    /// such an alias. A module with traces, and a body that runs another
    /// frame (an `uplevel` body, a `namespace eval` block), have none.
    pub(super) fn authored_outer_writes(
        &self,
        id: &crate::ir::ProcedureId,
    ) -> Option<Vec<crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence>>
    {
        use crate::cfg_builder::global_write_info::{
            insert_source_write, logical_alias_targets_global,
        };
        use crate::signature_scan::variable_symbol::SignatureSourceVariableSlot;
        (!self.logical()).then_some(())?;
        let module = self.module;
        if module.has_dynamic_trace
            || !module.traced_commands.is_empty()
            || module.has_dynamic_variable_trace
            || !module.traced_variables.is_empty()
        {
            return None;
        }
        let procedure = self.declared_frame_procedure(id)?;
        if runs_another_frame(
            &procedure.body,
            procedure.body.namespace_context.as_deref()?,
        ) {
            return None;
        }
        let mut footprints = Vec::new();
        for tokens in self.same_frame_commands(procedure)? {
            if self.procedure_call(&tokens).is_some() {
                continue;
            }
            let footprint = self.authored_footprint(&tokens)?;
            footprints.push((tokens, footprint));
        }
        let mut aliases: Vec<(
            tcl_core_types::NameBytes,
            Vec<crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence>,
        )> = Vec::new();
        let mut writes = Vec::new();
        for (tokens, (_, declared)) in &footprints {
            let dialect = self.operation(tokens)?.dialect;
            for alias in declared {
                let local = self.alias_local(tokens, alias)?;
                if !logical_alias_targets_global(alias, dialect)? {
                    continue;
                }
                let target = self.alias_source_place(tokens, alias)?;
                let index = if let Some(index) = aliases.iter().position(|(held, _)| held == &local)
                {
                    index
                } else {
                    aliases.push((local, Vec::new()));
                    aliases.len() - 1
                };
                insert_source_write(&mut aliases[index].1, target.clone());
                if alias.writes_value {
                    insert_source_write(&mut writes, target);
                }
            }
        }
        for (tokens, (projection, _)) in &footprints {
            if projection.opaque_variable_frame {
                return None;
            }
            for (written, form) in self.authored_write_arguments(tokens)? {
                let place = self.argument_source_place(tokens, written, form)?;
                match place.symbol().slot() {
                    SignatureSourceVariableSlot::Local { simple, .. } => {
                        if let Some((_, targets)) = aliases.iter().find(|(held, _)| held == simple)
                        {
                            for target in targets {
                                insert_source_write(&mut writes, target.clone());
                            }
                        }
                    }
                    SignatureSourceVariableSlot::C { .. } | SignatureSourceVariableSlot::Jim(_) => {
                        insert_source_write(&mut writes, place);
                    }
                }
            }
        }
        Some(writes)
    }

    /// The selected source schema's variable-write projection and declared
    /// aliases for a release unit's command.
    fn authored_footprint(
        &self,
        tokens: &CommandTokens,
    ) -> Option<(
        tcl_registry::VariableWriteProjection,
        Vec<tcl_registry::VariableCellAliasTransition>,
    )> {
        let input = self.module.source_metadata_input.as_ref()?;
        self.authored_registry_words(tokens)?.with_source_schema(
            input.borrowed_context_registry(),
            |schema| {
                Some((
                    tcl_registry::CommandRegistry::variable_write_projection_for_selected_source(
                        schema,
                    ),
                    schema.authored_logical_source_variable_aliases()?,
                ))
            },
        )?
    }

    /// The written arguments a release unit's command stores into, with the
    /// receiver form of each; an alias's local is the alias's, not a write.
    fn authored_write_arguments(
        &self,
        tokens: &CommandTokens,
    ) -> Option<
        Vec<(
            usize,
            tcl_registry::resolved_invocation::VariableReceiverOperandForm,
        )>,
    > {
        self.with_operation(tokens, |operation, schema| {
            let (roles, complete) = schema.authored_source_argument_roles();
            if !complete {
                return None;
            }
            let aliases = schema.authored_logical_source_variable_aliases()?;
            let mut writes = Vec::new();
            for (argument, role) in roles {
                if role != tcl_registry::ArgRole::VarWrite {
                    continue;
                }
                let argument = schema
                    .semantics
                    .argument_offset
                    .checked_add(usize::from(argument))?;
                if aliases
                    .iter()
                    .any(|alias| alias.local.argument_index() == Some(argument))
                {
                    continue;
                }
                let form = schema.authored_source_variable_receiver_operand_form(argument)?;
                writes.push((operation.effective.written_argument(argument)?, form));
            }
            Some(writes)
        })
    }

    /// A release unit's original registry words, through the document realm.
    fn authored_registry_words(
        &self,
        tokens: &CommandTokens,
    ) -> Option<crate::registry_invocation::source_structure::OriginalRegistryWords> {
        crate::registry_invocation::source_structure::original_registry_words_for_supplied_tokens(
            crate::realm::document_realm_bindings_for_module(self.module, self.registry)?.as_ref(),
            self.module.source_metadata_input.as_ref()?,
            tokens,
        )
    }

    pub(super) fn argument_source_place(
        &self,
        tokens: &CommandTokens,
        written: usize,
        form: tcl_registry::resolved_invocation::VariableReceiverOperandForm,
    ) -> Option<crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence> {
        if self.logical() {
            return self
                .variable_operands()
                .argument_source_place(tokens, written, form);
        }
        let (span, input, frame) = self.authored_argument(tokens, written)?;
        crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence::from_authored_procedure_source(
            span,
            input,
            crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver::Operand(form),
            false,
            frame.frame,
            frame.namespace,
        )
    }

    pub(super) fn alias_local(
        &self,
        tokens: &CommandTokens,
        alias: &tcl_registry::VariableCellAliasTransition,
    ) -> Option<tcl_core_types::NameBytes> {
        use crate::registry_invocation::DeclarationVariableAliasPurpose as Purpose;
        use tcl_registry::VariableAliasTarget as Target;
        if self.logical() {
            return self.variable_operands().alias_local(tokens, alias);
        }
        let operation = self.operation(tokens)?;
        let written = operation
            .effective
            .written_argument(alias.local.argument_index()?)?;
        let (_, input, _) = self.authored_argument(tokens, written)?;
        let purpose = match alias.target {
            Target::Global { .. } => Purpose::Global,
            Target::CurrentNamespace { .. } => Purpose::NamespaceVariable,
            Target::Namespace { .. } => Purpose::NamespaceUpvar,
            Target::CallerSelectedFrame { .. } => Purpose::Upvar,
        };
        crate::signature_scan::variable_symbol::original_variable_alias_local_name(&input, purpose)
    }

    pub(super) fn alias_source_place(
        &self,
        tokens: &CommandTokens,
        alias: &tcl_registry::VariableCellAliasTransition,
    ) -> Option<crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence> {
        use tcl_registry::VariableAliasTarget as Target;
        if self.logical() {
            return self.variable_operands().alias_source_place(tokens, alias);
        }
        let variable = match &alias.target {
            Target::Global { variable }
            | Target::CurrentNamespace { variable }
            | Target::Namespace { variable, .. }
            | Target::CallerSelectedFrame { variable, .. } => variable,
        };
        let operation = self.operation(tokens)?;
        let local = operation
            .effective
            .written_argument(alias.local.argument_index()?)?;
        let target = operation
            .effective
            .written_argument(variable.argument_index()?)?;
        let (local_span, _, _) = self.authored_argument(tokens, local)?;
        let (span, input, frame) = self.authored_argument(tokens, target)?;
        let words = self.authored_registry_words(tokens)?;
        let analysis_input = self.module.source_metadata_input.as_ref()?;
        let operands = crate::registry_invocation::OriginalSourceVariableAliasOperands::capture(
            words.clone(),
            analysis_input.borrowed_context_registry(),
        )?;
        let purpose =
            crate::signature_scan::original_name::OriginalSourceNamePurpose::for_analysis_input(
                analysis_input,
                None,
                input.policy(),
            );
        let namespace = operands.target_namespace_scope_for_original_words(
            &words,
            analysis_input,
            (local_span, span),
            &input,
            &frame.namespace,
            purpose,
        )?;
        crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence::from_original_input(
            span,
            input,
            namespace,
            crate::signature_scan::variable_symbol::OriginalVariableSymbolPurpose::NamespaceStorage,
            crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver::NamespaceAlias,
            true,
        )
    }

    pub(super) fn alias_receiver_for_local(
        &self,
        tokens: &CommandTokens,
        local: &str,
    ) -> Option<tcl_registry::resolved_invocation::VariableReceiverOperandForm> {
        let operation = self.operation(tokens)?;
        let words = if self.logical() {
            crate::registry_invocation::source_structure::original_logical_registry_words_for_module(
                self.module, self.registry, tokens,
            )?
        } else {
            self.authored_registry_words(tokens)?
        };
        let operands = crate::registry_invocation::OriginalSourceVariableAliasOperands::capture(
            words,
            self.module
                .source_metadata_input
                .as_ref()?
                .borrowed_context_registry(),
        )?;
        let transitions = operation.facts.state_transitions.declared()?;
        let mut found = None;
        for fact in transitions.facts() {
            let tcl_registry::StateTransition::VariableCellAlias(alias) = &fact.transition else {
                continue;
            };
            if self.alias_local(tokens, alias)?.as_bytes() != local.as_bytes() {
                continue;
            }
            let tcl_registry::VariableAliasTarget::CallerSelectedFrame { frame, variable } =
                &alias.target
            else {
                return None;
            };
            let level = match frame {
                tcl_registry::CallerFrameSelection::DefaultCaller => {
                    Some(tcl_registry::FrameLevel::Relative(1))
                }
                tcl_registry::CallerFrameSelection::Explicit(level) => {
                    level.literal().and_then(|level| {
                        tcl_registry::FrameLevel::parse_for_dialect(level, operation.dialect?)
                    })
                }
            };
            if level != Some(tcl_registry::FrameLevel::Relative(1)) {
                return None;
            }
            let local = operation
                .effective
                .written_argument(alias.local.argument_index()?)?;
            let target = operation
                .effective
                .written_argument(variable.argument_index()?)?;
            let local = self.original_argument_word(tokens, local)?;
            let target = self.original_argument_word(tokens, target)?;
            let form = operands.target_receiver_operand_form((local.span(), target.span()))?;
            if found.is_some_and(|held| held != form) {
                return None;
            }
            found = Some(form);
        }
        found
    }

    pub(super) fn expression(
        &self,
        parent: &CommandTokens,
        expression: &crate::expr_ast::ExprNode,
        base: Option<u32>,
    ) -> Option<OriginalSummaryExpression> {
        self.metadata(parent)?;
        let mut pending = vec![parent.clone()];
        let mut selected: Option<OriginalSummaryExpression> = None;
        while let Some(tokens) = pending.pop() {
            self.metadata(&tokens)?;
            for written in 1..tokens.words().len() {
                let Some(advice) =
                    crate::registry_invocation::original_expression_operand_advice_for_word(
                        self.registry,
                        &tokens,
                        written,
                    )
                else {
                    continue;
                };
                if advice.expression != *expression
                    || base.is_some_and(|base| base != advice.expression_base)
                {
                    continue;
                }
                let next = OriginalSummaryExpression {
                    parent: tokens.clone(),
                    base: advice.expression_base,
                };
                if selected.as_ref().is_some_and(|previous| {
                    previous.base != next.base || previous.parent != next.parent
                }) {
                    return None;
                }
                selected = Some(next);
            }
            // Only a supplied original producer base permits descent through
            // that parent's retained child inventory. Text/tree equality alone
            // never locates a producer for an assembled expression.
            if base.is_some() {
                pending.extend(self.children(&tokens)?);
            }
        }
        selected
    }

    pub(super) fn expression_substitution(
        &self,
        expression: &OriginalSummaryExpression,
        script: &str,
        start: u32,
        end: u32,
    ) -> Option<OriginalSummaryScript> {
        let site = crate::ir::SourceSite::source(tcl_lexer::Span::new(
            expression.base.checked_add(start)?,
            expression.base.checked_add(end)?.checked_add(1)?,
        ));
        self.substitution(&expression.parent, &site, script)
    }

    pub(super) fn substitution(
        &self,
        parent: &CommandTokens,
        site: &crate::ir::SourceSite,
        script: &str,
    ) -> Option<OriginalSummaryScript> {
        self.metadata(parent)?;
        if site.provenance != crate::ir::Provenance::Source {
            return None;
        }
        let original = &parent.source_binding.as_ref()?.invocation_site()?.source;
        let spelling = original
            .source_image()
            .try_text()
            .ok()?
            .get(site.span.as_range())?;
        if spelling.strip_prefix('[')?.strip_suffix(']')? != script {
            return None;
        }
        let calls = crate::word_subst::checked_original_lifted_calls_with_metadata_context(
            parent,
            self.config.nested(),
            self.registry,
            self.metadata(parent)?,
        )?;
        let mut selected = calls.into_iter().filter(|call| call.span == site.span);
        let first = selected.next()?;
        if selected.next().is_some() {
            return None;
        }
        let child = first.tokens?;
        self.metadata(&child)?;
        Some(OriginalSummaryScript {
            text: script.to_owned(),
            base: site.span.start().checked_add(1)?,
            commands: vec![child],
        })
    }

    pub(super) fn body(
        &self,
        parent: &CommandTokens,
        argument: usize,
        script: &str,
    ) -> Option<OriginalSummaryScript> {
        let operation = self.operation(parent)?;
        if !operation
            .written_argument_roles()
            .contains(&(argument, tcl_registry::ArgRole::Body))
        {
            return None;
        }
        let argument = (0..operation.effective.words.len().checked_sub(1)?)
            .find(|&index| operation.effective.written_argument(index) == Some(argument))?;
        let word = operation.effective.words.get(argument.checked_add(1)?)?;
        if !matches!(word, crate::ir::WordExpr::BracedLiteral { .. }) {
            return None;
        }
        let source = &parent.source_binding.as_ref()?.invocation_site()?.source;
        let base = crate::command_binding::ExecutedScriptSource::literal_word_base(
            source.source_image().try_text().ok()?,
            word,
            script,
            self.config.nested(),
        )?;
        let end = base.checked_add(u32::try_from(script.len()).ok()?)?;
        let mut selected: Option<Vec<CommandTokens>> = None;
        for id in self.module.procedures.keys() {
            let procedure = self.declared_frame_procedure(id)?;
            let mut pending = vec![&procedure.body];
            while let Some(body) = pending.pop() {
                for statement in &body.statements {
                    let children = crate::ir_helpers::nested_bodies(statement);
                    if body.retained_source_tokens_for_statement(statement) == Some(parent) {
                        for child in &children {
                            if child.statements.is_empty() {
                                continue;
                            }
                            let mut commands = Vec::new();
                            let mut in_extent = true;
                            for statement in &child.statements {
                                if statement.synthetic_marker().is_some() {
                                    continue;
                                }
                                let tokens =
                                    child.retained_source_tokens_for_statement(statement)?;
                                self.metadata(tokens)?;
                                let site = tokens.source_binding.as_ref()?.invocation_site()?;
                                if &site.source != source
                                    || site.offset < base
                                    || site.offset >= end
                                {
                                    in_extent = false;
                                    break;
                                }
                                commands.push(tokens.clone());
                            }
                            if in_extent && !commands.is_empty() {
                                if selected
                                    .as_ref()
                                    .is_some_and(|previous| previous != &commands)
                                {
                                    return None;
                                }
                                selected = Some(commands);
                            }
                        }
                    }
                    pending.extend(children);
                }
            }
        }
        Some(OriginalSummaryScript {
            text: script.to_owned(),
            base,
            commands: selected?,
        })
    }

    pub(super) fn children(&self, tokens: &CommandTokens) -> Option<Vec<CommandTokens>> {
        crate::word_subst::checked_original_lifted_calls_with_metadata_context(
            tokens,
            self.config.nested(),
            self.registry,
            self.metadata(tokens)?,
        )?
        .into_iter()
        .map(|call| {
            let child = call.tokens?;
            self.metadata(&child)?;
            Some(child)
        })
        .collect()
    }

    /// Obtain existing original words from the retained IR owner. A lowered CFG
    /// statement cannot manufacture that owner from its reporting head or span.
    pub(super) fn statement_tokens(
        &self,
        body_id: Option<crate::ir::ProcedureId>,
        statement: &Statement,
    ) -> Option<CommandTokens> {
        let mut pending = Vec::new();
        match body_id {
            Some(id) => pending.push(&self.declared_frame_procedure(&id)?.body),
            None => pending.push(&self.module.top_level),
        }
        if pending.is_empty() {
            return None;
        }
        let mut found = None;
        while let Some(script) = pending.pop() {
            for original in &script.statements {
                if original.span() == statement.span()
                    && let Some(tokens) = script.retained_source_tokens_for_statement(original)
                {
                    self.metadata(tokens)?;
                    if tokens.source_binding.as_ref()?.invocation_site()?.offset
                        != statement.span().start()
                        || statement.tokens().is_some_and(|current| current != tokens)
                    {
                        return None;
                    }
                    if found.as_ref().is_some_and(|previous| previous != tokens) {
                        return None;
                    }
                    found = Some(tokens.clone());
                }
                pending.extend(crate::ir_helpers::nested_bodies(original));
            }
        }
        found
    }

    pub(super) fn body_commands(
        &self,
        qname: &crate::ir::ProcedureId,
    ) -> Option<Vec<CommandTokens>> {
        let procedure = self.declared_frame_procedure(qname)?;
        let mut commands = Vec::new();
        let mut pending: Vec<&Script> = vec![&procedure.body];
        while let Some(script) = pending.pop() {
            for statement in &script.statements {
                if statement.synthetic_marker().is_some() {
                    continue;
                }
                let tokens = script.retained_source_tokens_for_statement(statement)?;
                self.metadata(tokens)?;
                if tokens.source_binding.as_ref()?.invocation_site()?.offset
                    != statement.span().start()
                {
                    return None;
                }
                commands.push(tokens.clone());
                commands.extend(self.children(tokens)?);
                pending.extend(crate::ir_helpers::nested_bodies(statement));
            }
        }
        Some(commands)
    }
}

#[cfg(test)]
mod tests {
    use crate::interprocedural::test_body_identity::fixture_id;

    use super::*;

    /// A release unit's authored simulation has its own summary context: its
    /// call to a module procedure resolves to that procedure, and `set`,
    /// whose registry descriptor declares no state transitions, has the
    /// closed, empty transitions of the authored source view. The answer is
    /// the same under every release; this runs under Tcl 8.6.
    #[test]
    fn a_release_unit_has_an_authored_summary_context() {
        let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
            "proc a {} {set x 0; return $x}\nproc e {} {a}\n",
            context.commands(),
            false,
            "tcl8.6",
        );
        let module = &unit.ir_module;
        let source =
            SourceSummaryContext::for_module(module, context.commands(), module.lexer_config)
                .expect("the authored simulation's summary context");
        let a = fixture_id(&module.procedures, "::a");
        let e = fixture_id(&module.procedures, "::e");
        let callees: Vec<_> = source
            .body_commands(&e)
            .expect("e's commands")
            .iter()
            .filter_map(|tokens| source.procedure_call(tokens))
            .map(|call| call.id)
            .collect();
        assert_eq!(callees, [a]);
        let commands = source.body_commands(&a).expect("a's commands");
        let set = commands
            .iter()
            .find(|tokens| tokens.argv_texts.first().map(String::as_str) == Some("set"))
            .expect("the set command");
        assert!(source.procedure_call(set).is_none());
        assert!(
            source
                .transitions(set)
                .is_some_and(|transitions| transitions.facts().is_empty())
        );
    }

    #[test]
    fn original_expression_fused_top_level_retains_its_existing_parent_record() {
        // naming.expression.original-positioned-analysis-services
        // docs/design/analysis/name-resolution-proofs/expression-original-positioned-analysis-services.md
        // Retained top-level IR ownership, independently of any execution frame.
        let (context, unit) = crate::interprocedural::logical_completion_unit(
            "proc leaf {} {return 7}\nset answer [expr {[leaf] + 1}]",
            "tcl8.6",
        );
        let module = &unit.ir_module;
        let source =
            SourceSummaryContext::for_module(module, context.commands(), module.lexer_config)
                .unwrap();
        let (statement, expression, base) = module
            .top_level
            .statements
            .iter()
            .find_map(|statement| match statement {
                Statement::AssignExpr {
                    expr,
                    expr_base: Some(base),
                    ..
                } => Some((statement, expr, *base)),
                _ => None,
            })
            .expect("genuine source-fused expression");
        let parent = source
            .statement_tokens(None, statement)
            .expect("retained outer set words");
        let receipt = source
            .expression(&parent, expression, Some(base))
            .expect("genuine retained nested expr producer");
        assert!(
            source.expression(&parent, expression, None).is_none(),
            "tree equality cannot recover an unspecified child producer"
        );
        let child = source
            .expression_substitution(&receipt, "leaf", 0, 5)
            .unwrap();
        assert_eq!(child.base(), base + 1);
        assert!(child.command_at(child.base()).is_some());
    }

    #[test]
    fn original_expression_receipts_join_actual_literal_operand_and_child_extent() {
        // naming.expression.original-positioned-analysis-services
        // docs/design/analysis/name-resolution-proofs/expression-original-positioned-analysis-services.md
        // Software source ancestry and byte extents, not Native Normal completion.
        let (context, unit) = crate::interprocedural::logical_completion_unit(
            "proc leaf {} {return 7}\ninterp alias {} e {} expr\nproc p {} {e {[leaf] + 1}; expr {\"é:[leaf]\" eq \"é:7\"}}",
            "tcl8.6",
        );
        let module = &unit.ir_module;
        let source =
            SourceSummaryContext::for_module(module, context.commands(), module.lexer_config)
                .unwrap();
        let expressions: Vec<_> = source
            .body_commands(&fixture_id(&module.procedures, "::p"))
            .unwrap()
            .into_iter()
            .filter(|tokens| {
                source
                    .operation(tokens)
                    .is_some_and(|operation| operation.facts.canonical_command == "expr")
            })
            .collect();
        assert_eq!(expressions.len(), 2);
        for (index, parent) in expressions.iter().enumerate() {
            let advice = crate::registry_invocation::original_expression_operand_advice_for_word(
                context.commands(),
                parent,
                1,
            )
            .expect("selected genuine literal expression operand");
            let receipt = source
                .expression(parent, &advice.expression, Some(advice.expression_base))
                .expect("actual parent and complete original expression");
            let relative = advice.expression_text.find("[leaf]").unwrap();
            let start = u32::try_from(relative).unwrap();
            let end = start + 5;
            let child = source
                .expression_substitution(&receipt, "leaf", start, end)
                .expect("same original child and inclusive parser extent");
            assert_eq!(child.base(), advice.expression_base + start + 1);
            assert!(child.command_at(child.base()).is_some());
            assert!(
                source
                    .expression_substitution(&receipt, "leaf", start, end + 1)
                    .is_none()
            );
            assert!(
                source
                    .expression_substitution(&receipt, " leaf", start, end)
                    .is_none()
            );
            assert!(
                source
                    .expression(parent, &advice.expression, Some(advice.expression_base + 1))
                    .is_none()
            );
            assert!(
                source
                    .expression(
                        parent,
                        &crate::expr_parser::parse_expr("0", None),
                        Some(advice.expression_base)
                    )
                    .is_none()
            );
            assert!(
                source
                    .expression(parent, &advice.expression, None)
                    .is_some(),
                "literal assembled operand {index}"
            );
        }
    }

    #[test]
    fn original_expression_receipts_refuse_cooked_and_foreign_producers() {
        // naming.expression.original-positioned-analysis-services
        // docs/design/analysis/name-resolution-proofs/expression-original-positioned-analysis-services.md
        // Whole source receipt refusal; no command-table or execution inference.
        let (context, unit) = crate::interprocedural::logical_completion_unit(
            "proc leaf {} {return 7}\nproc p {} {expr {[leaf]} + 1}",
            "tcl8.6",
        );
        let module = &unit.ir_module;
        let source =
            SourceSummaryContext::for_module(module, context.commands(), module.lexer_config)
                .unwrap();
        let parent = source
            .body_commands(&fixture_id(&module.procedures, "::p"))
            .unwrap()
            .into_iter()
            .find(|tokens| {
                source
                    .operation(tokens)
                    .is_some_and(|operation| operation.facts.canonical_command == "expr")
            })
            .unwrap();
        let assembled = crate::expr_parser::parse_expr("[leaf] + 1", None);
        assert!(source.expression(&parent, &assembled, None).is_none());
        let (foreign_context, foreign) = crate::interprocedural::logical_completion_unit(
            "proc leaf {} {return 8}\nproc p {} {expr {[leaf] + 1}}",
            "tcl8.6",
        );
        let foreign_source = SourceSummaryContext::for_module(
            &foreign.ir_module,
            foreign_context.commands(),
            foreign.ir_module.lexer_config,
        )
        .unwrap();
        let foreign_parent = foreign_source
            .body_commands(&fixture_id(&module.procedures, "::p"))
            .unwrap()
            .into_iter()
            .find(|tokens| {
                foreign_source
                    .operation(tokens)
                    .is_some_and(|operation| operation.facts.canonical_command == "expr")
            })
            .unwrap();
        let foreign_advice =
            crate::registry_invocation::original_expression_operand_advice_for_word(
                foreign_context.commands(),
                &foreign_parent,
                1,
            )
            .unwrap();
        assert!(
            source
                .expression(
                    &foreign_parent,
                    &foreign_advice.expression,
                    Some(foreign_advice.expression_base)
                )
                .is_none(),
            "independent whole source image cannot borrow this Module owner"
        );
    }

    #[test]
    fn original_expression_receipts_refuse_captured_and_shadowed_producers() {
        // naming.expression.original-positioned-analysis-services
        // docs/design/analysis/name-resolution-proofs/expression-original-positioned-analysis-services.md
        // No source producer can be invented from captured argv or a reported head.
        let assembled = crate::expr_parser::parse_expr("[leaf] + 1", None);
        let (captured_context, captured_unit) = crate::interprocedural::logical_completion_unit(
            "proc leaf {} {return 7}\ninterp alias {} e {} expr {[leaf] + 1}\nproc p {} {e}",
            "tcl8.6",
        );
        let captured = SourceSummaryContext::for_module(
            &captured_unit.ir_module,
            captured_context.commands(),
            captured_unit.ir_module.lexer_config,
        )
        .unwrap();
        let captured_parent = captured
            .body_commands(&fixture_id(&captured_unit.ir_module.procedures, "::p"))
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
        assert!(
            captured
                .expression(&captured_parent, &assembled, None)
                .is_none(),
            "captured argv without a retained original expression producer cannot donate a written operand"
        );
        let (shadow_context, shadow_unit) = crate::interprocedural::logical_completion_unit(
            "proc expr {args} {return 0}\nproc p {} {expr {[leaf] + 1}}",
            "tcl8.6",
        );
        let shadow = SourceSummaryContext::for_module(
            &shadow_unit.ir_module,
            shadow_context.commands(),
            shadow_unit.ir_module.lexer_config,
        )
        .unwrap();
        let parent = shadow
            .body_commands(&fixture_id(&shadow_unit.ir_module.procedures, "::p"))
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
        assert!(shadow.expression(&parent, &assembled, None).is_none());
    }

    #[test]
    fn original_transfer_alias_targets_keep_source_symbols_and_refuse_foreign_words() {
        let (context, unit) = crate::interprocedural::logical_completion_unit(
            "proc p {} {global ::g; global ::::g; set g 1}",
            "tcl8.6",
        );
        let module = &unit.ir_module;
        let source =
            SourceSummaryContext::for_module(module, context.commands(), module.lexer_config)
                .unwrap();
        let commands = source
            .body_commands(&fixture_id(&module.procedures, "::p"))
            .unwrap();
        let mut places = Vec::new();
        for tokens in &commands {
            let (aliases, _) = super::super::alias_facts(tokens, &source).unwrap();
            for alias in aliases {
                assert_eq!(source.alias_local(tokens, &alias).unwrap().as_bytes(), b"g");
                places.push(source.alias_source_place(tokens, &alias).unwrap());
                let mut missing = tokens.clone();
                missing.source_binding = None;
                assert!(source.alias_source_place(&missing, &alias).is_none());
            }
        }
        assert_eq!(places.len(), 2);
        assert_eq!(places[0].symbol(), places[1].symbol());
        assert_ne!(
            places[0].original_name_input().bytes(),
            places[1].original_name_input().bytes()
        );
        assert_eq!(
            places[0].symbol().checked_c_namespace_key().as_deref(),
            Some("::g")
        );
        let mut foreign = module.clone();
        foreign.source =
            tcl_lexer::SourceImage::document(&format!("{} ", module.source.try_text().unwrap()));
        assert!(
            SourceSummaryContext::for_module(&foreign, context.commands(), foreign.lexer_config)
                .is_none()
        );
    }

    #[test]
    fn original_transfer_call_keeps_selected_procedure_and_captured_operands_together() {
        // naming.interprocedural.original-transfer-summary-source-context
        // docs/design/analysis/name-resolution-proofs/interprocedural-original-transfer-summary-source-context.md
        // Conditional source/argv correspondence, not Native activation or effects.
        let (context, unit) = crate::interprocedural::logical_completion_unit(
            "proc leaf {name {by 1} args} {return $name}\ninterp alias {} frozen {} leaf chosen\nproc caller {} {frozen 2 extra}",
            "tcl8.6",
        );
        let module = &unit.ir_module;
        let source =
            SourceSummaryContext::for_module(module, context.commands(), module.lexer_config)
                .expect("genuine complete Logical source owner");
        let commands = source
            .body_commands(&fixture_id(&module.procedures, "::caller"))
            .unwrap();
        let [original] = commands.as_slice() else {
            panic!("one original call")
        };
        let call = source
            .procedure_call(original)
            .expect("selected original procedure");
        assert_eq!(call.procedure.qualified_name, "::leaf");
        assert_eq!(
            call.literal_arguments(module).unwrap(),
            [
                Some("chosen".into()),
                Some("2".into()),
                Some("extra".into())
            ]
        );
        assert_eq!(call.arguments.written_argument(0), None);
        assert_eq!(call.arguments.written_argument(1), Some(0));
        let mut altered = original.clone();
        let crate::ir::WordExpr::Literal { text, .. } = &mut altered.word_exprs[0] else {
            panic!("authentic literal alias head")
        };
        *text = "leaf".into();
        assert!(source.procedure_call(&altered).is_none());
    }

    #[test]
    fn original_transfer_child_and_body_require_the_genuine_parent_word() {
        // naming.interprocedural.original-transfer-summary-source-context
        // docs/design/analysis/name-resolution-proofs/interprocedural-original-transfer-summary-source-context.md
        // Existing original child carriers, independently of entered Body authority.
        let (context, unit) = crate::interprocedural::logical_completion_unit(
            "proc leaf {} {return 9}\ninterp alias {} each {} dict for\nproc caller {} {catch {leaf}; each {k v} {} {leaf}; return [leaf]}",
            "tcl8.6",
        );
        let module = &unit.ir_module;
        let source =
            SourceSummaryContext::for_module(module, context.commands(), module.lexer_config)
                .unwrap();
        let commands = source
            .body_commands(&fixture_id(&module.procedures, "::caller"))
            .unwrap();
        let catch = commands
            .iter()
            .find(|tokens| {
                source
                    .operation(tokens)
                    .is_some_and(|operation| operation.facts.canonical_command == "catch")
            })
            .unwrap();
        let body = source
            .body(catch, 0, "leaf")
            .expect("existing selected original Body child");
        assert!(body.base() > 0);
        assert!(body.command_at(body.base()).is_some());
        assert!(source.body(catch, 1, "leaf").is_none());
        assert!(source.body(catch, 0, " leaf").is_none());
        let compound = commands
            .iter()
            .find(|tokens| {
                source.operation(tokens).is_some_and(|operation| {
                    operation.facts.canonical_command == "dict"
                        && operation
                            .written_argument_roles()
                            .contains(&(2, tcl_registry::ArgRole::Body))
                })
            })
            .expect("selected compound operation through a captured subcommand");
        let compound_body = source
            .body(compound, 2, "leaf")
            .expect("same original written body after selected subcommand offset");
        assert!(compound_body.command_at(compound_body.base()).is_some());
        assert!(source.body(compound, 1, "leaf").is_none());
        let parent = commands
            .iter()
            .find(|tokens| {
                tokens
                    .words()
                    .last()
                    .is_some_and(|word| word.sole_command_substitution().is_some())
            })
            .unwrap();
        let (_, site) = parent
            .words()
            .last()
            .unwrap()
            .sole_command_substitution()
            .unwrap();
        let child = source
            .substitution(parent, site, "leaf")
            .expect("existing selected original substitution");
        assert!(child.command_at(child.base()).is_some());
        assert!(source.substitution(parent, site, " leaf").is_none());
        let mut detached = site.clone();
        detached.provenance = crate::ir::Provenance::Opaque;
        assert!(source.substitution(parent, &detached, "leaf").is_none());
    }

    #[test]
    fn original_transfer_source_refuses_missing_foreign_config_and_changed_headers() {
        // naming.interprocedural.original-transfer-summary-source-context
        // docs/design/analysis/name-resolution-proofs/interprocedural-original-transfer-summary-source-context.md
        // Software source-world correspondence; no Native dispatch permission.
        let (context, unit) =
            crate::interprocedural::logical_completion_unit("proc p {x} {return $x}", "tcl8.6");
        let module = &unit.ir_module;
        assert!(
            SourceSummaryContext::for_module(module, context.commands(), module.lexer_config)
                .is_some()
        );
        let mut config = module.lexer_config;
        config.expand_syntax = !config.expand_syntax;
        assert!(SourceSummaryContext::for_module(module, context.commands(), config).is_none());
        let foreign =
            tcl_registry::model::ingress::resolve_environment("tcl8.4").default_context_registry();
        assert!(
            SourceSummaryContext::for_module(module, foreign.commands(), module.lexer_config)
                .is_none()
        );
        let mut missing = module.clone();
        missing.source_metadata_input = None;
        assert!(
            SourceSummaryContext::for_module(&missing, context.commands(), missing.lexer_config)
                .is_none()
        );
        let mut changed = module.clone();
        {
            let id = fixture_id(&changed.procedures, "::p");
            changed.procedures.get_mut(&id)
        }
        .unwrap()
        .params_raw
        .push_str(" injected");
        let source =
            SourceSummaryContext::for_module(&changed, context.commands(), changed.lexer_config)
                .unwrap();
        assert!(
            source
                .procedure(&fixture_id(&changed.procedures, "::p"))
                .is_none()
        );
        let mut moved = module.clone();
        let procedure = {
            let id = fixture_id(&moved.procedures, "::p");
            moved.procedures.remove(&id)
        }
        .unwrap();
        let mut foreign_module = crate::ir::Module::default();
        let foreign_id = foreign_module.retain_procedure(procedure.clone());
        moved.procedures.insert(foreign_id, procedure);
        let source =
            SourceSummaryContext::for_module(&moved, context.commands(), moved.lexer_config)
                .unwrap();
        assert!(source.procedure(&foreign_id).is_none());
    }

    #[test]
    fn original_transfer_shadow_and_command_observers_do_not_borrow_catalogue_closure() {
        // naming.interprocedural.original-transfer-summary-source-context
        // docs/design/analysis/name-resolution-proofs/interprocedural-original-transfer-summary-source-context.md
        let (context, unit) = crate::interprocedural::logical_completion_unit(
            "proc set {name value} {return changed}\nproc caller {} {set x 1}",
            "tcl8.6",
        );
        let module = &unit.ir_module;
        let source =
            SourceSummaryContext::for_module(module, context.commands(), module.lexer_config)
                .unwrap();
        let commands = source
            .body_commands(&fixture_id(&module.procedures, "::caller"))
            .unwrap();
        let [original] = commands.as_slice() else {
            panic!("one original replacement call")
        };
        assert_eq!(
            source
                .procedure_call(original)
                .unwrap()
                .procedure
                .qualified_name,
            "::set"
        );
        assert!(source.operation(original).is_none());
        let mut observed = module.clone();
        observed.traced_commands.insert("::set".into());
        assert!(
            SourceSummaryContext::for_module(&observed, context.commands(), observed.lexer_config)
                .is_none()
        );
        // A release unit's authored simulation is a source model and has a
        // summary context; a Standalone unit, which no source model owns,
        // has none.
        let release_context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let registry = release_context.commands();
        let text = "proc p {} {return 1}";
        let release = crate::compilation_unit::CompilationUnit::build_for_dialect(
            text, registry, false, "tcl8.6",
        );
        let module = &release.ir_module;
        assert!(
            SourceSummaryContext::for_module(module, registry, module.lexer_config).is_some_and(
                |source| source.purpose == OriginalInvocationSourcePurpose::AuthoredSimulation
            )
        );
        let input = module.source_metadata_input.as_ref().unwrap();
        let standalone =
            crate::compilation_unit::CompilationUnit::build_standalone_with_analysis_input(
                text,
                crate::compilation_unit::UnitBuildOptions {
                    registry,
                    config: module.lexer_config,
                    dialect: Some(input.unit_profile()),
                    defer_top_level: false,
                    external_call_sites: None,
                    declared_commands: None,
                },
                input,
            )
            .unwrap();
        assert!(
            SourceSummaryContext::for_module(
                &standalone.ir_module,
                registry,
                standalone.ir_module.lexer_config
            )
            .is_none()
        );
    }
}
