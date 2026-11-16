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

//! Reading, storing and testing variables by name and by frame: scalars and elements, constants, confined stores, and the read-miss messages.

use super::super::{
    native_operation_currency, native_variable_names, native_variable_teardown, Code, Interp,
};
use crate::frame::VarError;
use crate::namespace::NsId;
use crate::obj;
use crate::obj::TclObj;

impl Interp {
    // variables (the var resolver; `crate::vars`)
    //
    // Every variable op routes through the one classification + link walk
    // (frame-local vs namespace, qualified vs not), instead of the old flat
    // per-frame table. Root and element inputs stay separate at the storage
    // door; complete names use the selected combined-name projection first.

    /// `set name` — borrowed value (the table keeps its +1), or `None`.
    pub(crate) fn var_get(&self, name: &[u8]) -> Option<*mut TclObj> {
        if self.observed_names.borrow().is_some() {
            let level = self.frames.borrow().current_level();
            return self.observed_variable_get(name, level).ok().flatten();
        }
        self.require_variable_name_protocol().ok()?;
        self.read_captured_byte_variable(self.current_ns.get(), None, name, None, false)
            .ok()
            .flatten()
    }

    /// Read a complete byte-valued variable name, retaining its literal
    /// element key through trace dispatch, lookup and failure presentation.
    /// The returned value remains borrowed from the selected variable cell.
    pub(crate) fn read_named_variable(&mut self, name: &[u8]) -> Result<*mut TclObj, Code> {
        if self.observed_names.borrow().is_some() {
            let level = self.frames.borrow().current_level();
            return self
                .observed_variable_get(name, level)
                .map_err(|_| Code::Error)?
                .ok_or(Code::Error);
        }
        let input = self
            .combined_variable_input(name)
            .map_err(|_| Code::Error)?;
        let base = input.root().selected();
        let key = input.element().map(|element| element.selected());
        if let Some(code) = self.fire_read_trace(base, key) {
            return Err(code);
        }
        let value = match key {
            Some(key) => self.var_get_elem(base, key),
            None => self.var_get(base),
        };
        if self.native_execution_refusal().is_some() {
            return Err(Code::Error);
        }
        value.ok_or_else(|| self.no_such_variable(name, None))
    }

    /// Store through a complete fresh byte-valued name. Root-only storage
    /// callers continue to use `var_set`; this ingress owns element splitting.
    pub(crate) fn var_set_named(
        &mut self,
        name: &[u8],
        value: *mut TclObj,
    ) -> Result<(), VarError> {
        if self.observed_names.borrow().is_some() {
            let level = self.frames.borrow().current_level();
            return self.observed_variable_set(name, level, value);
        }
        let input = self.combined_variable_input(name)?;
        match input.element() {
            Some(element) => self.var_set_elem(input.root().selected(), element.selected(), value),
            None => self.var_set(input.root().selected(), value),
        }
    }

    // frame-addressed access (the `VarStore` `FrameId`-honouring path)
    //
    // Resolve `name` as if `level` were the active frame. Used only for a
    // non-active `FrameId`; the active frame keeps the by-name accessors above.

    /// Frame-addressed [`var_get`](Self::var_get).
    pub(crate) fn var_get_at(&self, name: &[u8], level: usize) -> Option<*mut TclObj> {
        if self.observed_names.borrow().is_some() {
            return self.observed_variable_get(name, level).ok().flatten();
        }
        self.require_variable_name_protocol().ok()?;
        self.read_captured_byte_variable(self.current_ns.get(), Some(level), name, None, false)
            .ok()
            .flatten()
    }

    /// Frame-addressed `set name(key)`. The base and element stay separate at
    /// the variable-store boundary, avoiding an ambiguous reconstructed name.
    pub(crate) fn var_get_elem_at(
        &self,
        name: &[u8],
        key: &[u8],
        level: usize,
    ) -> Option<*mut TclObj> {
        self.require_variable_name_protocol().ok()?;
        self.read_captured_byte_variable(self.current_ns.get(), Some(level), name, Some(key), false)
            .ok()
            .flatten()
    }

    /// Frame-addressed [`var_set`](Self::var_set) — the cell takes a **+1**.
    pub(crate) fn var_set_at(
        &mut self,
        name: &[u8],
        obj: *mut TclObj,
        level: usize,
    ) -> Result<(), VarError> {
        if self.store_escapes_at(name, level) {
            return Err(VarError::Confined);
        }
        if self.observed_names.borrow().is_some() {
            return self.observed_variable_set(name, level, obj);
        }
        self.store_captured_byte_variable(self.current_ns.get(), Some(level), name, None, obj)
    }

    /// Frame-addressed `set name(key) value`. The table takes its +1 on `obj`.
    pub(crate) fn var_set_elem_at(
        &mut self,
        name: &[u8],
        key: &[u8],
        obj: *mut TclObj,
        level: usize,
    ) -> Result<(), VarError> {
        if self.store_escapes_at(name, level) {
            return Err(VarError::Confined);
        }
        self.store_captured_byte_variable(self.current_ns.get(), Some(level), name, Some(key), obj)
    }

    /// Operational byte removal in a genuine addressed variable frame.
    pub(crate) fn var_unset_at(&mut self, name: &[u8], level: usize) -> bool {
        if self.observed_names.borrow().is_some() {
            return self.observed_variable_unset(name, level).unwrap_or(false);
        }
        if level == self.frames.borrow().current_level() {
            return self.var_unset(name);
        }
        let Ok(input) = self.combined_variable_input(name) else {
            return false;
        };
        let root = input.root().selected().to_vec();
        let element = input.element().map(|element| element.selected().to_vec());
        self.unset_byte_variable_at(name, &root, element.as_deref(), level, false)
    }

    /// Storage-only removal for the quiet bootstrap clear/install boundary.
    pub(crate) fn quiet_var_unset_at(&mut self, name: &[u8], level: usize) -> bool {
        if self.observed_names.borrow().is_some() {
            return self.observed_variable_unset(name, level).unwrap_or(false);
        }
        self.remove_captured_byte_variable(
            native_variable_teardown::OriginalVariableLocation {
                namespace: self.current_ns.get(),
                level: Some(level),
            },
            name,
            None,
            name,
            false,
            false,
        )
        .unwrap_or(false)
    }

    /// Frame-addressed `unset name(key)`.
    pub(crate) fn var_unset_elem_at(&mut self, name: &[u8], key: &[u8], level: usize) -> bool {
        if level == self.frames.borrow().current_level() {
            return self.var_unset_elem(name, key);
        }
        let Ok(input) = self.separate_variable_input(name, Some(key)) else {
            return false;
        };
        let root = input.root().selected().to_vec();
        let key = input
            .element()
            .expect("separate member")
            .selected()
            .to_vec();
        // Presentation only: the receiver retains separate original parts.
        let mut spelling = name.to_vec();
        spelling.push(b'(');
        spelling.extend_from_slice(&key);
        spelling.push(b')');
        self.unset_byte_variable_at(&spelling, &root, Some(&key), level, true)
    }

    /// Frame-addressed [`var_exists`](Self::var_exists).
    pub(crate) fn var_exists_at(&self, name: &[u8], level: usize) -> bool {
        if self.observed_names.borrow().is_some() {
            return self.observed_variable_exists(name, level).unwrap_or(false);
        }
        if self.require_variable_name_protocol().is_err() {
            return false;
        };
        self.with_captured_byte_variable(
            self.current_ns.get(),
            Some(level),
            name,
            None,
            false,
            |place, currency| place.exists(self, currency),
        )
        .unwrap_or(false)
    }

    /// `set name(key)` — borrowed.
    pub(crate) fn var_get_elem(&self, name: &[u8], key: &[u8]) -> Option<*mut TclObj> {
        self.require_variable_name_protocol().ok()?;
        self.read_captured_byte_variable(self.current_ns.get(), None, name, Some(key), true)
            .ok()
            .flatten()
    }

    /// `set name value` — the cell takes a **+1** on `obj`.
    pub(crate) fn var_set(&mut self, name: &[u8], obj: *mut TclObj) -> Result<(), VarError> {
        if self.store_escapes(name) {
            return Err(VarError::Confined);
        }
        if self.observed_names.borrow().is_some() {
            let level = self.frames.borrow().current_level();
            return self.observed_variable_set(name, level, obj);
        }
        let scope =
            native_operation_currency::NativeOperationScope::enter(self).map_err(|first| {
                self.refuse_native_execution(first);
                VarError::NameProtocolUnavailable
            })?;
        self.store_captured_byte_variable(self.current_ns.get(), None, name, None, obj)?;
        if self.variable_traces_may_observe(name) {
            let input = self.separate_variable_input(name, None)?;
            if self
                .fire_var_trace(input.root().selected(), None, b"write")
                .map_err(|first| {
                    self.refuse_native_execution(first);
                    VarError::NameProtocolUnavailable
                })?
                .is_guest_error()
            {
                // A write trace errored: the value is set, but the command
                // fails (C's TclObjCallVarTraces). `var_error` wraps the
                // message from `pending_err` as `can't set "name": <msg>`.
                scope
                    .currency()
                    .ensure_current_or_refuse()
                    .map_err(|first| {
                        self.refuse_native_execution(first);
                        VarError::NameProtocolUnavailable
                    })?;
                return Err(VarError::TraceError);
            }
        }
        scope
            .currency()
            .ensure_current_or_refuse()
            .map_err(|first| {
                self.refuse_native_execution(first);
                VarError::NameProtocolUnavailable
            })?;
        Ok(())
    }

    /// `set name(key) value`.
    pub(crate) fn var_set_elem(
        &mut self,
        name: &[u8],
        key: &[u8],
        obj: *mut TclObj,
    ) -> Result<(), VarError> {
        if self.store_escapes(name) {
            return Err(VarError::Confined);
        }
        let scope =
            native_operation_currency::NativeOperationScope::enter(self).map_err(|first| {
                self.refuse_native_execution(first);
                VarError::NameProtocolUnavailable
            })?;
        self.store_captured_byte_variable(self.current_ns.get(), None, name, Some(key), obj)?;
        let input = self.separate_variable_input(name, Some(key))?;
        if self.variable_traces_may_observe(name)
            && self
                .fire_var_trace(
                    input.root().selected(),
                    input.element().map(|element| element.selected()),
                    b"write",
                )
                .map_err(|first| {
                    self.refuse_native_execution(first);
                    VarError::NameProtocolUnavailable
                })?
                .is_guest_error()
        {
            scope
                .currency()
                .ensure_current_or_refuse()
                .map_err(|first| {
                    self.refuse_native_execution(first);
                    VarError::NameProtocolUnavailable
                })?;
            return Err(VarError::TraceError);
        }
        scope
            .currency()
            .ensure_current_or_refuse()
            .map_err(|first| {
                self.refuse_native_execution(first);
                VarError::NameProtocolUnavailable
            })?;
        Ok(())
    }

    /// Store `obj` into the `(base, elem)` variable and, on success, publish it
    /// as the interp result — while holding a **protective reference** across the
    /// store. `var_set`/`var_set_elem` fire the write trace, and a trace that
    /// `unset`s the variable drops the store's reference; for a *fresh* `obj`
    /// (the value `lappend`/`append` just built) that was the only reference, so
    /// without this bracket the object is freed mid-command and the following
    /// `set_result` reads freed memory — a use-after-free that a write-traced
    /// `lappend`/`append` hits (append-7.x, var-traces). On a store error the
    /// bracket releases the reference (freeing a fresh `obj`, as the old
    /// `drop_fresh` did) and the error is returned for the caller to render.
    pub(crate) fn store_var_result(
        &mut self,
        base: &[u8],
        elem: Option<&[u8]>,
        obj: *mut TclObj,
    ) -> Result<(), VarError> {
        let scope =
            native_operation_currency::NativeOperationScope::enter(self).map_err(|first| {
                self.refuse_native_execution(first);
                VarError::NameProtocolUnavailable
            })?;
        // SAFETY: `obj` is a live object the caller just built or read; the
        // increment/decrement bracket keeps it alive across the trace firing.
        unsafe { obj::incr_ref_count(obj) };
        let stored = match elem {
            Some(k) => self.var_set_elem(base, k, obj),
            None => self.var_set(base, obj),
        };
        if stored.is_ok() {
            // The result is the variable's value *after* the write trace ran, not
            // necessarily the value we stored: a trace may have rewritten the
            // variable (C returns the new value) or unset it (C returns empty).
            // `var_get*` are trace-free store reads, so this fires no read trace.
            let final_val = match elem {
                Some(k) => self.var_get_elem(base, k),
                None => self.var_get(base),
            };
            match final_val {
                Some(v) => self.set_result(v),
                None => self.set_result_bytes(b""),
            }
        }
        // SAFETY: balances the protective increment above (the store retained its
        // own reference on success; `set_result` retained the result's).
        unsafe { obj::decr_ref_count(obj) };
        scope
            .currency()
            .ensure_current_or_refuse()
            .map_err(|first| {
                self.refuse_native_execution(first);
                VarError::NameProtocolUnavailable
            })?;
        stored
    }

    /// Flag the scalar `name` `const` (the `const` command, after its value is
    /// stored and its write traces have fired).
    pub(crate) fn mark_constant(&self, name: &[u8]) {
        if self.require_variable_name_protocol().is_err() {
            return;
        }
        crate::vars::mark_constant(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            self.current_ns.get(),
            name,
        );
    }

    /// `Some(error)` — `can't set "name": variable is a constant` — when the
    /// (possibly `arr(idx)`) `name` targets a `const` scalar; `None` otherwise.
    /// The read-modify-write commands (`lappend`/`dict set`/`regsub`/`gets`)
    /// call this before mutating, since their in-place value update would
    /// otherwise bypass the store-time constant check.
    pub(crate) fn const_write_check(&mut self, name: &[u8]) -> Option<Code> {
        let (base, elem) = match self.variable_name_parts(name) {
            Ok(parts) => parts,
            Err(_) => return Some(Code::Error),
        };
        if self.store_escapes(&base) {
            return Some(self.confined_store_error(name));
        }
        if elem.is_none() && self.is_constant(&base) {
            let mut m = b"can't set \"".to_vec();
            m.extend_from_slice(name);
            m.extend_from_slice(b"\": variable is a constant");
            return Some(self.set_error(&m));
        }
        None
    }

    /// Confine every later store to the running procedure's own frame, and
    /// remove the globals the host seeded (`::env`, `::tcl_platform` and the
    /// library paths), so an evaluation reads nothing of the machine it runs
    /// on and leaves nothing behind for the next one to read.
    ///
    /// A store, an array's creation and an unset are refused, as a Tcl error
    /// raised before anything is written or removed, when they would land
    /// anywhere else: at the global level, in a namespace (a qualified name,
    /// `variable`, `namespace eval`), in another frame (`uplevel`), or through
    /// a link a local holds to a variable outside the frame (`global`,
    /// `upvar`). So is a draw from the `rand()` generator,
    /// whose seed every evaluation shares. Reads are unaffected, and an error
    /// is not published to `::errorInfo` and `::errorCode`, which are globals.
    pub fn confine_stores(&mut self) {
        self.disarm_env_trace();
        // The host's own globals go before the confinement that would refuse
        // their removal.
        for name in tcl_platform::bootstrap::HOST_ARRAYS
            .iter()
            .chain(tcl_platform::bootstrap::HOST_PATH_GLOBALS)
        {
            self.var_unset_at(tcl_syntax::naming::root_unrooted_key(name).as_bytes(), 0);
        }
        self.stores_confined.set(true);
    }

    /// Whether stores are confined ([`Self::confine_stores`]).
    #[must_use]
    pub fn stores_confined(&self) -> bool {
        self.stores_confined.get()
    }

    /// Whether a store to `name` from the current frame must be refused:
    /// stores are confined and it would land outside the running procedure's
    /// own frame.
    pub(crate) fn store_escapes(&self, name: &[u8]) -> bool {
        self.stores_confined.get()
            && !crate::vars::lands_in_own_frame(
                &self.frames.borrow(),
                &self.namespaces.borrow(),
                self.current_ns.get(),
                name,
            )
    }

    /// [`Self::store_escapes`] for a store resolved as if `level` were active.
    pub(crate) fn store_escapes_at(&self, name: &[u8], level: usize) -> bool {
        self.stores_confined.get()
            && !crate::vars::lands_in_own_frame_at(
                &self.frames.borrow(),
                &self.namespaces.borrow(),
                name,
                level,
            )
    }

    /// The error a confined store to `name` is refused with — the bytecode
    /// VM's words and code.
    pub(crate) fn confined_store_error(&mut self, name: &[u8]) -> Code {
        self.confined_store_error_using(name, None)
    }

    pub(crate) fn confined_store_error_using(
        &mut self,
        name: &[u8],
        stage: native_variable_names::OriginalCNameStage<'_>,
    ) -> Code {
        if let Err(first) = self.check_original_c_name_stage(stage) {
            return self.report_cmd_error(first);
        }

        let mut message = b"can't set \"".to_vec();
        message.extend_from_slice(name);
        message.extend_from_slice(b"\": stores are confined to the activation");
        self.error_with_code_using(&message, b"TCL WRITE VARNAME", stage)
    }

    /// The error a confined unset of `name` is refused with — the bytecode
    /// VM's words and code.
    pub(crate) fn confined_unset_error(&mut self, name: &[u8]) -> Code {
        self.confined_unset_error_using(name, None)
    }

    pub(crate) fn confined_unset_error_using(
        &mut self,
        name: &[u8],
        stage: native_variable_names::OriginalCNameStage<'_>,
    ) -> Code {
        if let Err(first) = self.check_original_c_name_stage(stage) {
            return self.report_cmd_error(first);
        }

        let mut message = b"can't unset \"".to_vec();
        message.extend_from_slice(name);
        message.extend_from_slice(b"\": stores are confined to the activation");
        self.error_with_code_using(&message, b"TCL UNSET VARNAME", stage)
    }

    /// The error a draw from the `rand()` generator is refused with while
    /// stores are confined, or `None` when they are not. The math functions
    /// need the numeric tower, so without it there is no generator to refuse.
    #[cfg(have_tommath)]
    pub(crate) fn confined_generator_error(&mut self, function: &[u8]) -> Option<Code> {
        if !self.stores_confined.get() {
            return None;
        }
        let mut message = b"can't call \"".to_vec();
        message.extend_from_slice(function);
        message.extend_from_slice(
            b"\": stores are confined to the activation and the generator's seed is not",
        );
        Some(self.error(&message))
    }

    /// Whether `name` resolves to a `const` scalar.
    pub(crate) fn is_constant(&self, name: &[u8]) -> bool {
        if self.require_variable_name_protocol().is_err() {
            return false;
        };
        crate::vars::is_constant(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            self.current_ns.get(),
            name,
        )
    }

    /// Whether `name`, resolved as if `level` were active, is a `const` cell.
    pub(crate) fn is_constant_at(&self, name: &[u8], level: usize) -> bool {
        if self.require_variable_name_protocol().is_err() {
            return false;
        };
        crate::vars::is_constant_at(
            &self.frames.borrow(),
            &self.namespaces.borrow(),
            name,
            level,
        )
    }

    /// Ensure `name` is an array (creating an empty one if unset) — backs
    /// `array set name {}` with an empty value list. A scalar `name` errors.
    pub(crate) fn ensure_array(&self, name: &[u8]) -> Result<(), VarError> {
        if self.observed_names.borrow().is_some() {
            let level = self.frames.borrow().current_level();
            return self.observed_ensure_array(name, level);
        }
        self.require_variable_name_protocol()?;
        self.with_captured_byte_variable(
            self.current_ns.get(),
            None,
            name,
            None,
            true,
            |place, currency| place.ensure_array(self, currency),
        )
    }

    /// Ensure an array in the variable table of frame `level`, whatever frame
    /// is current (C's `TCL_GLOBAL_ONLY` for level 0).
    pub(crate) fn ensure_array_at(&self, name: &[u8], level: usize) -> Result<(), VarError> {
        if self.observed_names.borrow().is_some() {
            return self.observed_ensure_array(name, level);
        }
        self.require_variable_name_protocol()?;
        self.with_captured_byte_variable(
            self.current_ns.get(),
            Some(level),
            name,
            None,
            true,
            |place, currency| place.ensure_array(self, currency),
        )
    }

    /// Whether `name` resolves to an array variable (`set a` array-vs-scalar
    /// diagnostic, `array exists`).
    pub(crate) fn var_is_array(&self, name: &[u8]) -> bool {
        if self.require_variable_name_protocol().is_err() {
            return false;
        };
        self.with_captured_byte_variable(
            self.current_ns.get(),
            None,
            name,
            None,
            false,
            |place, currency| place.is_array(self, currency),
        )
        .unwrap_or(false)
    }

    /// Resolve a `global`/`variable`/`upvar` name argument to its
    /// `(target namespace, simple tail)`, in the given `context_ns` (global for
    /// `global`, the current ns for `variable`). `None` if the name is qualified
    /// into a namespace that doesn't exist.
    pub(crate) fn resolve_var_target(
        &self,
        context_ns: NsId,
        name: &[u8],
    ) -> Option<(NsId, Vec<u8>)> {
        let protocol = self.require_variable_name_protocol().ok()?;
        let input = protocol.variable_root_input(name);
        if protocol.is_jim084()
            || input.qualification() != tcl_syntax::naming::NativeNameQualification::Unqualified
        {
            self.namespaces.borrow().var_home(context_ns, name)
        } else {
            Some((context_ns, input.selected().to_vec()))
        }
    }

    /// Qualification selected from the original variable operand.
    pub(crate) fn variable_is_qualified(&self, name: &[u8]) -> bool {
        self.require_variable_name_protocol().is_ok_and(|protocol| {
            protocol.variable_root_input(name).qualification()
                != tcl_syntax::naming::NativeNameQualification::Unqualified
        })
    }

    /// The current call-frame level (`upvar` relative-level arithmetic).
    pub(crate) fn current_level(&self) -> usize {
        self.frames.borrow().current_level()
    }

    // introspection (`info` / `array`)

    /// `info exists name` — whether a scalar/array/element variable is set
    /// (splitting `arr(key)`).
    pub(crate) fn var_exists(&self, name: &[u8]) -> bool {
        if self.observed_names.borrow().is_some() {
            let level = self.frames.borrow().current_level();
            return self.observed_variable_exists(name, level).unwrap_or(false);
        }
        let Ok(input) = self.combined_variable_input(name) else {
            return false;
        };
        let base = input.root().selected();
        self.with_captured_byte_variable(
            self.current_ns.get(),
            None,
            base,
            input.element().map(|element| element.selected()),
            false,
            |place, currency| place.exists(self, currency),
        )
        .unwrap_or(false)
    }

    /// The element names of array `name` (`array names`/`get`), or `None`.
    pub(crate) fn array_names(&self, name: &[u8]) -> Option<Vec<Vec<u8>>> {
        self.require_variable_name_protocol().ok()?;
        self.with_captured_byte_variable(
            self.current_ns.get(),
            None,
            name,
            None,
            false,
            |place, currency| place.array_names(self, currency),
        )
        .ok()
        .flatten()
    }

    /// Read a variable's value bytes via the variable resolver.
    pub(in super::super) fn read_var(
        &mut self,
        name: &[u8],
        index: Option<&[u8]>,
    ) -> Option<Vec<u8>> {
        if self.observed_name_policy_selected() {
            let value = self.observed_substitution_receiver(name, index).ok()??;
            return tcl_syntax::value::ValueOps::native_string_bytes(self, &value)
                .ok()
                .map(|bytes| bytes.to_vec());
        }
        self.require_variable_name_protocol().ok()?;
        let input = match index {
            Some(index) => self.separate_variable_input(name, Some(index)),
            None => self.combined_variable_input(name),
        }
        .ok()?;
        let name = input.root().selected();
        let index = input.element().map(|element| element.selected());
        self.with_captured_byte_variable(
            self.current_ns.get(),
            None,
            name,
            index,
            false,
            |place, currency| {
                let Some(value) = place.read(self, currency, index.is_some())? else {
                    return Ok(None);
                };
                let reached = self.native_object_string_bytes(value);
                place.check(self, currency)?;
                reached
                    .map(|bytes| Some(bytes.to_vec()))
                    .map_err(Into::into)
            },
        )
        .ok()
        .flatten()
    }

    pub(in super::super) fn no_such_variable(&mut self, name: &[u8], index: Option<&[u8]>) -> Code {
        self.no_such_variable_using(name, index, None)
    }

    pub(in super::super::super) fn no_such_variable_using(
        &mut self,
        name: &[u8],
        index: Option<&[u8]>,
        stage: native_variable_names::OriginalCNameStage<'_>,
    ) -> Code {
        if let Err(first) = self.check_original_c_name_stage(stage) {
            return self.report_cmd_error(first);
        }

        if self.host_refusal_pending() {
            return Code::Error;
        }
        let msg = self.read_miss_msg(name, index);
        self.error_using(&msg, stage)
    }

    /// Build the C-faithful `can't read "NAME": …` message for a failed variable
    /// read, distinguishing the three cases `tclVar.c` reports: a scalar read of
    /// an array (`variable is array`), a missing element of an *existing* array
    /// (`no such element in array`), and a wholly missing variable (`no such
    /// variable`). `base`/`index` are the split reference (`base(index)`).
    pub(crate) fn read_miss_msg(&self, original: &[u8], index: Option<&[u8]>) -> Vec<u8> {
        use tcl_syntax::naming::{
            NativeVariableDiagnosticOperation as Operation,
            NativeVariableDiagnosticReason as Reason, NativeVariableFailureSite as Site,
            NativeVariableInputForm,
        };
        let input = match index {
            Some(element) => self.separate_variable_input(original, Some(element)),
            None => self.combined_variable_input(original),
        };
        let Ok(input) = input else {
            return Vec::new();
        };
        let root = input.root().selected();
        let element = input.element().map(|element| element.selected());
        let reason = if self.var_is_array(root) {
            if element.is_some() {
                Reason::NoSuchElement
            } else {
                Reason::IsArray
            }
        } else if element.is_some() && self.var_get(root).is_some() {
            Reason::NotArray
        } else {
            Reason::NoSuchVariable
        };
        let form = match index {
            Some(element) => NativeVariableInputForm::Separate {
                root: original,
                element: Some(element),
            },
            None => NativeVariableInputForm::Combined(original),
        };
        let protocol = input.root().protocol();
        let diagnostic = tcl_syntax::naming::report_native_variable_diagnostic_at(
            protocol,
            Operation::Read,
            reason,
            if reason == Reason::IsArray {
                Site::ValueRead
            } else {
                Site::NameLookup
            },
            form,
        );
        let Ok(diagnostic) = diagnostic else {
            self.clone().refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "variable read diagnostic",
                ),
            );
            return Vec::new();
        };
        let mut message = b"can't read \"".to_vec();
        message.extend_from_slice(&diagnostic.name);
        message.extend_from_slice(b"\": ");
        message.extend_from_slice(diagnostic.reason.message().as_bytes());
        message
    }
}
