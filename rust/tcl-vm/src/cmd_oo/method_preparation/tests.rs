// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual VM preparation owners and callbacks; no Native Method/Class grant.
use super::*;
use crate::cmd_oo::{Class, Code, Completion, Value, ok, resolve_object_value, run_step, teardown};
use crate::command::{Command, NativeCommand};
use std::cell::{Cell, RefCell};
use tcl_platform::{
    NativeIntegerFormatter, NativeIntegerFormatterBuild, NativeIntegerFormatterUnavailable,
    NativeIntegerKind,
};
use tcl_registry::native_tcloo_method_definition::NativeTclooMethodReportRole;
use tcl_runtime_api::NativeExecutionError;
use tcl_syntax::{
    number::Number, raw_string::NativeValueAccessRefusal, scalar_getter::NativeScalarCache,
};

fn provided() -> (Vm, OoId, OoId, ReachedMethodChain) {
    let mut vm =
        crate::native_fixture::interpreter(crate::environment::profile_for_dialect("tcl9.1"));
    let result = vm.eval_source("oo::class create PreparationC {method m {} {return ORIGINAL}}; PreparationC create preparationObject").unwrap();
    assert_eq!(result.code, Code::Ok);
    let target = resolve_object_value(&mut vm, &Value::string("preparationObject"))
        .unwrap()
        .unwrap();
    let provider = vm.oo.objects[&target].class;
    let chain = super::super::native_method_cache::retain_owners(
        &vm.oo,
        vec![Step {
            method_owner: None,
            provider,
            is_object: false,
        }],
        "m",
        NativeTclooMethodReportRole::Named,
    )
    .into();
    (vm, target, provider, chain)
}
fn invoke(vm: &mut Vm, target: OoId, chain: &ReachedMethodChain) -> Completion<Value> {
    run_step(
        vm,
        target,
        chain,
        0,
        "m".into(),
        &[],
        true,
        Value::string("preparationObject"),
        vec![Value::string("preparationObject"), Value::string("m")],
        Vec::new(),
    )
}
struct Body(Rc<Cell<usize>>);
impl NativeCommand for Body {
    fn invoke(&self, _: &mut Vm, _: &[Value]) -> Completion<Value> {
        self.0.set(self.0.get() + 1);
        ok(Value::string("BODY"))
    }
}
struct Formatter {
    fail: bool,
    owner: Weak<RefCell<super::super::Method>>,
    calls: Cell<usize>,
}
impl NativeIntegerFormatter for Formatter {
    fn build(&self) -> NativeIntegerFormatterBuild {
        NativeIntegerFormatterBuild {
            version: [9, 1, 0, 0],
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
        let owner = self.owner.upgrade().unwrap();
        assert!(
            owner.try_borrow_mut().is_ok(),
            "actual preparation holds no Method borrow"
        );
        owner.borrow_mut().private = true;
        owner.borrow_mut().exported = false;
        if self.fail {
            Err(NativeIntegerFormatterUnavailable::Operation)
        } else {
            Ok(value.to_string().into_bytes())
        }
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
#[test]
fn original_method_preparation_string_callback_allows_same_record_flags_and_keeps_host_before_cache_or_body()
 {

    // Actual numeric formatter/VM body callback; no Native procedure callback equivalence.
    for fail in [false, true] {
        let (mut vm, target, _, chain) = provided();
        let calls = Rc::new(Cell::new(0));
        vm.register_command("42", Command::Native(Rc::new(Body(Rc::clone(&calls)))));
        let formatter = Rc::new(Formatter {
            fail,
            owner: Rc::downgrade(chain[0].method_owner.as_ref().unwrap()),
            calls: Cell::new(0),
        });
        vm.set_host(Rc::new(Host {
            actual: vm.host_rc(),
            formatter: Rc::clone(&formatter),
        }));
        let original = Value::from_native_scalar_cache(
            NativeScalarCache::Number(Number::Int(42)),
            None,
            vm.actual_native_invocation_dialect(),
        )
        .unwrap();
        let owner = chain[0].method_owner.as_ref().unwrap();
        {
            let mut method = owner.borrow_mut();
            method.body_src = original.clone();
            method.compiled_body = None;
        }
        let outer =
            crate::interp::native_operation_currency::NativeOperationScope::enter(&mut vm).unwrap();
        let result = invoke(&mut vm, target, &chain);
        assert_eq!(formatter.calls.get(), 1);
        assert!(owner.borrow().private);
        assert!(!owner.borrow().exported);
        assert!(vm.oo.call_stack.is_empty());
        if fail {
            assert_eq!(result.code, Code::Error);
            let first = vm.execution_refusal.clone().unwrap();
            assert!(matches!(
                first,
                NativeExecutionError::ValueAccessRefusal(NativeValueAccessRefusal::StringAccess(_))
            ));
            assert!(owner.borrow().compiled_body.is_none());
            assert_eq!(calls.get(), 0);
            assert_eq!(invoke(&mut vm, target, &chain).code, Code::Error);
            assert_eq!(formatter.calls.get(), 1);
            assert_eq!(vm.execution_refusal.as_ref(), Some(&first));
            assert_eq!(
                outer.check(&mut vm).unwrap_err().native_execution_refusal(),
                Some(&first)
            );
        } else {
            assert_eq!(result.code, Code::Ok);
            assert_eq!(
                result.result.resident_string_bytes().unwrap().as_ref(),
                b"BODY"
            );
            assert!(owner.borrow().compiled_body.is_some());
            assert_eq!(calls.get(), 1);
            outer.check(&mut vm).unwrap();
        }
    }
}
#[test]
fn original_method_preparation_requires_original_receiver_and_declaring_facet_after_reached_work() {

    // Actual withdrawal/fresh Class birth between the same owner's entered stages.
    for facet in [false, true] {
        let (mut vm, target, provider, chain) = provided();
        let outer =
            crate::interp::native_operation_currency::NativeOperationScope::enter(&mut vm).unwrap();
        let owners = Rc::strong_count(chain[0].method_owner.as_ref().unwrap());
        let preparation = OriginalMethodPreparation::enter(&mut vm, target, &chain, 0).unwrap();
        assert_eq!(
            Rc::strong_count(chain[0].method_owner.as_ref().unwrap()),
            owners
        );
        if facet {
            let retired = vm.oo.classes.insert(provider, Class::default()).unwrap();
            drop(retired);
        } else {
            teardown(&mut vm, target);
            assert!(!vm.oo.objects.contains_key(&target));
        }
        let pending = Value::new_native_string_bytes(b"pending".as_slice());
        let observation = pending.native_lifetime_lease();
        let error = preparation.settle(&mut vm, Ok(pending)).unwrap_err();
        assert!(error.is_host());
        assert!(!observation.value().native_object_is_live());
        let first = vm.execution_refusal.clone().unwrap();
        assert_eq!(
            preparation
                .check(&mut vm)
                .unwrap_err()
                .native_execution_refusal(),
            Some(&first)
        );
        assert_eq!(
            outer.check(&mut vm).unwrap_err().native_execution_refusal(),
            Some(&first)
        );
        assert!(vm.oo.call_stack.is_empty());
    }
}
#[test]
fn original_method_preparation_keeps_entered_record_after_table_withdrawal_and_fresh_same_name() {

    // The real entered chain owns its record; no mutable table relookup is permitted.
    let (mut vm, target, provider, chain) = provided();
    let old = Rc::downgrade(chain[0].method_owner.as_ref().unwrap());
    let preparation = OriginalMethodPreparation::enter(&mut vm, target, &chain, 0).unwrap();
    let result = vm.eval_source("oo::define PreparationC deletemethod m; oo::define PreparationC method m {} {return REPLACEMENT}; oo::define PreparationC variable legal").unwrap();
    assert_eq!(result.code, Code::Ok);
    assert!(!Weak::ptr_eq(
        &old,
        &Rc::downgrade(&vm.oo.classes[&provider].methods.owner("m").unwrap())
    ));
    preparation.settle(&mut vm, Ok(())).unwrap();
    preparation.finish(&mut vm).unwrap();
    assert_eq!(
        old.strong_count(),
        1,
        "only the real entered chain owns the removed record"
    );
    let result = invoke(&mut vm, target, &chain);
    assert_eq!(result.code, Code::Ok);
    assert_eq!(
        result.result.resident_string_bytes().unwrap().as_ref(),
        b"ORIGINAL"
    );
    assert!(vm.execution_refusal.is_none());
}
struct ChangeCaller(Rc<Cell<usize>>);
impl NativeCommand for ChangeCaller {
    fn invoke(&self, vm: &mut Vm, _: &[Value]) -> Completion<Value> {
        let frame = super::super::native_context::current_method(vm).unwrap();
        let object = frame.object;
        let index = frame.index;
        let chain = frame.chain.clone();
        let preparation = OriginalMethodPreparation::enter(vm, object, &chain, index).unwrap();
        self.0.set(self.0.get() + 1);
        let position = vm.oo.call_stack.len() - 1;
        vm.oo.call_stack[position].index = usize::MAX;
        let error = preparation.settle(vm, Ok(())).unwrap_err();
        vm.oo.call_stack[position].index = index;
        crate::command::completion_from_tcl_error(vm, error)
    }
}
#[test]
fn original_method_preparation_checks_genuine_caller_frame_before_new_activation() {

    let (mut vm, target, _, _) = provided();
    let calls = Rc::new(Cell::new(0));
    vm.register_command(
        "change_preparation_caller",
        Command::Native(Rc::new(ChangeCaller(Rc::clone(&calls)))),
    );
    assert_eq!(
        vm.eval_source("oo::define PreparationC method m {} {change_preparation_caller}")
            .unwrap()
            .code,
        Code::Ok
    );
    let outer =
        crate::interp::native_operation_currency::NativeOperationScope::enter(&mut vm).unwrap();
    let result = vm.invoke_command("preparationObject", &[Value::string("m")]);
    assert_eq!(result.code, Code::Error);
    assert_eq!(calls.get(), 1);
    let first = vm.execution_refusal.clone().unwrap();
    assert_eq!(
        first,
        NativeExecutionError::ValueAccessRefusal(
            NativeValueAccessRefusal::CommandProtocolUnavailable(
                "original OO preparation caller frame"
            )
        )
    );
    assert!(vm.oo.call_stack.is_empty());
    assert!(vm.oo.objects.contains_key(&target));
    assert_eq!(
        outer.check(&mut vm).unwrap_err().native_execution_refusal(),
        Some(&first)
    );
}
#[test]
fn original_method_preparation_retains_reached_host_before_later_facet_or_context_refusal() {

    let (mut vm, target, provider, chain) = provided();
    let preparation = OriginalMethodPreparation::enter(&mut vm, target, &chain, 0).unwrap();
    let first = NativeExecutionError::ValueAccessRefusal(
        NativeValueAccessRefusal::CommandProtocolUnavailable(
            "reached original preparation provider",
        ),
    );
    let retired = vm.oo.classes.insert(provider, Class::default()).unwrap();
    drop(retired);
    vm.set_pack_facts(vec![tcl_runtime_api::PackFactStamp {
        pack: "preparation".into(),
        content_hash: 7,
        vocabulary_version: "actual".into(),
        overlay_generation: 0,
        evaluator_revision: 0,
    }]);
    vm.set_pack_facts(Vec::new());
    let error = preparation
        .settle::<()>(
            &mut vm,
            Err(TclError::from_execution_failure(first.clone())),
        )
        .unwrap_err();
    assert!(error.is_host());
    assert_eq!(vm.execution_refusal.as_ref(), Some(&first));
    assert_eq!(
        preparation
            .check(&mut vm)
            .unwrap_err()
            .native_execution_refusal(),
        Some(&first)
    );
}
