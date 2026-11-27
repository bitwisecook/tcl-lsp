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

//! Where φ nodes go: the non-local names and their definition sites, array elements expanded beside their roots and source-may calls.

use super::super::defs::{bound_defs, bound_names, switch_may_defs_in_context};
use super::super::uses::uses_of_classified_in_context;
use super::super::{RegistryClobberNames, SsaInvocationContext};
use super::read_references::{invocation_read_bindings, source_read_bindings};
use super::{ArrayElems, SsaDefinitionSites, SsaIdentitySet, SsaVariableIdentity};
use crate::cfg;
use crate::cfg::BlockId;
use crate::ir::Statement;
use crate::var_refs::{VarReferenceScanner, VarScanOptions};
use crate::variable_bindings::PointResolveContexts;
#[cfg(test)]
use crate::variable_bindings::build_point_resolve_contexts;
use rustc_hash::FxHashSet;
use std::collections::{HashMap, HashSet};
use tcl_names::variable::VariableCellKey;
use tcl_registry::CommandRegistry;

fn original_source_may_call(
    function: &cfg::Function,
    statement: &Statement,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    registry: &CommandRegistry,
) -> Option<crate::cfg_builder::global_write_info::OriginalSourceCallMayWrites> {
    original_source_may_call_from_inventory(
        &function.source_may_calls,
        statement,
        metadata,
        registry,
    )
}

pub(super) fn original_source_may_call_from_inventory(
    inventory: &HashMap<
        (usize, u32),
        Vec<crate::cfg_builder::global_write_info::OriginalSourceCallMayWrites>,
    >,
    statement: &Statement,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    registry: &CommandRegistry,
) -> Option<crate::cfg_builder::global_write_info::OriginalSourceCallMayWrites> {
    let site = statement
        .tokens()?
        .source_binding
        .as_ref()?
        .invocation_site()?;
    let candidates =
        inventory.get(&(std::sync::Arc::as_ptr(&site.source) as usize, site.offset))?;
    let mut matching = candidates
        .iter()
        .filter(|receipt| receipt.matches_statement(statement, metadata, registry));
    let first = matching.next()?;
    matching
        .all(|next| first.same_owner(next))
        .then(|| first.clone())
}

/// Compute which variables need phi nodes in each block.
///
/// Uses the iterated dominance frontier algorithm: for each variable,
/// starting from blocks where it is defined, propagate phi nodes to
/// the dominance frontier until convergence.
#[must_use]
#[cfg(test)]
pub(crate) fn compute_phi_vars(
    func: &cfg::Function,
    df: &HashMap<BlockId, HashSet<BlockId>>,
    registry: &CommandRegistry,
    elems: &ArrayElems,
) -> HashMap<BlockId, HashSet<VariableCellKey>> {
    compute_phi_vars_with_config(
        func,
        df,
        SsaInvocationContext::standalone(registry),
        elems,
        tcl_lexer::LexerConfig::for_profile(registry.profile()),
        &build_point_resolve_contexts(func, &func.name, registry),
        &HashMap::new(),
    )
    .into_iter()
    .map(|(block, identities)| {
        (
            block,
            identities
                .into_iter()
                .filter_map(|identity| match identity {
                    SsaVariableIdentity::Location(key) => Some(key),
                    SsaVariableIdentity::Source(_) => None,
                })
                .collect(),
        )
    })
    .collect()
}

/// [`compute_phi_vars`] under the exact lexical policy that produced `func`.
pub(super) fn compute_phi_vars_with_config(
    func: &cfg::Function,
    df: &HashMap<BlockId, HashSet<BlockId>>,
    selection: SsaInvocationContext<'_>,
    elems: &ArrayElems,
    config: tcl_lexer::LexerConfig,
    points: &PointResolveContexts,
    clobbers: &RegistryClobberNames,
) -> HashMap<BlockId, HashSet<SsaVariableIdentity>> {
    let reachable = func.reachable_blocks();
    let (nonlocal_names, mut all_defsites) =
        nonlocal_names_and_defsites(func, &reachable, selection, elems, config, points);
    for (block, markers) in clobbers {
        for name in markers.values().flatten() {
            all_defsites
                .or_default(SsaVariableIdentity::Location(name.clone()))
                .insert(*block);
        }
    }

    // Semi-pruned SSA (Briggs et al. 1998): place phis only for *non-local*
    // (upward-exposed-use) names. A phi for a purely-local name has no reader,
    // so dropping it removes only dead phis (~40% of minimal-SSA phis) without
    // changing any use/value/liveness/diagnostic result.
    let mut phi: HashMap<BlockId, HashSet<SsaVariableIdentity>> = HashMap::new();
    for id in func.blocks.keys() {
        phi.insert(*id, HashSet::new());
    }

    for (var, sites) in all_defsites.iter() {
        if !nonlocal_names.contains_key(var) {
            continue;
        }
        let mut work: Vec<BlockId> = sites.iter().copied().collect();
        work.sort_unstable();
        let mut has_phi: FxHashSet<BlockId> = FxHashSet::default();

        while let Some(nb) = work.pop() {
            for fb in df.get(&nb).into_iter().flatten() {
                if has_phi.insert(*fb) {
                    phi.entry(*fb).or_default().insert(var.clone());
                    if !sites.contains(fb) {
                        work.push(*fb);
                    }
                }
            }
        }
    }
    phi
}

/// Collect exact physical array elements at their own read/store boundaries.
pub(super) fn collect_array_elems(
    func: &cfg::Function,
    registry: &CommandRegistry,
    points: &PointResolveContexts,
) -> ArrayElems {
    let mut elems = ArrayElems::default();
    for (&id, block) in &func.blocks {
        for (index, statement) in block.statements.iter().enumerate() {
            for target in crate::place_bridge::def_places_at(statement, id, index, points, registry)
                .into_iter()
                .chain(crate::place_bridge::read_places_at(
                    statement, id, index, points, registry,
                ))
            {
                elems.note(target);
            }
        }
        if let Some(terminator) = &block.terminator {
            for target in
                crate::place_bridge::terminator_read_places_at(terminator, id, points, registry)
            {
                elems.note(target);
            }
        }
    }
    elems
}

/// Expand a statement's direct def names with the array-element fan-out:
///
/// - an element def (`arr(k)`) also defs its base `arr`, so whole-array
///   reads (`array get arr`) see a fresh version;
/// - a base def where constant-keyed elements exist (a dynamic-key write
///   `set arr($i) v`, or a whole-array writer like `array set`) **fans**
///   over every known element — the write may have hit any of them.
///
/// The rename walk records a *use* of each fanned-only name's prior
/// version, so type inference joins the old element type with the written
/// value instead of trusting either.
pub(in super::super) fn expand_defs(
    direct: &[VariableCellKey],
    elems: &ArrayElems,
) -> Vec<VariableCellKey> {
    let mut out: Vec<VariableCellKey> = Vec::new();
    let push = |n: VariableCellKey, out: &mut Vec<VariableCellKey>| {
        if !out.contains(&n) {
            out.push(n);
        }
    };
    for d in direct {
        if let Some(root) = elems.root_for_element(d) {
            push(d.clone(), &mut out);
            push(root.to_owned(), &mut out);
        } else {
            push(d.clone(), &mut out);
            if let Some(els) = elems.get(d) {
                for e in els {
                    push(e.clone(), &mut out);
                }
            }
        }
    }
    out
}

/// The use-side counterpart of [`expand_defs`]: reading a base whose
/// constant-keyed elements are known (`$a($i)`, `array get a`, `parray a`)
/// reads *every* element, and a dynamic-key / whole-array write reads each
/// fanned element's prior version (the may-def join input). Both make the
/// element chains live and upward-exposed so phi placement and liveness see
/// them.
pub(super) fn expand_uses(
    direct_uses: &[VariableCellKey],
    direct_defs: &[VariableCellKey],
    elems: &ArrayElems,
) -> Vec<VariableCellKey> {
    let mut out: Vec<VariableCellKey> = Vec::new();
    let push = |n: &VariableCellKey, out: &mut Vec<VariableCellKey>| {
        if !out.iter().any(|e| e == n) {
            out.push(n.to_owned());
        }
    };
    for u in direct_uses {
        if let Some(els) = elems.get(u) {
            for e in els {
                push(e, &mut out);
            }
        }
    }
    for d in direct_defs {
        if elems.root_for_element(d).is_none()
            && let Some(els) = elems.get(d)
        {
            for e in els {
                push(e, &mut out);
            }
        }
    }
    out
}

pub(super) type AuthoredScalarSourceAccess = (
    crate::signature_scan::variable_symbol::SignatureSourceVariableSymbol,
    String,
    crate::registry_invocation::AuthoredSourceVariableAccess,
);

pub(super) fn authored_scalar_source_variable(
    points: &PointResolveContexts,
    block: BlockId,
    index: usize,
    registry: &CommandRegistry,
) -> Option<(
    crate::signature_scan::variable_symbol::SignatureSourceVariableSymbol,
    String,
    crate::registry_invocation::AuthoredSourceVariableAccess,
)> {
    let tokens = points.source_tokens_at(block, index)?;
    let site = tokens.source_binding.as_ref()?.invocation_site()?;
    let offset = tokens.words().first()?.source().span.start();
    let mut original = tokens.clone();
    crate::lattice_rebase::rebase_command_tokens(
        &mut original,
        i64::from(site.offset) - i64::from(offset),
    );
    let invocation =
        crate::registry_invocation::original_authored_scalar_invocation(&original, registry);
    let scalar = invocation.map(|invocation| invocation.scalar_source_variable(registry));
    scalar
        .flatten()
        .map(|(symbol, name, definition)| {
            let access = if definition {
                crate::registry_invocation::AuthoredSourceVariableAccess::Write
            } else {
                crate::registry_invocation::AuthoredSourceVariableAccess::Read
            };
            (symbol, name, access)
        })
        .or_else(|| {
            crate::registry_invocation::original_authored_script_scalar_variable(
                &original, registry,
            )
        })
}

fn record_authored_terminator_use(
    points: &PointResolveContexts,
    bn: BlockId,
    registry: &CommandRegistry,
    defined_here: &SsaIdentitySet,
    nonlocal_names: &mut SsaIdentitySet,
) {
    if let Some((symbol, _, crate::registry_invocation::AuthoredSourceVariableAccess::Read)) =
        authored_scalar_source_variable(points, bn, usize::MAX, registry)
    {
        let identity = SsaVariableIdentity::Source(symbol);
        if !defined_here.contains_key(&identity) {
            nonlocal_names.or_default(identity);
        }
    }
}

fn nonlocal_terminator_uses(
    (bn, block): (BlockId, &cfg::Block),
    selection: SsaInvocationContext<'_>,
    scanner: &mut VarReferenceScanner,
    points: &PointResolveContexts,
) -> Vec<VariableCellKey> {
    let registry = selection.registry;
    // Terminator reads are element-qualified like statement reads (a
    // condition's `$a(k)` must place the element's phi), and a base
    // read fans over known elements.
    let mut term_uses: Vec<String> = Vec::new();
    match &block.terminator {
        Some(cfg::Terminator::Branch { condition, .. }) => {
            term_uses.extend(condition.vars_element_qualified_with_config(scanner.lexer_config()));
        }
        Some(cfg::Terminator::Return { value, expr, .. }) => {
            if let Some(v) = value {
                term_uses.extend(selection.word_reads(scanner, v));
            }
            if let Some(e) = expr {
                term_uses.extend(e.vars_element_qualified_with_config(scanner.lexer_config()));
            }
        }
        _ => {}
    }
    let mut term_uses = bound_names(term_uses, points.before_terminator(bn), registry);
    term_uses.extend(
        source_read_bindings(points, bn, usize::MAX, registry)
            .into_iter()
            .map(|(_, bound)| bound),
    );
    term_uses.extend(invocation_read_bindings(
        points,
        bn,
        usize::MAX,
        points.before_terminator(bn),
        registry,
    ));
    term_uses
}

fn nonlocal_names_and_defsites(
    func: &cfg::Function,
    reachable: &HashSet<BlockId>,
    selection: SsaInvocationContext<'_>,
    elems: &ArrayElems,
    config: tcl_lexer::LexerConfig,
    points: &PointResolveContexts,
) -> (SsaIdentitySet, SsaDefinitionSites) {
    let registry = selection.registry;
    let mut scanner = VarReferenceScanner::with_config(
        VarScanOptions {
            include_var_read_roles: selection.standalone,
            recurse_cmd_substitutions: true,
            include_reads_before_write: false,
            element_qualified: true,
        },
        config,
    );
    let mut nonlocal_names = SsaIdentitySet::default();
    let mut defsites = SsaDefinitionSites::default();

    for bn in reachable {
        let Some(block) = func.blocks.get(bn) else {
            continue;
        };
        let mut defined_here = SsaIdentitySet::default();
        for (index, stmt) in block.statements.iter().enumerate() {
            let context = points.before_statement(*bn, index);
            let mut direct_uses = bound_names(
                uses_of_classified_in_context(stmt, &mut scanner, selection)
                    .into_iter()
                    .map(|(name, _)| name)
                    .collect(),
                context,
                registry,
            );
            direct_uses.extend(
                source_read_bindings(points, *bn, index, registry)
                    .into_iter()
                    .map(|(_, bound)| bound),
            );
            direct_uses.extend(invocation_read_bindings(
                points, *bn, index, context, registry,
            ));
            let direct_defs = bound_defs(stmt, *bn, index, points, selection);
            let may_defs = bound_names(
                switch_may_defs_in_context(stmt, selection),
                context,
                registry,
            );
            let all_defs: Vec<_> = direct_defs.iter().chain(&may_defs).cloned().collect();
            for u in direct_uses
                .iter()
                .cloned()
                .chain(expand_uses(&direct_uses, &all_defs, elems))
                .chain(may_defs.iter().cloned())
            {
                let u = SsaVariableIdentity::Location(u);
                if !defined_here.contains_key(&u) {
                    nonlocal_names.or_default(u);
                }
            }
            for var in expand_defs(&all_defs, elems) {
                let var = SsaVariableIdentity::Location(var);
                defsites.or_default(var.clone()).insert(*bn);
                defined_here.or_default(var);
            }
            if let Some((symbol, _, definition)) =
                authored_scalar_source_variable(points, *bn, index, registry)
            {
                let identity = SsaVariableIdentity::Source(symbol);
                if definition.is_definition() {
                    if definition.is_may_write() {
                        nonlocal_names.or_default(identity.clone());
                    }
                    defsites.or_default(identity.clone()).insert(*bn);
                    defined_here.or_default(identity);
                } else if !defined_here.contains_key(&identity) {
                    nonlocal_names.or_default(identity);
                }
            }
            if let Some(receipt) =
                original_source_may_call(func, stmt, selection.metadata, registry)
            {
                for place in receipt.writes() {
                    let identity = SsaVariableIdentity::Source(place.symbol().clone());
                    // Source effects may be observed without a successful
                    // lexical read; retain their joins conservatively.
                    nonlocal_names.or_default(identity.clone());
                    defsites.or_default(identity.clone()).insert(*bn);
                    defined_here.or_default(identity);
                }
            }
        }
        record_authored_terminator_use(points, *bn, registry, &defined_here, &mut nonlocal_names);
        let term_uses = nonlocal_terminator_uses((*bn, block), selection, &mut scanner, points);
        for u in term_uses
            .iter()
            .cloned()
            .chain(expand_uses(&term_uses, &[], elems))
        {
            let u = SsaVariableIdentity::Location(u);
            if !defined_here.contains_key(&u) {
                nonlocal_names.or_default(u);
            }
        }
    }
    (nonlocal_names, defsites)
}
