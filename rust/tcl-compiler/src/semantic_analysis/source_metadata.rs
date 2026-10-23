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

//! Conditional source metadata, independently of entered invocation facts.

use std::sync::Arc;

use tcl_registry::{ArgRole, CanonicalCommandDescriptorKey, CommandRegistry, InvocationFacts};

use crate::analyser::ResolvedAnalysisInput;
use crate::ir::{CommandTokens, Script};
use crate::registry_invocation::{InvocationMetadataContext, original_source_metadata_name_roles};

/// Partial metadata inventory over authentic retained source occurrences.
/// Missing entries and an empty inventory supply no absence or completeness proof.
/// A registry descriptor here establishes neither invocation presence nor Normal
/// completion, an entered frame, a runtime value, or executable rewrite authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionalSourceMetadata {
    input: Option<ResolvedAnalysisInput>,
    invocations: Vec<ConditionalSourceInvocationMetadata>,
}

impl ConditionalSourceMetadata {
    pub(super) const fn unavailable() -> Self {
        Self {
            input: None,
            invocations: Vec::new(),
        }
    }

    pub(super) fn build(
        registry: &CommandRegistry,
        context: Option<InvocationMetadataContext<'_>>,
        script: &Script,
    ) -> Self {
        let Some(input) = context
            .filter(|context| context.matches_registry(registry))
            .and_then(InvocationMetadataContext::source_analysis_input)
        else {
            return Self::unavailable();
        };
        let mut invocations = Vec::new();
        let mut pending = vec![script];
        while let Some(script) = pending.pop() {
            for statement in &script.statements {
                if let Some(tokens) = script.retained_source_tokens_for_statement(statement)
                    && let Some(invocation) = ConditionalSourceInvocationMetadata::capture(
                        registry,
                        input,
                        statement.span(),
                        tokens,
                    )
                {
                    invocations.push(invocation);
                }
            }
            pending.extend(
                script
                    .statements
                    .iter()
                    .rev()
                    .flat_map(|statement| statement.child_scripts().into_iter().rev()),
            );
        }
        Self {
            input: Some(input.clone()),
            invocations,
        }
    }

    /// The exact supplied source and availability owner, when retained.
    /// Availability-only or missing context never manufactures this owner.
    #[must_use]
    pub const fn source_input(&self) -> Option<&ResolvedAnalysisInput> {
        self.input.as_ref()
    }

    /// Individually represented conditional source occurrences. Unrepresented
    /// commands and opaque regions remain unknown.
    pub fn invocations(&self) -> impl Iterator<Item = &ConditionalSourceInvocationMetadata> {
        self.invocations.iter()
    }

    pub(super) fn rebind(&mut self, transport: &super::BodySemanticSourceTransport) -> Option<()> {
        for invocation in &mut self.invocations {
            let (original, selected) = transport.tokens.get(&invocation.span.start())?;
            if original != invocation.original.as_ref() {
                return None;
            }
            let mut spans = transport
                .statements
                .iter()
                .filter_map(|(original, selected)| {
                    (original.span() == invocation.span).then_some(selected.span())
                });
            let span = spans.next()?;
            if spans.any(|other| other != span) {
                return None;
            }
            invocation.original = Arc::new(selected.clone());
            invocation.span = span;
        }
        Some(())
    }
}

/// Registry-selected source schema with its original occurrence and unknown
/// applicability retained. Its private facts cannot become executable resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionalSourceInvocationMetadata {
    original: Arc<CommandTokens>,
    span: tcl_lexer::Span,
    facts: Box<InvocationFacts>,
    roles: Vec<(usize, ArgRole)>,
    roles_complete: bool,
    direct_written_arguments: bool,
    unknown_residual: bool,
}

impl ConditionalSourceInvocationMetadata {
    fn capture(
        registry: &CommandRegistry,
        input: &ResolvedAnalysisInput,
        span: tcl_lexer::Span,
        tokens: &CommandTokens,
    ) -> Option<Self> {
        let binding = tokens.source_binding.as_ref()?;
        binding.original_invocation_metadata_for_input(tokens, input, registry)?;
        let (command, recorded) = binding.original_recorded_command()?;
        if command.span != span || recorded.words() != tokens.words() {
            return None;
        }
        let inventory = original_source_metadata_name_roles(registry, tokens, input)?;
        if inventory.config().normalized() != input.lexer_config().normalized() {
            return None;
        }
        Some(Self {
            original: Arc::new(tokens.clone()),
            span,
            facts: Box::new(inventory.facts().clone()),
            roles: inventory.roles().to_vec(),
            roles_complete: inventory.roles_complete(),
            direct_written_arguments: inventory.direct_written_arguments(),
            unknown_residual: inventory.has_unknown_residual(),
        })
    }

    /// Descriptor identity selected for this conditional source occurrence.
    /// It is independent of the runtime command binding.
    #[must_use]
    pub fn descriptor_key(&self) -> CanonicalCommandDescriptorKey<'_> {
        self.facts.descriptor_key()
    }

    /// Exact statement geometry from the current retained source.
    #[must_use]
    pub const fn span(&self) -> tcl_lexer::Span {
        self.span
    }

    /// Effective post-head role coordinates supplied by this source schema.
    /// Direct written correspondence remains a separate property.
    #[must_use]
    pub fn roles(&self) -> &[(usize, ArgRole)] {
        &self.roles
    }

    /// Whether the selected schema supplied every argument role.
    #[must_use]
    pub const fn roles_complete(&self) -> bool {
        self.roles_complete
    }

    /// Whether every post-head source argument is directly written, without
    /// alias-captured prefix operands supplying another source occurrence.
    #[must_use]
    pub const fn direct_written_arguments(&self) -> bool {
        self.direct_written_arguments
    }

    /// Source lookup, role or applicability alternatives that remain unknown.
    /// A false result still establishes no entered handler or runtime binding.
    #[must_use]
    pub const fn has_unknown_residual(&self) -> bool {
        self.unknown_residual
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation_unit::{CompilationUnit, UnitBuildOptions};
    use crate::dispatch_proof::DispatchEntryAssumption;
    use crate::semantic_analysis::SemanticAnalysisBundle;

    #[test]
    fn conditional_source_metadata_keeps_input_and_refuses_changed_original_carriers() {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let registry = context.commands();
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input = ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
        let unit = CompilationUnit::build_with_analysis_input(
            "puts VALUE",
            UnitBuildOptions {
                registry,
                defer_top_level: false,
                config,
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            &input,
        );
        let metadata = unit.top_level.semantic_facts.conditional_source_metadata();
        assert_eq!(metadata.source_input(), Some(&input));
        let calls = metadata.invocations().collect::<Vec<_>>();
        let [call] = calls.as_slice() else {
            panic!("actual original source occurrence");
        };
        assert_eq!(call.descriptor_key().label(), "puts");
        assert_eq!(call.span(), unit.ir_module.top_level.statements[0].span());
        assert!(call.direct_written_arguments());
        assert!(
            ConditionalSourceInvocationMetadata::capture(
                registry,
                &input,
                tcl_lexer::Span::new(call.span().start() + 1, call.span().end()),
                call.original.as_ref(),
            )
            .is_none()
        );
        assert!(
            !unit
                .top_level
                .semantic_facts
                .executable()
                .invocations()
                .any(|invocation| {
                    matches!(
                        invocation.resolution,
                        crate::executable_ir::InvocationResolution::Resolved(_)
                    )
                })
        );

        let mut changed = unit.ir_module.top_level.clone();
        changed.statements[0].tokens_mut().unwrap().argv_texts[0] = "set".to_owned();
        let mut missing = unit.ir_module.top_level.clone();
        missing.statements[0].tokens_mut().unwrap().source_binding = None;
        for body in [changed, missing] {
            let refused =
                SemanticAnalysisBundle::build_for_interactive_analysis_with_metadata_context(
                    registry,
                    InvocationMetadataContext::for_analysis_input(registry, &input),
                    &body,
                    DispatchEntryAssumption::UnknownWorld,
                );
            assert!(
                refused
                    .conditional_source_metadata()
                    .invocations()
                    .next()
                    .is_none()
            );
        }
        let absent = SemanticAnalysisBundle::build_for_interactive_analysis_with_metadata_context(
            registry,
            None,
            &unit.ir_module.top_level,
            DispatchEntryAssumption::UnknownWorld,
        );
        assert!(
            absent
                .conditional_source_metadata()
                .source_input()
                .is_none()
        );
        assert!(
            absent
                .conditional_source_metadata()
                .invocations()
                .next()
                .is_none()
        );
    }
}
