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
fn original_isolated_body_projects_its_genuine_parent_input_to_nested_source() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // An actual deferred source body; no entered procedure or frame claim.
    let environment = crate::environment_ingress::resolve_environment("tcl");
    let config = tcl_lexer::LexerConfig {
        leading_bom: tcl_lexer::LeadingBom::Skip,
        ..tcl_lexer::LexerConfig::for_file_grammar(environment.grammar())
    };
    let input = super::super::input::ResolvedAnalysisInput::new(
        environment.analyser_profile(),
        environment.unit_profile(),
        environment.default_context_registry(),
        config,
    );
    let source = "\u{feff}proc p {} {\u{feff}set x 1}";
    let mut shell = Analyser::new().with_resolved_input(input.clone());
    let commands = shell.per_item_setup(source, "tcl");
    shell.defer_proc_bodies = true;
    shell.walk_commands_top_level(&commands, false);
    shell.defer_proc_bodies = false;
    assert_eq!(shell.deferred_bodies.len(), 1);
    let body = shell.deferred_bodies.pop().unwrap();
    assert_eq!(body.resolved_input.as_ref(), Some(&input));
    assert_eq!(body.body_text.as_ref(), "\u{feff}set x 1");
    let fragment = analyse_proc_body_isolated(
        &body,
        "tcl9.1",
        &std::collections::HashSet::<String>::new(),
        super::super::state::NonAsciiMode::Off,
        None,
        None,
        None,
    );
    let nested = input.for_nested_source();
    assert_eq!(fragment.result.resolved_input.as_ref(), Some(&nested));
    assert_eq!(
        fragment.result.body_lexer_config,
        Some(nested.lexer_config())
    );
    assert_eq!(
        nested.lexer_config().leading_bom,
        tcl_lexer::LeadingBom::Content
    );
    assert!(
        fragment
            .result
            .command_invocations
            .iter()
            .any(|invocation| invocation.name == "\u{feff}set")
    );
}

#[test]
fn original_per_item_entry_uses_retained_configuration_and_unknown_is_distinct() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // Source-only fast-path control; no native entry or completion grant.
    let environment = crate::environment_ingress::resolve_environment("tcl9.1");
    let source = "set x ${a{}";
    let config = tcl_lexer::LexerConfig {
        braced_var: tcl_lexer::BracedVarStyle::FirstClose,
        ..tcl_lexer::LexerConfig::for_file_grammar(environment.grammar())
    };
    let input = super::super::input::ResolvedAnalysisInput::new(
        environment.analyser_profile(),
        environment.unit_profile(),
        environment.default_context_registry(),
        config,
    );
    let mut selected = Analyser::new().with_resolved_input(input.clone());
    let result = selected.analyse_per_item(source, "tcl9.1");
    assert_ne!(
        selected.per_item_fallback,
        Some(PerItemFallback::IncompleteScript)
    );
    assert_eq!(result.resolved_input.as_ref(), Some(&input));
    let mut balanced = Analyser::new();
    balanced.analyse_per_item(source, "tcl9.1");
    assert_eq!(
        balanced.per_item_fallback,
        Some(PerItemFallback::IncompleteScript)
    );
    let jim = crate::environment_ingress::resolve_environment("jim");
    let foreign = tcl_lexer::LexerConfig {
        braced_var: tcl_lexer::BracedVarStyle::Tcl9Nesting,
        ..tcl_lexer::LexerConfig::for_file_grammar(jim.grammar())
    };
    let input = super::super::input::ResolvedAnalysisInput::new(
        jim.analyser_profile(),
        jim.unit_profile(),
        jim.default_context_registry(),
        foreign,
    );
    let mut unknown = Analyser::new().with_resolved_input(input);
    unknown.analyse_per_item("set x 1", "jim");
    assert_eq!(
        unknown.per_item_fallback,
        Some(PerItemFallback::CompletenessUnavailable)
    );
    assert_eq!(
        PerItemFallback::CompletenessUnavailable.as_str(),
        "completeness-unavailable"
    );
}
