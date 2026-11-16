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

//! What a call over literal words resolves to: cell updates, keyed dictionary updates, iteration plans and whether a body absorbs its completion.

use super::super::argument_words::{typed_node_commands, view_of, word_of};
use super::StructureInputs;
use tcl_registry::hooks::LoweringHookId;
use tcl_registry::value_transfer::{
    AnalysisContext, Budget, CompletionOutcome, EvalAnswer, ExactValue, FactDomain,
    InvocationLayout, IterationPlan, OperandId, PlanAnswer, RepresentationEvidence, StoreOutcome,
    TargetId, TransferAnswer,
};
use tcl_registry::{
    ArgRole, CommandRegistry, InvocationWord, InvocationWords, ResolvedInvocation, TclType,
};

/// Whether a typed `Incr` over an absent place completes under every
/// release `registry`'s profile names — the release rule's
/// `creates_absent`, read from the declaration the typed node lowers from:
/// 8.5 onwards create the cell, 8.4 raises `can't read`. `false` when the
/// declaration says nothing.
pub(crate) fn typed_incr_creates_absent(registry: &CommandRegistry) -> bool {
    typed_node_commands(registry, LoweringHookId::Incr)
        .first()
        .and_then(|command| registry.get(command))
        .is_some_and(|spec| {
            match tcl_registry::value_transfer::resolve_semantics(spec, None, None) {
                tcl_registry::value_transfer::ResolvedSemantics::Derived(
                    tcl_registry::value_transfer::DerivedSemantics::CellUpdate(update),
                ) => update.creates_absent_under(
                    &tcl_registry::value_transfer::TargetSemantics::of(registry.profile()),
                ),
                _ => false,
            }
        })
}

/// The dictionary a keyed update of one variable stores (`dict set`,
/// `unset`, `incr`, `append`, `lappend`), when every word of `head args…` is
/// literal and `prior` proves the variable's value before the call: the
/// registry's declared semantics evaluate the update over the prior value.
/// `None` where the call reads no prior of its one written variable, the
/// prior is unknown, the evaluation declines or does not complete normally,
/// or the route does not construct a dictionary; a string or list update's
/// value has its own representation-aware owner.
pub(crate) fn literal_keyed_update(
    registry: &CommandRegistry,
    head: &str,
    args: &[&str],
    prior: impl FnOnce(&str) -> Option<String>,
) -> Option<String> {
    let words: Vec<InvocationWord<'_>> =
        args.iter().copied().map(InvocationWord::Literal).collect();
    let resolved = registry
        .resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal(head), &words),
            registry.own_surface_query(),
        )
        .resolved()?;
    let semantics = resolved.semantics.value.semantics()?;
    let view = view_of(&resolved, args, &words, InvocationLayout::Source);
    let mut written = view.operands_with_role(ArgRole::VarWrite);
    let target = written.next()?;
    if written.next().is_some() {
        return None;
    }
    let (subcommand, operands) = match (view.subcommand, view.argument_offset) {
        (None, 0) => (None, args),
        (Some(subcommand), 1) => (Some(subcommand), args.get(1..)?),
        _ => return None,
    };
    let mut inputs = tcl_registry::value_transfer::LiteralInputs::new(
        resolved.canonical_command,
        subcommand,
        operands,
        registry.profile(),
    );
    for (index, operand) in view.operands.iter().enumerate() {
        if let Some(role) = operand.role {
            inputs = inputs.with_role(OperandId(index), role);
        }
    }
    if semantics.incoming_targets(&inputs) != [TargetId(target)] {
        return None;
    }
    let name = args.get(target.0)?;
    let inputs = inputs.with_prior(name, ExactValue::text(prior(name)?));
    let EvalAnswer::Evaluated(outcome) = semantics.evaluate(&inputs, &mut Budget::evaluation())
    else {
        return None;
    };
    if outcome.completion != CompletionOutcome::Normal || !outcome.nested_writes.is_empty() {
        return None;
    }
    let mut value = None;
    for store in outcome.ordered_stores {
        match store {
            StoreOutcome::Write {
                target: at,
                value: stored,
            } if at == TargetId(target)
                && stored.representation == RepresentationEvidence::Constructed(TclType::Dict) =>
            {
                value = Some(String::from_utf8(stored.bytes).ok()?);
            }
            StoreOutcome::Preserve { .. } => {}
            _ => return None,
        }
    }
    value
}

/// The cell update a `head args…` call resolves to under `registry`, when
/// its declared plan is one: the operation and its target operand. The one
/// question every consumer of the write-chain shape asks (the chain fold,
/// the `append` usage check, the list-length walk), answered from the
/// registry's declaration rather than a command's spelling.
pub(crate) fn resolved_cell_update(
    registry: &CommandRegistry,
    head: &str,
    args: &[String],
) -> Option<(tcl_registry::native_lowering::CellUpdate, OperandId)> {
    let texts: Vec<&str> = args.iter().map(String::as_str).collect();
    let words: Vec<InvocationWord<'_>> = texts.iter().copied().map(word_of).collect();
    let resolved = registry
        .resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal(head), &words),
            registry.own_surface_query(),
        )
        .resolved()?;
    let semantics = resolved.semantics.value.semantics()?;
    let context = AnalysisContext::detached(registry.profile());
    let inputs = StructureInputs::new(
        view_of(&resolved, &texts, &words, InvocationLayout::Source),
        &context,
    );
    match semantics.structure(&inputs) {
        PlanAnswer::CellReadModifyWrite {
            target, operation, ..
        } => Some((operation, target.0)),
        _ => None,
    }
}

/// Whether the plan a `head args…` call declares under `registry` absorbs
/// every completion of its body (`catch`'s), so a `return` the body runs
/// ends there: what the return reading asks of a call holding a script
/// word, answered from the registry's declaration rather than a command's
/// spelling.
pub(crate) fn resolved_body_absorbs_completion(
    registry: &CommandRegistry,
    head: &str,
    args: &[String],
) -> bool {
    let texts: Vec<&str> = args.iter().map(String::as_str).collect();
    let words: Vec<InvocationWord<'_>> = texts.iter().copied().map(word_of).collect();
    let Some(resolved) = registry
        .resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal(head), &words),
            registry.own_surface_query(),
        )
        .resolved()
    else {
        return false;
    };
    let Some(semantics) = resolved.semantics.value.semantics() else {
        return false;
    };
    let context = AnalysisContext::detached(registry.profile());
    let inputs = StructureInputs::new(
        view_of(&resolved, &texts, &words, InvocationLayout::Source),
        &context,
    );
    matches!(
        semantics.structure(&inputs),
        PlanAnswer::Body {
            completion: tcl_registry::value_transfer::CompletionProtocol::CatchAll { .. },
            ..
        }
    )
}

/// The declaration a `head args…` call over literal words resolves to under
/// `registry`, its form selected by those words, with the role the resolver
/// gives each word: what a consumer runs over the call's own words, or asks
/// whether the call writes its value word
/// ([`tcl_registry::value_transfer::ResolvedSemantics::writes_value_word`]).
pub(crate) fn resolved_literal_semantics(
    registry: &CommandRegistry,
    head: &str,
    args: &[&str],
) -> Option<(
    tcl_registry::value_transfer::ResolvedSemantics,
    Vec<(usize, ArgRole)>,
)> {
    let words: Vec<InvocationWord<'_>> =
        args.iter().copied().map(InvocationWord::Literal).collect();
    let resolved = registry
        .resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal(head), &words),
            registry.own_surface_query(),
        )
        .resolved()?;
    let roles = view_of(&resolved, args, &words, InvocationLayout::Source)
        .operands
        .iter()
        .enumerate()
        .filter_map(|(index, operand)| operand.role.map(|role| (index, role)))
        .collect();
    Some((resolved.semantics.value, roles))
}

/// The iteration plan a `head args…` call declares in its source layout
/// under `registry`, when it declares one: which word is a counted loop's
/// start script, condition and step script, or a conditional loop's
/// condition, and which is the body. The question the loop checks ask —
/// W240 to W242's counter and IRULE5003's bound — answered from the
/// registry's declaration rather than a command's spelling.
pub(crate) fn resolved_iteration_plan(
    registry: &CommandRegistry,
    head: &str,
    args: &[String],
) -> Option<IterationPlan> {
    let texts: Vec<&str> = args.iter().map(String::as_str).collect();
    let words: Vec<InvocationWord<'_>> = texts.iter().copied().map(word_of).collect();
    let resolved = registry
        .resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal(head), &words),
            registry.own_surface_query(),
        )
        .resolved()?;
    selected_iteration_plan(&resolved, args, registry.profile())
}

/// Project the already selected source descriptor without resolving its name again.
pub(crate) fn selected_iteration_plan(
    resolved: &ResolvedInvocation<'_, '_>,
    args: &[String],
    profile: Option<&'static tcl_dialect::DialectProfile>,
) -> Option<IterationPlan> {
    let arguments = resolved.words.arguments();
    if arguments.exact_argv_len() != Some(args.len()) {
        return None;
    }
    let texts: Vec<_> = args.iter().map(String::as_str).collect();
    let words = (0..args.len())
        .map(|index| arguments.get(index))
        .collect::<Option<Vec<_>>>()?;
    let semantics = resolved.semantics.value.semantics()?;
    let context = AnalysisContext::detached(profile);
    let inputs = StructureInputs::new(
        view_of(resolved, &texts, &words, InvocationLayout::Source),
        &context,
    );
    match semantics.structure(&inputs) {
        PlanAnswer::Iterate(plan) => Some(plan),
        _ => None,
    }
}

/// The interval domain's model of a typed cell update with `operands`
/// post-head words: the registry-described operation the domain
/// interprets, from the specialisation the descriptor derives — the typed
/// IR node is that descriptor by construction, so no invocation is
/// resolved and no command is named.
pub(crate) fn cell_update_range_model(
    update: tcl_registry::native_lowering::CellUpdate,
    operands: usize,
) -> Option<tcl_registry::value_transfer::RangeModel> {
    use tcl_registry::value_transfer::{CommandSemantics as _, LiteralInputs};
    let semantics = tcl_registry::value_transfer::cell_update::CellUpdateSemantics {
        update,
        creates_absent: None,
    };
    let args: Vec<&str> = std::iter::repeat_n("", operands).collect();
    let inputs = LiteralInputs::new("", None, &args, None);
    match semantics.transfer(FactDomain::Range, &inputs, &mut Budget::evaluation()) {
        TransferAnswer::Range(model) => Some(model),
        _ => None,
    }
}
