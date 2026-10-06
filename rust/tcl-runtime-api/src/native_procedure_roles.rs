// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual native procedure references and resource retirement.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Native references are separate from the Rc handles that keep Rust metadata
/// and allocation addresses alive. Only `NativeProcedureReference` changes them.
#[derive(Default)]
pub struct NativeProcedureRoleLedger {
    references: Cell<usize>,
    retired: Cell<bool>,
}

impl NativeProcedureRoleLedger {
    /// Number of genuine command, activation, and internal-representation roles.
    #[must_use]
    pub fn references(&self) -> usize {
        self.references.get()
    }

    /// A final native release permanently withdraws the declaration's resources.
    #[must_use]
    pub fn is_retired(&self) -> bool {
        self.retired.get()
    }
}

/// A backend declaration whose final native reference frees its actual body,
/// defaults, caches, and other owned native resources. Surviving Rc metadata
/// transports must not retain those native references or resurrect the owner.
pub trait NativeProcedureRoleOwner {
    /// The declaration's independently retained native role ledger.
    fn native_procedure_role_ledger(&self) -> &NativeProcedureRoleLedger;

    /// Called exactly once after the final native role is withdrawn.
    /// Native resources must be taken out of the owner before their free hooks
    /// run, without holding a resource borrow across those hooks.
    fn retire_native_procedure_resources(owner: &Rc<Self>);
}

/// A transport attempted to acquire resources after their native retirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeProcedureRoleUnavailable;

impl std::fmt::Display for NativeProcedureRoleUnavailable {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("native procedure declaration has retired")
    }
}
impl std::error::Error for NativeProcedureRoleUnavailable {}

/// One actual command-binding, active-frame, or cache reference. Query and
/// dispatch handles retain Rc<T> directly and do not acquire this capsule.
pub struct NativeProcedureReference<T: NativeProcedureRoleOwner> {
    owner: Rc<T>,
    live: Cell<bool>,
}

impl<T: NativeProcedureRoleOwner> NativeProcedureReference<T> {
    /// Acquire a genuine native role on an independently selected declaration.
    /// This does not grant compiler, frame, or runtime handler authority.
    ///
    /// # Errors
    /// A permanently retired declaration cannot acquire another native role.
    pub fn acquire(owner: &Rc<T>) -> Result<Self, NativeProcedureRoleUnavailable> {
        let ledger = owner.native_procedure_role_ledger();
        if ledger.retired.get() {
            return Err(NativeProcedureRoleUnavailable);
        }
        ledger.references.set(
            ledger
                .references
                .get()
                .checked_add(1)
                .expect("native procedure references exhausted"),
        );
        Ok(Self {
            owner: Rc::clone(owner),
            live: Cell::new(true),
        })
    }

    /// Borrow the original declaration without acquiring another native role.
    #[must_use]
    pub fn owner(&self) -> &Rc<T> {
        &self.owner
    }

    /// Release the role at its actual native lifecycle frontier. The operation
    /// is idempotent so an explicitly released frame capsule can still drop.
    pub fn release(&self) {
        if !self.live.replace(false) {
            return;
        }
        let ledger = self.owner.native_procedure_role_ledger();
        let remaining = ledger
            .references
            .get()
            .checked_sub(1)
            .expect("native procedure reference underflow");
        ledger.references.set(remaining);
        if remaining == 0 {
            ledger.retired.set(true);
            T::retire_native_procedure_resources(&self.owner);
        }
    }
}

impl<T: NativeProcedureRoleOwner> Clone for NativeProcedureReference<T> {
    fn clone(&self) -> Self {
        assert!(
            self.live.get(),
            "duplicating released native procedure role"
        );
        Self::acquire(&self.owner).expect("live native procedure role")
    }
}

impl<T: NativeProcedureRoleOwner> Drop for NativeProcedureReference<T> {
    fn drop(&mut self) {
        self.release();
    }
}

/// One native command-binding role shared by all command-table, import, and
/// dispatch views of that binding. Cloning this owner is a lifetime transport;
/// it does not clone the native declaration reference.
pub struct NativeProcedureBinding<T: NativeProcedureRoleOwner> {
    reference: Rc<RefCell<Option<NativeProcedureReference<T>>>>,
}

impl<T: NativeProcedureRoleOwner> NativeProcedureBinding<T> {
    /// Publish one actual binding reference to a live declaration.
    ///
    /// # Errors
    /// A retired declaration cannot be installed as a native binding.
    pub fn new(owner: &Rc<T>) -> Result<Self, NativeProcedureRoleUnavailable> {
        Ok(Self {
            reference: Rc::new(RefCell::new(Some(NativeProcedureReference::acquire(
                owner,
            )?))),
        })
    }

    /// Borrow the binding's current declaration through a lifetime-only Rc.
    #[must_use]
    pub fn owner(&self) -> Option<Rc<T>> {
        self.reference
            .borrow()
            .as_ref()
            .map(|role| Rc::clone(role.owner()))
    }

    /// Withdraw the real command binding even while dispatch views survive.
    pub fn retire(&self) {
        let role = self.reference.borrow_mut().take();
        drop(role);
    }

    /// Transfer a freshly published replacement's actual binding role into
    /// this same command token. Views observe the new declaration before the
    /// old role's native resource retirement runs.
    ///
    /// # Errors
    /// A replacement without a live binding cannot supply a native role.
    pub fn replace_from(&self, replacement: &Self) -> Result<(), NativeProcedureRoleUnavailable> {
        let next = replacement
            .reference
            .borrow_mut()
            .take()
            .ok_or(NativeProcedureRoleUnavailable)?;
        let previous = self.reference.borrow_mut().replace(next);
        drop(previous);
        Ok(())
    }
}

impl<T: NativeProcedureRoleOwner> Clone for NativeProcedureBinding<T> {
    fn clone(&self) -> Self {
        Self {
            reference: Rc::clone(&self.reference),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    struct Declaration {
        roles: NativeProcedureRoleLedger,
        resources: RefCell<Option<Rc<()>>>,
        retirements: Cell<usize>,
    }
    impl NativeProcedureRoleOwner for Declaration {
        fn native_procedure_role_ledger(&self) -> &NativeProcedureRoleLedger {
            &self.roles
        }
        fn retire_native_procedure_resources(owner: &Rc<Self>) {
            let resources = owner.resources.borrow_mut().take();
            owner.retirements.set(owner.retirements.get() + 1);
            drop(resources);
        }
    }

    #[test]
    fn transport_views_do_not_hold_native_resources_after_final_release() {
        let resource = Rc::new(());
        let declaration = Rc::new(Declaration::default());
        *declaration.resources.borrow_mut() = Some(Rc::clone(&resource));
        let binding = NativeProcedureReference::acquire(&declaration).unwrap();
        let query = Rc::clone(&declaration);
        assert_eq!(query.roles.references(), 1);
        let active = binding.clone();
        binding.release();
        assert_eq!(query.roles.references(), 1);
        assert_eq!(Rc::strong_count(&resource), 2);
        active.release();
        assert_eq!(query.roles.references(), 0);
        assert_eq!(Rc::strong_count(&resource), 1);
        assert_eq!(query.retirements.get(), 1);
        assert!(query.roles.is_retired());
        assert!(NativeProcedureReference::acquire(&query).is_err());
        drop(binding);
        drop(active);
        assert_eq!(query.retirements.get(), 1);
    }

    #[test]
    fn binding_replacement_transfers_one_role_and_updates_all_views() {
        let original = Rc::new(Declaration::default());
        let binding = NativeProcedureBinding::new(&original).unwrap();
        let imported = binding.clone();
        let active = NativeProcedureReference::acquire(&original).unwrap();
        assert_eq!(original.roles.references(), 2);
        let replacement = Rc::new(Declaration::default());
        let next = NativeProcedureBinding::new(&replacement).unwrap();
        binding.replace_from(&next).unwrap();
        assert!(next.owner().is_none());
        assert!(Rc::ptr_eq(&imported.owner().unwrap(), &replacement));
        assert_eq!(original.roles.references(), 1);
        assert_eq!(replacement.roles.references(), 1);
        drop(active);
        assert_eq!(original.retirements.get(), 1);
        imported.retire();
        assert_eq!(replacement.retirements.get(), 1);
        assert!(binding.owner().is_none());
    }
}
