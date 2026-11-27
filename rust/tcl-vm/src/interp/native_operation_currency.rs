// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
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

/// The pinned world an entered operation is re-issued to.
#[derive(Clone)]
struct ReissuedWorld {
    context: std::sync::Arc<RuntimeContext>,
    profile: &'static tcl_dialect::DialectProfile,
}

#[derive(Clone)]
pub(crate) struct NativeOperationCurrency {
    owner: Rc<NativeOperationContext>,
    generation: u64,
    authorised_generation: Rc<Cell<Option<u64>>>,
    /// The world a sanctioned re-issue moved this operation to: a pin the host
    /// made from a native command, installed once that command returned.
    reissued_world: Rc<RefCell<Option<ReissuedWorld>>>,
    allocation_permit: Rc<Cell<bool>>,
    interpreter: NativeInterpreterIdentity,
    context: std::sync::Arc<RuntimeContext>,
    profile: &'static tcl_dialect::DialectProfile,
    engine: Option<&'static tcl_dialect::DialectProfile>,
    host: Rc<dyn tcl_platform::Host>,
    /// The interpreter's compile-service generation at capture.
    compiler_generation: u64,
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
}

struct EnteredOperation {
    currency: NativeOperationCurrency,
    allocation: crate::value::JimAllocationScope,
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
        let current = vm.cur;
        self.check_at(vm, current)
    }

    /// [`Self::check_current`] as if `interpreter` were selected.
    fn check_at(&self, vm: &mut Vm, interpreter: InterpId) -> Result<(), NativeExecutionError> {
        if let Some(first) = vm.execution_refusal.clone() {
            return Err(first);
        }
        if let Err(first) = &self.currency {
            return Err(retain_first(vm, first.clone()));
        }
        if !crate::verify_currency() {
            return Ok(());
        }
        if !self.owns_target(vm) || interpreter != self.interpreter {
            return Err(retain_first(vm, unavailable()));
        }
        match &self.currency {
            Ok(Some(currency)) => currency.check_saved_target(vm, interpreter),
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

    /// Whether this target's saved currency is the current interpreter's
    /// innermost live entry, which entering it again would reuse unchanged.
    fn entered_in_current(&self, vm: &Vm) -> bool {
        let Ok(Some(currency)) = &self.currency else {
            return false;
        };
        if vm.execution_refusal.is_some()
            || vm.cur != self.interpreter
            || self.owner != vm.owner_nonce
            || !Rc::ptr_eq(&currency.owner, &vm.native_operation_context)
        {
            return false;
        }
        let innermost = currency
            .owner
            .entered
            .borrow()
            .iter()
            .rev()
            .find_map(Weak::upgrade);
        innermost.is_some_and(|entered| {
            entered.currency.same_issue(currency) && NativeOperationScope::reusable(vm, &entered)
        })
    }

    fn settle_in_original<T>(
        &self,
        vm: &mut Vm,
        restoration: CleanupRestoration,
        publication: CleanupPublication,
        work: impl FnOnce(&mut Vm) -> T,
    ) -> Result<ReachedCleanup<Option<T>>, NativeExecutionError> {
        if !crate::VERIFY_CURRENCY
            && restoration == CleanupRestoration::PreserveCallbackSelection
            && self.entered_in_current(vm)
        {
            // Release has no re-proof to run around work that a reused entry
            // already encloses in its own interpreter.
            let mut metadata = Some(work(vm));
            if matches!(publication, CleanupPublication::OrdinaryResult)
                && vm.execution_refusal.is_some()
            {
                drop(metadata.take());
            }
            return Ok(ReachedCleanup {
                metadata,
                first_cause: vm.execution_refusal.clone(),
            });
        }
        // Preserve the callback-selected refusal before parking it. Withdrawn
        // currency still permits already-reached same-physical-target cleanup.
        // The work runs in this target, whichever interpreter selected it.
        let _ = self.check_at(vm, self.interpreter);
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
    fn same_issue(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.owner, &other.owner)
            && self.generation == other.generation
            && Rc::ptr_eq(&self.authorised_generation, &other.authorised_generation)
            && Rc::ptr_eq(&self.reissued_world, &other.reissued_world)
            && Rc::ptr_eq(&self.allocation_permit, &other.allocation_permit)
            && self.interpreter == other.interpreter
            && std::sync::Arc::ptr_eq(&self.context, &other.context)
            && std::ptr::eq(self.profile, other.profile)
            && self.engine.map(std::ptr::from_ref) == other.engine.map(std::ptr::from_ref)
            && Rc::ptr_eq(&self.host, &other.host)
    }
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
            reissued_world: Rc::new(RefCell::new(None)),
            allocation_permit,
            interpreter: vm.native_interpreter_identity(),
            context: std::sync::Arc::clone(&vm.pin.context),
            profile: vm.dialect_profile(),
            engine: vm.actual_engine_profile,
            host: vm.host_rc(),
            compiler_generation: vm.state.compiler_generation,
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
        if !crate::verify_currency() {
            return Ok(());
        }
        let world = self.reissued_world.borrow().clone();
        let (context, profile) = world.map_or_else(
            || (std::sync::Arc::clone(&self.context), self.profile),
            |world| (world.context, world.profile),
        );
        let current = self.owns_saved_target(vm, interpreter)
            && vm.st_of(interpreter).is_some_and(|state| {
                // A compile-service change is not an invariant violation:
                // release reports it as stale bytecode where bytecode
                // continues and otherwise carries on, so this re-proof leaves
                // it to them.
                state.compiler_generation != self.compiler_generation
                    || self.owner.generation.get() == Some(generation)
                        && state.pin.context == context
                        && std::ptr::eq(state.pin.profile, profile)
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

    /// An unpooled entry with the same currency is indistinguishable from a
    /// fresh one while no pool shadows it, so nested entry can share it.
    fn reusable(vm: &Vm, entered: &EnteredOperation) -> bool {
        entered.allocation.is_unpooled()
            && crate::value::JimAllocationScope::innermost_is_unpooled()
            && vm.actual_native_string_protocol()
                != Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
    }

    fn allocation_scope(
        vm: &mut Vm,
        currency: &NativeOperationCurrency,
        permit_new_context: bool,
    ) -> Result<crate::value::JimAllocationScope, NativeExecutionError> {
        let pool = if vm.actual_native_string_protocol()
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
        let Some(pool) = pool else {
            return Ok(crate::value::JimAllocationScope::unpooled());
        };
        let current = currency.clone();
        Ok(crate::value::JimAllocationScope::enter(
            Some(pool),
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
            if Self::reusable(vm, &original.entered) {
                return Ok(original);
            }
            original.currency().clone()
        } else {
            NativeOperationCurrency::capture_fields(vm)?
        };
        let allocation = Self::allocation_scope(vm, &currency, true)?;
        let entered = Rc::new(EnteredOperation {
            currency,
            allocation,
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
        let innermost = {
            let mut records = original.owner.entered.borrow_mut();
            records.retain(|record| record.strong_count() != 0);
            records.last().and_then(Weak::upgrade)
        };
        if let Some(entered) = innermost
            && entered.currency.same_issue(original)
            && Self::reusable(vm, &entered)
        {
            let _ = original.check_saved_target(vm, interpreter);
            return Ok(Self { entered });
        }
        let allocation = Self::allocation_scope(vm, original, false)?;
        let entered = Rc::new(EnteredOperation {
            currency: original.clone(),
            allocation,
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

    /// Whether a new operation can still be entered: the context epoch has not
    /// been exhausted.
    pub(crate) fn native_operation_context_available(&self) -> bool {
        self.native_operation_context.generation.get().is_some()
    }

    /// The sanctioned re-issue: move every operation entered in the current
    /// interpreter to the world it now stands on. Only a pin deferred from a
    /// native command reaches this; any other change of world still refuses
    /// the operations entered before it.
    pub(super) fn reissue_entered_native_operations(&self) {
        let owner = &self.native_operation_context;
        let Some(generation) = owner.generation.get() else {
            return;
        };
        let world = ReissuedWorld {
            context: std::sync::Arc::clone(&self.pin.context),
            profile: self.pin.profile,
        };
        let entered: Vec<_> = owner
            .entered
            .borrow()
            .iter()
            .filter_map(Weak::upgrade)
            .collect();
        for operation in entered {
            operation
                .currency
                .authorised_generation
                .set(Some(generation));
            *operation.currency.reissued_world.borrow_mut() = Some(world.clone());
        }
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
    fn original_saved_execution_target_checks_real_jim_teardown_after_restoration() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Actual dying-slot in_interp/finalize/Jim teardown path, not Native callback chronology.
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("jim"));
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
    fn original_retirement_metadata_survives_real_jim_post_restoration_refusal() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Real dying-slot teardown and retained output role; no native callback chronology.
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("jim"));
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
}
