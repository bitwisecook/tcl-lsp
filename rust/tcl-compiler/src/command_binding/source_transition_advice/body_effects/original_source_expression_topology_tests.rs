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

use crate::analyser::ResolvedAnalysisInput;
use crate::registry_invocation::source_structure::{
    OriginalRegistrySource, OriginalSourceRegistryContext,
};

fn input() -> ResolvedAnalysisInput {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context =
        crate::environment_ingress::resolve_environment("tcl8.6").default_context_registry();
    let config = tcl_lexer::LexerConfig {
        leading_bom: tcl_lexer::LeadingBom::Skip,
        strict_quoting: true,
        braced_var: tcl_lexer::BracedVarStyle::FirstClose,
        ..tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
    };
    ResolvedAnalysisInput::new(profile, profile, context, config)
}

#[test]
fn original_logical_expression_child_retains_alias_actual_base_and_no_native_recipe() {
    // naming.source.authored-command-transition-advice
    // docs/design/analysis/name-resolution-proofs/authored-command-transition-advice.md
    // Original conditional source schema only; no entered command or runtime output.
    let input = input();
    assert!(input.has_logical_source_name_context());
    let source = "\u{feff}interp alias {} \u{feff}match {} regexp\nif {![\u{feff}match λ λ out]} {puts $out}";
    let syntax = OriginalSourceRegistryContext::capture(source, input.clone());
    let start = u32::try_from(source.find("\u{feff}match λ").unwrap()).unwrap();
    let end = u32::try_from(source.find("]}").unwrap()).unwrap();
    let children = tcl_source::segment::segment_commands_with_offset_and_config(
        &source[start as usize..end as usize],
        start,
        input.lexer_config().nested(),
    );
    let [child] = children.as_slice() else {
        panic!("complete single original command child")
    };
    let words = syntax.words(child).unwrap();
    assert_eq!(words.command(), "regexp");
    let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
        panic!("original transition advice")
    };
    assert_eq!(advice.logical_source_input(), Some(&input));
    assert!(advice.original_head().native_input().is_none());
    assert_eq!(advice.original_head().bytes(), "\u{feff}match".as_bytes());
    assert_eq!(advice.original_words()[0].span().start(), start);
    assert!(
        advice
            .original_words()
            .iter()
            .all(|word| word.config() == input.lexer_config()
                && word.image().bytes() == source.as_bytes())
    );
    let foreign_source = format!("{source} ");
    let foreign = tcl_lexer::SourceImage::document(&foreign_source);
    assert!(!advice.matches_source(&foreign, input.lexer_config()));
    let mut changed = input.lexer_config();
    changed.strict_quoting = !changed.strict_quoting;
    assert!(!advice.matches_source(&tcl_lexer::SourceImage::document(source), changed));
    let shifted = tcl_source::segment::segment_commands_with_offset_and_config(
        &source[start as usize..end as usize],
        start + 1,
        input.lexer_config().nested(),
    );
    assert!(syntax.words(&shifted[0]).is_none());
}
