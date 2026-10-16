// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional parameter-source prefixes from the actual completed Logical walk.

use super::{
    OriginalLogicalProcedureBody, OriginalSourceCommandTransitionAdvice,
    OriginalSourceTransitionAdviceTape,
};
use crate::analyser::ResolvedAnalysisInput;
use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};
use tcl_lexer::{ExecutablePart, NativeWord};
use tcl_registry::model::ContextRegistry;

/// Derived positive receipts only. Warming this cache changes no source-body
/// identity, equality or binding semantics. No row retains a procedure card.
#[derive(Debug, Default)]
pub(super) struct ParameterSourcePrefixCache {
    prefixes: OnceLock<BTreeMap<u32, OriginalLogicalParameterSourcePrefix>>,
}
impl PartialEq for ParameterSourcePrefixCache {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}
impl Eq for ParameterSourcePrefixCache {}
impl ParameterSourcePrefixCache {
    pub(super) fn retain(&self, prefixes: BTreeMap<u32, OriginalLogicalParameterSourcePrefix>) {
        if !prefixes.is_empty() {
            let _ = self.prefixes.set(prefixes);
        }
    }
    pub(super) fn get(
        &self,
        original: &[NativeWord],
    ) -> Option<&OriginalLogicalParameterSourcePrefix> {
        let prefix = self
            .prefixes
            .get()?
            .get(&original.first()?.span().start())?;
        (prefix.current()?.original_words() == original).then_some(prefix)
    }
}

/// One retained symbolic input read. It is original lexical/formal source
/// correspondence, never a quiet read, current value, cell or observer closure.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ConditionalFormalSourceRead {
    original: NativeWord,
    parameter: usize,
}

/// Actual own-body source operations through one original invocation. All
/// runtime lookup/observer/caller applicability debt stays on retained advice.
/// This closes no execution, Native, Normal, successful alias or store query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OriginalLogicalParameterSourcePrefix {
    input: ResolvedAnalysisInput,
    scope: Arc<crate::command_binding::OriginalDeclaredLogicalBodyContext>,
    formal: NativeWord,
    parameters: Vec<tcl_syntax::formal_params::FormalParameter>,
    operations: Vec<Arc<OriginalSourceCommandTransitionAdvice>>,
    reads: Vec<Arc<ConditionalFormalSourceRead>>,
}
impl OriginalLogicalParameterSourcePrefix {
    pub(crate) fn current(&self) -> Option<&OriginalSourceCommandTransitionAdvice> {
        self.operations.last().map(Arc::as_ref)
    }
    pub(crate) fn matches_body(
        &self,
        body: &OriginalLogicalProcedureBody,
        input: &ResolvedAnalysisInput,
    ) -> bool {
        &self.input == input
            && self.scope == *body.scope()
            && self.formal == *body.formal_original()
            && self.parameters == body.parameters()
            && body.matches_source(self.formal.image(), input)
            && !body.has_earlier_unknown_mutations()
    }
    #[cfg(test)]
    pub(crate) fn operations(&self) -> &[Arc<OriginalSourceCommandTransitionAdvice>] {
        &self.operations
    }
    #[cfg(test)]
    pub(crate) fn formal_read_count(&self) -> usize {
        self.reads.len()
    }
}

/// No new lookup occurs here. Complete own-body vectors must already occur in
/// the actual Logical walk with the exact scope and selected descriptor advice.
/// The first unrepresented/unknown operation withdraws later source templates;
/// lexical position alone never repairs a missing walk row.
pub(super) fn capture_prefixes(
    tape: &OriginalSourceTransitionAdviceTape,
    body: &OriginalLogicalProcedureBody,
    context: &ContextRegistry,
) -> Option<BTreeMap<u32, OriginalLogicalParameterSourcePrefix>> {
    let input = body.scope().scope_body().source_words().source();
    let crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(
        parent,
    ) = input
    else {
        return None;
    };
    let input = parent.logical_source_input()?;
    if body.has_earlier_unknown_mutations()
        || !body.matches_source(body.formal_original().image(), input)
        || !body.scope().scope_body().matches_context(context)
    {
        return None;
    }
    let commands = body.scope().scope_body().original_commands(context)?;
    // Bounded immutable copies of actual existing receipts, not recursive
    // sidecar edges or an independent source invocation catalogue.
    if commands.len() > 128 {
        return None;
    }
    let mut result = BTreeMap::new();
    let mut operations: Vec<Arc<OriginalSourceCommandTransitionAdvice>> = Vec::new();
    let mut reads = Vec::new();
    for command in commands {
        let offset = command.words.first()?.span().start();
        let Some(actual) = tape
            .logical_summary_scopes
            .get(&offset)
            .and_then(Option::as_ref)
        else {
            break;
        };
        if actual.original.as_ref() != command.words.as_slice() || actual.scope != *body.scope() {
            break;
        }
        let Some(advice) = tape.advice.get(&offset) else {
            break;
        };
        if tape.blocks_registry_source(offset)
            || advice.original_words() != command.words.as_slice()
            || advice.logical_source_input() != Some(input)
            || advice.logical_scope.as_ref() != Some(body.scope())
            || advice.declaration.is_some()
            || !advice.lineage.is_empty()
            || !advice.matches_source(body.formal_original().image(), input.lexer_config())
            || !advice.matches_context(context)
            || advice.roles.is_none()
            || advice
                .original_words()
                .iter()
                .any(|word| word.group().expand)
        {
            break;
        }
        let Some(current_reads) = formal_source_reads(advice.original_words(), body) else {
            break;
        };
        // Every recorded uncertainty remains on its actual advice. It must
        // correspond to an already represented own-body operation, rather
        // than an unowned earlier callback or a missing source operation.
        if advice.uncertain_operations().iter().any(|unknown| {
            unknown.as_ref() != advice.original_words()
                && !operations
                    .iter()
                    .any(|earlier| earlier.original_words() == unknown.as_ref())
        }) {
            break;
        }
        let source = body.formal_original().image().try_text().ok()?;
        let Some(words) =
            crate::registry_invocation::source_structure::source_transition_words_from_advice(
                source,
                input.lexer_config(),
                context,
                advice.clone(),
            )
        else {
            break;
        };
        let Some((valid, subsequent)) = words
            .with_source_schema(context, |schema| {
                let valid = schema.facts().arg_roles_complete
                    && schema.facts().arity_accepts_frozen_arguments() == Some(true);
                let transitions = schema.authored_logical_source_state_transitions()?;
                // Shared Registry impact owns command-name stability. It is a
                // conditional source recipe only; observer callbacks stay debt.
                let subsequent = schema.semantics.state_transitions.is_declared()
                    && transitions.command_resolution_impact()
                        == tcl_registry::CommandResolutionImpact::None
                    && transitions.facts().iter().all(|fact| {
                        matches!(
                            fact.transition,
                            tcl_registry::StateTransition::VariableCellAlias(_)
                                | tcl_registry::StateTransition::Widen(_)
                        )
                    })
                    && ![
                        tcl_registry::StateTransitionDomain::VariableTraces,
                        tcl_registry::StateTransitionDomain::CommandTraces,
                        tcl_registry::StateTransitionDomain::ExecutionTraces,
                        tcl_registry::StateTransitionDomain::InterpreterTopology,
                        tcl_registry::StateTransitionDomain::InterpreterPolicy,
                        tcl_registry::StateTransitionDomain::Interpreters,
                        tcl_registry::StateTransitionDomain::ObjectDispatch,
                    ]
                    .into_iter()
                    .any(|domain| transitions.widens(domain))
                    && !words.roles().unwrap_or_default().iter().any(|(_, role)| {
                        matches!(
                            role,
                            tcl_registry::ArgRole::Body
                                | tcl_registry::ArgRole::Expr
                                | tcl_registry::ArgRole::LambdaLiteral
                                | tcl_registry::ArgRole::CommandPrefix
                        )
                    });
                Some((valid, subsequent))
            })
            .flatten()
        else {
            break;
        };
        if !valid {
            break;
        }
        operations.push(Arc::new(advice.clone()));
        reads.extend(current_reads.into_iter().map(Arc::new));
        result.insert(
            offset,
            OriginalLogicalParameterSourcePrefix {
                input: input.clone(),
                scope: Arc::clone(body.scope()),
                formal: body.formal_original().clone(),
                parameters: body.parameters().to_vec(),
                operations: operations.clone(),
                reads: reads.clone(),
            },
        );
        if !subsequent {
            break;
        }
    }
    Some(result)
}

fn formal_source_reads(
    original: &[NativeWord],
    body: &OriginalLogicalProcedureBody,
) -> Option<Vec<ConditionalFormalSourceRead>> {
    let mut reads = Vec::new();
    for word in original {
        let arena = word.executable_parts();
        if arena
            .all_parts()
            .all(|part| matches!(part.part, ExecutablePart::Text(_)))
        {
            continue;
        }
        let [part] = arena.list(arena.root()) else {
            return None;
        };
        if !matches!(part.part, ExecutablePart::Variable { index: None, .. }) {
            return None;
        }
        let spelling = std::str::from_utf8(arena.bytes(part.span)?).ok()?;
        let name =
            tcl_syntax::naming::whole_scalar_variable_reference_name(spelling, word.config())?;
        let mut parameters = body
            .parameters()
            .iter()
            .enumerate()
            .filter(|(_, formal)| formal.name == name);
        let (parameter, _) = parameters.next()?;
        parameters.next().is_none().then_some(())?;
        reads.push(ConditionalFormalSourceRead {
            original: word.clone(),
            parameter,
        });
    }
    Some(reads)
}
