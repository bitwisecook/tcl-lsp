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

//! The exact cells original reads and definitions select, and the namespace cells procedures read and write.

use crate::cfg::BlockId;
use std::collections::HashSet;

/// Collect every variable name defined anywhere in `cfg`.
///
/// Walks every block and pulls
/// the `defs` field off each [`crate::ir::Statement`] that has
/// one (assignments, ``incr``, ``Call`` statements with explicit
/// defs).  Used for the "did you mean…?" case-mismatch
/// suggestion in W210 / W211 / W220 messages.
pub(in super::super) fn collect_defined_vars(cfg: &crate::cfg::Function) -> HashSet<String> {
    use crate::ir::Statement;
    let mut names: HashSet<String> = HashSet::new();
    for block in cfg.blocks.values() {
        for stmt in &block.statements {
            match stmt {
                Statement::AssignConst { name, .. }
                | Statement::AssignExpr { name, .. }
                | Statement::AssignValue { name, .. }
                | Statement::Incr { name, .. } => {
                    let normalised = crate::naming::split_array_name_braced(name, true).0;
                    if !normalised.is_empty() {
                        names.insert(normalised.to_string());
                    }
                }
                Statement::Call { defs, .. } => {
                    for def in defs {
                        names.insert(def.clone());
                    }
                }
                _ => {}
            }
        }
    }
    names
}

/// Exact cross-procedure cells used only for diagnostic suppression.
#[derive(Default)]
pub(in super::super) struct DiagnosticCellFacts {
    pub known_defined: HashSet<crate::var_resolve::VariableCellKey>,
    pub externally_read: HashSet<crate::var_resolve::VariableCellKey>,
}

/// Selected successful definitions at the original statement boundary.
/// Alias registration and destruction are not value-producing writes.
pub(in super::super) fn original_definition_places(
    fu: &crate::compilation_unit::FunctionUnit,
    block: BlockId,
    index: usize,
    registry: &tcl_registry::CommandRegistry,
) -> Vec<crate::place::Place> {
    let Some(metadata) = fu.invocation_metadata_context(registry) else {
        return Vec::new();
    };
    let Some(tokens) =
        crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index).source_tokens()
    else {
        return Vec::new();
    };
    let Some(binding) = &tokens.source_binding else {
        return Vec::new();
    };
    let Some(normal) = crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
        registry,
        Some(metadata),
        tokens,
    ) else {
        return Vec::new();
    };
    if normal
        .variable_traits()
        .contains(tcl_registry::Traits::DESTROYS_VARIABLE)
        || normal.variable_alias_transitions().next().is_some()
    {
        return Vec::new();
    }
    normal
        .definition_places(&binding.variable_context, registry)
        .into_iter()
        .filter(|place| {
            if place.dynamic {
                return false;
            }
            if !place.observed {
                return true;
            }
            // A destination before variable callbacks is only a possible
            // definition. Retain an observed store's exact normal physical
            // successor; an error-only callback supplies no such world.
            binding.normal_variable_continuation().is_some_and(|after| {
                after.captured_contents_presence(place)
                    == crate::var_resolve::ContentsPresence::Defined
                    && place.cell.as_ref().is_some_and(|cell| {
                        cell.generation != crate::place::CellGeneration::Unknown
                            && after
                                .generations
                                .get(&crate::var_resolve::cell_key(place))
                                .copied()
                                .unwrap_or(crate::place::CellGeneration::Incoming)
                                == cell.generation
                    })
            })
        })
        .collect()
}

/// Consensus physical cell of the same original read, independently of SSA values.
pub(in super::super) fn original_read_cell(
    access: &crate::command_binding::SourceVariableAccess,
    registry: &tcl_registry::CommandRegistry,
) -> Option<crate::var_resolve::VariableCellKey> {
    if access.context_residual() != crate::command_binding::SourceVariableReadResidual::Closed {
        return None;
    }
    let mut selected = None;
    for context in access.context_alternatives() {
        let place = access.place_in_context(context, registry);
        let cell = crate::var_resolve::canonical_place_key(&place)?;
        if selected.as_ref().is_some_and(|previous| previous != &cell) {
            return None;
        }
        selected = Some(cell);
    }
    selected
}

/// Wrong-kind scalar read of an existing array, not an undefined contents read.
/// Every original physical alternative must retain the same live root kind.
pub(in super::super) fn original_read_is_live_array_scalar(
    access: &crate::command_binding::SourceVariableAccess,
    registry: &tcl_registry::CommandRegistry,
) -> bool {
    access.context_residual() == crate::command_binding::SourceVariableReadResidual::Closed
        && !access.context_alternatives().is_empty()
        && access.context_alternatives().iter().all(|context| {
            let place = access.place_in_context(context, registry);
            place.index.is_none()
                && context.root_contents_kind(&place)
                    == Some(crate::var_resolve::RootContentsKind::Array)
        })
}

/// Every retained occurrence at the span reads defined contents or sees a
/// live array root through a scalar read. Neither is an undefined read.
pub(in super::super) fn original_read_cannot_be_undefined_at_span(
    fu: &crate::compilation_unit::FunctionUnit,
    span: tcl_lexer::Span,
    registry: &tcl_registry::CommandRegistry,
) -> bool {
    let mut found = false;
    for (&block, body) in &fu.ssa.blocks {
        for index in (0..body.statements.len()).chain(std::iter::once(usize::MAX)) {
            let view = crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index);
            let Some(tokens) = view.source_tokens() else {
                continue;
            };
            for access in &tokens.variable_accesses {
                let original = fu.abs_span(access.source.span);
                if original.start() <= span.start() && span.end() <= original.end() {
                    found = true;
                    if !original_read_is_live_array_scalar(access, registry)
                        && !original_read_has_defined_contents(view, access, registry)
                    {
                        return false;
                    }
                }
            }
        }
    }
    found
}

fn original_read_has_defined_contents(
    view: crate::ssa::SsaSourceView<'_>,
    access: &crate::command_binding::SourceVariableAccess,
    registry: &tcl_registry::CommandRegistry,
) -> bool {
    view.read_contents_presence_alternatives_at(&access.source, &access.original_spelling, registry)
        .is_some_and(|presence| presence.may_be_defined() && !presence.may_be_undefined())
}

/// Exact attempted cell/version at one original diagnostic read occurrence.
/// Undefined versions remain diagnosable without a completed value receipt.
/// Conflicting source sites or physical cells cannot borrow a display label.
pub(in super::super) fn original_read_occurrence_at_span(
    fu: &crate::compilation_unit::FunctionUnit,
    span: tcl_lexer::Span,
) -> Option<crate::def_use::StorageValueKey> {
    let mut selected = None;
    for (&block, body) in &fu.ssa.blocks {
        for index in (0..body.statements.len()).chain(std::iter::once(usize::MAX)) {
            let view = crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index);
            let Some(tokens) = view.source_tokens() else {
                continue;
            };
            for access in &tokens.variable_accesses {
                let original = fu.abs_span(access.source.span);
                if original.start() > span.start() || span.end() > original.end() {
                    continue;
                }
                let (symbol, version) =
                    view.read_occurrence_reference(&access.source, &access.original_spelling)?;
                let value = (fu.ssa.cell_key(symbol)?.clone(), version);
                if selected.as_ref().is_some_and(|previous| previous != &value) {
                    return None;
                }
                selected = Some(value);
            }
        }
    }
    selected
}

/// Original physical cells read at this operation for one represented SSA slot.
pub(in super::super) fn original_read_cells_at(
    fu: &crate::ssa::SsaFunction,
    point: (BlockId, usize),
    value_cell: &crate::var_resolve::VariableCellKey,
    registry: &tcl_registry::CommandRegistry,
) -> Vec<crate::var_resolve::VariableCellKey> {
    crate::ssa::SsaSourceView::at_statement(fu, point.0, point.1)
        .source_tokens()
        .into_iter()
        .flat_map(|tokens| &tokens.variable_accesses)
        .filter(|access| {
            fu.read_reference_at(point.0, point.1, &access.source, &access.original_spelling)
                .is_some_and(|read| fu.cell_key(read.symbol) == Some(value_cell))
        })
        .filter_map(|access| original_read_cell(access, registry))
        .collect()
}

/// Original root-special-variable key of the exact cell defined here.
pub(in super::super) fn special_variable_definition(
    fu: &crate::compilation_unit::FunctionUnit,
    definition: &crate::def_use::DefSite,
    cell: &crate::var_resolve::VariableCellKey,
    registry: &tcl_registry::CommandRegistry,
) -> Option<String> {
    let block = fu.cfg.block_id(&definition.block)?;
    let index = usize::try_from(definition.statement_index).ok()?;
    original_definition_places(fu, block, index, registry)
        .iter()
        .find_map(|place| {
            (crate::var_resolve::canonical_binding_value_key(place).as_ref() == Some(cell))
                .then(|| {
                    crate::var_resolve::root_namespace_variable_simple_name(place)
                        .map(str::to_owned)
                })
                .flatten()
        })
}

/// A selected root definition at an original diagnostic source occurrence.
/// A declaration-only name without an actual definition supplies no effect.
pub(in super::super) fn special_variable_definition_at_span(
    fu: &crate::compilation_unit::FunctionUnit,
    span: tcl_lexer::Span,
    registry: &tcl_registry::CommandRegistry,
) -> Option<String> {
    let metadata = fu.invocation_metadata_context(registry)?;
    fu.cfg.blocks.iter().find_map(|(&block, data)| {
        data.statements.iter().enumerate().find_map(|(index, _)| {
            let tokens =
                crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index).source_tokens()?;
            let contains = |word: &tcl_source::words::WordExpr| {
                let original = fu.abs_span(word.source().span);
                original.start() <= span.start() && span.end() <= original.end()
            };
            // Only an argument word holding `span` can select a definition;
            // checking that first spares every other statement its resolution.
            if !tokens.words().iter().skip(1).any(contains) {
                return None;
            }
            let binding = tokens.source_binding.as_ref()?;
            let normal =
                crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
                    registry,
                    Some(metadata),
                    tokens,
                )?;
            if normal
                .variable_traits()
                .contains(tcl_registry::Traits::DESTROYS_VARIABLE)
                || normal.variable_alias_transitions().next().is_some()
            {
                return None;
            }
            normal
                .written_definition_places(&binding.variable_context, registry)
                .into_iter()
                .find_map(|(argument, place)| {
                    let original =
                        fu.abs_span(tokens.words().get(argument.checked_add(1)?)?.source().span);
                    if original.start() <= span.start() && span.end() <= original.end() {
                        crate::var_resolve::root_namespace_variable_simple_name(&place)
                            .map(str::to_owned)
                    } else {
                        None
                    }
                })
        })
    })
}

/// Namespace writes retain their selected cell and physical lifetime.
/// Written command names and displayed paths cannot donate a definition.
pub(in super::super) fn globals_written_by_procs(
    cu: &crate::compilation_unit::CompilationUnit,
    registry: &tcl_registry::CommandRegistry,
) -> HashSet<crate::var_resolve::VariableCellKey> {
    cu.procedures
        .values()
        .flat_map(|fu| {
            fu.ssa.blocks.iter().flat_map(move |(&block, data)| {
                (0..data.statements.len()).flat_map(move |index| {
                    let entered = crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index)
                        .source_tokens()
                        .and_then(|tokens| tokens.source_binding.as_ref())
                        .is_some_and(|binding| {
                            binding.runtime_reachability()
                                == crate::command_binding::SourceRuntimeReachability::Reached
                        });
                    entered
                        .then(|| original_definition_places(fu, block, index, registry))
                        .into_iter()
                        .flatten()
                        .filter(crate::place::Place::is_global)
                        .filter_map(|place| crate::var_resolve::canonical_place_key(&place))
                })
            })
        })
        .collect()
}

/// Whether the exact original definition is visible to a retained external read.
pub(in super::super) fn definition_has_cell_fact(
    fu: &crate::compilation_unit::FunctionUnit,
    definition: &crate::def_use::DefSite,
    value_cell: &crate::var_resolve::VariableCellKey,
    facts: &HashSet<crate::var_resolve::VariableCellKey>,
    registry: &tcl_registry::CommandRegistry,
) -> bool {
    let Some(block) = fu.cfg.block_id(&definition.block) else {
        return false;
    };
    let Ok(index) = usize::try_from(definition.statement_index) else {
        return false;
    };
    original_definition_places(fu, block, index, registry)
        .iter()
        .any(|place| {
            crate::var_resolve::canonical_binding_value_key(place).as_ref() == Some(value_cell)
                && crate::var_resolve::canonical_place_key(place)
                    .is_some_and(|cell| facts.contains(&cell))
        })
}

/// Every captured read of this represented value must have defined contents
/// or select an advised physical cell.
pub(in super::super) fn read_has_cell_fact(
    fu: &crate::compilation_unit::FunctionUnit,
    point: (BlockId, usize),
    value_cell: &crate::var_resolve::VariableCellKey,
    facts: &HashSet<crate::var_resolve::VariableCellKey>,
    registry: &tcl_registry::CommandRegistry,
) -> bool {
    let view = crate::ssa::SsaSourceView::at_statement(&fu.ssa, point.0, point.1);
    let Some(tokens) = view.source_tokens() else {
        return false;
    };
    let mut seen = false;
    for access in &tokens.variable_accesses {
        let place = access.place_in_context(&access.variable_context, registry);
        if crate::var_resolve::canonical_binding_value_key(&place).as_ref() != Some(value_cell) {
            continue;
        }
        seen = true;
        if !original_read_has_defined_contents(view, access, registry)
            && !original_read_cell(access, registry).is_some_and(|cell| facts.contains(&cell))
        {
            return false;
        }
    }
    seen
}

/// SSA keeps a name dependency across argument evaluation for liveness. The
/// original continuation and read inventory determine whether it is a read.
pub(in super::super) fn original_argument_use_has_no_read(
    fu: &crate::compilation_unit::FunctionUnit,
    statement: &crate::ir::Statement,
    point: (BlockId, usize),
    cell: &crate::var_resolve::VariableCellKey,
    registry: &tcl_registry::CommandRegistry,
) -> bool {
    use crate::var_resolve::ContentsPresence;
    let Some(points) = &fu.ssa.point_contexts else {
        return false;
    };
    let Some(binding) = points
        .source_tokens_at(point.0, point.1)
        .and_then(|tokens| tokens.source_binding.as_ref())
    else {
        return false;
    };
    binding.variable_context.contents_presence.get(cell) == Some(&ContentsPresence::Defined)
        && !points.source_arguments_unknown_at(point.0, point.1, registry)
        && crate::place_bridge::read_places_at(statement, point.0, point.1, points, registry)
            .iter()
            .all(|place| {
                crate::var_resolve::canonical_binding_value_key(place)
                    .is_some_and(|key| key != *cell)
            })
}

/// Original namespace reads retain exact table, cell and lifetime identity.
/// The consumer compares these keys before projecting diagnostic labels.
pub(in super::super) fn globals_read_by_procs(
    cu: &crate::compilation_unit::CompilationUnit,
    registry: &tcl_registry::CommandRegistry,
) -> HashSet<crate::var_resolve::VariableCellKey> {
    let mut cells = HashSet::new();
    for fu in cu.procedures.values() {
        for (&block, data) in &fu.ssa.blocks {
            for index in (0..data.statements.len()).chain(std::iter::once(usize::MAX)) {
                let Some(tokens) =
                    crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index).source_tokens()
                else {
                    continue;
                };
                cells.extend(tokens.variable_accesses.iter().filter_map(|access| {
                    let cell = original_read_cell(access, registry)?;
                    cell.namespace_membership_for_advice()
                        .is_some()
                        .then_some(cell)
                }));
                for place in tokens
                    .source_binding
                    .as_ref()
                    .and_then(|binding| binding.invocation_variable_reads.as_ref())
                    .into_iter()
                    .flat_map(|reads| &reads.native_reads)
                    .map(|read| &read.place)
                {
                    if place.is_global()
                        && let Some(key) = crate::var_resolve::canonical_place_key(place)
                    {
                        cells.insert(key);
                    }
                }
            }
        }
    }
    cells
}
