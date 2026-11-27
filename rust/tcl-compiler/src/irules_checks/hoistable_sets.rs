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

//! IRULE4004: a per-request literal assignment that could run once per connection.

use super::{IrulesCheckPayload, IrulesCheckWarning, IrulesSourceQuery, is_irules};
use crate::compilation_unit::CompilationUnit;
use crate::ir::Statement;
use std::collections::HashMap;
use tcl_core_types::DiagCode;
use tcl_dialect::model::SurfaceQuery;
use tcl_registry::CommandRegistry;
use tcl_registry::events::EventRegistry;

// IRULE4004 — `set var value` in per-request event hoistable to once-per-connection
//
// The event registry classifies per-request and once-per-connection events.
// An `AssignConst` whose value does not depend on per-request data could be
// hoisted to a once-per-connection event, saving work per request.
//
// A warning is only safe when the literal scalar assignment is unconditional
// and is the variable's sole write across the rule. Repeated initialisation is
// commonly a deliberate per-request reset; moving it changes connection-state
// semantics after the first request. Array elements and namespace/static
// variables likewise need a different destination and are excluded.

/// Hoistable-set warnings for IRULE4004.
#[must_use]
pub fn find_hoistable_set_warnings(
    cu: &CompilationUnit,
    dialect: Option<SurfaceQuery<'_>>,
) -> Vec<IrulesCheckWarning> {
    let mut out = Vec::new();
    if !is_irules(dialect) {
        return out;
    }
    let events = EventRegistry::build();
    let registry = cu.ir_module.resolved_registry();
    let Some(source) = IrulesSourceQuery::capture(cu, registry) else {
        return out;
    };
    let Some(write_counts) = event_mutation_counts(&source) else {
        return out;
    };
    for fu in cu.analysable_functions() {
        let Some(event) = source.event(fu) else {
            return Vec::new();
        };
        let tcl_core_types::Knowledge::Proven(event) = event else {
            continue;
        };
        if !events.is_per_request(event.body.event()) {
            continue;
        }
        let Some(block) = fu.cfg.blocks.get(&fu.cfg.entry) else {
            continue;
        };
        let contexts = crate::place_bridge::build_point_resolve_contexts_with_entry(
            &fu.cfg,
            event.body.conditional_entry(),
            registry,
        );
        for (index, stmt) in block.statements.iter().enumerate() {
            let Some((name, value)) = hoistable_literal_assignment(stmt, registry) else {
                continue;
            };
            let places = crate::place_bridge::def_places_with_continuation(
                stmt,
                contexts.before_statement(fu.cfg.entry, index),
                contexts.after_statement(fu.cfg.entry, index),
                registry,
            );
            let [place] = places.as_slice() else {
                continue;
            };
            let Some(cell) = crate::connection_scope::cell_from_place(place) else {
                continue;
            };
            if !cell.is_connection() {
                continue;
            }
            if name.is_empty()
                || value.is_empty()
                || place.observed
                || place.kind != tcl_names::place::PlaceKind::Scalar
                || write_counts.get(&cell) != Some(&1)
            {
                continue;
            }
            let span = stmt.span();
            out.push(IrulesCheckWarning {
                span: fu.abs_span(span),
                code: DiagCode::Irule4004,
                message: format!(
                    "`set {name} ...` runs on every request — consider hoisting to a once-per-connection event.",
                ),
                payload: IrulesCheckPayload::HoistableAssignment {
                    variable: name.clone(),
                },
                replacement: None,
                fixes: Vec::new(),
            });
        }
    }
    out
}

/// Count physical mutations rather than displayed operands. An unbounded
/// mutation cannot prove that another event leaves a proposed destination alone.
pub(super) fn event_mutation_counts(
    source: &IrulesSourceQuery<'_>,
) -> Option<HashMap<crate::connection_scope::EventCell, usize>> {
    let mut counts = HashMap::new();
    let registry = source.registry;
    for fu in source.unit.functions() {
        let tcl_core_types::Knowledge::Proven(event) = source.event(fu)? else {
            continue;
        };
        let contexts = crate::place_bridge::build_point_resolve_contexts_with_entry(
            &fu.cfg,
            event.body.conditional_entry(),
            registry,
        );
        for (&id, block) in &fu.cfg.blocks {
            for (index, stmt) in block.statements.iter().enumerate() {
                for place in crate::place_bridge::statement_mutation_places_with_continuation(
                    stmt,
                    contexts.before_statement(id, index),
                    contexts.after_statement(id, index),
                    registry,
                ) {
                    let cell = crate::connection_scope::cell_from_place(&place)?;
                    *counts.entry(cell).or_insert(0) += 1;
                }
            }
        }
    }
    Some(counts)
}

fn hoistable_literal_assignment(
    statement: &Statement,
    registry: &CommandRegistry,
) -> Option<(String, String)> {
    if let Some(tokens) = statement.tokens() {
        let normal =
            crate::registry_invocation::normal_representation_invocation(registry, None, tokens)?;
        let assignment = normal.value_assignment()?;
        let dialect = tokens
            .source_binding
            .as_ref()?
            .variable_context
            .invocation_dialect?;
        let value = crate::registry_invocation::effective_invocation_word(
            &assignment.value,
            dialect.lexer_grammar.escapes,
            dialect.word_values,
        );
        return match value {
            crate::registry_invocation::EffectiveInvocationWord::Literal(value) => {
                Some((assignment.name, value))
            }
            _ => None,
        };
    }
    match statement {
        Statement::AssignConst { name, value, .. } => Some((name.clone(), value.clone())),
        _ => None,
    }
}
