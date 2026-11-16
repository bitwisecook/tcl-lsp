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

//! What a statement defines: `defs_of` and its registry and barrier forms, the may-defs of an opaque `switch`, nested writes and the names a bound call defines.

use super::{SsaInvocationContext, is_effect_marker};
use crate::cfg::BlockId;
use crate::ir::{CommandTokens, Statement, WordExpr, WordPart};
use crate::var_resolve::{ResolveContext, VariableCellKey, canonical_variable_key};
use crate::variable_bindings::PointResolveContexts;
use rustc_hash::FxHashSet;
use std::collections::BTreeSet;
use tcl_registry::CommandRegistry;
use tcl_syntax::naming::unattributed_dynamic_written_variable_target as is_dynamic_write_target;

pub(super) fn bound_names(
    names: Vec<String>,
    context: &ResolveContext,
    registry: &CommandRegistry,
) -> Vec<VariableCellKey> {
    let mut bound = Vec::new();
    for name in names {
        if let Some(name) = canonical_variable_key(&name, context, registry)
            && !bound.contains(&name)
        {
            bound.push(name);
        }
    }
    bound
}

pub(super) fn bound_defs(
    statement: &Statement,
    block: BlockId,
    index: usize,
    points: &PointResolveContexts,
    selection: SsaInvocationContext<'_>,
) -> Vec<VariableCellKey> {
    let registry = selection.registry;
    let context = points.before_statement(block, index);
    let after = points.after_statement(block, index);
    if matches!(
        statement,
        Statement::Call { .. }
            | Statement::AssignConst { .. }
            | Statement::AssignValue { .. }
            | Statement::AssignExpr { .. }
            | Statement::Incr { .. }
    ) {
        let mut definitions: Vec<_> =
            crate::place_bridge::def_places_at(statement, block, index, points, registry)
                .iter()
                .filter_map(crate::var_resolve::canonical_binding_value_key)
                .collect();
        definitions.extend(
            crate::place_bridge::ssa_destruction_keys(
                statement,
                context,
                after,
                registry,
                points.source_metadata_owner().metadata_context(registry),
            )
            .into_iter()
            .map(|(key, _)| key),
        );
        let mut seen = FxHashSet::default();
        definitions.retain(|key| seen.insert(key.clone()));
        definitions
    } else {
        bound_names(
            defs_of_in_context(statement, Some(selection)),
            context,
            registry,
        )
    }
}

// Variable definition extraction

/// Extract variable names defined by an IR statement.
///
/// Handles assignments (`set`, `incr`), call defs, `trace add variable`
/// (via registry roles when `registry` is supplied), and `dict for`/
/// `dict map` barriers.
///
/// Pass `Some(&CommandRegistry)` when available so barrier defs route
/// through the registry's `ArgRole::VarWrite` query; pass
/// `None` for the string-match fallback used by the unit-test
/// helpers.
#[must_use]
pub fn defs_of(stmt: &Statement) -> Vec<String> {
    if !stmt.is_executable_invocation() {
        return Vec::new();
    }
    defs_of_with_registry(stmt, None)
}

/// Whether a barrier's command is an ensemble loop subcommand whose first
/// argument is the iteration-variable list (`::tcl::dict::for` / `::map`,
/// `::tcl::array::for`). Resolved from the registry's `loop_list_header`
/// flag on the subcommand: the barrier name's last segment is the
/// subcommand, the one before it the base command (`… ::tcl::dict::for` →
/// `dict for`). Callers without a registry (test helpers) fall back to the
/// suffix heuristic, mirroring the trace fallback below.
fn barrier_is_loop_list_header(command: &str, registry: Option<&CommandRegistry>) -> bool {
    let Some(registry) = registry else {
        let tail = crate::naming::written_command_tail(command.as_bytes());
        return crate::naming::is_qualified(command.as_bytes())
            && (tail == b"for" || tail == b"map");
    };
    let segments = crate::naming::qualifier_segments(command.as_bytes());
    let [.., base, sub] = segments.as_slice() else {
        return false;
    };
    let (Ok(base), Ok(sub)) = (std::str::from_utf8(base), std::str::from_utf8(sub)) else {
        return false;
    };
    registry
        .get(base)
        .and_then(|spec| spec.resolve_subcommand(sub))
        .is_some_and(|sub| sub.loop_list_header)
}

/// Whether a loop-variable-list word is **static** — its source performs no
/// substitution, so the word's text *is* the Tcl list of names the loop binds.
///
/// Dynamism is a property of the source word's *kind*, not of the bytes in its
/// value — the same rule the neighbouring `VarWrite` path applies via
/// [`CommandTokens::arg_is_braced_literal`].  A brace-quoted word substitutes
/// nothing, so `foreach {{$x}} {*}$spec {puts ${$x}}` binds the perfectly legal
/// variable named `$x`; testing the reconstructed word text for `$` / `[`
/// drops that name from the barrier's def set and reports a phantom
/// `W210 Variable '$x' is read before it is set` on code `tclsh` runs happily.
/// A word that really does substitute
/// (`foreach $names …`, `foreach [names] …`, `foreach {*}$spec …`) names
/// nothing statically and contributes no def.
///
/// `text` is only consulted when the statement carries no structured word for
/// this argument — a synthetic or lossy token snapshot, where the kind is
/// unknown and the conservative text test is the best available answer.
fn loop_var_list_word_is_static(word: Option<&WordExpr>, text: &str) -> bool {
    match word {
        Some(WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. }) => true,
        Some(
            WordExpr::Variable { .. }
            | WordExpr::CommandSubstitution { .. }
            | WordExpr::Expand { .. },
        ) => false,
        // A compound word is static exactly when every part is text: a quoted
        // (`"a b"`) or backslash-escaped (`a\ b`) var-list substitutes nothing,
        // while any `$` / `[` part makes the whole word dynamic.
        Some(WordExpr::Template { parts, .. }) => parts
            .iter()
            .all(|part| matches!(part, WordPart::Text { .. })),
        Some(WordExpr::Opaque { .. }) | None => !text.contains('$') && !text.contains('['),
    }
}

/// The defs a `Statement::Barrier` contributes under the registry — its
/// loop-variable lists plus its `ArgRole::VarWrite` targets.
///
/// Skips *scope-alias* commands (`global`, `variable`, `upvar`) whose variable
/// bindings are tracked separately by the `var_scoping` pass; without the skip
/// we'd produce partial defs for the vararg forms (`global x y z` would mark
/// only `x`).  The discriminator is `CREATES_SCOPE_ALIAS`, *not*
/// `CREATES_DYNAMIC_BARRIER`: `trace` is a dynamic barrier but not a scope
/// alias, so `trace add variable x` must still surface its `VarWrite` def.
fn registry_barrier_defs(
    reg: &CommandRegistry,
    command: &str,
    args: &[String],
    tokens: Option<&CommandTokens>,
) -> Vec<String> {
    use tcl_registry::{ArgRole, Traits};

    let spec = reg.get(command);
    if spec.is_some_and(|spec| spec.traits.contains(Traits::CREATES_SCOPE_ALIAS)) {
        return Vec::new();
    }
    let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
    // Loop variables (`ArgRole::LoopVarList` — `foreach` / `lmap` var-lists,
    // `dict for` / `array for` key-value pairs).  A structured loop that stays
    // a barrier — `{*}` expansion among its words, a dynamic body — still
    // binds every variable its *literal* var-list words name, and its body is
    // scanned for reads in this same frame, so without these defs a body read
    // of a loop variable reports W210 "read before it is set".
    // A dynamic var-list word (`foreach {*}$pairs …`) names nothing statically
    // and contributes no def — see [`loop_var_list_word_is_static`] for how
    // that verdict is reached.
    //
    // A **conditionally-bound** layout is excluded: `dict update dictVar key
    // varName … body` binds `varName` only when the key is present at runtime,
    // so it is not a definite def and the key-aware read-before-set harvester
    // owns it instead (`RepeatedArgLayout::conditional_binding`).
    let loop_vars_conditional = |repeated: &[tcl_registry::RepeatedArgLayout]| {
        repeated
            .iter()
            .any(|layout| layout.conditional_binding && layout.role == ArgRole::LoopVarList)
    };
    let conditional_loop_vars = spec.is_some_and(|spec| {
        args.first()
            .and_then(|first| spec.resolve_subcommand(first))
            .map_or_else(
                || loop_vars_conditional(spec.repeated_args),
                |sub| loop_vars_conditional(sub.repeated_args),
            )
    });
    let mut defs: Vec<String> = if conditional_loop_vars {
        Vec::new()
    } else {
        reg.arg_indices_for_role(command, &arg_strs, ArgRole::LoopVarList)
            .into_iter()
            .filter_map(|idx| args.get(idx).map(|word| (idx, word)))
            .filter(|(idx, word)| {
                loop_var_list_word_is_static(tokens.and_then(|t| t.words().get(idx + 1)), word)
            })
            .filter_map(|(_, word)| {
                tcl_syntax::word_rules::WordValueRules::of_profile(reg.profile())
                    .split_list(word)
                    .ok()
            })
            .flatten()
            .map(std::borrow::Cow::into_owned)
            .collect()
    };
    defs.extend(
        reg.arg_indices_for_role(command, &arg_strs, ArgRole::VarWrite)
            .into_iter()
            .filter_map(|idx| {
                // A brace-quoted name word is a literal name, `$` and all.
                let braced = tokens.is_some_and(|t| t.arg_is_braced_literal(idx));
                args.get(idx).map(|s| {
                    let name = crate::naming::element_var_name_braced(s, braced);
                    if reg.option_variable_scope(command, &arg_strs, idx, reg.own_surface_query())
                        == Some(tcl_registry::VariableScope::Global)
                        && !tcl_syntax::naming::is_absolute(name)
                    {
                        tcl_syntax::naming::root_unrooted_key(name)
                    } else {
                        name.to_owned()
                    }
                })
            }),
    );
    defs
}

pub(super) fn selected_barrier_defs(
    selection: SsaInvocationContext<'_>,
    tokens: &CommandTokens,
) -> Vec<String> {
    if let Some(source) = selection.source_structure(tokens) {
        let Some(traits) = source.traits() else {
            return Vec::new();
        };
        if traits.contains(tcl_registry::Traits::CREATES_SCOPE_ALIAS)
            || source.loop_bindings_conditional()
        {
            return Vec::new();
        }
        let Some(config) = tokens
            .source_binding
            .as_ref()
            .and_then(|binding| binding.original_lexer_config_for_tokens(tokens))
        else {
            return Vec::new();
        };
        return source
            .written_roles()
            .into_iter()
            .filter(|(_, role)| *role == tcl_registry::ArgRole::LoopVarList)
            .filter_map(|(argument, _)| source.argument_literal(argument))
            .filter_map(|text| {
                tcl_syntax::word_rules::WordValueRules::from_config(&config)
                    .split_list(&text)
                    .ok()
                    .map(|names| {
                        names
                            .into_iter()
                            .map(std::borrow::Cow::into_owned)
                            .collect::<Vec<_>>()
                    })
            })
            .flatten()
            .collect();
    }
    let Some(crate::registry_invocation::RegistryInvocationResolution::Resolved(facts)) =
        selection.resolve(tokens)
    else {
        return Vec::new();
    };
    if facts
        .traits
        .contains(tcl_registry::Traits::CREATES_SCOPE_ALIAS)
        || !facts.arg_roles_complete
    {
        return Vec::new();
    }
    let Some(effective) = crate::registry_invocation::effective_command_words(tokens) else {
        return Vec::new();
    };
    let Some(config) = tokens
        .source_binding
        .as_ref()
        .and_then(|binding| binding.original_lexer_config_for_tokens(tokens))
    else {
        return Vec::new();
    };
    let conditional = facts.repeated_args.iter().any(|layout| {
        layout.conditional_binding && layout.role == tcl_registry::ArgRole::LoopVarList
    });
    let mut defs = selection
        .normal(tokens)
        .map_or_else(Vec::new, |invocation| invocation.definition_names());
    if !conditional {
        for &(index, role) in &facts.arg_roles {
            if role != tcl_registry::ArgRole::LoopVarList {
                continue;
            }
            let Some(word) = effective
                .words
                .get(facts.argument_offset + usize::from(index) + 1)
            else {
                continue;
            };
            let Some(text) = crate::registry_invocation::invocation_word(word).literal() else {
                continue;
            };
            if let Ok(names) =
                tcl_syntax::word_rules::WordValueRules::from_config(&config).split_list(text)
            {
                defs.extend(names.into_iter().map(std::borrow::Cow::into_owned));
            }
        }
    }
    defs
}

/// Explicit standalone registry-aware `defs_of`.
///
/// Barrier defs route through
/// `ArgRole::VarWrite` instead of a hardcoded string-match.  The
/// registry-aware path also covers `::trace` and any future trace
/// alias spellings without code edits, plus skips
/// `creates_dynamic_barrier` specs (`global` / `variable` /
/// `upvar` are handled by `var_scoping`, not by SSA's per-arg
/// `VarWrite` walk).
#[must_use]
pub fn defs_of_with_registry(stmt: &Statement, registry: Option<&CommandRegistry>) -> Vec<String> {
    defs_of_in_context(stmt, registry.map(SsaInvocationContext::standalone))
}

/// Definition projection under the caller's retained complete source input.
/// Supplied absence never reopens standalone descriptor selection; original
/// typed structural writes and Normal-transfer checks retain their own purpose.
pub(crate) fn defs_of_with_metadata_context(
    stmt: &Statement,
    registry: &CommandRegistry,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) -> Vec<String> {
    defs_of_in_context(
        stmt,
        Some(SsaInvocationContext::supplied(registry, metadata)),
    )
}

pub(super) fn defs_of_in_context(
    stmt: &Statement,
    selection: Option<SsaInvocationContext<'_>>,
) -> Vec<String> {
    let registry = selection.map(|selection| selection.registry);
    if !stmt.is_executable_invocation() || stmt.has_opaque_native_accesses() {
        return Vec::new(); // Unknown writes are clobbers, never definite named values.
    }
    match stmt {
        Statement::AssignConst {
            name, name_braced, ..
        }
        | Statement::AssignExpr {
            name, name_braced, ..
        }
        | Statement::AssignValue {
            name, name_braced, ..
        }
        | Statement::Incr {
            name, name_braced, ..
        } => {
            // A write-target whose *name* is value-substituted (`set $p …`,
            // `set ${tok}(k) …`) denotes the variable named by the
            // substitution's value — a place that cannot be pinned down, so it
            // is **not** a static def of the name-bearing variable.  `uses_of`
            // records the name read separately.  A brace-quoted target
            // (`set {$n} 1`) substitutes nothing — it is a static def of the
            // variable literally named `$n`.
            if is_dynamic_write_target(name, *name_braced) {
                return Vec::new();
            }
            // A constant-keyed array element defs its own variable
            // (`arr(k)`); a dynamic key stays on the base (the rename walk
            // fans the def over the array's known elements).
            vec![crate::naming::element_var_name_braced(name, *name_braced).to_owned()]
        }
        Statement::Call { .. } if is_effect_marker(stmt) => Vec::new(),
        Statement::Call { defs, .. }
            if !defs.is_empty() && selection.is_none_or(|s| s.standalone) =>
        {
            defs.clone()
        }
        Statement::Call {
            tokens: Some(tokens),
            ..
        } => selection
            .and_then(|selection| selection.normal(tokens))
            .map_or_else(Vec::new, |invocation| {
                invocation
                    .definition_names()
                    .into_iter()
                    .map(|name| crate::naming::element_var_name_braced(&name, true).to_owned())
                    .collect()
            }),
        Statement::Barrier {
            command,
            args,
            tokens,
            ..
        } => {
            if let Some(selection) = selection.filter(|selection| !selection.standalone) {
                return tokens
                    .as_ref()
                    .map_or_else(Vec::new, |tokens| selected_barrier_defs(selection, tokens));
            }
            // Loop-header barriers (`::tcl::dict::for`/`::map`, `::tcl::array::for`
            // — any ensemble subcommand the registry marks `loop_list_header`):
            // args[0] is the iteration-variable list, so extract the names.
            // Resolved via the registry rather than a name-suffix match — a
            // user proc named `my::for` must NOT have its first argument
            // misread as loop variables.
            if !args.is_empty() && barrier_is_loop_list_header(command, registry) {
                // A loop var-list is a Tcl list, not whitespace-separated
                // source text.  Decode grouping and backslash substitutions so
                // `{one name}`, `"two name"`, and `three\ name` each define
                // one variable.  A malformed var-list makes the command fail
                // before its body runs, so it contributes no reaching defs.
                return tcl_syntax::word_rules::WordValueRules::of_profile(
                    registry.and_then(tcl_registry::CommandRegistry::profile),
                )
                .split_list(&args[0])
                .map_or_default(|names| {
                    names
                        .into_iter()
                        .map(std::borrow::Cow::into_owned)
                        .collect()
                });
            }
            if let Some(reg) = registry {
                let defs = registry_barrier_defs(reg, command, args, tokens.as_ref());
                if !defs.is_empty() {
                    return defs;
                }
            }
            // String-match fallback for callers without a registry
            // (test helpers).
            if command == "trace" && args.len() >= 3 && args[0] == "add" && args[1] == "variable" {
                return vec![crate::naming::element_var_name(&args[2]).to_owned()];
            }
            Vec::new()
        }
        // An opaque (glob/regexp/fall-through) `switch` definitely-defines a
        // variable only when *every reaching* path assigns it: there must be a
        // `default` arm (covering the no-match path) and the variable must be
        // *must-defined* in the default body and in every arm that has a body
        // (fall-through arms with no body delegate to a later body, already
        // covered). An arm that *cannot complete normally* (always
        // `return`s/`error`s/`tailcall`s/…) never reaches the code after the
        // switch, so it is excluded from the intersection (FP-RBS-14). This
        // reproduces the phi the expanded arm blocks would build, conservatively
        // — a variable only *conditionally* assigned inside an arm is not
        // claimed, so we never hide a genuine read-before-set. The expanded
        // (exact, non-fall-through) switch never reaches here; its arm defs come
        // from the real per-block statements. Shares the "flow facts"
        // definite-assignment helpers with the CFG builder (`cfg_builder`), so
        // both layers agree on "cannot complete normally".
        Statement::Switch {
            default_body: Some(_),
            ..
        } => match selection {
            Some(selection) if !selection.standalone => {
                crate::cfg_builder::switch_must_defines_with_source_input(
                    stmt,
                    selection.registry,
                    selection
                        .metadata
                        .and_then(crate::registry_invocation::InvocationMetadataContext::source_analysis_input),
                )
                .into_iter()
                .collect()
            }
            _ => crate::cfg_builder::switch_must_defines(stmt, registry)
                .into_iter()
                .collect(),
        },
        _ => Vec::new(),
    }
}

/// The names an opaque `switch`'s arm bodies may write, destroy or declare
/// beyond the ones every path through it defines
/// ([`defs_of_with_registry`]).
///
/// A `-glob`, `-regexp`, `-nocase`, fall-through or `case` statement keeps its
/// arm bodies inline, so no block of the CFG holds a write inside one, and
/// this walk is the only place that says `q* { set go 0 }` may leave `go` at
/// 0 where the statement began with 1. A name written anywhere in an arm —
/// however deeply nested, in a command's own words or in a `[…]` substitution
/// one of them carries, and whichever command writes it (`incr`, `append`,
/// `lappend`, `lset`, `dict set`, `array set`, `unset`, a `regexp`'s match
/// variable) — is a *may*-def: the statement leaves it as it was when no arm
/// runs, so the rename walk gives it a version that reads its
/// prior one, the solver joins the two, and existence is unknown afterwards.
/// The statement the CFG builder puts after such a `switch` for the names a
/// callee its arms call writes ([`crate::ir::SyntheticMarker::ArmWrites`])
/// states them as its own definitions, and they are may-definitions too, as
/// are the callee-summary names merged onto a call under supplied metadata.
/// Every other statement kind returns nothing.
pub(crate) fn switch_may_defs(stmt: &Statement, registry: &CommandRegistry) -> Vec<String> {
    switch_may_defs_in_context(stmt, SsaInvocationContext::standalone(registry))
}

/// Named possible arm writes under the same retained selection as SSA itself.
/// This is not a closed inventory of unknown writes or an executed store.
pub(super) fn switch_may_defs_in_context(
    stmt: &Statement,
    selection: SsaInvocationContext<'_>,
) -> Vec<String> {
    if let Statement::Call { defs, .. } = stmt
        && is_effect_marker(stmt)
    {
        return defs
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
    }
    // Under supplied metadata a call's definite definitions come from its
    // resolved invocation, so the callee-summary names the CFG builder merged
    // onto it (`upvar 1 g x; set x 9`, `global g`) are possible writes the
    // statement leaves as they were when the callee does not reach them.
    if let Statement::Call { defs, .. } = stmt
        && !selection.standalone
        && !defs.is_empty()
    {
        let definite = defs_of_in_context(stmt, Some(selection));
        return defs
            .iter()
            .filter(|name| !definite.contains(name))
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
    }
    let Statement::Switch {
        arms, default_body, ..
    } = stmt
    else {
        return Vec::new();
    };
    let mut written = BTreeSet::new();
    for body in arms
        .iter()
        .filter_map(|arm| arm.body.as_ref())
        .chain(default_body.as_ref())
    {
        nested_writes_in_context(body, selection, &mut written);
    }
    for name in defs_of_in_context(stmt, Some(selection)) {
        written.remove(&name);
    }
    written.into_iter().collect()
}

/// The bases a statement with arm may-definitions refreshes only because it
/// may write one of their elements: the array `set a(k) …` names in an
/// opaque `switch` arm or `catch` body, where the statement states no write
/// of the base itself ([`switch_may_defs`]). The rename walk gives each a
/// quoted read of its prior version, for liveness and the solver's join;
/// read-before-set reads such a base as the definition it is, never as a
/// name the statement may leave unset.
pub(crate) fn refreshed_bases(
    stmt: &Statement,
    function: &crate::compilation_unit::FunctionUnit,
    registry: &CommandRegistry,
) -> Vec<String> {
    if function.source_metadata_input().is_none() && function.cfg.metadata_context.is_standalone() {
        return bases_refreshed_by(&switch_may_defs(stmt, registry));
    }
    bases_refreshed_by(&switch_may_defs_in_context(
        stmt,
        SsaInvocationContext::supplied(registry, function.invocation_metadata_context(registry)),
    ))
}

/// The bases of the elements of `stated` that `stated` does not name itself.
fn bases_refreshed_by(stated: &[String]) -> Vec<String> {
    let mut bases: Vec<String> = stated
        .iter()
        .filter_map(|name| crate::naming::split_element_ref(name).map(|(root, _)| root.to_owned()))
        .filter(|base| !stated.contains(base))
        .collect();
    bases.sort();
    bases.dedup();
    bases
}

/// Every name any statement of `script`, however deeply nested, or any `[…]`
/// substitution in one of its words, may write.
pub(crate) fn nested_writes(
    script: &crate::ir::Script,
    registry: &CommandRegistry,
    out: &mut BTreeSet<String>,
) -> bool {
    nested_writes_in_context(script, SsaInvocationContext::standalone(registry), out)
}

pub(super) fn nested_writes_in_context(
    script: &crate::ir::Script,
    selection: SsaInvocationContext<'_>,
    out: &mut BTreeSet<String>,
) -> bool {
    let registry = selection.registry;
    let mut opaque = false;
    let mut pending = vec![(script, 0_u32)];
    while let Some((script, depth)) = pending.pop() {
        if MAX_COLLAPSED_EXTRA_DEFS_DEPTH.exceeded(depth) {
            opaque = true;
            continue;
        }
        for statement in &script.statements {
            let structural = crate::ir_helpers::structural_variable_write_projection(statement);
            opaque |= structural.opaque_variable_frame;
            out.extend(structural.literal_names);
            if let Statement::Call { defs, .. } = statement {
                // Existing callee-summary names remain possible writes, not completed stores.
                out.extend(defs.iter().cloned());
            }
            if let Some(tokens) = script.retained_source_tokens_for_statement(statement) {
                let source_purpose = selection.token_metadata(tokens).and_then(|metadata| {
                    tokens
                        .source_binding
                        .as_ref()?
                        .original_source_advice_purpose(tokens, metadata, registry)
                });
                if !selection.standalone && source_purpose.is_none() {
                    opaque = true;
                } else {
                    let names = crate::ir_helpers::original_statement_possible_name_writes(
                        statement, tokens, registry,
                    );
                    opaque |= names.opaque;
                    out.extend(names.names);
                }
            } else if statement.is_executable_invocation() && statement.synthetic_marker().is_none()
            {
                opaque |= !selection.standalone
                    || !crate::ir_helpers::standalone_typed_statement_writes_are_covered(
                        script, statement, registry,
                    );
            }
            for body in statement.child_scripts() {
                pending.push((body, depth + 1));
            }
        }
    }
    opaque
}

/// Depth cap for [`collapsed_extra_defs`]'s recursion over nested
/// `if`/`while`/`for`/`foreach`/`catch`/`try`/`switch` bodies.
/// Transitively bounded via `MAX_LOWER_NEST_DEPTH` (every `Script`
/// feeding SSA construction was built by `crate::lowering`, which already
/// caps its own construction at 256), capped here independently for
/// defence-in-depth and consistency with every other full-tree walker in
/// this crate.
const MAX_COLLAPSED_EXTRA_DEFS_DEPTH: tcl_core_types::RecursionLimit =
    tcl_core_types::RecursionLimit(256);

/// Explicit standalone condition/init/next write candidates for collapsed reads.
/// Source-owned callers use their complete retained statement inventories.
pub(super) fn collapsed_extra_defs(
    script: &crate::ir::Script,
    registry: &CommandRegistry,
    depth: u32,
) -> BTreeSet<String> {
    use crate::ir_helpers::{defs_from_expr, defs_from_ir_script};
    let mut extra = BTreeSet::new();
    if MAX_COLLAPSED_EXTRA_DEFS_DEPTH.exceeded(depth) {
        return extra;
    }
    for stmt in &script.statements {
        match stmt {
            Statement::If {
                clauses, else_body, ..
            } => {
                for clause in clauses {
                    extra.extend(defs_from_expr(&clause.condition, registry));
                    extra.extend(collapsed_extra_defs(&clause.body, registry, depth + 1));
                }
                if let Some(eb) = else_body {
                    extra.extend(collapsed_extra_defs(eb, registry, depth + 1));
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                extra.extend(defs_from_expr(condition, registry));
                extra.extend(collapsed_extra_defs(body, registry, depth + 1));
            }
            Statement::For {
                init,
                condition,
                next,
                body,
                ..
            } => {
                extra.extend(defs_from_ir_script(init));
                extra.extend(defs_from_ir_script(next));
                extra.extend(defs_from_expr(condition, registry));
                extra.extend(collapsed_extra_defs(init, registry, depth + 1));
                extra.extend(collapsed_extra_defs(next, registry, depth + 1));
                extra.extend(collapsed_extra_defs(body, registry, depth + 1));
            }
            Statement::Foreach { body, .. } | Statement::Catch { body, .. } => {
                extra.extend(collapsed_extra_defs(body, registry, depth + 1));
            }
            Statement::Try {
                body,
                handlers,
                finally_body,
                ..
            } => {
                extra.extend(collapsed_extra_defs(body, registry, depth + 1));
                for handler in handlers {
                    extra.extend(collapsed_extra_defs(&handler.body, registry, depth + 1));
                }
                if let Some(fb) = finally_body {
                    extra.extend(collapsed_extra_defs(fb, registry, depth + 1));
                }
            }
            Statement::Switch {
                arms, default_body, ..
            } => {
                for arm in arms {
                    if let Some(body) = &arm.body {
                        extra.extend(collapsed_extra_defs(body, registry, depth + 1));
                    }
                }
                if let Some(db) = default_body {
                    extra.extend(collapsed_extra_defs(db, registry, depth + 1));
                }
            }
            _ => {}
        }
    }
    extra
}
