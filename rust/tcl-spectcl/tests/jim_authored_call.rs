// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Authentic authored Jim call metadata and its source-only refusal boundaries.
use tcl_compiler::analyser::{Analyser, types::AnalysisResult};
use tcl_compiler::registry_invocation::source_structure::{
    source_authored_jim_call_at, source_class_declaration_at,
};
fn run(source: &str, per_item: bool) -> AnalysisResult {
    tcl_spectcl::core_surfaces::ensure();
    let dialect = tcl_registry::dialects::detect_dialect(source, None, "tcl");
    let mut analyser = Analyser::new();
    if per_item {
        analyser.analyse_per_item(source, dialect)
    } else {
        analyser.analyse(source, dialect)
    }
}
#[test]
fn an_authored_call_retains_its_original_class_without_native_identity() {
    let source = "# tcl-dialect: jim\nclass Point {x 0 y 0}\nPoint new\n";
    for per_item in [false, true] {
        let result = run(source, per_item);
        let offset = u32::try_from(source.find("Point new").unwrap()).unwrap();
        let call = source_authored_jim_call_at(source, &result, offset)
            .expect("genuine selected class call");
        assert_eq!(call.original_words().len(), 2);
        assert_eq!(
            call.class()
                .variable_fields()
                .unwrap()
                .iter()
                .map(tcl_syntax::word_rules::OriginalSourceListElement::value)
                .collect::<Vec<_>>(),
            ["x", "0", "y", "0"]
        );
        assert_eq!(result.all_classes["::Point"].variables, ["x", "y"]);
        assert!(result.all_classes["::Point"].source_name.is_none());
        assert!(
            source_class_declaration_at(source, &result, call.class().factory().site().offset)
                .is_none()
        );
        let changed = source.replace("Point new", "Other new");
        assert!(
            source_authored_jim_call_at(&changed, &result, offset).is_none(),
            "foreign bytes never reuse original call authority"
        );
        let stale = "# tcl-dialect: jim\nclass Point {x 0 y 0}\nOther new\n";
        let stale_result = run(stale, per_item);
        assert!(
            source_authored_jim_call_at(source, &stale_result, offset).is_none(),
            "another analysis cannot supply the original call"
        );
    }
}
#[test]
fn callbacks_deferred_calls_missing_cells_and_foreign_dialects_refuse() {
    for source in [
        "# tcl-dialect: jim\nclass Point {x 0}\nPoint [unknown_callback]\n",
        "# tcl-dialect: jim\nclass Point {x 0}\nproc f {} { Point new }\n",
        "# tcl-dialect: jim\nPoint new\n",
        "# tcl-dialect: tcl8.6\nclass Point {x 0}\nPoint new\n",
    ] {
        for per_item in [false, true] {
            let result = run(source, per_item);
            let offset = u32::try_from(source.rfind("Point ").unwrap()).unwrap();
            assert!(
                source_authored_jim_call_at(source, &result, offset).is_none(),
                "{source}"
            );
        }
    }
}
#[test]
fn earlier_unknown_effects_remain_explicit_conditional_debt() {
    let source = "# tcl-dialect: jim\nclass Point {x 0}\nunknown_callback\nPoint new\n";
    for per_item in [false, true] {
        let result = run(source, per_item);
        let offset = u32::try_from(source.rfind("Point new").unwrap()).unwrap();
        let call = source_authored_jim_call_at(source, &result, offset)
            .expect("retained conditional class syntax");
        assert!(call.has_earlier_unknown_mutations());
        assert!(call.obligations().contains(&tcl_compiler::command_binding::SourceCommandTransitionObligation::UnknownEarlierMutation));
        assert!(result.all_classes["::Point"].source_name.is_none());
    }
}
