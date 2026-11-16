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

//! `AnalysisResult` queries over the original declarations it retains: symbols, namespace exports and patterns, vendor source names, procedures and their formals, classes.

use super::AnalysisResult;
use super::class_def::ClassDef;
use super::original_configurations::{
    OriginalSourceClassConfiguration, OriginalSourceObjectConfiguration,
};
use super::procedures::{DefinedSymbol, ProcDef};
use tcl_lexer::Span;

impl AnalysisResult {
    /// Source declaration cards from genuine original operands and selected
    /// Registry `SymbolDef` roles. No publication, entered handler, cell or edit
    /// capability is supplied by this immutable inventory.
    pub fn original_symbol_declarations(
        &self,
    ) -> impl Iterator<Item = &crate::signature_scan::symbol_name::OriginalSourceSymbolDeclaration>
    {
        self.original_symbol_metadata.iter()
    }

    /// Original export/clear events in source traversal order. These are readonly
    /// source naming advice, not export completion or live-import receipts.
    pub fn original_namespace_exports(
        &self,
    ) -> impl Iterator<Item = &crate::signature_scan::original_name::SourceNamespaceExport> {
        self.original_namespace_exports.iter()
    }

    /// May obligations for export operands without an original evaluated input.
    /// These spans provide no role, namespace, runtime or editable value proof.
    #[must_use]
    pub fn original_namespace_export_unknowns(&self) -> &[Span] {
        &self.original_namespace_export_unknowns
    }

    /// Every retained original import/forget pattern, in source traversal order.
    /// This is source assistance without import completion or namespace lifetime.
    pub fn original_namespace_patterns(
        &self,
    ) -> impl Iterator<Item = &crate::signature_scan::original_name::SourceNamespacePattern> {
        self.original_namespace_patterns.iter()
    }

    /// The analyser independently selected lexical-only declaration advice
    /// because no native name recipe was supplied. Byte-policy failures do not
    /// enable this compatibility surface; default-constructed results abstain.
    /// An independently retained hosted input remains outside this domain even
    /// when no complete source name occurrence can be retained.
    #[must_use]
    pub fn allows_lexical_declaration_advice(&self) -> bool {
        self.analysis_context_unavailable.is_none()
            && self.lexical_declaration_advice
            && !self.has_original_vendor_source_names()
            && self
                .resolved_input
                .as_ref()
                .is_some_and(super::super::ResolvedAnalysisInput::has_logical_source_name_context)
    }

    /// Positively retained Logical declaration advice for reporting consumers.
    /// Missing input, Native recipes and hosted source cannot borrow this domain
    /// from a copied flag or display profile. This grants no executed operation,
    /// current method-table closure, successful effect or writable identity.
    #[must_use]
    pub fn allows_retained_logical_declaration_advice(&self) -> bool {
        self.allows_lexical_declaration_advice()
    }

    /// Original hosted source producers remain present even when interpreted
    /// name units are unsupported. Consumers must not use reporting maps in
    /// place of these inputs or derive a C/Jim recipe from compatibility.
    pub fn original_vendor_source_names(
        &self,
    ) -> impl Iterator<Item = &crate::signature_scan::vendor_name::VendorSourceNameOccurrence> {
        self.original_vendor_source_names
            .values()
            .filter_map(Option::as_ref)
    }

    /// Whether this analysis owns hosted source names, including conflicting
    /// producers whose selected input has been withdrawn.
    #[must_use]
    pub fn has_original_vendor_source_names(&self) -> bool {
        !self.original_vendor_source_names.is_empty()
    }

    /// Exact original word at this consumer's source point. Registry name
    /// roles and purpose-specific materialisation are independent queries.
    #[must_use]
    pub fn original_vendor_source_name_in_source(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        span: tcl_lexer::Span,
    ) -> Option<&crate::signature_scan::vendor_name::VendorSourceNameOccurrence> {
        self.matches_original_source_image(image, config)
            .then_some(())?;
        let occurrence = self.original_vendor_source_names.get(&span)?.as_ref()?;
        occurrence
            .name_input()
            .matches_source(image, config)
            .then_some(occurrence)
    }

    /// Registry-selected hosted variable and formal source advice. These rows
    /// supply neither selected cells nor entered frames or runtime name units.
    pub fn original_vendor_variable_advice(
        &self,
    ) -> impl Iterator<Item = &crate::signature_scan::vendor_variable::VendorSourceVariableAdvice>
    {
        self.original_vendor_variable_advice.iter()
    }

    /// Authentic hosted source body at a cursor, independently of runtime
    /// activation and execution-domain state. Conflicting body owners abstain.
    #[must_use]
    pub fn original_vendor_variable_body_in_source(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        offset: u32,
    ) -> Option<&crate::signature_scan::vendor_variable::VendorSourceVariableBody> {
        self.matches_original_source_image(image, config)
            .then_some(())?;
        let contains =
            |body: &&crate::signature_scan::vendor_variable::VendorSourceVariableBody| {
                body.matches_source(image, config) && body.contains_cursor(offset)
            };
        let length = self
            .original_vendor_variable_bodies
            .iter()
            .filter(contains)
            .map(|body| body.span().end() - body.span().start())
            .min()?;
        let mut bodies = self
            .original_vendor_variable_bodies
            .iter()
            .filter(contains)
            .filter(|body| body.span().end() - body.span().start() == length);
        let first = bodies.next()?;
        bodies.all(|body| body == first).then_some(first)
    }

    /// Hosted procedure declaration advice from authentic source words and roles.
    pub fn original_vendor_procedure_declarations(
        &self,
    ) -> impl Iterator<
        Item = &crate::signature_scan::vendor_name::VendorSourceDeclarationMetadata<ProcDef>,
    > {
        self.original_vendor_procedure_metadata.iter()
    }

    /// Hosted class declaration advice, separate from runtime class allocations.
    pub fn original_vendor_class_declarations(
        &self,
    ) -> impl Iterator<
        Item = &crate::signature_scan::vendor_name::VendorSourceDeclarationMetadata<ClassDef>,
    > {
        self.original_vendor_class_metadata.iter()
    }

    /// Hosted Registry symbol declarations, including owned unsupported names.
    pub fn original_vendor_symbol_declarations(
        &self,
    ) -> impl Iterator<
        Item = &crate::signature_scan::vendor_name::VendorSourceDeclarationMetadata<DefinedSymbol>,
    > {
        self.original_vendor_symbol_metadata.iter()
    }

    /// Every independently retained original procedure declaration.
    pub fn original_procedure_declarations(
        &self,
    ) -> impl Iterator<Item = &crate::signature_scan::original_name::SourceDeclarationMetadata<ProcDef>>
    {
        self.original_procedure_metadata.iter()
    }

    /// Whether `declaration` equals a retained procedure row. A caller
    /// usually passes a retained row itself, which is found by address before
    /// any row is compared.
    pub(super) fn retains_original_procedure(
        &self,
        declaration: &crate::signature_scan::original_name::SourceDeclarationMetadata<ProcDef>,
    ) -> bool {
        self.original_procedure_metadata
            .iter()
            .any(|row| std::ptr::eq(row, declaration))
            || self.original_procedure_metadata.contains(declaration)
    }

    /// Original byte formal topology from this exact retained procedure
    /// declaration and unanimous Registry-selected declaration recipe.
    /// Reporting parameter names supply no fallback or activation authority.
    #[must_use]
    pub fn original_procedure_formals(
        &self,
        declaration: &crate::signature_scan::original_name::SourceDeclarationMetadata<ProcDef>,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<crate::signature_scan::formal_parameters::SignatureSourceFormalParameters> {
        if !self.retains_original_procedure(declaration)
            || !self.matches_original_source_image(
                declaration.name_input().source_image(),
                declaration.name_input().lexer_config(),
            )
        {
            return None;
        }
        self.command_realm
            .as_ref()?
            .source_bindings_ref()
            .original_procedure_formals_at(declaration.declaration_site(), registry)
    }

    /// Conditional Native source formal names at one retained declaration.
    /// These names and spans provide no installed procedure, frame or cell.
    #[must_use]
    pub fn conditional_native_procedure_formal_name_sources(
        &self,
        declaration: &crate::signature_scan::original_name::SourceDeclarationMetadata<ProcDef>,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<Vec<crate::signature_scan::formal_parameters::OriginalSourceFormalName>> {
        if !self
            .original_procedure_metadata
            .iter()
            .any(|row| std::ptr::eq(row, declaration))
            || !self.matches_original_source_image(
                declaration.name_input().source_image(),
                declaration.name_input().lexer_config(),
            )
        {
            return None;
        }
        let input = self.resolved_input.as_ref()?;
        let realm = self.command_realm.as_ref()?;
        if !realm.matches_resolved_analysis_input(input) {
            return None;
        }
        let name = crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(
            declaration.name_input().clone(),
        );
        realm
            .source_bindings_ref()
            .conditional_native_procedure_formal_names_at(
                declaration.declaration_site(),
                &name,
                registry,
                input,
            )
    }

    /// Readonly formal names at this exact original declaration. Native topology
    /// and the independently selected Logical source recipe remain separate.
    #[must_use]
    pub fn original_procedure_formal_name_sources(
        &self,
        declaration: &crate::signature_scan::original_name::SourceDeclarationMetadata<ProcDef>,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<Vec<crate::signature_scan::formal_parameters::OriginalSourceFormalName>> {
        if !self.retains_original_procedure(declaration)
            || !self.matches_original_source_image(
                declaration.name_input().source_image(),
                declaration.name_input().lexer_config(),
            )
        {
            return None;
        }
        let input = self.resolved_input.as_ref()?;
        if input.has_logical_source_name_context() {
            self.command_realm
                .as_ref()?
                .source_bindings_ref()
                .original_logical_formal_name_sources_at(
                    declaration.declaration_site(),
                    registry,
                    input,
                )
        } else {
            self.original_procedure_formals(declaration, registry)
                .map(|formals| formals.name_sources())
        }
    }

    /// Every independently retained original class declaration.
    pub fn original_class_declarations(
        &self,
    ) -> impl Iterator<Item = &crate::signature_scan::original_name::SourceDeclarationMetadata<ClassDef>>
    {
        self.original_class_metadata.iter()
    }

    /// Original class configurations retain their own target lookup obligations.
    /// These deltas do not inherit a class identity from reporting maps.
    pub fn original_class_configurations(
        &self,
    ) -> impl Iterator<Item = &OriginalSourceClassConfiguration> {
        self.original_class_configuration_metadata.iter()
    }

    /// Own-object source deltas retain their independently selected instance.
    /// Consumers must prove a current allocation join; walk order is not a clock.
    pub fn original_object_configurations(
        &self,
    ) -> impl Iterator<Item = &OriginalSourceObjectConfiguration> {
        self.original_object_configuration_metadata.iter()
    }

    /// Byte-slot declaration candidates in local-before-root lookup order.
    /// Every declaration at the first matching slot is retained; this metadata
    /// query does not select a temporal installation or reached implementation.
    #[must_use]
    pub fn procedures_for_original_name<'a>(
        &'a self,
        input: &crate::signature_scan::scope::SignatureSourceNameInput,
        namespace: &crate::signature_scan::scope::SignatureNamespaceScope,
    ) -> Vec<&'a ProcDef> {
        let Some(lookup) = crate::signature_scan::scope::SignatureSourceLookup::from_input(
            namespace.clone(),
            input,
        ) else {
            return Vec::new();
        };
        lookup.first_matching_publications(
            self.original_procedure_metadata
                .iter()
                .map(|record| (record.name(), record.metadata())),
        )
    }

    /// Actual retained naming geometry of the innermost source scope. Missing
    /// geometry cannot be reconstructed from its UI namespace label.
    #[must_use]
    pub fn original_namespace_scope_at(
        &self,
        offset: u32,
    ) -> Option<&crate::signature_scan::scope::SignatureNamespaceScope> {
        super::super::scope::original_namespace_scope_at(&self.global_scope, offset)
    }
}
