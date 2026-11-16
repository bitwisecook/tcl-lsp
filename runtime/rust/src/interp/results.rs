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

//! The interpreter result: setting, appending, reading and resetting it, and the byte views of a value.

use super::{
    native_arguments, native_jim_result, native_operation_currency, new_string, obj_bytes, Code,
    ExceptionState, Interp,
};
use crate::obj;
use crate::obj::TclObj;

impl Interp {
    /// Convert `obj` to the byte view consumed by Tcl's `binary` command.
    ///
    /// Byte-array objects retain their own raw payload. Ordinary string objects
    /// are converted through the release profile instead: Tcl 8 truncates a
    /// wide code point to one byte, while Tcl 9 rejects it. Keeping this at the
    /// interpreter boundary makes every `binary` subcommand use the same
    /// dual-representation and version rule.
    pub(crate) fn binary_bytes(&mut self, value: *mut TclObj) -> Result<Vec<u8>, Code> {
        let Some(policy) = self.native_invocation_dialect().binary_data_conversion() else {
            return Err(self.refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "binary conversion",
                ),
            ));
        };
        self.native_binary_bytes_with(value, policy, true)
    }

    pub(crate) fn native_binary_bytes_with(
        &mut self,
        value: *mut TclObj,
        policy: tcl_registry::native_binary_value::NativeBinaryByteConversion,
        cache: bool,
    ) -> Result<Vec<u8>, Code> {
        if policy == tcl_registry::native_binary_value::NativeBinaryByteConversion::Utf8 {
            return Ok(obj::bytes_of(value));
        }
        let recipe = self.byte_array_string_recipe()?;
        crate::bytearray::native_binary_bytes(value, policy, cache, recipe)
            .map_err(|error| self.error_with_code(error.message().as_bytes(), b"TCL VALUE BYTES"))
    }

    // result

    /// Construct a List retaining the producer's selected string updater.
    /// An unavailable dialect leaves the backing unselected; construction
    /// grants no native conversion or execution authority.
    pub(crate) fn new_list_object(&self, elements: &[*mut TclObj]) -> *mut TclObj {
        let recipe = self
            .native_invocation_dialect()
            .native_string_materialization(Some(
            tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation,
        ));
        match recipe {
            Some(recipe) => crate::list::new_list_obj_native(elements, recipe.protocol()),
            None => crate::list::new_list_obj(elements),
        }
    }

    /// `Tcl_SetObjResult`: retain `obj` into the result slot, release the prior.
    ///
    /// # Safety
    /// `obj` must be a live `TclObj`.
    pub unsafe fn set_obj_result(&mut self, obj: *mut TclObj) {
        if let Some(recipe) = self
            .native_invocation_dialect()
            .native_string_materialization(Some(
            tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation,
        )) {
            // A fresh compound producer retains the selected updater without
            // materialising members or changing their native references.
            if crate::list::native_string_protocol(obj).is_none() {
                let _ = crate::list::seal_string_protocol(obj, recipe.protocol());
            }
            if crate::dict::native_string_protocol(obj).is_none() {
                let _ = crate::dict::seal_string_protocol(obj, recipe.protocol());
            }
        }
        // A different result cannot inherit a primitive getter projection.
        let mut exc = self.exc.borrow_mut();
        exc.primitive_getter = None;
        exc.expression_error_stage = None;
        drop(exc);
        let old = self.result.get();
        // SAFETY: `obj` live (caller); `old` is the interp's owned result.
        unsafe {
            obj::incr_ref_count(obj);
            if self
                .native_invocation_dialect()
                .native_jim_lookup_protocol()
                .is_some()
            {
                let retirement: native_jim_result::JimResultRetirement =
                    native_jim_result::JimResultRetirement::enter(self, old);
                obj::decr_ref_count(old);
                self.result.set(obj);
                drop(retirement);
            } else {
                self.result.set(obj);
                obj::decr_ref_count(old);
            }
        }
    }

    /// Set the result to `obj` (the safe wrapper builtins use on objects they
    /// already hold — argv elements, or fresh objects they just minted).
    ///
    /// Not marked `unsafe`: in the runtime every `TclObj *` in flight is a live
    /// object from our single allocator (the Tcl C model), and builtins handle
    /// these pointers ubiquitously — threading `unsafe` through every call site
    /// would add noise without adding safety. The invariant is upheld by the
    /// eval loop's retain/release discipline.
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn set_result(&mut self, obj: *mut TclObj) {
        // SAFETY: builtins only pass live objects (argv elements / fresh objs).
        unsafe { self.set_obj_result(obj) }
    }

    /// Set the result to a fresh string object with `bytes`.
    pub fn set_result_bytes(&mut self, bytes: &[u8]) {
        let obj = new_string(bytes);
        // SAFETY: fresh obj; set_obj_result retains it, then we drop our 0-ref
        // (the obj was rc 0; set_obj_result took it to rc 1, interp-owned).
        unsafe { self.set_obj_result(obj) };
    }

    /// Set the result to a byte-array object with `bytes` as its exact raw
    /// payload. Its normal string representation is generated only when a
    /// string consumer requests it.
    pub(crate) fn byte_array_string_recipe(
        &mut self,
    ) -> Result<tcl_registry::native_string_materialization::ByteArrayStringRecipe, Code> {
        self.native_invocation_dialect().byte_array_string_recipe(
            Some(tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation),
        ).ok_or_else(|| self.refuse_native_access(tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("byte-array string materialization")))
    }

    /// Create byte-array storage only after its complete updater recipe is selected.
    pub(crate) fn new_native_byte_array(&mut self, bytes: &[u8]) -> Result<*mut TclObj, Code> {
        let recipe = self.byte_array_string_recipe()?;
        Ok(crate::bytearray::new_byte_array(bytes, recipe))
    }

    pub(crate) fn set_result_byte_array(&mut self, bytes: &[u8]) -> Code {
        let obj = match self.new_native_byte_array(bytes) {
            Ok(value) => value,
            Err(code) => return code,
        };
        // SAFETY: fresh byte-array object; the interpreter takes its owning
        // reference exactly as it does for an ordinary fresh string object.
        unsafe { self.set_obj_result(obj) };
        Code::Ok
    }

    /// `Tcl_GetObjResult` — borrowed (interp keeps its +1).
    pub fn get_obj_result(&self) -> *mut TclObj {
        self.result.get()
    }

    /// `Tcl_ResetResult`: an empty result, no error in flight (the next one
    /// starts its own `errorInfo`, `-errorcode` and error stack) and no pending
    /// `return`, so its level and code are the defaults again.
    pub(crate) fn reset_result(&mut self) {
        if self
            .native_invocation_dialect()
            .native_error_log_protocol()
            .is_some()
        {
            if let Err(first) = self.reset_original_c_result() {
                self.refuse_native_execution(first);
            }
            return;
        }
        // The separately selected Jim compatibility entry retains its actual
        // common reset state; it does not borrow the C private-field recipe.
        let Ok(scope) = native_operation_currency::NativeOperationScope::enter(self) else {
            return;
        };
        self.set_result_bytes(b"");
        if scope.currency().ensure_current_or_refuse().is_err() {
            return;
        }
        if self
            .replace_exception_state_checked(scope.currency(), ExceptionState::default())
            .is_err()
        {
            return;
        }
        self.mark_error_stack_reset();
        self.set_return_state(1, Code::Ok);
        if self.clear_return_options_checked().is_err() {
            return;
        }
        let _ = scope.currency().ensure_current_or_refuse();
    }

    /// `Tcl_AppendResult`'s one piece: the result grows by `piece`, in place when
    /// it is a plain string nothing else holds.
    pub(crate) fn append_result_bytes_checked(
        &mut self,
        piece: &[u8],
    ) -> Result<(), tcl_cmd_core::CmdError> {
        let current = self.result.get();
        let arguments = native_arguments::OriginalArguments::enter(self, &[current])?;
        arguments.check_original_for(self, 0, current)?;
        if obj::is_plain_string(current) && !obj::is_shared(current) {
            obj::string_append_inplace(current, piece);
            arguments.check_original_for(self, 0, current)?;
            return Ok(());
        }
        let bytes = arguments.bytes(0)?;
        arguments.check_for(self)?;
        if self.result.get() != current {
            return Err(tcl_cmd_core::CmdError::from_execution_refusal(
                self.refuse_completion_value_access(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "changed original appended result slot",
                    ),
                ),
            ));
        }
        arguments.check_original_for(self, 0, current)?;
        let mut bytes = bytes.to_vec();
        bytes.extend_from_slice(piece);
        self.set_result_bytes(&bytes);
        // A successful replacement can retire the old header; its allocation
        // lease adds no native role. Only the original operation continues.
        arguments.check_for(self)
    }

    /// The current result's string bytes (copied).
    pub fn result_bytes(&self) -> Vec<u8> {
        obj_bytes(self.result.get())
    }

    /// Run an implicit callback while preserving the caller's result and
    /// carried completion options. The callback may inspect and replace the
    /// live result internally; its final value is discarded when `f` returns.
    pub(crate) fn with_preserved_result_checked<R>(
        &mut self,
        f: impl FnOnce(&mut Self) -> R,
    ) -> Result<R, tcl_runtime_api::NativeExecutionError> {
        let scope = native_operation_currency::NativeOperationScope::enter(self)?;
        let saved_currency = native_operation_currency::SavedNativeOperationCurrency::capture(self);
        let saved = self.result.get();
        // SAFETY: preserve the same real saved result role across the callback.
        unsafe { obj::incr_ref_count(saved) };
        let saved_options = self.take_return_options();
        let output = f(self);
        let cleanup =
            self.finish_saved_execution_result(&saved_currency, saved, saved_options, true);
        scope.currency().ensure_current_or_refuse()?;
        cleanup?;
        Ok(output)
    }

    /// The current result **object** — a borrowed pointer (the interp keeps its
    /// reference; the caller must not release it without first taking its own
    /// `+1`). Backs `Commands::dispatch`'s completion capture in `state_traits.rs`.
    pub(crate) fn result_obj(&self) -> *mut TclObj {
        self.result.get()
    }
}
