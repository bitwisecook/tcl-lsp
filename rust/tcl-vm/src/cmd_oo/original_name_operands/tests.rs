// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fixed software controls for actual OO target and Name continuation owners.
use super::*;
use crate::cmd_oo::{
    Class, make_class_bytes, native_context, oo_dispatch, resolve_object_value, teardown,
};
use crate::command::{Command, NativeCommand};
use crate::interp::ok;
use std::cell::Cell;
use tcl_platform::{
    NativeIntegerFormatter, NativeIntegerFormatterBuild, NativeIntegerFormatterUnavailable,
    NativeIntegerKind,
};
use tcl_runtime_api::{Code, NativeExecutionError};
use tcl_syntax::{
    number::Number, raw_string::NativeValueAccessRefusal, scalar_getter::NativeScalarCache,
};

#[derive(Default)]
struct Formatter {
    fail: Cell<bool>,
    formats: Cell<usize>,
    queries: Cell<usize>,
}
impl NativeIntegerFormatter for Formatter {
    fn build(&self) -> NativeIntegerFormatterBuild {
        NativeIntegerFormatterBuild {
            version: [8, 6, 18, 0],
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
        if self.fail.get() {
            return Err(NativeIntegerFormatterUnavailable::Operation);
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
    fn numeric_environment(&self) -> Option<&dyn tcl_platform::NumericEnvironment> {
        self.actual.numeric_environment()
    }
    fn filesystem(&self) -> Option<&dyn tcl_platform::Filesystem> {
        self.actual.filesystem()
    }
    fn native_integer_formatter(&self) -> Option<&dyn NativeIntegerFormatter> {
        self.formatter.queries.set(self.formatter.queries.get() + 1);
        Some(self.formatter.as_ref())
    }
}
fn provided() -> (Vm, OoId, Rc<Formatter>) {
    let mut vm =
        crate::native_fixture::interpreter(crate::environment::profile_for_dialect("tcl8.6"));
    let root = vm.oo.class_root.unwrap();
    assert_eq!(
        make_class_bytes(&mut vm, b"OriginalOOName", None, root).code,
        Code::Ok
    );
    let target = resolve_object_value(
        &mut vm,
        &Value::new_native_string_bytes(b"OriginalOOName".as_slice()),
    )
    .unwrap()
    .unwrap();
    let formatter = Rc::new(Formatter::default());
    vm.set_host(Rc::new(Host {
        actual: vm.host_rc(),
        formatter: Rc::clone(&formatter),
    }));
    (vm, target, formatter)
}
fn numeric(vm: &Vm) -> Value {
    Value::from_native_scalar_cache(
        NativeScalarCache::Number(Number::Int(42)),
        None,
        vm.actual_native_invocation_dialect(),
    )
    .unwrap()
}
fn expect_target_refusal(vm: &Vm) {
    assert_eq!(
        vm.execution_refusal,
        Some(NativeExecutionError::ValueAccessRefusal(
            NativeValueAccessRefusal::CommandProtocolUnavailable("original OO Name target/facet")
        ))
    );
}
#[test]
fn original_oo_name_target_keeps_rename_and_ordinary_method_mutation_without_reference_roles() {

    let (mut vm, target, formatter) = provided();
    let name_owner = Rc::clone(&vm.oo.objects[&target].cached_name);
    let facet = Rc::clone(&vm.oo.classes[&target].birth);
    let owner_count = Rc::strong_count(&name_owner);
    let facet_count = Rc::strong_count(&facet);
    let original = Value::new_native_string_bytes(b"member\0\xff".as_slice());
    let count = original.native_object_reference_count();
    let reached = run_result(&mut vm, Some((true, target)), |vm, names| {
        assert_eq!(Rc::strong_count(&name_owner), owner_count);
        assert_eq!(Rc::strong_count(&facet), facet_count);
        let renamed = vm.invoke_command(
            "rename",
            &[
                Value::new_native_string_bytes(b"OriginalOOName".as_slice()),
                Value::new_native_string_bytes(b"RenamedOOName".as_slice()),
            ],
        );
        assert_eq!(renamed.code, Code::Ok);
        let defined = vm.invoke_command(
            "oo::define",
            &[
                Value::new_native_string_bytes(b"RenamedOOName".as_slice()),
                Value::string("method"),
                Value::string("ordinary"),
                Value::empty(),
                Value::string("return OK"),
            ],
        );
        assert_eq!(defined.code, Code::Ok);
        names
            .read(vm, &original)
            .map_err(|error| crate::command::completion_from_cmd_error(vm, error))
    })
    .unwrap();
    assert_eq!(reached.as_ref(), b"member\0\xff");
    assert_eq!(original.native_object_reference_count(), count);
    assert!(Rc::ptr_eq(&name_owner, &vm.oo.objects[&target].cached_name));
    assert!(Rc::ptr_eq(&facet, &vm.oo.classes[&target].birth));
    assert_eq!(formatter.queries.get(), 0);
    assert!(vm.execution_refusal.is_none());
}
#[test]
fn original_oo_name_withdrawal_refuses_recreated_written_object_before_later_getter() {

    let (mut vm, target, formatter) = provided();
    let original = numeric(&vm);
    let root = vm.oo.class_root.unwrap();
    let reached = run_result(&mut vm, Some((true, target)), |vm, names| {
        teardown(vm, target);
        assert_eq!(
            make_class_bytes(vm, b"OriginalOOName", None, root).code,
            Code::Ok
        );
        let current = resolve_object_value(
            vm,
            &Value::new_native_string_bytes(b"OriginalOOName".as_slice()),
        )
        .unwrap()
        .unwrap();
        assert_ne!(current, target);
        names
            .read(vm, &original)
            .map_err(|error| crate::command::completion_from_cmd_error(vm, error))
    });
    assert_eq!(reached.unwrap_err().code, Code::Error);
    expect_target_refusal(&vm);
    assert_eq!(formatter.queries.get(), 0);
    assert!(original.resident_string_bytes().is_none());
}
#[test]
fn original_oo_name_facet_replacement_keeps_object_and_namespace_guards_independent() {

    // Real Default/map metadata replacement; no Native Class pointer or demotion API claim.
    let (mut vm, target, formatter) = provided();
    let object = Rc::clone(&vm.oo.objects[&target].cached_name);
    let namespace = vm.oo.objects[&target].namespace_token;
    let observed = vm.oo.classes[&target].clone();
    let original = numeric(&vm);
    let reached = run_result(&mut vm, Some((true, target)), |vm, names| {
        let retired = vm.oo.classes.insert(target, Class::default()).unwrap();
        assert!(Rc::ptr_eq(&retired.birth, &observed.birth));
        assert!(!Rc::ptr_eq(&retired.birth, &vm.oo.classes[&target].birth));
        drop(retired);
        assert!(Rc::ptr_eq(&object, &vm.oo.objects[&target].cached_name));
        assert_eq!(vm.oo.objects[&target].namespace_token, namespace);
        names
            .read(vm, &original)
            .map_err(|error| crate::command::completion_from_cmd_error(vm, error))
    });
    assert_eq!(reached.unwrap_err().code, Code::Error);
    expect_target_refusal(&vm);
    assert_eq!(formatter.queries.get(), 0);
}
struct Body(Rc<Cell<usize>>);
impl NativeCommand for Body {
    fn invoke(&self, _: &mut Vm, _: &[Value]) -> Completion<Value> {
        self.0.set(self.0.get() + 1);
        ok(Value::string("BODY"))
    }
}
#[test]
fn original_oo_method_name_formatter_refusal_preserves_full_first_cause_before_body() {

    // Supplied formatter callback behavior is software evidence only.
    let (mut vm, target, formatter) = provided();
    let calls = Rc::new(Cell::new(0));
    vm.register_command(
        "record_oo_name_body",
        Command::Native(Rc::new(Body(Rc::clone(&calls)))),
    );
    let defined = vm.invoke_command(
        "oo::define",
        &[
            Value::new_native_string_bytes(b"OriginalOOName".as_slice()),
            Value::string("method"),
            Value::string("42"),
            Value::empty(),
            Value::string("record_oo_name_body"),
        ],
    );
    assert_eq!(defined.code, Code::Ok);
    assert_eq!(
        vm.invoke_command(
            "oo::define",
            &[
                Value::new_native_string_bytes(b"OriginalOOName".as_slice()),
                Value::string("export"),
                Value::string("42")
            ]
        )
        .code,
        Code::Ok
    );
    let created = vm.invoke_command(
        "OriginalOOName",
        &[Value::string("create"), Value::string("OONameObject")],
    );
    assert_eq!(created.code, Code::Ok);
    let object = resolve_object_value(
        &mut vm,
        &Value::new_native_string_bytes(b"OONameObject".as_slice()),
    )
    .unwrap()
    .unwrap();
    let prior = vm
        .adopt_native_interp_result(Value::new_native_string_bytes(b"BEFORE".as_slice()))
        .unwrap();
    let original = numeric(&vm);
    formatter.fail.set(true);
    let result = oo_dispatch(
        &mut vm,
        object,
        &Value::new_native_string_bytes(b"OONameObject".as_slice()),
        std::slice::from_ref(&original),
    );
    assert_eq!(result.code, Code::Error);
    let first = vm.execution_refusal.clone().unwrap();
    assert!(matches!(
        first,
        NativeExecutionError::ValueAccessRefusal(NativeValueAccessRefusal::StringAccess(_))
    ));
    assert_eq!(formatter.formats.get(), 1);
    assert_eq!(calls.get(), 0);
    assert!(original.resident_string_bytes().is_none());
    assert!(
        vm.with_native_interp_result(|value| value.is_same_object(prior.value()))
            .unwrap()
    );
    let again = native_context::object_name(&vm, target);
    assert_eq!(
        oo_dispatch(&mut vm, object, &again, &[Value::string("42")]).code,
        Code::Error
    );
    assert_eq!(vm.execution_refusal, Some(first));
    assert_eq!(calls.get(), 0);
    assert_eq!(formatter.formats.get(), 1);
}
#[test]
fn original_oo_definition_name_refusal_restores_real_definition_activation_before_return() {

    let (mut vm, target, formatter) = provided();
    let original = numeric(&vm);
    let level = vm.current_level();
    let namespace = vm.current_ns_id();
    formatter.fail.set(true);
    let completion = vm.invoke_command(
        "oo::define",
        &[
            Value::new_native_string_bytes(b"OriginalOOName".as_slice()),
            Value::string("variable"),
            original,
        ],
    );
    assert_eq!(completion.code, Code::Error);
    assert!(matches!(
        vm.execution_refusal,
        Some(NativeExecutionError::ValueAccessRefusal(
            NativeValueAccessRefusal::StringAccess(_)
        ))
    ));
    assert_eq!(formatter.formats.get(), 1);
    assert!(vm.oo.classes[&target].variables.is_empty());
    assert!(vm.oo.def_stack.is_empty());
    assert_eq!(vm.current_level(), level);
    assert_eq!(vm.current_ns_id(), namespace);
}

struct ReplaceDeclaringFacet {
    target: OoId,
    hits: Rc<Cell<usize>>,
    original: Value,
}
impl NativeCommand for ReplaceDeclaringFacet {
    fn invoke(&self, vm: &mut Vm, _: &[Value]) -> Completion<Value> {
        let frame = native_context::current_method(vm).expect("genuine method activation");
        let receiver = frame.object;
        let step = frame.chain.get(frame.index).expect("entered chain step");
        assert_eq!(step.provider, self.target);
        assert!(!step.is_object);
        self.hits.set(self.hits.get() + 1);
        run(vm, Some((false, receiver)), |vm, names| {
            if let Err(error) = names.select_declaring_provider(vm) {
                return crate::command::completion_from_cmd_error(vm, error);
            }
            let retired = vm.oo.classes.insert(self.target, Class::default()).unwrap();
            drop(retired);
            match names.read(vm, &self.original) {
                Ok(_) => ok(Value::string("UNREACHED")),
                Err(error) => crate::command::completion_from_cmd_error(vm, error),
            }
        })
    }
}
#[test]
fn original_oo_name_declaring_facet_comes_from_the_genuine_entered_method_chain() {

    // Selected live chain plus real metadata replacement; no Native Class lifetime claim.
    let (mut vm, target, formatter) = provided();
    let hits = Rc::new(Cell::new(0));
    vm.register_command(
        "replace_oo_declaring_facet",
        Command::Native(Rc::new(ReplaceDeclaringFacet {
            target,
            hits: Rc::clone(&hits),
            original: Value::new_native_string_bytes(b"private".as_slice()),
        })),
    );
    assert_eq!(
        vm.invoke_command(
            "oo::define",
            &[
                Value::string("OriginalOOName"),
                Value::string("method"),
                Value::string("inspect"),
                Value::empty(),
                Value::string("replace_oo_declaring_facet"),
            ]
        )
        .code,
        Code::Ok
    );
    assert_eq!(
        vm.invoke_command(
            "OriginalOOName",
            &[Value::string("create"), Value::string("OOProviderObject")]
        )
        .code,
        Code::Ok
    );
    let receiver = resolve_object_value(&mut vm, &Value::string("OOProviderObject"))
        .unwrap()
        .unwrap();
    let object = Rc::clone(&vm.oo.objects[&receiver].cached_name);
    let namespace = vm.oo.objects[&receiver].namespace_token;
    let result = vm.invoke_command("OOProviderObject", &[Value::string("inspect")]);
    assert_eq!(result.code, Code::Error);
    assert_eq!(hits.get(), 1);
    assert_eq!(formatter.queries.get(), 0);
    assert_eq!(
        vm.execution_refusal,
        Some(NativeExecutionError::ValueAccessRefusal(
            NativeValueAccessRefusal::CommandProtocolUnavailable(
                "original OO Name declaring facet"
            )
        ))
    );
    assert!(Rc::ptr_eq(&object, &vm.oo.objects[&receiver].cached_name));
    assert_eq!(vm.oo.objects[&receiver].namespace_token, namespace);
    assert!(vm.oo.call_stack.is_empty());
}

struct WithdrawDeclaringStep {
    hits: Rc<Cell<usize>>,
}
impl NativeCommand for WithdrawDeclaringStep {
    fn invoke(&self, vm: &mut Vm, _: &[Value]) -> Completion<Value> {
        let activation = vm.native_oo_variable_activation();
        let frame = vm
            .oo
            .call_stack
            .iter_mut()
            .rev()
            .find(|frame| frame.activation == Some(activation))
            .expect("genuine selected method activation");
        assert!(frame.chain.get(frame.index).is_some());
        let receiver = frame.object;
        frame.index = frame.chain.len();
        self.hits.set(self.hits.get() + 1);
        super::super::builtin_variable(
            vm,
            receiver,
            &[Value::new_native_string_bytes(b"private".as_slice())],
        )
    }
}

#[test]
fn original_oo_name_present_method_requires_its_actual_declaring_step() {

    // Actual entered method, then software index withdrawal; no Native Class lifetime claim.
    let (mut vm, _target, formatter) = provided();
    let hits = Rc::new(Cell::new(0));
    vm.register_command(
        "withdraw_oo_declaring_step",
        Command::Native(Rc::new(WithdrawDeclaringStep {
            hits: Rc::clone(&hits),
        })),
    );
    assert_eq!(
        vm.invoke_command(
            "oo::define",
            &[
                Value::string("OriginalOOName"),
                Value::string("method"),
                Value::string("inspect"),
                Value::empty(),
                Value::string("withdraw_oo_declaring_step"),
            ]
        )
        .code,
        Code::Ok,
    );
    assert_eq!(
        vm.invoke_command(
            "OriginalOOName",
            &[Value::string("create"), Value::string("OOStepObject"),]
        )
        .code,
        Code::Ok,
    );
    let completion = vm.invoke_command("OOStepObject", &[Value::string("inspect")]);
    assert_eq!(completion.code, Code::Error);
    assert_eq!(hits.get(), 1);
    assert_eq!(formatter.queries.get(), 0);
    assert_eq!(
        vm.execution_refusal,
        Some(NativeExecutionError::ValueAccessRefusal(
            NativeValueAccessRefusal::CommandProtocolUnavailable("original OO Name declaring step"),
        ))
    );
    assert!(vm.oo.call_stack.is_empty());
}
