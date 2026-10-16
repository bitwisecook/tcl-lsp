// SPDX-License-Identifier: AGPL-3.0-or-later
//! Software controls for entered lookup, selected binding and reached cache retirement.

use super::*;
use crate::interp::ok;
use std::cell::{Cell, RefCell};
use tcl_platform::{
    NativeIntegerFormatter, NativeIntegerFormatterBuild, NativeIntegerFormatterUnavailable,
    NativeIntegerKind,
};
use tcl_runtime_api::NativeExecutionError;
use tcl_syntax::{
    native_string::NativeStringProtocol,
    number::Number,
    raw_string::{NativeStringAccessError, NativeValueAccessRefusal},
    scalar_getter::NativeScalarCache,
    value::ValueError,
};

#[derive(Default)]
struct Formatter {
    queries: Cell<usize>,
    builds: Cell<usize>,
    withdraw_on_query: RefCell<Option<Rc<RefCell<NativeNameWorld>>>>,
    retire_caller_on_query: Cell<bool>,
    calls: Cell<usize>,
    mode: Cell<u8>,
    caller: RefCell<Option<Value>>,
    counts: RefCell<Vec<usize>>,
}
impl NativeIntegerFormatter for Formatter {
    fn build(&self) -> NativeIntegerFormatterBuild {
        self.builds.set(self.builds.get() + 1);
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
        self.calls.set(self.calls.get() + 1);
        self.counts.borrow_mut().push(
            self.caller
                .borrow()
                .as_ref()
                .unwrap()
                .native_object_reference_count(),
        );
        if self.mode.get() == 1 {
            drop(self.caller.borrow_mut().take());
        }
        if self.mode.get() == 2 {
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
    fn native_integer_formatter(&self) -> Option<&dyn NativeIntegerFormatter> {
        self.formatter.queries.set(self.formatter.queries.get() + 1);
        if let Some(world) = self.formatter.withdraw_on_query.borrow_mut().take() {
            world.borrow_mut().retire();
        }
        if self.formatter.retire_caller_on_query.replace(false) {
            drop(self.formatter.caller.borrow_mut().take());
        }
        Some(&*self.formatter)
    }
}
fn actual(name: &str) -> Vm {
    crate::native_fixture::core(crate::environment::profile_for_dialect(name))
}
fn command(vm: &mut Vm, name: &[u8]) -> String {
    fn worker(_: &mut Vm, _: &[Value]) -> crate::Completion<Value> {
        ok(Value::empty())
    }
    vm.register_command_in_slot(
        CommandSlot {
            namespace: super::super::ROOT_NS,
            simple: name.into(),
        },
        super::super::Command::Builtin(worker),
    )
}
fn preserved_result(vm: &mut Vm) -> NativeObjectLifetimeLease {
    vm.adopt_native_interp_result(Value::new_native_string_bytes(b"BEFORE".as_slice()))
        .unwrap()
}
fn unchanged_result(vm: &Vm, prior: &NativeObjectLifetimeLease) {
    assert!(
        vm.with_native_interp_result(|value| value.is_same_object(prior.value()))
            .unwrap()
    );
    assert_eq!(
        prior.value().resident_string_bytes().unwrap().as_ref(),
        b"BEFORE"
    );
}

#[test]
fn original_lookup_checks_reached_string_failure_before_cache_or_miss() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Supplied formatter and header withdrawal are actual software hooks, not Native callback equivalence.
    for mode in 0..3 {
        let mut vm = actual("tcl8.4");
        let formatter = Rc::new(Formatter::default());
        vm.set_host(Rc::new(Host {
            actual: vm.host_rc(),
            formatter: Rc::clone(&formatter),
        }));
        command(&mut vm, i64::MIN.to_string().as_bytes());
        let caller = Value::from_native_scalar_cache(
            NativeScalarCache::Number(Number::Int(i64::MIN)),
            None,
            vm.actual_native_invocation_dialect(),
        )
        .unwrap();
        let original = caller.native_lifetime_lease();
        *formatter.caller.borrow_mut() = Some(caller);
        formatter.mode.set(mode);
        let prior = preserved_result(&mut vm);
        let reached = vm.native_command_from_original(original.value());
        assert_eq!(formatter.calls.get(), 1);
        assert_eq!(&*formatter.counts.borrow(), &[1]);
        if mode == 0 {
            assert!(reached.unwrap().is_some());
            assert!(original.value().native_command_name_cache().is_some());
            assert_eq!(original.value().native_object_reference_count(), 1);
        } else {
            let error = reached.unwrap_err();
            let expected = if mode == 1 {
                NativeExecutionError::ValueAccessRefusal(
                    NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "retired native object header",
                    ),
                )
            } else {
                NativeExecutionError::ValueAccessRefusal(NativeValueAccessRefusal::StringAccess(
                    NativeStringAccessError::Unavailable(
                        tcl_syntax::native_string::NativeStringUnavailable::StringUpdater,
                    ),
                ))
            };
            assert_eq!(error.native_execution_refusal(), Some(&expected));
            assert_eq!(vm.execution_refusal.as_ref(), Some(&expected));
            assert!(original.value().native_command_name_cache().is_none());
            assert!(original.value().resident_string_bytes().is_none());
            assert_eq!(original.value().native_object_is_live(), mode != 1);
        }
        unchanged_result(&vm, &prior);
    }
}

#[test]
fn original_lookup_cache_hit_preserves_first_cause_after_context_change_and_restore() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // All six genuine profile owners retain the same Scope; no fresh receipt repairs an ABA.
    for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut vm = actual(name);
        command(&mut vm, b"head");
        let original = Value::new_native_string_bytes(b"head".as_slice());
        assert!(
            vm.native_command_from_original(&original)
                .unwrap()
                .is_some()
        );
        let before = original.native_object_snapshot();
        let scope = NativeOperationScope::enter(&mut vm).unwrap();
        let host = vm.host_rc();
        vm.set_host(Rc::new(crate::host_native::NativeHost::new()));
        vm.set_host(host);
        let first = scope.check_native(&mut vm).unwrap_err();
        let error = vm.native_command_from_original(&original).unwrap_err();
        assert_eq!(error.native_execution_refusal(), Some(&first));
        assert_eq!(vm.execution_refusal.as_ref(), Some(&first));
        assert_eq!(original.native_object_snapshot(), before);
        assert_eq!(original.native_object_reference_count(), 1);
        let later = Value::new_native_string_bytes(b"absent".as_slice());
        let again = vm.namespace_object_lookup(&later).unwrap_err();
        assert_eq!(again.native_execution_refusal(), Some(&first));
        assert!(later.native_namespace_name_cache().is_none());
    }
}

#[test]
fn original_selected_command_refuses_recreated_slot_after_actual_primary_retirement() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // A software stage checker observes genuine List-member retirement and performs actual table replacement.
    let mut vm = actual("tcl9.0");
    let key = command(&mut vm, b"head");
    let probe = Value::new_native_string_bytes(b"head".as_slice());
    vm.native_command_from_original(&probe).unwrap().unwrap();
    let cache = probe.native_command_name_cache().unwrap();
    let member = Value::new_native_string_bytes(b"head".as_slice());
    let observed = member.native_lifetime_lease();
    let original = Value::native_list_constructor(
        vec![member],
        NativeStringProtocol::C(tcl_dialect::TclVersion::V9_0),
    );
    assert_eq!(
        vm.native_name_operand_bytes_checked(&original)
            .unwrap()
            .as_ref(),
        b"head"
    );
    let resident = original.resident_string_bytes().unwrap();
    let entered = OriginalLookup::enter(&mut vm, super::super::ROOT_NS, &original).unwrap();
    let selected = entered.command(&mut vm, &key).unwrap();
    let mut stages = Vec::new();
    let dialect = vm.actual_native_invocation_dialect();
    let reached = original.install_native_command_name_cache_checked(cache, dialect, &mut || {
        stages.push(observed.value().native_object_reference_count());
        if !observed.value().native_object_is_live() {
            command(&mut vm, b"head");
        }
        selected.check(&mut vm, &entered)
    });
    let error = entered.finish(&mut vm, reached).unwrap_err();
    let expected = CmdError::from(ValueError::CommandProtocolUnavailable(
        "original lookup selected command binding",
    ));
    assert_eq!(
        error.native_execution_refusal(),
        expected.native_execution_refusal()
    );
    assert_eq!(
        vm.execution_refusal.as_ref(),
        expected.native_execution_refusal()
    );
    assert_eq!(stages, vec![1, 0]);
    assert!(!observed.value().native_object_is_live());
    assert!(original.native_command_name_cache().is_none());
    assert!(Rc::ptr_eq(
        &resident,
        &original.resident_string_bytes().unwrap()
    ));
    assert_ne!(
        vm.visible_command_generation(&key),
        Some(selected.generation())
    );
    assert_eq!(original.native_object_reference_count(), 1);
}

#[test]
fn original_jim_cache_adoption_acquires_namespace_role_after_old_primary_release() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual VM namespace references are software stage observations, not Native Jim pointer-count equivalence.
    let mut vm = actual("jim");
    let key = command(&mut vm, b"head");
    let original = Value::new_native_string_bytes(b"head".as_slice());
    vm.native_jim_command_from_original_at(super::super::ROOT_NS, &original)
        .unwrap()
        .unwrap();
    let token = original
        .with_jim_command_cache(|cache| cache.token)
        .unwrap();
    let epoch = vm.name_world.borrow().jim_procedure_epoch;
    let namespace = vm
        .with_jim_current_namespace(|value| Ok(value.native_lifetime_lease()))
        .unwrap();
    let before = namespace.value().native_object_reference_count();
    let entered = OriginalLookup::enter(&mut vm, super::super::ROOT_NS, &original).unwrap();
    assert_eq!(namespace.value().native_object_reference_count(), before);
    let selected = entered.command(&mut vm, &key).unwrap();
    let interpreter = vm.native_interpreter_identity();
    let dialect = vm.actual_native_invocation_dialect();
    let mut counts = Vec::new();
    let reached = original.install_jim_command_cache_checked(
        dialect,
        || {
            Ok(crate::value::JimCommandCache {
                interpreter,
                epoch,
                token,
                namespace: namespace.value().clone(),
            })
        },
        &mut || {
            counts.push(namespace.value().native_object_reference_count());
            selected.check(&mut vm, &entered)
        },
    );
    entered.finish(&mut vm, reached).unwrap();
    assert_eq!(counts, vec![before, before - 1, before, before]);
    assert_eq!(namespace.value().native_object_reference_count(), before);
    assert_eq!(
        original.with_jim_command_cache(|cache| cache.token),
        Some(token)
    );
    assert_eq!(original.native_object_reference_count(), 1);
}

#[test]
fn shared_original_alias_consumer_preserves_prior_host_before_missing_name_presentation() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // The shared typed alias boundary retains a genuine prior cause rather than publishing a missing-name Guest error.
    let mut vm = actual("jim");
    let prior = preserved_result(&mut vm);
    let original = Value::new_native_string_bytes(b"missing".as_slice());
    let before = original.native_object_snapshot();
    let _completion = vm.refuse_host_command("FIRST_ALIAS_HOST".into());
    let first = vm.execution_refusal.clone().unwrap();
    let error = tcl_cmd_core::info::jim_original_alias(&mut vm, &original).unwrap_err();
    assert_eq!(error.native_execution_refusal(), Some(&first));
    assert_eq!(vm.execution_refusal.as_ref(), Some(&first));
    assert_eq!(original.native_object_snapshot(), before);
    assert_eq!(original.native_object_reference_count(), 1);
    unchanged_result(&vm, &prior);
}

#[test]
fn original_lookup_checks_namespace_world_after_the_reached_formatter_query() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // The supplied provider query withdraws the actual software world before String publication.
    for withdraw in [false, true] {
        let mut vm = actual("tcl8.4");
        let formatter = Rc::new(Formatter::default());
        vm.set_host(Rc::new(Host {
            actual: vm.host_rc(),
            formatter: Rc::clone(&formatter),
        }));
        command(&mut vm, i64::MIN.to_string().as_bytes());
        let caller = Value::from_native_scalar_cache(
            NativeScalarCache::Number(Number::Int(i64::MIN)),
            None,
            vm.actual_native_invocation_dialect(),
        )
        .unwrap();
        let original = caller.native_lifetime_lease();
        *formatter.caller.borrow_mut() = Some(caller);
        let prior = preserved_result(&mut vm);
        if withdraw {
            *formatter.withdraw_on_query.borrow_mut() = Some(Rc::clone(&vm.name_world));
        }
        let reached = vm.native_command_from_original(original.value());
        assert_eq!(formatter.queries.get(), 1);
        assert_eq!(original.value().native_object_reference_count(), 1);
        assert!(original.value().native_object_is_live());
        if withdraw {
            let error = reached.unwrap_err();
            let expected = NativeExecutionError::ValueAccessRefusal(
                NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original lookup namespace and world",
                ),
            );
            assert_eq!(error.native_execution_refusal(), Some(&expected));
            assert_eq!(vm.execution_refusal.as_ref(), Some(&expected));
            assert_eq!(formatter.builds.get(), 0);
            assert_eq!(formatter.calls.get(), 0);
            assert!(original.value().resident_string_bytes().is_none());
            assert!(original.value().native_command_name_cache().is_none());
        } else {
            assert!(reached.unwrap().is_some());
            assert!(formatter.builds.get() > 0);
            assert_eq!(formatter.calls.get(), 1);
            assert_eq!(&*formatter.counts.borrow(), &[1]);
            assert_eq!(
                original.value().resident_string_bytes().unwrap().as_ref(),
                i64::MIN.to_string().as_bytes(),
            );
            assert!(original.value().native_command_name_cache().is_some());
            assert!(vm.execution_refusal.is_none());
        }
        unchanged_result(&vm, &prior);
    }
}

fn enter_original_jim_namespace(vm: &mut Vm, original: &Value) -> NamespaceExecutionContext {
    let context = super::super::OriginalJimNamespaceContext::for_lambda(vm, original).unwrap();
    vm.push_call_frame(Some("original_lookup_holder".into()), Vec::new());
    vm.install_jim_procedure_namespace_owner(None, original.clone());
    vm.push_procedure_namespace_context(super::super::ProcedureNamespace::OriginalJim(Rc::clone(
        &context,
    )));
    NamespaceExecutionContext::OriginalJim(context)
}

#[test]
fn original_jim_lookup_holder_getter_checks_the_lookup_world_at_the_provider_query() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // The entered original Jim frame supplies its real holder without materialising a token.
    for withdraw in [false, true] {
        let mut vm = actual("jim");
        let formatter = Rc::new(Formatter::default());
        vm.set_host(Rc::new(Host {
            actual: vm.host_rc(),
            formatter: Rc::clone(&formatter),
        }));
        let holder = Value::int(42);
        let context = enter_original_jim_namespace(&mut vm, &holder);
        let original = Value::new_native_string_bytes(b"head".as_slice());
        let entered = OriginalLookup::enter_in_context(&mut vm, &context, &original).unwrap();
        let before = holder.native_object_reference_count();
        let namespaces = vm.name_world.borrow().ns_arena.len();
        assert!(
            entered
                .jim_namespace_original()
                .unwrap()
                .is_same_object(&holder)
        );
        assert!(holder.resident_string_bytes().is_none());
        let prior = preserved_result(&mut vm);
        if withdraw {
            *formatter.withdraw_on_query.borrow_mut() = Some(Rc::clone(&vm.name_world));
        }
        let reached = entered.read_jim_namespace(&mut vm);
        let reached = entered.finish(&mut vm, reached);
        assert_eq!(formatter.queries.get(), 1);
        assert_eq!(formatter.calls.get(), 0);
        assert_eq!(holder.native_object_reference_count(), before);
        assert_eq!(vm.name_world.borrow().ns_arena.len(), namespaces);
        assert!(context.resolved_token().is_none());
        assert_eq!(original.native_object_reference_count(), 1);
        if withdraw {
            let expected = NativeExecutionError::ValueAccessRefusal(
                NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original lookup namespace and world",
                ),
            );
            assert_eq!(
                reached.unwrap_err().native_execution_refusal(),
                Some(&expected)
            );
            assert_eq!(vm.execution_refusal.as_ref(), Some(&expected));
            assert!(holder.resident_string_bytes().is_none());
        } else {
            assert_eq!(reached.unwrap().as_ref(), b"42");
            assert_eq!(holder.resident_string_bytes().unwrap().as_ref(), b"42");
            assert!(vm.execution_refusal.is_none());
        }
        unchanged_result(&vm, &prior);
    }
}

#[test]
fn original_jim_materialisation_checks_the_independent_lookup_operand_before_string_or_token() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Retiring the separate operand leaves the original Jim context owner live.
    for retire in [false, true] {
        let mut vm = actual("jim");
        let formatter = Rc::new(Formatter::default());
        vm.set_host(Rc::new(Host {
            actual: vm.host_rc(),
            formatter: Rc::clone(&formatter),
        }));
        let holder = Value::int(42);
        let context = enter_original_jim_namespace(&mut vm, &holder);
        let caller = Value::new_native_string_bytes(b"head".as_slice());
        let original = caller.native_lifetime_lease();
        *formatter.caller.borrow_mut() = Some(caller);
        let entered =
            OriginalLookup::enter_in_context(&mut vm, &context, original.value()).unwrap();
        let before = holder.native_object_reference_count();
        let namespaces = vm.name_world.borrow().ns_arena.len();
        let prior = preserved_result(&mut vm);
        formatter.retire_caller_on_query.set(retire);
        let reached = entered.materialize_namespace(&mut vm);
        let reached = entered.finish(&mut vm, reached);
        assert_eq!(formatter.queries.get(), 1);
        assert_eq!(formatter.calls.get(), 0);
        assert_eq!(holder.native_object_reference_count(), before);
        assert_eq!(original.value().native_object_is_live(), !retire);
        assert_eq!(
            original.value().native_object_reference_count(),
            usize::from(!retire)
        );
        let NamespaceExecutionContext::OriginalJim(owner) = &context else {
            unreachable!();
        };
        assert!(owner.owner_is_current(&vm));
        if retire {
            let expected = NativeExecutionError::ValueAccessRefusal(
                NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "retired native object header",
                ),
            );
            assert_eq!(
                reached.unwrap_err().native_execution_refusal(),
                Some(&expected)
            );
            assert_eq!(vm.execution_refusal.as_ref(), Some(&expected));
            assert!(holder.resident_string_bytes().is_none());
            assert!(context.resolved_token().is_none());
            assert_eq!(vm.name_world.borrow().ns_arena.len(), namespaces);
        } else {
            let token = reached.unwrap();
            assert_eq!(context.resolved_token(), Some(token));
            assert_eq!(vm.current_ns_id(), token);
            assert_eq!(holder.resident_string_bytes().unwrap().as_ref(), b"42");
            assert_eq!(
                vm.name_world.borrow().jim_namespace_objects[&token].as_bytes(),
                b"42"
            );
            assert_eq!(vm.name_world.borrow().ns_arena.len(), namespaces + 1);
            assert!(vm.execution_refusal.is_none());
        }
        unchanged_result(&vm, &prior);
    }
}
