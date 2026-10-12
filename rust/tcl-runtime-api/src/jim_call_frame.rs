// SPDX-License-Identifier: AGPL-3.0-or-later
//! Jim call-frame storage reuse, separate from activation and lookup epochs.

use std::rc::{Rc, Weak};

/// One concrete Jim call-frame storage slot. Reuse supplies no activation,
/// namespace, source-entry or variable-cache currency.
#[derive(Debug, Clone, Default)]
pub struct JimCallFrameStorageSlot(Rc<()>);

/// A link's retained frame-storage address, independently of activation birth.
#[derive(Debug, Clone)]
pub struct JimCallFrameStorageReference(Weak<()>);

impl PartialEq for JimCallFrameStorageReference {
    fn eq(&self, other: &Self) -> bool {
        Weak::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for JimCallFrameStorageReference {}

impl JimCallFrameStorageSlot {
    /// Retain the actual selected storage address without keeping its locals.
    #[must_use]
    pub fn reference(&self) -> JimCallFrameStorageReference {
        JimCallFrameStorageReference(Rc::downgrade(&self.0))
    }
    /// Exact storage correspondence; the caller independently selects a live
    /// frame and its current activation, namespace and original getter.
    #[must_use]
    pub fn matches(&self, reference: &JimCallFrameStorageReference) -> bool {
        reference
            .0
            .upgrade()
            .is_some_and(|slot| Rc::ptr_eq(&self.0, &slot))
    }
}

/// This interpreter's LIFO free storage. The selected Jim backend releases
/// actual frame owners before recycling; C and activation owners stay separate.
#[derive(Debug, Default)]
pub struct JimCallFrameStoragePool {
    free: Vec<JimCallFrameStorageSlot>,
}
impl JimCallFrameStoragePool {
    /// Select the last released concrete storage, or allocate a fresh slot.
    /// This does not reuse any activation identity or original variable table.
    pub fn acquire(&mut self) -> JimCallFrameStorageSlot {
        // naming.procedure-static.jim-original-link-frame-storage
        // docs/design/analysis/name-resolution-proofs/procedure-static-jim-original-link-frame-storage.md
        self.free.pop().unwrap_or_default()
    }
    /// Publish a slot only after its actual local-command, argument, script,
    /// namespace and variable owners have been released by the selected backend.
    pub fn recycle(&mut self, slot: JimCallFrameStorageSlot) {
        // naming.procedure-static.jim-original-link-frame-storage
        // docs/design/analysis/name-resolution-proofs/procedure-static-jim-original-link-frame-storage.md
        self.free.push(slot);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_jim_storage_reuse_keeps_live_and_foreign_slots_separate() {
        // naming.procedure-static.jim-original-link-frame-storage
        // docs/design/analysis/name-resolution-proofs/procedure-static-jim-original-link-frame-storage.md
        let mut pool = JimCallFrameStoragePool::default();
        let first = pool.acquire();
        let reference = first.reference();
        let second = pool.acquire();
        let second_reference = second.reference();
        assert!(!second.matches(&reference));
        pool.recycle(first);
        pool.recycle(second);
        let next = pool.acquire();
        assert!(next.matches(&second_reference));
        assert!(!next.matches(&reference));
        let reused = pool.acquire();
        assert!(reused.matches(&reference));
        assert!(
            !JimCallFrameStoragePool::default()
                .acquire()
                .matches(&reference)
        );
        let vanished = JimCallFrameStorageSlot::default().reference();
        assert!(!next.matches(&vanished));
    }
}
