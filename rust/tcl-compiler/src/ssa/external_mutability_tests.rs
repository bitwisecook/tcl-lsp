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

use super::*;

#[test]
fn named_external_dependencies_use_actual_point_bindings_and_ignore_display_labels() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for(
        "proc p {} {set dictionary [dict create k 1]; upvar 0 dictionary view; set dictionary [dict create k 2]}",
        registry,
        false,
    );
    let mut ssa = unit.function("::p").unwrap().ssa.clone();
    let mut writes = Vec::new();
    for (&block, body) in &ssa.blocks {
        for (index, statement) in body.statements.iter().enumerate() {
            if matches!(&statement.statement, Statement::AssignValue { name, .. } if name == "dictionary")
            {
                writes.push((
                    statement.statement.span().start(),
                    block,
                    index,
                    *statement.defs.keys().next().unwrap(),
                ));
            }
        }
    }
    writes.sort_by_key(|(offset, ..)| *offset);
    assert_eq!(writes.len(), 2);
    assert_eq!(writes[0].3, writes[1].3);
    let symbol = writes[0].3;
    let names = HashSet::from(["view".to_owned()]);
    for label in ["view", "dictionary", "unrelated"] {
        ssa.var_names[symbol.0 as usize] = label.to_owned();
        for (point, expected) in [(writes[0], false), (writes[1], true)] {
            let source = SsaSourceView::at_statement(&ssa, point.1, point.2);
            assert_eq!(
                source.externally_mutable_by(symbol, &names, false, registry),
                Some(expected),
                "{label}; canonical={}; before={:?}; carrier={:?}",
                ssa.cell_name(symbol).expect("original location symbol"),
                ssa.point_contexts
                    .as_ref()
                    .and_then(|points| points.context_before(point.1, point.2))
                    .map(|context| canonical_variable_key("view", context, registry)),
                source
                    .source_tokens()
                    .and_then(|tokens| tokens.source_binding.as_ref())
                    .map(|binding| canonical_variable_key(
                        "view",
                        &binding.variable_context,
                        registry
                    )),
            );
            assert_eq!(
                source.externally_mutable_by(symbol, &names, true, registry),
                Some(true)
            );
            assert_eq!(
                source.externally_mutable_by(
                    symbol,
                    &HashSet::from([ssa
                        .cell_name(symbol)
                        .expect("original location symbol")
                        .to_owned()]),
                    false,
                    registry
                ),
                Some(false)
            );
        }
    }
    assert_eq!(
        SsaSourceView::unpositioned(&ssa).externally_mutable_by(symbol, &names, false, registry),
        None
    );
    assert_eq!(
        SsaSourceView::unpositioned(&ssa).externally_mutable_by(
            symbol,
            &HashSet::new(),
            false,
            registry
        ),
        Some(false)
    );
}
