// SPDX-License-Identifier: AGPL-3.0-or-later
//! Retained native namespace-name descriptors without interpreter ownership.

use std::{cell::Cell, rc::Rc};

use crate::native_compilation::NativeInterpreterIdentity;
use tcl_core_types::NameBytes;
use tcl_syntax::native_namespace_name::{NativeNamespaceLifecycle, NativeNamespaceNameRecipe};

struct NamespaceState {
    interpreter: NativeInterpreterIdentity,
    token: u64,
    full_name: NameBytes,
    lifecycle: Cell<NativeNamespaceLifecycle>,
    parent: Cell<Option<u64>>,
}

/// Actual namespace-owner state retained independently of callable lifetimes.
/// A live owner must compare the closed state identity with its own ledger;
/// numeric token equality or a constructed reporting name grants no authority.
#[derive(Clone)]
pub struct NativeNamespaceNameToken(Rc<NamespaceState>);

impl NativeNamespaceNameToken {
    /// Mint state for one actual namespace incarnation in the owning ledger.
    #[must_use]
    pub fn new(
        interpreter: NativeInterpreterIdentity,
        token: u64,
        full_name: NameBytes,
        parent: Option<u64>,
    ) -> Self {
        Self(Rc::new(NamespaceState {
            interpreter,
            token,
            full_name,
            lifecycle: Cell::new(NativeNamespaceLifecycle::Live),
            parent: Cell::new(parent),
        }))
    }

    /// Actual interpreter owning this original namespace incarnation.
    #[must_use]
    pub fn interpreter(&self) -> NativeInterpreterIdentity {
        self.0.interpreter
    }
    /// Actual never-reused namespace incarnation.
    #[must_use]
    pub fn token(&self) -> u64 {
        self.0.token
    }
    /// Original counted native full-name reporting storage.
    #[must_use]
    pub fn full_name(&self) -> &NameBytes {
        &self.0.full_name
    }
    /// Actual native lifetime, separate from public unpublication.
    #[must_use]
    pub fn lifecycle(&self) -> NativeNamespaceLifecycle {
        self.0.lifecycle.get()
    }
    /// Actual physical parent; deletion can detach it while fullName remains.
    #[must_use]
    pub fn parent(&self) -> Option<u64> {
        self.0.parent.get()
    }
    /// Advance the actual namespace owner's deletion state.
    pub fn mark_dying(&self) {
        if self.lifecycle() != NativeNamespaceLifecycle::Dead {
            self.0.lifecycle.set(NativeNamespaceLifecycle::Dying);
        }
    }
    /// Detach the actual native parent edge independently of deletion flags.
    /// Deferred deletion detaches immediately; synchronous teardown owns its order.
    pub fn detach_parent(&self) {
        self.0.parent.set(None);
    }
    /// Restore the same global incarnation after its completed namespace reset.
    /// Only a dying root can be restored; dead tokens never revive.
    #[must_use]
    pub fn restore_after_global_reset(&self) -> bool {
        if self.token() != 0 || self.lifecycle() != NativeNamespaceLifecycle::Dying {
            return false;
        }
        self.0.lifecycle.set(NativeNamespaceLifecycle::Live);
        true
    }
    /// Invalidate this incarnation after actual teardown or interpreter retirement.
    pub fn mark_dead(&self) {
        self.0.lifecycle.set(NativeNamespaceLifecycle::Dead);
        self.0.parent.set(None);
    }
    /// Compare the actual retained state, independently of names and numeric IDs.
    #[must_use]
    pub fn same_token(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl std::fmt::Debug for NativeNamespaceNameToken {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NativeNamespaceNameToken")
            .field("interpreter", &self.interpreter())
            .field("token", &self.token())
            .field("lifecycle", &self.lifecycle())
            .finish_non_exhaustive()
    }
}

struct ResolvedNamespaceName {
    namespace: NativeNamespaceNameToken,
    reference: Option<NativeNamespaceNameToken>,
}

/// An original physical nsName primary and its shared resolved descriptor.
/// A NULL descriptor remains a distinct C8.4 primary. Duplication shares this
/// descriptor, but never keeps the actual interpreter or its callables alive.
#[derive(Clone)]
pub struct NativeNamespaceNameCache {
    interpreter: NativeInterpreterIdentity,
    version: tcl_dialect::TclVersion,
    resolved: Option<Rc<ResolvedNamespaceName>>,
}

impl NativeNamespaceNameCache {
    /// Retain an actual resolved descriptor after the owner's checked lookup.
    #[must_use]
    pub fn resolved(
        version: tcl_dialect::TclVersion,
        namespace: NativeNamespaceNameToken,
        reference: Option<NativeNamespaceNameToken>,
    ) -> Self {
        Self {
            interpreter: namespace.interpreter(),
            version,
            resolved: Some(Rc::new(ResolvedNamespaceName {
                namespace,
                reference,
            })),
        }
    }
    /// Construct the C8.4 NULL descriptor after an actual missing conversion.
    #[must_use]
    pub fn unresolved(interpreter: NativeInterpreterIdentity) -> Self {
        Self {
            interpreter,
            version: tcl_dialect::TclVersion::V8_4,
            resolved: None,
        }
    }
    /// Original physical C issuer of this primary.
    #[must_use]
    pub fn version(&self) -> tcl_dialect::TclVersion {
        self.version
    }
    /// Original actual interpreter scope of this descriptor.
    #[must_use]
    pub fn interpreter(&self) -> NativeInterpreterIdentity {
        self.interpreter
    }
    /// Actual target state, or the C8.4 unresolved NULL descriptor.
    #[must_use]
    pub fn namespace(&self) -> Option<&NativeNamespaceNameToken> {
        self.resolved.as_ref().map(|resolved| &resolved.namespace)
    }
    /// Actual lookup reference context; later absolute names have none.
    #[must_use]
    pub fn reference(&self) -> Option<&NativeNamespaceNameToken> {
        self.resolved
            .as_ref()
            .and_then(|resolved| resolved.reference.as_ref())
    }
    /// Validate scope, native lifecycle and actual retained reference context.
    /// The owning engine must separately verify target state membership in its ledger.
    #[must_use]
    pub fn is_current(
        &self,
        recipe: NativeNamespaceNameRecipe,
        interpreter: NativeInterpreterIdentity,
        reference: &NativeNamespaceNameToken,
    ) -> bool {
        self.version == recipe.version()
            && self.interpreter == interpreter
            && self
                .namespace()
                .is_some_and(|namespace| recipe.permits_target(namespace.lifecycle()))
            && self
                .reference()
                .is_none_or(|retained| retained.same_token(reference))
    }
    /// The C8.4 updater's native bytes, independently of resident original spelling.
    /// `None` denotes a release without an updater, not empty native string storage.
    #[must_use]
    pub fn string_update_bytes(&self, recipe: NativeNamespaceNameRecipe) -> Option<&[u8]> {
        if self.version != recipe.version() || !recipe.has_string_updater() {
            return None;
        }
        Some(
            self.namespace()
                .filter(|namespace| namespace.lifecycle() != NativeNamespaceLifecycle::Dead)
                .map_or(b"".as_slice(), |namespace| namespace.full_name().as_bytes()),
        )
    }
    /// Whether genuine duplicate headers share the same resolved descriptor.
    #[must_use]
    pub fn same_descriptor(&self, other: &Self) -> bool {
        match (&self.resolved, &other.resolved) {
            (Some(left), Some(right)) => Rc::ptr_eq(left, right),
            (None, None) => self.version == other.version && self.interpreter == other.interpreter,
            _ => false,
        }
    }
}

impl std::fmt::Debug for NativeNamespaceNameCache {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NativeNamespaceNameCache")
            .field("interpreter", &self.interpreter)
            .field("version", &self.version)
            .field("namespace", &self.namespace())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        NativeInterpreterIdentity, NativeNamespaceNameCache as Cache,
        NativeNamespaceNameToken as Token,
    };
    use tcl_core_types::NameBytes;
    use tcl_dialect::TclVersion;
    use tcl_syntax::native_namespace_name::NativeNamespaceNameRecipe as Recipe;

    #[test]
    fn native_global_reset_restores_same_incarnation_without_dead_revival() {
        let interpreter = NativeInterpreterIdentity {
            owner: 3,
            interpreter: 0,
        };
        let root = Token::new(interpreter, 0, NameBytes::from("::"), None);
        let cache = Cache::resolved(TclVersion::V9_0, root.clone(), None);
        root.mark_dying();
        assert_eq!(root.parent(), None);
        assert!(root.restore_after_global_reset());
        assert!(cache.is_current(
            Recipe::for_tcl_version(TclVersion::V9_0),
            interpreter,
            &root
        ));
        root.mark_dead();
        assert!(!root.restore_after_global_reset());
        let child = Token::new(interpreter, 1, NameBytes::from("::n"), Some(0));
        child.mark_dying();
        assert_eq!(child.parent(), Some(0));
        assert!(!child.restore_after_global_reset());
        child.detach_parent();
        assert_eq!(child.parent(), None);
    }

    #[test]
    fn original_descriptor_survives_unaddressable_reporting_and_real_duplication() {
        let interpreter = NativeInterpreterIdentity {
            owner: 1,
            interpreter: 0,
        };
        let root = Token::new(interpreter, 0, NameBytes::from("::"), None);
        let child = Token::new(interpreter, 2, NameBytes::from("::a:::q"), Some(1));
        let cache = Cache::resolved(TclVersion::V9_0, child.clone(), None);
        let duplicate = cache.clone();
        assert!(cache.same_descriptor(&duplicate));
        assert!(cache.is_current(
            Recipe::for_tcl_version(TclVersion::V9_0),
            interpreter,
            &root
        ));
        let fresh = Token::new(interpreter, 2, NameBytes::from("::a:::q"), Some(1));
        assert!(!child.same_token(&fresh));
        child.mark_dying();
        child.detach_parent();
        assert!(!duplicate.is_current(
            Recipe::for_tcl_version(TclVersion::V9_0),
            interpreter,
            &root
        ));
        assert_eq!(child.parent(), None);
        assert_eq!(child.full_name().as_bytes(), b"::a:::q");
    }

    #[test]
    fn tcl84_dying_cache_and_updater_are_independent_of_public_lookup() {
        let interpreter = NativeInterpreterIdentity {
            owner: 2,
            interpreter: 0,
        };
        let root = Token::new(interpreter, 0, NameBytes::from("::"), None);
        let child = Token::new(interpreter, 1, NameBytes::from("::Z"), Some(0));
        let recipe = Recipe::for_tcl_version(TclVersion::V8_4);
        let cache = Cache::resolved(TclVersion::V8_4, child.clone(), Some(root.clone()));
        child.mark_dying();
        child.detach_parent();
        assert!(cache.is_current(recipe, interpreter, &root));
        assert_eq!(child.parent(), None);
        assert_eq!(cache.string_update_bytes(recipe), Some(b"::Z".as_slice()));
        child.mark_dead();
        assert!(!cache.is_current(recipe, interpreter, &root));
        assert_eq!(cache.string_update_bytes(recipe), Some(b"".as_slice()));
        let unresolved = Cache::unresolved(interpreter);
        assert_eq!(unresolved.version(), TclVersion::V8_4);
        assert!(unresolved.namespace().is_none());
        assert_eq!(unresolved.string_update_bytes(recipe), Some(b"".as_slice()));
    }

    #[test]
    fn relative_reference_and_interpreter_use_closed_original_identity() {
        let interpreter = NativeInterpreterIdentity {
            owner: 3,
            interpreter: 0,
        };
        let root = Token::new(interpreter, 0, NameBytes::from("::"), None);
        let context = Token::new(interpreter, 1, NameBytes::from("::A"), Some(0));
        let target = Token::new(interpreter, 2, NameBytes::from("::A::x"), Some(1));
        let recipe = Recipe::for_tcl_version(TclVersion::V8_6);
        let cache = Cache::resolved(TclVersion::V8_6, target, Some(context.clone()));
        assert!(cache.is_current(recipe, interpreter, &context));
        assert!(!cache.is_current(recipe, interpreter, &root));
        let same_numeric_context = Token::new(interpreter, 1, NameBytes::from("::A"), Some(0));
        assert!(!cache.is_current(recipe, interpreter, &same_numeric_context));
        assert!(!cache.is_current(
            recipe,
            NativeInterpreterIdentity {
                owner: 3,
                interpreter: 1
            },
            &context
        ));
        assert_eq!(cache.string_update_bytes(recipe), None);
    }
}
