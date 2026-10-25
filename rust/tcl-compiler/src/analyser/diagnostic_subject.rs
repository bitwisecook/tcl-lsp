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
    /// Original literal-head namespace source policy, never private-call proof.
    PrivateTclNamespace(Arc<SourcePrivateTclNamespaceSubject>),
    /// Conditional original user procedure signature and its source premises.
    UserSourceArity(Arc<SourceUserCallAritySubject>),
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

#[derive(Debug, Clone, PartialEq, Eq)]
enum OriginalUserCallOwner {
    NativeSource(crate::command_binding::OriginalSourceProcedureCall),
    Logical(crate::command_binding::OriginalLogicalProcedureCall),
}

/// An original conditional user procedure signature and its actual source argv.
/// Earlier uncertainty, lookup and body entry stay explicit; no Native call,
/// physical formal binding, completed result or source edit grant follows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceUserCallAritySubject {
    original: OriginalUserCallOwner,
    arity: tcl_registry::Arity,
    obligations: Vec<crate::command_binding::SourceCommandTransitionObligation>,
}

impl SourceUserCallAritySubject {
    pub(crate) fn authenticated_report_eq(
        &self,
        other: &Self,
        output: &super::authenticated_report::ReportOutputCorrespondence<'_>,
    ) -> Option<bool> {
        let Self {
            original: _,
            arity: _,
            obligations: _,
        } = self;
        let originals = match (&self.original, &other.original) {
            (OriginalUserCallOwner::NativeSource(a), OriginalUserCallOwner::NativeSource(b)) => {
                a.authenticated_report_eq(b, output)?
            }
            (OriginalUserCallOwner::Logical(a), OriginalUserCallOwner::Logical(b)) if a == b => {
                true
            }
            _ => return None,
        };
        Some(originals && self.arity == other.arity && self.obligations == other.obligations)
    }

    pub(crate) fn capture(
        source: &str,
        analysis: &super::AnalysisResult,
        offset: u32,
    ) -> Option<Self> {
        use crate::command_binding::SourceCommandTransitionObligation as Obligation;
        use crate::registry_invocation::source_structure::{
            source_logical_procedure_call_at, source_original_procedure_call_at,
        };
        let input = analysis.resolved_input.as_ref()?;
        let (original, arity, mut obligations, uncertain) =
            if input.has_logical_source_name_context() {
                let call = source_logical_procedure_call_at(source, analysis, offset)?;
                let arity = call
                    .declaration(analysis)?
                    .formal_count_projection()?
                    .arity();
                let uncertain = call.has_unresolved_applicability();
                (
                    OriginalUserCallOwner::Logical(call),
                    arity,
                    vec![Obligation::ExplicitLogicalSourceApplicability],
                    uncertain,
                )
            } else {
                let call = source_original_procedure_call_at(source, analysis, offset)?;
                let arity = call.declaration(analysis)?.metadata().arity();
                let mut obligations = call.declaration_advice().obligations().to_vec();
                if call.is_future_body() {
                    obligations.push(Obligation::FutureOriginalProcedureSourceApplicability);
                }
                let uncertain =
                    !call.uncertain_operations().is_empty() || call.has_lookup_operand_effects();
                (
                    OriginalUserCallOwner::NativeSource(call),
                    arity,
                    obligations,
                    uncertain,
                )
            };
        obligations.push(Obligation::UnavailableActualLookup);
        obligations.push(Obligation::OriginalProcedureBodyApplicability);
        if uncertain {
            obligations.push(Obligation::UnknownEarlierMutation);
        }
        Some(Self {
            original,
            arity,
            obligations,
        })
    }
    /// Complete actual written vector; captured slots cannot borrow its extents.
    #[must_use]
    pub fn original_words(&self) -> &[tcl_lexer::NativeWord] {
        match &self.original {
            OriginalUserCallOwner::NativeSource(call) => call.original_words(),
            OriginalUserCallOwner::Logical(call) => call.original_words(),
        }
    }
    /// Genuine declared formal count, before original alias-prefix binding.
    #[must_use]
    pub const fn declared_arity(&self) -> tcl_registry::Arity {
        self.arity
    }
    /// Retained effective captured argv count, not current written operands.
    #[must_use]
    pub fn captured_argument_count(&self) -> usize {
        match &self.original {
            OriginalUserCallOwner::NativeSource(call) => call.captured_argument_count(),
            OriginalUserCallOwner::Logical(call) => call.captured_argument_count(),
        }
    }
    /// Source applicability remains independent of a runtime invocation.
    #[must_use]
    pub fn obligations(&self) -> &[crate::command_binding::SourceCommandTransitionObligation] {
        &self.obligations
    }
    /// Explicit Logical source model, independently of Native source recipes.
    #[must_use]
    pub const fn is_logical_source(&self) -> bool {
        matches!(self.original, OriginalUserCallOwner::Logical(_))
    }
    /// Rejoin the same original call/allocation/header at its unchanged horizon.
    #[must_use]
    pub fn matches_source(&self, source: &str, analysis: &super::AnalysisResult) -> bool {
        self.original_words()
            .first()
            .and_then(|word| Self::capture(source, analysis, word.span().start()))
            .as_ref()
            == Some(self)
    }
}

#[cfg(test)]
mod user_source_call_controls {
    use super::*;
    use crate::analyser::{Analyser, ResolvedAnalysisInput};

    #[test]
    fn conditional_user_signature_keeps_real_alias_prefix_and_written_extents() {
        for logical in [false, true] {
            let source =
                "proc target {a b c} {}; interp alias {} selected {} target first second; selected";
            let profile = if logical {
                tcl_dialect::DialectProfile::plain_tcl()
            } else {
                tcl_dialect::DialectProfile::find("tcl8.6").unwrap()
            };
            let input = ResolvedAnalysisInput::new(
                profile,
                profile,
                tcl_registry::model::ingress::resolve_environment("tcl8.6")
                    .default_context_registry(),
                tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
            );
            let result = Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(source, "tcl");
            let subject = result
                .diagnostics
                .iter()
                .find_map(|diagnostic| diagnostic.user_source_arity())
                .unwrap();
            assert_eq!(subject.is_logical_source(), logical);
            assert_eq!(subject.captured_argument_count(), 2);
            assert_eq!(subject.declared_arity().min, 3);
            assert_eq!(subject.original_words().len(), 1);
            assert!(subject.matches_source(source, &result));
            assert!(!subject.matches_source(&format!("{source} changed"), &result));
            assert!(subject.obligations().contains(
                &crate::command_binding::SourceCommandTransitionObligation::UnavailableActualLookup
            ));
            let complete = Analyser::new()
                .with_resolved_input(input)
                .analyse(&format!("{source} third"), "tcl");
            assert!(
                !complete
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.user_source_arity().is_some())
            );
        }
    }

    #[test]
    fn literal_sigil_head_uses_original_word_and_deleted_target_cannot_reissue() {
        let source = "proc {$literal} {a b} {}; {$literal} one";
        let result = Analyser::new().analyse(source, "tcl8.6");
        let subject = result
            .diagnostics
            .iter()
            .find_map(|diagnostic| diagnostic.user_source_arity())
            .unwrap();
        assert_eq!(subject.original_words().len(), 2);
        assert!(subject.matches_source(source, &result));
        let deleted = "proc target {a b} {}; rename target {}; target one";
        let result = Analyser::new().analyse(deleted, "tcl8.6");
        assert!(
            SourceUserCallAritySubject::capture(
                deleted,
                &result,
                u32::try_from(deleted.rfind("target").unwrap()).unwrap()
            )
            .is_none()
        );
    }
}

/// Independent debts of private-namespace source advice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourcePrivateTclNamespaceObligation {
    /// Original source spelling has not proved a physical head value.
    OriginalHeadMaterialization,
    /// Namespace spelling/descriptor policy is not actual private handler entry.
    SourcePolicyApplicability,
    /// Neither the private nor public command has a current lookup proof.
    UnavailableActualLookup,
    /// A suggested public spelling may differ in values, effects and behaviour.
    PublicRewriteEquivalence,
}

/// Genuine original head and its distinct selected-schema or literal-source policy.
/// The latter deliberately supports unregistered helper spellings without
/// constructing a Registry invocation or asserting helper existence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcePrivateTclNamespaceSubject {
    head: tcl_lexer::NativeWord,
    input: super::ResolvedAnalysisInput,
    offset: u32,
    selected: Option<crate::registry_invocation::source_structure::OriginalRegistryWords>,
    classified_name: String,
    call: tcl_registry::private_tcl_namespaces::PrivateTclNamespaceCall,
}
impl SourcePrivateTclNamespaceSubject {
    pub(crate) fn from_original_source(
        head: tcl_lexer::NativeWord,
        input: super::ResolvedAnalysisInput,
        offset: u32,
        selected: Option<crate::registry_invocation::source_structure::OriginalRegistryWords>,
        classified_name: String,
        call: tcl_registry::private_tcl_namespaces::PrivateTclNamespaceCall,
    ) -> Self {
        Self {
            head,
            input,
            offset,
            selected,
            classified_name,
            call,
        }
    }
    /// Complete original written head; source bytes do not grant a Native name.
    #[must_use]
    pub const fn original_head(&self) -> &tcl_lexer::NativeWord {
        &self.head
    }
    /// Request position of the retained original parent head.
    #[must_use]
    pub const fn source_offset(&self) -> u32 {
        self.offset
    }
    /// Independently retained real selected source schema, if available.
    /// None is the explicit literal-head source policy, not a fake descriptor.
    #[must_use]
    pub fn selected_source_words(
        &self,
    ) -> Option<&crate::registry_invocation::source_structure::OriginalRegistryWords> {
        self.selected.as_ref()
    }
    /// Source policy spelling, never a runtime key or lookup request.
    #[must_use]
    pub fn classified_source_name(&self) -> &str {
        &self.classified_name
    }
    /// Existing Registry-owned namespace policy and conditional public suggestion.
    #[must_use]
    pub const fn policy(&self) -> &tcl_registry::private_tcl_namespaces::PrivateTclNamespaceCall {
        &self.call
    }
    /// Unresolved original-value, lookup and public-rewrite obligations.
    #[must_use]
    pub const fn obligations(&self) -> &'static [SourcePrivateTclNamespaceObligation] {
        &[
            SourcePrivateTclNamespaceObligation::OriginalHeadMaterialization,
            SourcePrivateTclNamespaceObligation::SourcePolicyApplicability,
            SourcePrivateTclNamespaceObligation::UnavailableActualLookup,
            SourcePrivateTclNamespaceObligation::PublicRewriteEquivalence,
        ]
    }
    /// Rejoin the same whole original source/input and original head occurrence.
    /// This supplies no selected runtime target or package installation.
    #[must_use]
    pub fn matches_source(&self, source: &str, analysis: &super::AnalysisResult) -> bool {
        let Some(input) = analysis.resolved_input.as_ref() else {
            return false;
        };
        if input != &self.input
            || self.head.config() != input.lexer_config()
            || self.head.image().bytes() != source.as_bytes()
            || !analysis.matches_original_source_image(self.head.image(), input.lexer_config())
        {
            return false;
        }
        let Some(realm) = analysis.retained_command_realm() else {
            return false;
        };
        if !realm.matches_resolved_analysis_input(input) {
            return false;
        }
        let Some((head, site)) = realm.original_written_head_at_span_in_source(
            self.head.image(),
            self.head.span(),
            input.lexer_config(),
        ) else {
            return false;
        };
        head == self.head
            && site.offset == self.offset
            && self.selected.as_ref().is_none_or(|words| {
                words.matches_source(self.head.image(), input.lexer_config())
                    && words.matches_registry(input.borrowed_context_registry().commands())
            })
    }
}
