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
fn method_source_candidates_keep_pre_lookup_operand_effects_unresolved() {
    let source = "proc helper {args} {}; oo::class create C {method m {} {::helper {[replaceHelper]}; ::helper [replaceHelper]; ::helper $value}}";
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let analysis = tcl_compiler::analyser::Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name);
    let offsets = source
        .match_indices("::helper")
        .map(|(offset, _)| u32::try_from(offset).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(offsets.len(), 3);
    for (offset, effects) in offsets.iter().zip([false, true, true]) {
        let original = tcl_compiler::registry_invocation::source_structure::source_logical_absolute_command_candidate_at(source, &analysis, *offset)
                .expect("actual original absolute method head syntax");
        assert_eq!(original.has_lookup_operand_effects(), effects);
        if effects {
            assert!(matches!(
                logical_method_procedure_at(source, &analysis, *offset),
                LogicalMethodProcedureSelection::Unavailable
            ));
        } else {
            assert!(matches!(
                logical_method_procedure_at(source, &analysis, *offset),
                LogicalMethodProcedureSelection::Occupied {
                    procedure: Some(_),
                    earlier_unknown_mutations: true,
                }
            ));
        }
    }
    let class = &analysis.all_classes["::C"];
    let item = item_for_method(source, class, &class.methods["m"], &LineIndex::new(source));
    let outgoing = outgoing_calls(source, profile, &item, &analysis);
    assert_eq!(outgoing.len(), 1);
    assert_eq!(outgoing[0].from_ranges.len(), 1);
    assert_eq!(
        outgoing[0].from_ranges[0],
        span_to_range(
            source,
            &LineIndex::new(source),
            tcl_lexer::Span::new(offsets[0], offsets[0] + 8)
        )
    );
    let unresolved = unresolved_outgoing_calls(source, profile, &item, &analysis);
    let helper = unresolved
        .iter()
        .find(|call| call.name == "::helper")
        .unwrap();
    assert_eq!(
        helper.from_ranges,
        offsets
            .iter()
            .map(|offset| span_to_range(
                source,
                &LineIndex::new(source),
                tcl_lexer::Span::new(*offset, *offset + 8)
            ))
            .collect::<Vec<_>>()
    );
    let effectful_ranges = offsets[1..]
        .iter()
        .map(|offset| {
            span_to_range(
                source,
                &LineIndex::new(source),
                tcl_lexer::Span::new(*offset, *offset + 8),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(effectful_ranges.len(), 2);
    assert!(
        effectful_ranges
            .iter()
            .all(|range| helper.from_ranges.contains(range))
    );
    assert!(
        helper
            .logical_occurrences
            .iter()
            .all(|occurrence| occurrence.candidates.is_none())
    );
}
