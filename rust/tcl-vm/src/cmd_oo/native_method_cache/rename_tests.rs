// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original VM rename record observations; C physical reference roles are separate.
use super::*;
use std::cell::Cell;
use tcl_platform::{
    NativeIntegerFormatter, NativeIntegerFormatterBuild, NativeIntegerFormatterUnavailable,
    NativeIntegerKind,
};
use tcl_runtime_api::NativeExecutionError;
use tcl_syntax::{
    number::Number, raw_string::NativeValueAccessRefusal, scalar_getter::NativeScalarCache,
};

struct Formatter {
    version: [i32; 4],
    calls: Cell<usize>,
    fail: Cell<bool>,
    source: RefCell<Option<Weak<RefCell<Method>>>>,
    observed_owners: Cell<usize>,
    change_flags: Cell<bool>,
}
impl NativeIntegerFormatter for Formatter {
    fn build(&self) -> NativeIntegerFormatterBuild {
        NativeIntegerFormatterBuild {
            version: self.version,
            sha256: [7; 32],
            long_bits: 64,
        }
    }
    fn format(
        &self,
        _: NativeIntegerKind,
        value: i64,
    ) -> Result<Vec<u8>, NativeIntegerFormatterUnavailable> {
        self.calls.set(self.calls.get() + 1);
        if let Some(source) = self.source.borrow().as_ref() {
            self.observed_owners.set(source.strong_count());
            if self.change_flags.get() {
                let owner = source.upgrade().unwrap();
                let mut current = owner.borrow_mut();
                current.private = true;
                current.exported = false;
            }
        }
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
    fn filesystem(&self) -> Option<&dyn tcl_platform::Filesystem> {
        self.actual.filesystem()
    }
    fn numeric_environment(&self) -> Option<&dyn tcl_platform::NumericEnvironment> {
        self.actual.numeric_environment()
    }
    fn native_integer_formatter(&self) -> Option<&dyn NativeIntegerFormatter> {
        Some(self.formatter.as_ref())
    }
}
fn source(vm: &mut Vm, script: &str) {
    let reached = vm.eval_source(script).unwrap();
    assert_eq!(reached.code, Code::Ok, "{}", reached.result.to_str());
}
fn provided(version: &str, class: bool) -> (Vm, OoId, Rc<Formatter>) {
    let mut vm =
        crate::native_fixture::interpreter(crate::environment::profile_for_dialect(version));
    source(
        &mut vm,
        "oo::class create C {method m {} {return ORIGINAL}}; C create obj; oo::objdefine obj method m {} {return OBJECT}",
    );
    let target = super::super::resolve_object_value(
        &mut vm,
        &Value::new_native_string_bytes(if class {
            b"C".as_slice()
        } else {
            b"obj".as_slice()
        }),
    )
    .unwrap()
    .unwrap();
    let formatter = Rc::new(Formatter {
        version: match version {
            "tcl8.6" => [8, 6, 18, 0],
            "tcl9.0" => [9, 0, 4, 0],
            "tcl9.1" => [9, 1, 0, 0],
            _ => unreachable!(),
        },
        calls: Cell::new(0),
        fail: Cell::new(false),
        source: RefCell::new(None),
        observed_owners: Cell::new(0),
        change_flags: Cell::new(false),
    });
    vm.set_host(Rc::new(Host {
        actual: vm.host_rc(),
        formatter: formatter.clone(),
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
fn table(vm: &Vm, class: bool, target: OoId) -> &MethodTable {
    if class {
        &vm.oo.classes[&target].methods
    } else {
        &vm.oo.objects[&target].methods
    }
}

#[test]
fn original_vm_rename_missing_source_does_not_reach_destination_string() {
    // naming.tcloo.original-method-rename-operands-and-records
    // docs/design/analysis/name-resolution-proofs/tcloo-original-method-rename-operands-and-records.md
    // Native source lookup order and the actual supplied formatter are distinct facts.
    for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        for class in [false, true] {
            let (mut vm, target, formatter) = provided(version, class);
            formatter.fail.set(true);
            let destination = numeric(&vm);
            let snapshot = destination.native_object_snapshot();
            let count = destination.native_object_reference_count();
            let epochs = vm.oo.native_methods.observation_epochs(target);
            let reached = rename_method(
                &mut vm,
                class,
                target,
                &[
                    Value::new_native_string_bytes(b"absent".as_slice()),
                    destination.clone(),
                ],
            );
            assert_eq!(reached.code, Code::Error);
            assert_eq!(reached.result.to_str().as_ref(), "method does not exist");
            assert_eq!(formatter.calls.get(), 0);
            assert!(destination.resident_string_bytes().is_none());
            assert_eq!(destination.native_object_snapshot(), snapshot);
            assert_eq!(destination.native_object_reference_count(), count);
            assert_eq!(vm.oo.native_methods.observation_epochs(target), epochs);
            assert!(table(&vm, class, target).contains_key("m"));
            assert!(vm.execution_refusal.is_none());
        }
    }
}

#[test]
fn original_vm_rename_destination_string_failure_keeps_source_and_first_cause() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Host formatter refusal is software evidence, not a measured Native callback mode.
    for class in [false, true] {
        let (mut vm, target, formatter) = provided("tcl8.6", class);
        let _original_scope =
            crate::interp::native_operation_currency::NativeOperationScope::enter(&mut vm).unwrap();
        let selected = OriginalMethodRenameSource::capture(&vm.oo, class, target, "m").unwrap();
        *formatter.source.borrow_mut() = Some(selected.owner.clone());
        formatter.fail.set(true);
        let prior = vm
            .adopt_native_interp_result(Value::new_native_string_bytes(b"BEFORE".as_slice()))
            .unwrap();
        let destination = numeric(&vm);
        let epochs = vm.oo.native_methods.observation_epochs(target);
        let reached = rename_method(
            &mut vm,
            class,
            target,
            &[
                Value::new_native_string_bytes(b"m".as_slice()),
                destination.clone(),
            ],
        );
        assert_eq!(reached.code, Code::Error);
        let first = vm.execution_refusal.clone().unwrap();
        assert!(matches!(
            first,
            NativeExecutionError::ValueAccessRefusal(NativeValueAccessRefusal::StringAccess(_))
        ));
        assert_eq!(formatter.calls.get(), 1);
        assert_eq!(
            formatter.observed_owners.get(),
            1,
            "only the real table owns the record during String conversion"
        );
        assert!(destination.resident_string_bytes().is_none());
        assert!(table(&vm, class, target).contains_key("m"));
        assert!(!table(&vm, class, target).contains_key("42"));
        assert_eq!(vm.oo.native_methods.observation_epochs(target), epochs);
        assert!(
            vm.with_native_interp_result(|value| value.is_same_object(prior.value()))
                .unwrap()
        );
        assert_eq!(
            vm.native_name_operand_bytes_checked(&Value::string("later"))
                .unwrap_err()
                .native_execution_refusal(),
            Some(&first)
        );
        assert_eq!(vm.execution_refusal, Some(first));
        assert_eq!(formatter.calls.get(), 1);
    }
}

#[test]
fn original_vm_rename_moves_same_record_and_observes_reached_payload_changes() {
    // naming.tcloo.original-method-rename-operands-and-records
    // docs/design/analysis/name-resolution-proofs/tcloo-original-method-rename-operands-and-records.md
    // Real RefCell record flags mutate inside the supplied formatter; no Native flag parity is inferred.
    for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        for class in [false, true] {
            let (mut vm, target, formatter) = provided(version, class);
            let selected = OriginalMethodRenameSource::capture(&vm.oo, class, target, "m").unwrap();
            *formatter.source.borrow_mut() = Some(selected.owner.clone());
            formatter.change_flags.set(true);
            let destination = numeric(&vm);
            let reached = rename_method(
                &mut vm,
                class,
                target,
                &[
                    Value::new_native_string_bytes(b"m".as_slice()),
                    destination.clone(),
                ],
            );
            assert_eq!(reached.code, Code::Ok);
            assert_eq!(formatter.calls.get(), 1);
            assert_eq!(formatter.observed_owners.get(), 1);
            assert!(!table(&vm, class, target).contains_key("m"));
            let (_, current) = table(&vm, class, target).get_key_value("42").unwrap();
            assert_eq!(Rc::as_ptr(current), selected.owner.as_ptr());
            assert!(current.borrow().private);
            assert!(!current.borrow().exported);
            assert_eq!(selected.owner.strong_count(), 1);
            assert_eq!(destination.resident_string_bytes().unwrap().as_ref(), b"42");
            assert!(vm.execution_refusal.is_none());
        }
    }
}

#[test]
fn original_vm_rename_source_receipt_refuses_deleted_and_reborn_slot() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // The same production receipt is checked after real public delete/recreate effects.
    // This is a software continuation control, not a Native destination callback claim.
    for class in [false, true] {
        let (mut vm, target, formatter) = provided("tcl8.6", class);
        let destination = numeric(&vm);
        let reached = original_name_operands::run_result(
            &mut vm,
            Some((class, target)),
            |vm, names| {
                let selected =
                    OriginalMethodRenameSource::capture(&vm.oo, class, target, "m").unwrap();
                assert_eq!(selected.owner.strong_count(), 1);
                source(
                    vm,
                    if class {
                        "oo::define C deletemethod m; oo::define C method m {} {return REBORN}"
                    } else {
                        "oo::objdefine obj deletemethod m; oo::objdefine obj method m {} {return REBORN}"
                    },
                );
                assert!(selected.owner.upgrade().is_none());
                let key = super::super::native_method_key_for_target(
                    vm,
                    &destination,
                    Some((class, target)),
                )?;
                assert_eq!(key, "42");
                names
                    .check_target(vm)
                    .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
                selected
                    .check_at(vm, names, "m")
                    .map_err(|error| crate::command::completion_from_cmd_error(vm, error))
            },
        );
        assert_eq!(reached.unwrap_err().code, Code::Error);
        assert_eq!(formatter.calls.get(), 1);
        assert!(table(&vm, class, target).contains_key("m"));
        assert!(!table(&vm, class, target).contains_key("42"));
        assert_eq!(
            vm.execution_refusal,
            Some(NativeExecutionError::ValueAccessRefusal(
                NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original OO rename source record"
                )
            ))
        );
    }
}

#[test]
fn original_vm_rename_record_receipt_keeps_context_aba_first_cause() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let (mut vm, target, formatter) = provided("tcl8.6", true);
    let reached = original_name_operands::run_result(&mut vm, Some((true, target)), |vm, names| {
        let selected = OriginalMethodRenameSource::capture(&vm.oo, true, target, "m").unwrap();
        let actual = vm.host_rc();
        vm.set_host(Rc::new(crate::host_native::NativeHost::new()));
        vm.set_host(actual);
        selected
            .check_at(vm, names, "m")
            .map_err(|error| crate::command::completion_from_cmd_error(vm, error))
    });
    assert_eq!(reached.unwrap_err().code, Code::Error);
    assert_eq!(
        vm.execution_refusal,
        Some(NativeExecutionError::ValueAccessRefusal(
            NativeValueAccessRefusal::CommandProtocolUnavailable(
                "entered VM original execution context"
            )
        ))
    );
    assert_eq!(formatter.calls.get(), 0);
    assert!(table(&vm, true, target).contains_key("m"));
}
