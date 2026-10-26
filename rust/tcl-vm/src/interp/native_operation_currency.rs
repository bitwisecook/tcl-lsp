// SPDX-License-Identifier: AGPL-3.0-or-later
//! Neutral current-operation receipts for the actual selected interpreter.
//!
//! This owner records execution-context ingress, independently of command,
//! namespace, receiver, object, compiler and recipe authority. Its state parks
//! with the actual interpreter. Nested operations inherit the original receipt;
//! neither callback re-entry nor change-and-restore can issue it again.

use super::{InterpId, Vm};
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
    selected: Cell<bool>,
    allocation_permit: RefCell<Rc<Cell<bool>>>,
    entered: RefCell<Vec<Weak<EnteredOperation>>>,
}
impl Default for NativeOperationContext {
    fn default() -> Self {
        Self {
            generation: Cell::new(Some(0)),
            selected: Cell::new(true),
            allocation_permit: RefCell::new(Rc::new(Cell::new(true))),
            entered: RefCell::new(Vec::new()),
        }
    }
}
impl NativeOperationContext {
    pub(super) fn has_entered_operation(&self) -> bool {
        let mut entered = self.entered.borrow_mut();
        entered.retain(|record| record.strong_count() != 0);
        !entered.is_empty()
    }

    pub(super) fn select(&self, selected: bool) {
        self.selected.set(selected);
    }

    pub(super) fn withdraw_allocations(&self) {
        self.allocation_permit.borrow().set(false);
    }

    pub(super) fn reset_allocations(&self) {
        *self.allocation_permit.borrow_mut() = Rc::new(Cell::new(true));
    }

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
    authorised_generation: Rc<Cell<Option<u64>>>,
    allocation_permit: Rc<Cell<bool>>,
    interpreter: NativeInterpreterIdentity,
    context: RuntimeContext,
    profile: &'static tcl_dialect::DialectProfile,
    engine: Option<&'static tcl_dialect::DialectProfile>,
    host: Rc<dyn tcl_platform::Host>,
}
/// Opaque readonly currentness; it exposes no entered or saved currency door.
/// Original authored operations may retain this even when entry is absent.
pub(crate) struct OriginalOperationObservation {
    original: NativeOperationCurrency,
}
impl OriginalOperationObservation {
    pub(crate) fn capture(vm: &mut Vm) -> Result<Self, NativeExecutionError> {
        let original = match NativeOperationScope::current_entered(vm)? {
            Some(entered) => entered.currency().clone(),
            None => NativeOperationCurrency::capture_fields(vm)?,
        };
        Ok(Self { original })
    }
    pub(crate) fn check(&self, vm: &mut Vm) -> Result<(), CmdError> {
        self.original.check(vm)
    }

    /// Advance only at the actual measured-input publication point. Cache
    /// retirement remains checked work, with the same first-cause owner.
    pub(super) fn replace_observed_inputs_using(
        &self,
        vm: &mut Vm,
        names: Option<super::execution_name_policy::ObservedNameSelection>,
        storage: Option<super::execution_name_policy::ObservedFrameStorage>,
        mandatory_retirement: bool,
        current: &mut dyn FnMut(&mut Vm) -> Result<(), CmdError>,
    ) -> Result<(), CmdError> {
        use super::execution_name_policy::ObservedInputReplacementStage;
        if !self.original.owns_saved_target(vm, vm.cur) {
            return Err(CmdError::from_execution_refusal(retain_first(
                vm,
                unavailable(),
            )));
        }
        let before = self.original.check_native(vm);
        if !mandatory_retirement && let Err(first) = &before {
            return Err(CmdError::from_execution_refusal(first.clone()));
        }
        let generation = self.original.current_generation();
        let reached = vm.replace_observed_operation_inputs_using(
            names,
            storage,
            Some(&mut |vm, stage| {
                if let ObservedInputReplacementStage::Published { changed } = stage {
                    before
                        .as_ref()
                        .map_err(|first| CmdError::from_execution_refusal(first.clone()))?;
                    let next = generation
                        .checked_add(u64::from(changed))
                        .ok_or_else(|| retain_first(vm, unavailable()))
                        .map_err(CmdError::from_execution_refusal)?;
                    let interpreter = vm.cur;
                    self.original
                        .check_saved_target_at_generation(vm, interpreter, next)
                        .map_err(CmdError::from_execution_refusal)?;
                    self.original.authorised_generation.set(Some(next));
                }
                self.original.check(vm)?;
                current(vm)
            }),
        );
        before.map_err(CmdError::from_execution_refusal)?;
        self.original.check(vm)?;
        current(vm)?;
        reached.map(|_| ())
    }

    #[cfg(test)]
    fn replace_observed_inputs(
        &self,
        vm: &mut Vm,
        names: Option<super::execution_name_policy::ObservedNameSelection>,
        storage: Option<super::execution_name_policy::ObservedFrameStorage>,
        mandatory_retirement: bool,
    ) -> Result<(), CmdError> {
        self.replace_observed_inputs_using(
            vm,
            names,
            storage,
            mandatory_retirement,
            &mut |_| Ok(()),
        )
    }
}

struct EnteredOperation {
    currency: NativeOperationCurrency,
    _allocation: crate::value::JimAllocationScope,
}

/// Allocation-only scope ownership creates no native object/reference role.
#[derive(Clone)]
pub(crate) struct NativeOperationScope {
    entered: Rc<EnteredOperation>,
}

/// Cleanup-only physical target captured at the original execution stage.
/// A saved currency is descriptive; this holder retains no entered scope.
#[derive(Clone)]
pub(crate) struct SavedExecutionTarget {
    owner: u64,
    interpreter: InterpId,
    context: Weak<NativeOperationContext>,
    currency: Result<Option<NativeOperationCurrency>, NativeExecutionError>,
}

/// Calling convention for physical mandatory retirement, not invocation admission.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum CleanupRestoration {
    RestoreCaller,
    PreserveCallbackSelection,
}

/// Already reached mandatory-continuation transport and its retained first cause.
/// Metadata may own extracted retirement iterators or tickets. It is neither
/// ordinary result publication nor permission to acquire a new Native role.
pub(super) struct ReachedCleanup<T> {
    pub(super) metadata: T,
    pub(super) first_cause: Option<NativeExecutionError>,
}

#[derive(Clone, Copy)]
enum CleanupPublication {
    OrdinaryResult,
    MandatoryContinuation,
}

impl SavedExecutionTarget {
    /// Observe the actual target before callbacks or hidden-frame publication.
    /// A refused readonly issue is retained without fabricating a receipt.
    pub(crate) fn capture(vm: &mut Vm) -> Self {
        let owner = vm.owner_nonce;
        let interpreter = vm.cur_interp();
        let context = Rc::downgrade(&vm.native_operation_context);
        let currency = NativeOperationScope::current_entered(vm)
            .map(|original| original.map(|scope| scope.currency().clone()));
        Self {
            owner,
            interpreter,
            context,
            currency,
        }
    }

    /// Describe only the physical target already issued by this authentic currency.
    /// This permits mandatory cleanup after the last activation has been consumed.
    pub(crate) fn from_currency(
        original: &NativeOperationCurrency,
    ) -> Result<Self, NativeExecutionError> {
        let interpreter = usize::try_from(original.interpreter.interpreter)
            .map(InterpId)
            .map_err(|_| unavailable())?;
        Ok(Self {
            owner: original.interpreter.owner,
            interpreter,
            context: Rc::downgrade(&original.owner),
            currency: Ok(Some(original.clone())),
        })
    }

    pub(super) fn interpreter(&self) -> InterpId {
        self.interpreter
    }

    /// Authenticate allocation identity only; this is not a currency/entry check.
    pub(super) fn owns_target(&self, vm: &Vm) -> bool {
        self.owner == vm.owner_nonce
            && self.context.upgrade().is_some_and(|context| {
                vm.st_of(self.interpreter)
                    .is_some_and(|state| Rc::ptr_eq(&context, &state.native_operation_context))
            })
            && self
                .currency
                .as_ref()
                .ok()
                .and_then(Option::as_ref)
                .is_none_or(|currency| currency.owns_saved_target(vm, self.interpreter))
    }

    /// Check the original receipt and callback-selected current physical target.
    /// This grants no command, object, frame, namespace or Native getter purpose.
    pub(crate) fn check_current(&self, vm: &mut Vm) -> Result<(), NativeExecutionError> {
        if let Some(first) = vm.execution_refusal.clone() {
            return Err(first);
        }
        if let Err(first) = &self.currency {
            return Err(retain_first(vm, first.clone()));
        }
        if !self.owns_target(vm) || !vm.interp_is_current(self.interpreter) {
            return Err(retain_first(vm, unavailable()));
        }
        match &self.currency {
            Ok(Some(currency)) => currency.check_native(vm),
            Ok(None) => Ok(()),
            Err(_) => unreachable!("original readonly refusal returned above"),
        }
    }

    /// Complete original cleanup before publishing an ordinary checked result.
    /// Refused owned results retire at their reached attachment/restoration stage.
    pub(crate) fn in_original<T>(
        &self,
        vm: &mut Vm,
        work: impl FnOnce(&mut Vm) -> T,
    ) -> Result<T, NativeExecutionError> {
        let reached = self.settle_in_original(
            vm,
            CleanupRestoration::RestoreCaller,
            CleanupPublication::OrdinaryResult,
            work,
        )?;
        match reached.first_cause {
            Some(first) => {
                drop(reached.metadata);
                Err(retain_first(vm, first))
            }
            None => Ok(reached.metadata.expect("unrefused reached result")),
        }
    }

    /// Transport reached original retirement work even after a typed refusal.
    /// Outer Err means the physical target was unavailable and no work ran.
    pub(super) fn retire_in_original<T>(
        &self,
        vm: &mut Vm,
        policy: CleanupRestoration,
        work: impl FnOnce(&mut Vm) -> T,
    ) -> Result<ReachedCleanup<T>, NativeExecutionError> {
        let reached =
            self.settle_in_original(vm, policy, CleanupPublication::MandatoryContinuation, work)?;
        Ok(ReachedCleanup {
            metadata: reached
                .metadata
                .expect("mandatory reached metadata is retained"),
            first_cause: reached.first_cause,
        })
    }

    fn settle_in_original<T>(
        &self,
        vm: &mut Vm,
        restoration: CleanupRestoration,
        publication: CleanupPublication,
        work: impl FnOnce(&mut Vm) -> T,
    ) -> Result<ReachedCleanup<Option<T>>, NativeExecutionError> {
        // Preserve the callback-selected refusal before parking it. Withdrawn
        // currency still permits already-reached same-physical-target cleanup.
        let _ = self.check_current(vm);
        if !self.owns_target(vm) {
            return Err(retain_first(vm, unavailable()));
        }
        let reached = vm.in_interp_for_cleanup(self.interpreter, restoration, |vm| {
            let cleanup = match &self.currency {
                Ok(Some(currency)) => Some(NativeOperationScope::enter_saved_cleanup(
                    vm,
                    currency,
                    self.interpreter,
                )?),
                Ok(None) => None,
                Err(first) => {
                    retain_first(vm, first.clone());
                    None
                }
            };
            let mut metadata = Some(work(vm));
            let _ = self.check_current(vm);
            if matches!(publication, CleanupPublication::OrdinaryResult)
                && vm.execution_refusal.is_some()
            {
                // Preserve the existing refused-result disposal while the
                // genuine original cleanup attachment is still present.
                drop(metadata.take());
                let _ = self.check_current(vm);
            }
            drop(cleanup);
            let _ = self.check_current(vm);
            if matches!(publication, CleanupPublication::OrdinaryResult)
                && vm.execution_refusal.is_some()
            {
                drop(metadata.take());
            }
            Ok::<_, NativeExecutionError>(metadata)
        })?;
        // Actual restoration/finalization can retain a later teardown cause.
        // Its intentional current selection must not be checked against self.
        Ok(ReachedCleanup {
            metadata: reached,
            first_cause: vm.execution_refusal.clone(),
        })
    }
}

fn unavailable() -> NativeExecutionError {
    NativeExecutionError::ValueAccessRefusal(NativeValueAccessRefusal::CommandProtocolUnavailable(
        "entered VM original execution context",
    ))
}
fn retain_first(vm: &mut Vm, cause: NativeExecutionError) -> NativeExecutionError {
    vm.retain_native_execution_refusal(cause)
}
impl NativeOperationCurrency {
    fn capture_fields(vm: &mut Vm) -> Result<Self, NativeExecutionError> {
        if let Some(first) = vm.execution_refusal.clone() {
            return Err(first);
        }
        let owner = Rc::clone(&vm.native_operation_context);
        let generation = owner
            .generation
            .get()
            .ok_or_else(|| retain_first(vm, unavailable()))?;
        let allocation_permit = Rc::clone(&owner.allocation_permit.borrow());
        Ok(Self {
            owner,
            generation,
            authorised_generation: Rc::new(Cell::new(None)),
            allocation_permit,
            interpreter: vm.native_interpreter_identity(),
            context: vm.runtime_context().clone(),
            profile: vm.dialect_profile(),
            engine: vm.actual_engine_profile,
            host: vm.host_rc(),
        })
    }

    pub(crate) fn check(&self, vm: &mut Vm) -> Result<(), CmdError> {
        self.check_native(vm)
            .map_err(CmdError::from_execution_refusal)
    }

    pub(crate) fn check_native(&self, vm: &mut Vm) -> Result<(), NativeExecutionError> {
        let interpreter = vm.cur;
        self.check_saved_target(vm, interpreter)
    }

    fn owns_saved_target(&self, vm: &Vm, interpreter: InterpId) -> bool {
        vm.st_of(interpreter).is_some_and(|state| {
            Rc::ptr_eq(&self.owner, &state.native_operation_context)
                && self.interpreter.owner == vm.owner_nonce
                && u64::try_from(interpreter.0).ok() == Some(self.interpreter.interpreter)
        })
    }

    pub(super) fn check_saved_target(
        &self,
        vm: &mut Vm,
        interpreter: InterpId,
    ) -> Result<(), NativeExecutionError> {
        self.check_saved_target_at_generation(vm, interpreter, self.current_generation())
    }

    fn current_generation(&self) -> u64 {
        self.authorised_generation.get().unwrap_or(self.generation)
    }

    fn check_saved_target_at_generation(
        &self,
        vm: &mut Vm,
        interpreter: InterpId,
        generation: u64,
    ) -> Result<(), NativeExecutionError> {
        if let Some(first) = vm.execution_refusal.clone() {
            return Err(first);
        }
        let current = self.owns_saved_target(vm, interpreter)
            && vm.st_of(interpreter).is_some_and(|state| {
                self.owner.generation.get() == Some(generation)
                    && state.pin.context == self.context
                    && std::ptr::eq(state.pin.profile, self.profile)
                    && state.actual_engine_profile.map(std::ptr::from_ref)
                        == self.engine.map(std::ptr::from_ref)
                    && Rc::ptr_eq(&state.host, &self.host)
            });
        if !current {
            return Err(retain_first(vm, unavailable()));
        }
        Ok(())
    }
}
impl NativeOperationScope {
    pub(crate) fn enter(vm: &mut Vm) -> Result<Self, CmdError> {
        Self::enter_native(vm).map_err(CmdError::from_execution_refusal)
    }

    /// Borrow only an actual live entry in the current physical interpreter.
    /// Retained saved receipts and parked or ended entries issue no authority.
    pub(crate) fn current_entered(vm: &mut Vm) -> Result<Option<Self>, NativeExecutionError> {
        if let Some(first) = vm.execution_refusal.clone() {
            return Err(first);
        }
        let owner = Rc::clone(&vm.native_operation_context);
        // Weak records never retain ended execution or its Host. No owner
        // borrow crosses a check, callback or retirement.
        let entered = {
            let mut records = owner.entered.borrow_mut();
            records.retain(|record| record.strong_count() != 0);
            records.last().and_then(Weak::upgrade)
        };
        let original = entered.map(|entered| Self { entered });
        if let Some(original) = &original {
            original.check_native(vm)?;
        }
        Ok(original)
    }

    fn allocation_scope(
        vm: &mut Vm,
        currency: &NativeOperationCurrency,
        permit_new_context: bool,
    ) -> Result<crate::value::JimAllocationScope, NativeExecutionError> {
        let pool = if vm
            .actual_native_invocation_dialect()
            .native_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            if permit_new_context {
                Some(
                    vm.native_jim_object_context()
                        .map_err(|error| {
                            retain_first(
                                vm,
                                NativeExecutionError::ValueAccessRefusal(
                                    error.native_access_refusal().expect("actual Jim context"),
                                ),
                            )
                        })?
                        .allocation_pool(),
                )
            } else {
                vm.jim_object_context
                    .borrow()
                    .as_ref()
                    .map(|context| context.allocation_pool())
            }
        } else {
            None
        };
        let current = currency.clone();
        Ok(crate::value::JimAllocationScope::enter(
            pool,
            Rc::new(move || {
                current.allocation_permit.get()
                    && current.owner.selected.get()
                    && current.owner.generation.get() == Some(current.current_generation())
            }),
        ))
    }

    pub(crate) fn enter_native(vm: &mut Vm) -> Result<Self, NativeExecutionError> {
        let inherited = Self::current_entered(vm)?;
        let owner = Rc::clone(&vm.native_operation_context);
        let currency = if let Some(original) = inherited {
            original.currency().clone()
        } else {
            NativeOperationCurrency::capture_fields(vm)?
        };
        let allocation = Self::allocation_scope(vm, &currency, true)?;
        let entered = Rc::new(EnteredOperation {
            currency,
            _allocation: allocation,
        });
        owner.entered.borrow_mut().push(Rc::downgrade(&entered));
        Ok(Self { entered })
    }
    /// Attach only the original saved receipt to its authentic physical target.
    /// Currency failure is retained but never skips same-target mandatory cleanup.
    pub(super) fn enter_saved_cleanup(
        vm: &mut Vm,
        original: &NativeOperationCurrency,
        interpreter: InterpId,
    ) -> Result<Self, NativeExecutionError> {
        if !original.owns_saved_target(vm, interpreter) {
            return Err(retain_first(vm, unavailable()));
        }
        let _ = original.check_saved_target(vm, interpreter);
        let allocation = Self::allocation_scope(vm, original, false)?;
        let entered = Rc::new(EnteredOperation {
            currency: original.clone(),
            _allocation: allocation,
        });
        original
            .owner
            .entered
            .borrow_mut()
            .push(Rc::downgrade(&entered));
        let _ = original.check_saved_target(vm, interpreter);
        Ok(Self { entered })
    }

    /// Re-enter the same saved execution only after complete original currency checks.
    pub(crate) fn resume_saved(
        vm: &mut Vm,
        original: &NativeOperationCurrency,
    ) -> Result<Self, NativeExecutionError> {
        original.check_native(vm)?;
        Self::enter_saved_cleanup(vm, original, vm.cur)
    }

    /// Same-target retirement remains mandatory after original currency withdrawal.
    pub(crate) fn cleanup_saved(
        vm: &mut Vm,
        original: &NativeOperationCurrency,
    ) -> Result<Self, NativeExecutionError> {
        Self::enter_saved_cleanup(vm, original, vm.cur)
    }

    pub(crate) fn currency(&self) -> &NativeOperationCurrency {
        &self.entered.currency
    }
    pub(crate) fn check(&self, vm: &mut Vm) -> Result<(), CmdError> {
        self.check_native(vm)
            .map_err(CmdError::from_execution_refusal)
    }

    pub(crate) fn check_native(&self, vm: &mut Vm) -> Result<(), NativeExecutionError> {
        self.currency().check_native(vm)
    }
}
impl Vm {
    pub(super) fn has_entered_native_operation(&self) -> bool {
        self.state.native_operation_context.has_entered_operation()
            || self.interps.iter().any(|slot| {
                slot.parked
                    .as_deref()
                    .is_some_and(|state| state.native_operation_context.has_entered_operation())
            })
    }

    pub(super) fn advance_native_operation_context(&self) {
        self.native_compiler_pass_owner
            .invalidate_ordinary_entry_observation();
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
    fn actual_factory_scope_tracks_parked_selection_and_retains_first_cause_withdrawal() {
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("jim"));
        let child_name = vm.create_child(Some("allocator-child".into()), false);
        let child = vm.child_id(&child_name).unwrap();
        let parent = NativeOperationScope::enter(&mut vm).unwrap();
        let before = parent.currency().owner.generation.get();
        vm.in_interp(child, |vm| {
            let parked = Value::new_native_string_bytes(b"PARKED".as_slice());
            assert!(matches!(
                crate::value::JimCommandNameObservation::capture(&parked).unwrap(),
                crate::value::JimCommandNameObservation::Exact(_)
            ));
            let _child = NativeOperationScope::enter(vm).unwrap();
            let entered = Value::new_native_string_bytes(b"CHILD".as_slice());
            assert!(matches!(
                crate::value::JimCommandNameObservation::capture(&entered).unwrap(),
                crate::value::JimCommandNameObservation::Resident(_)
            ));
        });
        parent.check(&mut vm).unwrap();
        let restored = Value::new_native_string_bytes(b"RESTORED".as_slice());
        assert!(matches!(
            crate::value::JimCommandNameObservation::capture(&restored).unwrap(),
            crate::value::JimCommandNameObservation::Resident(_)
        ));
        let saved = parent.currency().clone();
        let _ = vm.refuse_host_command("actual allocator first cause".into());
        let first = vm.execution_refusal.clone().unwrap();
        assert_eq!(parent.currency().owner.generation.get(), before);
        let withdrawn = Value::new_native_string_bytes(b"WITHDRAWN".as_slice());
        assert!(matches!(
            crate::value::JimCommandNameObservation::capture(&withdrawn).unwrap(),
            crate::value::JimCommandNameObservation::Exact(_)
        ));
        assert!(parent.check(&mut vm).is_err());
        assert_eq!(vm.execution_refusal, Some(first.clone()));
        drop(parent);
        assert_eq!(vm.take_host_boundary_refusal(), Some(first));
        assert!(vm.execution_refusal.is_none());
        {
            let _old_allocation =
                NativeOperationScope::allocation_scope(&mut vm, &saved, false).unwrap();
            let old = Value::new_native_string_bytes(b"OLD EPISODE".as_slice());
            assert!(matches!(
                crate::value::JimCommandNameObservation::capture(&old).unwrap(),
                crate::value::JimCommandNameObservation::Exact(_)
            ));
        }
        let _fresh = NativeOperationScope::enter(&mut vm).unwrap();
        let fresh = Value::new_native_string_bytes(b"FRESH".as_slice());
        assert!(matches!(
            crate::value::JimCommandNameObservation::capture(&fresh).unwrap(),
            crate::value::JimCommandNameObservation::Resident(_)
        ));
    }

    #[test]
    fn original_vm_context_receipt_survives_guest_table_and_same_input_policy_changes() {
        // naming.embedding.original-host-publication-and-fact-transport

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
        assert_eq!(
            result.result.resident_string_bytes().unwrap().as_ref(),
            b"RESULT"
        );
        scope.check(&mut vm).unwrap();
        assert!(vm.execution_refusal.is_none());
    }
    #[test]
    fn original_vm_context_receipt_refuses_host_profile_compiler_and_pack_change_restore() {
        // naming.embedding.original-host-publication-and-fact-transport

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
                .map_or_else(core::convert::identity, |()| {
                    panic!("actual input ABA retires original receipt")
                });
            let first = vm.execution_refusal.clone().unwrap();
            assert_eq!(refusal.native_execution_refusal(), Some(&first));
            assert!(NativeOperationScope::enter(&mut vm).is_err());
            assert_eq!(vm.execution_refusal, Some(first));
        }
    }
    #[test]
    fn original_vm_nested_receipt_cannot_reissue_after_context_change_or_poison() {
        // naming.embedding.original-host-publication-and-fact-transport

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
    #[test]
    fn original_current_entry_distinguishes_live_saved_ended_and_parked_receipts() {
        // naming.embedding.original-host-publication-and-fact-transport

        let mut vm = native();
        assert!(
            NativeOperationScope::current_entered(&mut vm)
                .unwrap()
                .is_none()
        );
        let child_name = vm.create_child(Some("entry_child".into()), false);
        let child = vm.child_id(&child_name).unwrap();
        let outer = NativeOperationScope::enter_native(&mut vm).unwrap();
        let observed = NativeOperationScope::current_entered(&mut vm)
            .unwrap()
            .unwrap();
        assert!(Rc::ptr_eq(&outer.entered, &observed.entered));
        drop(observed);
        let saved_metadata = outer.currency().clone();
        vm.in_interp(child, |vm| {
            assert!(NativeOperationScope::current_entered(vm).unwrap().is_none());
            let child_scope = NativeOperationScope::enter_native(vm).unwrap();
            let current = NativeOperationScope::current_entered(vm).unwrap().unwrap();
            assert!(Rc::ptr_eq(&child_scope.entered, &current.entered));
            assert!(!Rc::ptr_eq(&outer.entered, &current.entered));
        });
        outer.check_native(&mut vm).unwrap();
        drop(outer);
        assert!(
            NativeOperationScope::current_entered(&mut vm)
                .unwrap()
                .is_none()
        );
        saved_metadata.check_native(&mut vm).unwrap();
        assert!(!vm.native_operation_context.has_entered_operation());
    }

    #[test]
    fn original_numeric_routing_checks_inherited_currency_before_authored_policy() {
        // naming.embedding.original-host-publication-and-fact-transport

        let mut vm = native();
        let outer = NativeOperationScope::enter_native(&mut vm).unwrap();
        vm.set_dialect_profile(tcl_dialect::DialectProfile::irules());
        assert!(vm.authored_numeric_context().is_some());
        let called = std::cell::Cell::new(false);
        let reached = crate::expr::with_original_numeric_context(&mut vm, |_| {
            called.set(true);
            Ok(())
        });
        assert!(!called.get());
        let crate::TclError::Host(crate::error::TclHostFailure::Execution(first)) =
            reached.unwrap_err()
        else {
            panic!("the genuine inherited receipt must stop authored routing");
        };
        assert_eq!(vm.execution_refusal, Some(first.clone()));
        assert_eq!(outer.check_native(&mut vm).unwrap_err(), first);
        let mut authored = native();
        authored.set_dialect_profile(tcl_dialect::DialectProfile::irules());
        let answer = crate::expr::with_original_numeric_context(&mut authored, |context| {
            assert!(context.original.is_none());
            Value::int(7).as_int_with_stages(None)
        })
        .unwrap();
        assert_eq!(answer, 7);
        assert!(!authored.native_operation_context.has_entered_operation());
        assert!(authored.execution_refusal.is_none());
    }
    #[test]
    fn original_coroutine_park_preserves_only_the_actual_active_parent_entry() {
        // naming.embedding.original-host-publication-and-fact-transport

        let mut vm = native();
        let outer = NativeOperationScope::enter_native(&mut vm).unwrap();
        let parked = vm
            .try_eval_source("coroutine paused apply {{} {yield READY; return END}}")
            .unwrap();
        assert_eq!(parked.code, Code::Ok);
        let observed = NativeOperationScope::current_entered(&mut vm)
            .unwrap()
            .unwrap();
        assert!(Rc::ptr_eq(&outer.entered, &observed.entered));
        drop(observed);
        outer.check_native(&mut vm).unwrap();
        drop(outer);
        assert!(
            NativeOperationScope::current_entered(&mut vm)
                .unwrap()
                .is_none()
        );
        let resumed = vm.try_invoke_command("paused", &[]).unwrap();
        assert_eq!(resumed.code, Code::Ok);
        assert_eq!(
            resumed.result.resident_string_bytes().unwrap().as_ref(),
            b"END"
        );
        assert!(
            NativeOperationScope::current_entered(&mut vm)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn original_saved_execution_checks_same_target_currency_before_entry_and_keeps_cleanup() {
        // naming.embedding.original-host-publication-and-fact-transport

        let mut vm = native();
        let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
        let saved = scope.currency().clone();
        drop(scope);
        assert!(
            NativeOperationScope::current_entered(&mut vm)
                .unwrap()
                .is_none()
        );
        let resumed = NativeOperationScope::resume_saved(&mut vm, &saved).unwrap();
        let observed = NativeOperationScope::current_entered(&mut vm)
            .unwrap()
            .unwrap();
        assert!(Rc::ptr_eq(&resumed.entered, &observed.entered));
        drop((observed, resumed));
        let original_host = vm.host_rc();
        vm.set_host(Rc::new(crate::host_native::NativeHost::new()));
        vm.set_host(original_host);
        let first = NativeOperationScope::resume_saved(&mut vm, &saved)
            .err()
            .unwrap();
        assert!(!vm.native_operation_context.has_entered_operation());
        assert_eq!(saved.check_native(&mut vm).unwrap_err(), first);
        let cleanup = NativeOperationScope::cleanup_saved(&mut vm, &saved).unwrap();
        assert!(vm.native_operation_context.has_entered_operation());
        assert_eq!(cleanup.check_native(&mut vm).unwrap_err(), first);
        drop(cleanup);
        assert!(!vm.native_operation_context.has_entered_operation());
        assert_eq!(vm.execution_refusal.as_ref(), Some(&first));
    }
    #[test]
    fn original_saved_execution_target_is_descriptive_until_same_currency_cleanup() {
        // naming.embedding.original-host-publication-and-fact-transport

        // Readonly software target/currency state; no Native execution or reference grant.
        let mut vm = native();
        let independent = SavedExecutionTarget::capture(&mut vm);
        assert!(
            NativeOperationScope::current_entered(&mut vm)
                .unwrap()
                .is_none()
        );
        independent
            .in_original(&mut vm, |vm| {
                assert!(NativeOperationScope::current_entered(vm).unwrap().is_none());
            })
            .unwrap();
        let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
        let captured = SavedExecutionTarget::capture(&mut vm);
        let original = scope.currency().clone();
        drop(scope);
        assert!(
            NativeOperationScope::current_entered(&mut vm)
                .unwrap()
                .is_none()
        );
        captured
            .in_original(&mut vm, |vm| {
                let cleanup = NativeOperationScope::current_entered(vm).unwrap().unwrap();
                assert!(Rc::ptr_eq(&cleanup.currency().owner, &original.owner));
                assert_eq!(cleanup.currency().generation, original.generation);
            })
            .unwrap();
        assert!(
            NativeOperationScope::current_entered(&mut vm)
                .unwrap()
                .is_none()
        );
        assert!(vm.execution_refusal.is_none());
    }

    #[test]
    fn original_saved_execution_target_cleans_parent_and_preserves_callback_child() {
        // naming.embedding.original-host-publication-and-fact-transport

        // Actual arena switch and original cleanup receipt; no external callback chronology.
        let mut vm = native();
        let parent = vm.cur_interp();
        let child_name = vm.create_child(Some("saved_execution_child".into()), false);
        let child = vm.child_id(&child_name).unwrap();
        vm.in_interp(child, |vm| vm.activation_depth = 7);
        let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
        let target = SavedExecutionTarget::capture(&mut vm);
        crate::interp::oo_cleanup::select_cleanup_fixture_interpreter(&mut vm, child);
        let first = target.check_current(&mut vm).unwrap_err();
        let mut returned = None;
        let reached = target.in_original(&mut vm, |vm| {
            assert!(vm.interp_is_current(parent));
            assert_eq!(
                NativeOperationScope::enter_native(vm).err(),
                Some(first.clone())
            );
            vm.activation_depth = 41;
            let owned = Value::string("REFUSED_OWNED_RESULT");
            returned = Some(owned.native_lifetime_lease());
            owned
        });
        assert_eq!(reached.err(), Some(first.clone()));
        assert!(!returned.unwrap().value().native_object_is_live());
        assert!(vm.interp_is_current(child));
        assert_eq!(vm.activation_depth, 7);
        vm.in_interp(parent, |vm| {
            assert_eq!(vm.activation_depth, 41);
        });
        assert!(vm.interp_is_current(child));
        assert_eq!(vm.execution_refusal, Some(first.clone()));
        assert_eq!(scope.check_native(&mut vm).err(), Some(first));
    }

    #[test]
    fn original_saved_execution_target_retains_failed_issue_and_finishes_owned_cleanup() {
        // naming.embedding.original-host-publication-and-fact-transport

        // Existing full software Host issue and mandatory same-target retirement only.
        let mut vm = native();
        let _ = vm.refuse_host_command("saved cleanup original first Host".into());
        let first = vm.execution_refusal.clone().unwrap();
        let target = SavedExecutionTarget::capture(&mut vm);
        let mut reached = 0;
        assert_eq!(target.check_current(&mut vm).err(), Some(first.clone()));
        assert_eq!(
            target
                .in_original(&mut vm, |vm| {
                    reached += 1;
                    assert!(!vm.native_operation_context.has_entered_operation());
                    assert_eq!(
                        NativeOperationScope::enter_native(vm).err(),
                        Some(first.clone())
                    );
                })
                .err(),
            Some(first.clone())
        );
        assert_eq!(reached, 1);
        assert_eq!(vm.execution_refusal, Some(first));
    }

    #[test]
    fn original_saved_execution_target_rejects_foreign_owner_before_state_publication() {
        // naming.embedding.original-host-publication-and-fact-transport

        // Actual independent Vm nonce/context, not a fabricated foreign receipt.
        let mut original = native();
        let scope = NativeOperationScope::enter_native(&mut original).unwrap();
        let target = SavedExecutionTarget::capture(&mut original);
        let mut foreign = native();
        foreign.activation_depth = 53;
        let _ = foreign.refuse_host_command("foreign original first Host".into());
        let first = foreign.execution_refusal.clone().unwrap();
        let selected = foreign.cur_interp();
        let mut reached = 0;
        assert_eq!(
            target
                .in_original(&mut foreign, |_| {
                    reached += 1;
                })
                .err(),
            Some(first.clone())
        );
        assert_eq!(reached, 0);
        assert!(foreign.interp_is_current(selected));
        assert_eq!(foreign.activation_depth, 53);
        assert_eq!(foreign.execution_refusal, Some(first));
        scope.check_native(&mut original).unwrap();
    }
    #[test]
    fn original_saved_execution_target_checks_real_jim_teardown_after_restoration() {
        // naming.embedding.original-host-publication-and-fact-transport

        // Actual dying-slot in_interp/finalize/Jim teardown path, not Native callback chronology.
        let mut vm =
            crate::native_fixture::interpreter(crate::environment::profile_for_dialect("jim"));
        let parent = vm.cur_interp();
        let child_name = vm.create_child(Some("saved_execution_teardown".into()), false);
        let child = vm.child_id(&child_name).unwrap();
        crate::interp::oo_cleanup::select_cleanup_fixture_interpreter(&mut vm, child);
        let context = vm.native_jim_object_context().unwrap();
        let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
        let target = SavedExecutionTarget::capture(&mut vm);
        let mut returned = None;
        let reached = target.in_original(&mut vm, |vm| {
            assert!(vm.delete_interp(child));
            assert!(vm.interps[child.0].dying);
            assert_eq!(vm.interps[child.0].active, 1);
            // Withdrawal is in a separately owned Jim context. The operation
            // fields stay current until the real outer finalizer reaches it.
            context.retire();
            target.check_current(vm).unwrap();
            assert!(vm.execution_refusal.is_none());
            let owned = Value::string("UNPUBLISHED_AFTER_RESTORE");
            returned = Some(owned.native_lifetime_lease());
            owned
        });
        let first = NativeExecutionError::ValueAccessRefusal(
            NativeValueAccessRefusal::CommandProtocolUnavailable("retired Jim interpreter"),
        );
        assert_eq!(reached.err(), Some(first.clone()));
        assert_eq!(vm.execution_refusal, Some(first.clone()));
        assert!(!returned.unwrap().value().native_object_is_live());
        assert!(vm.interps[child.0].dying);
        assert_eq!(vm.interps[child.0].active, 0);
        assert!(vm.frames.is_empty());
        assert_eq!(scope.check_native(&mut vm).err(), Some(first));
        crate::interp::oo_cleanup::select_cleanup_fixture_interpreter(&mut vm, parent);
        assert_eq!(vm.frames.len(), 1);
    }
    #[test]
    fn original_retirement_transports_owned_iterator_after_failed_issue_until_mandatory_cleanup() {
        // naming.embedding.original-host-publication-and-fact-transport

        // Existing owned roles remain mandatory-continuation transport, not publication.
        let mut vm = native();
        drop(vm.refuse_host_command("mandatory iterator first Host".into()));
        let first = vm.execution_refusal.clone().unwrap();
        let target = SavedExecutionTarget::capture(&mut vm);
        let value = Value::string("EXTRACTED_OLD_OWNER");
        let lease = value.native_lifetime_lease();
        let reached = target
            .retire_in_original(
                &mut vm,
                CleanupRestoration::PreserveCallbackSelection,
                |vm| {
                    assert!(!vm.native_operation_context.has_entered_operation());
                    assert_eq!(vm.interps[vm.cur_interp().0].active, 0);
                    vec![value].into_iter()
                },
            )
            .unwrap();
        assert_eq!(reached.first_cause, Some(first.clone()));
        assert!(lease.value().native_object_is_live());
        let cleanup = target
            .retire_in_original(
                &mut vm,
                CleanupRestoration::PreserveCallbackSelection,
                |_| drop(reached.metadata),
            )
            .unwrap();
        assert_eq!(cleanup.first_cause, Some(first.clone()));
        assert!(!lease.value().native_object_is_live());
        assert_eq!(vm.execution_refusal, Some(first));
        assert!(!vm.native_operation_context.has_entered_operation());
    }

    #[test]
    fn original_retirement_policies_keep_direct_current_callback_selection_and_hold_schedule() {
        // naming.embedding.original-host-publication-and-fact-transport

        // Actual current/parked arenas; no native callback or preservation chronology claim.
        for policy in [
            CleanupRestoration::RestoreCaller,
            CleanupRestoration::PreserveCallbackSelection,
        ] {
            let mut vm = native();
            let parent = vm.cur_interp();
            let name = vm.create_child(Some("direct_retirement_child".into()), false);
            let child = vm.child_id(&name).unwrap();
            let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
            let target = SavedExecutionTarget::capture(&mut vm);
            let reached = target
                .retire_in_original(&mut vm, policy, |vm| {
                    assert_eq!(
                        vm.interps[parent.0].active,
                        u32::from(policy == CleanupRestoration::RestoreCaller)
                    );
                    vm.activation_depth = 19;
                    crate::interp::oo_cleanup::select_cleanup_fixture_interpreter(vm, child);
                    23
                })
                .unwrap();
            assert_eq!(reached.metadata, 23);
            assert_eq!(reached.first_cause, vm.execution_refusal);
            assert!(reached.first_cause.is_some());
            assert_eq!(
                vm.cur_interp(),
                if policy == CleanupRestoration::RestoreCaller {
                    parent
                } else {
                    child
                }
            );
            assert_eq!(vm.interps[parent.0].active, 0);
            vm.in_interp(parent, |vm| assert_eq!(vm.activation_depth, 19));
            assert!(scope.check_native(&mut vm).is_err());
        }
    }

    #[test]
    fn original_retirement_parked_target_preserves_latest_work_selection_or_entry_caller() {
        // naming.embedding.original-host-publication-and-fact-transport

        let mut vm = native();
        let parent = vm.cur_interp();
        let caller_name = vm.create_child(Some("retirement_entry_caller".into()), false);
        let caller = vm.child_id(&caller_name).unwrap();
        let latest_name = vm.create_child(Some("retirement_latest_child".into()), false);
        let latest = vm.child_id(&latest_name).unwrap();
        let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
        let target = SavedExecutionTarget::capture(&mut vm);
        for policy in [
            CleanupRestoration::RestoreCaller,
            CleanupRestoration::PreserveCallbackSelection,
        ] {
            crate::interp::oo_cleanup::select_cleanup_fixture_interpreter(&mut vm, caller);
            let reached = target
                .retire_in_original(&mut vm, policy, |vm| {
                    assert_eq!(vm.cur_interp(), parent);
                    assert_eq!(vm.interps[parent.0].active, 1);
                    crate::interp::oo_cleanup::select_cleanup_fixture_interpreter(vm, latest);
                    31
                })
                .unwrap();
            assert_eq!(reached.metadata, 31);
            assert_eq!(reached.first_cause, vm.execution_refusal);
            assert!(reached.first_cause.is_some());
            assert_eq!(
                vm.cur_interp(),
                if policy == CleanupRestoration::RestoreCaller {
                    caller
                } else {
                    latest
                }
            );
            assert_eq!(vm.interps[parent.0].active, 0);
        }
        assert!(scope.check_native(&mut vm).is_err());
    }

    #[test]
    fn original_retirement_ordinary_refused_owner_drops_inside_original_attachment() {
        // naming.embedding.original-host-publication-and-fact-transport

        struct Observed {
            context: Weak<NativeOperationContext>,
            attached: Rc<Cell<bool>>,
            value: Value,
        }
        impl Drop for Observed {
            fn drop(&mut self) {
                self.attached
                    .set(self.context.upgrade().unwrap().has_entered_operation());
                assert!(self.value.native_object_is_live());
            }
        }
        let mut vm = native();
        let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
        let target = SavedExecutionTarget::capture(&mut vm);
        drop(scope);
        assert!(!vm.native_operation_context.has_entered_operation());
        let attached = Rc::new(Cell::new(false));
        let value = Value::string("REFUSED_AT_ORIGINAL_ATTACHMENT");
        let lease = value.native_lifetime_lease();
        let reached = target.in_original(&mut vm, |vm| {
            drop(vm.refuse_host_command("owned settlement first Host".into()));
            Observed {
                context: Rc::downgrade(&vm.native_operation_context),
                attached: Rc::clone(&attached),
                value,
            }
        });
        assert!(reached.is_err());
        assert!(attached.get());
        assert!(!lease.value().native_object_is_live());
        assert!(!vm.native_operation_context.has_entered_operation());
        assert_eq!(reached.err(), vm.execution_refusal);
    }

    #[test]
    fn original_retirement_metadata_survives_real_jim_post_restoration_refusal() {
        // naming.embedding.original-host-publication-and-fact-transport

        // Real dying-slot teardown and retained output role; no native callback chronology.
        let mut vm =
            crate::native_fixture::interpreter(crate::environment::profile_for_dialect("jim"));
        let parent = vm.cur_interp();
        let name = vm.create_child(Some("mandatory_retirement_teardown".into()), false);
        let child = vm.child_id(&name).unwrap();
        crate::interp::oo_cleanup::select_cleanup_fixture_interpreter(&mut vm, child);
        let context = vm.native_jim_object_context().unwrap();
        let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
        let target = SavedExecutionTarget::capture(&mut vm);
        let value = Value::string("REACHED_MANDATORY_OWNER");
        let lease = value.native_lifetime_lease();
        let reached = target
            .retire_in_original(&mut vm, CleanupRestoration::RestoreCaller, |vm| {
                assert!(vm.delete_interp(child));
                assert_eq!(vm.interps[child.0].active, 1);
                context.retire();
                target.check_current(vm).unwrap();
                value
            })
            .unwrap();
        let first = NativeExecutionError::ValueAccessRefusal(
            NativeValueAccessRefusal::CommandProtocolUnavailable("retired Jim interpreter"),
        );
        assert_eq!(reached.first_cause, Some(first.clone()));
        assert_eq!(vm.execution_refusal, Some(first.clone()));
        assert!(lease.value().native_object_is_live());
        drop(reached.metadata);
        assert!(!lease.value().native_object_is_live());
        assert_eq!(scope.check_native(&mut vm).err(), Some(first));
        crate::interp::oo_cleanup::select_cleanup_fixture_interpreter(&mut vm, parent);
        assert_eq!(vm.frames.len(), 1);
    }

    #[test]
    fn original_retirement_foreign_physical_target_refuses_before_reached_metadata() {
        // naming.embedding.original-host-publication-and-fact-transport

        let mut original = native();
        let scope = NativeOperationScope::enter_native(&mut original).unwrap();
        let target = SavedExecutionTarget::capture(&mut original);
        let mut foreign = native();
        let selected = foreign.cur_interp();
        let mut calls = 0;
        let reached = target.retire_in_original(
            &mut foreign,
            CleanupRestoration::PreserveCallbackSelection,
            |_| {
                calls += 1;
                41
            },
        );
        assert!(reached.is_err());
        assert_eq!(calls, 0);
        assert_eq!(foreign.cur_interp(), selected);
        assert!(!foreign.native_operation_context.has_entered_operation());
        scope.check_native(&mut original).unwrap();
    }
    #[test]
    fn original_retirement_finalizer_follows_later_callback_child_and_retires_only_original_world()
    {
        // naming.embedding.original-host-publication-and-fact-transport

        // Supplied software retirement callback at the shared finalization frontier;
        // this asserts physical return policy, not Native/Jim defer chronology.
        for policy in [
            CleanupRestoration::RestoreCaller,
            CleanupRestoration::PreserveCallbackSelection,
        ] {
            let mut vm = native();
            let caller = vm.cur_interp();
            let original_name = vm.create_child(Some("finalized_original_target".into()), false);
            let original = vm.child_id(&original_name).unwrap();
            let earlier_name = vm.create_child(Some("earlier_retirement_selection".into()), false);
            let earlier = vm.child_id(&earlier_name).unwrap();
            let later_name = vm.create_child(Some("later_finalizer_selection".into()), false);
            let later = vm.child_id(&later_name).unwrap();
            vm.in_interp(earlier, |vm| vm.activation_depth = 47);
            vm.in_interp(later, |vm| vm.activation_depth = 79);
            let original_world = vm.in_interp(original, |vm| Rc::downgrade(&vm.name_world));
            let earlier_frame = vm.in_interp(earlier, |vm| Rc::downgrade(&vm.frames[0].activation));
            let later_frame = vm.in_interp(later, |vm| Rc::downgrade(&vm.frames[0].activation));
            let callbacks = Cell::new(0);
            let expected_first = RefCell::new(None);
            let reached = vm.in_interp_using(
                original,
                policy,
                |vm| {
                    assert_eq!(vm.interps[original.0].active, 1);
                    assert!(vm.delete_interp(original));
                    drop(vm.refuse_host_command("original physical retirement first Host".into()));
                    *expected_first.borrow_mut() = vm.execution_refusal.clone();
                    crate::interp::oo_cleanup::select_cleanup_fixture_interpreter(vm, earlier);
                    83
                },
                |vm, id, actual_policy| {
                    assert_eq!(id, original);
                    assert_eq!(vm.interps[id.0].active, 0);
                    assert!(original_world.upgrade().unwrap().borrow().is_live());
                    vm.finalize_slot_using(id, actual_policy, |vm| {
                        assert_eq!(vm.cur_interp(), original);
                        callbacks.set(callbacks.get() + 1);
                        drop(vm.refuse_host_command("later finalizer Host".into()));
                        crate::interp::oo_cleanup::select_cleanup_fixture_interpreter(vm, later);
                    });
                },
            );
            assert_eq!(reached, 83);
            assert_eq!(callbacks.get(), 1);
            assert_eq!(
                vm.cur_interp(),
                if policy == CleanupRestoration::RestoreCaller {
                    caller
                } else {
                    later
                }
            );
            assert!(vm.interps[original.0].parked.is_none());
            assert_eq!(vm.interps[original.0].active, 0);
            assert!(original_world.upgrade().is_none());
            let first = vm.execution_refusal.clone().unwrap();
            assert_eq!(Some(&first), expected_first.borrow().as_ref());
            vm.in_interp(earlier, |vm| {
                assert_eq!(vm.activation_depth, 47);
                assert!(vm.name_world.borrow().is_live());
                assert!(
                    earlier_frame
                        .upgrade()
                        .is_some_and(|frame| Rc::ptr_eq(&frame, &vm.frames[0].activation))
                );
            });
            vm.in_interp(later, |vm| {
                assert_eq!(vm.activation_depth, 79);
                assert!(vm.name_world.borrow().is_live());
                assert!(
                    later_frame
                        .upgrade()
                        .is_some_and(|frame| Rc::ptr_eq(&frame, &vm.frames[0].activation))
                );
            });
            assert_eq!(vm.execution_refusal, Some(first));
        }
    }
    fn measured_names(vm: &mut Vm) -> super::super::execution_name_policy::ObservedNameSelection {
        assert!(vm.set_observed_bigip_name_policy(
            tcl_registry::f5::evidence::BigIpBuild::MEASURED_21_1_0_1,
            tcl_registry::f5::BigIpExecutionContext::TmmIRule,
            tcl_registry::f5::naming::BigIpNameEvent::HttpRequest,
        ));
        let selected = vm.observed_names.clone().unwrap();
        vm.replace_observed_operation_inputs(None, None);
        selected
    }

    #[test]
    fn original_observed_transition_advances_same_entered_lineage_without_new_authority() {
        // naming.embedding.original-host-publication-and-fact-transport

        let mut vm = native();
        let names = measured_names(&mut vm);
        let outer = NativeOperationScope::enter_native(&mut vm).unwrap();
        let original = OriginalOperationObservation::capture(&mut vm).unwrap();
        let issued = outer.currency().generation;
        let identity = vm.native_interpreter_identity();
        let strings = vm
            .actual_native_invocation_dialect()
            .native_string_protocol();
        let objects = vm.object_materialization();
        original
            .replace_observed_inputs(&mut vm, Some(names), None, false)
            .unwrap();
        outer.check_native(&mut vm).unwrap();
        let inner = NativeOperationScope::enter_native(&mut vm).unwrap();
        assert!(Rc::ptr_eq(
            &outer.currency().authorised_generation,
            &inner.currency().authorised_generation
        ));
        assert_eq!(outer.currency().generation, issued);
        assert_eq!(inner.currency().generation, issued);
        assert_eq!(vm.native_interpreter_identity(), identity);
        assert_eq!(
            vm.actual_native_invocation_dialect()
                .native_string_protocol(),
            strings
        );
        assert_eq!(vm.object_materialization(), objects);
        assert!(outer.currency().current_generation() > issued);
        original
            .replace_observed_inputs(&mut vm, None, None, true)
            .unwrap();
        outer.check_native(&mut vm).unwrap();
        inner.check_native(&mut vm).unwrap();
        assert!(vm.observed_names.is_none());
        assert!(vm.execution_refusal.is_none());
    }

    #[test]
    fn original_observed_transition_none_stays_readonly_and_keeps_missing_string_provider() {
        // naming.embedding.original-host-publication-and-fact-transport

        let mut vm = Vm::new();
        let names = measured_names(&mut vm);
        let original =
            super::super::original_authored_operands::OriginalAuthoredOperandScope::capture(
                &mut vm,
            )
            .unwrap();
        assert!(
            vm.actual_native_invocation_dialect()
                .native_string_protocol()
                .is_none()
        );
        original
            .replace_observed_inputs(&mut vm, Some(names), None, false)
            .unwrap();
        assert!(!vm.native_operation_context.has_entered_operation());
        assert!(
            vm.actual_native_invocation_dialect()
                .native_string_protocol()
                .is_none()
        );
        let resident = Value::string("RESIDENT");
        assert_eq!(
            original.string(&mut vm, &resident).unwrap().as_ref(),
            b"RESIDENT"
        );
        original
            .replace_observed_inputs(&mut vm, None, None, true)
            .unwrap();
        original.finish(&mut vm).unwrap();
        assert!(!vm.native_operation_context.has_entered_operation());
        let missing =
            super::super::original_authored_operands::OriginalAuthoredOperandScope::capture(
                &mut vm,
            )
            .unwrap();
        assert!(missing.string(&mut vm, &Value::double(1.5)).is_err());
        assert!(!vm.native_operation_context.has_entered_operation());
    }

    #[test]
    fn original_observed_transition_never_authorises_foreign_aba_or_raw_policy_setters() {
        // naming.embedding.original-host-publication-and-fact-transport

        for stage in 0..3 {
            let mut vm = native();
            let names = measured_names(&mut vm);
            let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
            let original = OriginalOperationObservation::capture(&mut vm).unwrap();
            original
                .replace_observed_inputs(&mut vm, Some(names.clone()), None, false)
                .unwrap();
            let allowed = scope.currency().authorised_generation.get();
            match stage {
                0 => {
                    let host = vm.host_rc();
                    vm.set_host(Rc::new(crate::host_native::NativeHost::new()));
                    vm.set_host(host);
                }
                1 => {
                    let profile = vm.dialect_profile();
                    vm.set_dialect_profile(crate::environment::profile_for_dialect("tcl9.1"));
                    vm.set_dialect_profile(profile);
                }
                _ => {
                    vm.replace_observed_operation_inputs(None, None);
                    vm.replace_observed_operation_inputs(Some(names), None);
                }
            }
            let first = original.check(&mut vm).unwrap_err();
            let cause = first.native_execution_refusal().unwrap().clone();
            assert_eq!(
                original
                    .replace_observed_inputs(&mut vm, None, None, true)
                    .unwrap_err()
                    .native_execution_refusal(),
                Some(&cause)
            );
            assert!(vm.observed_names.is_none());
            assert_eq!(scope.currency().authorised_generation.get(), allowed);
            assert_eq!(scope.check_native(&mut vm).unwrap_err(), cause);
            assert_eq!(vm.execution_refusal, Some(cause));
        }
    }

    #[test]
    fn original_observed_transition_exhaustion_at_enter_or_restore_never_advances_lineage() {
        // naming.embedding.original-host-publication-and-fact-transport

        for exhaust_on_restore in [false, true] {
            let mut vm = native();
            let names = measured_names(&mut vm);
            vm.native_operation_context
                .generation
                .set(Some(if exhaust_on_restore {
                    u64::MAX - 1
                } else {
                    u64::MAX
                }));
            let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
            let original = OriginalOperationObservation::capture(&mut vm).unwrap();
            let entered = original.replace_observed_inputs(&mut vm, Some(names), None, false);
            if exhaust_on_restore {
                entered.unwrap();
                assert_eq!(scope.currency().authorised_generation.get(), Some(u64::MAX));
            } else {
                assert!(entered.is_err());
                assert_eq!(scope.currency().authorised_generation.get(), None);
                assert!(vm.observed_names.is_some());
            }
            let allowed = scope.currency().authorised_generation.get();
            assert!(
                original
                    .replace_observed_inputs(&mut vm, None, None, true)
                    .is_err()
            );
            assert!(vm.observed_names.is_none());
            assert_eq!(vm.native_operation_context.generation.get(), None);
            assert_eq!(scope.currency().authorised_generation.get(), allowed);
            let cause = vm.execution_refusal.clone().unwrap();
            assert_eq!(scope.check_native(&mut vm).unwrap_err(), cause);
            assert!(NativeOperationScope::enter_native(&mut vm).is_err());
        }
    }
    #[test]
    fn original_observed_readonly_transition_does_not_refresh_ended_native_currency() {
        // naming.embedding.original-host-publication-and-fact-transport

        let mut vm = native();
        let names = measured_names(&mut vm);
        let entered = NativeOperationScope::enter_native(&mut vm).unwrap();
        let saved = entered.currency().clone();
        drop(entered);
        let original = OriginalOperationObservation::capture(&mut vm).unwrap();
        assert!(!Rc::ptr_eq(
            &saved.authorised_generation,
            &original.original.authorised_generation
        ));
        original
            .replace_observed_inputs(&mut vm, Some(names), None, false)
            .unwrap();
        assert!(!vm.native_operation_context.has_entered_operation());
        assert_eq!(saved.authorised_generation.get(), None);
        let first = saved.check_native(&mut vm).unwrap_err();
        assert!(NativeOperationScope::resume_saved(&mut vm, &saved).is_err());
        assert!(
            original
                .replace_observed_inputs(&mut vm, None, None, true)
                .is_err()
        );
        assert!(vm.observed_names.is_none());
        assert_eq!(saved.authorised_generation.get(), None);
        assert_eq!(vm.execution_refusal, Some(first));
    }
    #[test]
    fn original_observed_mandatory_transition_refuses_foreign_physical_inputs() {
        // naming.embedding.original-host-publication-and-fact-transport

        let mut vm = native();
        let names = measured_names(&mut vm);
        let parent = vm.cur_interp();
        let child_name = vm.create_child(Some("observed_transition_child".into()), false);
        let child = vm.child_id(&child_name).unwrap();
        let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
        let original = OriginalOperationObservation::capture(&mut vm).unwrap();
        super::super::oo_cleanup::select_cleanup_fixture_interpreter(&mut vm, child);
        let generation = vm.native_operation_context.generation.get();
        assert!(vm.observed_names.is_none());
        assert!(
            original
                .replace_observed_inputs(&mut vm, Some(names), None, true)
                .is_err()
        );
        assert!(vm.observed_names.is_none());
        assert_eq!(vm.native_operation_context.generation.get(), generation);
        assert_eq!(scope.currency().authorised_generation.get(), None);
        let first = vm.execution_refusal.clone().unwrap();
        vm.in_interp(parent, |vm| {
            assert!(
                original
                    .replace_observed_inputs(vm, None, None, true)
                    .is_err()
            );
            assert!(vm.observed_names.is_none());
        });
        assert!(vm.interp_is_current(child));
        assert_eq!(vm.execution_refusal, Some(first));
    }

    #[test]
    fn original_observed_transition_checks_actual_cache_retirement_before_next_stage() {
        // naming.embedding.original-host-publication-and-fact-transport

        // Software stage check after the actual first cache clear; this does
        // not claim a Native child-Drop or external callback chronology.
        let mut vm = native();
        let names = measured_names(&mut vm);
        let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
        let original = OriginalOperationObservation::capture(&mut vm).unwrap();
        let operands =
            super::super::original_authored_operands::OriginalAuthoredOperandScope::capture(
                &mut vm,
            )
            .unwrap();
        let mut checks = 0;
        let reached =
            original.replace_observed_inputs_using(&mut vm, Some(names), None, false, &mut |vm| {
                checks += 1;
                if checks == 2 {
                    let namespace = vm.current_ns_id();
                    tcl_runtime_api::Frames::push(vm, namespace);
                }
                operands.check(vm)
            });
        assert!(reached.is_err());
        assert_eq!(checks, 2);
        assert!(vm.eval_cache.is_empty());
        assert!(vm.eval_cache_plain.is_empty());
        assert!(vm.module_procs.is_empty());
        assert_eq!(
            scope.currency().authorised_generation.get(),
            vm.native_operation_context.generation.get()
        );
        let first = vm.execution_refusal.clone().unwrap();
        assert!(
            original
                .replace_observed_inputs(&mut vm, None, None, true)
                .is_err()
        );
        assert!(vm.observed_names.is_none());
        assert_eq!(vm.execution_refusal, Some(first));
        tcl_runtime_api::Frames::pop(&mut vm);
    }
}
