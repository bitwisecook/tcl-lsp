// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Positioned original declaration grammar for source navigation.

use super::{
    CommandRegistry, CommandTokens, EffectiveInvocationWord, InvocationWord,
    RegistryInvocationResolution, effective_invocation_word, effective_words_for_target,
    frozen_argument_words, resolve_registry_words_in_realm,
};

/// A declaration name and its original written argument. This is source
/// navigation advice, never a successful alias, physical cell or editable
/// reference to an unobserved target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclarationArgument {
    /// Position among the original written post-head arguments.
    pub argument: usize,
    /// Local alias name supplied by the selected declaration grammar.
    pub name: String,
}

/// A genuine written argv receiver selected by the original handler grammar.
/// This establishes naming geometry only, never a successful read or write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclarationVariableReceiver {
    /// Position among the original written post-head arguments.
    pub argument: usize,
    /// Runtime receiver input form, independently of whole-array requirements.
    pub form: tcl_registry::resolved_invocation::VariableReceiverOperandForm,
}

/// Selected local alias-name operation, independent of target lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclarationVariableAliasPurpose {
    /// Local name from the global-variable declaration formatter.
    Global,
    /// Local name from the namespace-variable declaration formatter.
    NamespaceVariable,
    /// Ordinary upvar local operand.
    Upvar,
    /// Namespace upvar's independently selected local operand.
    NamespaceUpvar,
}

/// Original written operands of one selected active alias declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclarationVariableAlias {
    /// Exact original written local-name argument.
    pub local: usize,
    /// Exact original written target-name argument.
    pub target: usize,
    /// Independent local-name operation selected by Registry transition data.
    pub purpose: DeclarationVariableAliasPurpose,
}

/// Unanimous grammar of an unchanged original declaration-local invocation.
/// Alias prefixes, member selection and source origins share one mapping.
/// Runtime dispatch, callbacks and body entry remain independent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalDeclarationAssistance {
    /// Selected original alias declarations requiring their own byte receipts.
    pub variable_alias_declarations: usize,
    /// Active alias operands retaining their exact original written positions.
    pub variable_alias_operands: Vec<DeclarationVariableAlias>,
    /// Original receivers with independently selected native input forms.
    pub variable_receivers: Vec<DeclarationVariableReceiver>,
    /// Original variable-role operands without an independently selected
    /// naming form. A role cannot donate ordinary combined lookup semantics.
    pub variable_name_obligations: Vec<usize>,
    /// Names with editable original declaration operands.
    pub declarations: Vec<DeclarationArgument>,
    /// Original written arguments containing authored script bodies.
    pub body_arguments: Vec<usize>,
    /// Original written arguments containing lambda lists.
    pub lambda_arguments: Vec<usize>,
}

type LogicalAliasNameObligation = (
    tcl_lexer::Span,
    Option<tcl_registry::VariableAliasDestination>,
);

fn original_logical_alias_name_obligations(
    words: &super::source_structure::OriginalRegistryWords,
    context: &tcl_registry::model::ContextRegistry,
) -> Option<Vec<LogicalAliasNameObligation>> {
    Some(match words.source() {
        super::source_structure::OriginalRegistrySource::SourceTransitions(advice)
            if advice.logical_source_input().is_some() =>
        {
            words.with_source_schema(context, |schema| {
                let Some(transitions) = schema.authored_logical_source_state_transitions() else {
                    if schema
                        .semantics
                        .traits
                        .contains(tcl_registry::Traits::CREATES_SCOPE_ALIAS)
                    {
                        return advice
                            .original_words()
                            .first()
                            .map(|head| vec![(head.span(), None)])
                            .unwrap_or_default();
                    }
                    return Vec::new();
                };
                let mut obligations = Vec::new();
                for fact in transitions.facts() {
                    let tcl_registry::StateTransition::VariableCellAlias(alias) = &fact.transition
                    else {
                        continue;
                    };
                    let variable = match &alias.target {
                        tcl_registry::VariableAliasTarget::Global { variable }
                        | tcl_registry::VariableAliasTarget::CurrentNamespace { variable }
                        | tcl_registry::VariableAliasTarget::CallerSelectedFrame {
                            variable, ..
                        }
                        | tcl_registry::VariableAliasTarget::Namespace { variable, .. } => variable,
                    };
                    let local = written_alias_operand(words, &alias.local);
                    let target = written_alias_operand(words, variable);
                    if local.is_some()
                        && target.is_some()
                        && !matches!(alias.local, tcl_registry::TransitionSubject::Unknown { .. })
                        && !matches!(variable, tcl_registry::TransitionSubject::Unknown { .. })
                    {
                        continue;
                    }
                    let span = target
                        .or(local)
                        .or_else(|| advice.original_words().first())
                        .map(tcl_lexer::NativeWord::span);
                    if let Some(span) = span {
                        let obligation = (span, Some(alias.destination));
                        if !obligations.contains(&obligation) {
                            obligations.push(obligation);
                        }
                    }
                }
                obligations
            })?
        }
        _ => Vec::new(),
    })
}

/// Original source operands of alias grammar, retaining conditional schema
/// applicability independently of executed alias installation or target cells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OriginalSourceVariableAliasOperands {
    words: super::source_structure::OriginalRegistryWords,
    operands: Vec<(
        tcl_lexer::Span,
        tcl_lexer::Span,
        DeclarationVariableAliasPurpose,
    )>,
    logical_declarations: Option<Vec<tcl_registry::VariableCellAliasTransition>>,
    conditional_no_level:
        Option<std::sync::Arc<tcl_registry::resolved_invocation::ConditionalNoLevelAliasLayout>>,
    logical_name_obligations: Vec<(
        tcl_lexer::Span,
        Option<tcl_registry::VariableAliasDestination>,
    )>,
}
impl OriginalSourceVariableAliasOperands {
    pub(crate) fn capture(
        words: super::source_structure::OriginalRegistryWords,
        context: &tcl_registry::model::ContextRegistry,
    ) -> Option<Self> {
        // Source alias operands describe the relationship if the selected
        // handler completes successfully. The ordinary transition may remain
        // unknown (for example, optional upvar level presence in Tcl 8.4/8.5).
        // This projection supplies no normal edge, installed alias or cell.
        let selected = words.with_source_schema(context, |schema| {
            schema
                .semantics
                .state_transitions
                .resolve_after_success_with_effect_coverage(schema.words.arguments())
                .0
        });
        let transitions = selected?;
        // The independent Logical frame grammar may retain an alias whose
        // ordinary transition only widens the variable domains. Preserve its
        // unresolved original names without donating a link or alias operand.
        let logical_name_obligations = original_logical_alias_name_obligations(&words, context)?;
        // Source frame applicability is independent of ordinary alias facts.
        // Even an ordinary alias may be inactive in this actual source frame.
        let logical_declarations = match words.source() {
            super::source_structure::OriginalRegistrySource::SourceTransitions(advice)
                if advice.logical_source_input().is_some() =>
            {
                words
                    .with_source_schema(context, |schema| {
                        schema.authored_logical_source_variable_aliases()
                    })
                    .flatten()
            }
            _ => None,
        };
        let conditional_no_level = match words.source() {
            super::source_structure::OriginalRegistrySource::SourceTransitions(advice)
                if advice.logical_source_input().is_some() =>
            {
                words
                    .with_source_schema(context, |schema| {
                        schema.conditional_no_level_source_alias_layout()
                    })
                    .flatten()
                    .map(std::sync::Arc::new)
            }
            _ => None,
        };
        let mut operands = Vec::new();
        for fact in transitions.facts() {
            let tcl_registry::StateTransition::VariableCellAlias(alias) = &fact.transition else {
                continue;
            };
            let (target, purpose) = match &alias.target {
                tcl_registry::VariableAliasTarget::Global { variable } => {
                    (variable, DeclarationVariableAliasPurpose::Global)
                }
                tcl_registry::VariableAliasTarget::CurrentNamespace { variable } => {
                    (variable, DeclarationVariableAliasPurpose::NamespaceVariable)
                }
                tcl_registry::VariableAliasTarget::CallerSelectedFrame { variable, .. } => {
                    (variable, DeclarationVariableAliasPurpose::Upvar)
                }
                tcl_registry::VariableAliasTarget::Namespace { variable, .. } => {
                    (variable, DeclarationVariableAliasPurpose::NamespaceUpvar)
                }
            };
            let Some(local) = written_alias_operand(&words, &alias.local) else {
                continue;
            };
            let Some(target) = written_alias_operand(&words, target) else {
                continue;
            };
            let operand = (local.span(), target.span(), purpose);
            if !operands.contains(&operand) {
                operands.push(operand);
            }
        }
        (!operands.is_empty()
            || logical_declarations
                .as_ref()
                .is_some_and(|aliases| !aliases.is_empty())
            || conditional_no_level.is_some()
            || !logical_name_obligations.is_empty())
        .then_some(Self {
            words,
            operands,
            logical_declarations,
            conditional_no_level,
            logical_name_obligations,
        })
    }

    /// Unknown original alias names in the genuine Logical source frame.
    /// An inactive procedure-local declaration remains a no-op. Missing frame
    /// correspondence keeps the obligation; it cannot establish alias coverage.
    pub(crate) fn unresolved_logical_name_spans(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        realm: &crate::realm::CommandBindingRealm,
    ) -> Option<Vec<tcl_lexer::Span>> {
        let super::source_structure::OriginalRegistrySource::SourceTransitions(advice) =
            self.words.source()
        else {
            return None;
        };
        if advice.logical_source_input() != Some(input)
            || !input.has_logical_source_name_context()
            || !self.matches_source_context(
                advice.site().source.source_image(),
                input.lexer_config(),
                input.borrowed_context_registry(),
            )
            || !realm.matches_resolved_analysis_input(input)
            || !realm.matches_original_source_image(
                advice.site().source.source_image(),
                input.lexer_config(),
            )
        {
            return None;
        }
        let scope = self.logical_scope(input, realm);
        let frame = logical_alias_frame(scope.as_ref());
        self.words
            .with_source_schema(input.borrowed_context_registry(), |schema| {
                self.logical_name_obligations
                    .iter()
                    .filter_map(|&(span, destination)| {
                        (destination.and_then(|destination| {
                            schema.authored_logical_source_alias_destination_is_active_in_frame(
                                destination,
                                frame,
                            )
                        }) != Some(false))
                        .then_some(span)
                    })
                    .collect()
            })
    }

    /// Conditional declarations from the independently selected Logical source
    /// frame. Ordinary aliases, normal-edge state and coverage stay separate.
    pub(crate) fn logical_declarations(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        realm: &crate::realm::CommandBindingRealm,
    ) -> Option<Vec<OriginalLogicalAliasDeclaration>> {
        let scope = self.logical_scope(input, realm)?;
        let frame = logical_alias_frame(Some(&scope));
        if self.logical_declarations.is_none() && self.conditional_no_level.is_none() {
            return None;
        }
        let declarations = self.logical_declarations.as_deref().unwrap_or_default();
        let mut result: Vec<_> =
            self.words
                .with_source_schema(input.borrowed_context_registry(), |schema| {
                    declarations
                        .iter()
                        .filter_map(|alias| {
                            (schema.authored_logical_source_alias_destination_is_active_in_frame(
                                alias.destination,
                                frame,
                            ) == Some(true))
                            .then_some(())?;
                            let local = written_alias_operand(&self.words, &alias.local)?;
                            let value =
                                tcl_syntax::word_rules::original_static_word_unicode_value(local)?;
                            // A derived tail has a different source receiver. This facet
                            // admits only the actual independently written local name.
                            (alias.local.literal()?.as_bytes() == value).then_some(())?;
                            Some(OriginalLogicalAliasDeclaration {
                                source: self.clone(),
                                input: input.clone(),
                                alias: OriginalLogicalAliasKind::Known(alias.clone()),
                                local: local.clone(),
                                scope: scope.clone(),
                            })
                        })
                        .collect()
                })?;
        if let Some(card) = &self.conditional_no_level {
            // These names are only a finite source branch. No actual level,
            // completed local link or successful argument evaluation is issued.
            for (ordinal, (_, local_subject)) in card.pairs().iter().enumerate() {
                let local = written_alias_operand(&self.words, local_subject)?;
                let value = tcl_syntax::word_rules::original_static_word_unicode_value(local)?;
                if local_subject.literal()?.as_bytes() != value {
                    return None;
                }
                if logical_alias_frame(Some(&scope)) != tcl_registry::VariableAliasFrame::Procedure
                {
                    continue;
                }
                result.push(OriginalLogicalAliasDeclaration {
                    source: self.clone(),
                    input: input.clone(),
                    alias: OriginalLogicalAliasKind::ConditionalNoLevel {
                        card: std::sync::Arc::clone(card),
                        ordinal,
                    },
                    local: local.clone(),
                    scope: scope.clone(),
                });
            }
        }
        Some(result)
    }

    /// A newer source alias with an unavailable local cannot prove that an
    /// older relationship survives. This is a veto, never alias continuity.
    pub(crate) fn blocks_logical_template(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        realm: &crate::realm::CommandBindingRealm,
        alias: &crate::signature_scan::variable_symbol::OriginalVariableAliasReceipt,
        anchor: tcl_lexer::Span,
    ) -> bool {
        let Some(offset) = self.invocation_offset() else {
            return true;
        };
        if offset <= alias.invocation_offset() || offset > anchor.start() {
            return false;
        }
        let Some(scope) = self.logical_scope(input, realm) else {
            return true;
        };
        if scope.alias_frame().as_ref() != Some(alias.frame()) {
            return false;
        }
        let frame = logical_alias_frame(Some(&scope));
        self.words
            .with_source_schema(input.borrowed_context_registry(), |schema| {
                self.logical_declarations.as_ref().map_or(
                    !self.logical_name_obligations.is_empty(),
                    |declarations| {
                        declarations.iter().any(|declaration| {
                            schema.authored_logical_source_alias_destination_is_active_in_frame(
                                declaration.destination,
                                frame,
                            ) != Some(false)
                                && declaration.local.literal().is_none_or(|local| {
                                    local.as_bytes() == alias.local().as_bytes()
                                })
                        })
                    },
                )
            })
            .unwrap_or(true)
    }

    fn logical_scope(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        realm: &crate::realm::CommandBindingRealm,
    ) -> Option<crate::command_binding::OriginalLogicalVariableScope> {
        let super::source_structure::OriginalRegistrySource::SourceTransitions(advice) =
            self.words.source()
        else {
            return None;
        };
        if advice.logical_source_input() != Some(input)
            || !input.has_logical_source_name_context()
            || !self.matches_source_context(
                advice.site().source.source_image(),
                input.lexer_config(),
                input.borrowed_context_registry(),
            )
            || !realm.matches_resolved_analysis_input(input)
            || !realm.matches_original_source_image(
                advice.site().source.source_image(),
                input.lexer_config(),
            )
        {
            return None;
        }
        let head = self.words.head_source()?.word()?;
        let policy = advice.dialect().authored_name_policy()?;
        let purpose =
            crate::signature_scan::original_name::OriginalSourceNamePurpose::for_analysis_input(
                input, None, policy,
            );
        let name = purpose.key(
            head,
            tcl_syntax::word_rules::WordValueRules::from_config(&input.lexer_config()),
        )?;
        realm.original_logical_variable_scope_at(
            input,
            head.image(),
            head.span(),
            &crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(name),
        )
    }

    pub(crate) fn invocation_offset(&self) -> Option<u32> {
        Some(self.words.head_source()?.word()?.span().start())
    }

    pub(crate) fn operands(
        &self,
    ) -> &[(
        tcl_lexer::Span,
        tcl_lexer::Span,
        DeclarationVariableAliasPurpose,
    )] {
        &self.operands
    }

    /// Receiver ingress of an actual direct-written alias target. This source
    /// grammar card supplies no selected frame, installed alias or successful access.
    pub(crate) fn target_receiver_operand_form(
        &self,
        operands: (tcl_lexer::Span, tcl_lexer::Span),
    ) -> Option<tcl_registry::resolved_invocation::VariableReceiverOperandForm> {
        self.operands
            .iter()
            .any(|&(local, target, _)| (local, target) == operands)
            .then_some(tcl_registry::resolved_invocation::VariableReceiverOperandForm::Combined)
    }

    /// Original namespace operand of this exact source alias relationship.
    /// Current source/Registry grammar and direct Written correspondence select
    /// naming geometry only, never an installed alias, frame or physical cell.
    pub(crate) fn target_namespace_scope(
        &self,
        source: &str,
        analysis: &crate::analyser::AnalysisResult,
        operands: (tcl_lexer::Span, tcl_lexer::Span),
        target_input: &crate::signature_scan::scope::SignatureSourceNameInput,
        current: &crate::signature_scan::scope::SignatureNamespaceScope,
        purpose: crate::signature_scan::original_name::OriginalSourceNamePurpose,
    ) -> Option<crate::signature_scan::scope::SignatureNamespaceScope> {
        let original = super::source_structure::source_registry_words_at(
            source,
            analysis,
            self.invocation_offset()?,
        )?;
        if original != self.words {
            return None;
        }
        self.target_namespace_scope_for_original_words(
            &original,
            analysis.resolved_input.as_ref()?,
            operands,
            target_input,
            current,
            purpose,
        )
    }

    /// The same alias operands under an independently validated Module source owner.
    /// Exact original words and the complete input remain separate from cells.
    pub(crate) fn target_namespace_scope_for_original_words(
        &self,
        original: &super::source_structure::OriginalRegistryWords,
        input: &crate::analyser::ResolvedAnalysisInput,
        operands: (tcl_lexer::Span, tcl_lexer::Span),
        target_input: &crate::signature_scan::scope::SignatureSourceNameInput,
        current: &crate::signature_scan::scope::SignatureNamespaceScope,
        purpose: crate::signature_scan::original_name::OriginalSourceNamePurpose,
    ) -> Option<crate::signature_scan::scope::SignatureNamespaceScope> {
        self.target_namespace_scope_for_words(
            original,
            input,
            operands,
            target_input,
            current,
            AliasNamespacePurpose::Recorded(purpose),
        )
    }

    fn target_namespace_scope_for_words(
        &self,
        original: &super::source_structure::OriginalRegistryWords,
        input: &crate::analyser::ResolvedAnalysisInput,
        operands: (tcl_lexer::Span, tcl_lexer::Span),
        target_input: &crate::signature_scan::scope::SignatureSourceNameInput,
        current: &crate::signature_scan::scope::SignatureNamespaceScope,
        selection: AliasNamespacePurpose,
    ) -> Option<crate::signature_scan::scope::SignatureNamespaceScope> {
        use tcl_registry::VariableAliasTarget as Target;
        let (purpose, logical_declaration) = match selection {
            AliasNamespacePurpose::Recorded(purpose) => (purpose, false),
            AliasNamespacePurpose::LogicalDeclaration(purpose) => (purpose, true),
        };
        if original != &self.words {
            return None;
        }
        if logical_declaration
            && (!input.has_logical_source_name_context()
                || !matches!(self.words.source(),
                super::source_structure::OriginalRegistrySource::SourceTransitions(advice)
                    if advice.logical_source_input() == Some(input)))
        {
            return None;
        }
        let context = input.borrowed_context_registry();
        let (transitions, global_frame) = self.words.with_source_schema(context, |schema| {
            let layout = if input.has_logical_source_name_context() {
                schema.authored_logical_source_frame_argument_layout()
            } else {
                schema.semantics.frame_effect.map(|frame| frame.resolve_arguments(
                    schema.words.arguments().slice_from(schema.semantics.argument_offset),
                ))
            };
            let global_frame = matches!(layout, Some(tcl_registry::frame_effect::FrameArgumentResolution::Valid { level, .. }) if level.is_global_frame());
            let transitions = if logical_declaration {
                schema.authored_logical_source_state_transitions()
            } else {
                Some(schema.semantics.state_transitions
                    .resolve_after_success_with_effect_coverage(schema.words.arguments()).0)
            };
            transitions.map(|transitions| (transitions, global_frame))
        })??;
        let mut selected = None;
        for fact in transitions.facts() {
            let tcl_registry::StateTransition::VariableCellAlias(alias) = &fact.transition else {
                continue;
            };
            let variable = match &alias.target {
                Target::Global { variable }
                | Target::CurrentNamespace { variable }
                | Target::CallerSelectedFrame { variable, .. }
                | Target::Namespace { variable, .. } => variable,
            };
            let local = written_alias_operand(&self.words, &alias.local)?;
            let target = written_alias_operand(&self.words, variable)?;
            if (local.span(), target.span()) != operands {
                continue;
            }
            let policy = purpose.policy()?;
            let rules = tcl_syntax::word_rules::WordValueRules::from_config(&target.config());
            let actual_input = purpose.key(target, rules)
                .map(crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord)
                .or_else(|| crate::signature_scan::scope::SignatureSourceNameValue::from_original_variable_operand_root(target, rules, policy)
                    .map(crate::signature_scan::scope::SignatureSourceNameInput::OriginalValue))?;
            if &actual_input != target_input {
                return None;
            }
            let namespace = match &alias.target {
                Target::Global { .. } => {
                    crate::signature_scan::scope::SignatureNamespaceScope::root(Some(policy))
                }
                Target::CurrentNamespace { .. } => current.clone(),
                Target::Namespace { namespace, .. } => {
                    let namespace = written_alias_operand(&self.words, namespace)?;
                    let key = purpose.key(
                        namespace,
                        tcl_syntax::word_rules::WordValueRules::from_config(&namespace.config()),
                    )?;
                    current.child_from_key(&key)?
                }
                Target::CallerSelectedFrame { .. } => {
                    let root = policy
                        .recipe()
                        .combined_variable_input(target_input.bytes());
                    if root.root().qualification()
                        != tcl_syntax::naming::NativeNameQualification::Absolute
                        && !global_frame
                    {
                        return None;
                    }
                    crate::signature_scan::scope::SignatureNamespaceScope::root(Some(policy))
                }
            };
            if selected.as_ref().is_some_and(|prior| prior != &namespace) {
                return None;
            }
            selected = Some(namespace);
        }
        selected
    }

    pub(crate) fn matches_source_context(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        context: &tcl_registry::model::ContextRegistry,
    ) -> bool {
        self.words.matches_source(image, config)
            && self.words.with_source_schema(context, |_| ()).is_some()
    }
}

#[derive(Clone, Copy)]
enum AliasNamespacePurpose {
    Recorded(crate::signature_scan::original_name::OriginalSourceNamePurpose),
    LogicalDeclaration(crate::signature_scan::original_name::OriginalSourceNamePurpose),
}

/// Source declaration geometry only. Its whole vector and independent frame
/// remain held; no ordinary alias receipt or completed link is issued.
#[derive(Debug, Clone, PartialEq, Eq)]
enum OriginalLogicalAliasKind {
    Known(tcl_registry::VariableCellAliasTransition),
    ConditionalNoLevel {
        card: std::sync::Arc<tcl_registry::resolved_invocation::ConditionalNoLevelAliasLayout>,
        ordinal: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OriginalLogicalAliasDeclaration {
    source: OriginalSourceVariableAliasOperands,
    input: crate::analyser::ResolvedAnalysisInput,
    alias: OriginalLogicalAliasKind,
    local: tcl_lexer::NativeWord,
    scope: crate::command_binding::OriginalLogicalVariableScope,
}
impl OriginalLogicalAliasDeclaration {
    pub(crate) fn local(&self) -> (&str, &tcl_lexer::NativeWord) {
        (
            self.local_subject()
                .literal()
                .expect("retained literal local"),
            &self.local,
        )
    }
    fn local_subject(&self) -> &tcl_registry::TransitionSubject {
        match &self.alias {
            OriginalLogicalAliasKind::Known(alias) => &alias.local,
            OriginalLogicalAliasKind::ConditionalNoLevel { card, ordinal } => {
                &card.pairs()[*ordinal].1
            }
        }
    }
    pub(crate) fn alias(&self) -> Option<&tcl_registry::VariableCellAliasTransition> {
        match &self.alias {
            OriginalLogicalAliasKind::Known(alias) => Some(alias),
            OriginalLogicalAliasKind::ConditionalNoLevel { .. } => None,
        }
    }
    pub(crate) fn target(&self) -> Option<&tcl_lexer::NativeWord> {
        if let OriginalLogicalAliasKind::ConditionalNoLevel { card, ordinal } = &self.alias {
            return written_alias_operand(&self.source.words, &card.pairs()[*ordinal].0);
        }
        let target = match &self.alias()?.target {
            tcl_registry::VariableAliasTarget::Global { variable }
            | tcl_registry::VariableAliasTarget::CurrentNamespace { variable }
            | tcl_registry::VariableAliasTarget::CallerSelectedFrame { variable, .. }
            | tcl_registry::VariableAliasTarget::Namespace { variable, .. } => variable,
        };
        written_alias_operand(&self.source.words, target)
    }
    pub(crate) fn target_occurrence(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        target: crate::signature_scan::scope::SignatureSourceNameInput,
        purpose: crate::signature_scan::original_name::OriginalSourceNamePurpose,
    ) -> Option<crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence> {
        let namespace = self.target_namespace(input, &target, purpose)?;
        crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence::from_logical_alias_target(
            self.target()?.span(), target, namespace,
            crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver::NamespaceAlias,
            self.scope.clone(),
        )
    }

    /// Readonly template geometry. The relation remains inside this source
    /// carrier and never enters the ordinary alias receipt inventory.
    pub(crate) fn template_source(&self) -> Option<OriginalLogicalAliasTemplateSource> {
        use crate::signature_scan::scope::{SignatureSourceNameInput, SignatureSourceNameValue};
        use crate::signature_scan::variable_symbol::{
            OriginalVariableAliasReceipt, original_variable_alias_local_name,
        };
        let super::source_structure::OriginalRegistrySource::SourceTransitions(advice) =
            self.source.words.source()
        else {
            return None;
        };
        let policy = advice.dialect().authored_name_policy()?;
        let purpose =
            crate::signature_scan::original_name::OriginalSourceNamePurpose::for_analysis_input(
                &self.input,
                None,
                policy,
            );
        let rules = tcl_syntax::word_rules::WordValueRules::from_config(&self.input.lexer_config());
        let local_input = SignatureSourceNameInput::OriginalWord(purpose.key(&self.local, rules)?);
        let target_word = self.target()?;
        let target_input = purpose
            .key(target_word, rules)
            .map(SignatureSourceNameInput::OriginalWord)
            .or_else(|| {
                SignatureSourceNameValue::from_original_variable_operand_root(
                    target_word,
                    rules,
                    policy,
                )
                .map(SignatureSourceNameInput::OriginalValue)
            })?;
        let target = self.target_occurrence(&self.input, target_input, purpose)?;
        let alias_purpose = match self.alias().map(|alias| &alias.target) {
            Some(tcl_registry::VariableAliasTarget::Global { .. }) => {
                DeclarationVariableAliasPurpose::Global
            }
            Some(tcl_registry::VariableAliasTarget::CurrentNamespace { .. }) => {
                DeclarationVariableAliasPurpose::NamespaceVariable
            }
            Some(tcl_registry::VariableAliasTarget::CallerSelectedFrame { .. }) => {
                DeclarationVariableAliasPurpose::Upvar
            }
            Some(tcl_registry::VariableAliasTarget::Namespace { .. }) => {
                DeclarationVariableAliasPurpose::NamespaceUpvar
            }
            None => DeclarationVariableAliasPurpose::Upvar,
        };
        let local = original_variable_alias_local_name(&local_input, alias_purpose)?;
        let relation = OriginalVariableAliasReceipt::new(
            self.scope.alias_frame()?,
            local,
            target.symbol().clone(),
            target.original_name_input().clone(),
            local_input,
            false,
            (self.source.invocation_offset()?, self.local.span()),
        );
        Some(OriginalLogicalAliasTemplateSource {
            declaration: self.clone(),
            relation,
        })
    }

    fn target_namespace(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        target: &crate::signature_scan::scope::SignatureSourceNameInput,
        purpose: crate::signature_scan::original_name::OriginalSourceNamePurpose,
    ) -> Option<crate::signature_scan::scope::SignatureNamespaceScope> {
        if input != &self.input {
            return None;
        }
        if let OriginalLogicalAliasKind::ConditionalNoLevel { card, .. } = &self.alias {
            card.may_argument_error().then_some(())?;
            let word = self.target()?;
            let policy = purpose.policy()?;
            let rules = tcl_syntax::word_rules::WordValueRules::from_config(&word.config());
            let actual = purpose.key(word, rules)
                .map(crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord)
                .or_else(|| crate::signature_scan::scope::SignatureSourceNameValue::from_original_variable_operand_root(word, rules, policy)
                    .map(crate::signature_scan::scope::SignatureSourceNameInput::OriginalValue))?;
            if &actual != target
                || policy
                    .recipe()
                    .combined_variable_input(actual.bytes())
                    .root()
                    .qualification()
                    != tcl_syntax::naming::NativeNameQualification::Absolute
            {
                return None;
            }
            return Some(crate::signature_scan::scope::SignatureNamespaceScope::root(
                Some(policy),
            ));
        }
        self.source.target_namespace_scope_for_words(
            &self.source.words,
            input,
            (self.local.span(), self.target()?.span()),
            target,
            &self.scope.namespace(),
            AliasNamespacePurpose::LogicalDeclaration(purpose),
        )
    }
}

/// Full source declaration ownership for a conditional alias template only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OriginalLogicalAliasTemplateSource {
    declaration: OriginalLogicalAliasDeclaration,
    relation: crate::signature_scan::variable_symbol::OriginalVariableAliasReceipt,
}
impl OriginalLogicalAliasTemplateSource {
    pub(crate) fn matches_source(
        &self,
        image: &tcl_lexer::SourceImage,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> bool {
        &self.declaration.input == input
            && self.declaration.scope.matches_source(image, input)
            && self.declaration.source.matches_source_context(
                image,
                input.lexer_config(),
                input.borrowed_context_registry(),
            )
    }
    pub(crate) fn relation(
        &self,
    ) -> &crate::signature_scan::variable_symbol::OriginalVariableAliasReceipt {
        &self.relation
    }
}

fn logical_alias_frame(
    scope: Option<&crate::command_binding::OriginalLogicalVariableScope>,
) -> tcl_registry::VariableAliasFrame {
    scope.map_or(tcl_registry::VariableAliasFrame::Unknown, |scope| {
        if let Some(frame) = scope.alias_frame() {
            match frame.frame().layout() {
                crate::var_resolve::VariableExecutionFrame::Procedure { .. }
                | crate::var_resolve::VariableExecutionFrame::ReceiverMethod { .. } => {
                    tcl_registry::VariableAliasFrame::Procedure
                }
                crate::var_resolve::VariableExecutionFrame::Namespace(_)
                | crate::var_resolve::VariableExecutionFrame::NamespaceActivation { .. } => {
                    tcl_registry::VariableAliasFrame::Namespace
                }
                _ => tcl_registry::VariableAliasFrame::Unknown,
            }
        } else if scope.namespace_storage() {
            tcl_registry::VariableAliasFrame::Global
        } else {
            tcl_registry::VariableAliasFrame::Unknown
        }
    })
}

fn written_alias_operand<'a>(
    words: &'a super::source_structure::OriginalRegistryWords,
    subject: &tcl_registry::TransitionSubject,
) -> Option<&'a tcl_lexer::NativeWord> {
    let argument = subject.argument_index()?;
    // Captured prefixes and expanded elements cannot borrow an enclosing
    // written word as an alias declaration or target operand.
    let super::InvocationWordOrigin::Written(_) = words.origins().get(argument.checked_add(1)?)?
    else {
        return None;
    };
    words.operands().get(argument)?.as_ref()?.word()
}

pub(crate) fn original_declaration_assistance(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
    advice: &crate::command_binding::OriginalCompilationLookupAdvice,
) -> Option<OriginalDeclarationAssistance> {
    let dialect = advice.dialect();
    let mut agreed = None;
    for target in advice.targets() {
        let effective = effective_words_for_target(tokens, target)?;
        let values = frozen_argument_words(tokens, &effective);
        let mut words = vec![InvocationWord::Literal(&target.command)];
        words.extend(
            values
                .iter()
                .map(|word| super::source_structure::source_schema_word(word.as_registry_word())),
        );
        let RegistryInvocationResolution::Resolved(facts) =
            resolve_registry_words_in_realm(registry, None, &words, Some(dialect), advice.realm())
                .ok()?
        else {
            return None;
        };
        if !facts.arg_roles_complete {
            return None;
        }
        let mut selected = OriginalDeclarationAssistance {
            variable_alias_declarations: 0,
            variable_alias_operands: Vec::new(),
            variable_receivers: Vec::new(),
            variable_name_obligations: Vec::new(),
            declarations: Vec::new(),
            body_arguments: Vec::new(),
            lambda_arguments: Vec::new(),
        };
        retain_variable_aliases(&mut selected, &facts, &effective, advice);
        for &(index, role) in &facts.arg_roles {
            let argument = facts.argument_offset.checked_add(usize::from(index))?;
            let Some(written) = effective.written_argument(argument) else {
                continue;
            };
            match role {
                tcl_registry::ArgRole::Body => selected.body_arguments.push(written),
                tcl_registry::ArgRole::LambdaLiteral => selected.lambda_arguments.push(written),
                tcl_registry::ArgRole::VarWrite => {
                    let original = effective.words.get(argument.checked_add(1)?)?;
                    let EffectiveInvocationWord::Literal(name) = effective_invocation_word(
                        original,
                        dialect.lexer_grammar.escapes,
                        dialect.word_values,
                    ) else {
                        continue;
                    };
                    for declaration in alias_declarations(&facts, &name, argument, written, dialect)
                    {
                        if !selected.declarations.contains(&declaration) {
                            selected.declarations.push(declaration);
                        }
                    }
                }
                _ => {}
            }
            if let Some(form) = facts.variable_receiver_operand_form(argument) {
                let receiver = DeclarationVariableReceiver {
                    argument: written,
                    form,
                };
                if !selected.variable_receivers.contains(&receiver) {
                    selected.variable_receivers.push(receiver);
                }
            } else if matches!(
                role,
                tcl_registry::ArgRole::VarRead | tcl_registry::ArgRole::VarWrite
            ) && !selected.variable_name_obligations.contains(&written)
            {
                selected.variable_name_obligations.push(written);
            }
        }
        if agreed
            .as_ref()
            .is_some_and(|previous| previous != &selected)
        {
            return None;
        }
        agreed = Some(selected);
    }
    agreed
}

fn retain_variable_aliases(
    selected: &mut OriginalDeclarationAssistance,
    facts: &tcl_registry::InvocationFacts,
    effective: &super::EffectiveCommandWords,
    advice: &crate::command_binding::OriginalCompilationLookupAdvice,
) {
    let dialect = advice.dialect();
    for fact in facts
        .state_transitions
        .declared()
        .into_iter()
        .flat_map(tcl_registry::StateTransitions::facts)
    {
        let tcl_registry::StateTransition::VariableCellAlias(alias) = &fact.transition else {
            continue;
        };
        if alias
            .destination
            .is_active_in_frame(advice.alias_frame(), dialect)
            == Some(false)
        {
            continue;
        }
        selected.variable_alias_declarations += 1;
        let (variable, purpose) = match &alias.target {
            tcl_registry::VariableAliasTarget::Global { variable } => {
                (variable, DeclarationVariableAliasPurpose::Global)
            }
            tcl_registry::VariableAliasTarget::CurrentNamespace { variable } => {
                (variable, DeclarationVariableAliasPurpose::NamespaceVariable)
            }
            tcl_registry::VariableAliasTarget::CallerSelectedFrame { variable, .. } => {
                (variable, DeclarationVariableAliasPurpose::Upvar)
            }
            tcl_registry::VariableAliasTarget::Namespace { variable, .. } => {
                (variable, DeclarationVariableAliasPurpose::NamespaceUpvar)
            }
        };
        let Some(local) = alias
            .local
            .argument_index()
            .and_then(|index| effective.written_argument(index))
        else {
            continue;
        };
        let Some(target) = variable
            .argument_index()
            .and_then(|index| effective.written_argument(index))
        else {
            continue;
        };
        selected
            .variable_alias_operands
            .push(DeclarationVariableAlias {
                local,
                target,
                purpose,
            });
    }
}

fn alias_declarations(
    facts: &tcl_registry::InvocationFacts,
    name: &str,
    argument: usize,
    written: usize,
    dialect: tcl_registry::InvocationDialect,
) -> Vec<DeclarationArgument> {
    let mut declarations = Vec::new();
    for fact in facts
        .state_transitions
        .declared()
        .into_iter()
        .flat_map(tcl_registry::StateTransitions::facts)
    {
        let tcl_registry::StateTransition::VariableCellAlias(alias) = &fact.transition else {
            continue;
        };
        if alias.local.argument_index() != Some(argument) {
            continue;
        }
        let Some(local) = alias.local.literal() else {
            continue;
        };
        let written_local = match alias.target {
            tcl_registry::VariableAliasTarget::Global { .. } => {
                tcl_registry::state_transition::local_alias_name(
                    &tcl_registry::TransitionSubject::Literal(name.to_owned()),
                    argument,
                    tcl_registry::state_transition::VariableAliasNamePurpose::Global,
                    Some(dialect),
                )
            }
            tcl_registry::VariableAliasTarget::CurrentNamespace { .. } => {
                tcl_registry::state_transition::local_alias_name(
                    &tcl_registry::TransitionSubject::Literal(name.to_owned()),
                    argument,
                    tcl_registry::state_transition::VariableAliasNamePurpose::NamespaceVariable,
                    Some(dialect),
                )
            }
            tcl_registry::VariableAliasTarget::CallerSelectedFrame { .. }
            | tcl_registry::VariableAliasTarget::Namespace { .. } => {
                Some(tcl_registry::TransitionSubject::Literal(name.to_owned()))
            }
        };
        let Some(written_local) = written_local else {
            continue;
        };
        if written_local.literal() != Some(local) || !alias_target_is_literal(&alias.target) {
            continue;
        }
        let declaration = DeclarationArgument {
            argument: written,
            name: local.to_owned(),
        };
        if !declarations.contains(&declaration) {
            declarations.push(declaration);
        }
    }
    declarations
}

fn alias_target_is_literal(target: &tcl_registry::VariableAliasTarget) -> bool {
    match target {
        tcl_registry::VariableAliasTarget::Global { variable }
        | tcl_registry::VariableAliasTarget::CurrentNamespace { variable }
        | tcl_registry::VariableAliasTarget::CallerSelectedFrame { variable, .. } => {
            variable.literal().is_some()
        }
        tcl_registry::VariableAliasTarget::Namespace {
            namespace,
            variable,
        } => namespace.literal().is_some() && variable.literal().is_some(),
    }
}

#[cfg(test)]
mod original_alias_source_schema_tests {
    #[test]
    fn logical_alias_declarations_keep_actual_source_frame_and_ordinary_widening() {
        let source = "upvar #0 {a(b)} local";
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
        let input = analysis.resolved_input.as_ref().unwrap();
        let realm = analysis.retained_command_realm().unwrap();
        let words = crate::registry_invocation::source_structure::source_registry_words_at(
            source, &analysis, 0,
        )
        .unwrap();
        let ordinary = words
            .with_source_schema(input.borrowed_context_registry(), |schema| {
                schema.state_transitions()
            })
            .unwrap();
        assert!(
            ordinary
                .facts()
                .iter()
                .all(|fact| matches!(fact.transition, tcl_registry::StateTransition::Widen(_)))
        );
        let aliases = super::OriginalSourceVariableAliasOperands::capture(
            words,
            input.borrowed_context_registry(),
        )
        .unwrap();
        assert!(aliases.operands().is_empty());
        let declarations = aliases.logical_declarations(input, realm).unwrap();
        let [declaration] = declarations.as_slice() else {
            panic!("one actual source local");
        };
        assert_eq!(declaration.local().0, "local");
        assert_eq!(declaration.local().1.image().bytes(), source.as_bytes());
        assert_eq!(&source[declaration.local().1.span().as_range()], "local");
        assert_eq!(
            &source[declaration.target().unwrap().span().as_range()],
            "{a(b)}"
        );
        let policy = tcl_registry::InvocationDialect::of_profile(input.unit_profile())
            .authored_name_policy()
            .unwrap();
        let purpose =
            crate::signature_scan::original_name::OriginalSourceNamePurpose::for_analysis_input(
                input, None, policy,
            );
        let target = original_target_input(&aliases, declaration.target().unwrap().span(), purpose);
        assert!(
            declaration
                .target_namespace(input, &target, purpose)
                .is_some()
        );

        let mut config = input.lexer_config();
        config.strict_quoting = !config.strict_quoting;
        let changed = crate::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            input.context_registry(),
            config,
        );
        assert!(aliases.logical_declarations(&changed, realm).is_none());
        assert!(
            declaration
                .target_namespace(&changed, &target, purpose)
                .is_none()
        );
        let foreign = crate::analyser::Analyser::new()
            .with_resolved_input(input.clone())
            .analyse("upvar #0 {z(b)} local", "tcl");
        assert!(
            aliases
                .logical_declarations(input, foreign.retained_command_realm().unwrap())
                .is_none()
        );

        let source = "namespace eval N {global shared}";
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
        let input = analysis.resolved_input.as_ref().unwrap();
        let words = crate::registry_invocation::source_structure::source_registry_words_at(
            source,
            &analysis,
            u32::try_from(source.find("global").unwrap()).unwrap(),
        )
        .unwrap();
        let aliases = super::OriginalSourceVariableAliasOperands::capture(
            words,
            input.borrowed_context_registry(),
        )
        .unwrap();
        assert!(
            aliases
                .logical_declarations(input, analysis.retained_command_realm().unwrap())
                .unwrap()
                .is_empty()
        );
    }

    fn assert_original_namespace_frame_has_no_execution_policy(
        aliases: &super::OriginalSourceVariableAliasOperands,
        input: &crate::analyser::ResolvedAnalysisInput,
        realm: &crate::realm::CommandBindingRealm,
    ) {
        let head = aliases
            .words
            .head_source()
            .and_then(|head| head.word())
            .unwrap();
        let super::super::source_structure::OriginalRegistrySource::SourceTransitions(advice) =
            aliases.words.source()
        else {
            panic!("actual original Logical alias declaration source");
        };
        let dialect = advice.dialect();
        let policy = dialect.authored_name_policy().unwrap();
        let purpose =
            crate::signature_scan::original_name::OriginalSourceNamePurpose::for_analysis_input(
                input, None, policy,
            );
        let key = purpose
            .key(
                head,
                tcl_syntax::word_rules::WordValueRules::from_config(&input.lexer_config()),
            )
            .unwrap();
        let scope = realm
            .original_logical_variable_scope_at(
                input,
                head.image(),
                head.span(),
                &crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(key),
            )
            .expect("genuine independent original namespace source frame");
        assert!(matches!(
            scope.alias_frame().unwrap().frame().layout(),
            crate::var_resolve::VariableExecutionFrame::NamespaceActivation { .. }
        ));
        assert_eq!(dialect.variable_lookup_policy, None);
        assert_eq!(
            tcl_registry::VariableAliasDestination::ProcedureLocal
                .is_active_in_frame(tcl_registry::VariableAliasFrame::Namespace, dialect,),
            None,
            "source geometry must not supply the missing execution policy"
        );
    }

    fn assert_original_namespace_alias_refusals(
        aliases: &[super::OriginalSourceVariableAliasOperands],
        source: &str,
        analysis: &crate::analyser::AnalysisResult,
        operands: (tcl_lexer::Span, tcl_lexer::Span),
        target_input: &crate::signature_scan::scope::SignatureSourceNameInput,
        current: &crate::signature_scan::scope::SignatureNamespaceScope,
        purpose: crate::signature_scan::original_name::OriginalSourceNamePurpose,
    ) {
        let (local, target) = operands;
        assert!(
            aliases[0]
                .target_namespace_scope(
                    &source.replace("child", "other"),
                    analysis,
                    (local, target),
                    target_input,
                    current,
                    purpose,
                )
                .is_none()
        );
        assert!(
            aliases[0]
                .target_namespace_scope(
                    source,
                    analysis,
                    (target, local),
                    target_input,
                    current,
                    purpose,
                )
                .is_none()
        );
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(
            aliases[0]
                .target_namespace_scope(
                    source,
                    &missing,
                    (local, target),
                    target_input,
                    current,
                    purpose,
                )
                .is_none()
        );
    }

    #[test]
    fn logical_alias_name_obligations_keep_actual_dynamic_targets_and_frame_noops() {
        for (source, head, target) in [
            (
                "proc p {} {global shared; upvar 0 $unknown shared; $dispatch $shared}",
                "upvar 0",
                Some("$unknown"),
            ),
            (
                "proc p {} {upvar $level $unknown shared; $dispatch $shared}",
                "upvar",
                Some("upvar"),
            ),
            ("proc p {} {global $unknown}", "global", Some("$unknown")),
            ("global $unknown", "global", None),
            ("namespace eval N {global $unknown}", "global", None),
        ] {
            let profile = tcl_dialect::DialectProfile::plain_tcl();
            let input = crate::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                tcl_registry::model::ingress::resolve_environment("tcl9.1")
                    .default_context_registry(),
                tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
            );
            let analysis = crate::analyser::Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(source, profile.name);
            let realm = analysis.retained_command_realm().unwrap();
            let offset = u32::try_from(source.find(head).unwrap()).unwrap();
            let words =
                super::super::source_structure::source_registry_words_at(source, &analysis, offset)
                    .expect("actual selected Logical alias source schema");
            let aliases = super::OriginalSourceVariableAliasOperands::capture(
                words,
                input.borrowed_context_registry(),
            )
            .expect("original unresolved alias naming receipt");
            if source == "namespace eval N {global $unknown}" {
                assert_original_namespace_frame_has_no_execution_policy(&aliases, &input, realm);
            }
            let spans = aliases
                .unresolved_logical_name_spans(&input, realm)
                .expect("genuine complete original input and source frame");
            let expected = target
                .into_iter()
                .map(|target| {
                    let start = u32::try_from(source.find(target).unwrap()).unwrap();
                    tcl_lexer::Span::new(start, start + u32::try_from(target.len()).unwrap())
                })
                .collect::<Vec<_>>();
            assert_eq!(spans, expected, "{source}");
            if target.is_some() {
                assert!(
                    !analysis.original_variable_alias_source_is_complete(
                        &tcl_lexer::SourceImage::document(source),
                        input.lexer_config(),
                    ),
                    "{source}"
                );
            }
            let mut config = input.lexer_config();
            config.strict_quoting = !config.strict_quoting;
            let changed = crate::analyser::ResolvedAnalysisInput::new(
                input.analyser_profile(),
                input.unit_profile(),
                input.context_registry(),
                config,
            );
            assert!(
                aliases
                    .unresolved_logical_name_spans(&changed, realm)
                    .is_none()
            );
            let foreign = crate::analyser::Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(&source.replace("unknown", "changed"), profile.name);
            assert!(
                aliases
                    .unresolved_logical_name_spans(
                        &input,
                        foreign.retained_command_realm().unwrap(),
                    )
                    .is_none()
            );
        }
    }

    fn original_target_input(
        aliases: &super::OriginalSourceVariableAliasOperands,
        target: tcl_lexer::Span,
        purpose: crate::signature_scan::original_name::OriginalSourceNamePurpose,
    ) -> crate::signature_scan::scope::SignatureSourceNameInput {
        let word = aliases
            .words
            .operands()
            .iter()
            .flatten()
            .filter_map(|operand| operand.word())
            .find(|word| word.span() == target)
            .expect("authentic retained target operand");
        crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(
            purpose
                .key(
                    word,
                    tcl_syntax::word_rules::WordValueRules::from_config(&word.config()),
                )
                .expect("original literal target naming input"),
        )
    }

    #[test]
    fn original_namespace_alias_target_uses_written_namespace_and_current_scope() {
        let source = "namespace eval outer {namespace upvar child value local}";
        let mut analyser = crate::analyser::Analyser::new();
        let analysis = analyser.analyse(source, "tcl");
        let current = analysis.global_scope.children[0]
            .naming_scope
            .as_ref()
            .unwrap();
        let input = analysis.resolved_input.as_ref().unwrap();
        let policy = tcl_registry::InvocationDialect::of_profile(input.unit_profile())
            .authored_name_policy()
            .unwrap();
        let purpose =
            crate::signature_scan::original_name::OriginalSourceNamePurpose::for_analysis_input(
                input, None, policy,
            );
        let words = crate::registry_invocation::source_structure::source_registry_words_at(
            source,
            &analysis,
            u32::try_from(source.find("namespace upvar").unwrap()).unwrap(),
        )
        .expect("original alias invocation source");
        let aliases = super::OriginalSourceVariableAliasOperands::capture(
            words,
            input.borrowed_context_registry(),
        )
        .into_iter()
        .collect::<Vec<_>>();
        assert_eq!(aliases.len(), 1);
        let (local, target, _) = aliases[0].operands()[0];
        let target_input = original_target_input(&aliases[0], target, purpose);
        let selected = aliases[0]
            .target_namespace_scope(
                source,
                &analysis,
                (local, target),
                &target_input,
                current,
                purpose,
            )
            .expect("actual selected written namespace operand");
        let crate::signature_scan::scope::SignatureNamespaceScope::C(path) = &selected else {
            panic!("selected C source namespace geometry");
        };
        assert_eq!(
            path.as_segments()
                .iter()
                .map(tcl_core_types::NameBytes::as_bytes)
                .collect::<Vec<_>>(),
            vec![b"outer".as_slice(), b"child".as_slice()]
        );
        let occurrence = analysis
            .original_variable_symbols
            .iter()
            .find(|occurrence| occurrence.span() == target)
            .expect("original target occurrence");
        let crate::signature_scan::variable_symbol::SignatureSourceVariableSlot::C {
            namespace,
            simple,
        } = occurrence.symbol().slot()
        else {
            panic!("C source variable slot");
        };
        assert_eq!(namespace.as_segments(), path.as_segments());
        assert_eq!(simple.as_bytes(), b"value");
        assert_original_namespace_alias_refusals(
            &aliases,
            source,
            &analysis,
            (local, target),
            &target_input,
            current,
            purpose,
        );
    }

    #[test]
    fn original_namespace_alias_target_refuses_dynamic_and_captured_namespace_operands() {
        for source in [
            "namespace eval outer {namespace upvar $ns value local}",
            "interp alias {} link {} namespace upvar ::target; link value local",
        ] {
            let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
            let input = analysis.resolved_input.as_ref().unwrap();
            let policy = tcl_registry::InvocationDialect::of_profile(input.unit_profile())
                .authored_name_policy()
                .unwrap();
            let purpose =
                crate::signature_scan::original_name::OriginalSourceNamePurpose::for_analysis_input(
                    input, None, policy,
                );
            let current = analysis
                .global_scope
                .children
                .first()
                .and_then(|scope| scope.naming_scope.as_ref())
                .or(analysis.global_scope.naming_scope.as_ref())
                .unwrap();
            let offset = source
                .rfind("link value")
                .or_else(|| source.find("namespace upvar"))
                .unwrap();
            let aliases = crate::registry_invocation::source_structure::source_registry_words_at(
                source,
                &analysis,
                u32::try_from(offset).unwrap(),
            )
            .and_then(|words| {
                super::OriginalSourceVariableAliasOperands::capture(
                    words,
                    input.borrowed_context_registry(),
                )
            });
            if let Some(aliases) = aliases {
                for &(local, target, _) in aliases.operands() {
                    let target_input = original_target_input(&aliases, target, purpose);
                    assert!(
                        aliases
                            .target_namespace_scope(
                                source,
                                &analysis,
                                (local, target),
                                &target_input,
                                current,
                                purpose,
                            )
                            .is_none()
                    );
                }
            }
        }
    }

    #[test]
    fn original_alias_source_schema_keeps_counted_controls_and_written_operands() {

        let source = "set shared 1\nproc first {argument} {global shared; upvar #0 shared link; puts $link}\n";
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let image = tcl_lexer::SourceImage::document(source);
        let config = analysis.body_lexer_config.unwrap();
        let advice = analysis
            .original_variable_alias_advice_in_source(&image, config)
            .unwrap();
        let aliases = advice
            .iter()
            .map(|alias| alias.local_name().as_bytes())
            .collect::<Vec<_>>();
        assert!(aliases.contains(&b"shared".as_slice()), "{aliases:?}");
        assert!(aliases.contains(&b"link".as_slice()), "{aliases:?}");
        assert!(analysis.original_variable_symbols.iter().any(|occurrence| {
            occurrence
                .original_local_alias()
                .is_some_and(|(_, name)| name.as_bytes() == b"link")
        }));
        assert!(
            analysis
                .original_variable_alias_advice_in_source(
                    &tcl_lexer::SourceImage::document(&format!("{source} ")),
                    config
                )
                .is_none()
        );
        for source in [
            "proc global args {}; proc first {} {global shared}",
            "proc upvar args {}; proc first {} {upvar #0 shared link}",
            "proc first {level} {upvar $level shared link}",
            "proc first {names} {global {*}$names}",
        ] {
            let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
            assert!(
                analysis
                    .original_variable_alias_advice_in_source(
                        &tcl_lexer::SourceImage::document(source),
                        analysis.body_lexer_config.unwrap()
                    )
                    .unwrap()
                    .is_empty(),
                "{source}"
            );
        }
    }
}
