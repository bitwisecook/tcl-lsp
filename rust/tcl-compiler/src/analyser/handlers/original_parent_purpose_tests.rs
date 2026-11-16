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
fn original_procedure_receipts_keep_file_config_and_extracted_body_bom() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // Original declaration/body syntax, without entered procedure or frame authority.
    let body = r"namespace eval N {proc p\uD800 {x} {return $x}}";
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
        let mut analyser = supplied.as_ref().map_or_else(Analyser::new, |input| {
            Analyser::new().with_resolved_input(input.clone())
        });
        analyser.analyse_and_retain_result_for_test(&source, "tcl9.1");
        let config = analyser.file_lexer_config();
        let image = tcl_lexer::SourceImage::document(&source);
        let declarations = analyser
            .result
            .original_procedure_declarations()
            .collect::<Vec<_>>();
        let [declaration] = declarations.as_slice() else {
            panic!("original nested declaration: {source}");
        };
        assert_eq!(declaration.name_input().bytes(), b"p\xed\xa0\x80");
        assert_eq!(declaration.name_input().source_image(), &image);
        assert_eq!(declaration.name_input().lexer_config(), config);
        let name = u32::try_from(source.find(r"p\uD800").unwrap()).unwrap();
        assert_eq!(declaration.name_input().span().start(), name);
        if let Some(supplied) = supplied {
            assert_eq!(analyser.result.resolved_input.as_ref(), Some(&supplied));
        }
        let mut foreign = config;
        foreign.strict_quoting = !foreign.strict_quoting;
        assert!(
            !analyser
                .result
                .matches_original_source_image(&image, foreign)
        );
        assert!(!analyser.result.matches_original_source_image(
            &tcl_lexer::SourceImage::document(&(source + "\n")),
            config
        ));
    }
    for bom in [tcl_lexer::LeadingBom::Skip, tcl_lexer::LeadingBom::Content] {
        let mut supplied = input.clone();
        supplied.config.leading_bom = bom;
        let source = "proc p {} {\u{feff}set hidden 1; set visible 2}";
        let mut analyser = Analyser::new().with_resolved_input(supplied.clone());
        analyser.analyse_and_retain_result_for_test(source, "display-only");
        let proc = analyser.result.all_procs.get("::p").unwrap();
        let statements = analyser.direct_statements_in_span(proc.body_span).unwrap();
        assert_eq!(
            statements
                .iter()
                .map(SegmentedCommand::name)
                .collect::<Vec<_>>(),
            ["\u{feff}set", "set"]
        );
        assert_eq!(
            statements[0].argv[0].span.start(),
            u32::try_from(source.find('\u{feff}').unwrap()).unwrap()
        );
        assert_eq!(analyser.result.resolved_input.as_ref(), Some(&supplied));
    }
}
