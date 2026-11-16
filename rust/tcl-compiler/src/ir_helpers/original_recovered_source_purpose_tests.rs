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
fn original_recovered_word_keeps_bom_and_supplied_input_through_nested_inventory() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Conditional lexical inventory; no original child execution or Normal handler grant.
    let mut prepared = crate::analyser::Analyser::new();
    let base = prepared.prepare_analysis_input("tcl9.1").unwrap();
    let config = LexerConfig {
        leading_bom: tcl_lexer::LeadingBom::Skip,
        braced_var: tcl_lexer::BracedVarStyle::FirstClose,
        strict_quoting: true,
        ..base.lexer_config()
    };
    let input = crate::analyser::ResolvedAnalysisInput::new(
        base.analyser_profile(),
        base.unit_profile(),
        base.context_registry(),
        config,
    );
    let context = input.context_registry();
    let registry = context.commands();
    let text = "\u{feff}#prefix ${a{b}";
    let words = tokenise_command_words(text, config);
    assert_eq!(words.len(), 1);
    assert_eq!(words[0][0].literal(), Some("\u{feff}#prefix"));
    assert_eq!(words[0][1].raw, "${a{b}");
    assert!(words[0][1].substituted);
    assert!(tokenise_command_words("#prefix ${a{b}", config).is_empty());
    let source = "\u{feff}puts \u{feff}#prefix[list ${a{b}]";
    let mut lowerer = crate::lowering::Lowerer::with_config(registry, config)
        .with_resolved_analysis_input(input.clone());
    lowerer.lower(source);
    let module = lowerer.finish_module(source);
    let statement = &module.top_level.statements[0];
    let metadata =
        crate::registry_invocation::InvocationMetadataContext::for_analysis_input(registry, &input)
            .unwrap();
    let inventory = evaluated_command_substitutions_with_metadata_context(
        statement,
        registry,
        Some(metadata),
        config,
    );
    assert_eq!(inventory.commands.len(), 1);
    assert_eq!(inventory.commands[0][0].literal(), Some("list"));
    assert_eq!(inventory.commands[0][1].raw, "${a{b}");
    assert!(!inventory.opaque);
    assert!(
        evaluated_command_substitutions_with_metadata_context(statement, registry, None, config)
            .opaque
    );
    assert!(
        evaluated_command_substitutions_with_metadata_context(
            statement,
            registry,
            Some(metadata),
            config.nested()
        )
        .opaque
    );
    let foreign = tcl_registry::model::ingress::static_context_for("tcl8.4");
    assert!(
        evaluated_command_substitutions_with_metadata_context(
            statement,
            registry,
            Some(foreign.into()),
            config
        )
        .opaque
    );
    assert_eq!(module.source_metadata_input.as_ref(), Some(&input));
    assert_eq!(module.lexer_config, config);
}
