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

//! The original tokens of one invocation and the registry metadata context it is resolved in.

use super::super::{Lowerer, LoweringMetadataOrigin, SourceEntryOrigin};
use crate::ir::CommandTokens;
use crate::lowering_hooks::ArgTokenKind;
use tcl_lexer::TokenType;
use tcl_source::segment::SegmentedCommand;

/// Map token kind to the simplified `ArgTokenKind` used by lowering hooks.
fn arg_token_kind(kind: TokenType) -> ArgTokenKind {
    match kind {
        TokenType::Str => ArgTokenKind::Str,
        TokenType::Esc => ArgTokenKind::Esc,
        TokenType::Cmd => ArgTokenKind::Cmd,
        TokenType::ExprSugar => ArgTokenKind::ExprSugar,
        TokenType::Var => ArgTokenKind::Var,
        _ => ArgTokenKind::Other,
    }
}

impl Lowerer<'_> {
    pub(in super::super) fn logical_structured_invocation(
        &self,
        tokens: &CommandTokens,
    ) -> Option<crate::registry_invocation::SourceStructuredInvocation> {
        if self.allows_standalone_registry_metadata() {
            return crate::registry_invocation::original_authored_control_invocation(
                tokens,
                self.registry,
            )
            .or_else(|| {
                crate::registry_invocation::original_authored_scalar_invocation(
                    tokens,
                    self.registry,
                )
                .map(|selected| {
                    crate::registry_invocation::SourceStructuredInvocation::Authored(Box::new(
                        selected,
                    ))
                })
            });
        }
        let metadata = self.invocation_metadata_context()?;
        if self.readonly_native_declaration_body.is_read_only() {
            return crate::registry_invocation::original_native_source_structured_invocation(
                self.registry,
                metadata,
                tokens,
            );
        }
        if metadata
            .source_analysis_input()
            .is_some_and(crate::analyser::ResolvedAnalysisInput::has_hosted_source_name_context)
        {
            return crate::registry_invocation::original_hosted_structured_invocation(
                self.registry,
                metadata,
                tokens,
            );
        }
        crate::registry_invocation::logical_structured_invocation_with_metadata_context(
            self.registry,
            self.invocation_metadata_context()?,
            tokens,
            self.source_bindings.as_deref(),
        )
        .map(|selected| {
            crate::registry_invocation::SourceStructuredInvocation::Logical(Box::new(selected))
        })
        .or_else(|| {
            crate::registry_invocation::original_logical_source_structured_invocation(
                self.registry,
                metadata,
                tokens,
            )
        })
        .or_else(|| {
            if self.target.is_bytecode() {
                return None;
            }
            crate::registry_invocation::original_native_source_structured_invocation(
                self.registry,
                metadata,
                tokens,
            )
        })
    }

    /// Construct the full source carrier outside recursive lowering frames.
    #[inline(never)]
    pub(super) fn cmd_tokens_boxed(&self, seg: &SegmentedCommand) -> Box<CommandTokens> {
        Box::new(self.cmd_tokens(seg))
    }

    /// Extract arg token kinds for the lowering hooks.
    pub(in super::super) fn arg_kinds(seg: &SegmentedCommand) -> Vec<ArgTokenKind> {
        seg.argv
            .iter()
            .skip(1)
            .map(|t| arg_token_kind(t.kind))
            .collect()
    }

    pub(super) fn retained_metadata_descriptor_label(
        &self,
        tokens: &CommandTokens,
    ) -> Option<String> {
        self.resolved_metadata_invocation(tokens)
            .map(|invocation| invocation.facts.descriptor_key().label().to_owned())
    }

    pub(in super::super) fn resolved_metadata_invocation(
        &self,
        tokens: &CommandTokens,
    ) -> Option<crate::registry_invocation::ResolvedStatementInvocation> {
        // Explicit standalone authoring has its own metadata context. An
        // unavailable supplied owner cannot enter the scalar compatibility API.
        match self.invocation_metadata_context() {
            Some(context) => {
                crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
                    self.registry,
                    Some(context),
                    tokens,
                )
            }
            None if self.allows_standalone_registry_metadata() => {
                crate::registry_invocation::resolved_tokens_invocation(self.registry, None, tokens)
            }
            None => None,
        }
    }

    pub(in super::super) fn allows_standalone_registry_metadata(&self) -> bool {
        self.source_entry_origin == SourceEntryOrigin::Authoring
            && self.metadata_origin == LoweringMetadataOrigin::Standalone
            && self.metadata_input.is_none()
            && self.logical_source_input.is_none()
            && self.vendor_source_input.is_none()
            && self.dialect_context.is_none()
    }

    pub(in super::super) fn source_metadata_input(
        &self,
    ) -> crate::registry_invocation::InvocationMetadataInput<'_> {
        use crate::registry_invocation::InvocationMetadataInput;
        if let Some(input) = self
            .metadata_input
            .as_ref()
            .or(self.logical_source_input.as_ref())
            .or(self.vendor_source_input.as_ref())
        {
            return InvocationMetadataInput::SuppliedSource(Some(input));
        }
        if let Some(context) = self.dialect_context.as_ref() {
            return InvocationMetadataInput::Supplied(context);
        }
        if self.allows_standalone_registry_metadata() {
            InvocationMetadataInput::Standalone
        } else {
            InvocationMetadataInput::SuppliedSource(None)
        }
    }

    pub(in super::super) fn invocation_metadata_context(
        &self,
    ) -> Option<crate::registry_invocation::InvocationMetadataContext<'_>> {
        if let Some(input) = self
            .metadata_input
            .as_ref()
            .or(self.logical_source_input.as_ref())
            .or(self.vendor_source_input.as_ref())
        {
            return crate::registry_invocation::InvocationMetadataContext::for_source_input(
                self.registry,
                input,
                self.config,
                self.dialect,
            );
        }
        match self.dialect_context.as_deref() {
            Some(context) => Some(context.into()),
            None if self.allows_standalone_registry_metadata() => self
                .dialect
                .or_else(|| self.registry.profile())
                .map(tcl_registry::model::semantic::SemanticContext::for_profile)
                .map(crate::registry_invocation::InvocationMetadataContext::from),
            None => None,
        }
    }
}
