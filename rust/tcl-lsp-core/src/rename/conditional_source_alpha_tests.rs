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
use tcl_compiler::analyser::{Analyser, ResolvedAnalysisInput};
fn analyse(source: &str) -> AnalysisResult {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let context = tcl_registry::model::ingress::context_for_profile(profile);
    Analyser::new()
        .with_resolved_input(ResolvedAnalysisInput::new(
            profile, profile, context, config,
        ))
        .analyse(source, profile.name)
}
#[test]
fn conditional_source_alpha_proposes_two_edits_without_native_permission() {
    let source = "set {x} 1\nputs ${x}\n";
    let analysis = analyse(source);
    assert!(prepare_rename(source, 1, 7, &analysis).is_none());
    assert!(prepare_conditional_variable_rename(source, 1, 7, &analysis).is_some());
    let edits = conditional_variable_rename_with_diagnosis(source, 1, 7, "new", &analysis).unwrap();
    assert_eq!(edits.edits.len(), 2);
    assert_eq!(edits.generated_source, "set {new} 1\nputs ${new}\n");
    assert!(
        conditional_variable_rename_with_diagnosis(
            &(source.to_owned() + " "),
            1,
            7,
            "new",
            &analysis
        )
        .is_err()
    );
}
#[test]
fn conditional_source_alpha_preserves_supplementary_unicode_source_geometry() {
    let source = "set {café🙂} 1\nputs ${café🙂}\n";
    let analysis = analyse(source);
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    assert!(rename_with_diagnosis(source, profile, 1, 7, "new", &analysis, None).is_err());
    assert!(prepare_conditional_variable_rename(source, 1, 7, &analysis).is_some());
    let proposal = conditional_variable_rename_with_diagnosis(source, 1, 7, "new", &analysis)
        .expect("source-local Unicode variable edit");
    assert_eq!(proposal.generated_source, "set {new} 1\nputs ${new}\n");
    assert_eq!(
        proposal.edits,
        vec![
            TextEdit {
                range: LspRange {
                    start_line: 0,
                    start_character: 4,
                    end_line: 0,
                    end_character: 12
                },
                new_text: "{new}".to_owned(),
            },
            TextEdit {
                range: LspRange {
                    start_line: 1,
                    start_character: 7,
                    end_line: 1,
                    end_character: 13
                },
                new_text: "new".to_owned(),
            },
        ]
    );
    assert!(
        conditional_variable_rename_with_diagnosis(
            &(source.to_owned() + " "),
            1,
            7,
            "new",
            &analysis,
        )
        .is_err()
    );
}

#[test]
fn conditional_source_alpha_refuses_trace_and_string_name_uses() {
    for source in [
        "trace add variable x write callback\nset x 1\nputs $x\n",
        "set x 1\nputs $x\nlist x\n",
    ] {
        let analysis = analyse(source);
        let line = if source.starts_with("trace") { 2 } else { 1 };
        assert!(
            conditional_variable_rename_with_diagnosis(source, line, 6, "new", &analysis).is_err(),
            "{source}"
        );
    }
}
