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

//! The original source occurrence kept for a variable write, receiver, formal or qualified name.

use super::super::state::Analyser;
use super::super::types::ScopeKind;
use super::scope_at;
use tcl_lexer::Span;

impl Analyser {
    pub(in super::super) fn record_original_variable_write_advice(
        &mut self,
        span: Span,
        scope_path: &[usize],
        form: tcl_registry::resolved_invocation::VariableReceiverOperandForm,
        conditional_metadata: Option<
            &crate::registry_invocation::OriginalConditionalRegistryMetadata,
        >,
    ) {
        let image = tcl_lexer::SourceImage::document(&self.source);
        let config = self.file_lexer_config();
        if let Some(original) = self
            .result
            .original_vendor_source_name_in_source(&image, config, span)
        {
            let Some(metadata) = self.original_vendor_variable_metadata(original) else {
                return;
            };
            let home =
                self.result
                    .original_vendor_variable_body_in_source(&image, config, span.start());
            let source_root = home.is_none()
                && self.body_depth == 0
                && scope_at(&self.result.global_scope, scope_path)
                    .is_some_and(|scope| scope.kind == ScopeKind::Global);
            let Some(advice) =
                crate::signature_scan::vendor_variable::VendorSourceVariableAdvice::written(
                    original,
                    span,
                    form,
                    home,
                    source_root,
                    metadata,
                )
            else {
                return;
            };
            if !self
                .result
                .original_vendor_variable_advice
                .contains(&advice)
            {
                self.result.original_vendor_variable_advice.push(advice);
            }
            return;
        }
        let Some(input) = self.original_qualified_variable_input(span, false) else {
            return;
        };
        let frame = self
            .head_identities
            .source_bindings_ref()
            .original_variable_frame_at_span(span, self.file_lexer_config())
            .or_else(|| {
                self.head_identities
                    .source_bindings_ref()
                    .original_variable_frame_at_offset(span.start(), self.file_lexer_config())
            });
        let namespace = scope_at(&self.result.global_scope, scope_path)
            .and_then(|scope| scope.naming_scope.clone());
        let context = self.analysis_context();
        let advice = crate::signature_scan::variable_symbol::OriginalVariableWriteAdvice::new(
            span,
            input,
            form,
            frame,
            namespace,
            conditional_metadata.cloned(),
            &context,
        );
        let same_card =
            |existing: &crate::signature_scan::variable_symbol::OriginalVariableWriteAdvice| {
                // Cheapest test first: most cards are at another span.
                existing.span() == advice.span()
                    && existing.receiver_form() == advice.receiver_form()
                    && existing.matches_context(&context)
                    && existing.original_name_input() == advice.original_name_input()
                    && existing.original_frame() == advice.original_frame()
                    && existing.original_namespace() == advice.original_namespace()
            };
        if let Some(existing) = self
            .result
            .original_variable_write_advice
            .iter_mut()
            .find(|existing| same_card(existing))
        {
            if existing.conditional_metadata().is_none() && advice.conditional_metadata().is_some()
            {
                *existing = advice;
            } else if advice.conditional_metadata().is_some() && existing != &advice {
                self.result.original_variable_write_advice.push(advice);
            }
        } else {
            self.result.original_variable_write_advice.push(advice);
        }
    }

    pub(in super::super) fn record_original_vendor_variable_body(
        &mut self,
        declaration_span: Span,
        body_span: Span,
        parameters_span: Option<Span>,
        kind: crate::signature_scan::vendor_variable::VendorSourceVariableBodyKind,
    ) {
        let image = tcl_lexer::SourceImage::document(&self.source);
        let config = self.file_lexer_config();
        let Some(declaration) =
            self.result
                .original_vendor_source_name_in_source(&image, config, declaration_span)
        else {
            return;
        };
        let Some(body) = self
            .result
            .original_vendor_source_name_in_source(&image, config, body_span)
        else {
            return;
        };
        let Some(metadata) = self.original_vendor_variable_metadata(body) else {
            return;
        };
        let parameters = parameters_span.and_then(|span| {
            self.result
                .original_vendor_source_name_in_source(&image, config, span)
        });
        let Some(receipt) = crate::signature_scan::vendor_variable::VendorSourceVariableBody::new(
            declaration,
            body,
            parameters,
            kind,
            metadata,
        ) else {
            return;
        };
        if !self
            .result
            .original_vendor_variable_bodies
            .contains(&receipt)
        {
            for advice in receipt.formal_advice() {
                if !self
                    .result
                    .original_vendor_variable_advice
                    .contains(&advice)
                {
                    self.result.original_vendor_variable_advice.push(advice);
                }
            }
            self.result.original_vendor_variable_bodies.push(receipt);
        }
    }

    pub(in super::super) fn original_vendor_variable_metadata(
        &self,
        original: &crate::signature_scan::vendor_name::VendorSourceNameOccurrence,
    ) -> Option<crate::registry_invocation::VendorRegistryInvocationShape> {
        let head = crate::signature_scan::vendor_name::VendorSourceNameInput::from_original_word(
            original.original_words().first()?,
            original.name_input().policy(),
        );
        crate::registry_invocation::vendor_registry_invocation_shape(
            &self.analysis_context(),
            &head,
            original.original_words(),
        )
    }

    pub(in super::super) fn record_original_variable_receiver(
        &mut self,
        span: Span,
        scope_path: &[usize],
        form: tcl_registry::resolved_invocation::VariableReceiverOperandForm,
        declaration: bool,
    ) {
        use crate::signature_scan::variable_symbol::{
            OriginalVariableSymbolPurpose, OriginalVariableSymbolReceiver,
        };
        if self.result.original_variable_alias_sites.contains(&span)
            || self
                .result
                .original_variable_symbol_conflicts
                .contains(&span)
        {
            return;
        }
        let Some(scope) = scope_at(&self.result.global_scope, scope_path) else {
            return;
        };
        let purpose = if matches!(
            scope.kind,
            ScopeKind::Global | ScopeKind::Namespace | ScopeKind::Uplevel
        ) {
            OriginalVariableSymbolPurpose::NamespaceStorage
        } else {
            OriginalVariableSymbolPurpose::QualifiedOnly
        };
        let namespace = scope.naming_scope.clone();
        let site = self
            .original_qualified_variable_input(span, false)
            .and_then(|input| {
                let namespace = namespace.or_else(|| {
                    let protocol = input.policy().recipe();
                    let tcl_syntax::naming::NativeVariableInputForm::Combined(bytes) =
                        form.input_form(protocol, input.bytes())?
                    else {
                        return None;
                    };
                    let selected = protocol.combined_variable_input(bytes);
                    (selected.root().qualification()
                        == tcl_syntax::naming::NativeNameQualification::Absolute)
                        .then(|| self.original_variable_root_namespace())
                        .flatten()
                });
                self.original_variable_occurrence(
                    span,
                    input,
                    namespace,
                    purpose,
                    OriginalVariableSymbolReceiver::Operand(form),
                    declaration,
                )
            });
        if site.is_none() && !self.result.original_variable_name_unknowns.contains(&span) {
            self.result.original_variable_name_unknowns.push(span);
        }
        if let Some(existing) = self
            .result
            .original_variable_symbols
            .iter_mut()
            .find(|existing| existing.span() == span)
        {
            if site
                .as_ref()
                .is_none_or(|site| !existing.merge_declaration(site))
            {
                self.result
                    .original_variable_symbols
                    .retain(|site| site.span() != span);
                self.result.original_variable_symbol_conflicts.push(span);
            }
        } else if let Some(site) = site {
            self.result.original_variable_symbols.push(site);
        }
    }

    pub(in super::super) fn record_original_formal_variables(&mut self) {
        let Some(policy) = self.declaration_name_policy() else {
            return;
        };
        let input = self.resolved_analysis_input();
        let occurrences = if input.has_logical_source_name_context() {
            self.head_identities
                .original_logical_variable_formal_occurrences(&input)
        } else {
            self.head_identities
                .source_bindings_ref()
                .original_variable_formal_occurrences(self.file_lexer_config())
        };
        for occurrence in occurrences {
            if occurrence.symbol().policy() != policy {
                continue;
            }
            if let Some(existing) = self
                .result
                .original_variable_symbols
                .iter_mut()
                .find(|existing| existing.span() == occurrence.span())
            {
                if !existing.merge_declaration(&occurrence) {
                    self.result
                        .original_variable_symbol_conflicts
                        .push(occurrence.span());
                }
            } else {
                self.result.original_variable_symbols.push(occurrence);
            }
        }
    }

    pub(super) fn original_variable_occurrence(
        &self,
        span: Span,
        input: crate::signature_scan::scope::SignatureSourceNameInput,
        namespace: Option<crate::signature_scan::scope::SignatureNamespaceScope>,
        purpose: crate::signature_scan::variable_symbol::OriginalVariableSymbolPurpose,
        receiver: crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver,
        declaration: bool,
    ) -> Option<crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence> {
        use crate::signature_scan::variable_symbol::{
            OriginalVariableSymbolPurpose, SignatureSourceVariableOccurrence,
        };
        let bindings = self.head_identities.source_bindings_ref();
        let mut aliases = self
            .result
            .original_variable_alias_receipts
            .iter()
            .filter(|alias| {
                self.registry.as_deref().is_some_and(|registry| {
                    bindings.original_variable_alias_matches_at_span(
                        span,
                        &input,
                        receiver,
                        alias,
                        self.file_lexer_config(),
                        registry,
                    )
                })
            });
        if let Some(alias) = aliases.next() {
            if aliases.any(|other| other != alias) {
                return None;
            }
            return Some(
                SignatureSourceVariableOccurrence::from_original_alias_input(
                    span,
                    input,
                    namespace,
                    receiver,
                    alias.clone(),
                ),
            );
        }
        if let Some(source_input) = self.result.resolved_input.as_ref()
            && source_input.has_logical_source_name_context()
        {
            return self.original_logical_variable_occurrence(
                source_input,
                span,
                input,
                receiver,
                declaration,
            );
        }

        if let Some(namespace) = &namespace
            && let Some(site) = SignatureSourceVariableOccurrence::from_original_input(
                span,
                input.clone(),
                namespace.clone(),
                purpose,
                receiver,
                declaration,
            )
        {
            if purpose == OriginalVariableSymbolPurpose::NamespaceStorage {
                return self.original_namespace_storage_occurrence(
                    span,
                    input,
                    namespace,
                    receiver,
                    declaration,
                    site,
                );
            }
            return Some(site);
        }
        (purpose == OriginalVariableSymbolPurpose::QualifiedOnly).then_some(())?;
        let (frame, simple) = self
            .head_identities
            .source_bindings_ref()
            .original_local_variable_symbol_at_span(
                span,
                &input,
                receiver,
                self.file_lexer_config(),
                self.registry.as_deref()?,
            )?;
        Some(
            SignatureSourceVariableOccurrence::from_original_local_input(
                span,
                input,
                namespace,
                frame,
                simple,
                receiver,
                declaration,
            ),
        )
    }

    fn original_logical_variable_occurrence(
        &self,
        source_input: &super::super::ResolvedAnalysisInput,
        span: Span,
        input: crate::signature_scan::scope::SignatureSourceNameInput,
        receiver: crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver,
        declaration: bool,
    ) -> Option<crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence> {
        use crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence;
        let image = tcl_lexer::SourceImage::document(&self.source);
        let scope = self.head_identities.original_logical_variable_scope_at(
            source_input,
            &image,
            span,
            &input,
        )?;
        let mut aliases = self
            .result
            .original_variable_alias_receipts
            .iter()
            .filter(|alias| scope.matches_alias_template(span, &input, receiver, alias));
        if let Some(alias) = aliases.next() {
            if aliases.any(|other| other != alias) {
                return None;
            }
            return SignatureSourceVariableOccurrence::from_logical_alias_input(
                span,
                input,
                receiver,
                alias.clone(),
                scope,
            );
        }
        SignatureSourceVariableOccurrence::from_logical_source(
            span,
            input,
            receiver,
            declaration,
            scope,
            &self.result.original_variable_alias_receipts,
        )
    }

    fn original_namespace_storage_occurrence(
        &self,
        span: Span,
        input: crate::signature_scan::scope::SignatureSourceNameInput,
        namespace: &crate::signature_scan::scope::SignatureNamespaceScope,
        receiver: crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver,
        declaration: bool,
        site: crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence,
    ) -> Option<crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence> {
        use crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence;
        let bindings = self.head_identities.source_bindings_ref();
        // Bare names in a namespace frame can select a global table
        // on C8. Only the independently retained point owns that hop.
        let protocol = input.policy().recipe();
        let selected = match receiver.input_form(&input)? {
            tcl_syntax::naming::NativeVariableInputForm::Combined(bytes) => {
                protocol.combined_variable_input(bytes)
            }
            tcl_syntax::naming::NativeVariableInputForm::Separate { root, element } => {
                protocol.separate_variable_input(root, element)
            }
        };
        if matches!(
            protocol.variable_root_geometry(namespace.context()?, selected.root().selected()),
            tcl_syntax::naming::NativeVariableRootGeometry::Local(_)
        ) {
            if let Some((selected_namespace, simple)) = bindings
                .original_namespace_variable_symbol_at_span(
                    span,
                    &input,
                    receiver,
                    self.file_lexer_config(),
                    self.registry.as_deref()?,
                )
            {
                return SignatureSourceVariableOccurrence::from_original_namespace_cell(
                    span,
                    input,
                    namespace.clone(),
                    &selected_namespace,
                    simple,
                    receiver,
                    declaration,
                );
            }
            let (frame, simple) = bindings.original_local_variable_symbol_at_span(
                span,
                &input,
                receiver,
                self.file_lexer_config(),
                self.registry.as_deref()?,
            )?;
            return Some(
                SignatureSourceVariableOccurrence::from_original_local_input(
                    span,
                    input,
                    Some(namespace.clone()),
                    frame,
                    simple,
                    receiver,
                    declaration,
                ),
            );
        }
        Some(site)
    }

    pub(in super::super) fn original_qualified_variable_input(
        &self,
        span: Span,
        substitution: bool,
    ) -> Option<crate::signature_scan::scope::SignatureSourceNameInput> {
        use crate::signature_scan::scope::SignatureSourceNameInput;
        let policy = self.declaration_name_policy()?;
        let config = self.file_lexer_config();
        if !substitution && self.selected_logical_declaration_advice() {
            let image = tcl_lexer::SourceImage::document(&self.source);
            return self
                .head_identities
                .original_written_word_at_span_in_source(&image, span, config)
                .and_then(|(word, _)| {
                    crate::signature_scan::scope::SignatureSourceNameKey::from_original_logical_source_word(
                        &word, self.word_rules(), policy,
                    )
                })
                .map(SignatureSourceNameInput::OriginalWord)
                .or_else(|| {
                    let (word, _) = self.head_identities.original_written_word_at_span_in_source(&image, span, config)?;
                    crate::signature_scan::scope::SignatureSourceNameValue::from_original_variable_operand_root(
                        &word, self.word_rules(), policy,
                    ).map(SignatureSourceNameInput::OriginalValue)
                })
                .or_else(|| self.original_static_variable_operand_input(span))
                .filter(|input| input.policy() == policy);
        }
        if !substitution {
            return self
                .head_identities
                .original_written_name_input_at_span(span, config)
                .or_else(|| self.original_static_variable_operand_input(span))
                .or_else(|| {
                    self.head_identities
                        .original_c_array_operand_root_at_span_in_source(
                            &tcl_lexer::SourceImage::document(&self.source),
                            span,
                            config,
                            policy,
                        )
                })
                .filter(|input| input.policy() == policy);
        }
        let image = tcl_lexer::SourceImage::document(&self.source);
        let start = usize::try_from(span.start()).ok()?;
        let reference =
            tcl_lexer::word_parts::scan_var_ref(image.bytes(), start, config).ok()??;
        (reference.source_span(image.bytes(), start, 0)? == span).then_some(())?;
        let raw_span = Span::new(span.start(), u32::try_from(reference.next).ok()?);
        let arena = tcl_lexer::ExecutablePartArena::decompose(
            image.clone(),
            raw_span,
            tcl_lexer::word_parts::SubstFlags::default(),
            config,
        )
        .ok()?;
        let root = if self.selected_logical_declaration_advice() {
            crate::signature_scan::variable_name::SignatureSourceVariableRoot::from_original_logical_source_executable(
                &arena, &image, config, span, self.word_rules(), policy,
            )
        } else {
            crate::signature_scan::variable_name::SignatureSourceVariableRoot::from_original_executable(
                &arena, &image, config, span, self.word_rules(), policy,
            )
        };
        root.map(SignatureSourceNameInput::OriginalVariableRoot)
    }
}
