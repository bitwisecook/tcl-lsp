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
use std::rc::Rc;

use tcl_core_types::NameBytes;
use tcl_engine_api::{
    CommandPublicationKey, CommandPublicationPurpose, CommandPublicationService, EngineError,
    PreparedCommandPublication,
};

use crate::obj::{Obj, ObjRef, TclError};

/// `Tcl_ObjCmdProc`.
pub type ObjCmdProc =
    unsafe extern "C" fn(*mut c_void, *mut InterpState, c_int, *const *mut Obj) -> c_int;

/// `Tcl_CmdDeleteProc`.
pub type CmdDeleteProc = unsafe extern "C" fn(*mut c_void);

/// A `<Pkg>_Init` entry point.
pub type InitProc = unsafe extern "C" fn(*mut InterpState) -> c_int;

/// Private receipt for code and persistent tables promised by `load_static`.
pub(crate) struct StaticExtensionLifetime {
    _initializer: InitProc,
}

impl StaticExtensionLifetime {
    pub(crate) fn new(initializer: InitProc) -> Self {
        Self {
            _initializer: initializer,
        }
    }
}

/// One registered C command.
pub struct CommandEntry {
    pub(crate) extension: Option<Rc<StaticExtensionLifetime>>,
    /// The command name as registered.
    pub name: NameBytes,
    /// Engine-issued native slot selected before delete callbacks.
    pub publication: PreparedCommandPublication,
    /// The procedure to call.
    pub proc: ObjCmdProc,
    /// The opaque pointer handed back to `proc` and `delete_proc`.
    pub client_data: *mut c_void,
    /// The teardown procedure, if one was registered.
    pub delete_proc: Option<CmdDeleteProc>,
    deleting: Cell<bool>,
}

impl CommandEntry {
    fn run_delete_proc(&self) {
        if self.deleting.replace(true) {
            return;
        }
        if let Some(delete_proc) = self.delete_proc {
            // SAFETY: the extension registered this procedure with this
            // client data; the shim calls it exactly once, at deletion.
            unsafe { delete_proc(self.client_data) };
        }
    }
}

/// A change to the command table not yet applied to the engine.
#[derive(Debug, Clone)]
pub enum CommandChange {
    /// `Tcl_CreateObjCommand` registered this name.
    Created(PreparedCommandPublication),
    /// `Tcl_DeleteCommand` removed this name.
    Deleted(PreparedCommandPublication),
}

/// The state behind a `Tcl_Interp *`.
pub struct InterpState {
    commands: RefCell<BTreeMap<CommandPublicationKey, Rc<CommandEntry>>>,
    result: RefCell<ObjRef>,
    error_code: RefCell<Option<ObjRef>>,
    error_info: RefCell<Option<ObjRef>>,
    packages: RefCell<Vec<(NameBytes, Vec<u8>)>>,
    pending: RefCell<Vec<CommandChange>>,
    publication_service: RefCell<Option<Rc<dyn CommandPublicationService>>>,
    host_refusal: RefCell<Option<EngineError>>,
    original_objects: RefCell<BTreeMap<(u64, u64, usize), crate::obj::WeakObjRef>>,
    importing_originals: RefCell<std::collections::BTreeSet<(u64, u64, usize)>>,
    extension: RefCell<Option<Rc<StaticExtensionLifetime>>>,
}

struct ImportingOriginal<'a> {
    state: &'a InterpState,
    key: (u64, u64, usize),
}
impl Drop for ImportingOriginal<'_> {
    fn drop(&mut self) {
        self.state
            .importing_originals
            .borrow_mut()
            .remove(&self.key);
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
        Self {
            commands: RefCell::new(BTreeMap::new()),
            result: RefCell::new(ObjRef::new(Obj::from_text(""))),
            error_code: RefCell::new(None),
            error_info: RefCell::new(None),
            packages: RefCell::new(Vec::new()),
            pending: RefCell::new(Vec::new()),
            publication_service: RefCell::new(None),
            host_refusal: RefCell::new(None),
            original_objects: RefCell::new(BTreeMap::new()),
            importing_originals: RefCell::new(std::collections::BTreeSet::new()),
            extension: RefCell::new(None),
        }
    }

    pub(crate) fn static_extension(&self) -> Option<Rc<StaticExtensionLifetime>> {
        self.extension.borrow().clone()
    }

    pub(crate) fn with_static_extension<R>(
        &self,
        extension: Option<Rc<StaticExtensionLifetime>>,
        run: impl FnOnce() -> R,
    ) -> R {
        struct Restore<'a>(&'a InterpState, Option<Rc<StaticExtensionLifetime>>);
        impl Drop for Restore<'_> {
            fn drop(&mut self) {
                *self.0.extension.borrow_mut() = self.1.take();
            }
        }
        let previous = self.extension.replace(extension);
        let _restore = Restore(self, previous);
        run()
    }

    pub(crate) fn prune_original_objects(&self) {
        self.original_objects
            .borrow_mut()
            .retain(|_, object| object.is_alive());
    }

    pub(crate) fn import_original(
        self: &Rc<Self>,
        original: Rc<dyn tcl_engine_api::OriginalObject>,
    ) -> Result<ObjRef, EngineError> {
        let scope = original.scope_identity();
        let key = (scope.0, scope.1, original.identity());
        if !self.importing_originals.borrow_mut().insert(key) {
            return Err(EngineError::ExecutionRefusal(
                "cyclic original-object callback input".into(),
            ));
        }
        let _importing = ImportingOriginal { state: self, key };
        let existing = self
            .original_objects
            .borrow()
            .get(&key)
            .and_then(crate::obj::WeakObjRef::upgrade);
        if let Some(existing) = existing {
            existing.get().refresh_original(self)?;
            return Ok(existing);
        }
        let object = ObjRef::new(Obj::from_original(original, self)?);
        self.original_objects
            .borrow_mut()
            .insert(key, object.downgrade());
        Ok(object)
    }

    /// Bind an independently owned native address service for a callback scope.
    pub fn replace_publication_service(
        &self,
        service: Option<Rc<dyn CommandPublicationService>>,
    ) -> Option<Rc<dyn CommandPublicationService>> {
        std::mem::replace(&mut *self.publication_service.borrow_mut(), service)
    }

    fn prepare(
        &self,
        name: &[u8],
        purpose: CommandPublicationPurpose,
    ) -> Result<
        (
            Rc<dyn CommandPublicationService>,
            PreparedCommandPublication,
        ),
        EngineError,
    > {
        let service = self.publication_service.borrow().clone().ok_or_else(|| {
            EngineError::ExecutionRefusal("C command publication has no live native scope".into())
        })?;
        let publication = service.prepare(name, purpose)?;
        Ok((service, publication))
    }

    /// Retain an unavailable native operation outside the guest completion channel.
    pub fn refuse_host(&self, error: EngineError) {
        *self.host_refusal.borrow_mut() = Some(error);
    }

    /// Consume the host failure at the native invocation boundary.
    pub fn take_host_refusal(&self) -> Option<EngineError> {
        self.host_refusal.borrow_mut().take()
    }

    /// Register a command, replacing (and tearing down) any existing one of
    /// that name — `Tcl_CreateObjCommand`.
    pub fn create_command(
        &self,
        name: impl AsRef<[u8]>,
        proc: ObjCmdProc,
        client_data: *mut c_void,
        delete_proc: Option<CmdDeleteProc>,
    ) -> Result<Rc<CommandEntry>, EngineError> {
        let (service, publication) =
            self.prepare(name.as_ref(), CommandPublicationPurpose::CreateCCommand)?;
        let name = NameBytes::from(name.as_ref());
        let previous = self.commands.borrow().get(&publication.key).cloned();
        if let Some(previous) = previous {
            self.with_static_extension(previous.extension.clone(), || previous.run_delete_proc());
        }
        let entry = Rc::new(CommandEntry {
            extension: self.static_extension(),
            name: name.clone(),
            publication: publication.clone(),
            proc,
            client_data,
            delete_proc,
            deleting: Cell::new(false),
        });
        self.commands
            .borrow_mut()
            .insert(publication.key.clone(), Rc::clone(&entry));
        service.note_publication(&publication, true)?;
        self.pending
            .borrow_mut()
            .push(CommandChange::Created(publication));
        Ok(entry)
    }

    /// Remove a command, running its delete procedure — `Tcl_DeleteCommand`.
    /// Reports whether there was one.
    pub fn delete_command(&self, name: impl AsRef<[u8]>) -> Result<bool, EngineError> {
        let (service, publication) =
            self.prepare(name.as_ref(), CommandPublicationPurpose::DeleteCCommand)?;
        let key = &publication.key;
        let previous = self.commands.borrow().get(key).cloned();
        let Some(previous) = previous else {
            if !service.observed_presence(&publication)? {
                return Ok(false);
            }
            service.note_publication(&publication, false)?;
            self.pending
                .borrow_mut()
                .push(CommandChange::Deleted(publication));
            return Ok(true);
        };
        self.with_static_extension(previous.extension.clone(), || previous.run_delete_proc());
        let still_original = self
            .commands
            .borrow()
            .get(key)
            .is_some_and(|entry| Rc::ptr_eq(entry, &previous));
        if still_original {
            self.commands.borrow_mut().remove(key);
            service.note_publication(&publication, false)?;
            self.pending
                .borrow_mut()
                .push(CommandChange::Deleted(publication));
        }
        Ok(true)
    }

    /// Retire the exact shim binding selected by its engine generation.
    /// Native callbacks run before the local entry is removed. A callback's
    /// replacement entry survives; the enclosing engine owns the old unlink.
    pub(crate) fn retire_entry(&self, key: &CommandPublicationKey, entry: &Rc<CommandEntry>) {
        let original = self
            .commands
            .borrow()
            .get(key)
            .is_some_and(|live| Rc::ptr_eq(live, entry));
        if !original {
            return;
        }
        self.with_static_extension(entry.extension.clone(), || entry.run_delete_proc());
        let original = self
            .commands
            .borrow()
            .get(key)
            .is_some_and(|live| Rc::ptr_eq(live, entry));
        if original {
            self.commands.borrow_mut().remove(key);
        }
    }

    /// The registered command of that name.
    #[must_use]
    pub fn command(&self, key: &CommandPublicationKey) -> Option<Rc<CommandEntry>> {
        self.commands.borrow().get(key).cloned()
    }

    /// Every registered command name, sorted.
    #[must_use]
    pub fn command_names(&self) -> Vec<NameBytes> {
        self.commands
            .borrow()
            .values()
            .map(|entry| entry.name.clone())
            .collect()
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
        self.set_result_bytes(text.as_bytes());
    }

    /// Install an exact length-delimited native result.
    pub fn set_result_bytes(&self, bytes: &[u8]) {
        self.set_result(ObjRef::new(Obj::from_bytes(bytes)));
    }

    /// Append text to the result — one piece of `Tcl_AppendResult`. Always
    /// builds a fresh object so a result shared with an argument is never
    /// mutated in place.
    pub fn append_result(&self, piece: &[u8]) {
        let mut bytes = self.result().get().bytes();
        bytes.extend_from_slice(piece);
        self.set_result_bytes(&bytes);
    }

    /// Clear the result and the error code — `Tcl_ResetResult`.
    pub fn reset_result(&self) {
        self.set_result_text("");
        *self.error_code.borrow_mut() = None;
        *self.error_info.borrow_mut() = None;
    }

    /// C9 return options for the supported native interpreter state.
    /// Error initialization retains the same result object as errorInfo.
    pub(crate) fn return_options(&self, code: c_int) -> Obj {
        fn key(bytes: &[u8]) -> ObjRef {
            ObjRef::new(Obj::from_bytes(bytes))
        }
        let mut entries = vec![
            (
                key(b"-code"),
                ObjRef::new(Obj::int(if code == crate::ffi::TCL_RETURN {
                    0
                } else {
                    i64::from(code)
                })),
            ),
            (
                key(b"-level"),
                ObjRef::new(Obj::int(i64::from(code == crate::ffi::TCL_RETURN))),
            ),
        ];
        if code == crate::ffi::TCL_ERROR {
            if self.error_code.borrow().is_none() {
                self.set_error_code(Some(key(b"NONE")));
            }
            if self.error_info.borrow().is_none() {
                *self.error_info.borrow_mut() = Some(self.result());
            }
            entries.push((key(b"-errorstack"), ObjRef::new(Obj::list(Vec::new()))));
        }
        if let Some(error_code) = &*self.error_code.borrow() {
            entries.push((key(b"-errorcode"), error_code.clone()));
        }
        if let Some(error_info) = &*self.error_info.borrow() {
            entries.push((key(b"-errorinfo"), error_info.clone()));
            entries.push((key(b"-errorline"), ObjRef::new(Obj::int(1))));
        }
        Obj::dictionary_cache(entries)
    }

    /// Set the `-errorcode` — `Tcl_SetObjErrorCode`.
    pub fn set_error_code(&self, code: Option<ObjRef>) {
        *self.error_code.borrow_mut() = code;
    }

    /// The `-errorcode` as text, if one was set.
    #[must_use]
    pub fn error_code_bytes(&self) -> Option<Vec<u8>> {
        self.error_code
            .borrow()
            .as_ref()
            .map(|code| code.get().bytes())
    }

    /// Install a conversion error as the result and error code.
    pub fn set_error(&self, error: &TclError) {
        self.set_result_bytes(&error.message);
        if let Some(getter) = &error.getter {
            match getter.error_code_update() {
                tcl_syntax::scalar_getter::NativeScalarGetterErrorCode::Unchanged => {}
                tcl_syntax::scalar_getter::NativeScalarGetterErrorCode::Set(bytes) => {
                    self.set_error_code(Some(ObjRef::new(Obj::from_bytes(bytes))));
                }
            }
        } else {
            self.set_error_code(
                error
                    .code
                    .as_deref()
                    .map(|code| ObjRef::new(Obj::from_bytes(code))),
            );
        }
    }

    /// Record a provided package — `Tcl_PkgProvideEx`.
    pub fn provide(&self, name: &[u8], version: &[u8]) -> Result<(), TclError> {
        let mut packages = self.packages.borrow_mut();
        if let Some((_, existing)) = packages
            .iter()
            .find(|(provided, _)| provided.as_bytes() == name)
        {
            if existing == version {
                return Ok(());
            }
            let mut message = b"conflicting versions provided for package \"".to_vec();
            message.extend_from_slice(name);
            message.extend_from_slice(b"\": ");
            message.extend_from_slice(existing);
            message.extend_from_slice(b", then ");
            message.extend_from_slice(version);
            return Err(TclError::with_code(message, b"TCL PACKAGE VERSIONCONFLICT"));
        }
        packages.push((NameBytes::from(name), version.to_vec()));
        Ok(())
    }

    /// The packages provided so far, in provision order.
    #[must_use]
    pub fn provided_packages(&self) -> Vec<(NameBytes, Vec<u8>)> {
        self.packages.borrow().clone()
    }
}

impl Drop for InterpState {
    /// Tear the commands down as C Tcl does when an interpreter is deleted.
    fn drop(&mut self) {
        let commands = std::mem::take(&mut *self.commands.borrow_mut());
        for entry in commands.into_values() {
            self.with_static_extension(entry.extension.clone(), || entry.run_delete_proc());
        }
    }
}
