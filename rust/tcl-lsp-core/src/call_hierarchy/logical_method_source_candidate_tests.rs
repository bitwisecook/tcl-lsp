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

fn analyse(source: &str) -> AnalysisResult {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    tcl_compiler::analyser::Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name)
}

#[test]
fn method_proc_source_candidates_keep_unknown_namespace_and_mutation_alternatives() {
    let source = "proc helper {} {}; oo::class create C {method m {} {unknown; helper; ::helper}}";
    let mut analysis = analyse(source);
    let relative = u32::try_from(source.find("helper;").unwrap()).unwrap();
    assert!(matches!(
        logical_method_procedure_at(source, &analysis, relative),
        LogicalMethodProcedureSelection::Unavailable
    ));
    let absolute = u32::try_from(source.find("::helper").unwrap()).unwrap();
    let LogicalMethodProcedureSelection::Occupied {
        procedure: Some(target),
        earlier_unknown_mutations,
    } = logical_method_procedure_at(source, &analysis, absolute)
    else {
        panic!("actual absolute head must match its independent original procedure header");
    };
    let target_span = target.name_span;
    assert_eq!(target_span, analysis.all_procs["::helper"].name_span);
    assert!(earlier_unknown_mutations);
    let class = &analysis.all_classes["::C"];
    let item = item_for_method(source, class, &class.methods["m"], &LineIndex::new(source));
    let outgoing = outgoing_calls(
        source,
        analysis.resolved_profile().unwrap(),
        &item,
        &analysis,
    );
    assert_eq!(outgoing.len(), 1);
    assert_eq!(
        outgoing[0].to.selection_range,
        span_to_range(source, &LineIndex::new(source), target_span)
    );
    assert_eq!(outgoing[0].from_ranges.len(), 1);
    assert_eq!(
        outgoing[0].from_ranges[0],
        span_to_range(
            source,
            &LineIndex::new(source),
            tcl_lexer::Span::new(absolute, absolute + 8)
        )
    );
    let unresolved = unresolved_outgoing_calls(
        source,
        analysis.resolved_profile().unwrap(),
        &item,
        &analysis,
    );
    assert!(unresolved.iter().any(|call| call.name == "helper"));
    assert!(unresolved.iter().any(|call| call.name == "::helper"));
    analysis.all_classes.get_mut("::C").unwrap().qualified_name = "::report::Different".into();
    analysis
        .all_procs
        .get_mut("::helper")
        .unwrap()
        .qualified_name = "::report::Elsewhere".into();
    assert!(
        matches!(logical_method_procedure_at(source, &analysis, absolute), LogicalMethodProcedureSelection::Occupied { procedure: Some(procedure), .. } if procedure.name_span == target_span)
    );
    assert!(matches!(
        logical_method_procedure_at(&format!("{source} "), &analysis, absolute),
        LogicalMethodProcedureSelection::Unavailable
    ));
    analysis.resolved_input = None;
    assert!(matches!(
        logical_method_procedure_at(source, &analysis, absolute),
        LogicalMethodProcedureSelection::Unavailable
    ));
}

#[test]
fn method_proc_source_candidates_preserve_duplicate_headers_and_class_occupancy() {
    for source in [
        "proc helper {} {}; proc helper {} {}; oo::class create C {method m {} {::helper}}",
        "oo::class create helper {}; oo::class create C {method m {} {::helper}}",
    ] {
        let analysis = analyse(source);
        let absolute = u32::try_from(source.find("::helper").unwrap()).unwrap();
        assert!(matches!(
            logical_method_procedure_at(source, &analysis, absolute),
            LogicalMethodProcedureSelection::Occupied {
                procedure: None,
                ..
            }
        ));
    }
}
