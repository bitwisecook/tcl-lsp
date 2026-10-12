// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Hosted source declaration cards, separate from native names and lookup.

use std::ops::ControlFlow;
use tcl_compiler::analyser::types::DefinedSymbol;
use tcl_compiler::analyser::{AnalysisResult, ClassDef, ProcDef};
use tcl_compiler::signature_scan::vendor_name::{
    VendorSourceDeclarationMetadata, VendorSourceNameInput,
};
use tcl_lexer::{LexerConfig, SourceImage, Span};
use tcl_syntax::naming::VendorSourceNamePurpose;

enum Kind<'a> {
    Procedure(&'a VendorSourceDeclarationMetadata<ProcDef>),
    Class(&'a VendorSourceDeclarationMetadata<ClassDef>),
    Symbol(&'a VendorSourceDeclarationMetadata<DefinedSymbol>),
}

/// A current source declaration header selected independently of UI maps.
/// This card supplies no canonical command slot, cell, invocation or edit.
pub struct OriginalVendorDeclaration<'a> {
    kind: Kind<'a>,
}

impl<'a> OriginalVendorDeclaration<'a> {
    /// Complete original hosted input, including unavailable materialisation.
    #[must_use]
    pub fn input(&self) -> &'a VendorSourceNameInput {
        match self.kind {
            Kind::Procedure(row) => row.name_input(),
            Kind::Class(row) => row.name_input(),
            Kind::Symbol(row) => row.name_input(),
        }
    }
    /// Independently selected source role; no native recipe is donated.
    #[must_use]
    pub fn purpose(&self) -> VendorSourceNamePurpose {
        match self.kind {
            Kind::Procedure(row) => row.purpose(),
            Kind::Class(row) => row.purpose(),
            Kind::Symbol(row) => row.purpose(),
        }
    }
    /// Actual original declaration word extent, without an editable grant.
    #[must_use]
    pub fn span(&self) -> Span {
        self.input().span()
    }
    /// Immutable procedure source metadata, if this is a procedure header.
    #[must_use]
    pub fn procedure_metadata(&self) -> Option<&'a ProcDef> {
        match self.kind {
            Kind::Procedure(row) => Some(row.metadata()),
            _ => None,
        }
    }
    /// Immutable class source metadata, if this is a class header.
    #[must_use]
    pub fn class_metadata(&self) -> Option<&'a ClassDef> {
        match self.kind {
            Kind::Class(row) => Some(row.metadata()),
            _ => None,
        }
    }
    /// Immutable selected Registry symbol metadata, if this is a symbol header.
    #[must_use]
    pub fn symbol_metadata(&self) -> Option<&'a DefinedSymbol> {
        match self.kind {
            Kind::Symbol(row) => Some(row.metadata()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
enum RetainedKind {
    Procedure(VendorSourceDeclarationMetadata<ProcDef>),
    Class(VendorSourceDeclarationMetadata<ClassDef>),
    Symbol(VendorSourceDeclarationMetadata<DefinedSymbol>),
}

/// An immutable retained hosted header for readonly indexed source advice.
/// It retains the original producer even when materialised units are unknown;
/// it supplies no command publication, namespace, cell, lookup or edit grant.
#[derive(Debug, Clone)]
pub struct RetainedOriginalVendorDeclaration {
    kind: RetainedKind,
}

impl RetainedOriginalVendorDeclaration {
    /// Borrow the same current-source header interface used by local consumers.
    #[must_use]
    pub fn declaration(&self) -> OriginalVendorDeclaration<'_> {
        OriginalVendorDeclaration {
            kind: match &self.kind {
                RetainedKind::Procedure(row) => Kind::Procedure(row),
                RetainedKind::Class(row) => Kind::Class(row),
                RetainedKind::Symbol(row) => Kind::Symbol(row),
            },
        }
    }
}

/// Retain headers only after the shared whole-source/configuration check.
/// The index separately retains the selected Registry and document URI.
#[must_use]
pub fn retained_declarations(
    source: &str,
    analysis: &AnalysisResult,
) -> Option<Vec<RetainedOriginalVendorDeclaration>> {
    Some(
        declarations(source, analysis)?
            .into_iter()
            .map(|card| RetainedOriginalVendorDeclaration {
                kind: match card.kind {
                    Kind::Procedure(row) => RetainedKind::Procedure(row.clone()),
                    Kind::Class(row) => RetainedKind::Class(row.clone()),
                    Kind::Symbol(row) => RetainedKind::Symbol(row.clone()),
                },
            })
            .collect(),
    )
}

/// Current direct hosted source headers. Unsupported units remain represented
/// by their actual source spelling; calls and namespaces are not resolved.
#[must_use]
pub fn declarations<'a>(
    source: &str,
    analysis: &'a AnalysisResult,
) -> Option<Vec<OriginalVendorDeclaration<'a>>> {
    let image = SourceImage::document(source);
    let config = analysis.body_lexer_config?;
    if !analysis.has_original_vendor_source_names()
        || !analysis.matches_original_source_image(&image, config)
    {
        return None;
    }
    let mut rows = Vec::new();
    rows.extend(
        analysis
            .original_vendor_procedure_declarations()
            .map(|row| OriginalVendorDeclaration {
                kind: Kind::Procedure(row),
            }),
    );
    rows.extend(analysis.original_vendor_class_declarations().map(|row| {
        OriginalVendorDeclaration {
            kind: Kind::Class(row),
        }
    }));
    rows.extend(analysis.original_vendor_symbol_declarations().map(|row| {
        OriginalVendorDeclaration {
            kind: Kind::Symbol(row),
        }
    }));
    if rows
        .iter()
        .any(|row| !row.input().matches_source(&image, config))
    {
        return None;
    }
    Some(rows)
}

/// Direct declaration cursor selection only. An owned unsupported input stays
/// terminal at its header; missing runtime lookup is never supplied by a label.
#[must_use]
pub fn select_at_offset<'a>(
    source: &str,
    analysis: &'a AnalysisResult,
    offset: u32,
) -> ControlFlow<Option<OriginalVendorDeclaration<'a>>> {
    if !analysis.has_original_vendor_source_names() {
        return ControlFlow::Continue(());
    }
    let Some(rows) = declarations(source, analysis) else {
        return ControlFlow::Break(None);
    };
    let mut selected = rows
        .into_iter()
        .filter(|row| row.span().start() <= offset && offset < row.span().end());
    let Some(first) = selected.next() else {
        return ControlFlow::Continue(());
    };
    ControlFlow::Break(selected.next().is_none().then_some(first))
}

/// A source card label from selected literal units or its exact original
/// content spelling when those units are unavailable. This is presentation.
#[must_use]
pub fn source_label(
    input: &VendorSourceNameInput,
    purpose: VendorSourceNamePurpose,
) -> Option<String> {
    if let Some(units) = input.literal_units(purpose) {
        return std::str::from_utf8(units).ok().map(str::to_owned);
    }
    let span = input.original_word().content_span().ok()?;
    std::str::from_utf8(input.source_image().bytes().get(span.as_range())?)
        .ok()
        .map(str::to_owned)
}

/// Render only supported hosted literal source units under the full original
/// grammar. A fresh shared parse must reproduce the same vendor units. This
/// suggests source syntax, without live command, name or native edit authority.
#[must_use]
pub fn source_word(
    input: &VendorSourceNameInput,
    purpose: VendorSourceNamePurpose,
    config: LexerConfig,
) -> Option<String> {
    if input.lexer_config() != config {
        return None;
    }
    let units = input.literal_units(purpose)?;
    let text = std::str::from_utf8(units).ok()?;
    let spelling = tcl_syntax::word_rules::lexical_ascii_source_word(text, config)?;
    let image = SourceImage::document(&spelling);
    let parsed = tcl_lexer::native_script_words_in(
        image.clone(),
        Span::new(0, u32::try_from(image.len()).ok()?),
        config,
    )
    .ok()?;
    let [command] = parsed.commands.as_slice() else {
        return None;
    };
    let [word] = command.words.as_slice() else {
        return None;
    };
    (tcl_syntax::naming::vendor_source_literal_units(input.policy(), word, purpose)? == units)
        .then_some(spelling)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn original_vendor_cards_preserve_owned_unknown_units_without_reporting_maps() {
        // Implementation contract: naming.vendor.original-source-declaration-consumers
        // docs/design/analysis/name-resolution-proofs/vendor-original-source-declaration-consumers.md
        let source = "proc helper {argument} {return $argument}\nproc {p\\uD800} {} {}\n";
        let mut analysis = Analyser::new().analyse(source, "f5-iapps");
        assert!(analysis.has_original_vendor_source_names());
        assert!(!analysis.allows_lexical_declaration_advice());
        assert_eq!(analysis.original_procedure_declarations().count(), 0);
        analysis.all_procs.clear();
        analysis.global_scope.procs.clear();
        let rows = declarations(source, &analysis).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(
            source_label(rows[0].input(), rows[0].purpose()).as_deref(),
            Some("helper")
        );
        assert_eq!(
            source_word(
                rows[0].input(),
                rows[0].purpose(),
                analysis.body_lexer_config.unwrap()
            )
            .as_deref(),
            Some("helper")
        );
        assert_eq!(
            source_label(rows[1].input(), rows[1].purpose()).as_deref(),
            Some("p\\uD800")
        );
        assert!(rows[1].input().literal_units(rows[1].purpose()).is_none());
        assert!(
            source_word(
                rows[1].input(),
                rows[1].purpose(),
                analysis.body_lexer_config.unwrap()
            )
            .is_none()
        );
        let offset = u32::try_from(source.find("p\\uD800").unwrap()).unwrap();
        assert!(matches!(
            select_at_offset(source, &analysis, offset),
            ControlFlow::Break(Some(_))
        ));
        assert!(declarations(&format!("# displaced\n{source}"), &analysis).is_none());
        let native = Analyser::new().analyse(source, "tcl8.6");
        assert!(matches!(
            select_at_offset(source, &native, offset),
            ControlFlow::Continue(())
        ));
    }
    #[test]
    fn original_vendor_event_cards_use_selected_metadata_and_current_whole_source() {
        // Implementation contract: naming.vendor.original-source-declaration-consumers
        // docs/design/analysis/name-resolution-proofs/vendor-original-source-declaration-consumers.md
        let source = "when HTTP_REQUEST {set local 1}\n";
        let mut analysis = Analyser::new().analyse(source, "f5-irules");
        assert_eq!(analysis.original_vendor_symbol_declarations().count(), 1);
        analysis.all_defined_symbols.clear();
        analysis.global_scope.defined_symbols.clear();
        let rows = declarations(source, &analysis).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(
            source_label(rows[0].input(), rows[0].purpose()).as_deref(),
            Some("HTTP_REQUEST")
        );
        assert_eq!(
            rows[0].symbol_metadata().unwrap().kind,
            tcl_registry::DefinedSymbolKind::Event
        );
        assert!(declarations("when HTTP_RESPONSE {}\n", &analysis).is_none());
        let invalid = Analyser::new().analyse("when HTTP_REQUEST set", "f5-irules");
        assert!(
            invalid
                .original_vendor_symbol_declarations()
                .next()
                .is_none()
        );
    }
}
