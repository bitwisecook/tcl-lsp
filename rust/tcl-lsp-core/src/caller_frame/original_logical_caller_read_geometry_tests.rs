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

#[test]
fn logical_caller_scalar_reads_use_original_components_and_selected_brace_grammar() {
    let source = "proc caller {} {puts $a(k); puts ${a}; puts {$a}; puts \"$a\"; puts $aa; proc hidden {} {puts $a}}";
    let analysis = Analyser::new().analyse(source, "tcl");
    let cursor = u32::try_from(source.find("${a}").unwrap()).unwrap();
    let spans = substituted_read_spans(source, &analysis, 0, source.len(), cursor, "a");
    assert_eq!(spans.len(), 2, "{spans:?}");
    assert!(spans.iter().all(|span| &source[span.as_range()] == "a"));
    assert_eq!(
        substituted_read_spans(
            &format!("{source}\n# changed"),
            &analysis,
            0,
            source.len(),
            cursor,
            "a"
        ),
        [] as [tcl_lexer::Span; 0]
    );

    let source = "proc caller {} {puts ${a{b}c}}";
    for (style, name) in [
        (tcl_dialect::BracedVarStyle::FirstClose, "a{b"),
        (tcl_dialect::BracedVarStyle::Tcl9Nesting, "a{b}c"),
    ] {
        let input = Analyser::new().prepare_analysis_input("tcl").unwrap();
        let mut config = input.lexer_config();
        config.braced_var = style;
        let input = ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            input.context_registry(),
            config,
        );
        let analysis = Analyser::new()
            .with_resolved_input(input)
            .analyse(source, "tcl");
        let cursor = u32::try_from(source.find("${").unwrap()).unwrap();
        let spans = substituted_read_spans(source, &analysis, 0, source.len(), cursor, name);
        assert_eq!(spans.len(), 1, "{style:?}: {spans:?}");
        assert_eq!(&source[spans[0].as_range()], name);
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert_eq!(
            substituted_read_spans(source, &missing, 0, source.len(), cursor, name),
            [] as [tcl_lexer::Span; 0]
        );
    }
}
