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

//! The runtime's impls of the `tcl-runtime-api` interp-state role traits.
//!
//! The runtime satisfies the shared state-mutation contract over its
//! `*mut TclObj` value model, so a consumer of the `tcl-runtime-api` role
//! traits can reach into this runtime's state with the *same* contract the
//! bytecode VM (`tcl-vm`) satisfies over `Rc<Obj>`. All six role traits —
//! `VarStore`, `Introspect`, `Commands`, `Traces`, `Frames`, `Namespaces` — are
//! implemented here. `Namespaces::find_command` mints a `CommandId` that
//! `Commands::dispatch_id` invokes (the resolve-then-invoke pairing).

use tcl_runtime_api::{
    CommandId, Commands, Completion, FrameId, Frames, Introspect, Namespaces, NsId, ProcInfo,
    ProcParam, Procs, Traces, VarStore,
};

use crate::frame::Link;
use crate::interp::{new_string, Interp};
use crate::list::new_list_obj;
use crate::obj::{self, TclObj};

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

    // Element access carries the base and key independently through both the
    // current and frame-addressed resolver paths.

    fn get_elem(&self, frame: FrameId, name: &str, key: &str) -> Option<*mut TclObj> {
        if frame.0 == self.frames.borrow().current_level() {
            self.var_get_elem(name.as_bytes(), key.as_bytes())
        } else {
            self.var_get_elem_at(name.as_bytes(), key.as_bytes(), frame.0)
        }
    }

    fn set_elem(&mut self, frame: FrameId, name: &str, key: &str, value: *mut TclObj) {
        if frame.0 == self.frames.borrow().current_level() {
            let _ = self.var_set_elem(name.as_bytes(), key.as_bytes(), value);
        } else {
            let _ = self.var_set_elem_at(name.as_bytes(), key.as_bytes(), value, frame.0);
        }
    }

    fn unset_elem(&mut self, frame: FrameId, name: &str, key: &str) -> bool {
        if frame.0 == self.frames.borrow().current_level() {
            self.var_unset_elem(name.as_bytes(), key.as_bytes())
        } else {
            self.var_unset_elem_at(name.as_bytes(), key.as_bytes(), frame.0)
        }
    }

    fn exists_elem(&self, frame: FrameId, name: &str, key: &str) -> bool {
        self.get_elem(frame, name, key).is_some()
    }

    fn array_keys(&self, _frame: FrameId, name: &str) -> Option<Vec<String>> {
        // `array_names` is `None` for a non-array, `Some(keys)` (possibly empty)
        // for an array — exactly the contract. Resolves against the active frame.
        self.array_names(name.as_bytes()).map(|keys| {
            keys.iter()
                .map(|k| String::from_utf8_lossy(k).into_owned())
                .collect()
        })
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

/// Proc introspection (`info body`/`args`/`default`) over the runtime's retained
/// [`ProcDef`](crate::interp::ProcDef). `proc_def` already follows `namespace
/// import` redirects to the underlying proc, so an imported proc introspects as
/// its source (info-1.7/2.4). The body/params are cloned into owned bytes — the
/// contract is value-agnostic, and the shared `info` core rebuilds the result
/// objects through `ValueOps`.
impl Procs for Interp {
    fn proc_info(&self, name: &str) -> Option<ProcInfo> {
        let def = self.proc_def(name.as_bytes())?;
        Some(ProcInfo {
            body: def.body.clone(),
            params: def
                .params
                .iter()
                .map(|p| ProcParam {
                    name: p.name.clone(),
                    default: p.default.clone(),
                })
                .collect(),
        })
    }
}

/// Bridge the runtime's own [`crate::interp::Code`] to the contract's
/// [`tcl_runtime_api::Code`]. The two enums are structurally identical (the
/// runtime predates the shared `tcl-core-types`); the explicit match keeps them
/// honestly decoupled and breaks loudly if either drifts.
fn api_code(code: crate::interp::Code) -> tcl_runtime_api::Code {
    use crate::interp::Code as Rt;
    use tcl_runtime_api::Code as Api;
    match code {
        Rt::Ok => Api::Ok,
        Rt::Error => Api::Error,
        Rt::Return => Api::Return,
        Rt::Break => Api::Break,
        Rt::Continue => Api::Continue,
        Rt::Other(n) => Api::Other(n),
    }
}

/// Snapshot one runtime completion as the shared [`Completion`] contract.
///
/// Both returned object handles own one reference. The caller must release
/// each exactly once, or transfer it to a runtime slot that takes ownership.
/// The return-options dict is built by the same live error/return-state helper
/// used by `catch` and `try`; it is not a placeholder.
pub(crate) fn capture_completion(
    interp: &mut Interp,
    code: crate::interp::Code,
) -> Completion<*mut TclObj> {
    let result = interp.result_obj();
    let options = crate::cmd_error::completion_options(interp, code);
    // SAFETY: `result` is interp-owned and `options` is fresh. Give the
    // completion one owned reference to each; the caller adopts both.
    unsafe {
        obj::incr_ref_count(result);
        obj::incr_ref_count(options);
    }
    Completion::new(api_code(code), result, options)
}

/// Run a full, prebuilt command argv through the inherent dispatcher, then
/// snapshot the resulting [`Completion`].
///
/// `argv[0]` is the already-evaluated command head. This deliberately performs
/// resolution and dispatch only: it does not parse source or repeat word
/// substitution. Callers retain argv elements for the call duration; this
/// helper borrows them and neither adopts nor releases them.
pub(crate) fn dispatch_prebuilt_argv(
    interp: &mut Interp,
    argv: &[*mut TclObj],
) -> Completion<*mut TclObj> {
    let code = Interp::dispatch(interp, argv);
    capture_completion(interp, code)
}

/// Run `name` + `argv` through the inherent dispatch and snapshot a [`Completion`]
/// — the shared body of [`Commands::dispatch`]/[`Commands::dispatch_id`].
///
/// The runtime is natively a `Code`-plus-`set_result` machine, so this bridges
/// to the `Completion` ABI: it builds the name-included argv the inherent
/// `dispatch` expects (taking an owning `+1` on every word for the call, mirroring
/// `dispatch_list_obj`), runs it, then snapshots `(code, result, options)`.
///
/// **Refcount contract.** Unlike [`VarStore::get`]'s borrowed pointer, the
/// returned completion *owns* a `+1` on both `result` and `options`; the caller
/// adopts them and must release each (`decr_ref_count` / `drop_fresh`) when
/// done — the `*mut TclObj` analogue of the VM completion's owned `Rc`s.
fn dispatch_named(
    interp: &mut Interp,
    name: &[u8],
    argv: &[*mut TclObj],
) -> Completion<*mut TclObj> {
    let name_obj = new_string(name);
    let mut full: Vec<*mut TclObj> = Vec::with_capacity(argv.len() + 1);
    full.push(name_obj);
    full.extend_from_slice(argv);
    // An owning +1 on each word for the call's duration, released after (the
    // `dispatch_list_obj` discipline). The fresh `name_obj` (rc 0) is freed by its
    // release unless dispatch retained it; the caller's `argv` (rc >= 1) are
    // returned to their prior count untouched.
    for &w in &full {
        unsafe { obj::incr_ref_count(w) };
    }
    let code = Interp::dispatch(interp, &full); // the inherent dispatch, not the trait method
    for &w in &full {
        unsafe { obj::decr_ref_count(w) };
    }
    capture_completion(interp, code)
}

/// The error completion for a fabricated/stale `CommandId` (rc-balanced like a
/// real one: `result`/`options` each `+1`).
fn invalid_command_id(interp: &mut Interp) -> Completion<*mut TclObj> {
    let code = interp.set_error(b"invalid command id");
    capture_completion(interp, code)
}

/// Command dispatch by name ([`dispatch`](Commands::dispatch)) or by a resolved
/// [`CommandId`] ([`dispatch_id`](Commands::dispatch_id), which reverses the id
/// to its FQN and invokes that) — the resolve-then-invoke pairing with
/// [`Namespaces::find_command`].
impl Commands for Interp {
    type Value = *mut TclObj;

    fn dispatch(&mut self, name: &str, argv: &[*mut TclObj]) -> Completion<*mut TclObj> {
        dispatch_named(self, name.as_bytes(), argv)
    }

    fn dispatch_id(&mut self, cmd: CommandId, argv: &[*mut TclObj]) -> Completion<*mut TclObj> {
        match self.command_fqn(cmd.0) {
            Some(fqn) => dispatch_named(self, &fqn, argv),
            None => invalid_command_id(self),
        }
    }
}

/// Variable traces: fire `var`'s `op` (`read`/`write`/`unset`) traces, aborting
/// the access if a callback errors.
///
/// Delegates to [`Interp::fire_var_traces_for`], which keeps the trace internals
/// (firing guard, result preservation, per-op `can't read/set "var"` wrapping)
/// in `interp.rs`. Only `read`/`write` callback errors abort (matching C and the
/// VM); `unset`/`array` errors are swallowed, so those fire as `Ok`. The error
/// value is **freshly built** (rc-0) — the caller adopts it (store via
/// `set_result`, or `drop_fresh`), as for [`Introspect::level_argv`].
impl Traces for Interp {
    type Value = *mut TclObj;

    fn fire(&mut self, var: &str, op: &str) -> Result<(), *mut TclObj> {
        match self.fire_var_traces_for(var.as_bytes(), op.as_bytes()) {
            Some(msg) => Err(new_string(&msg)),
            None => Ok(()),
        }
    }
}

/// The call-frame stack. `NsId` is native here (each frame carries its
/// namespace), so [`push`](Frames::push) maps directly to a proc-call frame.
/// [`link`](Frames::link) installs an `upvar`/`global`-style alias in the
/// current frame — the only frame `upvar` ever targets — to `target`'s variable;
/// the target home follows the same `level → frame-or-namespace` rule as
/// variable resolution ([`crate::vars::home_at`]), so a link to `GLOBAL_FRAME`
/// correctly lands in the global namespace table.
impl Frames for Interp {
    fn push(&mut self, ns: NsId) -> FrameId {
        // The contract's `NsId` is a `u32` newtype; the runtime's is a `usize`
        // arena index (`ROOT_NS`/`GLOBAL` both 0).
        FrameId(self.frames.borrow_mut().push(ns.0 as usize))
    }

    fn pop(&mut self) {
        self.frames.borrow_mut().pop();
    }

    fn current(&self) -> FrameId {
        FrameId(self.frames.borrow().current_level())
    }

    fn link(&mut self, here: FrameId, target: FrameId, local: &str, target_name: &str) {
        debug_assert_eq!(
            here.0,
            self.frames.borrow().current_level(),
            "upvar installs in the current frame"
        );
        let home = crate::vars::home_at(&self.frames.borrow(), target.0);
        let target = Link {
            home,
            name: target_name.as_bytes().to_vec(),
            elem: None,
        };
        self.make_upvar(target, local.as_bytes());
    }

    fn in_proc(&self) -> bool {
        self.frames.borrow().in_proc()
    }

    fn var_names(&self, include_links: bool) -> Vec<String> {
        let frames = self.frames.borrow();
        let names = if include_links {
            frames.local_names()
        } else {
            frames.local_names_no_links()
        };
        names
            .iter()
            .map(|s| String::from_utf8_lossy(s).into_owned())
            .collect()
    }
}

/// Namespace name resolution. `NsId` is native (the contract's `u32` newtype
/// bridges the runtime's `usize` arena id), so [`current`](Namespaces::current)
/// is a direct read. [`find_command`](Namespaces::find_command) resolves `name`
/// from `cxt` through the namespace tree to its FQN (`resolve_fqn`) and interns
/// that to a stable `CommandId`. Note: the handle is currently produced for
/// command *identity* only — nothing dispatches by it (the `Commands` trait
/// dispatches by name), the open `find_command`/`CommandId` consumer question.
impl Namespaces for Interp {
    fn find_command(&self, cxt: NsId, name: &str) -> Option<CommandId> {
        // The contract's `NsId` is a `u32` newtype; the runtime's is a `usize`.
        self.find_command_id(cxt.0 as usize, name.as_bytes())
            .map(CommandId)
    }

    fn current(&self) -> NsId {
        NsId(self.current_ns() as u32)
    }

    fn name(&self, ns: NsId) -> String {
        String::from_utf8_lossy(&self.ns_qualified_name(ns.0 as usize)).into_owned()
    }

    fn command_name(&self, cmd: CommandId) -> Option<String> {
        self.command_fqn(cmd.0)
            .map(|b| String::from_utf8_lossy(&b).into_owned())
    }

    // Namespace-tree navigation. The contract's `NsId` is a `u32` newtype; the
    // runtime's arena id is a `usize` (both `ROOT_NS`/`GLOBAL` = 0).
    fn find_namespace(&self, cxt: NsId, name: &str) -> Option<NsId> {
        self.namespaces()
            .find_namespace(cxt.0 as usize, name.as_bytes())
            .map(|id| NsId(id as u32))
    }

    fn namespace_is_live(&self, ns: NsId) -> bool {
        self.namespaces().namespace_is_live(ns.0 as usize)
    }

    fn parent(&self, ns: NsId) -> Option<NsId> {
        self.namespaces()
            .parent(ns.0 as usize)
            .map(|id| NsId(id as u32))
    }

    fn children(&self, ns: NsId) -> Vec<NsId> {
        self.namespaces()
            .children(ns.0 as usize)
            .into_iter()
            .map(|id| NsId(id as u32))
            .collect()
    }

    fn children_hash_order(&self, ns: NsId) -> Vec<NsId> {
        self.namespaces()
            .children_hash_order(ns.0 as usize)
            .into_iter()
            .map(|id| NsId(id as u32))
            .collect()
    }

    // Command enumeration consumes the runtime-selected command surface. The
    // runtime owns the registry query that hides an unavailable builtin while
    // retaining user-defined entries in the same namespace.
    fn commands_in(&self, ns: NsId) -> Vec<String> {
        self.visible_command_names_in(ns.0 as usize)
            .iter()
            .map(|s| String::from_utf8_lossy(s).into_owned())
            .collect()
    }

    fn procs_in(&self, ns: NsId) -> Vec<String> {
        self.namespaces()
            .proc_names(ns.0 as usize)
            .iter()
            .map(|s| String::from_utf8_lossy(s).into_owned())
            .collect()
    }

    fn vars_in(&self, ns: NsId) -> Vec<String> {
        self.namespaces()
            .var_names(ns.0 as usize)
            .iter()
            .map(|s| String::from_utf8_lossy(s).into_owned())
            .collect()
    }

    // `Tcl_FindNamespaceVar`'s single probe: the namespace's own `varTable`,
    // including a `variable`-declared but as-yet-unset cell, and never a call
    // frame.
    fn namespace_var_exists(&self, ns: NsId, simple: &str) -> bool {
        self.namespaces()
            .var_table(ns.0 as usize)
            .cell(simple.as_bytes())
            .is_some()
    }

    // -- byte-valued spellings. This runtime keys every table by bytes, so
    // these are the lossless forms; the `&str` methods above are the lossy
    // convenience the UTF-8-keyed VM uses. Shared cores call these, so a name
    // like `[binary format c 255]` reaches the table verbatim.

    fn find_command_bytes(&self, cxt: NsId, name: &[u8]) -> Option<CommandId> {
        self.find_command_id(cxt.0 as usize, name).map(CommandId)
    }

    fn find_namespace_bytes(&self, cxt: NsId, name: &[u8]) -> Option<NsId> {
        self.namespaces()
            .find_namespace(cxt.0 as usize, name)
            .map(|id| NsId(id as u32))
    }

    fn namespace_var_exists_bytes(&self, ns: NsId, simple: &[u8]) -> bool {
        self.namespaces()
            .var_table(ns.0 as usize)
            .cell(simple)
            .is_some()
    }

    fn name_bytes(&self, ns: NsId) -> Vec<u8> {
        self.ns_qualified_name(ns.0 as usize)
    }

    fn command_name_bytes(&self, cmd: CommandId) -> Option<Vec<u8>> {
        self.command_fqn(cmd.0)
    }

    fn command_origin(&self, cmd: CommandId) -> Option<CommandId> {
        self.imported_source_id(cmd.0).map(CommandId)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::counters;
    use crate::interp::{drop_fresh, new_string, obj_bytes};
    use tcl_runtime_api::{Code, GLOBAL_FRAME, ROOT_NS};

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
    fn varstore_array_elements() {
        leak_free(|i| {
            assert!(!i.exists_elem(GLOBAL_FRAME, "a", "k"));
            i.set_elem(GLOBAL_FRAME, "a", "k", new_string(b"v"));
            assert!(i.exists_elem(GLOBAL_FRAME, "a", "k"));
            assert_eq!(obj_bytes(i.get_elem(GLOBAL_FRAME, "a", "k").unwrap()), b"v");
            assert!(!i.exists_elem(GLOBAL_FRAME, "a", "nope"));
            assert!(i.unset_elem(GLOBAL_FRAME, "a", "k"));
            assert!(!i.exists_elem(GLOBAL_FRAME, "a", "k"));
            i.unset(GLOBAL_FRAME, "a"); // drop the now-empty array
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
            // Frame-addressed pair access must not rebuild `z(b(key)` and
            // reparse it as a different array reference.
            i.set_elem(GLOBAL_FRAME, "z(b", "key", new_string(b"pair"));
            assert_eq!(
                obj_bytes(i.get_elem(GLOBAL_FRAME, "z(b", "key").unwrap()),
                b"pair"
            );
            assert!(i.exists_elem(GLOBAL_FRAME, "z(b", "key"));
            assert!(i.unset_elem(GLOBAL_FRAME, "z(b", "key"));
            // Reach back into the global frame from the proc frame.
            i.set(GLOBAL_FRAME, "g2", new_string(b"two"));
            // Pop the proc frame (frees `loc`); the reached-back write is visible.
            i.frames.borrow_mut().pop();
            assert_eq!(obj_bytes(i.get(GLOBAL_FRAME, "g2").unwrap()), b"two");
            assert!(i.unset(GLOBAL_FRAME, "g2"));
            assert!(i.unset(GLOBAL_FRAME, "g"));
            assert!(i.unset(GLOBAL_FRAME, "z(b"));
            assert!(!i.exists(GLOBAL_FRAME, "g"));
        });
    }

    #[test]
    fn commands_dispatch_builtin_and_unknown() {
        leak_free(|i| {
            // A builtin runs and yields its result. The caller owns its argv
            // (rc >= 1, the dispatch contract); the completion owns +1 on the
            // result + options, which the caller releases. UFCS reaches the trait
            // method — the inherent `Interp::dispatch` shadows it for method
            // syntax (a generic `T: Commands` consumer is unaffected).
            let lst = new_string(b"a b c");
            unsafe { obj::incr_ref_count(lst) };
            let c = Commands::dispatch(i, "llength", &[lst]);
            assert_eq!(c.code, Code::Ok);
            assert_eq!(obj_bytes(c.result), b"3");
            unsafe {
                obj::decr_ref_count(c.result);
                obj::decr_ref_count(c.options);
                obj::decr_ref_count(lst);
            }

            // An unknown command name is an error completion.
            let c = Commands::dispatch(i, "no_such_command", &[]);
            assert_eq!(c.code, Code::Error);
            assert!(obj_bytes(c.result).starts_with(b"invalid command name"));
            unsafe {
                obj::decr_ref_count(c.result);
                obj::decr_ref_count(c.options);
            }
        });
    }

    #[test]
    fn traces_fire_ok_and_error() {
        leak_free(|i| {
            // A read trace whose callback succeeds → the access proceeds (Ok).
            // (`;#` comments out the appended `name elem op` words.)
            let _ = i.eval_str(b"trace add variable x read {list ok;#}");
            assert!(Traces::fire(i, "x", "read").is_ok());

            // A read trace whose callback errors → the access aborts, with the
            // error wrapped to the user-facing `can't read "var"` form. The error
            // value is fresh (rc-0); adopt and release it.
            let _ = i.eval_str(b"trace add variable y read {error boom;#}");
            let e = Traces::fire(i, "y", "read").unwrap_err();
            assert_eq!(obj_bytes(e), b"can't read \"y\": boom");
            drop_fresh(e);

            // An `unset` trace error does *not* abort (matches C / the VM): Ok.
            let _ = i.eval_str(b"trace add variable z unset {error nope;#}");
            assert!(Traces::fire(i, "z", "unset").is_ok());
        });
    }

    #[test]
    fn frames_push_pop_current_link() {
        leak_free(|i| {
            // A global, set while the global frame is current.
            i.set(GLOBAL_FRAME, "g", new_string(b"orig"));
            let outer = Frames::current(i);
            assert_eq!(outer, GLOBAL_FRAME);
            // Push a proc-call frame in the global namespace; it becomes current.
            let inner = Frames::push(i, ROOT_NS);
            assert_ne!(inner, outer);
            assert_eq!(Frames::current(i), inner);
            // `upvar`: link `gg` (inner) to the outer frame's global `g`. The
            // link target resolves to the global namespace table (level 0), so
            // reads and writes through `gg` reach `g`.
            Frames::link(i, inner, outer, "gg", "g");
            assert_eq!(obj_bytes(i.get(inner, "gg").unwrap()), b"orig");
            i.set(inner, "gg", new_string(b"changed"));
            // Pop back to the global frame: the link is gone, the global updated.
            Frames::pop(i);
            assert_eq!(Frames::current(i), outer);
            assert_eq!(obj_bytes(i.get(GLOBAL_FRAME, "g").unwrap()), b"changed");
            assert!(i.unset(GLOBAL_FRAME, "g"));
        });
    }

    #[test]
    fn namespaces_current_and_find_command() {
        leak_free(|i| {
            // At the top level the current namespace is the global root.
            assert_eq!(Namespaces::current(i), ROOT_NS);
            // A builtin resolves from the global namespace to a stable CommandId.
            let a = Namespaces::find_command(i, ROOT_NS, "list").expect("list resolves");
            let b = Namespaces::find_command(i, ROOT_NS, "list").expect("list resolves");
            assert_eq!(a, b);
            // A different command resolves to a distinct id.
            let c = Namespaces::find_command(i, ROOT_NS, "llength").expect("llength resolves");
            assert_ne!(a, c);
            // An unknown command resolves to nothing.
            assert!(Namespaces::find_command(i, ROOT_NS, "no_such_command").is_none());
        });
    }

    #[test]
    fn commands_dispatch_id_composes() {
        leak_free(|i| {
            // Resolve a command to a handle, then invoke it *by that handle* —
            // the find_command -> dispatch_id composition.
            let id = Namespaces::find_command(i, ROOT_NS, "list").expect("list resolves");
            let arg = new_string(b"x");
            unsafe { obj::incr_ref_count(arg) };
            let c = Commands::dispatch_id(i, id, &[arg]);
            assert_eq!(c.code, Code::Ok);
            assert_eq!(obj_bytes(c.result), b"x");
            unsafe {
                obj::decr_ref_count(c.result);
                obj::decr_ref_count(c.options);
                obj::decr_ref_count(arg);
            }
            // A fabricated id yields an error completion (no such command).
            let c = Commands::dispatch_id(i, CommandId(9999), &[]);
            assert_eq!(c.code, Code::Error);
            assert_eq!(obj_bytes(c.result), b"invalid command id");
            unsafe {
                obj::decr_ref_count(c.result);
                obj::decr_ref_count(c.options);
            }
        });
    }
}
