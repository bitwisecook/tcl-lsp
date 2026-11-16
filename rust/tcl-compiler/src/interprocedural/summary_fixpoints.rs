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

//! From per-procedure local facts to summaries: the scan over every body, the purity and effect fixpoints and the materialised result.

use super::body_scan::scan_proc;
use super::call_facts::is_plain_local_name;
use super::return_summaries::{SeedlessAnswer, summarise_returns};
use super::{LocalFacts, ProcArgTrait, ProcScan, ProcSummary, SummaryBodyInputs, arity_from_names};
use crate::ir::ProcedureId;
use crate::side_effects::EffectRegion;
use std::collections::{HashMap, HashSet};
use tcl_registry::value_transfer::ExactValue;

/// Conditional source summaries retain genuine declaration IDs independently
/// of callable publication, completed definition reach and Native admission.
pub(super) fn summary_procedure_bodies<'a>(
    module: &'a crate::ir::Module,
    registry: &tcl_registry::CommandRegistry,
) -> HashMap<ProcedureId, &'a crate::ir::Procedure> {
    let mut procedures: HashMap<_, _> = module
        .procedures
        .iter()
        .filter(|(id, _)| module.procedure_ids.owns(**id))
        .map(|(id, procedure)| (*id, procedure))
        .collect();
    if let Some(owner) = module.retained_source_bindings.as_ref()
        && owner.positioned_for_module(module, registry).is_some()
    {
        for (id, procedure) in module.original_source_procedure_bodies() {
            if procedure.body_kind == crate::ir::ProcedureBodyKind::OriginalDeclaration
                && owner.matches_original_procedure_summary(*id, procedure)
            {
                procedures.insert(*id, procedure);
            }
        }
    }
    procedures
}

pub(super) fn scan_all_procs(
    ir_module: &crate::ir::Module,
    source: SummaryBodyInputs<'_>,
    registry: &tcl_registry::CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    object_types: &HashMap<String, HashSet<String>>,
    identities: &crate::realm::CommandBindingRealm,
    declared: Option<&tcl_registry::model::DeclaredSurface>,
) -> HashMap<ProcedureId, LocalFacts> {
    let source_input =
        crate::registry_invocation::InvocationMetadataContext::for_module(registry, ir_module)
            .and_then(crate::registry_invocation::InvocationMetadataContext::source_analysis_input);
    let known = source.known;
    let mut local: HashMap<ProcedureId, LocalFacts> = HashMap::with_capacity(known.len());
    for (&id, &proc) in source.bodies {
        local.insert(
            id,
            scan_proc(ProcScan {
                proc,
                unit: source.unit,
                known,
                registry,
                dialect,
                object_types,
                identities,
                declared,
                source_input,
                source_procedures: &ir_module.procedures,
                source_module: ir_module,
            }),
        );
    }
    local
}

pub(super) fn compute_all_transitive_calls(
    known: &HashSet<ProcedureId>,
    local: &HashMap<ProcedureId, LocalFacts>,
) -> HashMap<ProcedureId, HashSet<ProcedureId>> {
    let mut out: HashMap<ProcedureId, HashSet<ProcedureId>> = HashMap::with_capacity(known.len());
    for qname in known {
        out.insert(*qname, compute_transitive_calls(qname, local));
    }
    out
}

/// Each procedure's purity: its own body pure and every procedure it calls
/// pure — or one whose calls reach no further than the places its `Name`
/// arguments name, called with locals of the caller's own frame that no
/// other frame reaches, which end with the caller's call
/// (`docs/design/compiler/value-transfers.md` § *Proc-level transfer
/// summaries*). Such a procedure is one whose body does nothing its caller
/// observes but through those links, and calls only procedures of the same
/// kind with places of its own, its links' among them.
pub(super) fn fixpoint_pure(
    local: &HashMap<ProcedureId, LocalFacts>,
) -> HashMap<ProcedureId, bool> {
    let mut pure: HashMap<ProcedureId, bool> =
        local.iter().map(|(q, f)| (*q, f.local_pure)).collect();
    let mut bounded: HashMap<ProcedureId, bool> = local
        .iter()
        .map(|(q, f)| (*q, !f.impure_beyond_links))
        .collect();
    loop {
        let mut changed = false;
        for (qname, facts) in local {
            let (now_bounded, now_pure) = {
                let calls_stay = |own_links: bool| {
                    facts.procedure_calls.iter().all(|(callee, words)| {
                        pure.get(callee).copied().unwrap_or(false)
                            || (bounded.get(callee).copied().unwrap_or(false)
                                && words.as_deref().is_some_and(|words| {
                                    local.get(callee).is_some_and(|called| {
                                        names_own_places(
                                            facts,
                                            &called.name_params,
                                            words,
                                            own_links,
                                        )
                                    })
                                }))
                    })
                };
                (
                    bounded[qname] && calls_stay(true),
                    pure[qname] && calls_stay(false),
                )
            };
            if now_bounded != bounded[qname] {
                bounded.insert(*qname, now_bounded);
                changed = true;
            }
            if now_pure != pure[qname] {
                pure.insert(*qname, now_pure);
                changed = true;
            }
        }
        if !changed {
            return pure;
        }
    }
}

/// Whether a call's `words` name, at each of the callee's `name_params`, a
/// plain local of the caller's frame: one no `global` or `variable` aliases
/// and no `upvar` links elsewhere — or, where `own_links` is set, one the
/// caller links to the place its own parameter names one frame up.
fn names_own_places(
    caller: &LocalFacts,
    name_params: &[usize],
    words: &[String],
    own_links: bool,
) -> bool {
    name_params.iter().all(|&index| {
        words.get(index).is_some_and(|word| {
            is_plain_local_name(word)
                && !caller.global_aliases.contains(word)
                && (!caller.linked_locals.contains(word)
                    || (own_links && caller.upvar_aliases.contains_key(word)))
        })
    })
}

pub(super) fn fixpoint_effects(
    local: &HashMap<ProcedureId, LocalFacts>,
) -> (
    HashMap<ProcedureId, EffectRegion>,
    HashMap<ProcedureId, EffectRegion>,
) {
    let mut reads: HashMap<ProcedureId, EffectRegion> =
        local.iter().map(|(q, f)| (*q, f.effect_reads)).collect();
    let mut writes: HashMap<ProcedureId, EffectRegion> =
        local.iter().map(|(q, f)| (*q, f.effect_writes)).collect();
    let mut changed = true;
    while changed {
        changed = false;
        for (qname, facts) in local {
            let mut r = reads[qname];
            let mut w = writes[qname];
            for c in &facts.direct_calls {
                r |= reads.get(c).copied().unwrap_or(EffectRegion::UNKNOWN_STATE);
                w |= writes
                    .get(c)
                    .copied()
                    .unwrap_or(EffectRegion::UNKNOWN_STATE);
            }
            if r != reads[qname] {
                reads.insert(*qname, r);
                changed = true;
            }
            if w != writes[qname] {
                writes.insert(*qname, w);
                changed = true;
            }
        }
    }
    (reads, writes)
}

pub(super) fn materialise_summaries(
    ir_module: &crate::ir::Module,
    bodies: &HashMap<ProcedureId, &crate::ir::Procedure>,
    local: &HashMap<ProcedureId, LocalFacts>,
    transitive_calls: &HashMap<ProcedureId, HashSet<ProcedureId>>,
    (pure, completes): (&HashMap<ProcedureId, bool>, &HashMap<ProcedureId, bool>),
    (effect_reads, effect_writes): (
        &HashMap<ProcedureId, EffectRegion>,
        &HashMap<ProcedureId, EffectRegion>,
    ),
    seedless: &HashMap<ProcedureId, Option<ExactValue>>,
) -> HashMap<ProcedureId, ProcSummary> {
    let mut procedures: HashMap<ProcedureId, ProcSummary> = HashMap::with_capacity(local.len());
    let rules =
        tcl_syntax::word_rules::WordValueRules::of_dialect_name(ir_module.dialect.as_deref());
    for (qname, facts) in local {
        let Some(proc) = bodies.get(qname) else {
            continue;
        };
        let report_label = if proc.body_kind == crate::ir::ProcedureBodyKind::OriginalDeclaration {
            let Some(label) =
                ir_module.original_procedure_report_label(*qname, ir_module.resolved_registry())
            else {
                continue;
            };
            label.to_owned()
        } else {
            proc.qualified_name.clone()
        };
        let mut calls_list: Vec<ProcedureId> = transitive_calls
            .get(qname)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect();
        calls_list.sort();
        let mut direct_calls: Vec<ProcedureId> = facts.direct_calls.iter().copied().collect();
        direct_calls.sort();
        let is_pure = *pure.get(qname).unwrap_or(&false);

        // Transitive effects include every retained callee's local flags.
        let transitive_flag = |pick: fn(&LocalFacts) -> bool| -> bool {
            pick(facts) || calls_list.iter().any(|c| local.get(c).is_some_and(pick))
        };
        let writes_global = transitive_flag(|f| f.writes_global);
        let has_unknown_calls = transitive_flag(|f| f.has_unknown_calls);

        let (returns_constant, constant_return, passthrough, depends) =
            summarise_returns(&facts.returns, SeedlessAnswer::of(seedless, qname));
        // A proc is foldable at a call site when its return is
        // fully determined by the static call — that means pure
        // AND (constant return OR passthrough of a param).
        let can_fold = is_pure && (returns_constant || passthrough.is_some());
        let param_traits = finalise_param_traits(
            &proc.params,
            &facts.param_trait_flags,
            passthrough.as_deref(),
            &facts.param_references,
            !facts.param_observations_incomplete,
        );

        procedures.insert(
            *qname,
            ProcSummary {
                qualified_name: report_label,
                params: proc.params.clone(),
                arity: arity_from_names(&proc.params),
                calls: calls_list,
                direct_calls,
                has_barrier: facts.has_barrier,
                has_unknown_calls,
                writes_global,
                pure: is_pure,
                completes: completes.get(qname).copied().unwrap_or(false),
                // The summary build states the definition reach once every
                // summary exists.
                defined_at: None,
                defined_callees: Vec::new(),
                effect_reads: *effect_reads
                    .get(qname)
                    .unwrap_or(&EffectRegion::UNKNOWN_STATE),
                effect_writes: *effect_writes
                    .get(qname)
                    .unwrap_or(&EffectRegion::UNKNOWN_STATE),
                returns_constant,
                constant_return,
                return_depends_on_params: depends,
                return_passthrough_param: passthrough,
                can_fold_static_calls: can_fold,
                param_traits,
                param_defaults: declared_defaults(&proc.params_raw, rules),
            },
        );
    }
    procedures
}

/// The defaults the parameter list `params_raw` declares, by parameter, as
/// Tcl decodes them under `rules`; none where the list cannot be read.
fn declared_defaults(
    params_raw: &str,
    rules: tcl_syntax::word_rules::WordValueRules,
) -> HashMap<String, String> {
    crate::signature_scan::params::parse_param_list_strict(params_raw, rules).map_or_default(
        |formals| {
            formals
                .into_iter()
                .filter_map(|formal| Some((formal.name, formal.default?)))
                .collect()
        },
    )
}

/// Collapse the raw trait observation flags into the final
/// per-param trait set. Adds `Passthrough` when the proc's
/// `return_passthrough_param` matches this param; adds `Unused`
/// only when the complete owned source inventory contains no reference.
pub(super) fn finalise_param_traits(
    params: &[String],
    flags: &HashMap<String, HashSet<ProcArgTrait>>,
    passthrough: Option<&str>,
    references: &HashSet<String>,
    complete: bool,
) -> HashMap<String, HashSet<ProcArgTrait>> {
    let mut out: HashMap<String, HashSet<ProcArgTrait>> = HashMap::new();
    for p in params {
        let mut traits: HashSet<ProcArgTrait> = flags.get(p).cloned().unwrap_or_default();
        if passthrough == Some(p) {
            traits.insert(ProcArgTrait::Passthrough);
        }
        if complete && traits.is_empty() && !references.contains(p) {
            traits.insert(ProcArgTrait::Unused);
        }
        out.insert(p.clone(), traits);
    }
    out
}

fn compute_transitive_calls(
    root: &ProcedureId,
    local: &HashMap<ProcedureId, LocalFacts>,
) -> HashSet<ProcedureId> {
    let mut visited: HashSet<ProcedureId> = HashSet::new();
    let mut stack: Vec<ProcedureId> = Vec::new();
    if let Some(f) = local.get(root) {
        stack.extend(f.direct_calls.iter().copied());
    }
    while let Some(cur) = stack.pop() {
        if !visited.insert(cur) {
            continue;
        }
        if let Some(f) = local.get(&cur) {
            for d in &f.direct_calls {
                if !visited.contains(d) {
                    stack.push(*d);
                }
            }
        }
    }
    visited
}
