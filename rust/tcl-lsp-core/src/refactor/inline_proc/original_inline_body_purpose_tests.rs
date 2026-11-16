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
fn logical_inline_body_uses_retained_word_extent_for_literal_closers() {
    let source = "proc helper {value} {puts \"literal \\} ${value}\"}\nhelper TEXT";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl");
    let procedure = &analysis.all_procs["::helper"];
    let config = analysis.body_lexer_config.unwrap();
    let body = single_command_body(source, procedure, &analysis, config).unwrap();
    assert_eq!(body.text, "puts \"literal \\} ${value}\"");
    assert_eq!(
        source.get(body.command.span.as_range()),
        Some(body.text.as_str())
    );
    let mut stale = analysis.clone();
    stale.all_procs.get_mut("::helper").unwrap().params.clear();
    assert!(single_command_body(source, &stale.all_procs["::helper"], &stale, config).is_err());
}

#[test]
fn original_inline_body_keeps_bom_head_and_retained_parent_correspondence() {
    // naming.refactor.original-procedure-inline-binding
    // docs/design/analysis/name-resolution-proofs/refactor-original-procedure-inline-binding.md
    // Original body syntax/geometry only; no complete inline edit permission.
    let source = "\u{feff}proc p {x} {\u{feff}#prefix $x}\np VALUE";
    let mut prepared = tcl_compiler::analyser::Analyser::new();
    let base = prepared.prepare_analysis_input("tcl9.1").unwrap();
    let config = LexerConfig {
        leading_bom: tcl_lexer::LeadingBom::Skip,
        braced_var: BracedVarStyle::FirstClose,
        strict_quoting: true,
        ..base.lexer_config()
    };
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        base.analyser_profile(),
        base.unit_profile(),
        base.context_registry(),
        config,
    );
    let analysis = tcl_compiler::analyser::Analyser::new()
        .with_resolved_input(input.clone())
        .analyse(source, "tcl9.1");
    assert!(!analysis.allows_lexical_declaration_advice());
    let procedure = analysis
        .all_procs
        .get("::p")
        .expect("actual original procedure header");
    let body = single_command_body(source, procedure, &analysis, config).unwrap();
    assert_eq!(body.command.name(), "\u{feff}#prefix");
    assert_eq!(
        body.origin,
        u32::try_from(source.find("\u{feff}#prefix").unwrap()).unwrap()
    );
    assert_eq!(
        source.get(body.command.argv[0].span.as_range()),
        Some("\u{feff}#prefix")
    );
    assert_eq!(body.text, "\u{feff}#prefix $x");
    assert!(
        single_command_body("different original document", procedure, &analysis, config).is_err()
    );
    assert!(single_command_body(source, procedure, &analysis, config.nested()).is_err());
    assert_eq!(analysis.resolved_input.as_ref(), Some(&input));
}
