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
