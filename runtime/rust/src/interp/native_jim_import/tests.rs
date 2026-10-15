// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use crate::interp::{
    native_compilation, native_operation_currency::NativeOperationScope, CallMeta, Command,
    ProcFrame,
};
use crate::obj::{self, Owned};

fn jim() -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("jimtcl"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}

// Exercise the existing activation owner directly; public marked entry remains denied.
fn entered_import(
    interpreter: &mut Interp,
    arguments: &[Owned],
    action: impl FnOnce(
        &mut Interp,
        &Rc<OriginalJimImportTraversal>,
        *mut TclObj,
        super::super::CmdFrame,
    ),
) {
    let operation = NativeOperationScope::enter(interpreter).unwrap();
    let helper = interpreter
        .native_invocation_dialect()
        .native_jim_namespace_helper(
            tcl_registry::native_jim_lookup::NativeJimNamespaceHelperKind::Import,
        )
        .unwrap();
    let generation = interpreter
        .namespaces()
        .resolve_generation(crate::namespace::GLOBAL, helper.name().as_bytes())
        .unwrap();
    let Some(Command::Proc(binding)) = interpreter.raw_command_by_generation(generation) else {
        panic!("actual original Import declaration")
    };
    let declaration = binding.declaration();
    let body = declaration.body.checked_ptr().unwrap();
    let location = declaration.location();
    let head = Owned::fresh(obj::new_string_bytes(helper.name().as_bytes()));
    let mut argv = vec![head.as_ptr()];
    argv.extend(arguments.iter().map(Owned::as_ptr));
    let grammar = interpreter
        .native_invocation_dialect()
        .parameter_grammar()
        .unwrap();
    let plan =
        native_compilation::native_parameter_plan(&declaration.params, arguments.len(), grammar)
            .unwrap();
    let mut meta = CallMeta {
        original_argv: Some(&argv),
        err: ProcFrame::Proc(helper.name().as_bytes()),
        fqn: Some(&location.qualified_name),
        source: declaration.source.clone(),
        body_line_base: declaration.body_line_base,
        link_vars: &[],
        oo_variable_resolver: None,
        keep_loop_codes: false,
        same_level: false,
        usage_prefix: None,
        level_words: None,
        quote_name: true,
        native: None,
        statics: declaration.native_statics(),
        c_procedure: Some(&declaration),
        c_method_client_data: None,
        jim_parameters: declaration
            .jim_parameters
            .as_ref()
            .map(obj::ProcedureObject::checked_ptr)
            .transpose()
            .unwrap(),
        jim_body: Some(body),
        jim_namespace: location.jim_namespace.as_ref(),
    };
    let (saved, frame) = interpreter
        .enter_native_procedure_frame(
            &declaration.params,
            &argv[1..],
            helper.name().as_bytes(),
            (location.namespace, &mut meta),
            plan,
            None,
        )
        .unwrap();
    let traversal = OriginalJimImportTraversal::capture(interpreter, &declaration)
        .unwrap()
        .unwrap();
    interpreter
        .frames
        .borrow_mut()
        .observe_original_jim_import_traversal(Rc::clone(&traversal));
    let level = interpreter.jim_procedure_level.get();
    interpreter.jim_procedure_level.set(level + 1);
    action(interpreter, &traversal, body, *frame);
    interpreter.jim_procedure_level.set(level);
    interpreter.clean_current_jim_local_commands();
    let popped = interpreter.pop_native_call_frame();
    interpreter.current_ns.set(saved);
    interpreter.leave_namespace_activation(popped);
    interpreter
        .recursion_depth
        .set(interpreter.recursion_depth.get() - 1);
    operation.currency().ensure_current_or_refuse().ok();
}

fn empty_result(interpreter: &mut Interp, _: &[*mut TclObj]) -> Code {
    interpreter.set_result(interpreter.new_list_object(&[]));
    Code::Ok
}

#[test]
fn empty_rest_needs_the_actual_outer_iterator_completion() {
    let mut interpreter = jim();
    entered_import(&mut interpreter, &[], |actual, traversal, body, frame| {
        assert!(traversal.normal_completion(actual).is_err());
        assert_eq!(
            actual.eval_original_body_framed(
                tcl_registry::native_eval_object::EvalObjectPurpose::ControlBody,
                body,
                frame,
            ),
            Code::Ok,
            "{:?}",
            actual.native_execution_refusal()
        );
        traversal.normal_completion(actual).unwrap();
        assert!(actual.native_execution_refusal().is_none());
    });
    let mut interpreter = jim();
    interpreter.register_builtin(b"foreach", empty_result);
    entered_import(&mut interpreter, &[], |actual, traversal, body, frame| {
        assert_eq!(
            actual.eval_original_body_framed(
                tcl_registry::native_eval_object::EvalObjectPurpose::ControlBody,
                body,
                frame,
            ),
            Code::Ok
        );
        assert!(traversal.normal_completion(actual).is_err());
    });
}

#[test]
fn an_unknown_empty_command_result_cannot_admit_an_inventory() {
    let mut interpreter = jim();
    interpreter.register_builtin(b"info", empty_result);
    let pattern = Owned::fresh(obj::new_string_bytes(b"not_present::*"));
    entered_import(
        &mut interpreter,
        &[pattern],
        |actual, traversal, body, frame| {
            assert_eq!(
                actual.eval_original_body_framed(
                    tcl_registry::native_eval_object::EvalObjectPurpose::ControlBody,
                    body,
                    frame,
                ),
                Code::Error
            );
            assert_eq!(
                actual.native_access_refusal(),
                Some(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "original Import inventory return producer"
                    )
                )
            );
            assert!(traversal.normal_completion(actual).is_err());
        },
    );
}

const SOURCE_OBSERVATIONS: &str = include_str!(
    "../../../tests/data/native_jim_import_source_completion/observations-20261015.tsv"
);

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn import_case(label: &str) {
    let source = include_str!("../../../tests/data/native_jim_import_source_completion/cases.tsv")
        .lines()
        .find_map(|line| {
            line.split_once('|')
                .filter(|(selected, _)| *selected == label)
                .map(|(_, source)| source)
        })
        .unwrap();
    let arguments = if source == "namespace import" {
        Vec::new()
    } else {
        let pattern = source.strip_prefix("namespace import ").unwrap();
        assert!(!pattern.contains(' '));
        vec![Owned::fresh(obj::new_string_bytes(pattern.as_bytes()))]
    };
    let mut actual = jim();
    assert_eq!(
        actual.eval_str(include_bytes!(
            "../../../tests/data/native_jim_import_source_completion/setup.tcl"
        )),
        Code::Ok,
        "{:?}",
        actual.native_execution_refusal()
    );
    entered_import(&mut actual, &arguments, |actual, traversal, body, frame| {
        let code = actual.eval_original_body_framed(
            tcl_registry::native_eval_object::EvalObjectPurpose::ControlBody,
            body,
            frame,
        );
        let row = format!(
            "IMPORT|{label}|{}|{}",
            code.as_int(),
            hex(&actual.result_bytes())
        );
        assert!(
            SOURCE_OBSERVATIONS.lines().any(|line| line == row),
            "{row}; refusal={:?}",
            actual.native_execution_refusal()
        );
        traversal.normal_completion(actual).unwrap();
        assert!(actual.native_execution_refusal().is_none());
    });
    for (phase, input) in [
        (
            "COMMANDS",
            include_bytes!(
                "../../../tests/data/native_jim_import_source_completion/commands-report.tcl"
            )
            .as_slice(),
        ),
        (
            "ALIAS",
            include_bytes!(
                "../../../tests/data/native_jim_import_source_completion/alias-report.tcl"
            )
            .as_slice(),
        ),
    ] {
        let code = actual.eval_str(input);
        let row = format!(
            "{phase}|{label}|{}|{}",
            code.as_int(),
            hex(&actual.result_bytes())
        );
        assert!(
            SOURCE_OBSERVATIONS.lines().any(|line| line == row),
            "{row}; refusal={:?}",
            actual.native_execution_refusal()
        );
    }
}

#[test]
fn genuine_import_source_completes_a_direct_command() {
    import_case("SIMPLE");
}
#[test]
fn genuine_import_source_completes_one_original_alias_hop() {
    import_case("ALIAS");
}
#[test]
fn genuine_import_source_completes_two_original_alias_hops() {
    import_case("DEEP_ALIAS");
}
#[test]
fn genuine_import_source_completes_an_observed_empty_inventory() {
    import_case("EMPTY");
}
#[test]
fn genuine_import_source_completes_an_empty_actual_rest_list() {
    import_case("EMPTY_REST");
}

#[test]
fn genuine_import_source_declines_a_repeated_original_alias_prefix() {
    let observation = include_bytes!(
        "../../../tests/data/native_jim_import_source_completion/cycle-observation-20261015.tsv"
    );
    assert!(observation.starts_with(b"CYCLE_SETUP|0|"));
    assert!(observation.ends_with(b"CYCLE_EVAL_ENTER\n"));
    let mut actual = jim();
    assert_eq!(
        actual.eval_str(include_bytes!(
            "../../../tests/data/native_jim_import_source_completion/cycle-setup.tcl"
        )),
        Code::Ok
    );
    let input =
        include_str!("../../../tests/data/native_jim_import_source_completion/cycle-input.tcl");
    let argument = Owned::fresh(obj::new_string_bytes(
        input.strip_prefix("namespace import ").unwrap().as_bytes(),
    ));
    entered_import(
        &mut actual,
        &[argument],
        |actual, traversal, body, frame| {
            assert_eq!(
                actual.eval_original_body_framed(
                    tcl_registry::native_eval_object::EvalObjectPurpose::ControlBody,
                    body,
                    frame
                ),
                Code::Error
            );
            assert_eq!(
                actual.native_access_refusal(),
                Some(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "Jim namespace import cyclic alias chain"
                    )
                )
            );
            assert!(traversal.normal_completion(actual).is_err());
        },
    );
}

#[test]
fn installed_helper_source_selection_requires_the_actual_body_and_live_definition() {
    for kind in [
        tcl_registry::native_jim_lookup::NativeJimNamespaceHelperKind::Info,
        tcl_registry::native_jim_lookup::NativeJimNamespaceHelperKind::Import,
    ] {
        let mut actual = jim();
        let _operation = NativeOperationScope::enter(&actual).unwrap();
        let helper = actual
            .native_invocation_dialect()
            .native_jim_namespace_helper(kind)
            .unwrap();
        let generation = actual
            .namespaces()
            .resolve_generation(crate::namespace::GLOBAL, helper.name().as_bytes())
            .unwrap();
        let Some(Command::Proc(binding)) = actual.raw_command_by_generation(generation) else {
            panic!("actual helper declaration");
        };
        let declaration = binding.declaration();
        let body = declaration.body.checked_ptr().unwrap();
        assert!(declaration
            .requires_original_namespace_helper_source_execution(&actual, Some(body))
            .unwrap());
        assert!(declaration
            .requires_original_namespace_helper_source_execution(&actual, Some(body))
            .unwrap());
        assert!(declaration
            .requires_original_namespace_helper_source_execution(&actual, None)
            .is_err());
        let copy = Owned::fresh(obj::duplicate(body));
        assert!(declaration
            .requires_original_namespace_helper_source_execution(&actual, Some(copy.as_ptr()))
            .is_err());
        let ordinary = actual.create_native_callable_from_chosen(
            Vec::new(),
            copy,
            crate::namespace::GLOBAL,
            None,
            0,
        );
        assert!(!ordinary
            .declaration()
            .requires_original_namespace_helper_source_execution(
                &actual,
                Some(ordinary.declaration().body.checked_ptr().unwrap())
            )
            .unwrap());
        drop(binding);
        let mut input = b"rename {".to_vec();
        input.extend_from_slice(helper.name().as_bytes());
        input.extend_from_slice(b"} {}");
        assert_eq!(actual.eval_str(&input), Code::Ok);
        assert!(declaration
            .requires_original_namespace_helper_source_execution(&actual, Some(body))
            .is_err());
    }
}
