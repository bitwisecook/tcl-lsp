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
fn original_source_schema_uses_retained_word_dialect_for_file_grammar() {
    // naming.core.original-document-link-selection
    // docs/design/analysis/name-resolution-proofs/original-document-link-selection.md
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0", "jim"] {
        let source = "source {a $b [c].tcl}";
        let analysis = crate::analyser::Analyser::new().analyse(source, dialect);
        let config = analysis.body_lexer_config.unwrap();
        let command =
            tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config)
                .remove(0);
        let words = source_registry_words(source, &analysis, &command).expect(dialect);
        let grammar = words
            .dialect
            .and_then(tcl_registry::InvocationDialect::source_file_grammar)
            .expect(dialect);
        assert!(
            matches!(
                grammar.select_original(
                    words.arguments.len(),
                    words
                        .arguments
                        .first()
                        .and_then(|word| word.literal_bytes())
                ),
                tcl_registry::source_file::SourceFileSelection::Selected(_)
            ),
            "{dialect}"
        );
        assert_eq!(
            words.arguments[0].literal_bytes(),
            Some(b"a $b [c].tcl".as_slice())
        );
        assert!(
            words.operands[0].as_ref().unwrap().input().is_some(),
            "{dialect}"
        );
        assert!(source_registry_words(&format!("#{source}"), &analysis, &command).is_none());
    }
}
#[test]
fn original_package_source_point_keeps_whole_image_context_and_alias_grammar() {
    // naming.diagnostic.original-package-source-advice
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-package-source-advice.md
    let source = "interp alias {} need {} package require -exact csv\nneed 1";
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    let offset = u32::try_from(source.find("need 1").unwrap()).unwrap();
    let words = super::source_registry_words_at(source, &analysis, offset).unwrap();
    let context = analysis.resolved_input.as_ref().unwrap().context_registry();
    let reference = words.package_reference(&context).unwrap();
    assert_eq!(
        reference.kind(),
        tcl_registry::source_navigation::SourcePackageReferenceKind::Require
    );
    assert!(reference.matches_ascii("csv"));
    assert!(!reference.matches_ascii("other"));
    assert!(
        super::source_registry_words_at(&source.replace("need 1", "need 2"), &analysis, offset)
            .is_none()
    );
    let foreign = tcl_registry::model::ingress::static_context_for("tcl8.5");
    assert!(words.package_reference(foreign).is_none());
    let mut stale = analysis.clone();
    let original = stale.resolved_input.as_ref().unwrap();
    let mut config = original.lexer_config();
    config.leading_bom = match config.leading_bom {
        tcl_lexer::LeadingBom::Skip => tcl_lexer::LeadingBom::Content,
        tcl_lexer::LeadingBom::Content => tcl_lexer::LeadingBom::Skip,
    };
    stale.resolved_input = Some(crate::analyser::ResolvedAnalysisInput::new(
        original.analyser_profile(),
        original.unit_profile(),
        original.context_registry(),
        config,
    ));
    assert!(super::source_registry_words_at(source, &stale, offset).is_none());
}
#[test]
fn original_package_insertion_uses_root_commands_split_lines_and_full_input() {
    // naming.core.original-package-source-action-context
    // docs/design/analysis/name-resolution-proofs/core-original-package-source-action-context.md
    let sources = [
        (
            r"#!/usr/bin/env tclsh
package \
 require \
 csv
set x 1
",
            0usize,
        ),
        ("proc later {} {\n package require csv\n}\n", 0usize),
        ("set text {package require csv}\n", 0usize),
        ("package provide csv\n", 0usize),
        ("package require csv", "package require csv".len()),
    ];
    for (index, (source, fallback)) in sources.into_iter().enumerate() {
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let expected = if index == 0 {
            source.find("set x").unwrap()
        } else {
            fallback
        };
        assert_eq!(
            super::original_package_require_insert_offset(source, &analysis),
            Some(u32::try_from(expected).unwrap()),
            "{source:?}"
        );
        assert!(
            super::original_package_require_insert_offset(
                &format!("# foreign\n{source}"),
                &analysis
            )
            .is_none()
        );
    }
}

#[test]
fn original_package_insertion_includes_actual_command_terminator() {
    let source = "package require bar; http::get /resource\n";
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    let after_semicolon = source.find(';').unwrap() + 1;
    assert_eq!(
        super::original_package_require_insert_offset(source, &analysis),
        Some(u32::try_from(after_semicolon).unwrap())
    );
}
