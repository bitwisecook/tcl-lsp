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
use tcl_compiler::analyser::Analyser;

fn at(source: &str, analysis: &AnalysisResult, offset: usize) -> Vec<CompletionItem> {
    let profile = analysis.resolved_profile().unwrap();
    let position =
        tcl_lexer::LineIndex::new(source).position_at_utf16(u32::try_from(offset).unwrap(), source);
    completions(
        source,
        position.line,
        position.character.get(),
        analysis,
        analysis.resolved_registry(),
        None,
        profile,
    )
}

#[test]
fn original_snippet_variable_choices_keep_source_references_after_ui_erasure() {
    // naming.core.original-snippet-source-context
    // docs/design/analysis/name-resolution-proofs/original-snippet-source-context.md
    let source = "set {café🙂} LIST\nset v\\uD800 OTHER\ntcl-foreach";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    analysis.global_scope.variables.clear();
    analysis.all_variables.clear();
    let candidates = at(source, &analysis, source.len());
    let snippet = candidates
        .iter()
        .find(|item| item.label == "Foreach")
        .unwrap();
    assert!(
        snippet.insert_text.contains("${café🙂}"),
        "{}",
        snippet.insert_text
    );
    assert!(!snippet.insert_text.contains("D800"));
    assert_eq!(
        at(&format!("#{source}"), &analysis, source.len() + 1),
        [] as [crate::completion::CompletionItem; 0]
    );
}

#[test]
fn logical_snippet_choices_use_current_scope_and_selected_source_reference_grammar() {
    let source = "set {two words} LIST\nset {a{b}c} LIST\nset a_\\}_closebrace LIST\nproc p {} {\nset local LIST\ntcl-foreach\n}\n";
    let cursor = source.find("tcl-foreach").unwrap() + "tcl-foreach".len();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let selected = tcl_registry::model::ingress::resolve_environment(environment);
        let config = tcl_lexer::LexerConfig::for_profile(Some(selected.analyser_profile()));
        let analysis = Analyser::new()
            .with_resolved_input(tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                selected.default_context_registry(),
                config,
            ))
            .analyse(source, "display only");
        let candidates = at(source, &analysis, cursor);
        let snippet = candidates
            .iter()
            .find(|item| item.label == "Foreach")
            .unwrap();
        assert!(
            snippet.insert_text.contains("$local"),
            "{environment}: {}",
            snippet.insert_text
        );
        assert!(
            snippet.insert_text.contains("${::two words}"),
            "{environment}: {}",
            snippet.insert_text
        );
        assert!(
            !snippet.insert_text.contains("closebrace"),
            "{environment}: {}",
            snippet.insert_text
        );
        assert_eq!(
            snippet.insert_text.contains("${::a{b}c}"),
            config.braced_var == tcl_dialect::BracedVarStyle::Tcl9Nesting,
            "{environment}: {}",
            snippet.insert_text,
        );
        assert_eq!(
            at(&format!("#{source}"), &analysis, cursor + 1),
            [] as [crate::completion::CompletionItem; 0]
        );
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        let position = tcl_lexer::LineIndex::new(source)
            .position_at_utf16(u32::try_from(cursor).unwrap(), source);
        assert_eq!(
            completions(
                source,
                position.line,
                position.character.get(),
                &missing,
                None,
                None,
                profile
            ),
            [] as [crate::completion::CompletionItem; 0]
        );
        let mut changed = analysis.clone();
        changed.body_lexer_config.as_mut().unwrap().strict_quoting = !config.strict_quoting;
        assert_eq!(
            at(source, &changed, cursor),
            [] as [crate::completion::CompletionItem; 0]
        );
    }
}

#[test]
fn original_snippet_events_keep_retained_cards_and_actual_script_placement() {
    // naming.core.original-snippet-source-context
    // docs/design/analysis/name-resolution-proofs/original-snippet-source-context.md
    let source = "when RULE_INIT {\n  irule-http-request\n}\nirule-http-request";
    let mut analysis = Analyser::new().analyse(source, "f5-irules");
    assert!(
        analysis
            .original_vendor_symbol_declarations()
            .any(|row| row.metadata().kind == tcl_registry::DefinedSymbolKind::Event)
    );
    analysis.all_defined_symbols.clear();
    analysis.global_scope.defined_symbols.clear();
    let inside = source.find("irule-http-request").unwrap() + "irule-http-request".len();
    let outside = source.len();
    assert!(
        !at(source, &analysis, inside)
            .iter()
            .any(|item| item.label == "iRule HTTP_REQUEST")
    );
    assert!(
        at(source, &analysis, outside)
            .iter()
            .any(|item| item.label == "iRule HTTP_REQUEST")
    );
    let facts = crate::irules_context::EventHandlerFacts::from_analysis(source, &analysis).unwrap();
    assert_eq!(facts.file_events(), ["RULE_INIT"]);
    assert!(
        crate::irules_context::EventHandlerFacts::from_analysis(&format!("#{source}"), &analysis)
            .is_none()
    );
    analysis.body_lexer_config = analysis
        .body_lexer_config
        .map(|config| tcl_lexer::LexerConfig {
            expand_syntax: !config.expand_syntax,
            ..config
        });
    assert!(crate::irules_context::EventHandlerFacts::from_analysis(source, &analysis).is_none());
}
