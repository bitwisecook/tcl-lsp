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

//! What a `Tcl_Interp *` points at: the shim's per-interpreter state.
//!
//! This is *not* the engine. The C side sees a result slot, an error code, a
//! command table, and the packages it has provided — the things the C API
//! reads and writes through the interpreter pointer. Commands the C side
//! registers are recorded here and published to the engine by
//! [`crate::Interp::sync`], because a command procedure may itself call
//! `Tcl_CreateObjCommand` while the engine is busy invoking it.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::ffi::{c_int, c_void};
use std::rc::{Rc, Weak};

use tcl_engine_api::{CommandRegistrar, EngineError};

use crate::obj::{Obj, ObjRef, TclError};

/// `Tcl_ObjCmdProc`.
pub type ObjCmdProc =
    unsafe extern "C" fn(*mut c_void, *mut InterpState, c_int, *const *mut Obj) -> c_int;

/// `Tcl_CmdDeleteProc`.
pub type CmdDeleteProc = unsafe extern "C" fn(*mut c_void);

/// A `<Pkg>_Init` entry point.
pub type InitProc = unsafe extern "C" fn(*mut InterpState) -> c_int;

/// One registered C command.
pub struct CommandEntry {
    /// The command name as registered.
    pub name: String,
    /// The procedure to call.
    pub proc: ObjCmdProc,
    /// The opaque pointer handed back to `proc` and `delete_proc`.
    pub client_data: *mut c_void,
    /// The teardown procedure, if one was registered.
    pub delete_proc: Option<CmdDeleteProc>,
}

impl CommandEntry {
    fn run_delete_proc(&self) {
        if let Some(delete_proc) = self.delete_proc {
            // SAFETY: the extension registered this procedure with this
            // client data; the shim calls it exactly once, at deletion.
            unsafe { delete_proc(self.client_data) };
        }
    }
}

/// A change to the command table not yet applied to the engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandChange {
    /// `Tcl_CreateObjCommand` registered this name.
    Created(String),
    /// `Tcl_DeleteCommand` removed this name.
    Deleted(String),
}

/// The state behind a `Tcl_Interp *`.
pub struct InterpState {
    commands: RefCell<BTreeMap<String, Rc<CommandEntry>>>,
    result: RefCell<ObjRef>,
    error_code: RefCell<Option<ObjRef>>,
    packages: RefCell<Vec<(String, String)>>,
    pending: RefCell<Vec<CommandChange>>,
    /// The `Rc` this state lives in, when it lives in one, so a door call can
    /// publish a command C has just created before it evaluates a script.
    shared: Weak<InterpState>,
    /// The engine's door while a command runs, as a pointer to the
    /// `&mut dyn CommandRegistrar` the scope that opened it holds; null outside
    /// a scope. See [`Self::open_door`].
    door: Cell<*mut c_void>,
    /// Objects a door call handed to C, kept until the scope of the command
    /// that made the call ends.
    retained: RefCell<Vec<ObjRef>>,
    /// The error C cannot swallow: a budget the engine enforces or a crash.
    fatal: RefCell<Option<EngineError>>,
}

/// The engine's door as one command's invocation holds it, for
/// [`InterpState::open_door`] to point at.
pub(crate) struct DoorRef<'a>(&'a mut dyn CommandRegistrar);

impl<'a> DoorRef<'a> {
    /// The door `registrar` is.
    pub(crate) fn new(registrar: &'a mut dyn CommandRegistrar) -> Self {
        Self(registrar)
    }
}

/// An open door: the engine's [`CommandRegistrar`] reachable from the C API
/// while one command's procedure runs. Dropping it closes the door, restores the
/// one an enclosing command had open and releases the objects its calls handed
/// to C.
pub(crate) struct DoorScope<'a> {
    state: &'a InterpState,
    previous: *mut c_void,
    retained_mark: usize,
}

impl Drop for DoorScope<'_> {
    fn drop(&mut self) {
        self.state.door.set(self.previous);
        let released = {
            let mut retained = self.state.retained.borrow_mut();
            let mark = self.retained_mark.min(retained.len());
            retained.split_off(mark)
        };
        drop(released);
    }
}

impl Default for InterpState {
    fn default() -> Self {
        Self::new()
    }
}

impl InterpState {
    /// Fresh state: an empty result and no commands.
    #[must_use]
    pub fn new() -> Self {
        Self::with_shared(Weak::new())
    }

    /// Fresh state in an `Rc` that knows itself, which is what lets a door call
    /// publish a command C created a moment ago ([`Self::shared`]).
    #[must_use]
    pub(crate) fn new_shared() -> Rc<Self> {
        Rc::new_cyclic(|me| Self::with_shared(me.clone()))
    }

    fn with_shared(shared: Weak<InterpState>) -> Self {
        Self {
            commands: RefCell::new(BTreeMap::new()),
            result: RefCell::new(ObjRef::new(Obj::from_text(""))),
            error_code: RefCell::new(None),
            packages: RefCell::new(Vec::new()),
            pending: RefCell::new(Vec::new()),
            shared,
            door: Cell::new(std::ptr::null_mut()),
            retained: RefCell::new(Vec::new()),
            fatal: RefCell::new(None),
        }
    }

    /// The `Rc` this state lives in, or `None` for a state that does not live in
    /// one.
    #[must_use]
    pub(crate) fn shared(&self) -> Option<Rc<Self>> {
        self.shared.upgrade()
    }

    /// Open the engine's `door` to the C API until the returned scope drops.
    ///
    /// The state keeps a pointer to the reference the caller holds, not a
    /// reference of its own, so the borrow is the caller's for the scope's whole
    /// life and nothing else may use `door` until the scope has dropped. A scope
    /// opened inside another (a command that evaluates a script that calls a
    /// command) replaces the outer door for its own length and restores it.
    pub(crate) fn open_door<'a>(&'a self, door: &'a mut DoorRef<'_>) -> DoorScope<'a> {
        let previous = self.door.replace(std::ptr::from_mut(door).cast::<c_void>());
        DoorScope {
            state: self,
            previous,
            retained_mark: self.retained.borrow().len(),
        }
    }

    /// Run `body` with the open door, or answer `None` when no door is open.
    ///
    /// The door is taken for the call: a command the body causes to run opens a
    /// door of its own, and nothing reaches the outer one until `body` returns.
    pub(crate) fn with_door<T>(
        &self,
        body: impl FnOnce(&mut dyn CommandRegistrar) -> T,
    ) -> Option<T> {
        struct Restore<'a>(&'a Cell<*mut c_void>, *mut c_void);
        impl Drop for Restore<'_> {
            fn drop(&mut self) {
                self.0.set(self.1);
            }
        }

        let door = self.door.replace(std::ptr::null_mut());
        if door.is_null() {
            return None;
        }
        let _restore = Restore(&self.door, door);
        // SAFETY: a non-null `door` was set by `open_door` from a `DoorRef` its
        // scope still borrows exclusively: the scope outlives this call, and the
        // caller that opened it is inside the C procedure that is running now,
        // so nothing else uses that reference until the call returns.
        let registrar = unsafe { &mut *door.cast::<DoorRef<'_>>() };
        Some(body(&mut *registrar.0))
    }

    /// Keep `object` alive until the scope of the running command ends, and
    /// answer its pointer for C.
    pub(crate) fn retain(&self, object: ObjRef) -> *mut Obj {
        let pointer = object.as_ptr();
        self.retained.borrow_mut().push(object);
        pointer
    }

    /// Record the error C cannot swallow: the first one a call reports stays, and
    /// the command that is running fails with it whatever it returns.
    pub(crate) fn set_fatal(&self, error: EngineError) {
        let mut fatal = self.fatal.borrow_mut();
        if fatal.is_none() {
            *fatal = Some(error);
        }
    }

    /// The fatal error recorded, if any.
    pub(crate) fn fatal(&self) -> Option<EngineError> {
        self.fatal.borrow().clone()
    }

    /// Take the fatal error recorded, if any.
    pub(crate) fn take_fatal(&self) -> Option<EngineError> {
        self.fatal.borrow_mut().take()
    }

    /// Register a command, replacing (and tearing down) any existing one of
    /// that name — `Tcl_CreateObjCommand`.
    pub fn create_command(
        &self,
        name: &str,
        proc: ObjCmdProc,
        client_data: *mut c_void,
        delete_proc: Option<CmdDeleteProc>,
    ) -> Rc<CommandEntry> {
        let entry = Rc::new(CommandEntry {
            name: name.to_owned(),
            proc,
            client_data,
            delete_proc,
        });
        let previous = self
            .commands
            .borrow_mut()
            .insert(name.to_owned(), Rc::clone(&entry));
        if let Some(previous) = previous {
            previous.run_delete_proc();
        }
        self.pending
            .borrow_mut()
            .push(CommandChange::Created(name.to_owned()));
        entry
    }

    /// Remove a command, running its delete procedure — `Tcl_DeleteCommand`.
    /// Reports whether there was one.
    pub fn delete_command(&self, name: &str) -> bool {
        let removed = self.commands.borrow_mut().remove(name);
        match removed {
            Some(entry) => {
                entry.run_delete_proc();
                self.pending
                    .borrow_mut()
                    .push(CommandChange::Deleted(name.to_owned()));
                true
            }
            None => false,
        }
    }

    /// The registered command of that name.
    #[must_use]
    pub fn command(&self, name: &str) -> Option<Rc<CommandEntry>> {
        self.commands.borrow().get(name).cloned()
    }

    /// Every registered command name, sorted.
    #[must_use]
    pub fn command_names(&self) -> Vec<String> {
        self.commands.borrow().keys().cloned().collect()
    }

    /// Drain the changes the engine has not seen.
    pub fn take_pending(&self) -> Vec<CommandChange> {
        std::mem::take(&mut *self.pending.borrow_mut())
    }

    /// The current result object.
    #[must_use]
    pub fn result(&self) -> ObjRef {
        self.result.borrow().clone()
    }

    /// Replace the result — `Tcl_SetObjResult`.
    pub fn set_result(&self, result: ObjRef) {
        *self.result.borrow_mut() = result;
    }

    /// Replace the result with text.
    pub fn set_result_text(&self, text: &str) {
        self.set_result(ObjRef::new(Obj::from_text(text)));
    }

    /// Append text to the result — one piece of `Tcl_AppendResult`. Always
    /// builds a fresh object so a result shared with an argument is never
    /// mutated in place.
    pub fn append_result(&self, piece: &str) {
        let mut text = self.result().get().text();
        text.push_str(piece);
        self.set_result_text(&text);
    }

    /// Clear the result and the error code — `Tcl_ResetResult`.
    pub fn reset_result(&self) {
        self.set_result_text("");
        *self.error_code.borrow_mut() = None;
    }

    /// Set the `-errorcode` — `Tcl_SetObjErrorCode`.
    pub fn set_error_code(&self, code: Option<ObjRef>) {
        *self.error_code.borrow_mut() = code;
    }

    /// The `-errorcode` as text, if one was set.
    #[must_use]
    pub fn error_code_text(&self) -> Option<String> {
        self.error_code
            .borrow()
            .as_ref()
            .map(|code| code.get().text())
    }

    /// Install a conversion error as the result and error code.
    pub fn set_error(&self, error: &TclError) {
        self.set_result_text(&error.message);
        self.set_error_code(
            error
                .code
                .as_deref()
                .map(|code| ObjRef::new(Obj::from_text(code))),
        );
    }

    /// Record a provided package — `Tcl_PkgProvideEx`.
    pub fn provide(&self, name: &str, version: &str) -> Result<(), TclError> {
        let mut packages = self.packages.borrow_mut();
        if let Some((_, existing)) = packages.iter().find(|(provided, _)| provided == name) {
            if existing == version {
                return Ok(());
            }
            return Err(TclError::with_code(
                format!(
                    "conflicting versions provided for package \"{name}\": {existing}, then {version}"
                ),
                "TCL PACKAGE VERSIONCONFLICT",
            ));
        }
        packages.push((name.to_owned(), version.to_owned()));
        Ok(())
    }

    /// The packages provided so far, in provision order.
    #[must_use]
    pub fn provided_packages(&self) -> Vec<(String, String)> {
        self.packages.borrow().clone()
    }
}

impl Drop for InterpState {
    /// Tear the commands down as C Tcl does when an interpreter is deleted.
    fn drop(&mut self) {
        let commands = std::mem::take(&mut *self.commands.borrow_mut());
        for entry in commands.into_values() {
            entry.run_delete_proc();
        }
    }
}
