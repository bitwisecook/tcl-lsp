// SPDX-License-Identifier: AGPL-3.0-or-later
//! Neutral current-operation receipts for the actual selected interpreter.
//!
//! This owner records execution-context ingress, independently of command,
//! namespace, receiver, object, compiler and recipe authority. Its state parks
//! with the actual interpreter. Nested operations inherit the original receipt;
//! neither callback re-entry nor change-and-restore can issue it again.

use super::Vm;
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
use tcl_cmd_core::CmdError;
use tcl_runtime_api::native_compilation::NativeInterpreterIdentity;
use tcl_runtime_api::{NativeExecutionError, RuntimeContext};
use tcl_syntax::raw_string::NativeValueAccessRefusal;

pub(super) struct NativeOperationContext {
    generation: Cell<Option<u64>>,
    entered: RefCell<Vec<Weak<EnteredOperation>>>,
}
impl Default for NativeOperationContext {
    fn default() -> Self {
        Self {
            generation: Cell::new(Some(0)),
            entered: RefCell::new(Vec::new()),
        }
    }
}
impl NativeOperationContext {
    fn advance(&self) {
        self.generation.set(
            self.generation
                .get()
                .and_then(|generation| generation.checked_add(1)),
        );
    }
}

#[derive(Clone)]
pub(crate) struct NativeOperationCurrency {
    owner: Rc<NativeOperationContext>,
    generation: u64,
    interpreter: NativeInterpreterIdentity,
    context: RuntimeContext,
    profile: &'static tcl_dialect::DialectProfile,
    engine: Option<&'static tcl_dialect::DialectProfile>,
    host: Rc<dyn tcl_platform::Host>,
}
struct EnteredOperation {
    currency: NativeOperationCurrency,
}

/// Allocation-only scope ownership creates no native object/reference role.
#[derive(Clone)]
pub(crate) struct NativeOperationScope {
    entered: Rc<EnteredOperation>,
}

fn unavailable() -> NativeExecutionError {
    NativeExecutionError::ValueAccessRefusal(NativeValueAccessRefusal::CommandProtocolUnavailable(
        "entered VM original execution context",
    ))
}
fn retain_first(vm: &mut Vm, cause: NativeExecutionError) -> CmdError {
    CmdError::from_execution_refusal(vm.execution_refusal.get_or_insert(cause).clone())
}
impl NativeOperationCurrency {
    pub(crate) fn check(&self, vm: &mut Vm) -> Result<(), CmdError> {
        if let Some(first) = vm.execution_refusal.clone() {
            return Err(CmdError::from_execution_refusal(first));
        }
        if !Rc::ptr_eq(&self.owner, &vm.native_operation_context)
            || self.owner.generation.get() != Some(self.generation)
            || vm.native_interpreter_identity() != self.interpreter
            || vm.runtime_context() != &self.context
            || !std::ptr::eq(vm.dialect_profile(), self.profile)
            || vm.actual_engine_profile.map(std::ptr::from_ref)
                != self.engine.map(std::ptr::from_ref)
            || !Rc::ptr_eq(&vm.host_rc(), &self.host)
        {
            return Err(retain_first(vm, unavailable()));
        }
        Ok(())
    }
}
impl NativeOperationScope {
    pub(crate) fn enter(vm: &mut Vm) -> Result<Self, CmdError> {
        if let Some(first) = vm.execution_refusal.clone() {
            return Err(CmdError::from_execution_refusal(first));
        }
        let owner = Rc::clone(&vm.native_operation_context);
        // Weak scope records never keep a finished operation or Host alive.
        // No owner borrow is held while checking, querying Host, or dropping
        // an actual scope's retained host/context facets.
        let inherited = {
            let mut entered = owner.entered.borrow_mut();
            entered.retain(|record| record.strong_count() != 0);
            entered.last().and_then(Weak::upgrade)
        };
        let currency = if let Some(original) = inherited {
            original.currency.check(vm)?;
            original.currency.clone()
        } else {
            let generation = owner
                .generation
                .get()
                .ok_or_else(|| retain_first(vm, unavailable()))?;
            NativeOperationCurrency {
                owner: Rc::clone(&owner),
                generation,
                interpreter: vm.native_interpreter_identity(),
                context: vm.runtime_context().clone(),
                profile: vm.dialect_profile(),
                engine: vm.actual_engine_profile,
                host: vm.host_rc(),
            }
        };
        let entered = Rc::new(EnteredOperation { currency });
        owner.entered.borrow_mut().push(Rc::downgrade(&entered));
        Ok(Self { entered })
    }
    pub(crate) fn currency(&self) -> &NativeOperationCurrency {
        &self.entered.currency
    }
    pub(crate) fn check(&self, vm: &mut Vm) -> Result<(), CmdError> {
        self.currency().check(vm)
    }
}
impl Vm {
    pub(super) fn advance_native_operation_context(&self) {
        self.native_operation_context.advance();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Value;
    use tcl_runtime_api::Code;
    fn native() -> Vm {
        crate::native_fixture::interpreter(crate::environment::profile_for_dialect("tcl8.6"))
    }
    #[test]
    fn original_vm_context_receipt_survives_guest_table_and_same_input_policy_changes() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let mut vm = native();
        let scope = NativeOperationScope::enter(&mut vm).unwrap();
        let context = vm.runtime_context().clone();
        vm.pin_context(&context).unwrap();
        let host = vm.host_rc();
        vm.set_host(host);
        vm.set_pack_facts(Vec::new());
        let result = vm
            .eval_source("set ordinary VALUE; proc p {} {return RESULT}; rename p q; q")
            .unwrap();
        assert_eq!(result.code, Code::Ok);
        assert_eq!(result.result.resident_string_bytes().unwrap().as_ref(), b"RESULT");
        scope.check(&mut vm).unwrap();
        assert!(vm.execution_refusal.is_none());
    }
    #[test]
    fn original_vm_context_receipt_refuses_host_profile_compiler_and_pack_change_restore() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        for stage in 0..4 {
            let mut vm = native();
            let scope = NativeOperationScope::enter(&mut vm).unwrap();
            match stage {
                0 => {
                    let original = vm.host_rc();
                    vm.set_host(Rc::new(crate::host_native::NativeHost::new()));
                    vm.set_host(original);
                }
                1 => {
                    vm.set_dialect_profile(crate::environment::profile_for_dialect("tcl9.1"));
                    vm.set_dialect_profile(crate::environment::profile_for_dialect("tcl8.6"));
                }
                2 => {
                    vm.set_compiler(Box::new(
                        tcl_compiler::compile_service::BytecodeCompileService::for_profile(
                            crate::environment::profile_for_dialect("tcl8.6"),
                        ),
                    ));
                }
                _ => {
                    vm.set_pack_facts(vec![tcl_runtime_api::PackFactStamp {
                        pack: "actual-entered-pack".into(),
                        content_hash: 1,
                        vocabulary_version: "current".into(),
                        overlay_generation: 0,
                        evaluator_revision: 0,
                    }]);
                    vm.set_pack_facts(Vec::new());
                }
            }
            let refusal = scope
                .check(&mut vm)
                .err()
                .expect("actual input ABA retires original receipt");
            let first = vm.execution_refusal.clone().unwrap();
            assert_eq!(refusal.native_execution_refusal(), Some(&first));
            assert!(NativeOperationScope::enter(&mut vm).is_err());
            assert_eq!(vm.execution_refusal, Some(first));
        }
    }
    #[test]
    fn original_vm_nested_receipt_cannot_reissue_after_context_change_or_poison() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let mut vm = native();
        let outer = NativeOperationScope::enter(&mut vm).unwrap();
        let inner = NativeOperationScope::enter(&mut vm).unwrap();
        assert_eq!(outer.currency().generation, inner.currency().generation);
        drop(inner);
        vm.native_operation_context.generation.set(Some(u64::MAX));
        vm.advance_native_operation_context();
        assert_eq!(vm.native_operation_context.generation.get(), None);
        vm.advance_native_operation_context();
        assert_eq!(vm.native_operation_context.generation.get(), None);
        assert!(outer.check(&mut vm).is_err());
        let first = vm.execution_refusal.clone().unwrap();
        drop(outer);
        assert!(NativeOperationScope::enter(&mut vm).is_err());
        assert_eq!(vm.execution_refusal, Some(first));
    }
    #[test]
    fn original_vm_parent_and_child_receipts_keep_independent_parked_owners() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let mut vm = native();
        let child_name = vm.create_child(Some("child".into()), false);
        let child = vm.child_id(&child_name).unwrap();
        let parent = NativeOperationScope::enter(&mut vm).unwrap();
        vm.in_interp(child, |vm| {
            let scope = NativeOperationScope::enter(vm).unwrap();
            assert!(!Rc::ptr_eq(
                &parent.currency().owner,
                &scope.currency().owner
            ));
            let nested = NativeOperationScope::enter(vm).unwrap();
            nested.check(vm).unwrap();
            drop(nested);
            scope.check(vm).unwrap();
        });
        parent.check(&mut vm).unwrap();
        vm.in_interp(child, |vm| {
            let scope = NativeOperationScope::enter(vm).unwrap();
            let original = vm.host_rc();
            vm.set_host(Rc::new(crate::host_native::NativeHost::new()));
            vm.set_host(original);
            assert!(scope.check(vm).is_err());
        });
        // The engine's genuine first Host crosses the child boundary; the
        // parent's actual owner/generation were not refreshed or mutated.
        assert_eq!(
            parent.currency().owner.generation.get(),
            Some(parent.currency().generation)
        );
        assert!(parent.check(&mut vm).is_err());
    }
    #[test]
    fn original_vm_operation_preserves_pending_first_host_before_receipt_or_getter() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let mut vm = native();
        let _ = vm.refuse_host_command("first original VM operation".into());
        let first = vm.execution_refusal.clone().unwrap();
        let refusal = NativeOperationScope::enter(&mut vm)
            .err()
            .expect("first Host stops ingress");
        assert_eq!(refusal.native_execution_refusal(), Some(&first));
        let original = Value::new_native_string_bytes(b"17".as_slice());
        assert!(original.native_scalar_cache().is_none());
        let result = crate::expr::boolean_for_vm(
            &mut vm,
            &original,
            tcl_syntax::native_boolean_truth::NativeBooleanTruthPurpose::LogicalNot,
        );
        assert!(matches!(
            result,
            Err(crate::error::TclError::Host(crate::error::TclHostFailure::Execution(cause)))
                if cause == first
        ));
        assert!(original.native_scalar_cache().is_none());
        assert_eq!(original.resident_string_bytes().unwrap().as_ref(), b"17");
        assert_eq!(vm.execution_refusal, Some(first));
    }
}
