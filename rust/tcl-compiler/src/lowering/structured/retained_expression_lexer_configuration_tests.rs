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

use crate::expr_ast::ExprNode;
use crate::ir::Statement;

#[test]
fn structured_conditions_preserve_the_actual_braced_variable_override() {
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let context = tcl_registry::model::ingress::context_for_profile(profile);
    let baseline = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    for (config, variable) in [
        (baseline, true),
        (
            tcl_lexer::LexerConfig {
                braced_var: tcl_dialect::BracedVarStyle::FirstClose,
                ..baseline
            },
            false,
        ),
    ] {
        for source in [
            "if {${a{b}c}} {}",
            "while {${a{b}c}} {}",
            "for {} {${a{b}c}} {} {}",
        ] {
            let module = crate::lowering::lower_to_ir_with_dialect(
                source,
                context.commands(),
                config,
                Some(profile),
            );
            let condition = match &module.top_level.statements[0] {
                Statement::If { clauses, .. } => &clauses[0].condition,
                Statement::For { condition, .. } | Statement::While { condition, .. } => condition,
                other => panic!("selected structured source: {other:?}"),
            };
            assert_eq!(
                matches!(condition, ExprNode::Var { .. }),
                variable,
                "{source}"
            );
            assert_eq!(
                matches!(condition, ExprNode::Raw { .. }),
                !variable,
                "{source}"
            );
        }
    }
}
