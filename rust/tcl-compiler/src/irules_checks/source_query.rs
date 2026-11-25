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

//! The `IrulesSourceQuery` operations: original events, argument calls and the handler pairing census.

use super::flow_inventory::possible_irule_flow_command;
use super::{
    CollectFlowState, ConditionalSourcePairingAdvice, IrulesSourceQuery, OriginalEvent,
    OriginalIrulesFlowCommand, irules_function_source_config,
};
use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use tcl_lexer::Span;
use tcl_registry::{CommandRegistry, Traits};

struct PairingHandlerInput<'a> {
    id: &'a crate::ir::ProcedureId,
    function: &'a FunctionUnit,
    pub(super) procedure: &'a crate::ir::Procedure,
    pub(super) commands: &'a [tcl_lexer::NativeScriptCommandWords],
    metadata: crate::registry_invocation::InvocationMetadataContext<'a>,
}

struct PairingHandlerOutput<'a> {
    pub(super) original: &'a mut Vec<Vec<tcl_lexer::NativeWord>>,
    handler_flow_commands: &'a mut Vec<OriginalIrulesFlowCommand>,
    pub(super) schemas: &'a mut Vec<crate::registry_invocation::RegistryInvocationShape>,
    pub(super) completions: &'a mut Vec<tcl_registry::registry::InvocationCompletion>,
}

impl<'a> IrulesSourceQuery<'a> {
    pub(super) fn capture(
        unit: &'a CompilationUnit,
        registry: &'a CommandRegistry,
    ) -> Option<Self> {
        let module = &unit.ir_module;
        if module.source != tcl_lexer::SourceImage::document(&unit.source) {
            return None;
        }
        let standalone = module.source_entry.metadata_context.is_standalone()
            && module.source_metadata_input.is_none();
        let metadata = if standalone {
            None
        } else {
            let metadata = crate::registry_invocation::InvocationMetadataContext::for_module(
                registry, module,
            )?;
            let input = module.source_metadata_input.as_ref()?;
            if input.lexer_config() != module.lexer_config {
                return None;
            }
            crate::realm::document_realm_bindings_for_module(module, registry)?;
            Some(metadata)
        };
        Some(Self {
            unit,
            registry,
            standalone,
            metadata,
        })
    }

    pub(super) fn function_config(
        &self,
        function: &FunctionUnit,
    ) -> Option<tcl_lexer::LexerConfig> {
        irules_function_source_config(
            &self.unit.ir_module,
            function,
            self.registry,
            self.standalone,
        )
    }

    /// Distinguish ordinary functions from an unavailable original event.
    /// The actual map entry and lowering descriptor own identity; report names
    /// select neither source bodies nor conditional event frame policy.
    pub(super) fn event(
        &self,
        function: &FunctionUnit,
    ) -> Option<tcl_core_types::Knowledge<OriginalEvent<'_>>> {
        let module = &self.unit.ir_module;
        let Some((key, owned)) = self
            .unit
            .procedures
            .iter()
            .find(|(_, owned)| std::ptr::eq(*owned, function))
        else {
            return std::ptr::eq(function, std::ptr::from_ref(&self.unit.top_level))
                .then_some(tcl_core_types::Knowledge::Absent);
        };
        let Some(expected) = module.irules_event_bodies.get(key) else {
            return owned
                .irules_event_body
                .is_none()
                .then_some(tcl_core_types::Knowledge::Absent);
        };
        self.function_config(owned)?;
        let body = owned.irules_event_body.as_ref()?;
        let procedure = module.procedures.get(key)?;
        if !std::sync::Arc::ptr_eq(body, expected)
            || !body.owns_procedure(
                procedure,
                &module.source,
                module.lexer_config,
                self.registry,
            )
        {
            return None;
        }
        Some(tcl_core_types::Knowledge::Proven(OriginalEvent {
            body,
            procedure,
        }))
    }

    pub(super) fn argument_calls(
        &self,
        parent: &crate::ir::CommandTokens,
        config: tcl_lexer::LexerConfig,
    ) -> Option<Vec<crate::word_subst::LiftedCall>> {
        if let Some(metadata) = self.metadata {
            crate::word_subst::checked_original_argument_lifted_calls(
                parent,
                self.registry,
                metadata,
            )
        } else if self.standalone {
            crate::word_subst::checked_argument_lifted_calls(
                parent,
                parent.native_lexer_config(config),
            )
        } else {
            None
        }
    }

    pub(super) fn original_argv(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<Vec<tcl_lexer::NativeWord>> {
        let metadata = self.metadata?;
        let binding = tokens.source_binding.as_ref()?;
        binding.original_argument_metadata_for_input(
            tokens,
            metadata.source_analysis_input()?,
            self.registry,
        )?;
        let config = binding.original_argument_lexer_config(tokens)?;
        let image = binding.original_argument_source_image(tokens)?;
        crate::registry_invocation::original_native_compiler_words(
            image,
            tokens.words(),
            tokens.words().first()?.source().span.start(),
            config,
        )
    }

    fn pairing_declaration_words(
        &self,
        metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
        declaration: &crate::ir::CommandTokens,
        event: &OriginalEvent<'_>,
        context: &tcl_registry::model::ContextRegistry,
    ) -> Option<crate::registry_invocation::source_structure::OriginalRegistryWords> {
        let module = &self.unit.ir_module;
        let declaration_metadata =
            crate::registry_invocation::original_hosted_source_registry_metadata(
                self.registry,
                metadata,
                declaration,
            )?;
        let event_word =
            event
                .body
                .event_word(&module.source, module.lexer_config, self.registry)?;
        if !declaration_metadata
            .shape()
            .possible_traits()
            .contains(Traits::IS_EVENT_HANDLER)
            || !declaration_metadata
                .shape()
                .original_words()
                .contains(event_word)
            || declaration_metadata
                .authored_barriers()
                .has_nominal_command_name_hazard()
        {
            return None;
        }
        crate::registry_invocation::source_structure::original_vendor_registry_words(
            context,
            declaration,
            declaration_metadata,
        )
    }

    fn retain_pairing_handler_commands(
        &self,
        state: &CollectFlowState,
        input: &PairingHandlerInput<'_>,
        output: &mut PairingHandlerOutput<'_>,
    ) -> Option<()> {
        for statement in &input.procedure.body.statements {
            if matches!(
                statement.original_argv_inventory_disposition(),
                crate::ir::OriginalArgvInventoryDisposition::EffectOnly(_)
            ) {
                continue;
            }
            if !statement.child_scripts().is_empty() {
                // A statement with bodies is walked path by path by the flow
                // advice. The census keeps its written command so the handler
                // stays fully accounted for, and adds nothing else.
                let tokens = input
                    .procedure
                    .body
                    .retained_source_tokens_for_statement(statement)?;
                let vector = self.original_argv(tokens)?;
                if !input.commands.iter().any(|command| command.words == vector) {
                    return None;
                }
                output.original.push(vector);
                continue;
            }
            let tokens = input
                .procedure
                .body
                .retained_source_tokens_for_statement(statement)?;
            let vector = self.original_argv(tokens)?;
            if !input.commands.iter().any(|command| command.words == vector)
                || !state
                    .source_argv
                    .iter()
                    .any(|(owner, words)| owner == input.id && words == &vector)
            {
                return None;
            }
            output.original.push(vector);
            let children = self.argument_calls(tokens, self.function_config(input.function)?)?;
            for token in children
                .iter()
                .map(|child| child.tokens.as_ref())
                .collect::<Option<Vec<_>>>()?
                .into_iter()
                .chain(std::iter::once(tokens))
            {
                let vector = self.original_argv(token)?;
                if !state
                    .source_argv
                    .iter()
                    .any(|(owner, words)| owner == input.id && words == &vector)
                {
                    return None;
                }
                let possible = crate::registry_invocation::original_registry_invocation_assistance_with_metadata_context(self.registry, Some(input.metadata), token)?;
                if possible.candidates.is_empty() {
                    return None;
                }
                if let tcl_core_types::Knowledge::Proven(command) =
                    possible_irule_flow_command(self.registry, token, &possible)?
                {
                    output.handler_flow_commands.push(command);
                }
                for candidate in &possible.candidates {
                    let completion = candidate.with_possible_source_schema(
                        self.registry,
                        input.metadata,
                        token,
                        leaf_handler_source_completion,
                    )?;
                    output.schemas.push(candidate.clone());
                    output.completions.push(completion);
                }
            }
        }
        Some(())
    }

    pub(super) fn conditional_pairing_advice<'query>(
        &'query self,
        state: &'query CollectFlowState,
    ) -> Option<ConditionalSourcePairingAdvice<'query, 'a>> {
        let metadata = self.metadata?;
        let input = metadata.source_analysis_input()?;
        input.has_hosted_source_name_context().then_some(())?;
        let context = input.borrowed_context_registry();
        let module = &self.unit.ir_module;
        let root = tcl_lexer::native_script_words_in(
            module.source.clone(),
            Span::new(0, u32::try_from(module.source.len()).ok()?),
            module.lexer_config,
        )
        .ok()?;
        if root.fatal_tail.is_some() || root.commands.len() != module.irules_event_bodies.len() {
            return None;
        }
        let mut declarations_seen = Vec::new();
        let mut handlers = Vec::new();
        let mut schemas = Vec::new();
        let mut completions = Vec::new();
        let mut flow_commands = Vec::new();
        for id in module.irules_event_bodies.keys() {
            let function = self.unit.procedures.get(id)?;
            let event = self.event(function)?.into_proven()?;
            let mut declarations = module
                .top_level
                .statements
                .iter()
                .filter(|statement| statement.span() == event.procedure.span);
            let declaration = declarations.next()?;
            if declarations.next().is_some() {
                return None;
            }
            let declaration = module
                .top_level
                .retained_source_tokens_for_statement(declaration)?;
            let declaration_vector = self.original_argv(declaration)?;
            if !root
                .commands
                .iter()
                .any(|command| command.words == declaration_vector)
                || declarations_seen.contains(&declaration_vector)
            {
                return None;
            }
            declarations_seen.push(declaration_vector.clone());
            let words = self.pairing_declaration_words(metadata, declaration, &event, context)?;
            let mut bodies = words.source_script_bodies_for_mutation_coverage(context)?;
            if bodies.len() != 1 {
                return None;
            }
            let body = bodies.pop()?;
            for word in &declaration_vector {
                if word != body.original_container() {
                    tcl_syntax::word_rules::original_static_word_unicode_value(word)?;
                }
            }
            let commands = body.original_commands(context)?;
            let mut original = Vec::new();
            let mut handler_flow_commands = Vec::new();
            self.retain_pairing_handler_commands(
                state,
                &PairingHandlerInput {
                    id,
                    function,
                    procedure: event.procedure,
                    commands: &commands,
                    metadata,
                },
                &mut PairingHandlerOutput {
                    original: &mut original,
                    handler_flow_commands: &mut handler_flow_commands,
                    schemas: &mut schemas,
                    completions: &mut completions,
                },
            )?;
            if commands.len() != original.len()
                || commands
                    .iter()
                    .any(|command| !original.contains(&command.words))
            {
                return None;
            }
            handlers.push((*id, body));
            flow_commands.push((*id, handler_flow_commands));
        }
        if root
            .commands
            .iter()
            .any(|command| !declarations_seen.contains(&command.words))
        {
            return None;
        }
        (!handlers.is_empty()).then_some(ConditionalSourcePairingAdvice {
            _source: self,
            handlers,
            schemas,
            completions,
            flow_commands,
            _obligations: &state.obligations,
        })
    }

    pub(super) fn metadata(
        &self,
    ) -> Option<crate::registry_invocation::InvocationMetadataContext<'_>> {
        self.metadata
    }
}

pub(super) fn leaf_handler_source_completion(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> Option<tcl_registry::registry::InvocationCompletion> {
    let (roles, complete) = schema.authored_source_argument_roles();
    if !complete
        || roles
            .iter()
            .any(|(_, role)| *role == tcl_registry::ArgRole::LambdaLiteral)
    {
        return None;
    }
    let scripts = schema.authored_source_script_arguments()?;
    let prefixes = schema.authored_source_command_prefix_arguments()?;
    let expressions = schema.authored_source_expression_arguments()?;
    (scripts.is_empty() && prefixes.is_empty() && expressions.arguments.is_empty())
        .then(|| schema.authored_source_invocation_completion())
}
