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
fn original_variable_frames_and_aliases_keep_the_whole_parent_config() {
    // naming.variable.conditional-write-name-advice
    // docs/design/analysis/name-resolution-proofs/conditional-write-name-advice.md
    // Original lexical frames and source alias advice supply no entered link or edit.
    let body = "set ::g VALUE; proc p {} {global g; set local 1; set local; set g}";
    let mut prepared = Analyser::new();
    let input = prepared.prepare_analysis_input("tcl9.1").unwrap();
    for (supplied, source) in [
        (None, format!("\u{feff}{body}")),
        (Some(input.clone()), format!("\u{feff}{body}")),
        (
            {
                let mut content = input.clone();
                content.config.leading_bom = tcl_lexer::LeadingBom::Content;
                content.config.braced_var = tcl_lexer::BracedVarStyle::FirstClose;
                Some(content)
            },
            body.to_owned(),
        ),
    ] {
        let analysis = supplied
            .as_ref()
            .map_or_else(Analyser::new, |input| {
                Analyser::new().with_resolved_input(input.clone())
            })
            .analyse(&source, "tcl9.1");
        let image = tcl_lexer::SourceImage::document(&source);
        let config = analysis.resolved_input.as_ref().unwrap().lexer_config();
        let local = u32::try_from(source.find("local 1").unwrap()).unwrap();
        let frame = analysis
            .original_variable_frame_in_source(&image, config, local)
            .unwrap();
        assert!(matches!(
            frame.frame().layout(),
            tcl_names::variable::VariableExecutionFrame::Procedure { .. }
        ));
        assert_eq!(frame.lexer_config(), config);
        let write = analysis
            .original_variable_write_advice
            .iter()
            .find(|advice| advice.original_name_input().bytes() == b"local")
            .unwrap();
        assert_eq!(write.original_frame(), Some(&frame));
        assert_eq!(
            write
                .original_name_input()
                .original_word_key()
                .unwrap()
                .source_image(),
            &image
        );
        let aliases = analysis
            .original_variable_alias_advice_in_source(&image, config)
            .unwrap();
        let alias = aliases
            .iter()
            .find(|alias| alias.original_name_input().bytes() == b"g")
            .unwrap();
        assert_eq!(alias.original_frame().lexer_config(), config);
        assert_eq!(
            alias
                .original_name_input()
                .original_word_key()
                .unwrap()
                .source_image(),
            &image
        );
        let mut foreign = config;
        foreign.strict_quoting = !foreign.strict_quoting;
        assert!(
            analysis
                .original_variable_frame_in_source(&image, foreign, local)
                .is_none()
        );
        assert!(
            analysis
                .original_variable_alias_advice_in_source(&image, foreign)
                .is_none()
        );
        let stale = tcl_lexer::SourceImage::document(&source.replace("VALUE", "OTHER"));
        assert!(
            analysis
                .original_variable_frame_in_source(&stale, config, local)
                .is_none()
        );
        assert!(
            analysis
                .original_variable_alias_advice_in_source(&stale, config)
                .is_none()
        );
        if let Some(supplied) = supplied {
            assert_eq!(analysis.resolved_input.as_ref(), Some(&supplied));
        }
    }
}
