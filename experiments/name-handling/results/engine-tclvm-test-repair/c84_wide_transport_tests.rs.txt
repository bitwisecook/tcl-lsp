// SPDX-License-Identifier: AGPL-3.0-or-later
//! Real bridge recursive import and Host recovery retain actual ingress.
use super::*;
use tcl_engine_api::{HostArgumentView, NativeScalarCache as Carrier, OriginalObjectResult as R};
fn native() -> Vm {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.4").unit_profile();
    let mut vm = Vm::with_native_core(
        Box::new(std::io::sink()),
        Rc::new(tcl_vm::host_native::NativeHost::new()),
        profile,
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    vm.set_compiler(Box::new(
        tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
    ));
    vm
}
#[test]
fn original_c84_wide_recursive_bridge_retains_actual_ingress_for_plain_and_resident_members() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut vm = native();
    let dialect = vm.native_scalar_carrier_dialect();
    let carrier = Value::List(
        vec![
            Value::NativeScalar(Carrier::Tcl84Wide(17)),
            Value::Resident {
                value: Rc::new(Value::NativeScalar(Carrier::Tcl84Wide(4_294_967_296))),
                string: Rc::from(b"4294967296".as_slice()),
                storage: tcl_engine_api::NativeStringStorageIdentity::Allocated,
            },
        ]
        .into(),
    );
    assert!(to_vm_value(&carrier, dialect).is_err());
    let original = vm.capture_original_scalar_ingress().unwrap();
    let imported = {
        let actual = ActualScalarImport {
            vm: RefCell::new(&mut vm),
            original: &original,
        };
        to_vm_value_using(&carrier, dialect, Some(&actual)).unwrap()
    };
    let children = imported
        .native_object_list_elements(dialect.native_string_protocol().unwrap())
        .unwrap();
    assert_eq!(children.len(), 2);
    assert_eq!(
        children[0].native_scalar_descriptor_origin(),
        Some(tcl_dialect::TclVersion::V8_4)
    );
    assert!(children[0].resident_string_bytes().is_none());
    assert!(matches!(
        from_vm_object_value(&children[0]).unwrap(),
        Value::NativeScalar(Carrier::Tcl84Wide(17))
    ));
    assert_eq!(
        children[1].resident_string_bytes().unwrap().as_ref(),
        b"4294967296"
    );
    assert!(
        matches!(from_vm_object_value(&children[1]).unwrap(), Value::Resident { value, .. }
        if matches!(*value, Value::NativeScalar(Carrier::Tcl84Wide(4_294_967_296))))
    );
    drop(children);
    drop(imported);
    vm.finish_original_scalar_ingress(original).unwrap();
}
struct Withdraw(u8);
impl NativeCommand for Withdraw {
    fn invoke(&self, vm: &mut Vm, _: &[tcl_vm::Value]) -> Completion<tcl_vm::Value> {
        if self.0 == 1 {
            vm.set_dialect_profile(
                tcl_registry::model::ingress::resolve_environment("tcl8.6").unit_profile(),
            );
            vm.set_dialect_profile(
                tcl_registry::model::ingress::resolve_environment("tcl8.4").unit_profile(),
            );
            Completion::new(Code::Ok, tcl_vm::Value::empty(), tcl_vm::Value::empty())
        } else {
            vm.refuse_host_command("original scalar callback first Host".into())
        }
    }
}
struct WideReply(u8);
impl HostCommand for WideReply {
    fn argument_view(&self) -> HostArgumentView {
        HostArgumentView::NativeObjectSnapshots
    }
    fn invoke(&self, _: &[Value]) -> Result<HostOutcome, EngineError> {
        Ok(Value::NativeScalar(Carrier::Tcl84Wide(4_294_967_296)).into())
    }
    fn invoke_with_registrar(
        &self,
        registrar: &mut dyn CommandRegistrar,
        args: &[Value],
    ) -> Result<HostOutcome, EngineError> {
        if self.0 != 0 {
            // A real nested callback may fail; its actual cause remains owned
            // by the outer receipt before this new descriptive reply is recovered.
            let _ = registrar.eval_in_invocation("originalScalarWithdraw");
        }
        self.invoke(args)
    }
}
#[test]
fn original_c84_wide_host_reply_uses_pre_callback_receipt_and_finishes_only_the_real_outer_boundary()
 {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for mode in [0, 1, 2] {
        let mut vm = native();
        vm.try_register_native_command("originalScalarWithdraw", Rc::new(Withdraw(mode)))
            .unwrap();
        let original = vm.capture_original_scalar_ingress().unwrap();
        let shim = HostCommandShim {
            command: Rc::new(WideReply(mode)),
            host_commands: Rc::new(RefCell::new(Vec::new())),
        };
        let completion = shim.invoke(&mut vm, &[]);
        if mode == 0 {
            assert_eq!(completion.code, Code::Ok);
            assert!(matches!(
                from_vm_object_value(&completion.result).unwrap(),
                Value::NativeScalar(Carrier::Tcl84Wide(4_294_967_296))
            ));
            assert!(completion.result.resident_string_bytes().is_none());
            drop(completion);
            vm.finish_original_scalar_ingress(original).unwrap();
        } else {
            assert_eq!(completion.code, Code::Error);
            assert_eq!(completion.result.native_scalar_descriptor_origin(), None);
            let first = vm.check_original_scalar_ingress(&original).unwrap_err();
            if mode == 2 {
                assert!(
                    matches!(&first, tcl_runtime_api::NativeExecutionError::HostCommandRefusal(cause)
                    if cause.reason == "original scalar callback first Host")
                );
            } else {
                assert!(matches!(
                    &first,
                    tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(_)
                ));
            }
            drop(completion);
            assert_eq!(vm.finish_original_scalar_ingress(original), Err(first));
            let next = vm.capture_original_scalar_ingress().unwrap();
            let accepted = vm
                .import_original_native_scalar(&next, &Carrier::Tcl84Wide(17), None)
                .unwrap();
            assert_eq!(
                accepted.native_scalar_descriptor_origin(),
                Some(tcl_dialect::TclVersion::V8_4)
            );
            drop(accepted);
            vm.finish_original_scalar_ingress(next).unwrap();
        }
    }
    // The original-object fresh reply path uses the same recursive adapter.
    let mut vm = native();
    let original = vm.capture_original_scalar_ingress().unwrap();
    let dialect = vm.native_scalar_carrier_dialect();
    let identity = vm.native_object_bridge_identity();
    let reply = R::List {
        items: vec![R::Value(Value::NativeScalar(Carrier::Tcl84Wide(17)))],
        canonical: false,
    };
    let recovered = {
        let actual = ActualScalarImport {
            vm: RefCell::new(&mut vm),
            original: &original,
        };
        recover_original_result_in(
            &reply,
            dialect,
            identity,
            &mut std::collections::BTreeMap::new(),
            Some(&actual),
        )
        .unwrap()
    };
    let children = recovered
        .native_object_list_elements(dialect.native_string_protocol().unwrap())
        .unwrap();
    assert_eq!(
        children[0].native_scalar_descriptor_origin(),
        Some(tcl_dialect::TclVersion::V8_4)
    );
    assert!(children[0].resident_string_bytes().is_none());
    drop(children);
    drop(recovered);
    vm.finish_original_scalar_ingress(original).unwrap();
}
