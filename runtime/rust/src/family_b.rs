//! Family-B interp-state trait impls (`tcl-runtime-api`).
//!
//! The runtime satisfies the shared state-mutation contract over its
//! `*mut TclObj` value model, so a consumer of the `tcl-runtime-api` role
//! traits can reach into this runtime's state with the *same* contract the
//! bytecode VM (`tcl-vm`) satisfies over `Rc<Obj>`. This is the runtime's first
//! Family-B impl; further role traits (`Frames`/`Namespaces`/`Traces`/
//! `Introspect`) follow as their handle model is reconciled with this runtime's
//! arena/level addressing.

use tcl_runtime_api::{FrameId, VarStore};

use crate::interp::Interp;
use crate::obj::TclObj;

/// The Family-B variable store over the active call frame.
///
/// Matching the bytecode VM's impl, `FrameId` is not yet honoured — both
/// runtimes resolve against the *current* frame today (frame-addressed access
/// is future work, gated on reconciling this runtime's logical-level addressing
/// with the VM's `Vec`-index frames). The refcount contract mirrors the
/// runtime's internal accessors: [`get`](VarStore::get) returns a **borrowed**
/// pointer (the variable table keeps its reference — the caller must not
/// release it), and [`set`](VarStore::set) has the table take its own `+1` on
/// the value.
impl VarStore for Interp {
    type Value = *mut TclObj;

    fn get(&self, _frame: FrameId, name: &str) -> Option<*mut TclObj> {
        self.var_get(name.as_bytes())
    }

    fn set(&mut self, _frame: FrameId, name: &str, value: *mut TclObj) {
        // The variable table takes a +1; a write-trace error is irrelevant to
        // the storage contract, so the `Result` is intentionally discarded
        // (the VM's impl likewise drops it).
        let _ = self.var_set(name.as_bytes(), value);
    }

    fn unset(&mut self, _frame: FrameId, name: &str) -> bool {
        self.var_unset(name.as_bytes())
    }

    fn exists(&self, _frame: FrameId, name: &str) -> bool {
        self.var_exists(name.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::counters;
    use crate::interp::{new_string, obj_bytes};
    use tcl_runtime_api::GLOBAL_FRAME;

    /// Run `body` against a fresh interpreter and assert it leaks nothing (the
    /// `*mut TclObj` refcount discipline of the `VarStore` impl is correct).
    fn leak_free(body: impl FnOnce(&mut Interp)) {
        counters::reset();
        {
            let mut interp = Interp::new();
            body(&mut interp);
        }
        assert_eq!(
            counters::finalize(),
            0,
            "residual: {} objs, {} bufs",
            counters::live_objs(),
            counters::live_bufs()
        );
        assert_eq!(counters::double_free_count(), 0);
    }

    #[test]
    fn varstore_set_get_unset_exists() {
        leak_free(|i| {
            assert!(!i.exists(GLOBAL_FRAME, "x"));
            // A fresh (rc-0) object handed to `set`; the table takes its +1.
            i.set(GLOBAL_FRAME, "x", new_string(b"hi"));
            assert!(i.exists(GLOBAL_FRAME, "x"));
            // `get` is borrowed — read it without releasing.
            assert_eq!(obj_bytes(i.get(GLOBAL_FRAME, "x").unwrap()), b"hi");
            // Overwrite: the table releases the old value and takes the new.
            i.set(GLOBAL_FRAME, "x", new_string(b"bye"));
            assert_eq!(obj_bytes(i.get(GLOBAL_FRAME, "x").unwrap()), b"bye");
            assert!(i.unset(GLOBAL_FRAME, "x"));
            assert!(!i.exists(GLOBAL_FRAME, "x"));
            assert!(!i.unset(GLOBAL_FRAME, "x")); // already gone
        });
    }
}
