// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original Registry and declared source syntax shared by naming and diagnostics.

use crate::registry_invocation::source_structure::{OriginalOperandSource, OriginalRegistryWords};
use std::sync::Arc;
use tcl_lexer::{NativeWord, Span, Token};

// The source-schema callback lends a different argv lifetime at each call.
// Free functions keep that lifetime late-bound independently of the retained
// Registry loan; inherent method items bind the invocation's impl lifetime.
pub(super) fn source_format_arguments(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> Option<Vec<tcl_registry::FormatStringArg>> {
    schema.authored_source_format_arguments()
}

pub(super) fn source_arity<'r>(
    schema: &tcl_registry::ResolvedInvocation<'r, '_>,
) -> Option<tcl_registry::AuthoredSourceArity<'r>> {
    schema.authored_source_arity()
}

pub(super) fn source_option_relationships<'r>(
    schema: &tcl_registry::ResolvedInvocation<'r, '_>,
) -> Option<tcl_registry::AuthoredSourceOptionRelationships<'r>> {
    schema.authored_source_option_relationships()
}

pub(super) fn source_lambda_call(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> Option<tcl_registry::AuthoredSourceLambdaCall> {
    schema.authored_source_lambda_call()
}

pub(super) fn source_variable_name_arguments(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> Vec<(usize, tcl_registry::ArgRole)> {
    schema.authored_source_variable_name_arguments()
}

pub(super) fn source_expression_arguments(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> Option<tcl_registry::AuthoredSourceExpressionArguments> {
    schema.authored_source_expression_arguments()
}

pub(super) fn source_descriptors<'r>(
    schema: &tcl_registry::ResolvedInvocation<'r, '_>,
) -> tcl_registry::AuthoredSourceDescriptors<'r> {
    schema.authored_source_descriptors()
}

pub(super) fn source_diagnostic_options<'r>(
    schema: &tcl_registry::ResolvedInvocation<'r, '_>,
) -> Option<tcl_registry::AuthoredSourceOptionScan<'r>> {
    schema.authored_source_diagnostic_options()
}

pub(super) fn source_clause_issue(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> Option<tcl_registry::ClauseShapeIssue> {
    schema.authored_source_clause_issue()
}

pub(super) fn source_subcommand_diagnostic(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> Option<tcl_registry::AuthoredSourceSubcommandDiagnostic> {
    schema.authored_source_subcommand_diagnostic()
}

pub(super) fn source_literal_validation(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> Option<tcl_registry::LiteralArgumentValidation> {
    schema.validate_literal_arguments()
}

pub(super) fn source_append_arguments(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> Option<tcl_registry::AuthoredSourceAppendArguments> {
    schema.authored_source_append_arguments()
}

pub(super) fn source_case_body_arguments(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> Option<Vec<tcl_registry::AuthoredSourceCaseBody>> {
    schema.authored_source_case_body_arguments()
}

pub(super) fn source_unset_option_only_arguments(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> Option<std::ops::Range<usize>> {
    schema.authored_source_unset_option_only_arguments()
}

pub(super) fn source_regex_pattern_arguments(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> Option<Vec<usize>> {
    Some(
        schema
            .authored_source_pattern_arguments()?
            .into_iter()
            .filter(|pattern| pattern.kind == tcl_registry::patterns::PatternType::Regex)
            .map(|pattern| usize::from(pattern.index))
            .collect(),
    )
}

pub(super) fn source_declares_regex_quoting(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> bool {
    // Authored transform metadata describes the intended source idiom only.
    // It cannot prove a result value or an entered successful handler.
    schema.semantics.taint_transform.is_some_and(|transform| {
        transform.condition.is_none()
            && transform
                .colour
                .contains(tcl_registry::TaintColour::REGEX_LITERAL)
    })
}

/// Syntax purpose retained independently of diagnostic messages and codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistrySourceDiagnosticKind {
    /// The selected clause grammar reports a source shape defect.
    ClauseShape,
    /// A selected descriptor restricts a lexical event context.
    ContextGate,
    /// A selected error-capture body omits its result operand.
    ErrorCapture,
    /// Registry-owned validation of a literal argument domain.
    LiteralArgument,
    /// A substitution written in a selected variable-name position.
    VariableName,
    /// An optional terminator before an original data operand.
    OptionTerminator,
    /// An option descriptor excluded by the actual availability context.
    DisabledOption,
    /// A literal selector absent or ambiguous in the selected table.
    Subcommand,
    /// Registry-owned deprecation advice for the selected command.
    DeprecatedCommand,
    /// Argument counts from a selected original source signature.
    Arity,
    /// Positive and complete-negative option relationship facts.
    ArgumentRelationship,
    /// An expression operand selected by the original source grammar.
    Expression,
    /// A selected command, selector, option or argument lifecycle.
    Lifecycle,
    /// A channel operand selected by the original argument grammar.
    ChannelArgument,
    /// Missing source requirement for the selected descriptor's package.
    PackageRequirement,
    /// Source reading order relative to an authenticated requirement.
    PackageOrdering,
    /// Original padded payload of the selected append source grammar.
    AppendList,
    /// Original unbraced action of the selected case-list source grammar.
    CaseBody,
    /// Selected dialect option grammar consumed every unset operand.
    OptionOnly,
    /// Live substitution in an original selected regular-expression pattern.
    PatternSubstitution,
}

/// A structured source diagnostic subject with original schema ownership.
/// It supplies no native handler, current value, completion or writable proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistrySourceDiagnosticSubject {
    words: Arc<OriginalRegistryWords>,
    kind: RegistrySourceDiagnosticKind,
    argument: Option<usize>,
    written_argument: Option<usize>,
    span: Span,
    owning_package: Option<&'static str>,
    required_package: Option<&'static str>,
    required_package_key: Option<tcl_registry::native_package::NativePackageNameKey>,
    supplemental_literals: Vec<(usize, String)>,
}
impl RegistrySourceDiagnosticSubject {
    /// Effective source operands whose advisory value is supplied by the
    /// actual SSA lattice. They do not replace the original written words or
    /// prove a native argument value, successful dispatch or completion.
    #[must_use]
    pub fn supplemental_literals(&self) -> &[(usize, String)] {
        &self.supplemental_literals
    }

    /// Exact source syntax and retained context, separate from execution.
    #[must_use]
    pub fn words(&self) -> &OriginalRegistryWords {
        &self.words
    }
    /// The emitting source-syntax purpose; never inferred from presentation.
    #[must_use]
    pub const fn kind(&self) -> RegistrySourceDiagnosticKind {
        self.kind
    }
    /// Effective post-head operand ordinal, or the original command head.
    #[must_use]
    pub const fn argument(&self) -> Option<usize> {
        self.argument
    }
    /// Original written post-head operand ordinal, absent for a head subject.
    #[must_use]
    pub const fn written_argument(&self) -> Option<usize> {
        self.written_argument
    }
    /// Selected owning package metadata, separate from package installation.
    #[must_use]
    pub const fn owning_package(&self) -> Option<&'static str> {
        self.owning_package
    }
    /// Required package from the actual selected Registry descriptor.
    #[must_use]
    pub const fn required_package(&self) -> Option<&'static str> {
        self.required_package
    }
    /// ASCII authored package metadata under the actual original Native naming
    /// policy. Logical/hosted inputs do not borrow a Native package recipe.
    #[must_use]
    pub fn required_package_key(
        &self,
    ) -> Option<&tcl_registry::native_package::NativePackageNameKey> {
        self.required_package_key.as_ref()
    }
    /// Original whole written extent selected by the emitter.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
}

/// One sealed source selection reused by each dispatch-site diagnostic.
/// Effective ordinals and original written operands are independent axes.
#[derive(Debug, Clone)]
pub(super) struct OriginalDiagnosticInvocation {
    words: Arc<OriginalRegistryWords>,
    context: Arc<tcl_registry::model::ContextRegistry>,
    supplemental_literals: Vec<(usize, String)>,
}
impl OriginalDiagnosticInvocation {
    pub(super) fn new(
        words: OriginalRegistryWords,
        context: Arc<tcl_registry::model::ContextRegistry>,
    ) -> Option<Self> {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        words.with_source_schema(&context, |_| ())?;
        words.head_source()?.word()?;
        Some(Self {
            words: Arc::new(words),
            context,
            supplemental_literals: Vec::new(),
        })
    }
    pub(super) fn with_schema<'r, T>(
        &'r self,
        project: impl FnOnce(&tcl_registry::ResolvedInvocation<'r, '_>) -> T,
    ) -> Option<T> {
        if self.supplemental_literals.is_empty() {
            return self.words.with_source_schema(&self.context, project);
        }
        let values = self
            .supplemental_literals
            .iter()
            .map(|(ordinal, value)| (*ordinal, value.as_str()))
            .collect::<Vec<_>>();
        self.words
            .with_source_value_projection(&self.context, &values, project)
    }
    /// Supplement only original, ordinary written operands at their retained
    /// effective ordinals; captures and expansions retain their own owner.
    pub(super) fn with_supplemental_literals(&self, literals: &[(usize, String)]) -> Option<Self> {
        let mut supplemental_literals = Vec::new();
        for (written, value) in literals {
            let indices = (0..self.words.arguments().len())
                .filter(|&index| self.written_index(index) == Some(*written))
                .collect::<Vec<_>>();
            let [effective] = indices.as_slice() else {
                return None;
            };
            self.word(*effective)?;
            if supplemental_literals
                .iter()
                .any(|(index, _)| index == effective)
            {
                return None;
            }
            supplemental_literals.push((*effective, value.clone()));
        }
        let projected = Self {
            words: Arc::clone(&self.words),
            context: Arc::clone(&self.context),
            supplemental_literals,
        };
        projected.with_schema(|_| ())?;
        Some(projected)
    }

    /// Effective original operands with separately retained advisory values.
    pub(super) fn supplemental_literals(&self) -> &[(usize, String)] {
        &self.supplemental_literals
    }
    pub(super) fn command(&self) -> &str {
        self.words.command()
    }
    pub(super) fn context(&self) -> &tcl_registry::model::ContextRegistry {
        &self.context
    }
    pub(super) fn words(&self) -> &OriginalRegistryWords {
        &self.words
    }
    pub(super) fn operand(&self, argument: usize) -> Option<&OriginalOperandSource> {
        let operand = self.words.operands().get(argument)?.as_ref()?;
        let word = operand.word()?;
        (!word.group().expand).then_some(operand)
    }
    pub(super) fn word(&self, argument: usize) -> Option<&NativeWord> {
        self.operand(argument)?.word()
    }
    pub(super) fn literal(&self, argument: usize) -> Option<&str> {
        self.operand(argument)?;
        if let Some((_, value)) = self
            .supplemental_literals
            .iter()
            .find(|(index, _)| *index == argument)
        {
            return Some(value);
        }
        std::str::from_utf8(self.words.arguments().get(argument)?.literal_bytes()?).ok()
    }
    /// Effective operand text and its genuine original first token. Captured
    /// operands retain their producer; expansion children have no whole word.
    pub(super) fn source_arguments(
        &self,
    ) -> Option<(Vec<String>, Vec<tcl_lexer::Token>, Vec<bool>)> {
        let mut text = Vec::new();
        let mut tokens = Vec::new();
        let mut single = Vec::new();
        for ordinal in 0..self.words.arguments().len() {
            let word = self.word(ordinal)?;
            let original = word.tokens().first().copied()?;
            let value = self.literal(ordinal).map(str::to_owned).or_else(|| {
                let span = word.content_span();
                std::str::from_utf8(word.image().bytes().get(span.as_range())?)
                    .ok()
                    .map(str::to_owned)
            })?;
            text.push(value);
            tokens.push(original);
            single.push(word.tokens().len() == 1);
        }
        Some((text, tokens, single))
    }
    pub(super) fn written_index(&self, argument: usize) -> Option<usize> {
        self.operand(argument)?;
        match self.words.origins().get(argument.checked_add(1)?)? {
            crate::registry_invocation::InvocationWordOrigin::Written(index) => {
                index.checked_sub(1)
            }
            _ => None,
        }
    }
    pub(super) fn head(&self) -> &NativeWord {
        self.words
            .head_source()
            .and_then(|operand| operand.word())
            .expect("constructor retains the original head")
    }
    pub(super) fn subject(
        &self,
        kind: RegistrySourceDiagnosticKind,
        argument: Option<usize>,
    ) -> Option<super::DiagnosticSubject> {
        let span = match argument {
            Some(argument) => self.word(argument)?.span(),
            None => self.head().span(),
        };
        Some(super::DiagnosticSubject::RegistrySource(Arc::new(
            RegistrySourceDiagnosticSubject {
                words: Arc::clone(&self.words),
                kind,
                argument,
                written_argument: argument.and_then(|index| self.written_index(index)),
                span,
                owning_package: self
                    .with_schema(source_descriptors)?
                    .command
                    .owning_package(),
                required_package: self
                    .with_schema(source_descriptors)?
                    .command
                    .required_package,
                required_package_key: self.required_package_key(),
                supplemental_literals: self.supplemental_literals.clone(),
            },
        )))
    }
    fn required_package_key(&self) -> Option<tcl_registry::native_package::NativePackageNameKey> {
        // naming.diagnostic.original-package-source-advice
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-package-source-advice.md
        let package = self
            .with_schema(source_descriptors)?
            .command
            .required_package?;
        package.is_ascii().then_some(())?;
        let input = self.words.head_source()?.input()?;
        Some(
            tcl_registry::native_package::NativePackageNameKey::from_native_units(
                package.as_bytes(),
                input.policy(),
            ),
        )
    }
    pub(super) fn expression_arguments(
        &self,
    ) -> Option<tcl_registry::AuthoredSourceExpressionArguments> {
        self.with_schema(source_expression_arguments).flatten()
    }
    pub(super) fn subject_range(
        &self,
        kind: RegistrySourceDiagnosticKind,
        arguments: std::ops::RangeInclusive<usize>,
    ) -> Option<super::DiagnosticSubject> {
        let start = *arguments.start();
        let end = *arguments.end();
        let first = self.word(start)?;
        let last = self.word(end)?;
        let written = self.written_index(start)?;
        for index in arguments {
            if self.written_index(index)? != written.checked_add(index.checked_sub(start)?)? {
                return None;
            }
        }
        Some(super::DiagnosticSubject::RegistrySource(Arc::new(
            RegistrySourceDiagnosticSubject {
                words: Arc::clone(&self.words),
                kind,
                argument: Some(start),
                written_argument: Some(written),
                span: Span::new(first.span().start(), last.span().end()),
                owning_package: self
                    .with_schema(source_descriptors)?
                    .command
                    .owning_package(),
                required_package: self
                    .with_schema(source_descriptors)?
                    .command
                    .required_package,
                required_package_key: self.required_package_key(),
                supplemental_literals: self.supplemental_literals.clone(),
            },
        )))
    }
    pub(super) fn invocation_span(&self) -> Span {
        let end = self
            .words
            .operands()
            .iter()
            .flatten()
            .filter_map(OriginalOperandSource::word)
            .map(|word| word.span().end())
            .max()
            .unwrap_or(self.head().span().end());
        Span::new(self.head().span().start(), end)
    }
    pub(super) fn count_for_arity(
        &self,
        arity: tcl_registry::Arity,
    ) -> Option<tcl_registry::InvocationArgumentCount> {
        self.with_schema(|schema| schema.authored_source_count_for_arity(arity))
            .flatten()
    }
    pub(super) fn subject_extent(
        &self,
        kind: RegistrySourceDiagnosticKind,
        span: Span,
    ) -> Option<super::DiagnosticSubject> {
        let whole = self.invocation_span();
        let argument = self.words.operands().iter().position(|operand| {
            operand
                .as_ref()
                .and_then(OriginalOperandSource::word)
                .is_some_and(|word| word.span().start() == span.start())
        });
        let ends_at_word = self.head().span().end() == span.end()
            || self
                .words
                .operands()
                .iter()
                .flatten()
                .filter_map(OriginalOperandSource::word)
                .any(|word| word.span().end() == span.end());
        if !(span == self.head().span()
            || span == whole
            || ((span.start() == self.head().span().start() || argument.is_some())
                && ends_at_word
                && whole.start() <= span.start()
                && span.end() <= whole.end()))
        {
            return None;
        }
        Some(super::DiagnosticSubject::RegistrySource(Arc::new(
            RegistrySourceDiagnosticSubject {
                words: Arc::clone(&self.words),
                kind,
                argument,
                written_argument: argument.and_then(|argument| self.written_index(argument)),
                span,
                owning_package: self
                    .with_schema(source_descriptors)?
                    .command
                    .owning_package(),
                required_package: self
                    .with_schema(source_descriptors)?
                    .command
                    .required_package,
                required_package_key: self.required_package_key(),
                supplemental_literals: self.supplemental_literals.clone(),
            },
        )))
    }
    pub(super) fn written_word(&self, index: usize) -> Option<&NativeWord> {
        let argument = (0..self.words.arguments().len())
            .find(|&argument| self.written_index(argument) == Some(index))?;
        self.word(argument)
    }
    pub(super) fn format_templates(&self) -> Vec<super::commands::OriginalFormatTemplate> {
        self.with_schema(source_format_arguments)
            .flatten()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|format| {
                let operand = self.operand(format.index)?;
                let bytes = self.literal(format.index)?.as_bytes();
                Some(super::commands::OriginalFormatTemplate {
                    format,
                    bytes: bytes.to_vec(),
                    span: operand.word()?.span(),
                })
            })
            .collect()
    }
}

/// Diagnostic purpose supplied by an authored document/workspace contract.
/// A declaration establishes neither a builtin implementation nor dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclaredSourceDiagnosticKind {
    /// An expression argument from the declaration's supplied optional slots.
    Expression,
    /// An argument count from the declaration's own finite signature.
    Arity,
    /// Variable-name purpose from only the declaration's supplied roles.
    VariableName,
    /// A channel operand from the declaration's supplied optional slots.
    ChannelArgument,
}

/// Genuine authored declaration and original invocation geometry.
/// Applicability and provenance stay in the retained receipt; no Registry
/// handler, concatenation semantics or successful execution are inherited.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredSourceDiagnosticSubject {
    words: Arc<crate::command_binding::OriginalDeclaredCommandWords>,
    kind: DeclaredSourceDiagnosticKind,
    argument: Option<usize>,
    span: Span,
}
impl DeclaredSourceDiagnosticSubject {
    /// Original source contract with full input, provenance and obligations.
    #[must_use]
    pub fn words(&self) -> &crate::command_binding::OriginalDeclaredCommandWords {
        &self.words
    }
    /// Authored diagnostic purpose, independent of presentation or codes.
    #[must_use]
    pub const fn kind(&self) -> DeclaredSourceDiagnosticKind {
        self.kind
    }
    /// Direct-written post-head argument ordinal, or an invocation/head subject.
    #[must_use]
    pub const fn argument(&self) -> Option<usize> {
        self.argument
    }
    /// Exact original whole-word or invocation extent.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
    pub(super) fn at_extent(
        words: Arc<crate::command_binding::OriginalDeclaredCommandWords>,
        kind: DeclaredSourceDiagnosticKind,
        span: Span,
    ) -> Option<super::DiagnosticSubject> {
        let original = words.original_words();
        let first = original.first()?;
        let last = original.last()?;
        if span.start() < first.span().start()
            || span.end() > last.span().end()
            || !original
                .iter()
                .any(|word| word.span().start() == span.start())
            || !original.iter().any(|word| word.span().end() == span.end())
        {
            return None;
        }
        let argument = original
            .iter()
            .position(|word| word.span().start() == span.start())
            .and_then(|index| index.checked_sub(1));
        Some(super::DiagnosticSubject::DeclaredSource(Arc::new(Self {
            words,
            kind,
            argument,
            span,
        })))
    }
}

/// Diagnostic source owners remain distinct: selected Registry grammar and
/// authored declaration roles each retain their own applicability receipt.
#[derive(Debug)]
pub(super) enum OriginalDiagnosticSource {
    Registry(OriginalDiagnosticInvocation),
    Declared(Arc<crate::command_binding::OriginalDeclaredCommandWords>),
}
impl OriginalDiagnosticSource {
    /// Effective source roles retain Registry or declaration ownership.
    /// Captured and expanded operands need independent written-origin mapping.
    pub(super) fn argument_roles(&self) -> Vec<(usize, tcl_registry::ArgRole)> {
        match self {
            Self::Registry(original) => original.words().roles().unwrap_or_default().to_vec(),
            Self::Declared(original) => original.supplied_argument_roles().unwrap_or_default(),
        }
    }
    pub(super) fn token(&self, argument: usize) -> Option<Token> {
        self.word(argument)?.tokens().first().copied()
    }
    pub(super) fn variable_name_arguments(&self) -> Vec<(usize, tcl_registry::ArgRole)> {
        match self {
            Self::Registry(original) => original
                .with_schema(source_variable_name_arguments)
                .unwrap_or_default(),
            Self::Declared(original) => original
                .supplied_argument_roles()
                .unwrap_or_default()
                .into_iter()
                .filter(|(_, role)| {
                    matches!(
                        role,
                        tcl_registry::ArgRole::VarRead | tcl_registry::ArgRole::VarWrite
                    )
                })
                .collect(),
        }
    }
    pub(super) fn variable_name_subject(
        &self,
        argument: usize,
    ) -> Option<super::DiagnosticSubject> {
        match self {
            Self::Registry(original) => {
                original.subject(RegistrySourceDiagnosticKind::VariableName, Some(argument))
            }
            Self::Declared(original) => DeclaredSourceDiagnosticSubject::at_extent(
                Arc::clone(original),
                DeclaredSourceDiagnosticKind::VariableName,
                original.argument_word(argument)?.span(),
            ),
        }
    }
    pub(super) fn display_command(&self, reporting_head: &str) -> String {
        match self {
            Self::Registry(original) => original
                .with_schema(|schema| {
                    schema.subcommand.resolved().map_or_else(
                        || reporting_head.to_owned(),
                        |subcommand| format!("{reporting_head} {}", subcommand.canonical_name),
                    )
                })
                .unwrap_or_else(|| reporting_head.to_owned()),
            Self::Declared(_) => reporting_head.to_owned(),
        }
    }

    pub(super) fn command(&self) -> &str {
        match self {
            Self::Registry(original) => original.command(),
            Self::Declared(original) => &original.descriptor().name,
        }
    }
    pub(super) fn arguments(&self) -> &[crate::registry_invocation::EffectiveInvocationWord] {
        match self {
            Self::Registry(original) => original.words().arguments(),
            Self::Declared(original) => original.arguments(),
        }
    }
    pub(super) fn word(&self, argument: usize) -> Option<&NativeWord> {
        match self {
            Self::Registry(original) => original.word(argument),
            Self::Declared(original) => original.argument_word(argument),
        }
    }
    pub(super) fn written_index(&self, argument: usize) -> Option<usize> {
        match self {
            Self::Registry(original) => original.written_index(argument),
            Self::Declared(original) => original.written_argument(argument),
        }
    }
    pub(super) fn registry(&self) -> Option<&OriginalDiagnosticInvocation> {
        match self {
            Self::Registry(original) => Some(original),
            Self::Declared(_) => None,
        }
    }
    pub(super) fn expression_arguments(
        &self,
    ) -> Option<tcl_registry::AuthoredSourceExpressionArguments> {
        match self {
            Self::Registry(original) => original.expression_arguments(),
            Self::Declared(original) => Some(tcl_registry::AuthoredSourceExpressionArguments {
                arguments: original
                    .supplied_argument_roles()?
                    .into_iter()
                    .filter_map(|(index, role)| {
                        (role == tcl_registry::ArgRole::Expr).then_some(index)
                    })
                    .collect(),
                concatenates: false,
            }),
        }
    }
    pub(super) fn expression_subject(
        &self,
        arguments: std::ops::RangeInclusive<usize>,
    ) -> Option<super::DiagnosticSubject> {
        match self {
            Self::Registry(original) => {
                original.subject_range(RegistrySourceDiagnosticKind::Expression, arguments)
            }
            Self::Declared(original) => {
                let first = original.argument_word(*arguments.start())?;
                let last = original.argument_word(*arguments.end())?;
                DeclaredSourceDiagnosticSubject::at_extent(
                    Arc::clone(original),
                    DeclaredSourceDiagnosticKind::Expression,
                    Span::new(first.span().start(), last.span().end()),
                )
            }
        }
    }
}
