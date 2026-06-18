//! Family-B interp-state trait impls (`tcl-runtime-api`).
//!
//! The runtime satisfies the shared state-mutation contract over its
//! `*mut TclObj` value model, so a consumer of the `tcl-runtime-api` role
//! traits can reach into this runtime's state with the *same* contract the
//! bytecode VM (`tcl-vm`) satisfies over `Rc<Obj>`. `VarStore` and `Introspect`
//! are implemented; the remaining role traits (`Frames`/`Namespaces`/`Traces`)
//! follow as their handle model is reconciled with this runtime's arena/level
//! addressing.

use tcl_runtime_api::{FrameId, Introspect, VarStore};

use crate::interp::{new_string, Interp};
use crate::list::new_list_obj;
use crate::obj::TclObj;

/// The Family-B variable store, honouring `FrameId` (the absolute frame level,
/// `GLOBAL_FRAME` = 0). Like the bytecode VM's impl, a `FrameId` naming the
/// *active* frame delegates to the by-name accessors verbatim (their namespace
/// resolution + trace firing), and any other frame uses the frame-addressed
/// resolver (`vars::*_at`, resolving as if that frame were active, following
/// links). The refcount contract mirrors the runtime's internal accessors:
/// [`get`](VarStore::get) returns a **borrowed** pointer (the variable table
/// keeps its reference — the caller must not release it), and
/// [`set`](VarStore::set) has the table take its own `+1` on the value.
impl VarStore for Interp {
    type Value = *mut TclObj;

    fn get(&self, frame: FrameId, name: &str) -> Option<*mut TclObj> {
        if frame.0 == self.frames.borrow().current_level() {
            self.var_get(name.as_bytes())
        } else {
            self.var_get_at(name.as_bytes(), frame.0)
        }
    }

    fn set(&mut self, frame: FrameId, name: &str, value: *mut TclObj) {
        // The variable table takes a +1; a write-trace error is irrelevant to
        // the storage contract, so the `Result` is intentionally discarded
        // (the VM's impl likewise drops it).
        if frame.0 == self.frames.borrow().current_level() {
            let _ = self.var_set(name.as_bytes(), value);
        } else {
            let _ = self.var_set_at(name.as_bytes(), value, frame.0);
        }
    }

    fn unset(&mut self, frame: FrameId, name: &str) -> bool {
        if frame.0 == self.frames.borrow().current_level() {
            self.var_unset(name.as_bytes())
        } else {
            self.var_unset_at(name.as_bytes(), frame.0)
        }
    }

    fn exists(&self, frame: FrameId, name: &str) -> bool {
        if frame.0 == self.frames.borrow().current_level() {
            self.var_exists(name.as_bytes())
        } else {
            self.var_exists_at(name.as_bytes(), frame.0)
        }
    }
}

/// Runtime introspection backing the `info` family (`info level`/`info level N`).
///
/// The handle-free role trait that fits *both* runtime models as-drafted (the
/// reconciliation finding), so it is the first beyond `VarStore` both runtimes
/// share. `level` is the current proc-nesting depth; `level_argv` builds a
/// **fresh** list of the retained invoking words at an absolute level (`None`
/// for a level with no call — the global frame). Unlike [`VarStore::get`]'s
/// borrowed pointer, the returned `*mut TclObj` is freshly constructed (rc-0)
/// and the caller adopts it (store via `set_result`, or `drop_fresh`), exactly
/// as `info level N` does inline.
impl Introspect for Interp {
    type Value = *mut TclObj;

    fn level(&self) -> usize {
        self.frames.borrow().current_level()
    }

    fn level_argv(&self, level: usize) -> Option<*mut TclObj> {
        let words = self.level_words(level)?;
        let objs: Vec<*mut TclObj> = words.iter().map(|w| new_string(w)).collect();
        Some(new_list_obj(&objs))
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

    #[test]
    fn introspect_level_and_argv() {
        leak_free(|i| {
            // Top level: depth 0, the global frame has no invoking call.
            assert_eq!(Introspect::level(i), 0);
            assert!(Introspect::level_argv(i, 0).is_none());
            // Push a proc-call frame and record its invoking words.
            i.frames.borrow_mut().push(crate::namespace::GLOBAL);
            i.frames
                .borrow_mut()
                .set_words(vec![b"p".to_vec(), b"x".to_vec()]);
            assert_eq!(Introspect::level(i), 1);
            // `level_argv` builds a fresh (rc-0) list; the result slot adopts it
            // via `set_result`, so the leak gate stays balanced.
            let argv = Introspect::level_argv(i, 1).expect("level 1 argv");
            i.set_result(argv);
            assert_eq!(i.result_bytes(), b"p x");
            i.frames.borrow_mut().pop();
            assert_eq!(Introspect::level(i), 0);
        });
    }

    #[test]
    fn varstore_honours_frame_id() {
        leak_free(|i| {
            // A global, written while the global frame is active.
            i.set(GLOBAL_FRAME, "g", new_string(b"global"));
            // Enter a proc-call frame (its own local table).
            let lvl = i.frames.borrow_mut().push(crate::namespace::GLOBAL);
            let here = FrameId(lvl);
            assert_ne!(here, GLOBAL_FRAME);
            i.set(here, "loc", new_string(b"local"));
            // FrameId is honoured: the global is reachable via GLOBAL_FRAME but
            // is not a proc local; the local lives in the proc frame only.
            assert_eq!(obj_bytes(i.get(GLOBAL_FRAME, "g").unwrap()), b"global");
            assert!(i.get(here, "g").is_none());
            assert!(i.exists(here, "loc"));
            assert!(!i.exists(GLOBAL_FRAME, "loc"));
            // Reach back into the global frame from the proc frame.
            i.set(GLOBAL_FRAME, "g2", new_string(b"two"));
            // Pop the proc frame (frees `loc`); the reached-back write is visible.
            i.frames.borrow_mut().pop();
            assert_eq!(obj_bytes(i.get(GLOBAL_FRAME, "g2").unwrap()), b"two");
            assert!(i.unset(GLOBAL_FRAME, "g2"));
            assert!(i.unset(GLOBAL_FRAME, "g"));
            assert!(!i.exists(GLOBAL_FRAME, "g"));
        });
    }
}
