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

//! Lexical source headers of a declaration and its authored source tokens.

use super::super::SourceCommandBindings;
use super::{
    DeclarationLayoutIssuer, DeclarationLayoutObservation, OriginalDiagnosticFrameEntry,
    original_declaration_layouts,
};
use std::sync::Arc;
use tcl_names::command::CommandAllocationSite;

impl SourceCommandBindings {
    /// Logical source metadata meets all genuine pre-operand occurrences of a
    /// complete head. Runtime opacity is not replaced by a Native lookup.
    pub(crate) fn lexical_source_header_unpositioned(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        name: &str,
    ) -> Option<String> {
        // naming.minifier.logical-source-header
        // docs/design/analysis/name-resolution-proofs/minifier-logical-source-header.md
        if !self.matches_original_source_image(image, config) {
            return None;
        }
        let origin = self.root_origin.as_ref()?;
        let mut selected = None;
        for (site, observations) in &self.declaration_layouts {
            if &site.source != origin {
                continue;
            }
            let rows = original_declaration_layouts(observations)?;
            for row in rows {
                if row.config != config || !row.entry.owns_source(origin, site.offset) {
                    return None;
                }
                let words = crate::registry_invocation::original_native_compiler_words(
                    image,
                    &row.words,
                    site.offset,
                    config,
                )?;
                let values = words
                    .iter()
                    .map(|word| {
                        tcl_syntax::word_rules::original_static_word_ascii_presentation(word)
                    })
                    .collect::<Vec<_>>();
                if values.first()?.as_deref() != Some(name.as_bytes()) {
                    continue;
                }
                let state = &row.snapshot.state;
                if values.iter().any(Option::is_none)
                    || state.baseline.native_entry.is_some()
                    || state.baseline.execution_name_policy.is_some()
                    || state.baseline.hosted_execution_context.is_some()
                    || state.baseline.unknown_entry
                {
                    return None;
                }
                let binding = super::super::source_binding_projection_in(
                    state,
                    name,
                    &row.namespace,
                    super::super::CommandTargetLookup::NamedSlots,
                );
                let target = binding.proved_target()?;
                if !target.registry_backed || !target.prepended.is_empty() {
                    return None;
                }
                if selected
                    .as_ref()
                    .is_some_and(|prior| prior != &target.command)
                {
                    return None;
                }
                selected = Some(target.command.clone());
            }
        }
        selected
    }

    pub(in super::super) fn original_logical_source_header_rows(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        words: &[tcl_lexer::NativeWord],
    ) -> Option<(String, Arc<[DeclarationLayoutObservation]>)> {
        let image = words.first()?.image();
        let config = input.lexer_config();
        if self.original_logical_source_name_advice_input() != Some(input)
            || !self.matches_original_source_image(image, config)
        {
            return None;
        }
        let origin = self.root_origin.as_ref()?;
        let offset = words.first()?.tokens().first()?.span.start();
        let site = CommandAllocationSite {
            source: Arc::clone(origin),
            offset,
        };
        let rows = original_declaration_layouts(self.declaration_layouts.get(&site)?)?;
        let head = tcl_syntax::word_rules::original_static_word_ascii_presentation(words.first()?)?;
        let name = std::str::from_utf8(&head).ok()?;
        let mut selected = None;
        let mut retained = Vec::new();
        for row in rows {
            let original = crate::registry_invocation::original_native_compiler_words(
                image, &row.words, offset, config,
            )?;
            let state = &row.snapshot.state;
            if row.config != config
                || !row.entry.owns_source(origin, offset)
                || original.as_slice() != words
                || state.logical_source_name_advice_input() != Some(input)
            {
                return None;
            }
            let binding = super::super::source_binding_projection_in(
                state,
                name,
                &row.namespace,
                super::super::CommandTargetLookup::NamedSlots,
            );
            // Retain actual positioned alternatives even when the conditional
            // entry has unenumerated runtime lookup. A known user target,
            // captured prefix or no represented Registry target is terminal.
            if binding.targets.is_empty() {
                return None;
            }
            for target in &binding.targets {
                if !target.registry_backed
                    || !target.prepended.is_empty()
                    || selected
                        .as_ref()
                        .is_some_and(|prior| prior != &target.command)
                {
                    return None;
                }
                selected = Some(target.command.clone());
            }
            retained.push(row.clone());
        }
        Some((selected?, Arc::from(retained)))
    }

    /// Positioned Logical metadata keeps the exact original vector while
    /// retaining unknown operand values. This is the before-argv candidate,
    /// not a proof that argument effects preserve runtime dispatch.
    pub(crate) fn lexical_source_header_for_words(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        words: &[tcl_lexer::NativeWord],
    ) -> Option<String> {
        // naming.minifier.complete-logical-metadata
        // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
        if !self.matches_original_source_image(image, config) {
            return None;
        }
        let origin = self.root_origin.as_ref()?;
        let offset = words.first()?.tokens().first()?.span.start();
        let site = CommandAllocationSite {
            source: Arc::clone(origin),
            offset,
        };
        let rows = original_declaration_layouts(self.declaration_layouts.get(&site)?)?;
        let head = tcl_syntax::word_rules::original_static_word_ascii_presentation(words.first()?)?;
        let name = std::str::from_utf8(&head).ok()?;
        let mut selected = None;
        for row in rows {
            let original = crate::registry_invocation::original_native_compiler_words(
                image, &row.words, offset, config,
            )?;
            let state = &row.snapshot.state;
            if row.config != config
                || !row.entry.owns_source(origin, offset)
                || original.as_slice() != words
                || state.baseline.native_entry.is_some()
                || state.baseline.execution_name_policy.is_some()
                || state.baseline.hosted_execution_context.is_some()
                || state.baseline.unknown_entry
            {
                return None;
            }
            let binding = super::super::source_binding_projection_in(
                state,
                name,
                &row.namespace,
                super::super::CommandTargetLookup::NamedSlots,
            );
            let target = binding.proved_target()?;
            if !target.registry_backed
                || !target.prepended.is_empty()
                || selected
                    .as_ref()
                    .is_some_and(|prior| prior != &target.command)
            {
                return None;
            }
            selected = Some(target.command.clone());
        }
        selected
    }

    /// Explicit readonly Authored source purpose at an actual retained row.
    /// Raw source tokens and their authentic body/header own this projection;
    /// an existing stronger point/declaration owner is never erased or replaced.
    pub(crate) fn original_authored_declaration_source_tokens(
        &self,
        tokens: &crate::ir::CommandTokens,
        scope: &crate::registry_invocation::OriginalAuthoredSourceBodyScope,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<crate::ir::CommandTokens> {
        if let Some(binding) = tokens.source_binding.as_ref() {
            let original =
                binding.original_authored_source_body_scope_for_name_roles(tokens, registry)?;
            std::ptr::eq(original, scope).then_some(())?;
            let source_only = super::super::SourceInvocationBinding {
                original_analysis_owner: scope.original_analysis_attachment().cloned(),
                original_authored_source_body_scope: binding
                    .original_authored_source_body_scope
                    .clone(),
                ..Default::default()
            };
            (binding == &source_only).then_some(())?;
        }
        let original = scope.original_words(tokens, registry)?;
        let origin = self.source_origin()?;
        let owner = self.original_analysis_owner.as_ref()?;
        if self.lexer_config != Some(scope.config())
            || origin.source_image() != scope.original_body_word().image()
            || !Arc::ptr_eq(origin, &owner.source)
            || !owner.metadata.is_standalone()
            || self.source_metadata_owner() != &owner.metadata
            || self.final_state.baseline.registry_snapshot.as_ref()
                != Some(&registry.snapshot().semantic_key())
            || scope
                .original_analysis_attachment()
                .is_none_or(|actual| !Arc::ptr_eq(actual, owner))
        {
            return None;
        }
        let site = CommandAllocationSite {
            source: Arc::clone(origin),
            offset: original.first()?.span().start(),
        };
        let observations = self.declaration_layouts.get(&site)?;
        let rows = original_declaration_layouts(observations)?;
        let span = crate::registry_invocation::original_written_script_body_span(
            scope.original_body_word(),
        )?;
        let body_bytes = origin.source_image().bytes().get(span.as_range())?;
        for row in rows {
            let (parent, source, namespace) = match row.entry.as_ref() {
                OriginalDiagnosticFrameEntry::Body(body) => (
                    &body.allocation().site,
                    body.source(),
                    body.namespace_context()?,
                ),
                OriginalDiagnosticFrameEntry::DeclaredProcedure(body) => {
                    (body.declaration_site(), &body.source, &body.namespace)
                }
                _ => return None,
            };
            if parent != scope.original_parent_site()?
                || source.origin != site.source
                || source.base() != span.start()
                || source.text.bytes() != body_bytes
                || namespace != scope.namespace()
                || row.words.as_ref() != tokens.words()
                || row.config != scope.config()
                || !row.entry.owns_source(&site.source, site.offset)
                || !row.snapshot.state.baseline.metadata_context.is_standalone()
            {
                return None;
            }
        }
        let first = observations
            .iter()
            .find(|row| row.issuer == DeclarationLayoutIssuer::OriginalDeclaration)?;
        let binding = super::super::SourceInvocationBinding {
            unknown: true,
            dispatch_site: Some(site),
            original_analysis_owner: Some(Arc::clone(owner)),
            declaration_layout_observations: Some(Arc::from(observations.as_slice())),
            declaration_flow_inventory: Some(self.current_declaration_flow_inventory()),
            lookup_namespace: first.namespace.advisory_key()?,
            lookup_namespace_key: first.namespace.clone(),
            variable_context: Arc::clone(&first.snapshot.state.source_variables),
            variable_frame: first.snapshot.state.variable_frame.clone(),
            ..Default::default()
        };
        let mut result = tokens.clone();
        result.source_binding = Some(binding);
        Some(result)
    }
}
