// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual source-producer effects, distinct from bytes/types/completion.

use super::*;
use crate::command_binding::{SourceAnalysisOptions, SourceCommandBindings, SourceOriginId};
use tcl_registry::list_object_methods::NativeListMethod;

fn analyse(source: &str, engine: &str) -> SourceCommandBindings {
    let profile = tcl_dialect::DialectProfile::find(engine).unwrap();
    let registry = tcl_registry::model::ingress::static_context_for(engine).commands();
    SourceCommandBindings::analyse_with_options(
        source,
        tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        registry,
        SourceAnalysisOptions {
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                ..Default::default()
            },
            ..Default::default()
        },
    )
}

#[test]
fn same_actual_source_template_and_append_close_only_string_effects() {
    let source = r#"set name world; set greeting "hello, $name"; append greeting "!"; puts $greeting; string length $greeting"#;
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let bindings = analyse(source, engine);
        let registry = tcl_registry::model::ingress::static_context_for(engine).commands();
        let append = bindings.invocation_at_source(
            "append",
            u32::try_from(source.find("append greeting").unwrap()).unwrap(),
        );
        assert!(matches!(
            append.native_compilation_selection(),
            tcl_registry::native_compilation::NativeCompilationSelection::Generic
        ));
        assert_eq!(
            append.proved_execution_target().unwrap().kind,
            crate::command_binding::BindingKind::Builtin
        );
        let dialect = tcl_registry::InvocationDialect::of_profile(
            tcl_dialect::DialectProfile::find(engine).unwrap(),
        );
        let resolved = registry
            .resolve_invocation("append", &["greeting", "!"], dialect.authoring_query())
            .unwrap();
        assert_eq!(
            resolved.facts().successful_handler,
            Some(tcl_registry::native_compilation::SuccessfulHandlerSpec::VariableStringAppend)
        );
        let point = bindings.invocation_at_source(
            "string",
            u32::try_from(source.rfind("string length").unwrap()).unwrap(),
        );
        let context = &point.variable_context;
        let place = crate::var_resolve::resolve_literal_access(
            "greeting",
            context,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        assert!(
            context.contents_native_string_access_closed_at(&place, registry),
            "{engine}"
        );
        assert!(
            context
                .contents_native_string_access_receipt_at(&place, registry)
                .is_some(),
            "real producer receipt {engine}"
        );
        assert_eq!(
            context.contents_representation_at(&place),
            tcl_syntax::value::ValueRepresentation::Unknown
        );
        assert!(
            context
                .contents_native_numeric_category_at(&place, registry)
                .is_none()
        );
        assert!(
            context
                .contents_stock_literal_object_at(&place, registry)
                .is_none()
        );
    }
}

#[test]
fn unknown_component_and_observers_cannot_launder_a_string_result_receipt() {
    for source in [
        r#"set name [external_factory]; set greeting "hello, $name"; append greeting !; string length $greeting"#,
        r#"set name world; trace add variable name read external_callback; set greeting "hello, $name"; append greeting !; string length $greeting"#,
        r#"set name world; set greeting "hello, $name"; trace add variable greeting write external_callback; append greeting !; string length $greeting"#,
        r#"external_callback; set name world; set greeting "hello, $name"; append greeting !; string length $greeting"#,
    ] {
        let bindings = analyse(source, "tcl9.0");
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let point = bindings.invocation_at_source(
            "string",
            u32::try_from(source.rfind("string length").unwrap()).unwrap(),
        );
        let context = &point.variable_context;
        let place = crate::var_resolve::resolve_literal_access(
            "greeting",
            context,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        assert!(
            !context.contents_native_string_access_closed_at(&place, registry),
            "{source}"
        );
    }
}

#[test]
fn captured_native_effect_owner_refuses_foreign_source_and_expires_without_fallback() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").unit_profile();
    let (mut vm, entry) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let options = SourceAnalysisOptions {
        native_entry: Some(&entry),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(
            registry.profile().unwrap(),
        )),
        ..Default::default()
    };
    let mut state = ModuleCommandBindings::initial_with_options(registry, options, None);
    state.current_source_origin = Some(Arc::new(SourceOriginId::authored_image(
        tcl_lexer::SourceImage::native(b"set original source".as_slice()),
    )));
    state.ordinary_literal_pool =
        super::super::literal_object_pool::SourceOrdinaryLiteralPool::at_entry(&state, options);
    let receipt = SourceNativeStringAccess::from_normal(&state, &[]).expect("actual current owner");
    let authored = analyse("set independently_created value", "tcl9.0");
    let carried = SourceNativeStringAccess::from_normal(
        authored.final_state.as_ref(),
        std::slice::from_ref(&receipt),
    )
    .unwrap();
    assert!(carried.is_current());
    assert_eq!(
        carried.owners.len(),
        2,
        "both distinct actual origins remain owned"
    );
    let frozen =
        super::super::source_representation::FrozenSourceRepresentation::capture_native_string(
            Arc::clone(&receipt),
            &state.source_variables,
        )
        .unwrap();
    assert!(
        frozen
            .callback_input_is_closed_for(&state.source_variables, NativeListMethod::StringAccess)
    );
    assert!(
        !frozen.callback_input_is_closed_for(&state.source_variables, NativeListMethod::Length)
    );
    assert!(!frozen.numeric_input_is_closed(&state.source_variables));
    let mut foreign = state.clone();
    foreign.current_source_origin = Some(Arc::new(SourceOriginId::authored_image(
        tcl_lexer::SourceImage::native(b"set foreign source".as_slice()),
    )));
    assert!(SourceNativeStringAccess::from_normal(&foreign, &[]).is_none());
    let _ = vm.try_eval_source("set changed_capture 1");
    assert!(!receipt.is_current());
    assert!(
        !carried.is_current(),
        "a new authored producer cannot launder its retained native donor"
    );
    assert!(!frozen.is_current(&state.source_variables));
    drop(vm);
    assert!(!receipt.is_current());
}
