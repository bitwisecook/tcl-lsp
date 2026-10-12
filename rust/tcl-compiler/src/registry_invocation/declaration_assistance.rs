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
        #[cfg(debug_assertions)]
        if std::env::var_os("TCL_LSP_TRACE_ALIAS_TEMPLATE_SOURCE").is_some() {
            eprintln!(
                "ALIAS_INGRESS_TRANSITIONS site={:?} selected={} origins={:?} transitions={selected:?}",
                words
                    .head_source()
                    .and_then(|head| head.word())
                    .map(tcl_lexer::NativeWord::span),
                selected.is_some(),
                words.origins()
            );
        }
        let transitions = selected?;
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
        (!operands.is_empty()).then_some(Self { words, operands })
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
    fn original_alias_source_schema_keeps_counted_controls_and_written_operands() {
        // naming.compiler.original-variable-alias-source-schema
        // docs/design/analysis/name-resolution-proofs/original-variable-alias-source-schema.md
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
