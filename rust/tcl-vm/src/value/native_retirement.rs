// SPDX-License-Identifier: AGPL-3.0-or-later
//! Retired native headers release owned value graphs without recursive descent.

use super::{DoubleFormatContext, IntRep, NativeJimObjectContext, RawString};
use std::cell::{Cell, RefCell};
use std::rc::Weak;
use tcl_runtime_api::script_source_location::ScriptSourceLocation;

pub(super) struct RetiredNativeHeader {
    pub primary: IntRep,
    pub string: Option<RawString>,
    pub format: Option<DoubleFormatContext>,
    pub location: Option<ScriptSourceLocation>,
    pub context: Option<Weak<NativeJimObjectContext>>,
}

thread_local! {
    static PENDING: RefCell<Vec<RetiredNativeHeader>> = const { RefCell::new(Vec::new()) };
    static DRAINING: Cell<bool> = const { Cell::new(false) };
}

struct RetirementDrain;
impl Drop for RetirementDrain {
    fn drop(&mut self) {
        let _ = DRAINING.try_with(|draining| draining.set(false));
    }
}

/// Release a detached header through the thread's pending queue, so a deep
/// value retires iteratively rather than by recursion.
///
/// During thread exit the queue may already be destroyed — a `Vm` held by
/// another thread-local (a pack hook host) can drop after it. Touching a
/// destroyed thread-local panics inside a destructor, which aborts the
/// process, so then the header is leaked instead: the thread is ending, and
/// its values hold only memory.
pub(super) fn release(header: RetiredNativeHeader) {
    let mut header = Some(header);
    let queued = PENDING
        .try_with(|pending| pending.borrow_mut().extend(header.take()))
        .is_ok()
        && header.is_none();
    if !queued {
        std::mem::forget(header);
        return;
    }
    if DRAINING
        .try_with(|draining| draining.replace(true))
        .unwrap_or(true)
    {
        return;
    }
    let _drain = RetirementDrain;
    loop {
        let Some(RetiredNativeHeader {
            primary,
            string,
            format,
            location,
            context,
        }) = PENDING
            .try_with(|pending| pending.borrow_mut().pop())
            .ok()
            .flatten()
        else {
            break;
        };
        // Child native releases can enqueue more detached headers. No pending
        // table guard survives while their physical fields are destroyed.
        drop((primary, string, format, location, context));
    }
}

#[cfg(test)]
mod tests {
    use crate::Value;

    #[test]
    fn deep_list_retirement_retires_native_headers_but_keeps_lifetime_leases() {
        let leaf = Value::new_native_string_bytes(b"leaf".as_slice());
        let lease = leaf.native_lifetime_lease();
        let weak = leaf.downgrade_native_object();
        let mut whole = leaf;
        for _ in 0..4_096 {
            whole = Value::list(vec![whole]);
        }
        assert!(lease.value().native_object_is_live());
        assert_eq!(lease.value().native_object_reference_count(), 1);
        drop(whole);
        assert!(!lease.value().native_object_is_live());
        assert_eq!(lease.value().native_object_reference_count(), 0);
        assert!(weak.upgrade().is_none());
        drop(lease);
        assert!(weak.0.upgrade().is_none());
    }

    #[test]
    fn deep_dictionary_retirement_preserves_a_real_external_member_reference() {
        let leaf = Value::new_native_string_bytes(b"leaf".as_slice());
        let weak = leaf.downgrade_native_object();
        let key = Value::new_native_string_bytes(b"key".as_slice());
        let mut whole = leaf.clone();
        for _ in 0..4_096 {
            whole = Value::dict(vec![(key.clone(), whole)]);
        }
        assert_eq!(leaf.native_object_reference_count(), 2);
        drop(whole);
        assert!(leaf.native_object_is_live());
        assert_eq!(leaf.native_object_reference_count(), 1);
        assert_eq!(leaf.resident_string_bytes().unwrap().as_ref(), b"leaf");
        assert_eq!(key.native_object_reference_count(), 1);
        drop(leaf);
        assert!(weak.upgrade().is_none());
    }
}
