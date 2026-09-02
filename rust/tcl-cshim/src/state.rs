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

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ffi::{c_int, c_void};
use std::rc::Rc;

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
        Self {
            commands: RefCell::new(BTreeMap::new()),
            result: RefCell::new(ObjRef::new(Obj::from_text(""))),
            error_code: RefCell::new(None),
            packages: RefCell::new(Vec::new()),
            pending: RefCell::new(Vec::new()),
        }
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
