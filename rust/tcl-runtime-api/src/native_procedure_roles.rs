// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual native procedure references and resource retirement.

use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;

/// Native references are separate from the Rc handles that keep Rust metadata
/// and allocation addresses alive. Only `NativeProcedureReference` changes them.
#[derive(Default)]
pub struct NativeProcedureRoleLedger {
    references: Cell<usize>,
    retired: Cell<bool>,
    compilation_lifetime: OnceCell<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    compilation_source:
        OnceCell<std::sync::Arc<crate::native_procedure_compilation::OriginalProcedureSource>>,
}

impl NativeProcedureRoleLedger {
    /// Issue a compilation request from this declaration's actual body and formals.
    /// Repeated preparation retains the same original shape. This carries no
    /// execution completion, variable frame, or handler admission.
    ///
    /// # Errors
    /// Rejects a retired declaration, changed original inputs, or invalid prefix.
    pub fn compilation_request(
        &self,
        issuer: &crate::native_compiler_pass::NativeCompilerPassOwner,
        input: crate::native_procedure_compilation::OriginalProcedureCompileInput,
    ) -> Result<
        crate::OriginalProcedureCompileRequest,
        crate::native_procedure_compilation::OriginalProcedureCompileError,
    > {
        use crate::native_procedure_compilation::{
            OriginalProcedureCompileError, OriginalProcedureCompileRequest,
        };
        if self.references() == 0 || self.is_retired() {
            return Err(OriginalProcedureCompileError::Retired);
        }
        let lifetime = self
            .compilation_lifetime
            .get_or_init(|| std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true)));
        OriginalProcedureCompileRequest::from_ledger(
            &self.compilation_source,
            issuer.procedure_compilation_identity(),
            std::sync::Arc::downgrade(lifetime),
            input,
        )
    }

    /// Withdraw every retained compilation context before changing an original
    /// declaration's namespace or source. Native roles and ordinary compilation
    /// remain independent; a withdrawn context is never revived.
    pub fn withdraw_compilation_custody(&self) {
        if let Some(source) = self.compilation_source.get() {
            source.withdraw();
        }
    }

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
            if let Some(lifetime) = ledger.compilation_lifetime.get() {
                lifetime.store(false, std::sync::atomic::Ordering::Release);
            }
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

    /// Extract the existing binding reference for retirement after the caller
    /// releases its command-table borrow. This transfers the actual reference;
    /// it neither acquires a role nor extends the declaration through a view.
    #[must_use]
    pub fn take_for_retirement(&self) -> Option<NativeProcedureReference<T>> {
        self.reference.borrow_mut().take()
    }

    /// Withdraw the real command binding even while dispatch views survive.
    pub fn retire(&self) {
        drop(self.take_for_retirement());
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

    fn compilation_input() -> crate::native_procedure_compilation::OriginalProcedureCompileInput {
        crate::native_procedure_compilation::OriginalProcedureCompileInput {
            original: crate::SourceImage::native(b"return $x; malformed {"),
            parameters: vec![crate::NameBytes::from("x")],
            binding_context: crate::native_procedure_compilation::OriginalProcedureBindingContext {
                parameter_grammar: tcl_dialect::ParameterGrammar::Tcl,
                has_static_bindings: false,
                has_adapter_bindings: false,
            },
            namespace: crate::native_compilation::NativeNamespaceContext {
                path: crate::ByteNamespacePath::root(),
                token: 1,
                interpreter: crate::native_compilation::NativeInterpreterIdentity {
                    owner: 11,
                    interpreter: 2,
                },
            },
            complete_prefix_len: b"return $x; ".len(),
            reported_name: "::p".into(),
        }
    }

    #[test]
    fn compilation_custody_retains_no_native_role_and_withdraws_before_free_hooks() {
        use crate::native_procedure_compilation::OriginalProcedureCompileError;
        let issuer = crate::native_compiler_pass::NativeCompilerPassOwner::new(
            compilation_input().namespace.interpreter,
        );
        let declaration = Rc::new(Declaration::default());
        assert_eq!(
            declaration
                .roles
                .compilation_request(&issuer, compilation_input())
                .unwrap_err(),
            OriginalProcedureCompileError::Retired,
        );
        let binding = NativeProcedureBinding::new(&declaration).unwrap();
        let request = declaration
            .roles
            .compilation_request(&issuer, compilation_input())
            .unwrap();
        let retained = request.clone();
        assert!(request.is_live());
        assert_eq!(declaration.roles.references(), 1);
        assert_eq!(request.target().source.bytes(), b"return $x; ");
        assert_eq!(request.original_source().bytes(), b"return $x; malformed {");
        assert_eq!(
            request.target().source.channel(),
            request.original_source().channel()
        );
        binding.retire();
        assert!(!request.is_live());
        assert!(!retained.is_live());
        assert_eq!(declaration.roles.references(), 0);
        assert_eq!(declaration.retirements.get(), 1);
        assert_eq!(
            declaration
                .roles
                .compilation_request(&issuer, compilation_input())
                .unwrap_err(),
            OriginalProcedureCompileError::Retired,
        );
    }

    fn assert_changed_original_withdraws(
        issuer: &crate::native_compiler_pass::NativeCompilerPassOwner,
        change: fn(&mut crate::native_procedure_compilation::OriginalProcedureCompileInput),
    ) {
        use crate::native_procedure_compilation::OriginalProcedureCompileError;
        let declaration = Rc::new(Declaration::default());
        let binding = NativeProcedureBinding::new(&declaration).unwrap();
        let request = declaration
            .roles
            .compilation_request(issuer, compilation_input())
            .unwrap();
        assert!(request.is_live());
        let mut changed = compilation_input();
        change(&mut changed);
        assert_eq!(
            declaration
                .roles
                .compilation_request(issuer, changed)
                .unwrap_err(),
            OriginalProcedureCompileError::ChangedOriginal,
        );
        assert!(
            !request.is_live(),
            "this payload mutation withdraws its initially live context"
        );
        assert!(request.ordinary_formals().is_none());
        assert_eq!(
            declaration
                .roles
                .compilation_request(issuer, compilation_input())
                .unwrap_err(),
            OriginalProcedureCompileError::ChangedOriginal,
            "restoring the original payload cannot revive withdrawn custody",
        );
        binding.retire();
    }

    #[test]
    fn compilation_custody_keeps_distinct_identical_declarations_and_original_payload() {
        use crate::native_procedure_compilation::OriginalProcedureCompileError;
        let issuer = crate::native_compiler_pass::NativeCompilerPassOwner::new(
            compilation_input().namespace.interpreter,
        );
        let original = Rc::new(Declaration::default());
        let original_binding = NativeProcedureBinding::new(&original).unwrap();
        let foreign = Rc::new(Declaration::default());
        let foreign_binding = NativeProcedureBinding::new(&foreign).unwrap();
        let request = original
            .roles
            .compilation_request(&issuer, compilation_input())
            .unwrap();
        let other = foreign
            .roles
            .compilation_request(&issuer, compilation_input())
            .unwrap();
        assert_ne!(request.declaration(), other.declaration());
        let mut repeated = compilation_input();
        repeated.reported_name = "::renamed".into();
        repeated.complete_prefix_len = repeated.original.len();
        let complete = original
            .roles
            .compilation_request(&issuer, repeated)
            .unwrap();
        assert_eq!(request.declaration(), complete.declaration());
        assert_eq!(
            complete.target().source.bytes(),
            complete.original_source().bytes()
        );
        assert_eq!(complete.reported_name(), "::renamed");
        let changes: &[fn(
            &mut crate::native_procedure_compilation::OriginalProcedureCompileInput,
        )] = &[
            |input| input.original = crate::SourceImage::native(b"return OTHER"),
            |input| input.parameters = vec![crate::NameBytes::from("y")],
            |input| input.binding_context.parameter_grammar = tcl_dialect::ParameterGrammar::Jim,
            |input| input.binding_context.has_static_bindings = true,
            |input| input.binding_context.has_adapter_bindings = true,
            |input| input.namespace.path = crate::ByteNamespacePath::from_segments(["foreign"]),
            |input| input.namespace.token += 1,
            |input| input.original = crate::SourceImage::document("return $x; malformed {"),
        ];
        for change in changes {
            assert_changed_original_withdraws(&issuer, *change);
        }
        let mut invalid = compilation_input();
        invalid.complete_prefix_len = invalid.original.len() + 1;
        assert_eq!(
            original
                .roles
                .compilation_request(&issuer, invalid)
                .unwrap_err(),
            OriginalProcedureCompileError::InvalidPrefix,
        );
        assert!(
            request.is_live(),
            "isolated foreign mutations do not withdraw this declaration"
        );
        original_binding.retire();
        assert!(!request.is_live());
        assert!(other.is_live());
        foreign_binding.retire();
        assert!(!other.is_live());
    }

    #[test]
    fn compilation_custody_requires_exact_live_compiler_owner() {
        use crate::native_compiler_pass::NativeCompilerPassOwner;
        use crate::native_procedure_compilation::OriginalProcedureCompileError;
        let issuer = NativeCompilerPassOwner::new(compilation_input().namespace.interpreter);
        let foreign = NativeCompilerPassOwner::new(compilation_input().namespace.interpreter);
        let environment = issuer.capture(true, false, false, None);
        let foreign_environment = foreign.capture(true, false, false, None);
        let declaration = Rc::new(Declaration::default());
        let binding = NativeProcedureBinding::new(&declaration).unwrap();
        let request = declaration
            .roles
            .compilation_request(&issuer, compilation_input())
            .unwrap();
        assert!(request.matches_compiler_environment(Some(&environment)));
        assert!(!request.matches_compiler_environment(Some(&foreign_environment)));
        assert!(!request.matches_compiler_environment(None));
        assert_eq!(
            declaration
                .roles
                .compilation_request(&foreign, compilation_input())
                .unwrap_err(),
            OriginalProcedureCompileError::ForeignIssuer,
        );
        assert!(
            request.is_live(),
            "foreign issuer cannot revoke the actual owner"
        );
        let mut changed = compilation_input();
        changed.namespace.interpreter.interpreter += 1;
        assert_eq!(
            declaration
                .roles
                .compilation_request(&issuer, changed)
                .unwrap_err(),
            OriginalProcedureCompileError::ForeignIssuer,
        );
        drop(issuer);
        assert_eq!(declaration.roles.references(), 1);
        assert!(!request.is_live());
        assert!(!request.matches_compiler_environment(Some(&environment)));
        binding.retire();
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
