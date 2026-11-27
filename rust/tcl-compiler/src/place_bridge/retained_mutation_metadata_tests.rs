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

#[test]
fn destruction_reborrows_the_actual_cfg_metadata_owner() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for(
        "proc p {} {set x OLD; unset x; set x NEW}",
        registry,
        false,
    );
    let function = unit.function("::p").unwrap();
    let points = function.ssa.point_contexts.as_ref().unwrap();
    let (block, index, statement) = function
        .cfg
        .blocks
        .iter()
        .find_map(|(&block, body)| {
            body.statements
                .iter()
                .enumerate()
                .find_map(|(index, statement)| {
                    let selected = super::statement_destruction_places_at(
                        statement, block, index, points, registry,
                    );
                    (!selected.is_empty()).then_some((block, index, statement))
                })
        })
        .unwrap();
    assert_ne!(
        super::statement_destruction_places_at(statement, block, index, points, registry),
        [] as [tcl_names::place::Place; 0]
    );
    assert_eq!(
        super::statement_destruction_places_with_continuation_with_metadata_context(
            statement,
            points.before_statement(block, index),
            points.after_statement(block, index),
            registry,
            None,
        ),
        [] as [tcl_names::place::Place; 0]
    );
}
