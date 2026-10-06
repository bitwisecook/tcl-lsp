// SPDX-License-Identifier: AGPL-3.0-or-later
//! Namespace-name descriptors observe actual arena lifetime and parent edges.

use super::{Namespaces, NsId, GLOBAL};
use tcl_core_types::NameBytes;
use tcl_runtime_api::native_compilation::NativeInterpreterIdentity;
use tcl_runtime_api::native_namespace_name::NativeNamespaceNameToken;

impl Namespaces {
    /// Retain metadata for an actual incarnation; a same-spelled node cannot
    /// replace its identity. This lease retains no interpreter or commands.
    pub(crate) fn namespace_name_token(
        &self,
        interpreter: NativeInterpreterIdentity,
        ns: NsId,
    ) -> Option<NativeNamespaceNameToken> {
        let namespace = self.arena.get(ns)?;
        if self.native_namespace_name_dead.contains(&ns) {
            return None;
        }
        let mut ledger = self.native_namespace_names.borrow_mut();
        if let Some(token) = ledger.get(&ns) {
            return (token.interpreter() == interpreter).then(|| token.clone());
        }
        let token = NativeNamespaceNameToken::new(
            interpreter,
            u64::try_from(ns).ok()?,
            NameBytes::from(self.qualified_name(ns)),
            namespace
                .parent
                .and_then(|parent| u64::try_from(parent).ok()),
        );
        if self.dying.contains(&ns) || self.deferred.contains(&ns) {
            token.mark_dying();
        }
        if namespace.parent.is_none() {
            token.detach_parent();
        }
        ledger.insert(ns, token.clone());
        Some(token)
    }

    /// Numeric IDs and namespace reporting bytes cannot authenticate a hit.
    pub(crate) fn owns_namespace_name_token(&self, token: &NativeNamespaceNameToken) -> bool {
        usize::try_from(token.token()).ok().is_some_and(|ns| {
            self.native_namespace_names
                .borrow()
                .get(&ns)
                .is_some_and(|owned| owned.same_token(token))
        })
    }

    pub(super) fn namespace_name_begin_deletion(&self, ns: NsId) {
        if let Some(token) = self.native_namespace_names.borrow().get(&ns) {
            token.mark_dying();
        }
    }

    pub(super) fn namespace_name_detach_parent(&self, ns: NsId) {
        if let Some(token) = self.native_namespace_names.borrow().get(&ns) {
            token.detach_parent();
        }
    }

    /// Complete each actual recursive deletion before sibling callbacks run.
    pub(crate) fn namespace_name_finish_deletion(&mut self, ns: NsId) {
        if ns == GLOBAL {
            if let Some(token) = self.native_namespace_names.borrow().get(&ns) {
                let restored = token.restore_after_global_reset();
                debug_assert!(restored);
            }
        } else {
            self.native_namespace_name_dead.insert(ns);
            if let Some(token) = self.native_namespace_names.borrow().get(&ns) {
                token.mark_dead();
            }
        }
    }
}

impl Drop for Namespaces {
    fn drop(&mut self) {
        for token in self.native_namespace_names.get_mut().values() {
            token.mark_dead();
        }
    }
}
