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

//! The reads a function retains: the cells each source read binds, and the read references, script source points and contents origins built from them.

use super::super::source_view::source_read_place;
use super::super::{
    OriginalScriptSourcePoint, SsaBlock, SsaReadReference, Symbol, UseClass, ValueClobbers, Version,
};
use super::VarInterner;
use super::phi_placement::authored_scalar_source_variable;
use crate::cfg::BlockId;
use crate::ir::{SourceSite, Statement, WordExpr, WordPart};
use crate::var_refs::{VarReferenceScanner, VarScanOptions};
use crate::var_resolve::{ContentsOrigin, ResolveContext, VariableCellKey};
use crate::variable_bindings::PointResolveContexts;
use std::collections::{HashMap, HashSet};
use tcl_registry::CommandRegistry;

/// Exact read spelling projection; unavailable original syntax cannot relabel
/// the retained read or suppress an independently scanned protective dependency.
fn source_read_name_with_config(
    access: &crate::command_binding::SourceVariableAccess,
    registry: &CommandRegistry,
) -> Option<String> {
    let config = access.variable_context.read_lexer_config(registry)?;
    Some(
        tcl_syntax::naming::var_reference_for_style(&access.original_spelling, config.braced_var)
            .to_owned(),
    )
}

/// Existing test probe delegates the same actual configured source projection.
#[cfg(test)]
pub(in super::super) fn source_read_name(
    access: &crate::command_binding::SourceVariableAccess,
) -> String {
    source_read_name_with_config(access, &CommandRegistry::build_default())
        .unwrap_or_else(|| access.original_spelling.clone())
}

pub(super) fn captured_read_names(
    points: &PointResolveContexts,
    block: BlockId,
    index: usize,
    registry: &CommandRegistry,
) -> HashSet<String> {
    points
        .source_reads_at(block, index)
        .iter()
        .filter_map(|access| source_read_name_with_config(access, registry))
        .collect()
}

pub(super) fn invocation_read_bindings(
    points: &PointResolveContexts,
    block: BlockId,
    index: usize,
    context: &ResolveContext,
    registry: &CommandRegistry,
) -> Vec<VariableCellKey> {
    // Implementation contract: naming.variable.invocation-caller-ssa-frame
    // docs/design/analysis/name-resolution-proofs/variable-invocation-caller-ssa-frame.md
    points
        .source_tokens_at(block, index)
        .into_iter()
        .flat_map(|tokens| {
            // A callee's static storage belongs to the callable, not to any
            // caller frame: the caller cannot reach it, so the callee's reads
            // of it are not caller SSA reads.
            crate::place_bridge::invocation_execution_read_places(tokens, registry)
                .into_iter()
                .filter_map(|place| crate::var_resolve::canonical_binding_value_key(&place))
                .filter(|key| !is_callable_storage(key))
                .chain(
                    crate::place_bridge::invocation_argument_execution_read_places_at(
                        points, block, index, registry,
                    )
                    .into_iter()
                    .filter_map(|place| crate::var_resolve::canonical_binding_value_key(&place)),
                )
        })
        .filter_map(|key| invocation_read_key_in_context(key, context))
        .collect()
}

/// Whether `key` is a callable's own static storage (Jim `proc … {{n 0}}`).
fn is_callable_storage(key: &VariableCellKey) -> bool {
    matches!(
        key,
        VariableCellKey::RetainedSlot(slot)
            if matches!(slot.as_ref(), crate::raw_binding::RawBindingSlotId::Callable { .. })
    )
}

/// Execution inventories retain all physical subtree reads. Foreign callee
/// locals are not caller SSA cells; shared namespace and caller alias cells stay.
pub(in super::super) fn invocation_read_key_in_context(
    key: VariableCellKey,
    context: &ResolveContext,
) -> Option<VariableCellKey> {
    // Implementation contract: naming.variable.invocation-caller-ssa-frame
    // docs/design/analysis/name-resolution-proofs/variable-invocation-caller-ssa-frame.md
    key.activation_identity()
        .is_none_or(|activation| context.activation.as_deref() == Some(activation))
        .then_some(key)
}

pub(super) fn source_read_bindings(
    points: &PointResolveContexts,
    block: BlockId,
    index: usize,
    registry: &CommandRegistry,
) -> Vec<(String, VariableCellKey)> {
    // A captured read the walk made inside a callee it followed is of the
    // callee's own frame: like an invocation's execution reads, a cell of
    // another activation is no SSA cell of this function.
    let context = if index == usize::MAX {
        points.before_terminator(block)
    } else {
        points.before_statement(block, index)
    };
    points
        .source_reads_at(block, index)
        .iter()
        .filter_map(|access| {
            let name = crate::var_resolve::canonical_binding_value_key(&source_read_place(
                access, registry,
            ))?;
            let name = invocation_read_key_in_context(name, context)?;
            Some((source_read_name_with_config(access, registry)?, name))
        })
        .collect()
}

pub(super) fn retained_read_references(
    points: &PointResolveContexts,
    blocks: &HashMap<BlockId, SsaBlock>,
    interner: &VarInterner,
    registry: &CommandRegistry,
    clobbers: &ValueClobbers,
) -> HashMap<(BlockId, usize, SourceSite, String), SsaReadReference> {
    let origins = represented_contents_origins(blocks, clobbers);
    let mut references = HashMap::new();
    for (&block_id, block) in blocks {
        for index in (0..block.statements.len()).chain(std::iter::once(usize::MAX)) {
            for access in points.source_reads_at(block_id, index) {
                let place = source_read_place(access, registry);
                let Some(name) = crate::var_resolve::canonical_binding_value_key(&place) else {
                    continue;
                };
                let Some(&symbol) = interner.to_symbol.get(&name) else {
                    continue;
                };
                let candidate = if index == usize::MAX {
                    block.exit_versions.get(&symbol).copied().unwrap_or(0)
                } else {
                    block.statements[index]
                        .uses
                        .get(&symbol)
                        .copied()
                        .unwrap_or(0)
                };
                let source_origin = access
                    .variable_context
                    .read_contents_origin(&place, registry);
                let represented = if candidate == 0 {
                    Some(&ContentsOrigin::Incoming)
                } else {
                    origins.get(&(symbol, candidate))
                };
                let version = (!place.observed
                    && access.variable_context.contents_presence(&place)
                        == crate::var_resolve::ContentsPresence::Defined
                    && !access.variable_context.store_would_error(&place)
                    && source_origin != ContentsOrigin::Unknown
                    && represented.is_some_and(|represented| source_origin.is_within(represented)))
                .then_some(candidate);
                references.insert(
                    (
                        block_id,
                        index,
                        access.source.clone(),
                        access.original_spelling.clone(),
                    ),
                    SsaReadReference { symbol, version },
                );
            }
        }
    }
    retain_cached_input_references(points, &mut references, registry);
    references
}

pub(super) fn original_script_source_points(
    points: &PointResolveContexts,
    blocks: &HashMap<BlockId, SsaBlock>,
    interner: &VarInterner,
    registry: &CommandRegistry,
) -> HashMap<(BlockId, usize), OriginalScriptSourcePoint> {
    let mut rows = HashMap::new();
    for (&(block, index), tokens) in points.source_token_inventory() {
        let Some(binding) = tokens.source_binding.as_ref() else {
            continue;
        };
        if binding
            .original_standalone_script_variable_scope(tokens, registry)
            .is_none()
        {
            continue;
        }
        let Some(site) = binding.invocation_site() else {
            continue;
        };
        let Some(offset) = tokens
            .words()
            .first()
            .map(|word| word.source().span.start())
        else {
            continue;
        };
        let mut original = tokens.clone();
        crate::lattice_rebase::rebase_command_tokens(
            &mut original,
            i64::from(site.offset) - i64::from(offset),
        );
        let Some((identity, _, access)) =
            crate::registry_invocation::original_authored_script_scalar_variable(
                &original, registry,
            )
        else {
            continue;
        };
        let Some(&symbol) = interner.source_to_symbol.get(&identity) else {
            continue;
        };
        let Some(body) = blocks.get(&block) else {
            continue;
        };
        let (used, defined) = if index == usize::MAX {
            (body.exit_versions.get(&symbol).copied(), None)
        } else {
            let Some(statement) = body.statements.get(index) else {
                continue;
            };
            (
                statement.uses.get(&symbol).copied(),
                statement.defs.get(&symbol).copied(),
            )
        };
        rows.insert(
            (block, index),
            OriginalScriptSourcePoint {
                symbol,
                access,
                used,
                defined,
            },
        );
    }
    rows
}

pub(super) fn retained_source_read_references(
    points: &PointResolveContexts,
    blocks: &HashMap<BlockId, SsaBlock>,
    symbols: &HashMap<(BlockId, usize), HashMap<String, Symbol>>,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    interner: &VarInterner,
) -> HashMap<(BlockId, usize, SourceSite, String), SsaReadReference> {
    let mut references = HashMap::new();
    let mut scanner = VarReferenceScanner::with_config(
        VarScanOptions {
            include_var_read_roles: false,
            recurse_cmd_substitutions: false,
            include_reads_before_write: false,
            element_qualified: true,
        },
        config,
    );
    for (&(block, index), tokens) in points.source_token_inventory() {
        if tokens.synthetic.is_some() || !tokens.evaluates_words() {
            continue;
        }
        let Some(names) = symbols.get(&(block, index)) else {
            continue;
        };
        let Some(body) = blocks.get(&block) else {
            continue;
        };
        let authored = authored_scalar_source_variable(points, block, index, registry);
        let mut retain = |spelling: &str, source: &SourceSite| {
            if source.provenance != crate::ir::Provenance::Source {
                return;
            }
            let read_names = scanner.scan_lexical_word(spelling, registry);
            let mut selected = read_names.iter();
            let Some(name) = selected.next() else {
                return;
            };
            if selected.next().is_some() {
                return;
            }
            let Some(&symbol) = names.get(name) else {
                return;
            };
            let represented = if index == usize::MAX {
                body.exit_versions.contains_key(&symbol)
            } else {
                body.statements
                    .get(index)
                    .is_some_and(|statement| statement.uses.contains_key(&symbol))
            };
            if represented {
                references.insert(
                    (block, index, source.clone(), spelling.to_owned()),
                    SsaReadReference {
                        symbol,
                        version: authored
                            .as_ref()
                            .filter(|(held, held_name, definition)| {
                                !definition.is_definition()
                                    && held_name == name
                                    && interner.source_variables.get(&symbol) == Some(held)
                            })
                            .and_then(|_| {
                                if index == usize::MAX {
                                    body.exit_versions.get(&symbol).copied()
                                } else {
                                    body.statements.get(index)?.uses.get(&symbol).copied()
                                }
                            }),
                    },
                );
            }
        };
        for word in tokens.words() {
            match word {
                WordExpr::Variable { spelling, source } => retain(spelling, source),
                WordExpr::Template {
                    parts,
                    rejected: None,
                    ..
                } => {
                    for part in parts {
                        if let WordPart::Variable { spelling, source } = part {
                            retain(spelling, source);
                        }
                    }
                }
                _ => {}
            }
        }
    }
    references
}

fn retain_cached_input_references(
    points: &PointResolveContexts,
    references: &mut HashMap<(BlockId, usize, SourceSite, String), SsaReadReference>,
    registry: &CommandRegistry,
) {
    type ReachedReadAt = (BlockId, usize, SsaReadReference);
    let mut by_source: HashMap<(&SourceSite, &str), Vec<ReachedReadAt>> = HashMap::new();
    for ((block, index, source, spelling), &read) in references.iter() {
        by_source
            .entry((source, spelling.as_str()))
            .or_default()
            .push((*block, *index, read));
    }
    let mut cached = Vec::new();
    for (&(block, index), tokens) in points.source_token_inventory() {
        if !matches!(
            tokens.synthetic,
            Some(crate::ir::SyntheticMarker::IterationBindings(_))
        ) {
            continue;
        }
        for access in &tokens.variable_accesses {
            let mut selected = None;
            let mut conflict = false;
            for &(origin_block, origin_index, read) in by_source
                .get(&(&access.source, access.original_spelling.as_str()))
                .into_iter()
                .flatten()
            {
                if !points
                    .source_reads_at(origin_block, origin_index)
                    .iter()
                    .any(|original| {
                        original.source == access.source
                            && original.original_spelling == access.original_spelling
                            && same_captured_read(original, access, registry)
                    })
                {
                    continue;
                }
                if read.version.is_none() || selected.is_some_and(|previous| previous != read) {
                    conflict = true;
                    break;
                }
                selected = Some(read);
            }
            if !conflict && let Some(read) = selected {
                cached.push((
                    (
                        block,
                        index,
                        access.source.clone(),
                        access.original_spelling.clone(),
                    ),
                    read,
                ));
            }
        }
    }
    references.extend(cached);
}

fn same_captured_read(
    original: &crate::command_binding::SourceVariableAccess,
    captured: &crate::command_binding::SourceVariableAccess,
    registry: &CommandRegistry,
) -> bool {
    let actual = source_read_place(original, registry);
    let retained = source_read_place(captured, registry);
    // Unrelated namespace and frame-world changes do not change a captured
    // input. Its physical address, observer state and reaching contents do.
    actual == retained
        && original
            .variable_context
            .read_contents_origin(&actual, registry)
            == captured
                .variable_context
                .read_contents_origin(&retained, registry)
}

pub(in super::super) fn represented_contents_origins(
    blocks: &HashMap<BlockId, SsaBlock>,
    clobbers: &ValueClobbers,
) -> HashMap<(Symbol, Version), ContentsOrigin> {
    let mut origins = HashMap::new();
    // Both merges and analysis-only scalar copies start at bottom until a
    // predecessor contributes. Missing copy results are not unknown stores.
    let derived_versions: HashSet<_> = blocks
        .values()
        .flat_map(|block| block.phis.iter().map(|phi| (phi.name, phi.version)))
        .chain(
            clobbers
                .values()
                .flat_map(|markers| markers.values())
                .flat_map(|versions| {
                    versions
                        .iter()
                        .map(|(&symbol, &(_, fresh))| (symbol, fresh))
                }),
        )
        .collect();
    for block in blocks.values() {
        for statement in &block.statements {
            for (&symbol, &version) in &statement.defs {
                origins.insert(
                    (symbol, version),
                    if statement.may_defs.contains(&symbol)
                        || statement.destruction_defs.contains(&symbol)
                    {
                        ContentsOrigin::Unknown
                    } else {
                        ContentsOrigin::WrittenAt(statement.statement.span().start())
                    },
                );
            }
        }
    }
    loop {
        let mut changed = false;
        // A scalar barrier creates a widened analysis value, not an
        // executable store. Keep its exact binding's original contents
        // lineage so retained reads select the fresh lattice key rather than
        // falling back to a source store before the barrier.
        for versions in clobbers.values().flat_map(|markers| markers.values()) {
            for (&symbol, &(prior, fresh)) in versions {
                let origin = if prior == 0 {
                    Some(ContentsOrigin::Incoming)
                } else {
                    origins.get(&(symbol, prior)).cloned().or_else(|| {
                        (!derived_versions.contains(&(symbol, prior)))
                            .then_some(ContentsOrigin::Unknown)
                    })
                };
                if let Some(origin) = origin
                    && origins.get(&(symbol, fresh)) != Some(&origin)
                {
                    origins.insert((symbol, fresh), origin);
                    changed = true;
                }
            }
        }
        for block in blocks.values() {
            for phi in &block.phis {
                let joined = phi
                    .incoming
                    .values()
                    .filter_map(|&version| {
                        if version == 0 {
                            Some(ContentsOrigin::Incoming)
                        } else {
                            origins.get(&(phi.name, version)).cloned().or_else(|| {
                                (!derived_versions.contains(&(phi.name, version)))
                                    .then_some(ContentsOrigin::Unknown)
                            })
                        }
                    })
                    .reduce(|left, right| left.joined(&right));
                if let Some(origin) = joined
                    && origins.get(&(phi.name, phi.version)) != Some(&origin)
                {
                    origins.insert((phi.name, phi.version), origin);
                    changed = true;
                }
            }
        }
        if !changed {
            return origins;
        }
    }
}

pub(super) fn additional_read_bindings(
    points: &PointResolveContexts,
    block: BlockId,
    index: usize,
    statement: &Statement,
    context: &ResolveContext,
    registry: &CommandRegistry,
) -> Vec<(VariableCellKey, UseClass)> {
    scope_read_bindings(statement, context, registry)
        .into_iter()
        .chain(
            invocation_read_bindings(points, block, index, context, registry)
                .into_iter()
                .map(|name| (name, UseClass::Name)),
        )
        .collect()
}

fn scope_read_bindings(
    statement: &Statement,
    context: &ResolveContext,
    registry: &CommandRegistry,
) -> Vec<(VariableCellKey, UseClass)> {
    crate::dictionary_bindings::scope_marker_effects(statement, context, registry)
        .into_iter()
        .flat_map(|effects| effects.reads)
        .filter_map(|place| {
            crate::var_resolve::canonical_binding_value_key(&place)
                .map(|name| (name, UseClass::Name))
        })
        .collect()
}
