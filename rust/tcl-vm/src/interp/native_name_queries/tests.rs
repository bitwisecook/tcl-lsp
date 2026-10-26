// SPDX-License-Identifier: AGPL-3.0-or-later
//! Software query ownership across real selected String callbacks.
use super::*;
use crate::command::{NativeProcedureCommand, Param};
use std::cell::{Cell, RefCell};
use tcl_platform::{
    NativeIntegerFormatter, NativeIntegerFormatterBuild, NativeIntegerFormatterUnavailable,
    NativeIntegerKind,
};
use tcl_syntax::{number::Number, scalar_getter::NativeScalarCache};

#[derive(Default)]
struct Formatter {
    formats: Cell<usize>,
    mode: Cell<u8>,
    selected: RefCell<Option<NativeProcedureCommand>>,
    replacement: RefCell<Option<NativeProcedureCommand>>,
}
impl NativeIntegerFormatter for Formatter {
    fn build(&self) -> NativeIntegerFormatterBuild {
        NativeIntegerFormatterBuild {
            version: [8, 4, 20, 0],
            sha256: [7; 32],
            long_bits: 64,
        }
    }
    fn format(
        &self,
        _: NativeIntegerKind,
        value: i64,
    ) -> Result<Vec<u8>, NativeIntegerFormatterUnavailable> {
        self.formats.set(self.formats.get() + 1);
        if self.formats.get() == 1 {
            match self.mode.get() {
                1 => self
                    .selected
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .replace_declaration(self.replacement.borrow().as_ref().unwrap()),
                2 => self.selected.borrow().as_ref().unwrap().retire(),
                3 => return Err(NativeIntegerFormatterUnavailable::Operation),
                _ => {}
            }
        }
        Ok(value.to_string().into_bytes())
    }
}
struct Host {
    actual: Rc<dyn tcl_platform::Host>,
    formatter: Rc<Formatter>,
}
impl tcl_platform::Host for Host {
    fn capabilities(&self) -> tcl_platform::Capabilities {
        self.actual.capabilities()
    }
    fn clock(&self) -> &dyn tcl_platform::Clock {
        self.actual.clock()
    }
    fn stdio(&self) -> &dyn tcl_platform::StdIo {
        self.actual.stdio()
    }
    fn env(&self) -> &dyn tcl_platform::Env {
        self.actual.env()
    }
    fn native_integer_formatter(&self) -> Option<&dyn NativeIntegerFormatter> {
        Some(&*self.formatter)
    }
}
fn provided() -> (Vm, Rc<Formatter>) {
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl8.4"));
    let formatter = Rc::new(Formatter::default());
    vm.set_host(Rc::new(Host {
        actual: vm.host_rc(),
        formatter: Rc::clone(&formatter),
    }));
    (vm, formatter)
}
fn numeric(vm: &Vm, value: i64) -> Value {
    Value::from_native_scalar_cache(
        NativeScalarCache::Number(Number::Int(value)),
        None,
        vm.actual_native_invocation_dialect(),
    )
    .unwrap()
}
fn declaration(body: Value, defaults: &[Value]) -> NativeProcedureCommand {
    NativeProcedureCommand::new(ProcDef {
        native_resources: Rc::default(),
        name: "p".into(),
        command_ns_id: tcl_core_types::ROOT_NS,
        simple_name: "p".into(),
        execution_namespace: crate::interp::ProcedureNamespace::Resolved {
            path: tcl_core_types::ByteNamespacePath::root(),
            token: tcl_core_types::ROOT_NS,
        },
        params: defaults
            .iter()
            .enumerate()
            .map(|(i, value)| Param {
                name: format!("x{i}").into(),
                default: Some(value.clone()),
            })
            .collect(),
        parameter_grammar: tcl_dialect::ParameterGrammar::Tcl,
        has_args: false,
        native_parameters: None,
        native_jim_namespace: None,
        native_header: tcl_dialect::NativeProcedureHeaderCompilation::Absent,
        statics: None,
        body: None,
        body_src: body,
        usage_name: None,
        call_identity: None,
        lambda_term: None,
    })
}
fn arm(vm: &mut Vm, formatter: &Formatter, body: &Value, defaults: &[Value]) -> Rc<ProcDef> {
    let selected = declaration(body.clone(), defaults);
    let original = selected.declaration();
    *formatter.selected.borrow_mut() = Some(selected.clone());
    *formatter.replacement.borrow_mut() = Some(declaration(
        Value::new_native_string_bytes(b"NEW".as_slice()),
        &[],
    ));
    vm.register_command("p", Command::Proc(selected));
    original
}

#[test]
fn original_procedure_query_checks_selected_declaration_after_each_actual_string_stage() {

    // Actual VM producer and Host callback roles; no Native Proc pointer equivalence.
    for mode in 0..4 {
        let (mut vm, formatter) = provided();
        let defaults = [numeric(&vm, i64::MIN), numeric(&vm, i64::MIN + 1)];
        let body = numeric(&vm, i64::MIN + 2);
        let original = arm(&mut vm, &formatter, &body, &defaults);
        let result = vm
            .adopt_native_interp_result(Value::new_native_string_bytes(b"BEFORE".as_slice()))
            .unwrap();
        formatter.mode.set(mode);
        assert_eq!(original.native_reference_count(), 1);
        assert_eq!(defaults[0].native_object_reference_count(), 2);
        let reached = tcl_runtime_api::Procs::proc_info_bytes(&mut vm, b"p");
        if mode == 0 {
            let info = reached.unwrap().unwrap();
            assert_eq!(
                info.params[0].default.as_deref(),
                Some(i64::MIN.to_string().as_bytes())
            );
            assert_eq!(
                info.params[1].default.as_deref(),
                Some((i64::MIN + 1).to_string().as_bytes())
            );
            assert_eq!(info.body, (i64::MIN + 2).to_string().as_bytes());
            assert_eq!(formatter.formats.get(), 3);
            assert_eq!(original.native_reference_count(), 1);
        } else {
            let error = reached.unwrap_err();
            assert_eq!(
                error.native_execution_refusal(),
                vm.execution_refusal.as_ref()
            );
            assert!(error.native_execution_refusal().is_some());
            assert_eq!(formatter.formats.get(), 1);
            assert!(defaults[1].resident_string_bytes().is_none());
            assert!(body.resident_string_bytes().is_none());
            if mode == 1 || mode == 2 {
                assert_eq!(original.native_reference_count(), 0);
                assert_eq!(defaults[0].native_object_reference_count(), 1);
                assert!(defaults[0].native_object_is_live());
            }
        }
        assert!(
            vm.with_native_interp_result(|value| value.is_same_object(result.value()))
                .unwrap()
        );
        assert_eq!(
            result.value().resident_string_bytes().unwrap().as_ref(),
            b"BEFORE"
        );
    }
}

#[test]
fn shared_proc_body_consumer_preserves_outer_refusal_instead_of_reporting_missing_procedure() {

    // The generic consumer transports full CmdError from the selected original query.
    let (mut vm, formatter) = provided();
    let body = numeric(&vm, i64::MIN);
    let unread = numeric(&vm, i64::MIN + 1);
    let original = {
        let argument = unread.clone();
        arm(&mut vm, &formatter, &body, std::slice::from_ref(&argument))
    };
    formatter.mode.set(1);
    let name = Value::new_native_string_bytes(b"p".as_slice());
    let error = tcl_cmd_core::info::body(&mut vm, &name).unwrap_err();
    assert_eq!(formatter.formats.get(), 1);
    assert!(unread.resident_string_bytes().is_none());
    assert_eq!(original.native_reference_count(), 0);
    assert_eq!(
        error.native_execution_refusal(),
        vm.execution_refusal.as_ref()
    );
    assert!(matches!(
        error.native_execution_refusal(),
        Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "original procedure query owner retired"
            )
        ))
    ));
}

#[test]
fn original_alias_query_keeps_resident_headers_and_pending_first_host() {

    // Byte projection of the actual existing VM alias, without a new Name/String recipe.
    let (mut vm, formatter) = provided();
    let words = Rc::new(vec![
        Value::new_native_string_bytes(b"target".as_slice()),
        Value::new_native_string_bytes(b"a\0b".as_slice()),
    ]);
    vm.register_command(
        "a",
        Command::Alias {
            words: Rc::clone(&words),
            publication_name: "a".into(),
            registration_name: Some(Value::new_native_string_bytes(b"a".as_slice())),
        },
    );
    let id = tcl_runtime_api::Namespaces::find_command_bytes(&vm, tcl_runtime_api::ROOT_NS, b"a")
        .unwrap();
    let snapshots = words
        .iter()
        .map(Value::native_object_snapshot)
        .collect::<Vec<_>>();
    let counts = words
        .iter()
        .map(Value::native_object_reference_count)
        .collect::<Vec<_>>();
    assert_eq!(
        vm.original_alias_bytes_query(id).unwrap(),
        Some(vec![b"target".to_vec(), b"a\0b".to_vec()])
    );
    assert_eq!(formatter.formats.get(), 0);
    assert_eq!(
        words
            .iter()
            .map(Value::native_object_snapshot)
            .collect::<Vec<_>>(),
        snapshots
    );
    assert_eq!(
        words
            .iter()
            .map(Value::native_object_reference_count)
            .collect::<Vec<_>>(),
        counts
    );
    let _unwind = vm.refuse_host_command("FIRST_QUERY_HOST".into());
    let first = vm.execution_refusal.clone().unwrap();
    let error = vm.original_alias_bytes_query(id).unwrap_err();
    assert_eq!(error.native_execution_refusal(), Some(&first));
    assert_eq!(formatter.formats.get(), 0);
}
