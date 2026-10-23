// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use crate::analyser::ResolvedAnalysisInput;
use std::sync::Arc;

fn input(actual: &str, store: Arc<tcl_registry::CommandRegistry>) -> ResolvedAnalysisInput {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context = tcl_registry::model::ingress::resolve_environment(actual)
        .default_context_registry()
        .with_command_store(store);
    ResolvedAnalysisInput::new(
        profile,
        profile,
        Arc::new(context),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    )
}

fn analyse(source: &str, input: &ResolvedAnalysisInput) -> Analyser {
    let mut analyser = Analyser::new().with_resolved_input(input.clone());
    analyser.analyse_and_retain_result_for_test(source, "display-only");
    assert!(analyser.registry.is_none());
    assert!(analyser.context.is_none());
    assert!(analyser.result.allows_retained_logical_declaration_advice());
    analyser
}

#[test]
fn original_static_result_uses_retained_context_after_transient_store_retirement() {
    // naming.compiler.original-analysis-metadata-context

    let mut store = tcl_registry::CommandRegistry::build_default();
    store.insert(tcl_registry::CommandSpec {
        name: "original_return",
        ..store.get("return").unwrap().clone()
    });
    let actual = input("tcl8.6", Arc::new(store));
    let source = "interp alias {} answer {} original_return; proc p {} {answer VALUE}";
    let mut analyser = analyse(source, &actual);
    let context = analyser.retained_static_source_context().unwrap();
    assert!(Arc::ptr_eq(&context, &actual.context_registry()));
    assert_eq!(
        analyser.static_proc_result("::p", &[], &mut Vec::new(), 0),
        Some("VALUE".to_owned())
    );
    analyser.registry = Some(Arc::new(tcl_registry::CommandRegistry::build_default()));
    assert!(
        analyser
            .registry
            .as_deref()
            .unwrap()
            .get("original_return")
            .is_none()
    );
    assert_eq!(
        analyser.static_proc_result("::p", &[], &mut Vec::new(), 0),
        Some("VALUE".to_owned()),
        "a transient foreign store cannot replace the retained original schema"
    );
}

#[test]
fn original_static_fold_keeps_availability_and_selected_control_context() {
    // naming.compiler.original-analysis-metadata-context

    let mut store = tcl_registry::CommandRegistry::build_default();
    store.insert(tcl_registry::CommandSpec {
        name: "original_length",
        surface: Some(tcl_dialect::model::SpecSurface::TCL86_PLUS),
        ..store.get("string").unwrap().clone()
    });
    let store = Arc::new(store);
    let current = input("tcl8.6", Arc::clone(&store));
    let older = input("tcl8.4", store);
    assert!(Arc::ptr_eq(
        current.borrowed_context_registry().commands(),
        older.borrowed_context_registry().commands()
    ));
    let source = "proc p {} {original_length length café}";
    let mut current_analyser = analyse(source, &current);
    assert_eq!(
        current_analyser.static_proc_result("::p", &[], &mut Vec::new(), 0),
        Some("4".to_owned())
    );
    let mut older_analyser = analyse(source, &older);
    assert!(
        older_analyser
            .static_proc_result("::p", &[], &mut Vec::new(), 0)
            .is_none()
    );
    let source = "proc p {} {if {1} {return SELECTED} else {return UNSELECTED}}";
    let mut selected = analyse(source, &current);
    assert_eq!(
        selected.static_proc_result("::p", &[], &mut Vec::new(), 0),
        Some("SELECTED".to_owned())
    );
}

#[test]
fn original_static_context_refuses_missing_foreign_and_changed_source_axes() {
    // naming.compiler.original-analysis-metadata-context

    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let actual = input("tcl8.6", Arc::clone(context.commands()));
    let source = "proc p {} {return VALUE}";
    for axis in 0..8 {
        let mut analyser = analyse(source, &actual);
        assert!(analyser.retained_static_source_context().is_some());
        match axis {
            0 => analyser.result.resolved_input = None,
            1 => {
                analyser.resolved_input = Some(input(
                    "tcl8.4",
                    Arc::clone(actual.borrowed_context_registry().commands()),
                ))
            }
            2 => {
                let foreign = input(
                    "tcl8.6",
                    Arc::new(tcl_registry::CommandRegistry::build_default()),
                );
                analyser.resolved_input = Some(foreign.clone());
                analyser.result.resolved_input = Some(foreign);
            }
            3 => {
                let mut config = actual.lexer_config();
                config.strict_quoting ^= true;
                let altered = ResolvedAnalysisInput::new(
                    actual.analyser_profile(),
                    actual.unit_profile(),
                    actual.context_registry(),
                    config,
                );
                analyser.resolved_input = Some(altered.clone());
                analyser.result.resolved_input = Some(altered);
            }
            4 => analyser.source.push(' '),
            5 => analyser.profile = tcl_dialect::DialectProfile::find("tcl9.1").unwrap(),
            6 => analyser.result.command_realm = None,
            7 => analyser.head_identities = crate::realm::CommandBindingRealm::default(),
            _ => unreachable!(),
        }
        assert!(
            analyser.retained_static_source_context().is_none(),
            "axis {axis}"
        );
        assert!(
            analyser
                .static_proc_result("::p", &[], &mut Vec::new(), 0)
                .is_none(),
            "axis {axis}"
        );
    }
}

#[test]
fn original_static_result_retains_native_and_hosted_advice_barriers() {
    // naming.compiler.original-analysis-metadata-context

    let source = "proc p {} {return VALUE}";
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.1", "jim", "f5-irules"] {
        let mut analyser = Analyser::new();
        analyser.analyse_and_retain_result_for_test(source, dialect);
        assert!(
            !analyser.result.allows_retained_logical_declaration_advice(),
            "{dialect}"
        );
        assert!(
            analyser
                .static_proc_result("::p", &[], &mut Vec::new(), 0)
                .is_none(),
            "{dialect}"
        );
    }
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let actual = input("tcl8.6", Arc::clone(context.commands()));
    let mut analyser = analyse(source, &actual);
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let (_owner, native) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    analyser.source_analysis_entry = Some(Arc::new(crate::command_binding::SourceAnalysisEntry {
        native_entry: Some(Arc::new(native)),
        ..Default::default()
    }));
    assert!(analyser.retained_static_source_context().is_none());
    assert!(
        analyser
            .static_proc_result("::p", &[], &mut Vec::new(), 0)
            .is_none()
    );
}
