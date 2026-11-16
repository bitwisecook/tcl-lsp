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
fn original_definition_suffix_keeps_bom_definer_and_authored_allocation() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // Retained authored declaration navigation; no completed call or editable definition grant.
    let mut prepared = crate::analyser::Analyser::new();
    let base = prepared.prepare_analysis_input("tcl9.1").unwrap();
    let config = tcl_lexer::LexerConfig {
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
    let source = "\u{feff}rename proc {\u{feff}Maker}\n\u{feff}Maker p {} {return BODY}\np";
    let mut result = crate::analyser::Analyser::new()
        .with_resolved_input(input.clone())
        .analyse(source, "presentation");
    let offset = u32::try_from(source.rfind('p').unwrap()).unwrap();
    let definition = result
        .command_invocations
        .iter()
        .find(|call| call.range.start() == offset)
        .unwrap()
        .resolved_definition
        .clone()
        .unwrap();
    let declaration = result
        .original_procedure_declarations()
        .find(|record| record.metadata().name == "p")
        .unwrap()
        .metadata()
        .name_span;
    let tokens = result.definition_tokens(&definition, source).unwrap();
    assert!(tokens.contains(&declaration));
    assert_eq!(
        definition.allocation().site.offset,
        u32::try_from(source.find("\u{feff}Maker p").unwrap()).unwrap()
    );
    assert_eq!(result.resolved_input.as_ref(), Some(&input));
    assert_eq!(result.body_lexer_config, Some(config));
    result.original_procedure_metadata.clear();
    assert_eq!(
        result
            .proc_for_definition(&definition, source)
            .unwrap()
            .name_span,
        declaration
    );
    assert!(
        result
            .definition_tokens(&definition, &source.replace("BODY", "OTHER"))
            .is_none()
    );
    result.body_lexer_config = None;
    assert!(result.definition_tokens(&definition, source).is_none());
}
