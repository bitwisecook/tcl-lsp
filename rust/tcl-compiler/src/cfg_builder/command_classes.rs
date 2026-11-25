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

//! Registry-derived command classifications of one release and dialect: completion routes and block-terminating heads.

use super::CfgCommandClasses;
use super::flow_facts::{Completion, exact_statement_completion};
use crate::ir::{CommandTokens, Statement};
use crate::registry_invocation::OwnedInvocationMetadataContext as CfgMetadataContext;
use std::sync::Arc;
use tcl_registry::CommandRegistry;
#[cfg(test)]
use tcl_registry::Traits;
#[cfg(test)]
use tcl_registry::hooks::LoweringHookId;

impl CfgCommandClasses {
    fn proved_statement_command(stmt: &Statement) -> Option<&str> {
        let target = stmt
            .tokens()?
            .source_binding
            .as_ref()?
            .proved_execution_target()?;
        target.registry_backed.then_some(target.command.as_str())
    }
    pub(super) fn from_registry(registry: &CommandRegistry) -> Self {
        Self {
            registry: registry.snapshot(),
            metadata: CfgMetadataContext::Standalone,
            #[cfg(test)]
            semantics: registry.effective_semantics_for_dialect(
                crate::environment_ingress::authoring_invocation_dialect(
                    registry,
                    None,
                    // dialect-drift-ok: a #[cfg(test)] field of a standalone (no document) context
                    tcl_lexer::LexerConfig::default(),
                ),
            ),
        }
    }

    pub(super) fn from_source_input(
        registry: &CommandRegistry,
        input: Option<&crate::analyser::ResolvedAnalysisInput>,
    ) -> Self {
        let mut classes = Self::from_registry(registry);
        classes.metadata = input
            .and_then(|input| {
                crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
                    registry, input,
                )
                .map(|_| input)
            })
            .map_or(CfgMetadataContext::Unavailable, |input| {
                CfgMetadataContext::SuppliedSource(Box::new(input.clone()))
            });
        classes
    }

    pub(super) fn from_metadata_input(
        registry: &CommandRegistry,
        input: crate::registry_invocation::InvocationMetadataInput<'_>,
    ) -> Self {
        let mut classes = Self::from_registry(registry);
        let retained = input.retain();
        classes.metadata = if retained.metadata_context(registry).is_some() {
            retained
        } else {
            CfgMetadataContext::Unavailable
        };
        classes
    }

    pub(super) fn from_metadata(
        registry: &CommandRegistry,
        metadata: Option<Arc<tcl_registry::model::ContextRegistry>>,
    ) -> Self {
        let mut classes = Self::from_registry(registry);
        classes.metadata = metadata.map_or(
            CfgMetadataContext::Unavailable,
            CfgMetadataContext::Supplied,
        );
        classes
    }

    pub(super) fn metadata_context(
        &self,
    ) -> Option<crate::registry_invocation::InvocationMetadataSelection<'_>> {
        self.metadata.metadata_context(self.registry.registry())
    }

    /// Source numeral grammar is independent of catalogue availability.
    pub(super) fn source_numbers(
        &self,
        config: tcl_lexer::LexerConfig,
    ) -> tcl_syntax::number::Numbers {
        use tcl_syntax::number::Numbers;
        match &self.metadata {
            CfgMetadataContext::SuppliedSource(input)
                if input.lexer_config().normalized() == config.normalized()
                    && self.metadata_context().is_some() =>
            {
                Numbers::Target(config.grammar_over(input.unit_profile().grammar).numbers)
            }
            CfgMetadataContext::Standalone => {
                Numbers::of_profile(self.registry.registry().profile())
            }
            CfgMetadataContext::Supplied(_)
            | CfgMetadataContext::SuppliedSource(_)
            | CfgMetadataContext::Unavailable => Numbers::Unknown,
        }
    }

    pub(super) fn resolved_tokens(
        &self,
        tokens: &CommandTokens,
    ) -> Option<crate::registry_invocation::ResolvedStatementInvocation> {
        crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
            self.registry.registry(),
            self.metadata_context()?.context(),
            tokens,
        )
    }

    pub(super) fn logical_structured_invocation(
        &self,
        tokens: &CommandTokens,
    ) -> Option<crate::registry_invocation::LogicalStructuredInvocation> {
        match &self.metadata {
            CfgMetadataContext::Supplied(context) => {
                crate::registry_invocation::logical_structured_invocation_with_metadata_context(
                    self.registry.registry(),
                    context.as_ref().into(),
                    tokens,
                    None,
                )
            }
            CfgMetadataContext::SuppliedSource(input) => {
                crate::registry_invocation::logical_structured_invocation_with_metadata_context(
                    self.registry.registry(),
                    crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
                        self.registry.registry(),
                        input,
                    )?,
                    tokens,
                    None,
                )
            }
            CfgMetadataContext::Standalone => {
                crate::registry_invocation::logical_structured_invocation(
                    self.registry.registry(),
                    tokens,
                    None,
                )
            }
            CfgMetadataContext::Unavailable => None,
        }
    }

    pub(super) fn conditional_source_completion_route(
        &self,
        tokens: &CommandTokens,
    ) -> Option<tcl_registry::completion_route::InvocationCompletionRoute> {
        let registry = self.registry.registry();
        let metadata = self
            .metadata_context()?
            .context()
            .filter(|context| context.source_analysis_input().is_some())?;
        if let Some(route) =
            crate::registry_invocation::original_native_source_conditional_completion(
                registry, metadata, tokens,
            )
        {
            return Some(route);
        }
        let invocation = crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(
            registry, metadata, tokens,
        )?;
        invocation.dialect?;
        Some(invocation.completion_route(registry))
    }

    pub(super) fn exact_completion(
        &self,
        stmt: &Statement,
        resolve: &dyn Fn(&str) -> Option<crate::ir_helpers::ResolvedEmbeddedHead>,
    ) -> Option<tcl_registry::registry::ExactInvocationCompletion> {
        exact_statement_completion(
            stmt,
            self.registry.registry(),
            resolve,
            self.metadata_context()?
                .context()
                .map(|context| context.context().authoring_query()),
        )
    }

    pub(super) fn exits_process(
        &self,
        stmt: &Statement,
        resolve: &dyn Fn(&str) -> Option<crate::ir_helpers::ResolvedEmbeddedHead>,
    ) -> bool {
        self.exact_completion(stmt, resolve)
            == Some(tcl_registry::registry::ExactInvocationCompletion::ProcessExit)
    }

    pub(super) fn completion_route(
        &self,
        stmt: &Statement,
    ) -> Option<tcl_registry::completion_route::InvocationCompletionRoute> {
        Self::proved_statement_command(stmt)?;
        let registry = self.registry.registry();
        self.resolved_tokens(stmt.tokens()?)
            .map(|invocation| invocation.completion_route(registry))
    }

    /// The completion codes `stmt` may produce, read from its registry route.
    /// An unproved command falls through. A process exit contributes no code;
    /// an abrupt route with no catch-visible code is classed as an error.
    pub(super) fn flow_completion(&self, stmt: &Statement) -> Completion {
        use tcl_registry::CompletionCode as Code;
        use tcl_registry::completion_route::InvocationCompletionRoute as Route;
        let Some(route) = self.completion_route(stmt) else {
            return Completion::exact(Code::Ok);
        };
        route
            .alternatives()
            .into_iter()
            .fold(Completion::bottom(), |joined, alternative| {
                let codes = match alternative {
                    Route::Unknown => Completion::top(),
                    Route::ProcessExit => Completion::bottom(),
                    Route::Return(_) => Completion::exact(Code::Return),
                    alternative => {
                        Completion::exact(alternative.immediate_code().unwrap_or(Code::Error))
                    }
                };
                joined.join(&codes)
            })
    }

    #[cfg(test)]
    pub(super) fn for_dialect(dialect: Option<&str>) -> Self {
        Self::from_registry(
            tcl_registry::model::ingress::static_context_for(dialect.unwrap_or("tcl")).commands(),
        )
    }

    #[cfg(test)]
    pub(super) fn is_block_terminating_command(&self, command: &str) -> bool {
        self.semantics
            .command(command.trim_start_matches(':'))
            .is_some_and(|facts| {
                facts.has_traits(Traits::TERMINATES_BLOCK)
                    && facts.lowering_hook() != Some(LoweringHookId::Return)
            })
    }

    #[cfg(test)]
    pub(super) fn is_tailcall_command(&self, command: &str) -> bool {
        self.semantics
            .command(command.trim_start_matches(':'))
            .is_some_and(|facts| facts.has_traits(Traits::REPLACES_FRAME))
    }

    #[cfg(test)]
    pub(super) fn is_loop_break_command(&self, name: &str) -> bool {
        self.semantics
            .command(name)
            .is_some_and(|facts| facts.has_traits(Traits::BREAKS_LOOP))
    }

    #[cfg(test)]
    pub(super) fn is_loop_continue_command(&self, name: &str) -> bool {
        self.semantics
            .command(name)
            .is_some_and(|facts| facts.has_traits(Traits::CONTINUES_LOOP))
    }
}

impl Default for CfgCommandClasses {
    fn default() -> Self {
        Self::from_registry(&CommandRegistry::build_default())
    }
}
