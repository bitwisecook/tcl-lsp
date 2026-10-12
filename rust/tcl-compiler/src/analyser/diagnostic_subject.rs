// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Diagnostic identity retained independently of human-readable messages.

mod callback;
pub use callback::{
    SourceCallbackArgumentCounts, SourceCallbackArityIssue, SourceCallbackAritySubject,
    SourceCallbackSignatureLookup,
};

use crate::command_binding::{
    OriginalMathFunctionOccurrence, SourceCommandSlotPresence, SourceInvocationBinding,
};
use crate::signature_scan::{scope::SignatureSourceNameKey, types::SignatureCommandInvocation};
use std::sync::Arc;
use tcl_lexer::{LexerConfig, SourceImage};
use tcl_syntax::{naming::NamePolicyProtocol, word_rules::WordValueRules};

/// A genuine emitting source name and its independently retained lookup point.
/// Unsupported source producers do not acquire identity from display text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceUnresolvedCommandSubject {
    invocation: Arc<SourceInvocationBinding>,
    name_input: SignatureSourceNameKey,
    reporting_name: String,
}

impl SourceUnresolvedCommandSubject {
    pub(crate) fn from_original_invocation(
        invocation: SourceInvocationBinding,
        occurrence: &SignatureCommandInvocation,
        config: LexerConfig,
        rules: WordValueRules,
        policy: NamePolicyProtocol,
    ) -> Option<Self> {
        if occurrence.is_mathfunc_call {
            return None;
        }
        let site = invocation.invocation_site()?;
        let offset = occurrence.lookup.offset(occurrence.range)?;
        if site.offset != offset {
            return None;
        }
        let (word, name_input) = invocation.original_source_name_word_at_span(
            occurrence.range,
            config,
            rules,
            policy,
        )?;
        if word.legacy_text() != occurrence.name {
            return None;
        }
        let reporting_name = word.legacy_text();
        Some(Self {
            invocation: Arc::new(invocation),
            name_input,
            reporting_name,
        })
    }

    /// Immutable positioned lookup owner; target names are separate projections.
    #[must_use]
    pub fn invocation(&self) -> &SourceInvocationBinding {
        &self.invocation
    }

    /// Exact original static name units and their lexical producer.
    #[must_use]
    pub fn name_input(&self) -> &SignatureSourceNameKey {
        &self.name_input
    }

    /// Emitting source word's advisory text, independent of diagnostic phrasing.
    #[must_use]
    pub fn reporting_name(&self) -> &str {
        &self.reporting_name
    }
}

/// Genuine unresolved expression identifier, independent of command-head words.
/// Its conditional source context and diagnostic classification supply no
/// actual function dispatch, registration, evaluation or workspace loading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceUnresolvedMathFunctionSubject {
    occurrence: Arc<OriginalMathFunctionOccurrence>,
    presence: SourceCommandSlotPresence,
}

impl SourceUnresolvedMathFunctionSubject {
    pub(crate) fn from_original_occurrence(
        original: OriginalMathFunctionOccurrence,
        invocation: &SignatureCommandInvocation,
        image: &SourceImage,
        config: LexerConfig,
        registry: &tcl_registry::CommandRegistry,
        presence: SourceCommandSlotPresence,
    ) -> Option<Self> {
        // naming.diagnostic.original-math-function-subject
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-math-function-subject.md
        if !invocation.is_mathfunc_call
            || invocation.existence_probe
            || presence != SourceCommandSlotPresence::Absent
            || invocation.range != original.span()
            || invocation.name != original.function()
            || invocation.argc != Some(original.argument_count())
            || invocation.lookup.offset(invocation.range) != Some(original.span().start())
            || !original.matches_source(image, config, registry)
        {
            return None;
        }
        Some(Self {
            occurrence: Arc::new(original),
            presence,
        })
    }

    /// Authentic expression occurrence and full conditional source context.
    #[must_use]
    pub fn occurrence(&self) -> &OriginalMathFunctionOccurrence {
        &self.occurrence
    }

    /// Parser-retained original function identifier, independent of wording.
    #[must_use]
    pub fn reporting_name(&self) -> &str {
        self.occurrence.function()
    }

    /// Diagnostic-purpose function absence, without a completion prediction.
    #[must_use]
    pub const fn presence(&self) -> SourceCommandSlotPresence {
        self.presence
    }
}

/// Readonly source diagnostic purpose for a selected instance factory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisteredInstanceSourceDiagnosticKind {
    /// Original method keyword under the selected source table.
    MethodName,
    /// Frozen effective method argc under the selected source schema.
    Arity,
    /// Original option observations under the selected method schema.
    OptionRelation,
}

/// Original instance source ownership retained independently of display text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisteredInstanceSourceDiagnosticSubject {
    words: Arc<crate::command_binding::OriginalSourceRegisteredInstanceWords>,
    kind: RegisteredInstanceSourceDiagnosticKind,
}
impl RegisteredInstanceSourceDiagnosticSubject {
    pub(crate) fn new(
        words: Arc<crate::command_binding::OriginalSourceRegisteredInstanceWords>,
        kind: RegisteredInstanceSourceDiagnosticKind,
    ) -> Self {
        Self { words, kind }
    }
    /// Actual original instance argv, factory, handle and applicability.
    #[must_use]
    pub fn words(&self) -> &crate::command_binding::OriginalSourceRegisteredInstanceWords {
        &self.words
    }
    /// Independent diagnostic source purpose, without runtime selection.
    #[must_use]
    pub const fn kind(&self) -> RegisteredInstanceSourceDiagnosticKind {
        self.kind
    }
}

/// Retained source-only object signature purpose. The lexical provider and
/// complete effective call are independent of any entered receiver/frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectSourceAritySubject {
    /// A genuine original class factory and its selected construction argv.
    Constructor(Arc<super::types::OriginalConstructorArityAdvice>),
    /// A selected next-chain helper and its original declaring method body.
    LexicalNext(Arc<super::types::OriginalLexicalNextCall>),
}

/// Typed semantic subject supplied by the genuine diagnostic emitter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticSubject {
    /// Original callback prefix plus current readonly source-signature advice.
    CallbackSourceArity(Arc<SourceCallbackAritySubject>),
    /// Source declaration-owned object signature, without dispatch authority.
    ObjectSourceArity(Arc<ObjectSourceAritySubject>),
    /// Original source availability metadata, without an admitted handler schema.
    CommandAvailability(Arc<crate::registry_invocation::OriginalSourceCommandAvailability>),
    /// Authored document/workspace source contract, without builtin inheritance.
    DeclaredSource(Arc<super::DeclaredSourceDiagnosticSubject>),
    /// Conditional registered instance syntax with exact factory ownership.
    RegisteredInstanceSource(Arc<RegisteredInstanceSourceDiagnosticSubject>),
    /// Original Registry-selected source syntax; no execution or edit grant.
    RegistrySource(Arc<super::RegistrySourceDiagnosticSubject>),
    /// Conditional child source visibility with explicit original applicability.
    ConditionalInterpreterVisibility(Arc<super::ConditionalInterpreterVisibilitySubject>),
    /// Required package selected from Registry metadata under its naming policy.
    RequiredPackage(tcl_registry::native_package::NativePackageNameKey),
    /// Original unresolved name and positioned invocation, without presence proof.
    UnresolvedCommand(Arc<SourceUnresolvedCommandSubject>),
    /// Genuine expression identifier and its separate diagnostic lookup purpose.
    UnresolvedMathFunction(Arc<SourceUnresolvedMathFunctionSubject>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::{Analyser, AnalysisResult};

    fn subject_and_invocation(
        analysis: &AnalysisResult,
    ) -> (
        &SourceUnresolvedMathFunctionSubject,
        &SignatureCommandInvocation,
    ) {
        let subject = analysis
            .diagnostics
            .iter()
            .find_map(|diagnostic| diagnostic.unresolved_math_function())
            .expect("original unresolved expression function");
        let invocation = analysis
            .command_invocations
            .iter()
            .find(|invocation| {
                invocation.is_mathfunc_call && invocation.range == subject.occurrence().span()
            })
            .unwrap();
        (subject, invocation)
    }

    #[test]
    fn original_math_subject_rejects_foreign_source_and_full_parser_context() {
        // naming.diagnostic.original-math-function-subject
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-math-function-subject.md
        let source = "expr {Pi()}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let (subject, invocation) = subject_and_invocation(&analysis);
        let original = subject.occurrence();
        assert_eq!(
            original.frame().layout(),
            &crate::var_resolve::VariableExecutionFrame::Global
        );
        assert_eq!(
            original.expression_context().native_syntax,
            tcl_syntax::expr::parser::ExprParseContext::for_profile(
                tcl_dialect::DialectProfile::find("tcl8.6").unwrap()
            )
            .native_syntax
        );
        assert_eq!(original.namespace().advisory_key().as_deref(), Some("::"));
        let registry = analysis.resolved_registry().unwrap();
        let image = original.source_image();
        let config = original.lexer_config();
        let issue =
            |image: &SourceImage, config: LexerConfig, registry: &tcl_registry::CommandRegistry| {
                SourceUnresolvedMathFunctionSubject::from_original_occurrence(
                    original.clone(),
                    invocation,
                    image,
                    config,
                    registry,
                    SourceCommandSlotPresence::Absent,
                )
            };
        assert_eq!(issue(image, config, registry).as_ref(), Some(subject));
        for foreign in [
            SourceImage::document("expr {Po()}"),
            SourceImage::native(source.as_bytes()),
        ] {
            assert!(issue(&foreign, config, registry).is_none());
        }
        let foreign_config = LexerConfig {
            strict_quoting: !config.strict_quoting,
            ..config
        };
        assert!(issue(image, foreign_config, registry).is_none());
        let foreign_registry =
            registry.project_for_profile(tcl_dialect::DialectProfile::find("tcl9.1").unwrap());
        assert!(issue(image, config, &foreign_registry).is_none());
    }

    #[test]
    fn original_math_subject_rejects_command_words_and_missing_lookup_classification() {
        // naming.diagnostic.original-math-function-subject
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-math-function-subject.md
        let analysis = Analyser::new().analyse("expr {Pi()}", "tcl8.6");
        let (subject, invocation) = subject_and_invocation(&analysis);
        let original = subject.occurrence();
        let issue = |invocation: &SignatureCommandInvocation, presence| {
            SourceUnresolvedMathFunctionSubject::from_original_occurrence(
                original.clone(),
                invocation,
                original.source_image(),
                original.lexer_config(),
                analysis.resolved_registry().unwrap(),
                presence,
            )
        };
        for presence in [
            SourceCommandSlotPresence::Present,
            SourceCommandSlotPresence::MayPresent,
            SourceCommandSlotPresence::Unknown,
        ] {
            assert!(issue(invocation, presence).is_none());
        }
        let mut ordinary = invocation.clone();
        ordinary.is_mathfunc_call = false;
        let mut foreign_name = invocation.clone();
        foreign_name.name = "Po".to_owned();
        let mut foreign_span = invocation.clone();
        foreign_span.range = tcl_lexer::Span::new(7, 9);
        let mut foreign_arity = invocation.clone();
        foreign_arity.argc = Some(1);
        let mut probe = invocation.clone();
        probe.existence_probe = true;
        for foreign in [ordinary, foreign_name, foreign_span, foreign_arity, probe] {
            assert!(issue(&foreign, SourceCommandSlotPresence::Absent).is_none());
        }
        let diagnostic = analysis
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.unresolved_math_function().is_some())
            .unwrap();
        assert!(diagnostic.unresolved_command().is_none());
        assert!(diagnostic.required_package_key().is_none());
    }
}
