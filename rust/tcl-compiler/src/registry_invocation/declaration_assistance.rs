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

/// Unanimous grammar of an unchanged original declaration-local invocation.
/// Alias prefixes, member selection and source origins share one mapping.
/// Runtime dispatch, callbacks and body entry remain independent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalDeclarationAssistance {
    /// Names with editable original declaration operands.
    pub declarations: Vec<DeclarationArgument>,
    /// Original written arguments containing authored script bodies.
    pub body_arguments: Vec<usize>,
    /// Original written arguments containing lambda lists.
    pub lambda_arguments: Vec<usize>,
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
        words.extend(values.iter().map(EffectiveInvocationWord::as_registry_word));
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
            declarations: Vec::new(),
            body_arguments: Vec::new(),
            lambda_arguments: Vec::new(),
        };
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
