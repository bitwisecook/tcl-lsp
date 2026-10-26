// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original interpreter environment for whole-unit native compiler passes.

use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Weak};

use crate::native_compilation::{NativeInterpreterIdentity, NativeNamespaceContext};

/// Actual command-owning namespace of the selected original procedure.
/// Anonymous procedures and source declaration candidates have no such receipt.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeCompilerPassProcedure {
    /// Same interpreter and retained namespace incarnation.
    pub namespace: NativeNamespaceContext,
    /// Original native namespace full name used by the compiler's library test.
    /// This is a presentation comparison, never a lookup input.
    pub namespace_full_name: crate::NameBytes,
}

/// Complete backend observer observation supplied at the actual capture site.
/// This input is not a lifetime, frame or effect receipt on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeOrdinaryEntryObservers {
    /// Registered, pending or firing observer absence has not been established.
    Unproven,
    /// The backend inspected every applicable observer and active callback.
    Absent,
}

impl From<bool> for NativeOrdinaryEntryObservers {
    fn from(absent: bool) -> Self {
        if absent { Self::Absent } else { Self::Unproven }
    }
}

/// Metadata lifetime owner belonging to one actual interpreter state.
/// It owns no native headers, procedure declarations or namespace resources.
#[derive(Debug)]
pub struct NativeCompilerPassOwner {
    interpreter: NativeInterpreterIdentity,
    token: u64,
    lifetime: Arc<()>,
    ordinary_entry_epoch: Arc<AtomicU64>,
}

impl NativeCompilerPassOwner {
    /// Create an independent owner for the actual interpreter state.
    ///
    /// # Panics
    /// Panics if process-unique compiler environment identities are exhausted.
    #[must_use]
    pub fn new(interpreter: NativeInterpreterIdentity) -> Self {
        use std::sync::atomic::AtomicU64;
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self {
            interpreter,
            token: crate::checked_counter::allocate(&NEXT)
                .expect("native compiler pass owner identities exhausted"),
            lifetime: Arc::new(()),
            ordinary_entry_epoch: Arc::new(AtomicU64::new(1)),
        }
    }

    /// Withdraw ordinary-entry observations before an engine observer, limit
    /// or policy mutation. Restoring the old state never revives a receipt.
    pub fn invalidate_ordinary_entry_observation(&self) {
        let _ =
            self.ordinary_entry_epoch
                .try_update(Ordering::AcqRel, Ordering::Acquire, |epoch| {
                    (epoch != 0).then(|| epoch.checked_add(1).unwrap_or(0))
                });
    }

    /// Permanently retire this interpreter's ordinary-entry observations.
    pub fn retire_ordinary_entry_observation(&self) {
        self.ordinary_entry_epoch.store(0, Ordering::Release);
    }

    /// Capture actual complete absence of implicit ordinary-entry observers.
    /// The backend must inspect registered, pending and firing callbacks.
    /// This supplies neither a fresh frame nor formals, storage or effects.
    #[must_use]
    pub fn capture_with_ordinary_entry_observation(
        &self,
        root_interpreter: bool,
        command_limit_enabled: bool,
        time_limit_enabled: bool,
        procedure: Option<NativeCompilerPassProcedure>,
        entry_observers: NativeOrdinaryEntryObservers,
    ) -> NativeCompilerPassEnvironment {
        let mut environment = self.capture(
            root_interpreter,
            command_limit_enabled,
            time_limit_enabled,
            procedure,
        );
        let epoch = self.ordinary_entry_epoch.load(Ordering::Acquire);
        if entry_observers == NativeOrdinaryEntryObservers::Absent
            && !environment.has_enabled_limits()
            && epoch != 0
        {
            environment.ordinary_entry_observation = Some(OrdinaryEntryObservation {
                epoch,
                currency: Arc::downgrade(&self.ordinary_entry_epoch),
            });
        }
        environment
    }

    pub(crate) fn ordinary_entry_generation(&self) -> Option<OrdinaryEntryObservation> {
        let epoch = self.ordinary_entry_epoch.load(Ordering::Acquire);
        (epoch != 0).then(|| OrdinaryEntryObservation {
            epoch,
            currency: Arc::downgrade(&self.ordinary_entry_epoch),
        })
    }

    pub(crate) fn procedure_compilation_identity(&self) -> NativeCompilerPassIdentity {
        NativeCompilerPassIdentity {
            interpreter: self.interpreter,
            token: self.token,
            lifetime: Arc::downgrade(&self.lifetime),
        }
    }

    /// Capture independently observed parent, enabled limits and procedure owner.
    /// The backend must use its actual interpreter state, not a logical profile.
    #[must_use]
    pub fn capture(
        &self,
        root_interpreter: bool,
        command_limit_enabled: bool,
        time_limit_enabled: bool,
        procedure: Option<NativeCompilerPassProcedure>,
    ) -> NativeCompilerPassEnvironment {
        NativeCompilerPassEnvironment {
            interpreter: self.interpreter,
            token: self.token,
            root_interpreter,
            command_limit_enabled,
            time_limit_enabled,
            procedure,
            ordinary_entry_observation: None,
            lifetime: Arc::downgrade(&self.lifetime),
        }
    }
}

/// Private relation to the actual interpreter-owned compiler environment.
/// A captured environment never exposes the owner needed to issue this identity.
#[derive(Debug, Clone)]
pub(crate) struct NativeCompilerPassIdentity {
    pub(crate) interpreter: NativeInterpreterIdentity,
    token: u64,
    lifetime: Weak<()>,
}

impl NativeCompilerPassIdentity {
    pub(crate) fn is_live(&self) -> bool {
        self.lifetime.strong_count() != 0
    }

    pub(crate) fn same_owner(&self, other: &Self) -> bool {
        self.interpreter == other.interpreter
            && self.token == other.token
            && Weak::ptr_eq(&self.lifetime, &other.lifetime)
    }

    pub(crate) fn matches_environment(&self, environment: &NativeCompilerPassEnvironment) -> bool {
        self.token == environment.token
            && Weak::ptr_eq(&self.lifetime, &environment.lifetime)
            && environment.is_current_for(self.interpreter)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct OrdinaryEntryObservation {
    epoch: u64,
    currency: Weak<AtomicU64>,
}

impl OrdinaryEntryObservation {
    pub(crate) fn is_current(&self) -> bool {
        self.epoch != 0
            && self
                .currency
                .upgrade()
                .is_some_and(|current| current.load(Ordering::Acquire) == self.epoch)
    }
}

impl PartialEq for OrdinaryEntryObservation {
    fn eq(&self, other: &Self) -> bool {
        self.epoch == other.epoch && Weak::ptr_eq(&self.currency, &other.currency)
    }
}
impl Eq for OrdinaryEntryObservation {}
impl Hash for OrdinaryEntryObservation {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.epoch.hash(state);
    }
}

/// Original compiler environment observed while its interpreter remains alive.
/// Capturing this receipt does not retain the interpreter or native resources.
#[derive(Debug, Clone)]
pub struct NativeCompilerPassEnvironment {
    interpreter: NativeInterpreterIdentity,
    token: u64,
    root_interpreter: bool,
    command_limit_enabled: bool,
    time_limit_enabled: bool,
    procedure: Option<NativeCompilerPassProcedure>,
    ordinary_entry_observation: Option<OrdinaryEntryObservation>,
    lifetime: Weak<()>,
}

impl NativeCompilerPassEnvironment {
    /// Same actual interpreter and unretired state owner.
    #[must_use]
    pub fn is_current_for(&self, interpreter: NativeInterpreterIdentity) -> bool {
        self.interpreter == interpreter
            && self.lifetime.strong_count() != 0
            && self
                .procedure
                .as_ref()
                .is_none_or(|procedure| procedure.namespace.interpreter == interpreter)
    }

    /// Current complete absence of implicit callbacks before an ordinary entry.
    /// The same actual request owner and fresh ordinary-frame induction remain
    /// independent requirements; this is no Store or completion authority.
    #[must_use]
    pub fn ordinary_entry_observers_absent_for(
        &self,
        interpreter: NativeInterpreterIdentity,
    ) -> bool {
        self.is_current_for(interpreter)
            && self
                .ordinary_entry_observation
                .as_ref()
                .is_some_and(OrdinaryEntryObservation::is_current)
    }

    /// Actual interpreter has no parent.
    #[must_use]
    pub const fn is_root(&self) -> bool {
        self.root_interpreter
    }

    /// Either native command-count or time limit is enabled.
    #[must_use]
    pub const fn has_enabled_limits(&self) -> bool {
        self.command_limit_enabled || self.time_limit_enabled
    }

    /// Actual selected original procedure's command-owning namespace, if any.
    #[must_use]
    pub const fn procedure(&self) -> Option<&NativeCompilerPassProcedure> {
        self.procedure.as_ref()
    }

    /// Retain the observed environment without borrowing a procedure owner.
    /// Source declaration candidates cannot claim the actual library exception.
    #[must_use]
    pub fn without_procedure(&self) -> Self {
        Self {
            procedure: None,
            ..self.clone()
        }
    }
}

impl PartialEq for NativeCompilerPassEnvironment {
    fn eq(&self, other: &Self) -> bool {
        self.interpreter == other.interpreter
            && self.token == other.token
            && self.root_interpreter == other.root_interpreter
            && self.command_limit_enabled == other.command_limit_enabled
            && self.time_limit_enabled == other.time_limit_enabled
            && self.procedure == other.procedure
            && self.ordinary_entry_observation == other.ordinary_entry_observation
    }
}
impl Eq for NativeCompilerPassEnvironment {}
impl Hash for NativeCompilerPassEnvironment {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.interpreter.hash(state);
        self.token.hash(state);
        self.root_interpreter.hash(state);
        self.command_limit_enabled.hash(state);
        self.time_limit_enabled.hash(state);
        self.procedure.hash(state);
        self.ordinary_entry_observation.hash(state);
    }
}

#[cfg(test)]
mod ordinary_entry_observation_tests {
    use super::*;
    use std::collections::hash_map::DefaultHasher;

    fn interpreter() -> NativeInterpreterIdentity {
        NativeInterpreterIdentity {
            owner: 1,
            interpreter: 0,
        }
    }
    fn hash(value: &NativeCompilerPassEnvironment) -> u64 {
        let mut hash = DefaultHasher::new();
        value.hash(&mut hash);
        hash.finish()
    }
    #[test]
    fn old_capture_foreign_owner_and_limits_never_supply_observer_absence() {
        let id = interpreter();
        let owner = NativeCompilerPassOwner::new(id);
        let foreign = NativeCompilerPassOwner::new(id);
        assert!(
            !owner
                .capture(true, false, false, None)
                .ordinary_entry_observers_absent_for(id)
        );
        let observed = owner.capture_with_ordinary_entry_observation(
            true,
            false,
            false,
            None,
            NativeOrdinaryEntryObservers::Absent,
        );
        assert!(observed.ordinary_entry_observers_absent_for(id));
        assert!(
            !observed.ordinary_entry_observers_absent_for(NativeInterpreterIdentity {
                owner: 2,
                interpreter: 0
            })
        );
        assert!(
            !foreign
                .procedure_compilation_identity()
                .matches_environment(&observed)
        );
        for (commands, time) in [(true, false), (false, true)] {
            assert!(
                !owner
                    .capture_with_ordinary_entry_observation(
                        true,
                        commands,
                        time,
                        None,
                        NativeOrdinaryEntryObservers::Absent
                    )
                    .ordinary_entry_observers_absent_for(id)
            );
        }
        assert!(
            !owner
                .capture_with_ordinary_entry_observation(
                    true,
                    false,
                    false,
                    None,
                    NativeOrdinaryEntryObservers::Unproven
                )
                .ordinary_entry_observers_absent_for(id)
        );
    }
    #[test]
    fn mutation_restore_retirement_and_exhaustion_keep_captured_hash_immutable() {
        let id = interpreter();
        let owner = NativeCompilerPassOwner::new(id);
        let original = owner.capture_with_ordinary_entry_observation(
            true,
            false,
            false,
            None,
            NativeOrdinaryEntryObservers::Absent,
        );
        let same = original.clone();
        let before = hash(&original);
        owner.invalidate_ordinary_entry_observation();
        assert!(!original.ordinary_entry_observers_absent_for(id));
        assert_eq!(original, same);
        assert_eq!(hash(&original), before);
        let restored = owner.capture_with_ordinary_entry_observation(
            true,
            false,
            false,
            None,
            NativeOrdinaryEntryObservers::Absent,
        );
        assert!(restored.ordinary_entry_observers_absent_for(id));
        assert_ne!(restored, original);
        owner.retire_ordinary_entry_observation();
        assert!(!restored.ordinary_entry_observers_absent_for(id));
        assert!(
            !owner
                .capture_with_ordinary_entry_observation(
                    true,
                    false,
                    false,
                    None,
                    NativeOrdinaryEntryObservers::Absent
                )
                .ordinary_entry_observers_absent_for(id)
        );
        let exhausted = NativeCompilerPassOwner::new(id);
        exhausted
            .ordinary_entry_epoch
            .store(u64::MAX, Ordering::Release);
        let last = exhausted.capture_with_ordinary_entry_observation(
            true,
            false,
            false,
            None,
            NativeOrdinaryEntryObservers::Absent,
        );
        exhausted.invalidate_ordinary_entry_observation();
        exhausted.invalidate_ordinary_entry_observation();
        assert!(!last.ordinary_entry_observers_absent_for(id));
        assert!(
            !exhausted
                .capture_with_ordinary_entry_observation(
                    true,
                    false,
                    false,
                    None,
                    NativeOrdinaryEntryObservers::Absent
                )
                .ordinary_entry_observers_absent_for(id)
        );
        drop(owner);
        assert!(!original.is_current_for(id));
    }
}
