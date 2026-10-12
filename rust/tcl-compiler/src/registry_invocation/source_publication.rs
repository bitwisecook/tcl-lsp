// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Source naming declarations without successful publication or lifetime.

use super::source_structure::OriginalRegistryWords;
use std::sync::Arc;
use tcl_lexer::NativeWord;
use tcl_registry::model::ContextRegistry;

/// Conditional source publication with the exact selected original naming word.
/// No command token, existence, native slot or successful creation is supplied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceCommandPublication {
    words: OriginalRegistryWords,
    naming: tcl_registry::AuthoredSourceCommandPublication,
    original: NativeWord,
    input: Option<crate::signature_scan::scope::SignatureSourceNameInput>,
    bytes: Arc<[u8]>,
}
impl OriginalSourceCommandPublication {
    pub(super) fn capture(words: OriginalRegistryWords, context: &ContextRegistry) -> Option<Self> {
        // naming.source.original-command-name-publications
        // docs/design/analysis/name-resolution-proofs/original-command-name-publications.md
        let naming = words.with_source_schema(context, |schema| {
            schema.authored_source_command_publication()
        })??;
        let operand = words.operands().get(naming.argument)?.as_ref()?;
        let original = operand.word()?.clone();
        let bytes = Arc::from(words.arguments().get(naming.argument)?.literal_bytes()?);
        let input = operand.input().cloned();
        Some(Self {
            words,
            naming,
            original,
            input,
            bytes,
        })
    }
    /// Same selected immutable source argv and its conditional applicability.
    #[must_use]
    pub const fn original_invocation(&self) -> &OriginalRegistryWords {
        &self.words
    }
    /// Genuine complete naming operand, including grouping, source and config.
    #[must_use]
    pub const fn original_name_word(&self) -> &NativeWord {
        &self.original
    }
    /// Native input only when its independent original source issuer supplied it.
    #[must_use]
    pub fn original_name_input(
        &self,
    ) -> Option<&crate::signature_scan::scope::SignatureSourceNameInput> {
        self.input.as_ref()
    }
    /// Exact source-produced naming value, independent of any command slot.
    #[must_use]
    pub fn name_bytes(&self) -> &[u8] {
        &self.bytes
    }
    /// Authored naming purpose, without installed implementation or class facts.
    #[must_use]
    pub const fn kind(&self) -> tcl_registry::AuthoredSourceCommandPublicationKind {
        self.naming.kind
    }
}

/// Genuine setter naming and whole single-command construction syntax.
/// This supplies no substituted result, stored variable or successful creation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceHandleConstruction {
    setter: OriginalRegistryWords,
    variable: NativeWord,
    variable_bytes: Arc<[u8]>,
    construction: tcl_lexer::NativeScriptCommandWords,
}
impl OriginalSourceHandleConstruction {
    pub(super) fn capture(
        setter: OriginalRegistryWords,
        context: &ContextRegistry,
    ) -> Option<Self> {
        let layout = setter.with_source_schema(context, |schema| {
            if schema.facts().arity_accepts_frozen_arguments() != Some(true) {
                return None;
            }
            schema
                .authored_source_descriptors()
                .command
                .binds_handle
                .copied()
        })??;
        let tcl_registry::handle_binding::HandleName::Word(variable) = layout.name_from else {
            return None;
        };
        let tcl_registry::handle_binding::HandleClassSource::ConstructionValue(value) =
            layout.class_from
        else {
            return None;
        };
        if let Some(keyword) = layout.keyword
            && setter
                .arguments()
                .get(usize::from(keyword.at))?
                .literal_bytes()
                != Some(keyword.word.as_bytes())
        {
            return None;
        }
        let ordinal = usize::from(variable);
        let variable = setter.operands().get(ordinal)?.as_ref()?.word()?.clone();
        let variable_bytes = Arc::from(setter.arguments().get(ordinal)?.literal_bytes()?);
        let operand = setter
            .operands()
            .get(usize::from(value))?
            .as_ref()?
            .word()?;
        let construction =
            crate::command_binding::original_single_command_substitution_words(operand)?
                .command()
                .clone();
        Some(Self {
            setter,
            variable,
            variable_bytes,
            construction,
        })
    }
    /// Selected original setter schema and conditional source applicability.
    #[must_use]
    pub const fn setter(&self) -> &OriginalRegistryWords {
        &self.setter
    }
    /// Complete original variable naming operand, without a frame/cell grant.
    #[must_use]
    pub const fn original_variable_word(&self) -> &NativeWord {
        &self.variable
    }
    /// Exact source-produced variable naming units.
    #[must_use]
    pub fn variable_bytes(&self) -> &[u8] {
        &self.variable_bytes
    }
    /// Whole original single-command substitution, preserving global geometry.
    #[must_use]
    pub const fn construction(&self) -> &tcl_lexer::NativeScriptCommandWords {
        &self.construction
    }
}

/// Original source handle assignment and selected factory naming declaration.
/// The conditional class label does not assert a stored value or live object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceHandleClassAdvice {
    setter: OriginalRegistryWords,
    variable: NativeWord,
    variable_bytes: Arc<[u8]>,
    factory: OriginalSourceCommandPublication,
}
impl OriginalSourceHandleClassAdvice {
    pub(super) fn capture(
        source: &str,
        analysis: &crate::analyser::types::AnalysisResult,
        setter: OriginalRegistryWords,
        context: &ContextRegistry,
    ) -> Option<Self> {
        let construction = OriginalSourceHandleConstruction::capture(setter, context)?;
        let offset = construction.construction.words.first()?.span().start();
        let factory =
            super::source_structure::source_command_publication_at(source, analysis, offset)?;
        if !matches!(
            factory.kind(),
            tcl_registry::AuthoredSourceCommandPublicationKind::Instance { .. }
        ) {
            return None;
        }
        Some(Self {
            setter: construction.setter,
            variable: construction.variable,
            variable_bytes: construction.variable_bytes,
            factory,
        })
    }
    /// Genuine selected setter argv, without a stored variable value.
    #[must_use]
    pub const fn setter(&self) -> &OriginalRegistryWords {
        &self.setter
    }
    /// Authentic whole variable operand, without a frame or variable cell.
    #[must_use]
    pub const fn original_variable_word(&self) -> &NativeWord {
        &self.variable
    }
    /// Source-produced variable naming value, independent of cell identity.
    #[must_use]
    pub fn variable_bytes(&self) -> &[u8] {
        &self.variable_bytes
    }
    /// Exact selected original factory and conditional naming declaration.
    #[must_use]
    pub const fn factory(&self) -> &OriginalSourceCommandPublication {
        &self.factory
    }
}

#[cfg(test)]
mod tests {
    use crate::analyser::Analyser;
    use crate::registry_invocation::source_structure::source_command_publication_at;

    #[test]
    fn source_publications_keep_original_factory_alias_and_refuse_shadowed_labels() {
        // naming.source.original-command-name-publications
        // docs/design/analysis/name-resolution-proofs/original-command-name-publications.md
        let source = "rename button widget; widget .b -text hello";
        let analysis = Analyser::new().analyse(source, "tk");
        let offset = u32::try_from(source.find("widget .b").unwrap()).unwrap();
        let publication = source_command_publication_at(source, &analysis, offset).unwrap();
        assert_eq!(publication.name_bytes(), b".b");
        assert_eq!(
            publication.kind(),
            tcl_registry::AuthoredSourceCommandPublicationKind::Instance {
                class_name: "button"
            }
        );
        assert_eq!(
            publication.original_name_word().image(),
            &tcl_lexer::SourceImage::document(source)
        );
        assert!(source_command_publication_at(&format!("#{source}"), &analysis, offset).is_none());
        let shadowed = "proc button args {}; button .b -text hello";
        let analysis = Analyser::new().analyse(shadowed, "tk");
        assert!(
            source_command_publication_at(
                shadowed,
                &analysis,
                u32::try_from(shadowed.find("button .b").unwrap()).unwrap()
            )
            .is_none()
        );
    }

    #[test]
    fn source_publications_refuse_unknown_options_expansion_and_missing_names() {
        // naming.source.original-command-name-publications
        // docs/design/analysis/name-resolution-proofs/original-command-name-publications.md
        for source in [
            "interp create $option child",
            "interp create {*}$options child",
            "interp create -safe",
            "coroutine {*}$names command",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl9.0");
            assert!(
                source_command_publication_at(source, &analysis, 0).is_none(),
                "{source}"
            );
        }
    }
}
