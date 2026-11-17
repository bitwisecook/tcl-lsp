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

//! Recording one call: the callee a command word names or dispatches to and the argument words it receives.

use super::body_invocations::record_invocation_body_evidence;
use super::scope_var_facts::{WordValues, word_values};
use super::{CallSiteEvidence, CallSiteScanCtx, CallerFrame};
use crate::ir::Statement;
use crate::naming::is_dynamic_word;
use std::collections::HashSet;
use tcl_registry::{ArgRole, CommandRegistry, Traits};

/// Record one call site's literal-argument evidence into `out`, then recurse
/// into any `ArgRole::Body` argument of `command` (regardless of whether
/// `command` itself is a user proc) — a nested script embedded in a nested
/// script embedded in a nested script, and so on, up to
/// the shared source-walk depth budget.
///
/// `catch { isEven 4 }`, a non-exact `switch` arm, a literal `uplevel
/// {…}` / `apply {{…} {…}}` body, and friends all carry their nested script
/// as one opaque *argument string* to a builtin (`catch`, `switch`,
/// `uplevel`, `apply`) that is never itself a user proc — so a flat,
/// one-level `Statement::Call`/`Statement::Barrier` walk resolves `catch`
/// (finds no matching proc, moves on) and never notices `isEven 4` sitting
/// inside its body argument at all. That's a *second* proc call this scan
/// cannot see, exactly like the namespace-resolution gap: an invisible call
/// site with a differing argument silently vanishes from
/// the evidence's "every caller agrees" answer.
///
/// The command surface already knows which argument position of which
/// command is a script body (`ArgRole::Body`, driving the identical recursive
/// call-graph walk in [`crate::interprocedural`] and the `BODY`-role scans in
/// `ir_helpers.rs` / `place_bridge.rs` / `ssa.rs`) — so this reuses that one
/// fact via
/// [`tcl_registry::model::DocumentCommandSurface::arg_indices_for_role`] and
/// the shared [`crate::segmenter`] rather than hand-rolling a second "which
/// commands embed scripts" list here.
/// Whether the unseen code an `<unseen-call>` marker stands for is already
/// accounted for by its host call (the nearest preceding non-marker
/// statement): a call to one retained in-module procedure, whose body is
/// scanned as its own caller, or a computed head, whose dispatch the host's
/// own scan enumerates or withdraws.
pub(super) fn unseen_code_is_scanned_host(
    func: &crate::cfg::Function,
    block_id: crate::cfg::BlockId,
    statements: &[Statement],
    marker: usize,
) -> bool {
    let Some(host) = (0..marker).rev().find(|&index| {
        statements[index]
            .tokens()
            .is_none_or(|tokens| tokens.synthetic.is_none())
    }) else {
        return false;
    };
    if matches!(&statements[host], Statement::Call { command, .. } if is_dynamic_word(command)) {
        return true;
    }
    func.source_tokens_at(block_id, host)
        .and_then(|tokens| tokens.source_binding.as_ref())
        .and_then(crate::command_binding::SourceInvocationBinding::proved_target)
        .is_some_and(|target| {
            target.kind == crate::command_binding::BindingKind::Proc
                && !target.registry_backed
                && target.implementation_allocation.is_some()
        })
}

pub(super) fn record_call_site_evidence(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    caller: &CallerFrame<'_>,
    command: &str,
    args: &[String],
    tokens: Option<&crate::ir::CommandTokens>,
    depth: u32,
) {
    if tokens.is_some_and(|tokens| tokens.synthetic.is_some()) {
        return;
    }
    let Some(tokens) = tokens else {
        record_unenumerable_targets(out, ctx);
        return;
    };
    let Ok(context) = ctx.original_metadata(tokens, caller.metadata_owner) else {
        record_unenumerable_targets(out, ctx);
        return;
    };
    if tokens.source_binding.as_ref().is_some_and(|binding| {
        binding.runtime_reachability()
            == crate::command_binding::SourceRuntimeReachability::NotEntered
    }) {
        // Complete source execution coverage proves this dispatch was not
        // entered. Its independent compiler visitation still matters to
        // admission, but cannot introduce an arbitrary runtime caller.
        return;
    }
    record_original_callback_callers(out, ctx, Some(tokens), context);
    if let Some(invocation) =
        crate::registry_invocation::normal_user_procedure_invocation_with_metadata_context(
            ctx.registry,
            context,
            tokens,
        )
    {
        record_normal_user_procedure_call(out, ctx, tokens, &invocation);
        return;
    }
    // A dispatched command word (`$cmd args`) is resolved by *value*, not
    // skipped: the scope's own literal assignments (unioned, for a parameter,
    // with the literals its callers pass) give the set of names it may hold,
    // and each becomes an ordinary call site.  Skipping it would let a
    // dispatch reach a proc this scan had already seeded from its literal call
    // sites, silently unsoundly.  A word whose value set is not enumerable
    // withdraws every seed instead.
    record_invocation(
        out,
        ctx,
        caller,
        command,
        IndirectArgs::Words(args),
        Some(tokens),
    );
    if is_dynamic_word(command) {
        // Which of a computed head's arguments carry scripts, callbacks, or a
        // callee name is unknowable; the dispatch itself is already accounted
        // for above.
        return;
    }
    let invocation = context
        .filter(|metadata| {
            tokens.source_binding.as_ref().and_then(|binding| {
                binding.original_invocation_source_purpose(tokens, *metadata, ctx.registry)
            }) == Some(crate::command_binding::OriginalInvocationSourcePurpose::Logical)
        })
        .and_then(|metadata| {
            crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(
                ctx.registry,
                metadata,
                tokens,
            )
        })
        .or_else(|| {
            crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
                ctx.registry,
                context,
                tokens,
            )
        });
    let Some(invocation) = invocation else {
        if context
            .and_then(crate::registry_invocation::InvocationMetadataContext::source_analysis_input)
            .is_some_and(|input| {
                tokens.source_binding.as_ref().is_some_and(|binding| {
                    binding
                        .original_source_procedure_argument_roles(tokens, input, ctx.registry)
                        .is_some()
                })
            })
        {
            return;
        }
        record_unenumerable_targets(out, ctx);
        return;
    };
    record_indirect_callers(out, ctx, caller, &invocation, Some(tokens));
    record_invocation_body_evidence(out, ctx, caller, &invocation, tokens, context, depth);
}

fn record_normal_user_procedure_call(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    tokens: &crate::ir::CommandTokens,
    invocation: &crate::registry_invocation::NormalUserProcedureInvocation,
) {
    let Some(source) = tokens.source_binding.as_ref() else {
        return;
    };
    let callee = source.lookup_command_word(&invocation.target);
    if invocation.unknown_runtime || callee.unknown {
        record_unenumerable_targets(out, ctx);
    }
    for target in &callee.targets {
        if target.kind == crate::command_binding::BindingKind::Proc
            && (ctx.module.source_procedure_for_target(target).is_some()
                || ctx.known.contains(&target.command))
        {
            out.record_source_call(
                target.command.clone(),
                &target.prepended,
                invocation.arguments.len(),
                &invocation.arguments,
                target.implementation_allocation.as_ref(),
            );
        } else if !target.registry_backed {
            record_unenumerable_targets(out, ctx);
        }
    }
}

/// What a resolved callee receives at an invocation site: the call's own
/// argument words (an ordinary or dispatched call, and the tail for a
/// registry-declared user-proc invoker), or nothing this scan can see (a
/// callback prefix, whose arguments the runtime appends).
#[derive(Clone, Copy)]
pub(super) enum IndirectArgs<'a> {
    /// The callee receives exactly these argument words.
    Words(&'a [String]),
    /// The runtime appends arguments this scan cannot see.
    Unknowable,
}

pub(super) fn record_unenumerable_targets(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
) {
    out.record_unenumerable_caller(ctx.unenumerable_reach);
    for (id, procedure) in ctx.module.procedures.iter() {
        out.record_opaque_body(*id, &procedure.qualified_name);
    }
}

fn record_source_target(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    args: IndirectArgs<'_>,
    source: &crate::command_binding::SourceInvocationBinding,
    target: &crate::command_binding::SourceCommandTarget,
) {
    let local = ctx.module.source_procedure_for_target(target);
    if target.kind == crate::command_binding::BindingKind::Proc
        && (local.is_some() || ctx.known.contains(&target.command))
    {
        match args {
            IndirectArgs::Words(words) => out.record_source_call(
                target.command.clone(),
                &target.prepended,
                words.len(),
                &source.evaluated_argument_values,
                target.implementation_allocation.as_ref(),
            ),
            IndirectArgs::Unknowable => {
                if let Some((id, procedure)) = local {
                    out.record_opaque_body(id, &procedure.qualified_name);
                }
                out.record_opaque_caller(&target.command);
            }
        }
    } else if !target.registry_backed {
        record_unenumerable_targets(out, ctx);
    }
}

/// Attribute one invocation of `word` (a command name, possibly dispatched
/// through a variable) to every procedure it may reach.
///
/// A literal word reaches at most one procedure and costs one resolution.  A
/// dispatch word reaches every procedure in its value set — recorded as an
/// ordinary call site for each, which is what keeps the fix per-target
/// rather than a module-wide wildcard: `$cmd prod` where `cmd` is provably
/// `helper` still lets an agreeing seed stand, and a dispatch that provably
/// names some *other* procedure leaves this one's seed alone.  A word whose
/// values cannot be enumerated names a caller of *something* the scan cannot
/// identify, so it withdraws every seed in the unit.
fn record_invocation(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    caller: &CallerFrame<'_>,
    word: &str,
    args: IndirectArgs<'_>,
    tokens: Option<&crate::ir::CommandTokens>,
) {
    if let Some(source) = tokens.and_then(|tokens| tokens.source_binding.as_ref())
        && tokens.is_some_and(|tokens| tokens.argv_texts.first().is_some_and(|head| head == word))
    {
        // A selected native operation and later generic lookup are different
        // execution alternatives. Exact execution proof takes precedence over
        // a command table changed by an argument or an earlier chunk command.
        if let Some(target) = source.proved_execution_target() {
            record_source_target(out, ctx, args, source, target);
            return;
        }
        // Compiler rejection can stop before dispatch; it cannot introduce an
        // arbitrary caller. Only actual runtime lookup uncertainty widens the
        // call graph. The retained execution candidates still cover a captured
        // native operation and a possible later generic dispatch.
        if source.unknown {
            record_unenumerable_targets(out, ctx);
        }
        if source.may_be_absent {
            record_unresolved_word_dispatch(out, ctx, word, args);
        }
        for target in source.execution_targets() {
            record_source_target(out, ctx, args, source, target);
        }
        return;
    }
    // A dynamic word is the only thing that reads the previous round, so it
    // alone makes another round necessary.  A module without one converges
    // after a single walk.
    if is_dynamic_word(word) {
        out.consulted_value_sets = true;
    }
    let values = match word_values(ctx, caller, word) {
        WordValues::Literals(values) => values,
        WordValues::Unknown => {
            record_unenumerable_targets(out, ctx);
            return;
        }
    };
    let Some(source) = tokens.and_then(|tokens| tokens.source_binding.as_ref()) else {
        record_unenumerable_targets(out, ctx);
        return;
    };
    for name in &values {
        let binding = if !is_dynamic_word(word)
            && tokens
                .is_some_and(|tokens| tokens.argv_texts.first().is_some_and(|head| head == word))
        {
            source.clone()
        } else {
            source.lookup_command_word(name)
        };
        if binding.unknown {
            record_unenumerable_targets(out, ctx);
        }
        if binding.may_be_absent {
            record_unresolved_word_dispatch(out, ctx, name, args);
        }
        for target in &binding.targets {
            record_source_target(out, ctx, args, source, target);
        }
    }
}

/// The qualified name of the unresolved-command handler this unit defines,
/// or `None` when it defines none — the overwhelmingly common case.
///
/// Tcl routes every command word that resolves to nothing to a single
/// global handler, passing the word itself followed by that call's own
/// arguments (tclsh8.6/9.0-confirmed). A module that defines one therefore
/// has callers no scan of its *direct* call sites can enumerate, and a
/// coincidentally-uniform set of those direct calls would fold a parameter
/// the unresolved words genuinely vary.
///
/// Which command is the handler comes from
/// [`Traits::UNRESOLVED_COMMAND_HANDLER`], never a literal name here. The
/// lookup is global-scope only: a namespace-local `proc unknown` is *not*
/// consulted for unresolved words in that namespace — tclsh8.6/9.0 both
/// dispatch to `::unknown` regardless of the calling namespace.
///
/// [`CommandRegistry::commands_with_trait`] iterates a hash map, so the
/// candidates are sorted before the first match is taken. Exactly one
/// command carries the trait today (pinned by
/// `only_one_command_carries_the_unresolved_handler_trait`), but an
/// order-dependent answer would be a silent source of build-to-build drift
/// the day a dialect adds a second carrier.
pub(super) fn unresolved_command_handler<'a, S: std::hash::BuildHasher>(
    registry: &CommandRegistry,
    known: &'a HashSet<String, S>,
) -> Option<&'a str> {
    let mut candidates = registry.commands_with_trait(Traits::UNRESOLVED_COMMAND_HANDLER);
    candidates.sort_unstable();
    candidates.into_iter().find_map(|name| {
        let qualified = crate::naming::qualify("::", name);
        known.get(&qualified).map(String::as_str)
    })
}

/// Record the unresolved-command handler's own invocation for a literal
/// command word this scan could not resolve.
///
/// Only fires when the module defines a handler, and only for a word the
/// registry does not know either — everything else either resolves or is a
/// builtin. The word becomes the handler's first argument and the call's
/// own arguments follow, exactly as Tcl passes them.
///
/// **This is additional evidence, never a complete caller set.** The
/// handler is poisoned unconditionally in [`scan_cfg_callers`] the moment
/// the module defines one, so nothing recorded here can seed a fold. It can
/// only widen an already-unfoldable value set, which is what makes the two
/// known imprecisions harmless:
///
/// * *Over-inclusive.* A word bound by something this scan does not model —
///   a `TclOO` class command, an ensemble, a coroutine — also reads as
///   unresolved and contributes a value real Tcl never passes the handler.
/// * *Under-inclusive.* Most of the handler's real callers are words that
///   exist nowhere in the source at all, so no scan can name them.
///
/// An earlier revision claimed the residue "can only retract a fold, never
/// manufacture one". That was false while these dispatches were the
/// handler's *only* recorded call sites: with no other evidence, a
/// coincidentally-uniform set of invented words *was* a fold. The
/// unconditional poison is what makes the claim true.
pub(super) fn record_unresolved_word_dispatch(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    word: &str,
    args: IndirectArgs<'_>,
) {
    let Some(handler) = ctx.unresolved_handler else {
        return;
    };
    if ctx.registry.get(word).is_some() {
        return;
    }
    match args {
        IndirectArgs::Words(words) => {
            let mut dispatched = Vec::with_capacity(words.len() + 1);
            dispatched.push(word.to_owned());
            dispatched.extend_from_slice(words);
            out.record_call(handler.to_owned(), &dispatched);
        }
        IndirectArgs::Unknowable => out.record_opaque_caller(handler),
    }
}

/// Record the callers a statement creates *without* naming their arguments:
/// a deferred command prefix, and a rebinding of a known command's name.
///
/// Both are real call paths whose argument list this scan can never see —
/// `after 0 helper`, `trace add variable v write helper`, `-command helper`
/// all invoke `helper` with runtime-supplied words appended, and `rename
/// helper other` / `interp alias {} h {} helper` let a call reach `helper`
/// under a name no scan attributed to it. Left out of the evidence entirely
/// they would read as "no caller
/// disagrees". Recording them as opaque callers states the truth instead: a
/// call site exists whose arguments are unknown.
///
/// Command-surface-driven throughout — the callback positions come from
/// [`tcl_registry::ArgRole::CommandPrefix`]
/// ([`tcl_registry::model::DocumentCommandSurface::arg_indices_for_role`])
/// and the rebinding forms from
/// [`crate::alias::command_table_transitions`], so no command name appears
/// here.  Covers the `CommandPrefix` forms for both the in-unit and the
/// cross-unit scan.
fn record_indirect_callers(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    caller: &CallerFrame<'_>,
    invocation: &crate::registry_invocation::ResolvedStatementInvocation,
    tokens: Option<&crate::ir::CommandTokens>,
) {
    let args = invocation.arguments.as_slice();
    // Moving a binding invalidates source and destination names.
    // Registry transitions distinguish movement from definition.
    let tcl_registry::StateTransitionKnowledge::Declared(transitions) =
        &invocation.facts.state_transitions
    else {
        return;
    };
    if transitions.command_bindings().any(|transition| {
        if matches!(
            transition,
            tcl_registry::CommandBindingTransition::Define { .. }
        ) {
            return false;
        }
        let covered = tokens.zip(ctx.source_input).is_some_and(|(tokens, input)| {
            tokens.source_binding.as_ref().is_some_and(|binding| {
                binding
                    .original_covered_alias_transition(
                        tokens,
                        ctx.module,
                        input,
                        ctx.registry,
                        transition,
                    )
                    .is_some()
            })
        });
        !covered
    }) {
        for word in args {
            let Some(word) = word.as_deref() else {
                record_unenumerable_targets(out, ctx);
                continue;
            };
            if word.is_empty() || word.contains(['$', '[']) {
                continue;
            }
            record_invocation(out, ctx, caller, word, IndirectArgs::Unknowable, tokens);
        }
    }
}

/// A genuine source target is only an opaque caller: no fixed callback argv,
/// successful registration, reached frame or current lookup is supplied.
fn record_original_callback_callers(
    out: &mut CallSiteEvidence,
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    tokens: Option<&crate::ir::CommandTokens>,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) {
    let Some(tokens) = tokens else {
        return;
    };
    let rows = ctx.source_input.and_then(|input| {
        ctx.identities
            .original_source_callback_targets_for_tokens(input, tokens)
    });
    if let Some(rows) = rows {
        for row in rows {
            if let Some(id) = row.target().and_then(|target| target.original_ir_procedure_id(ctx.procedures)) {
                if let Some(procedure) = ctx.procedures.get(&id) {
                    out.record_opaque_body(id, &procedure.qualified_name);
                }
            } else if row.refusal() != Some(crate::command_binding::OriginalSourceCallbackProcedureRefusal::KnownSourceBarrier) {
                record_unenumerable_targets(out, ctx);
            }
        }
    } else if metadata
        .and_then(|context| {
            crate::registry_invocation::original_callback_invocation_with_metadata_context(
                ctx.registry,
                context,
                tokens,
            )
        })
        .is_some_and(|invocation| {
            invocation
                .facts
                .arg_roles
                .iter()
                .any(|(_, role)| *role == ArgRole::CommandPrefix)
        })
    {
        record_unenumerable_targets(out, ctx);
    }
}
