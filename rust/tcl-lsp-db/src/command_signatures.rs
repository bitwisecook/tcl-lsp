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

//! Project-wide command signatures and arities from source headers: the original lookup's per-slot queries and the logical source arities.

use super::declarations::item_sigs;
use super::project_source_inputs::original_command_signatures_for_inputs;
use super::{Project, ProjectSourceInputMode, TclDb, project_source_inputs, project_token_inputs};
use std::collections::HashMap;
use std::sync::Arc;
use tcl_compiler::analyser::ItemSig;

/// Inclusive count bounds from the formal owner's frozen header metadata.
/// Unknown counts remain fully open. Selected C/Jim grammar and original
/// formal storage are projected before the body-free database boundary.
pub(super) fn proc_arity(
    count: tcl_compiler::signature_scan::formal_count::SourceFormalCountProjection,
) -> (usize, usize) {
    // naming.database.original-formal-count-header
    // docs/design/analysis/name-resolution-proofs/database-original-formal-count-header.md
    let arity = count.arity();
    let max = if arity.is_unlimited() {
        usize::MAX
    } else {
        usize::from(arity.max)
    };
    (usize::from(arity.min), max)
}

/// Body-free original source headers under their exact publication keys.
/// Current source declarations grant no installed command or callback entry.
#[salsa::tracked(returns(clone))]
pub fn project_original_command_signatures(
    db: &dyn TclDb,
    project: Project,
) -> Arc<
    HashMap<
        tcl_compiler::signature_scan::scope::SignatureSourceCommand,
        Vec<tcl_compiler::analyser::SourceDeclarationSignature>,
    >,
> {
    merge_original_command_signatures(project.files(db).iter().map(|&file| item_sigs(db, file)))
}

pub(super) fn merge_original_command_signatures(
    signatures: impl IntoIterator<Item = Arc<Vec<ItemSig>>>,
) -> Arc<
    HashMap<
        tcl_compiler::signature_scan::scope::SignatureSourceCommand,
        Vec<tcl_compiler::analyser::SourceDeclarationSignature>,
    >,
> {
    let mut entries: HashMap<_, Vec<_>> = HashMap::new();
    for signatures in signatures {
        for signature in signatures.iter() {
            let Some(header) = signature.original_declaration.as_ref() else {
                continue;
            };
            let entry = entries.entry(header.name().clone()).or_default();
            if !entry.contains(header) {
                entry.push(header.clone());
            }
        }
    }
    Arc::new(entries)
}

/// Count projection of the shared original source-header query. Non-procedure
/// publications retain lookup barriers without borrowing procedure counts.
#[salsa::tracked(returns(clone))]
pub fn project_original_command_arities(
    db: &dyn TclDb,
    project: Project,
) -> Arc<HashMap<tcl_compiler::signature_scan::scope::SignatureSourceCommand, Vec<(usize, usize)>>>
{
    use tcl_compiler::analyser::ItemKind;
    Arc::new(
        project_original_command_signatures(db, project)
            .iter()
            .map(|(name, headers)| {
                let mut arities = if headers
                    .iter()
                    .any(|header| header.kind() == ItemKind::Class)
                {
                    Vec::new()
                } else {
                    headers
                        .iter()
                        .filter(|header| header.kind() == ItemKind::Proc)
                        .map(|header| proc_arity(header.formal_count_projection()))
                        .collect::<Vec<_>>()
                };
                arities.sort_unstable();
                arities.dedup();
                (name.clone(), arities)
            })
            .collect(),
    )
}

/// Interned exact byte lookup key, without a command-publication grant.
#[salsa::interned]
pub struct OriginalCommandSlot<'db> {
    /// Complete selected name policy, including authored/native authority.
    #[returns(copy)]
    pub policy: tcl_syntax::naming::NamePolicyProtocol,
    /// Exact lookup geometry, without a publication or allocation receipt.
    #[returns(ref)]
    pub slot: tcl_core_types::ByteCommandSlot,
}

/// Per-slot exact source headers retain independent query early cutoff.
/// Display tails and current command occupancy do not enter this key.
#[salsa::tracked(returns(clone))]
pub fn original_command_signatures<'db>(
    db: &'db dyn TclDb,
    project: Project,
    name: OriginalCommandSlot<'db>,
) -> Option<Arc<Vec<tcl_compiler::analyser::SourceDeclarationSignature>>> {
    let table = project_original_command_signatures(db, project);
    select_original_command_signatures(db, name, &table)
}

pub(super) fn select_original_command_signatures(
    db: &dyn TclDb,
    name: OriginalCommandSlot<'_>,
    table: &HashMap<
        tcl_compiler::signature_scan::scope::SignatureSourceCommand,
        Vec<tcl_compiler::analyser::SourceDeclarationSignature>,
    >,
) -> Option<Arc<Vec<tcl_compiler::analyser::SourceDeclarationSignature>>> {
    let selected = tcl_compiler::signature_scan::scope::first_matching_byte_publications(
        name.policy(db),
        std::slice::from_ref(name.slot(db)),
        table.iter(),
    );
    if selected.is_empty() {
        return None;
    }
    Some(Arc::new(
        selected
            .into_iter()
            .flat_map(|headers| headers.iter().cloned())
            .collect(),
    ))
}

/// Exact current source headers under the original lookup's retained ordered
/// alternatives. This does not select a future implementation or entered frame.
#[must_use]
pub fn original_lookup_command_signatures(
    db: &dyn TclDb,
    project: Project,
    lookup: &tcl_compiler::command_binding::OriginalCommandLookup,
) -> Option<Arc<Vec<tcl_compiler::analyser::SourceDeclarationSignature>>> {
    original_lookup_command_signatures_using(db, lookup, |key| {
        original_command_signatures(db, project, key)
    })
}

pub(super) fn original_lookup_command_signatures_using<'db>(
    db: &'db dyn TclDb,
    lookup: &tcl_compiler::command_binding::OriginalCommandLookup,
    mut query: impl FnMut(
        OriginalCommandSlot<'db>,
    ) -> Option<Arc<Vec<tcl_compiler::analyser::SourceDeclarationSignature>>>,
) -> Option<Arc<Vec<tcl_compiler::analyser::SourceDeclarationSignature>>> {
    let mut slots = Vec::new();
    for candidate in lookup.candidates().iter().flatten() {
        if slots.iter().any(|(slot, _)| slot == candidate) {
            continue;
        }
        let key = OriginalCommandSlot::new(db, lookup.policy(), candidate.clone());
        if let Some(headers) = query(key) {
            slots.push((candidate.clone(), headers));
        }
    }
    let selected = lookup.matching_slot_publications(
        slots
            .iter()
            .map(|(slot, headers)| (slot, lookup.policy(), Arc::clone(headers))),
    )?;
    if selected.is_empty() {
        return None;
    }
    Some(Arc::new(
        selected
            .into_iter()
            .flat_map(|headers| headers.as_ref().clone())
            .collect(),
    ))
}

/// Per-slot original declaration arities. Early cutoff retains independent
/// consumers when an unrelated header changes; UI tails cannot enter the key.
#[salsa::tracked(returns(clone))]
pub fn original_command_arity<'db>(
    db: &'db dyn TclDb,
    project: Project,
    name: OriginalCommandSlot<'db>,
) -> Option<Arc<Vec<(usize, usize)>>> {
    let table = project_original_command_arities(db, project);
    let matching = tcl_compiler::signature_scan::scope::first_matching_byte_publications(
        name.policy(db),
        std::slice::from_ref(name.slot(db)),
        table.iter(),
    );
    if matching.is_empty() {
        return None;
    }
    if matching.iter().any(|arities| arities.is_empty()) {
        return Some(Arc::new(Vec::new()));
    }
    let mut arities = matching
        .into_iter()
        .flat_map(|arities| arities.iter().copied())
        .collect::<Vec<_>>();
    arities.sort_unstable();
    arities.dedup();
    Some(Arc::new(arities))
}

#[salsa::tracked(returns(clone))]
fn original_command_arity_for_inputs<'db>(
    db: &'db dyn TclDb,
    project: Project,
    name: OriginalCommandSlot<'db>,
) -> Option<Arc<Vec<(usize, usize)>>> {
    use tcl_compiler::analyser::ItemKind;
    let headers = original_command_signatures_for_inputs(db, project, name)?;
    let mut arities = if headers.iter().any(|header| header.kind() != ItemKind::Proc) {
        Vec::new()
    } else {
        headers
            .iter()
            .map(|header| proc_arity(header.formal_count_projection()))
            .collect()
    };
    arities.sort_unstable();
    arities.dedup();
    Some(Arc::new(arities))
}

/// Header arity assistance at an authentic original call's retained lookup.
/// Ordered paths and policy come from the shared owner; unknown alternatives
/// cannot be replaced by a bare-tail search. Each demanded slot retains the
/// signature query's early cutoff independently of unrelated declarations.
#[must_use]
pub fn original_lookup_command_arities(
    db: &dyn TclDb,
    project: Project,
    lookup: &tcl_compiler::command_binding::OriginalCommandLookup,
) -> Option<Arc<Vec<(usize, usize)>>> {
    original_lookup_command_arities_with_mode(
        (db, project, ProjectSourceInputMode::Standalone),
        lookup,
    )
}

pub(super) fn original_lookup_command_arities_with_mode(
    source: (&dyn TclDb, Project, ProjectSourceInputMode),
    lookup: &tcl_compiler::command_binding::OriginalCommandLookup,
) -> Option<Arc<Vec<(usize, usize)>>> {
    let (db, project, mode) = source;
    let mut slots = Vec::new();
    for candidate in lookup.candidates().iter().flatten() {
        if slots.iter().any(|(slot, _)| slot == candidate) {
            continue;
        }
        let key = OriginalCommandSlot::new(db, lookup.policy(), candidate.clone());
        let arities = match mode {
            ProjectSourceInputMode::Standalone => original_command_arity(db, project, key),
            ProjectSourceInputMode::Supplied => original_command_arity_for_inputs(db, project, key),
        };
        if let Some(arities) = arities {
            slots.push((candidate.clone(), arities));
        }
    }
    let selected = lookup.matching_slot_publications(
        slots
            .iter()
            .map(|(slot, arities)| (slot, lookup.policy(), Arc::clone(arities))),
    )?;
    if selected.is_empty() {
        return None;
    }
    if selected.iter().any(|arities| arities.is_empty()) {
        return Some(Arc::new(Vec::new()));
    }
    let mut arities = selected
        .iter()
        .flat_map(|arities| arities.iter().copied())
        .collect::<Vec<_>>();
    arities.sort_unstable();
    arities.dedup();
    Some(Arc::new(arities))
}

/// A Logical source slot carries no Native lookup policy or publication.
#[salsa::interned]
pub(super) struct LogicalSourceCommandSlot<'db> {
    #[returns(ref)]
    slot: tcl_core_types::ByteCommandSlot,
}

// Each entry is a body-free projection of independently checked source headers.
// Missing command-like header ownership keeps the project inventory unavailable.
fn merge_logical_source_arities(
    signatures: impl IntoIterator<Item = Arc<Vec<ItemSig>>>,
) -> Option<HashMap<tcl_core_types::ByteCommandSlot, Vec<(usize, usize)>>> {
    use tcl_compiler::analyser::ItemKind;
    let mut entries: HashMap<_, (Vec<_>, bool)> = HashMap::new();
    for signatures in signatures {
        for signature in signatures.iter().filter(|signature| {
            matches!(
                signature.id.kind,
                ItemKind::Proc | ItemKind::Class | ItemKind::Alias | ItemKind::Ensemble
            )
        }) {
            let slot = signature.logical_source_coordinate.as_ref()?;
            let entry = entries.entry(slot.clone()).or_default();
            if signature.id.kind == ItemKind::Proc {
                entry.0.push(proc_arity(signature.formal_count));
            } else {
                entry.1 = true;
            }
        }
    }
    Some(
        entries
            .into_iter()
            .map(|(slot, (arities, non_proc))| (slot, if non_proc { Vec::new() } else { arities }))
            .collect(),
    )
}

type ProjectLogicalSourceArities =
    Option<Arc<HashMap<tcl_core_types::ByteCommandSlot, Vec<(usize, usize)>>>>;

#[salsa::tracked(returns(clone))]
fn project_logical_source_arities(
    db: &dyn TclDb,
    project: Project,
    supplied: bool,
) -> ProjectLogicalSourceArities {
    let mut signatures = Vec::new();
    for &file in project.files(db) {
        signatures.push(if supplied {
            let config = project_token_inputs::configuration_for_file(db, project, file)?;
            project_source_inputs::checked_item_sigs_for_config(db, file, config)?
        } else {
            item_sigs(db, file)
        });
    }
    merge_logical_source_arities(signatures).map(Arc::new)
}

#[salsa::tracked(returns(clone))]
pub(super) fn logical_source_command_arity<'db>(
    db: &'db dyn TclDb,
    project: Project,
    key: LogicalSourceCommandSlot<'db>,
    supplied: bool,
) -> Option<Arc<Vec<(usize, usize)>>> {
    project_logical_source_arities(db, project, supplied)?
        .get(key.slot(db))
        .cloned()
        .map(Arc::new)
}
