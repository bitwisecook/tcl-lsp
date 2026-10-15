// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original Logical variable operands shared by source effect producers.

use crate::ir::{CommandTokens, Module, Script};
use crate::registry_invocation::{InvocationMetadataContext, ResolvedStatementInvocation};
use tcl_registry::CommandRegistry;

/// Source geometry only: no entered frame, current cell or completed write.
pub(crate) struct OriginalLogicalVariableOperands<'a> {
    module: &'a Module,
    registry: &'a CommandRegistry,
    realm: &'a crate::realm::CommandBindingRealm,
}

impl<'a> OriginalLogicalVariableOperands<'a> {
    pub(crate) const fn borrowing(
        module: &'a Module,
        registry: &'a CommandRegistry,
        realm: &'a crate::realm::CommandBindingRealm,
    ) -> Self {
        Self {
            module,
            registry,
            realm,
        }
    }

    pub(crate) fn metadata<'w>(
        &self,
        tokens: &'w CommandTokens,
    ) -> Option<InvocationMetadataContext<'w>>
    where
        'a: 'w,
    {
        let metadata = tokens
            .source_binding
            .as_ref()?
            .original_invocation_metadata_for_module(tokens, self.module, self.registry)?;
        (tokens
            .source_binding
            .as_ref()?
            .original_invocation_source_purpose(tokens, metadata, self.registry)?
            == crate::command_binding::OriginalInvocationSourcePurpose::Logical)
            .then_some(metadata)
    }

    pub(crate) fn operation(&self, tokens: &CommandTokens) -> Option<ResolvedStatementInvocation> {
        crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(
            self.registry,
            self.metadata(tokens)?,
            tokens,
        )
    }

    pub(crate) fn original_argument_word(
        &self,
        tokens: &CommandTokens,
        written: usize,
    ) -> Option<tcl_lexer::NativeWord> {
        self.metadata(tokens)?;
        let input = self.module.source_metadata_input.as_ref()?;
        let site = tokens.source_binding.as_ref()?.invocation_site()?;
        let candidate = self
            .realm
            .original_logical_navigation_candidates(input, site.offset)?;
        let original = candidate.original_words().get(written.checked_add(1)?)?;
        if original.span() != tokens.words().get(written.checked_add(1)?)?.source().span {
            return None;
        }
        Some(original.clone())
    }

    fn original_argument(
        &self,
        tokens: &CommandTokens,
        written: usize,
    ) -> Option<(
        tcl_lexer::Span,
        crate::signature_scan::scope::SignatureSourceNameInput,
        crate::command_binding::OriginalLogicalVariableScope,
    )> {
        let original = self.original_argument_word(tokens, written)?;
        let input = self.module.source_metadata_input.as_ref()?;
        let policy = tcl_registry::InvocationDialect::of_profile(input.unit_profile())
            .authored_name_policy()?;
        let purpose =
            crate::signature_scan::original_name::OriginalSourceNamePurpose::for_analysis_input(
                input, None, policy,
            );
        let name =
            crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(purpose.key(
                &original,
                tcl_syntax::word_rules::WordValueRules::from_config(&original.config()),
            )?);
        let scope = self.realm.original_logical_variable_scope_at(
            input,
            original.image(),
            original.span(),
            &name,
        )?;
        Some((original.span(), name, scope))
    }

    pub(crate) fn argument_source_place(
        &self,
        tokens: &CommandTokens,
        written: usize,
        form: tcl_registry::resolved_invocation::VariableReceiverOperandForm,
    ) -> Option<crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence> {
        let (span, input, scope) = self.original_argument(tokens, written)?;
        crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence::from_logical_source(
            span,
            input,
            crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver::Operand(form),
            false,
            scope,
            &[],
        )
    }

    pub(crate) fn alias_local(
        &self,
        tokens: &CommandTokens,
        alias: &tcl_registry::VariableCellAliasTransition,
    ) -> Option<tcl_core_types::NameBytes> {
        let crate::signature_scan::variable_symbol::SignatureSourceVariableSlot::Local {
            simple,
            ..
        } = self.alias_local_slot(tokens, alias)?
        else {
            return None;
        };
        Some(simple)
    }

    pub(crate) fn alias_local_slot(
        &self,
        tokens: &CommandTokens,
        alias: &tcl_registry::VariableCellAliasTransition,
    ) -> Option<crate::signature_scan::variable_symbol::SignatureSourceVariableSlot> {
        let operation = self.operation(tokens)?;
        let written = operation
            .effective
            .written_argument(alias.local.argument_index()?)?;
        let (_, input, scope) = self.original_argument(tokens, written)?;
        let frame = scope.local_frame()?;
        use crate::registry_invocation::DeclarationVariableAliasPurpose as Purpose;
        use tcl_registry::VariableAliasTarget as Target;
        let purpose = match alias.target {
            Target::Global { .. } => Purpose::Global,
            Target::CurrentNamespace { .. } => Purpose::NamespaceVariable,
            Target::Namespace { .. } => Purpose::NamespaceUpvar,
            Target::CallerSelectedFrame { .. } => Purpose::Upvar,
        };
        let simple = crate::signature_scan::variable_symbol::original_variable_alias_local_name(
            &input, purpose,
        )?;
        Some(
            crate::signature_scan::variable_symbol::SignatureSourceVariableSlot::Local {
                frame,
                simple,
            },
        )
    }

    pub(crate) fn alias_source_place(
        &self,
        tokens: &CommandTokens,
        alias: &tcl_registry::VariableCellAliasTransition,
    ) -> Option<crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence> {
        use tcl_registry::VariableAliasTarget as Target;
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
        let (local_span, _, local_scope) = self.original_argument(tokens, local)?;
        let (span, input, scope) = self.original_argument(tokens, target)?;
        if local_scope.local_frame()? != scope.local_frame()? {
            return None;
        }
        let words = crate::registry_invocation::source_structure::original_logical_registry_words_for_module(
            self.module, self.registry, tokens,
        )?;
        let analysis_input = self.module.source_metadata_input.as_ref()?;
        let operands = crate::registry_invocation::OriginalSourceVariableAliasOperands::capture(
            words.clone(),
            analysis_input.borrowed_context_registry(),
        )?;
        let policy = input.policy();
        let purpose =
            crate::signature_scan::original_name::OriginalSourceNamePurpose::for_analysis_input(
                analysis_input,
                None,
                policy,
            );
        let namespace = operands.target_namespace_scope_for_original_words(
            &words,
            analysis_input,
            (local_span, span),
            &input,
            &scope.namespace(),
            purpose,
        )?;
        crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence::from_logical_alias_target(
            span, input, namespace,
            crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver::NamespaceAlias,
            scope,
        )
    }

    pub(crate) fn body_commands(&self, body: &Script) -> Option<Vec<CommandTokens>> {
        let mut pending = vec![body];
        let mut commands = Vec::new();
        let mut sites = std::collections::HashMap::new();
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
                Self::retain_command(&mut commands, &mut sites, tokens.clone())?;
                pending.extend(crate::ir_helpers::nested_bodies(statement));
            }
        }
        let mut next = 0;
        while next < commands.len() {
            let parent = &commands[next];
            let metadata = self.metadata(parent)?;
            let mut children =
                crate::word_subst::checked_original_lifted_calls_with_metadata_context(
                    parent,
                    self.module.lexer_config.nested(),
                    self.registry,
                    metadata,
                )?
                .into_iter()
                .map(|call| call.tokens)
                .collect::<Option<Vec<_>>>()?;
            if self.operation(parent).is_some() {
                children.extend(super::original_immediate_body_commands_for_module(
                    self.module,
                    self.registry,
                    parent,
                )?);
            } else {
                crate::registry_invocation::original_logical_procedure_calls_for_module(
                    parent,
                    self.module,
                    self.registry,
                )?;
            }
            for child in children {
                self.metadata(&child)?;
                Self::retain_command(&mut commands, &mut sites, child)?;
            }
            next += 1;
        }
        Some(commands)
    }

    fn retain_command(
        commands: &mut Vec<CommandTokens>,
        sites: &mut std::collections::HashMap<(usize, u32), Vec<usize>>,
        tokens: CommandTokens,
    ) -> Option<()> {
        let site = tokens.source_binding.as_ref()?.invocation_site()?;
        let key = (std::sync::Arc::as_ptr(&site.source) as usize, site.offset);
        let same_site = sites.entry(key).or_default();
        for &index in same_site.iter() {
            if commands[index] == tokens {
                return Some(());
            }
            let mut held = commands[index].clone();
            let mut incoming = tokens.clone();
            held.source_binding = None;
            incoming.source_binding = None;
            if held != incoming {
                return None;
            }
        }
        // Distinct current binding/frame facets at one original source site
        // remain separate observations. The tokens retain every source Arc.
        same_site.push(commands.len());
        commands.push(tokens);
        Some(())
    }

    pub(crate) fn write_arguments(
        &self,
        tokens: &CommandTokens,
    ) -> Option<
        Vec<(
            usize,
            tcl_registry::resolved_invocation::VariableReceiverOperandForm,
        )>,
    > {
        let operation = self.operation(tokens)?;
        let realm = tokens.source_binding.as_ref()?.invocation_realm()?;
        operation.with_metadata_schema(self.registry, self.metadata(tokens)?, realm, |schema| {
            let (roles, complete) = schema.authored_source_argument_roles();
            if !complete {
                return None;
            }
            let mut writes = Vec::new();
            let aliases = schema.authored_logical_source_variable_aliases()?;
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_body_inventory_indexes_actual_source_sites_without_merging_images() {
        let (_, first) =
            crate::interprocedural::logical_completion_unit("set g 1; set g 1", "tcl8.6");
        let (_, second) =
            crate::interprocedural::logical_completion_unit("set g 1; set g 1", "tcl8.6");
        let original = |module: &Module, index: usize| {
            module
                .top_level
                .retained_source_tokens_for_statement(&module.top_level.statements[index])
                .unwrap()
                .clone()
        };
        let first_command = original(&first.ir_module, 0);
        let mut commands = Vec::new();
        let mut sites = std::collections::HashMap::new();
        OriginalLogicalVariableOperands::retain_command(
            &mut commands,
            &mut sites,
            first_command.clone(),
        )
        .unwrap();
        OriginalLogicalVariableOperands::retain_command(
            &mut commands,
            &mut sites,
            first_command.clone(),
        )
        .unwrap();
        assert_eq!(commands.len(), 1);
        OriginalLogicalVariableOperands::retain_command(
            &mut commands,
            &mut sites,
            original(&first.ir_module, 1),
        )
        .unwrap();
        OriginalLogicalVariableOperands::retain_command(
            &mut commands,
            &mut sites,
            original(&second.ir_module, 0),
        )
        .unwrap();
        assert_eq!(
            commands.len(),
            3,
            "equal spelling and equal offsets cannot merge distinct source occurrences"
        );
        let mut changed = first_command;
        changed.argv_texts[1] = "different".to_owned();
        assert!(
            OriginalLogicalVariableOperands::retain_command(&mut commands, &mut sites, changed)
                .is_none()
        );
        assert_eq!(
            commands.len(),
            3,
            "conflicting original token geometry cannot replace the retained source site"
        );
    }
}
