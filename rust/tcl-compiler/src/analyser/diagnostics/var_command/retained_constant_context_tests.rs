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
use std::sync::Arc;

#[test]
fn array_constant_harvest_uses_retained_operand_grammar_over_catalogue_profile() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Original source-value advice; no executed store or Native argv claim.
    let source = r#"array set state "callback \U0001F600""#;
    let catalogue = tcl_registry::model::ingress::static_context_for("tcl9.0");
    for (environment, expected) in [("tcl8.4", "U0001F600"), ("tcl9.0", "\u{1F600}")] {
        let profile = tcl_dialect::DialectProfile::find(environment).unwrap();
        let context = Arc::new(
            tcl_registry::model::ingress::static_context_for(environment)
                .with_command_store(Arc::clone(catalogue.commands())),
        );
        let dialect = tcl_registry::InvocationDialect::of_profile(profile);
        // The source-authoring entry supplies its explicit dialect below.
        // A Driver entry would require independent Native operand/handler
        // premises and would test a different admission boundary.
        let cu = crate::compilation_unit::CompilationUnit::build_with_context_registry(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry: context.commands(),
                defer_top_level: false,
                config: tcl_lexer::LexerConfig::for_profile(Some(profile)),
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            Arc::clone(&context),
        );
        let selected: Vec<_> = cu
            .top_level
            .cfg
            .blocks
            .values()
            .flat_map(|block| &block.statements)
            .filter_map(|statement| {
                crate::registry_invocation::resolved_statement_invocation_in_context(
                    &context, statement,
                )
            })
            .collect();
        assert_eq!(
            selected.len(),
            1,
            "positive original metadata: {environment}"
        );
        assert_eq!(selected[0].dialect, Some(dialect));
        assert!(matches!(
            selected[0].facts.var_elements_effect,
            Some(tcl_registry::VarElementsEffect::SetsArrayElementsFromList { .. })
        ));
        let mut values = HashMap::new();
        harvest_array_set_constants(&cu, &mut values, &context);
        assert_eq!(
            values.get("state(callback)"),
            Some(&HashSet::from([expected.to_owned()])),
            "retained source grammar: {environment}"
        );
    }
}
