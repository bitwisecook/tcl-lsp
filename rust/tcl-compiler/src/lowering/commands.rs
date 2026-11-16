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

//! Lowering one command: its tokens and metadata, the selection between structured hooks, registered hooks and the default call.

use super::Lowerer;
use crate::ir::{CommandBindingSite, CommandTokens, Statement};
use crate::lowering_hooks::{LoweringCommand, ResolvedLowering};
use crate::segmenter::SegmentedCommand;
use tcl_registry::CommandRegistry;
use tcl_registry::events::IrulesExecutionContext;
use tcl_registry::hooks::LoweringHookId;

mod default_call;
mod invocation_metadata;
mod structured_hooks;

#[inline(never)]
pub(super) fn resolved_top_level_priority(
    registry: &CommandRegistry,
    command: &str,
    args: &[Option<String>],
) -> Option<u16> {
    let references = args
        .iter()
        .map(|argument| argument.as_deref())
        .collect::<Option<Vec<_>>>()?;
    match registry.irules_top_level_effect(command, &references)? {
        tcl_registry::events::IrulesTopLevelDeclaration::Priority { value } => Some(value),
        _ => None,
    }
}

impl Lowerer<'_> {
    /// Collect proved handler directives as metadata, independently of opcode
    /// eligibility. Dead branches and unknown operands supply no literal row.
    fn record_namespace_directives(&mut self, tokens: &CommandTokens, namespace: &str) {
        use tcl_registry::{NamespaceTransition, NamespaceTransitionTarget, TransitionSubject};
        if self.dead_code_depth != 0 {
            return;
        }
        let footprint = match self.invocation_metadata_context() {
            Some(context) => {
                crate::registry_invocation::namespace_directive_footprint_with_metadata_context(
                    self.registry,
                    Some(context),
                    tokens,
                )
            }
            None if self.allows_standalone_registry_metadata() => {
                crate::registry_invocation::namespace_directive_footprint(
                    self.registry,
                    None,
                    tokens,
                )
            }
            None => None,
        };
        let Some(footprint) = footprint else { return };
        let namespace = tokens
            .source_binding
            .as_ref()
            .map_or(namespace, |binding| binding.lookup_namespace.as_str());
        if let Some(site) = tokens
            .source_binding
            .as_ref()
            .and_then(crate::command_binding::SourceInvocationBinding::invocation_site)
            && !self
                .namespace_directive_sites
                .insert((site.clone(), namespace.to_owned()))
        {
            return;
        }
        for directive in footprint.transitions() {
            match directive {
                NamespaceTransition::Import {
                    namespace: NamespaceTransitionTarget::Current,
                    patterns,
                    ..
                } => {
                    for pattern in patterns.iter().filter_map(TransitionSubject::literal) {
                        self.namespace_imports
                            .push((namespace.to_owned(), pattern.to_owned()));
                    }
                }
                NamespaceTransition::Export {
                    namespace: NamespaceTransitionTarget::Current,
                    patterns,
                } => {
                    let Some((_, patterns)) =
                        NamespaceTransition::export_pattern_operands(patterns)
                    else {
                        continue;
                    };
                    self.namespace_exports.extend(
                        patterns
                            .iter()
                            .filter_map(TransitionSubject::literal)
                            .map(|pattern| (namespace.to_owned(), pattern.to_owned())),
                    );
                }
                _ => {}
            }
        }
    }

    /// Store a non-structured hook's exact dependency only when it replaced
    /// ordinary command dispatch with specialised IR.
    fn finish_resolved_lowering(
        &mut self,
        seg: &SegmentedCommand,
        lowered: ResolvedLowering,
    ) -> Statement {
        let source_tokens = lowered
            .binding
            .as_ref()
            .map(|_| Box::new(self.cmd_tokens(seg)));
        let shape = if lowered.binding.is_some() {
            self.invocation_shape(seg)
        } else {
            tcl_registry::release_compilation::InvocationShape::default()
        };
        if let Some(binding) = lowered.binding
            && let Some(sites) = self.command_binding_site_stack.last_mut()
        {
            sites.push(CommandBindingSite {
                span: seg.span,
                source_tokens,
                known_namespaces: self
                    .source_bindings
                    .as_ref()
                    .map(|bindings| bindings.known_namespaces_at(seg.span.start())),
                variable_frame: self.source_bindings.as_ref().map(|bindings| {
                    bindings
                        .invocation_at_source(seg.name(), seg.span.start())
                        .variable_frame
                }),
                variable_context: self.source_bindings.as_ref().map(|bindings| {
                    bindings
                        .invocation_at_source(seg.name(), seg.span.start())
                        .variable_context
                }),
                existing_namespace_cells: self.source_bindings.as_ref().map(|bindings| {
                    bindings
                        .invocation_at_source(seg.name(), seg.span.start())
                        .existing_namespace_cells
                }),
                binding,
                shape,
            });
        }
        lowered.statement
    }

    /// Lower a single command.
    pub(super) fn lower_command(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
    ) -> Option<Box<Statement>> {
        if seg.texts.is_empty() {
            return None;
        }
        // Only the structured dispatch below recurses into nested bodies, so
        // every other selection runs in its own out-of-line frame: this frame
        // recurs once per nested body level under the depth guard's budget.
        match self.lower_command_selection(seg, namespace) {
            CommandLoweringSelection::Lowered(statement) => Some(statement),
            CommandLoweringSelection::Structured(resolved_name) => {
                if let Some(stmt) =
                    self.try_dispatch_structured_hook(&resolved_name, seg, namespace)
                {
                    return Some(stmt);
                }
                Some(self.lower_default_boxed(seg, namespace))
            }
        }
    }

    /// Every non-recursive selection of [`Self::lower_command`]: a finished
    /// statement, or the resolved label of a possible structured form.
    #[inline(never)]
    fn lower_command_selection(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
    ) -> CommandLoweringSelection {
        match self.select_command_lowering(seg, namespace) {
            CommandLoweringStep::Selected(selection) => selection,
            CommandLoweringStep::Default => {
                CommandLoweringSelection::Lowered(self.lower_default_boxed(seg, namespace))
            }
        }
    }

    /// Decide how a command lowers. The default lowering recurses into
    /// evaluated bodies, so it is left to the caller: this frame, with every
    /// branch's temporaries, is then not held across that recursion.
    #[inline(never)]
    fn select_command_lowering(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
    ) -> CommandLoweringStep {
        // Analysis uses the selected logical handler in its owned source scope.
        // Bytecode separately requires original native compiler admission.
        let tokens = self.cmd_tokens_boxed(seg);
        self.record_namespace_directives(&tokens, namespace);
        if let Some(declaration) = self.try_lower_rule_loader_declaration(seg, namespace, &tokens) {
            return CommandLoweringStep::lowered(declaration);
        }
        if self.target.is_bytecode()
            && crate::registry_invocation::proved_native_admitted_inline_operation(&tokens)
                .is_none()
        {
            self.retain_entered_namespace_body(seg, namespace, &tokens);
        }
        if !self.target.is_bytecode()
            && !self.target.is_trace_visible()
            && let Some(statement) = self
                .try_lower_authored_procedure_declaration(seg, namespace, &tokens)
                .or_else(|| self.try_lower_logical_procedure_declaration(seg, namespace, &tokens))
        {
            return CommandLoweringStep::lowered(Box::new(statement));
        }
        if !self.target.is_bytecode()
            && !self.target.is_trace_visible()
            && let Some(statement) =
                self.try_lower_original_operand_evaluation_update(seg, namespace, &tokens)
        {
            return CommandLoweringStep::lowered(statement);
        }
        let resolved_name = if self.target.is_bytecode() {
            let target = tokens
                .source_binding
                .as_ref()
                .and_then(|binding| {
                    binding
                        .admitted_inline_invocation()
                        .map(|proof| &proof.target)
                        .or_else(|| binding.proved_execution_target())
                })
                .filter(|target| target.registry_backed);
            if target
                .as_ref()
                .is_none_or(|target| !target.prepended.is_empty())
            {
                return CommandLoweringStep::Default;
            }
            if let Some(admitted) = crate::registry_invocation::admitted_native_compiler_invocation(
                self.registry,
                None,
                &tokens,
            ) {
                admitted.descriptor_key().label().to_owned()
            } else if let Some(label) = self.retained_metadata_descriptor_label(&tokens) {
                label
            } else {
                return CommandLoweringStep::Default;
            }
        } else {
            let Some(logical) = self.logical_structured_invocation(&tokens) else {
                return self
                    .expanded_structured_barrier(seg, &tokens)
                    .map_or(CommandLoweringStep::Default, CommandLoweringStep::lowered);
            };
            logical.descriptor_key().label().to_owned()
        };
        let resolved_name = resolved_name.as_str();

        if self.irules_execution_context == IrulesExecutionContext::TopLevel
            && let Some(value) = tokens.source_binding.as_ref().and_then(|binding| {
                resolved_top_level_priority(
                    self.registry,
                    resolved_name,
                    &binding.evaluated_argument_values,
                )
            })
        {
            self.irules_priority = value;
        }

        // Trace-visible compilation retains runtime dispatch at every site.
        if self.target.is_trace_visible() {
            return CommandLoweringStep::Default;
        }

        if self.target.is_bytecode()
            && crate::registry_invocation::proved_native_admitted_inline_operation(&tokens)
                .is_none()
        {
            // A declaration still supplies a deferred body inventory. Its
            // returned Call retains ordinary dispatch; no native opcode or
            // before-arguments command requirement is minted from that inventory.
            if self
                .registry
                .get(resolved_name)
                .is_some_and(|spec| spec.lowering_hook == Some(LoweringHookId::Proc))
                && let Some(statement) =
                    self.try_lower_proc_declaration_boxed(seg, namespace, resolved_name)
            {
                return CommandLoweringStep::lowered(statement);
            }
            return CommandLoweringStep::Default;
        }

        if let Some(statement) = self.try_registered_lowering_hook(resolved_name, seg, namespace) {
            return CommandLoweringStep::lowered(statement);
        }

        // Registry-driven hook-ID dispatch covers all 17
        // structured command forms — every typed
        // `LoweringHookId` (Proc, When, NamespaceEval, If,
        // Switch, For, While, Foreach, Lmap, ForeachLine, Catch,
        // Try, Dict, Eval, Uplevel, Apply, ArrayFor) flows through
        // [`try_dispatch_structured_hook`], which also raises the
        // `{*}`-expansion barrier for them.  Commands that
        // aren't in the registry, or whose hook is `None`, fall
        // through to [`lower_default`] in [`Self::lower_command`].
        CommandLoweringStep::Selected(CommandLoweringSelection::Structured(
            resolved_name.to_owned(),
        ))
    }

    /// Retain an attempted update's original read plan without admitting its
    /// failed argv, handler or any body. Ordinary handler lowering stays gated.
    fn try_lower_original_operand_evaluation_update(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
        tokens: &CommandTokens,
    ) -> Option<Box<Statement>> {
        let selected = crate::registry_invocation::original_operand_evaluation_update(
            self.registry,
            self.invocation_metadata_context()?,
            tokens,
            self.source_bindings.as_deref()?,
        )?;
        let command = LoweringCommand {
            span: seg.span,
            name: selected.descriptor_key().label(),
            resolution_namespace: namespace,
            args: seg.args(),
            single_token_word: &seg.single_token_word,
            expand_word: seg.expand_word.as_deref(),
            tokens: Some(tokens.clone()),
            arg_kinds: &Self::arg_kinds(seg),
            dialect: self.dialect,
            lexer_config: self.config,
        };
        let lowered = crate::lowering_hooks::try_lower_operand_evaluation_update_with_binding(
            &command,
            &selected,
            &self.aliases,
            self.registry,
            self.dialect_context
                .as_deref()
                .map(tcl_registry::model::ContextRegistry::context),
            self.safe_on_uninit(command.name, command.args),
        )?;
        Some(Box::new(self.finish_resolved_lowering(seg, lowered)))
    }

    /// Keep declaration construction out of ordinary command descent frames.
    #[inline(never)]
    fn try_lower_proc_declaration_boxed(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
        resolved_name: &str,
    ) -> Option<Box<Statement>> {
        self.try_lower_proc_declaration(seg, namespace, resolved_name)
            .map(Box::new)
    }

    #[inline(never)]
    fn try_registered_lowering_hook(
        &mut self,
        resolved_name: &str,
        seg: &SegmentedCommand,
        namespace: &str,
    ) -> Option<Box<Statement>> {
        let args = seg.args();
        let hook_cmd = LoweringCommand {
            span: seg.span,
            name: resolved_name,
            resolution_namespace: namespace,
            args,
            single_token_word: &seg.single_token_word,
            expand_word: seg.expand_word.as_deref(),
            tokens: Some(self.cmd_tokens(seg)),
            arg_kinds: &Self::arg_kinds(seg),
            dialect: self.dialect,
            lexer_config: self.config,
        };
        let context = self
            .dialect_context
            .as_deref()
            .map(tcl_registry::model::ContextRegistry::context);
        let admitted = self.target.is_bytecode().then(|| {
            crate::registry_invocation::admitted_native_compiler_invocation(
                self.registry,
                None,
                hook_cmd.tokens.as_ref()?,
            )
        });
        let logical = (!self.target.is_bytecode())
            .then(|| self.logical_structured_invocation(hook_cmd.tokens.as_ref()?));
        let requirement = if let Some(admitted) = &admitted {
            crate::registry_invocation::SelectedLoweringBindingIdentity::Native(admitted.as_ref()?)
        } else {
            logical.as_ref()?.as_ref()?.lowering_identity()
        };
        let mut lowered = if self.target.is_bytecode() {
            crate::lowering_hooks::try_lower_admitted_hook_with_binding(
                &hook_cmd,
                &self.aliases,
                self.registry,
                context,
                self.safe_on_uninit(resolved_name, args),
            )?
        } else {
            let logical = logical.as_ref()?.as_ref()?;
            crate::lowering_hooks::try_lower_logical_hook_with_binding(
                &hook_cmd,
                logical,
                &self.aliases,
                self.registry,
                context,
                self.safe_on_uninit(resolved_name, args),
            )?
        };
        let tokens = self.cmd_tokens(seg);
        let name = crate::registry_invocation::static_command_word(
            &tokens,
            self.config.escapes,
            tcl_syntax::word_rules::WordValueRules::from_config(&self.config),
        )
        .unwrap_or_else(|| seg.name().to_owned());
        let binding = tcl_runtime_api::CommandBindingIdentity::in_rooted_namespace(
            namespace,
            name,
            requirement.binding_identity(),
        )
        .with_namespace_context(crate::registry_invocation::compiled_namespace_context(
            &tokens,
        ))
        .with_guard(crate::registry_invocation::command_binding_guard(&tokens));
        if let Statement::ExprEval {
            command_binding, ..
        } = &mut lowered.statement
        {
            *command_binding = binding.clone();
        }
        if lowered.binding.is_some() {
            lowered.binding = Some(binding);
        }
        Some(Box::new(self.finish_resolved_lowering(seg, lowered)))
    }

    #[inline(never)]
    pub(super) fn lower_default_boxed(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
    ) -> Box<Statement> {
        Box::new(self.lower_default(seg, namespace))
    }
}

/// [`Lowerer::select_command_lowering`]'s answer: a finished selection, or the
/// default lowering, which the caller performs.
enum CommandLoweringStep {
    Selected(CommandLoweringSelection),
    Default,
}

impl CommandLoweringStep {
    #[allow(clippy::unnecessary_box_returns)]
    pub(super) fn lowered(statement: Box<Statement>) -> Self {
        Self::Selected(CommandLoweringSelection::Lowered(statement))
    }
}

/// What [`Lowerer::lower_command`] does after its out-of-line selection.
enum CommandLoweringSelection {
    /// A finished statement, built outside the recursive descent frame.
    Lowered(Box<Statement>),
    /// The resolved label of a possible structured form, whose hook may
    /// recurse into nested bodies.
    Structured(String),
}
