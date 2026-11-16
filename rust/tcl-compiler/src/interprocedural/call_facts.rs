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

//! Facts from one call statement: side effects, call-graph edges, scope and `upvar` aliases and code-bearing arguments.

use super::body_scan::possible_body_id;
use super::call_targets::command_prefix_head;
use super::parameter_observations::note_original_command_params;
use super::positioned_invocations::scan_candidate_method_callbacks;
use super::substitution_calls::{scan_source_for_calls, scan_value_substitutions};
use super::{LocalFacts, ProcArgTrait, ScanCtx};
use crate::side_effects::{EffectRegion, classify_side_effects_in};
use std::collections::HashSet;

fn scan_literal_command_prefix_edges(
    resolved: &str,
    args: &[String],
    arg_strs: &[&str],
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
) {
    for (idx, _appended) in ctx
        .surface()
        .command_prefixes(resolved, arg_strs)
        .into_iter()
        .filter(|_| !ctx.positioned_substitutions)
    {
        if let Some(word) = args
            .get(idx)
            .and_then(|a| command_prefix_head(ctx.registry, a))
            && is_plain_proc_name(&word)
            && let Some(target) = possible_body_id(&word, ctx)
        {
            facts.procedure_calls.push((target, None));
            facts.direct_calls.insert(target);
        }
    }
}

/// Process a `Statement::Call` for interprocedural facts: side-
/// effects classification, internal-call resolution, and param
/// trait inference.  Extracted from [`scan_statement`].
pub(super) fn scan_call_facts(
    command: &str,
    args: &[String],
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
) {
    let ScanCtx {
        registry,
        identities,
        ..
    } = ctx;
    // Effective metadata keeps its selected descriptor spelling; procedure
    // targets independently require their original source receipts.
    let resolved: &str = identities.resolve_unpositioned(command).spec_name();
    let invokes_named_proc = registry.get(resolved).is_some_and(|spec| {
        spec.traits
            .contains(tcl_registry::Traits::INVOKES_USER_PROC)
    });
    let internal_target =
        if invokes_named_proc && let Some(name) = args.first().filter(|n| is_plain_proc_name(n)) {
            possible_body_id(name, ctx)
        } else {
            possible_body_id(command, ctx)
        };

    // Command-prefix callbacks (`lsort -command myCompare`, `trace add … cb`,
    // `interp alias {} a {} target`) are call edges too: the referenced proc is
    // reachable, so a callback-only proc is not dead (O124) and appears in the
    // call graph. Registry-driven via `command_prefixes` — no command-name
    // matching here. Literal single-identifier heads only (`is_plain_proc_name`
    // rejects dynamic `$cb` / bracketed heads), mirroring the reference
    // extractor's bareword guard.
    let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
    scan_literal_command_prefix_edges(resolved, args, &arg_strs, ctx, facts);

    let words: Vec<_> = arg_strs
        .iter()
        .map(|value| tcl_registry::InvocationWord::Literal(value))
        .collect();
    if !ctx.positioned_substitutions {
        scan_candidate_method_callbacks(
            tcl_syntax::naming::whole_scalar_variable_reference_name(
                command,
                ctx.source_module.lexer_config,
            )
            .unwrap_or(command),
            &arg_strs,
            &words,
            ctx,
            facts,
        );
    }

    // A call that resolves to an internal proc contributes ONLY a
    // call-graph edge — its purity / effects flow through the
    // interprocedural fixpoints from the callee's summary, so the
    // command's own side-effect classification is NOT applied locally.
    // Non-internal commands apply their classified side effects.
    if let Some(target) = &internal_target {
        facts
            .procedure_calls
            .push((*target, (!invokes_named_proc).then(|| args.to_vec())));
        facts.direct_calls.insert(*target);
    } else {
        // Side-effect classification is dialect-agnostic here
        // (`classify_side_effects_in` is called with no dialect): a
        // command's effect profile reflects what it *does*,
        // not which dialect it is valid in. (The document `dialect` still
        // drives the lexer above so `[cmd …]` / bodies tokenise correctly.)
        // This is why e.g. `log`/`puts` resolve to their LOG_IO/FILE_IO
        // hints — impure but region-free — even under a Tcl dialect.
        // Classified against the document's surface, so a stub's `-pure`
        // or `-mutator` states the effect a catalogue spec would.
        let surface = ctx.surface();
        let ci = if ctx.source_input.is_some() || ctx.current_tokens.is_some() {
            // Original positioned invocations were consumed first by
            // scan_positioned_registry_facts and its Normal handler owner.
            // A retained-source fallback cannot replace that missing receiver
            // with the legacy available-spec hint walk.
            classify_side_effects_in(
                &surface,
                resolved,
                args,
                None,
                Some(&crate::side_effects::CalleeSummary {
                    effect_reads: EffectRegion::UNKNOWN_STATE,
                    effect_writes: EffectRegion::UNKNOWN_STATE,
                    pure: false,
                }),
            )
        } else {
            classify_side_effects_in(&surface, resolved, args, None, None)
        };
        if ci.dynamic_barrier {
            facts.param_observations_incomplete = true;
            facts.has_barrier = true;
            facts.mark_impure();
            facts.effect_reads |= EffectRegion::UNKNOWN_STATE;
            facts.effect_writes |= EffectRegion::UNKNOWN_STATE;
        }
        let (r, w) = ci.to_effect_regions();
        facts.effect_reads |= r;
        facts.effect_writes |= w;
        if w.intersects(EffectRegion::GLOBAL_STATE) {
            facts.writes_global = true;
        }
        if !ci.pure {
            facts.mark_impure();
        }
        // A command the document declares is known — its declaration is
        // a workspace-authored fact, classified above — so only a name
        // neither the catalogue nor the document knows is an unknown call.
        if registry.get(resolved).is_none() && !surface.declares(resolved) {
            facts.param_observations_incomplete = true;
            facts.has_unknown_calls = true;
            facts.mark_impure();
        }
    }

    note_original_command_params(
        ctx,
        if internal_target.is_some() {
            ProcArgTrait::ForwardedToCallee
        } else {
            ProcArgTrait::Passthrough
        },
        facts,
    );
}

/// Call-by-name alias-pair handling (`upvar`): the level and the `otherVar
/// myVar` pairs are the head's frame-effect declaration's, the level word
/// present by argument-count parity.  Mark a `$param` *other* (caller-var-name
/// source) as [`ProcArgTrait::VarRead`] and — only when the level is the
/// caller's frame, where a write lands in the caller — record `local → param`
/// so a later write of `local` upgrades it to [`ProcArgTrait::VarWrite`].  A
/// `$param` *local* name (the binding target) is itself a write of that
/// param's value. Every local linked is recorded; whether each pair links a
/// plain local to the place a parameter names one frame up is the answer.
fn handle_upvar_aliases(
    effect: tcl_registry::frame_effect::FrameEffectSpec,
    args: &[String],
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
) -> bool {
    let (level, pairs) = effect.resolve_in(args, ctx.registry);
    let caller_frame = level.is_caller_frame();
    let mut names_only = caller_frame && !pairs.is_empty() && pairs.len().is_multiple_of(2);
    for [other_var, my_var] in pairs.as_chunks::<2>().0 {
        facts.linked_locals.insert(my_var.clone());
        names_only &= tcl_syntax::naming::whole_scalar_variable_reference_name(
            other_var,
            ctx.source_module.lexer_config,
        )
        .is_some_and(|name| ctx.params.contains(name))
            && is_plain_local_name(my_var)
            && !ctx.params.contains(my_var);
        if let Some(other_vn) = tcl_syntax::naming::whole_scalar_variable_reference_name(
            other_var,
            ctx.source_module.lexer_config,
        ) && ctx.params.contains(other_vn)
        {
            facts
                .param_trait_flags
                .entry(other_vn.to_owned())
                .or_default()
                .insert(ProcArgTrait::VarRead);
            if caller_frame {
                facts
                    .upvar_aliases
                    .insert(my_var.clone(), other_vn.to_owned());
            }
        }
        if let Some(my_vn) = tcl_syntax::naming::whole_scalar_variable_reference_name(
            my_var,
            ctx.source_module.lexer_config,
        ) && ctx.params.contains(my_vn)
        {
            facts
                .param_trait_flags
                .entry(my_vn.to_owned())
                .or_default()
                .insert(ProcArgTrait::VarWrite);
        }
    }
    names_only
}

/// Whether `word` spells a plain local variable: a name of letters, digits
/// and underscores, neither qualified nor an array element.
pub(super) fn is_plain_local_name(word: &str) -> bool {
    word.chars()
        .next()
        .is_some_and(|first| first.is_alphabetic() || first == '_')
        && word.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// Upgrade the param aliased by `name` (if any) to
/// [`ProcArgTrait::VarWrite`] — a write through a level-1 upvar alias is
/// a write-back to the caller's variable.
pub(super) fn mark_upvar_alias_write(name: &str, facts: &mut LocalFacts) {
    if let Some(param) = facts.upvar_aliases.get(name).cloned() {
        facts
            .param_trait_flags
            .entry(param)
            .or_default()
            .insert(ProcArgTrait::VarWrite);
    }
}

/// Gather interprocedural facts from a `Statement::Call`: scope-aliasing
/// declarations (`global` / `variable` / `upvar #0`), global / upvar write-back
/// detection, and the per-call side-effect / call-graph facts. Extracted from
/// [`scan_statement`].
pub(super) fn scan_call_statement(
    command: &str,
    args: &[String],
    defs: &[String],
    tokens: Option<&crate::ir::CommandTokens>,
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
) {
    // Track scope-aliasing declarations (`global` / `variable` / `upvar #0`)
    // so a later bare write to an aliased name counts as `writes_global`.
    // Declaring is not writing, so handle the declaration before the
    // defs-based write check.
    let alias_pairs = ctx
        .registry
        .frame_effect(command)
        .filter(|effect| effect.layout == tcl_registry::frame_effect::FrameArgLayout::AliasPairs);
    let alias_names = if ctx.source_input.is_some() || tokens.is_some() || ctx.declared.is_some() {
        Some(GlobalAliasNameCoverage {
            names: HashSet::new(),
            unknown: true,
        })
    } else {
        global_alias_names(command, args, ctx.registry)
    };
    match alias_names {
        Some(alias_names) => {
            facts.writes_global |= alias_names.unknown;
            facts.global_aliases.extend(alias_names.names);
        }
        None => {
            // A non-declaration command that writes a `::`-qualified
            // or global-aliased variable (`append g`, `lappend g`,
            // `dict set ::cfg ...`) mutates caller-visible state.
            if defs
                .iter()
                .any(|n| tcl_syntax::naming::is_absolute(n) || facts.global_aliases.contains(n))
            {
                facts.writes_global = true;
            }
        }
    }
    // Call-by-name: record `upvar` aliases, and treat any command writing a
    // level-1 upvar alias (`append` / `lappend` / `lassign` … via `defs`) as a
    // write-back to the caller's variable.
    let links_only = if let Some(effect) = alias_pairs {
        handle_upvar_aliases(effect, args, ctx, facts)
    } else {
        // An alias-pair call itself is excluded — its `defs` are the locals it
        // *defines* (aliases), not writes.
        for d in defs {
            mark_upvar_alias_write(d, facts);
        }
        false
    };
    let beyond = facts.impure_beyond_links;
    scan_call_facts(command, args, ctx, facts);
    // Linking a local to the place a parameter names one frame up does
    // nothing the caller sees by itself: what the body does through the link
    // is what the caller sees.
    if links_only {
        facts.impure_beyond_links = beyond;
    }
    scan_role_code_arguments(command, args, tokens, ctx, facts);
}

/// Recurse into the code-bearing arguments of a statement that carries them
/// as opaque words, recording the calls they make as edges of the enclosing
/// unit.
///
/// A command whose script or expression argument has no dedicated lowering —
/// `time {…}`, and every declared `script:body` / `cond:expr` — reaches the
/// IR as a plain `Statement::Call` or `Statement::Barrier` holding its own
/// words. A barrier makes the call's *effects* opaque; neither shape makes
/// the code unreadable, and the procedures that code calls are exactly as
/// reachable as those in an `eval` body, which lowering splices inline and
/// this scan already walks. A callback proc reached only this way is live
/// code, not a call-graph leaf.
///
/// [`ArgRole::Body`] words are scanned as scripts and [`ArgRole::Expr`] words
/// for the `[cmd …]` substitutions the expression engine re-evaluates, which
/// is what [`scan_source_for_calls`] does for the same two roles.
/// [`ArgRole::LambdaLiteral`] stays with that scanner alone: splitting a
/// `{params body ?ns?}` list needs the word's token, and a statement carries
/// only its text.
///
/// Code that does not run in the caller's frame *and* namespace is left
/// alone. Lowering registers those as their own body units, which the
/// call-site scan visits under the namespace they actually target; walking
/// one here would instead invent an edge to a same-named proc in the
/// caller's namespace (the rule issues #977 / #980 set for the call-site
/// evidence scan).
///
/// Each word starts a fresh [`MAX_BRACKET_TEXT_DEPTH`] scan (depth 0), like
/// the `[cmd …]` substitution scans beside it: that counter bounds recursion
/// *within* re-segmented text, and is not the statement-tree depth the caller
/// is carrying.
pub(super) fn scan_role_code_arguments(
    command: &str,
    args: &[String],
    tokens: Option<&crate::ir::CommandTokens>,
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
) {
    use tcl_registry::prelude::Traits;
    let surface = ctx.surface();
    let resolved: &str = ctx.identities.resolve_unpositioned(command).spec_name();
    let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
    let context = ctx.metadata_context();
    let structured = tokens.map(|tokens| {
        crate::registry_invocation::resolve_command_tokens_with_metadata_context(
            ctx.registry,
            context.as_deref().map(Into::into),
            tokens,
        )
    });
    let roles;
    let traits;
    if let Some(structured) = structured {
        let Ok(crate::registry_invocation::RegistryInvocationResolution::Resolved(resolved)) =
            structured
        else {
            return;
        };
        if !resolved.arg_roles_complete {
            return;
        }
        traits = resolved.traits;
        roles = resolved
            .arg_roles
            .iter()
            .map(|&(index, role)| (resolved.argument_offset + usize::from(index), role))
            .collect::<Vec<_>>();
    } else {
        traits = ctx.registry.invocation_traits(resolved, &arg_strs, None);
        roles = [
            tcl_registry::ArgRole::NamespaceName,
            tcl_registry::ArgRole::Name,
            tcl_registry::ArgRole::Body,
            tcl_registry::ArgRole::Expr,
        ]
        .into_iter()
        .flat_map(|role| {
            surface
                .arg_indices_for_role(resolved, &arg_strs, role)
                .into_iter()
                .map(move |index| (index, role))
        })
        .collect();
    }
    if traits.intersects(
        tcl_registry::traits::FRAME_REACH_TRAITS
            .union(Traits::DEFINES_PROCEDURE)
            .union(Traits::DECLARES_NAMESPACE),
    ) {
        return;
    }
    // An absolutely-spelled name / namespace word says the same thing the
    // traits above do for the commands that carry neither: the code resolves
    // somewhere other than here.
    if [
        tcl_registry::ArgRole::NamespaceName,
        tcl_registry::ArgRole::Name,
    ]
    .into_iter()
    .flat_map(|wanted| {
        roles
            .iter()
            .filter(move |(_, role)| *role == wanted)
            .map(|(index, _)| *index)
    })
    .filter_map(|index| args.get(index))
    .any(|name| tcl_syntax::naming::is_absolute(name))
    {
        return;
    }
    for index in roles
        .iter()
        .filter(|(_, role)| *role == tcl_registry::ArgRole::Body)
        .map(|(index, _)| *index)
    {
        if let Some(body_text) = args.get(index) {
            scan_source_for_calls(body_text, ctx, facts, 0);
        }
    }
    for index in roles
        .iter()
        .filter(|(_, role)| *role == tcl_registry::ArgRole::Expr)
        .map(|(index, _)| *index)
    {
        if let Some(expr_text) = args.get(index) {
            scan_value_substitutions(strip_one_brace_layer(expr_text), ctx, facts, 0);
        }
    }
}

pub(super) fn scan_unproved_invocation(args: &[String], ctx: ScanCtx<'_>, facts: &mut LocalFacts) {
    facts.param_observations_incomplete = true;
    note_original_command_params(ctx, ProcArgTrait::Passthrough, facts);
    facts.has_barrier = true;
    facts.has_unknown_calls = true;
    facts.local_pure = false;
    facts.effect_reads |= EffectRegion::UNKNOWN_STATE;
    facts.effect_writes |= EffectRegion::UNKNOWN_STATE;
    for arg in args {
        scan_value_substitutions(arg, ctx, facts, 0);
    }
}

/// An expression operand's inner text: `expr {…}`'s brace quoting suppresses
/// substitution at the word level, but the expression engine re-evaluates the
/// contents, so a `[cmd …]` inside is a real call. An unbraced operand
/// (`[q]`, `$x`) is already its own inner text.
pub(super) fn strip_one_brace_layer(word: &str) -> &str {
    word.strip_prefix('{')
        .and_then(|inner| inner.strip_suffix('}'))
        .unwrap_or(word)
}

#[derive(Default)]
pub(super) struct GlobalAliasNameCoverage {
    pub(super) names: HashSet<String>,
    pub(super) unknown: bool,
}

/// Explicit standalone alias candidates; uncertainty is separate from legal empty data.
pub(super) fn global_alias_names(
    command: &str,
    args: &[String],
    registry: &tcl_registry::CommandRegistry,
) -> Option<GlobalAliasNameCoverage> {
    use tcl_registry::state_transition::{
        CallerFrameSelection, StateTransition, VariableAliasTarget,
    };
    use tcl_registry::{InvocationWord, InvocationWords};
    let arguments = args
        .iter()
        .map(|name| {
            if crate::dynamic_names::names_a_dynamic_variable(name) || name.contains('\0') {
                InvocationWord::Dynamic
            } else {
                InvocationWord::Literal(name)
            }
        })
        .collect::<Vec<_>>();
    let schema = registry
        .resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal(command), &arguments)
                .with_profile(registry.profile()),
            registry.own_surface_query(),
        )
        .resolved()?;
    let transitions = schema.state_transitions();
    let mut coverage = GlobalAliasNameCoverage::default();
    let mut selected = false;
    for fact in transitions.facts() {
        let StateTransition::VariableCellAlias(alias) = &fact.transition else {
            continue;
        };
        let visible = match &alias.target {
            VariableAliasTarget::Global { .. }
            | VariableAliasTarget::CurrentNamespace { .. }
            | VariableAliasTarget::Namespace { .. } => true,
            VariableAliasTarget::CallerSelectedFrame { frame, .. } => match frame {
                CallerFrameSelection::DefaultCaller => false,
                CallerFrameSelection::Explicit(level) => {
                    if let Some(level) = level.literal() {
                        let level = schema.words.arguments().dialect().map_or_else(
                            || tcl_registry::frame_effect::FrameLevel::parse(level),
                            |dialect| {
                                tcl_registry::frame_effect::FrameLevel::parse_for_dialect(
                                    level, dialect,
                                )
                            },
                        );
                        match level {
                            Some(level) if level.is_global_frame() || level.is_current_frame() => {
                                true
                            }
                            Some(tcl_registry::frame_effect::FrameLevel::Dynamic) | None => {
                                coverage.unknown = true;
                                true
                            }
                            Some(_) => false,
                        }
                    } else {
                        coverage.unknown = true;
                        true
                    }
                }
            },
        };
        if !visible {
            continue;
        }
        selected = true;
        match alias.local.literal() {
            Some(name) => {
                coverage.names.insert(
                    tcl_syntax::naming::split_element_ref(name)
                        .map_or(name, |(root, _)| root)
                        .to_owned(),
                );
            }
            None => coverage.unknown = true,
        }
    }
    selected.then_some(coverage)
}

/// True when `text` could be a plain procedure name — rejects
/// argument shapes that would make the ``call`` indirection
/// dynamic (variable substitutions, command substitutions, etc.).
pub(super) fn is_plain_proc_name(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b':'))
}
