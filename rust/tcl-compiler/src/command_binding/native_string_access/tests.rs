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

fn append_stages(
    bindings: &SourceCommandBindings,
    source: &str,
    registry: &tcl_registry::CommandRegistry,
) -> [(bool, bool, Option<u64>); 2] {
    ["append greeting", "puts $greeting"].map(|spelling| {
        let head = spelling.split_once(' ').unwrap().0;
        let earlier = bindings
            .invocation_at_source(head, u32::try_from(source.find(spelling).unwrap()).unwrap());
        let variables = &earlier.variable_context;
        let access = crate::var_resolve::resolve_literal_access(
            "greeting",
            variables,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        (
            variables.contents_native_string_access_closed_at(&access, registry),
            variables
                .contents_native_string_access_receipt_at(&access, registry)
                .is_some(),
            variables.representation_epoch,
        )
    })
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
        let stages = append_stages(&bindings, source, registry);
        assert_selected_append_value(&bindings, source, engine, "hello, world!");
        original_cell_values_remain_distinct_after_append(engine);
        assert!(
            stages[0].0 && stages[0].1 && stages[1].0 && stages[1].1,
            "same entered append effect {engine}"
        );
        assert!(
            matches!((stages[0].2, stages[1].2), (Some(before), Some(after)) if after > before),
            "cache withdrawal remains {engine}"
        );
        assert!(
            context.contents_native_string_access_closed_at(&place, registry),
            "{engine}; before_append/before_puts={stages:?}; final_epoch={:?}; final_read={}",
            context.representation_epoch,
            context.read_produces_value(&place, registry),
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

fn known_name_value_requires_its_own_actual_input_effects() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    for (source, closed) in [
        (
            r#"set name world; set greeting "hello, $name"; set receiver greeting; append $receiver !; string length $greeting"#,
            true,
        ),
        (
            r#"set name world; set greeting "hello, $name"; set receiver [list greeting]; append $receiver !; string length $greeting"#,
            false,
        ),
    ] {
        let bindings = analyse(source, "tcl9.0");
        let append = bindings.invocation_at_source(
            "append",
            u32::try_from(source.find("append $").unwrap()).unwrap(),
        );
        assert_eq!(append.evaluated_written_argument_value(0), Some("greeting"));
        let receiver = crate::var_resolve::resolve_literal_access(
            "receiver",
            &append.variable_context,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        assert_eq!(
            append
                .variable_context
                .contents_native_string_result_input_closed_at(&receiver, registry),
            closed
        );
        let point = bindings.invocation_at_source(
            "string",
            u32::try_from(source.rfind("string length").unwrap()).unwrap(),
        );
        let greeting = crate::var_resolve::resolve_literal_access(
            "greeting",
            &point.variable_context,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        assert_eq!(
            point
                .variable_context
                .contents_native_string_access_receipt_at(&greeting, registry)
                .is_some(),
            closed
        );
    }
}

fn replaced_handler_cannot_receive_the_stock_append_effect_receipt() {
    let source = r#"set name world; set greeting "hello, $name"; rename append saved_append; proc append args {uplevel 1 {set greeting changed}}; append greeting !; string length $greeting"#;
    let bindings = analyse(source, "tcl9.0");
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    let append = bindings.invocation_at_source(
        "append",
        u32::try_from(source.find("append greeting !").unwrap()).unwrap(),
    );
    assert_eq!(
        append.proved_execution_target().unwrap().kind,
        crate::command_binding::BindingKind::Proc
    );
    let point = bindings.invocation_at_source(
        "string",
        u32::try_from(source.rfind("string length").unwrap()).unwrap(),
    );
    let greeting = crate::var_resolve::resolve_literal_access(
        "greeting",
        &point.variable_context,
        false,
        registry,
        tcl_registry::TraceOperation::Read,
    );
    // A different producer may independently close ordinary string access.
    // It cannot borrow the stock append's distinct effect receipt.
    assert!(
        point
            .variable_context
            .contents_native_string_access_receipt_at(&greeting, registry)
            .is_none()
    );
}

#[test]
fn unknown_component_and_observers_cannot_launder_a_string_result_receipt() {
    known_name_value_requires_its_own_actual_input_effects();
    representation_sensitive_data_never_borrows_string_effects_as_a_value();
    replaced_handler_cannot_receive_the_stock_append_effect_receipt();
    for source in [
        r#"set name [external_factory]; set greeting "hello, $name"; append greeting !; string length $greeting"#,
        r#"set name world; trace add variable name read external_callback; set greeting "hello, $name"; append greeting !; string length $greeting"#,
        r#"set name world; set greeting "hello, $name"; trace add variable greeting write external_callback; append greeting !; string length $greeting"#,
        r#"external_callback; set name world; set greeting "hello, $name"; append greeting !; string length $greeting"#,
        r#"set name world; set greeting "hello, $name"; trace add execution append enter external_callback; append greeting !; string length $greeting"#,
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

// This checks the same frozen argument API used by the real const-fold
// consumer, independently from the source effect and object-class receipts.
fn assert_selected_append_value(
    bindings: &SourceCommandBindings,
    source: &str,
    engine: &str,
    expected: &str,
) {
    let registry = tcl_registry::model::ingress::static_context_for(engine).commands();
    let point = bindings.invocation_at_source(
        "string",
        u32::try_from(source.rfind("string length").unwrap()).unwrap(),
    );
    let variables = &point.variable_context;
    let place = crate::var_resolve::resolve_literal_access(
        "greeting",
        variables,
        false,
        registry,
        tcl_registry::TraceOperation::Read,
    );
    assert_eq!(point.evaluated_written_argument_value(1), Some(expected));
    assert_eq!(
        variables.literal_contents_at(&place, registry),
        Some(expected)
    );
    let read = variables
        .original_name_read_result(&place, registry)
        .expect("actual same-cell produced value");
    assert!(read.is_current(variables, registry));
    assert_eq!(read.value().bytes(), expected.as_bytes());
}

fn original_cell_values_remain_distinct_after_append(engine: &str) {
    let source =
        "set original world; set greeting $original; append greeting !; string length $greeting";
    let bindings = analyse(source, engine);
    assert_selected_append_value(&bindings, source, engine, "world!");
    let registry = tcl_registry::model::ingress::static_context_for(engine).commands();
    let point = bindings.invocation_at_source(
        "string",
        u32::try_from(source.rfind("string length").unwrap()).unwrap(),
    );
    let place = crate::var_resolve::resolve_literal_access(
        "original",
        &point.variable_context,
        false,
        registry,
        tcl_registry::TraceOperation::Read,
    );
    assert_eq!(
        point.variable_context.literal_contents_at(&place, registry),
        Some("world")
    );
    let source = "set greeting a; append greeting $greeting b; string length $greeting";
    let bindings = analyse(source, engine);
    assert_selected_append_value(&bindings, source, engine, "aab");
}

fn representation_sensitive_data_never_borrows_string_effects_as_a_value() {
    for engine in ["tcl8.4", "tcl8.6", "tcl9.0"] {
        let source = "set greeting é; append greeting !; string length $greeting";
        let bindings = analyse(source, engine);
        let registry = tcl_registry::model::ingress::static_context_for(engine).commands();
        let append = bindings.invocation_at_source(
            "append",
            u32::try_from(source.find("append greeting").unwrap()).unwrap(),
        );
        let before = crate::var_resolve::resolve_literal_access(
            "greeting",
            &append.variable_context,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        assert!(
            append
                .variable_context
                .original_name_read_result(&before, registry)
                .is_some(),
            "known original operand {source}"
        );
        assert!(
            append
                .variable_context
                .contents_native_string_result_input_closed_at(&before, registry),
            "eligible actual append input {engine}"
        );
        assert!(
            append
                .variable_context
                .contents_native_string_access_closed_at(&before, registry),
            "eligible pre-append effect {engine}"
        );
        let point = bindings.invocation_at_source(
            "string",
            u32::try_from(source.rfind("string length").unwrap()).unwrap(),
        );
        let place = crate::var_resolve::resolve_literal_access(
            "greeting",
            &point.variable_context,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        assert!(
            point
                .variable_context
                .contents_native_string_access_closed_at(&place, registry),
            "effect stays independent {source}; engine={engine}; before_effect={}; before_input={}; before_stock={}; before_string_receipt={}; epochs={:?}/{:?}",
            append
                .variable_context
                .contents_native_string_access_closed_at(&before, registry),
            append
                .variable_context
                .contents_native_string_result_input_closed_at(&before, registry),
            append
                .variable_context
                .contents_stock_literal_object_at(&before, registry)
                .is_some(),
            append
                .variable_context
                .contents_native_string_access_receipt_at(&before, registry)
                .is_some(),
            append.variable_context.representation_epoch,
            point.variable_context.representation_epoch,
        );
        assert!(
            point.evaluated_written_argument_value(1).is_none(),
            "no unsupported value laundering {source}"
        );
        assert!(
            point
                .variable_context
                .literal_contents_at(&place, registry)
                .is_none()
        );
    }
    nul_source_value_never_borrows_an_append_effect_receipt();
    an_unclosed_nul_template_does_not_gain_an_append_effect();
}

fn nul_source_value_never_borrows_an_append_effect_receipt() {
    let source = r#"set greeting "\u0000"; append greeting !; string length $greeting"#;
    for engine in ["tcl8.4", "tcl8.6", "tcl9.0"] {
        let bindings = analyse(source, engine);
        let registry = tcl_registry::model::ingress::static_context_for(engine).commands();
        let append = bindings.invocation_at_source(
            "append",
            u32::try_from(source.find("append greeting").unwrap()).unwrap(),
        );
        let before = crate::var_resolve::resolve_literal_access(
            "greeting",
            &append.variable_context,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        assert!(
            append
                .variable_context
                .original_name_read_result(&before, registry)
                .is_some(),
            "known original NUL operand {engine}"
        );
        let point = bindings.invocation_at_source(
            "string",
            u32::try_from(source.rfind("string length").unwrap()).unwrap(),
        );
        let place = crate::var_resolve::resolve_literal_access(
            "greeting",
            &point.variable_context,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        assert!(
            point.evaluated_written_argument_value(1).is_none(),
            "no unsupported NUL value laundering {engine}"
        );
        assert!(
            point
                .variable_context
                .literal_contents_at(&place, registry)
                .is_none(),
            "no NUL constant from effect-only ownership {engine}"
        );
    }
}

fn an_unclosed_nul_template_does_not_gain_an_append_effect() {
    // This C8.4 quoted template has a known source value without a retained
    // stock-literal or String-input owner. Value bytes do not supply that owner.
    let engine = "tcl8.4";
    let source = r#"set greeting "\u0000"; append greeting !; string length $greeting"#;
    let bindings = analyse(source, engine);
    let registry = tcl_registry::model::ingress::static_context_for(engine).commands();
    let append = bindings.invocation_at_source(
        "append",
        u32::try_from(source.find("append greeting").unwrap()).unwrap(),
    );
    let before = crate::var_resolve::resolve_literal_access(
        "greeting",
        &append.variable_context,
        false,
        registry,
        tcl_registry::TraceOperation::Read,
    );
    assert!(
        !append
            .variable_context
            .contents_native_string_access_closed_at(&before, registry)
    );
    assert!(
        !append
            .variable_context
            .contents_native_string_result_input_closed_at(&before, registry)
    );
    assert!(
        append
            .variable_context
            .contents_stock_literal_object_at(&before, registry)
            .is_none()
    );
    assert!(
        append
            .variable_context
            .contents_native_string_access_receipt_at(&before, registry)
            .is_none()
    );
    let point = bindings.invocation_at_source(
        "string",
        u32::try_from(source.rfind("string length").unwrap()).unwrap(),
    );
    let place = crate::var_resolve::resolve_literal_access(
        "greeting",
        &point.variable_context,
        false,
        registry,
        tcl_registry::TraceOperation::Read,
    );
    assert!(
        !point
            .variable_context
            .contents_native_string_access_closed_at(&place, registry)
    );
    assert!(
        point
            .variable_context
            .contents_native_string_access_receipt_at(&place, registry)
            .is_none()
    );
}
