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

//! The state intents of an instruction that is not a transition: variable cells, lowered footprints, expressions and word evaluation.

use crate::effect_ssa::WorldStateIntentKind;
use crate::executable_ir::{CellReference, ExecutableExpr, ExecutableFunction, LoweredFootprint};
use crate::state_ssa::adapters::{
    WorldInterpreterScope, WorldNamespaceScope, WorldRegion, WorldRegionKind,
};

/// The world region of one Tcl variable cell in the scopes current at the site.
fn cell_region(cell: &CellReference) -> WorldRegion {
    match cell.name() {
        Some(name) => WorldRegion::exact(
            WorldRegionKind::VariableStore,
            WorldInterpreterScope::Current,
            WorldNamespaceScope::Current,
            name,
        ),
        None => WorldRegion::domain_wildcard(
            WorldRegionKind::VariableStore,
            WorldInterpreterScope::Current,
            WorldNamespaceScope::Current,
        ),
    }
}

pub(super) fn cell_intents<'a>(
    cells: impl Iterator<Item = &'a CellReference>,
    kind: WorldStateIntentKind,
) -> Vec<(WorldStateIntentKind, WorldRegion)> {
    let mut intents: Vec<(WorldStateIntentKind, WorldRegion)> = Vec::new();
    let mut writes_a_cell = false;
    for cell in cells {
        writes_a_cell |= kind == WorldStateIntentKind::Clobber;
        let region = cell_region(cell);
        if !intents.iter().any(|(k, r)| *k == kind && *r == region) {
            intents.push((kind, region));
        }
    }
    if writes_a_cell {
        // A cell write is observable through a variable trace, so it depends on
        // the trace registrations of the domain.  Whether such a trace exists —
        // and therefore whether a callback runs here — is the contents/absence
        // lattice's question, exactly as it already is for a variable read.
        intents.push((
            WorldStateIntentKind::Use,
            WorldRegion::domain_wildcard(
                WorldRegionKind::VariableTraces,
                WorldInterpreterScope::Current,
                WorldNamespaceScope::Current,
            ),
        ));
    }
    intents
}

/// Project the exact footprint of an already-lowered assignment, increment,
/// expression, or return into world-state intents.
///
/// An operand that can run nested commands keeps the conservative barrier: no
/// cell footprint bounds what a command substitution does.
pub(super) fn lowered_footprint_intents(
    footprint: &LoweredFootprint,
) -> Vec<(WorldStateIntentKind, WorldRegion)> {
    if footprint.runs_commands {
        return conservative_region_intents();
    }
    let mut intents = cell_intents(footprint.reads.iter(), WorldStateIntentKind::Use);
    if footprint.reads_unbounded {
        intents.push((
            WorldStateIntentKind::Use,
            WorldRegion::domain_wildcard(
                WorldRegionKind::VariableStore,
                WorldInterpreterScope::Current,
                WorldNamespaceScope::Current,
            ),
        ));
    }
    intents.extend(cell_intents(
        footprint.writes.iter(),
        WorldStateIntentKind::Clobber,
    ));
    intents
}

/// Compatibility projection for an explicitly carrierless hand-built function.
/// Source-built functions use `executable_expr_intents_with_source` and never
/// use these flattened text heuristics to establish substitution absence.
fn executable_expr_intents(expr: &ExecutableExpr) -> Vec<(WorldStateIntentKind, WorldRegion)> {
    if let ExecutableExpr::TrapPrefix { .. } = expr {
        return Vec::new();
    }
    if let ExecutableExpr::Operand { braced: true, .. } = expr {
        return Vec::new();
    }
    if let ExecutableExpr::Condition {
        expr: condition, ..
    } = expr
    {
        if condition.command_texts().is_empty() {
            return known_noncommand_expr_intents(expr);
        }
    } else if let ExecutableExpr::Operand { text, .. } = expr
        && !text.contains('[')
    {
        return known_noncommand_expr_intents(expr);
    }
    conservative_region_intents()
}

fn known_noncommand_expr_intents(
    expr: &ExecutableExpr,
) -> Vec<(WorldStateIntentKind, WorldRegion)> {
    let variable_store = WorldRegion::domain_wildcard(
        WorldRegionKind::VariableStore,
        WorldInterpreterScope::Current,
        WorldNamespaceScope::Current,
    );
    match expr {
        ExecutableExpr::Condition { expr, .. } => {
            let _ = expr;
            vec![(WorldStateIntentKind::Use, variable_store)]
        }
        ExecutableExpr::Operand { braced, .. } => {
            if *braced {
                Vec::new()
            } else {
                vec![(WorldStateIntentKind::Use, variable_store)]
            }
        }
        // Reading the joined completion's options dictionary touches no cell.
        ExecutableExpr::TrapPrefix { .. } => Vec::new(),
    }
}

pub(super) fn executable_expr_intents_with_source(
    function: &ExecutableFunction,
    value: crate::executable_ir::ExecutableValueId,
    expression: &ExecutableExpr,
) -> Vec<(WorldStateIntentKind, WorldRegion)> {
    match function.original_expression_command_effects(value, expression) {
        None => executable_expr_intents(expression),
        Some(
            crate::executable_ir::OriginalCommandEffect::Known(true)
            | crate::executable_ir::OriginalCommandEffect::Unavailable,
        ) => conservative_region_intents(),
        Some(crate::executable_ir::OriginalCommandEffect::Known(false)) => {
            known_noncommand_expr_intents(expression)
        }
    }
}

pub(super) fn conservative_region_intents() -> Vec<(WorldStateIntentKind, WorldRegion)> {
    vec![
        (WorldStateIntentKind::Use, WorldRegion::Any),
        (WorldStateIntentKind::Clobber, WorldRegion::Any),
    ]
}

// Actual executable-value membership selects retained source geometry. Only
// explicit carrierless standalone models use the compatibility word shape.
pub(super) fn original_word_evaluation_runs_commands(
    function: &ExecutableFunction,
    value: crate::executable_ir::ExecutableValueId,
    word: &tcl_source::words::WordExpr,
) -> bool {
    function
        .original_word_command_effects(value, word)
        .map_or_else(
            || word_evaluation_runs_commands(word),
            |source| source.into_known().unwrap_or(true),
        )
}

/// Whether evaluating one source word may execute nested commands.
///
/// Variable substitutions are deliberately excluded: a variable read can
/// only run code through a live variable trace, which the contents analysis
/// ([`crate::dispatch_proof`]) discharges or fails closed on explicitly.
pub(super) fn word_evaluation_runs_commands(word: &tcl_source::words::WordExpr) -> bool {
    use tcl_source::words::WordExpr;
    use tcl_source::words::WordPart;
    match word {
        WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. } | WordExpr::Variable { .. } => {
            false
        }
        WordExpr::CommandSubstitution { .. } | WordExpr::Opaque { .. } => true,
        WordExpr::Template { parts, .. } => parts.iter().any(|part| {
            matches!(
                part,
                WordPart::CommandSubstitution { .. } | WordPart::Opaque { .. }
            )
        }),
        WordExpr::Expand { word, .. } => word_evaluation_runs_commands(word),
    }
}
