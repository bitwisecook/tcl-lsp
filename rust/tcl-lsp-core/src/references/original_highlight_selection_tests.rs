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

#[test]
fn original_highlights_preserve_opaque_declarations_and_whole_source_currency() {
    // Implementation contract: naming.editor.original-reference-highlight-selection
    // docs/design/analysis/name-resolution-proofs/original-reference-highlight-selection.md
    let source = r"proc p\uD800 {} {}
proc p\uD801 {} {}
p\uD800";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    assert_eq!(analysis.original_procedure_declarations().count(), 2);
    analysis.all_procs.clear();
    analysis.global_scope.procs.clear();
    for invocation in &mut analysis.command_invocations {
        invocation.name.clear();
        invocation.resolved_qualified_name = None;
        invocation.resolution_candidates.clear();
    }
    let profile = crate::profile_for_analysis(&analysis);
    let highlights = document_highlights(source, profile, 0, 7, &analysis);
    assert_eq!(highlights.len(), 2, "declaration and actual selected call");
    assert!(
        highlights
            .iter()
            .all(|(_, kind)| *kind == HighlightKind::Text)
    );
    assert_eq!(highlights[0].0.start_line, 0);
    assert_eq!(highlights[1].0.start_line, 2);
    assert_eq!(references(source, profile, 0, 7, &analysis, true).len(), 2);
    let stale = source.replace(r"p\uD800", r"q\uD800");
    assert_eq!(
        document_highlights(&stale, profile, 0, 7, &analysis),
        [] as [(
            crate::definition::LspRange,
            crate::references::HighlightKind
        ); 0]
    );
    assert_eq!(
        references(&stale, profile, 0, 7, &analysis, true),
        [] as [crate::definition::LspRange; 0]
    );
}

#[test]
fn original_method_reference_helpers_do_not_select_from_reporting_names() {
    // Implementation contract: naming.editor.original-reference-highlight-selection
    // docs/design/analysis/name-resolution-proofs/original-reference-highlight-selection.md
    let source = "oo::class create C {method m {} {}}";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    assert!(analysis.all_classes.contains_key("::C"));
    let profile = crate::profile_for_analysis(&analysis);
    let offset = u32::try_from(source.find("m {}").unwrap()).unwrap();
    let std::ops::ControlFlow::Break(Some(candidate)) =
        crate::method_symbol::local_candidate(source, &analysis, 0, offset)
    else {
        panic!("authentic source member remains available through the typed query");
    };
    let highlights = document_highlights(source, profile, 0, offset, &analysis);
    assert_eq!(highlights.len(), 1);
    assert_eq!(candidate.declaration_span().start(), offset);
    assert!(
        list_built_self_method_target_at_cursor(source, profile, &analysis, "m", offset,).is_none(),
        "typed original declarations cannot enter lexical tuple selection"
    );
    assert_eq!(
        method_reference_spans_in_document(source, profile, &analysis, "::C", "m", true, false,),
        [] as [tcl_lexer::Span; 0]
    );
    assert_eq!(
        obj_method_call_sites(source, profile, &analysis, "::C", "m", false, &[],),
        [] as [tcl_lexer::Span; 0]
    );
    assert_eq!(
        method_next_dispatch_spans(&analysis, source, profile, "::C", "m", false,),
        [] as [tcl_lexer::Span; 0]
    );
}
