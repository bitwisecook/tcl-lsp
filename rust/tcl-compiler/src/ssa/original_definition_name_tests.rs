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
// Implementation contract: naming.variable.original-ssa-definition-operand
// docs/design/analysis/name-resolution-proofs/original-ssa-definition-operand.md
fn original_ssa_definition_names_preserve_exact_operands_and_distinct_cells() {
    let source = r"set v\uD800 42; set v\uD801 TEXT; set a(k) 7";
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
        source, registry, false, "tcl8.6",
    );
    let function = &unit.top_level;
    let specialised = function
        .ssa
        .blocks
        .iter()
        .find_map(|(&block, body)| {
            body.statements
                .iter()
                .enumerate()
                .find_map(|(index, statement)| {
                    (statement.statement.span().start() == 0).then_some((
                        block,
                        index,
                        &statement.statement,
                    ))
                })
        })
        .expect("first original assignment");
    assert!(matches!(specialised.2, Statement::AssignConst { .. }));
    assert!(specialised.2.tokens().is_none());
    assert!(
        function
            .ssa
            .point_contexts
            .as_ref()
            .expect("source points")
            .source_tokens_at(specialised.0, specialised.1)
            .is_some()
    );
    let mut definitions = Vec::new();
    for (&block, body) in &function.ssa.blocks {
        for (index, statement) in body.statements.iter().enumerate() {
            let view = SsaSourceView::at_statement(&function.ssa, block, index);
            for &symbol in statement.defs.keys() {
                if let Some(definition) = view.original_definition_name(symbol, registry) {
                    assert_eq!(Some(definition.cell()), function.ssa.cell_key(symbol));
                    definitions.push(definition);
                }
            }
        }
    }
    let select = |spelling: &str| {
        definitions
            .iter()
            .find(|definition| &source[definition.span().as_range()] == spelling)
            .unwrap_or_else(|| {
                panic!(
                    "missing original definition for {spelling}; count={}",
                    definitions.len()
                )
            })
    };
    let first = select(r"v\uD800");
    let second = select(r"v\uD801");
    assert_ne!(
        first.original_name_input().bytes(),
        second.original_name_input().bytes()
    );
    assert_ne!(first.cell(), second.cell());
    assert_eq!(select("a(k)").original_name_input().bytes(), b"a(k)");
}
