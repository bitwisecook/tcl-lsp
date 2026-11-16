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

//! `ValueOps` for the WASM runtime — binds the portable `tcl-cmd-core` command
//! logic to the runtime's 24-byte C-ABI `*mut TclObj` value model.
//!
//! This is the **opposite** value model from the bytecode VM's `Rc<Obj>`:
//! manually refcounted raw objects over the shared linear memory. The same
//! shared command helpers run over it unchanged — that is the entire point of
//! the value seam. Coercion reuses `tcl_syntax::number`, so it is byte-for-byte
//! identical to the VM's `ValueOps`.
//!
//! The copy-on-write asymmetry the contract is designed around is visible here:
//! [`ValueOps::try_append_str_in_place`] performs the runtime's amortised
//! in-place string growth when the object is an unshared plain string,
//! whereas the VM always copies.
//!
//! Byte-array representation is runtime-only: it has no separate LSP request
//! or VS Code UI surface. Its tests therefore sit beside the `ValueOps` seam
//! and run scripts through the real runtime, while the compiler's registry
//! data separately drives static byte-array diagnostics.

mod append_objects;
mod clock_date_objects;
mod dictionary_objects;
mod increment_objects;
mod interp_impl;

pub(crate) use append_objects::RuntimeAppendObjects;
pub(crate) use clock_date_objects::RuntimeClockDateOps;
pub(crate) use dictionary_objects::RuntimeDictionaryObjects;
pub(crate) use increment_objects::{
    RuntimeLegacyIncrementAmountOps, RuntimeLegacyIncrementObjects,
};

type RuntimeDictionaryString = dyn Fn(
    *mut TclObj,
    &dyn Fn() -> Result<(), tcl_cmd_core::CmdError>,
) -> Result<std::rc::Rc<[u8]>, tcl_cmd_core::CmdError>;

#[cfg(test)]
#[path = "value_ops/native_coercion_tests.rs"]
mod native_coercion_tests;
#[path = "value_ops/native_concat.rs"]
mod native_concat;
#[cfg(test)]
#[path = "value_ops/native_concat_tests.rs"]
mod native_concat_tests;
#[path = "value_ops/native_indexed_append.rs"]
mod native_indexed_append;
#[cfg(test)]
#[path = "value_ops/native_integer_refusal_tests.rs"]
mod native_integer_refusal_tests;
mod native_jim_cat;
#[path = "value_ops/native_jim_index.rs"]
mod native_jim_index;
pub(crate) use native_jim_cat::OriginalJimStringCatConstruction;
pub(crate) use native_jim_index::list_length_using;
#[path = "value_ops/native_list_index.rs"]
pub(crate) mod native_list_index;
#[cfg(test)]
#[path = "value_ops/original_name_birth_tests.rs"]
mod original_name_birth_tests;

use std::rc::Rc;

use tcl_syntax::number::Number;
#[cfg(have_tommath)]
use tcl_syntax::number::{self, Radix};
#[cfg(have_tommath)]
use tcl_syntax::value::IntegerMagnitude;
use tcl_syntax::value::ValueError;
#[cfg(test)]
use tcl_syntax::value::ValueOps;

use crate::interp::{obj_bytes, Interp};
use crate::obj::{self, TclObj};

/// Encode a checked `tcl-cmd-core` string result back
/// to a Tcl string representation for [`ValueOps::new_str`]/[`ValueOps::new_string`].
///
/// Binary conversion is deliberately not performed here. The result remains a
/// normal Unicode string; the central byte-array conversion in
/// [`Interp::binary_bytes`] later applies the emulated Tcl release's policy.
/// That is why Tcl 8 truncates `Ÿ` to `x`, while Tcl 9 raises instead.
fn str_to_bytes(s: &str) -> Vec<u8> {
    s.as_bytes().to_vec()
}

impl Interp {
    /// A string object for host text, in the selected native spelling of NUL.
    pub(crate) fn new_host_text(&self, text: &str) -> *mut TclObj {
        match self.native_invocation_dialect().native_string_protocol() {
            Some(protocol) => obj::new_string_bytes(&protocol.host_text_bytes(text)),
            None => obj::new_string_bytes(&str_to_bytes(text)),
        }
    }
}

/// Opaque original-object transport for the shared append and regsub contracts.
/// Object acquisition and pointer access remain internal to Runtime adapters.
pub struct RuntimeAppendValue {
    pointer: *mut TclObj,
    retained: bool,
}

impl RuntimeAppendValue {
    pub(crate) fn borrowed(pointer: *mut TclObj) -> Self {
        Self {
            pointer,
            retained: false,
        }
    }
    pub(crate) fn fresh_string(bytes: &[u8]) -> Self {
        Self::retain(crate::interp::new_string(bytes))
    }
    /// Borrow pointer geometry without retaining or converting the original.
    pub(crate) fn pointer(&self) -> &*mut TclObj {
        &self.pointer
    }
    pub(crate) fn as_ptr(&self) -> *mut TclObj {
        self.pointer
    }
    /// Move this working reference into the normal unowned result transport.
    /// The next actual receiver retains it; no owning reference is leaked or
    /// dropped while a fresh native refcount-zero result is in transit.
    fn into_native_unowned(mut self) -> *mut TclObj {
        if !self.retained {
            return self.pointer;
        }
        self.retained = false;
        // SAFETY: this handle transfers its one independently owned +1.
        unsafe { obj::Owned::from_raw(self.pointer) }.into_native_unowned()
    }

    pub(crate) fn retain(pointer: *mut TclObj) -> Self {
        // SAFETY: each retained working object owns one actual native reference.
        unsafe { obj::incr_ref_count(pointer) };
        Self {
            pointer,
            retained: true,
        }
    }
}
impl Clone for RuntimeAppendValue {
    fn clone(&self) -> Self {
        Self::retain(self.pointer)
    }
}
impl Drop for RuntimeAppendValue {
    fn drop(&mut self) {
        if self.retained {
            // SAFETY: balances the working object's original owning reference.
            unsafe { obj::decr_ref_count(self.pointer) };
        }
    }
}

/// A ValueOps coercion retains its original interpreter and allocation without
/// adding a native reference. Its currency grants no getter or numeric purpose.
struct ValueCoercion {
    original: obj::NativeObjectLifetime,
    scope: crate::interp::native_operation_currency::NativeOperationScope,
    interpreter: Interp,
}

impl ValueCoercion {
    fn execution_error(
        interpreter: &Interp,
        first: tcl_runtime_api::NativeExecutionError,
    ) -> ValueError {
        let value_error = match &first {
            tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(cause) => (*cause).into(),
            // ValueOps has a value-domain error type. The actual interpreter
            // retains the full first execution cause for its command/ABI owner.
            _ => ValueError::ScalarNumericInputUnavailable,
        };
        interpreter.clone().refuse_native_execution(first);
        value_error
    }

    fn value_error(interpreter: &Interp, error: ValueError) -> ValueError {
        if let Some(cause) = error.native_access_refusal() {
            interpreter.clone().refuse_native_access(cause);
        }
        error
    }

    fn enter(interpreter: &Interp, original: *mut TclObj) -> Result<Self, ValueError> {
        let interpreter = interpreter.clone();
        let scope =
            crate::interp::native_operation_currency::NativeOperationScope::enter(&interpreter)
                .map_err(|first| Self::execution_error(&interpreter, first))?;
        obj::check_native_liveness(original)
            .map_err(|error| Self::value_error(&interpreter, error))?;
        Ok(Self {
            original: obj::NativeObjectLifetime::retain(original),
            scope,
            interpreter,
        })
    }

    fn check(&self) -> Result<(), ValueError> {
        self.scope
            .currency()
            .ensure_current_or_refuse()
            .map_err(|first| Self::execution_error(&self.interpreter, first))?;
        obj::check_native_liveness(self.original.as_ptr())
            .map_err(|error| Self::value_error(&self.interpreter, error))
    }

    fn after<T>(&self, result: Result<T, ValueError>) -> Result<T, ValueError> {
        self.check()?;
        result.map_err(|error| Self::value_error(&self.interpreter, error))
    }

    fn scalar(
        &self,
        kind: tcl_syntax::scalar_getter::NativeScalarGetterKind,
    ) -> Result<tcl_syntax::scalar_getter::NativeScalarGetterValue, ValueError> {
        self.check()?;
        let result = crate::capi::read_scalar_for_interpreter(
            &self.interpreter,
            self.original.as_ptr(),
            kind,
        );
        self.check()?;
        result.map_err(|error| match error {
            crate::capi::ScalarReadError::Guest(error) => error,
            crate::capi::ScalarReadError::Access(
                crate::capi::NativeScalarObjectAccessError::Value(error),
            ) => Self::value_error(&self.interpreter, error),
            crate::capi::ScalarReadError::Access(
                crate::capi::NativeScalarObjectAccessError::Execution(first),
            ) => Self::execution_error(&self.interpreter, first),
        })
    }

    fn integer_number(&self) -> Result<Option<Number>, ValueError> {
        self.check()?;
        let associated = self
            .interpreter
            .associate_native_jim_arguments(&[self.original.as_ptr()]);
        self.after(associated)?;
        let result = crate::typed_value::scalar_number_with_currency(
            self.original.as_ptr(),
            self.interpreter.native_invocation_dialect(),
            true,
            Some(self.scope.currency()),
        );
        self.check()?;
        result.map_err(|first| Self::execution_error(&self.interpreter, first))
    }

    fn original_string(&self) -> Result<Vec<u8>, ValueError> {
        self.check()?;
        self.after(
            self.interpreter
                .native_object_string_bytes(self.original.as_ptr()),
        )
        .map(|bytes| bytes.to_vec())
    }
}

/// Original entered increment currency and actual numeric Host, without
/// borrowing the interpreter across a getter, COW callback or cache retirement.
struct IncrementContext {
    currency: crate::interp::native_operation_currency::NativeOperationCurrency,
    host: Rc<dyn tcl_platform::Host>,
    abi: std::cell::Cell<Option<tcl_platform::NativeCIntegerAbi>>,
    original: Option<Rc<RuntimeDictionaryChecks>>,
}
impl IncrementContext {
    fn selected(interp: &Interp) -> Result<Self, tcl_cmd_core::CmdError> {
        Ok(Self {
            currency: crate::interp::native_operation_currency::NativeOperationCurrency::issue(
                interp,
            )
            .map_err(tcl_cmd_core::CmdError::from_execution_refusal)?,
            host: interp.host(),
            abi: std::cell::Cell::new(None),
            original: None,
        })
    }
    fn check(&self) -> Result<(), tcl_cmd_core::CmdError> {
        self.currency
            .ensure_current_or_refuse()
            .map_err(tcl_cmd_core::CmdError::from_execution_refusal)?;
        self.original
            .as_ref()
            .map_or(Ok(()), |original| original.check())
    }
    fn retain(&self, error: tcl_cmd_core::CmdError) -> tcl_cmd_core::CmdError {
        match &self.original {
            Some(original) => (original.retain)(error),
            None => error,
        }
    }
    fn refuse(&self, error: ValueError) -> tcl_cmd_core::CmdError {
        match &self.original {
            Some(original) => self.retain((original.refuse)(error)),
            None => error.into(),
        }
    }
    fn stage(&self, value: *mut TclObj) -> Result<(), ValueError> {
        self.check()
            .map_err(|_| ValueError::ScalarNumericInputUnavailable)?;
        crate::typed_value::scalar_object_stage_current(value, Some(&self.currency))
    }
    fn after<T>(&self, result: Result<T, ValueError>) -> Result<T, tcl_cmd_core::CmdError> {
        let result = result.map_err(|error| self.refuse(error));
        self.check()?;
        result
    }
    fn prepare(
        &self,
        original: Option<&RuntimeAppendValue>,
    ) -> Result<RuntimeAppendValue, tcl_cmd_core::CmdError> {
        self.check()?;
        if let Some(value) = original {
            obj::check_native_liveness(value.as_ptr())?;
        }
        let prepared = RuntimeAppendValue::retain(match original {
            Some(value) if obj::is_shared(value.as_ptr()) => {
                if self.original.is_some() {
                    obj::duplicate_with_stage_check(
                        value.as_ptr(),
                        &mut || {
                            self.check()?;
                            obj::check_native_liveness(value.as_ptr())
                                .map_err(|error| self.refuse(error))
                        },
                        &mut |error| {
                            self.retain(error.clone());
                        },
                    )?
                } else {
                    obj::duplicate(value.as_ptr())
                }
            }
            Some(value) => value.as_ptr(),
            None => obj::new_wide_int_obj(0),
        });
        self.check()?;
        Ok(prepared)
    }
    fn scalar(
        &self,
        value: *mut TclObj,
        dialect: tcl_registry::InvocationDialect,
        kind: tcl_syntax::scalar_getter::NativeScalarGetterKind,
    ) -> Result<tcl_syntax::scalar_getter::NativeScalarGetterValue, tcl_cmd_core::CmdError> {
        self.check()?;
        let environment = self.host.numeric_environment().map(|actual| {
            crate::interp::native_operation_currency::CheckedNumericEnvironment::new(
                actual,
                Some(&self.currency),
                &self.abi,
            )
        });
        if environment.is_none() {
            let protocol = dialect
                .native_scalar_getter_protocol()
                .ok_or(ValueError::ScalarNumericInputUnavailable)?;
            if protocol.is_jim084() || protocol.tcl_version() == Some(tcl_dialect::TclVersion::V8_4)
            {
                // Cache eligibility is a pure selected recipe query. A real
                // cached purpose needs no fresh Host conversion; a missing
                // fresh legacy environment cannot become a neutral parser.
                let cache = self.after(obj::native_scalar_cache(value))?;
                let cached = cache
                    .as_ref()
                    .and_then(|cache| protocol.cached_conversion(kind, cache, None).ok().flatten());
                if cached.is_none() {
                    return Err(ValueError::ScalarNumericInputUnavailable.into());
                }
            }
        }
        let stage = || self.stage(value);
        let outcome = crate::typed_value::native_scalar_probe_with_stage_check(
            value,
            dialect,
            kind,
            environment
                .as_ref()
                .map(|environment| environment as &dyn tcl_platform::NumericEnvironment),
            Some(&self.currency),
            self.original
                .as_ref()
                .map(|_| &stage as &dyn Fn() -> Result<(), ValueError>),
        );
        match self.after(outcome)? {
            Ok(value) => Ok(value),
            Err(failure) => {
                let presentation =
                    crate::typed_value::native_scalar_failure_presentation_with_stage_check(
                        value,
                        dialect,
                        kind,
                        failure,
                        Some(&self.currency),
                        self.original
                            .as_ref()
                            .map(|_| &stage as &dyn Fn() -> Result<(), ValueError>),
                    );
                let presentation = self.after(presentation)?;
                Err(ValueError::NativeScalarGetter(Box::new(presentation)).into())
            }
        }
    }
    fn store(
        &self,
        value: *mut TclObj,
        cache: tcl_syntax::scalar_getter::NativeScalarCache,
        dialect: tcl_registry::InvocationDialect,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        self.check()?;
        let protocol = dialect
            .native_scalar_getter_protocol()
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        let mut check = || {
            crate::typed_value::scalar_object_stage_current(value, Some(&self.currency))?;
            if self.original.is_some() {
                self.stage(value)?;
            }
            Ok(())
        };
        let stored =
            obj::adopt_native_scalar_cache_with_stage_check(value, cache, protocol, &mut check);
        self.after(stored)?;
        obj::invalidate_string(value);
        self.check()
    }
}

/// Selected modern C integer update operations on original physical objects.
pub(crate) struct RuntimeIncrementObjects {
    cached_sum: Option<(i64, i64)>,
    dialect: tcl_registry::InvocationDialect,
    context: IncrementContext,
}

struct RuntimeDictionaryChecks {
    current: Box<dyn Fn() -> Result<(), tcl_cmd_core::CmdError>>,
    string: Box<RuntimeDictionaryString>,
    parents: std::cell::RefCell<Vec<crate::dict::PreparedDictionaryObserver>>,
    refuse: Box<dyn Fn(ValueError) -> tcl_cmd_core::CmdError>,
    retain: Box<dyn Fn(tcl_cmd_core::CmdError) -> tcl_cmd_core::CmdError>,
}
impl RuntimeDictionaryChecks {
    fn new(
        current: impl Fn() -> Result<(), tcl_cmd_core::CmdError> + 'static,
        string: impl Fn(
                *mut TclObj,
                &dyn Fn() -> Result<(), tcl_cmd_core::CmdError>,
            ) -> Result<std::rc::Rc<[u8]>, tcl_cmd_core::CmdError>
            + 'static,
        refuse: impl Fn(ValueError) -> tcl_cmd_core::CmdError + 'static,
        retain: impl Fn(tcl_cmd_core::CmdError) -> tcl_cmd_core::CmdError + 'static,
    ) -> std::rc::Rc<Self> {
        std::rc::Rc::new(Self {
            current: Box::new(current),
            string: Box::new(string),
            refuse: Box::new(refuse),
            retain: Box::new(retain),
            parents: std::cell::RefCell::new(Vec::new()),
        })
    }

    fn check(&self) -> Result<(), tcl_cmd_core::CmdError> {
        (self.current)()?;
        for parent in self.parents.borrow().iter() {
            parent.check().map_err(|error| (self.refuse)(error))?;
        }
        Ok(())
    }
    fn bytes(&self, original: *mut TclObj) -> Result<std::rc::Rc<[u8]>, tcl_cmd_core::CmdError> {
        self.bytes_using(original, &|| Ok(()))
    }
    fn bytes_using(
        &self,
        original: *mut TclObj,
        extra: &dyn Fn() -> Result<(), tcl_cmd_core::CmdError>,
    ) -> Result<std::rc::Rc<[u8]>, tcl_cmd_core::CmdError> {
        (self.string)(original, &|| {
            self.check()?;
            extra()
        })
    }
}

#[path = "value_ops/original_append.rs"]
mod original_append;
pub(crate) use original_append::OriginalRuntimeAppendObjects;

impl Interp {
    /// Validate original primary origin without reaching a String getter.
    pub(crate) fn native_string_primary_admission(
        &self,
        value: *mut TclObj,
    ) -> Result<(), ValueError> {
        obj::check_native_liveness(value)?;
        obj::native_frame_level_cache_in(value, self.native_invocation_dialect())?;
        if obj::native_instruction_name::cache(value).is_some_and(|name| {
            self.invocation_string_protocol()
                != Some(tcl_syntax::native_string::NativeStringProtocol::C(
                    name.version(),
                ))
        }) {
            return Err(ValueError::CommandProtocolUnavailable(
                "foreign instruction-name primary",
            ));
        }
        if let Some(cache) = obj::native_jim_subcommand::cache(value) {
            let table = self
                .native_invocation_dialect()
                .native_jim_namespace_subcommand_table()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "foreign Jim subcommand primary issuer",
                ))?;
            if table.cached_entry(&cache).is_none() {
                return Err(ValueError::CommandProtocolUnavailable(
                    "foreign Jim subcommand primary table",
                ));
            }
        }
        if obj::native_jim_enum::cache(value).is_some()
            && self
                .native_invocation_dialect()
                .native_jim_enum_protocol()
                .is_none()
        {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "foreign Jim option origin",
            ));
        }
        if obj::native_property_name::is_cached(value)
            && self
                .native_invocation_dialect()
                .native_property_lookup_protocol()
                .is_none()
        {
            return Err(ValueError::CommandProtocolUnavailable(
                "foreign property-name primary",
            ));
        }
        Ok(())
    }

    /// Materialise the same original through the selected physical updater.
    /// The returned byte owner supplies no variable/cache receiver authority.
    pub(crate) fn native_object_string_bytes(
        &self,
        value: *mut TclObj,
    ) -> Result<Rc<[u8]>, ValueError> {
        self.native_object_string_bytes_with_stage(value, None)
    }

    pub(crate) fn native_object_string_bytes_using<E>(
        &self,
        value: *mut TclObj,
        check: &(impl Fn() -> Result<(), E> + ?Sized),
        refuse: &impl Fn(ValueError) -> E,
    ) -> Result<Rc<[u8]>, E> {
        crate::list::with_native_conversion_check(check, refuse, |stage, _retain| {
            self.native_object_string_bytes_with_stage(value, Some(stage))
        })
    }

    fn native_object_string_bytes_with_stage(
        &self,
        value: *mut TclObj,
        check: Option<&dyn Fn() -> Result<(), ValueError>>,
    ) -> Result<Rc<[u8]>, ValueError> {
        if let Some(check) = check {
            check()?;
        }
        self.check_entered_native_operation().map_err(|_| {
            ValueError::CommandProtocolUnavailable("entered original String getter")
        })?;
        if let Some(bytes) = self.retiring_jim_result_bytes(value)? {
            if let Some(check) = check {
                check()?;
            }
            return Ok(bytes);
        }
        self.native_string_primary_admission(value)?;
        if obj::has_string_rep(value) {
            let bytes = obj::bytes_of(value);
            if let Some(check) = check {
                check()?;
            }
            return Ok(Rc::from(bytes));
        }
        let recipe = self.native_invocation_dialect().native_string_materialization(Some(
            tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation,
        )).ok_or(ValueError::CommandProtocolUnavailable("native string materialization"))?;
        let host = self.host();
        // Only C8.4's integer updater reaches the host formatter.
        let formatter = (recipe.protocol()
            == tcl_syntax::native_string::NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4))
        .then(|| host.native_integer_formatter())
        .flatten();
        if let Some(check) = check {
            check()?;
        }
        self.check_entered_native_operation().map_err(|_| {
            ValueError::CommandProtocolUnavailable("entered original String getter")
        })?;
        let result = crate::dict::native_object_bytes_with_stage_handlers(
            value,
            recipe.protocol(),
            formatter,
            check,
        );
        if let Some(check) = check {
            check()?;
        }
        // Check the retained result even when the reached updater also failed.
        self.check_entered_native_operation().map_err(|_| {
            ValueError::CommandProtocolUnavailable("entered original String getter")
        })?;
        result.map(Rc::from)
    }
}

/// A failed integer read as the shared seam's error: the overflow and a held
/// double keep their own variants, and anything else is the non-integer `v`
/// spells.
fn value_error(v: *mut TclObj, error: &crate::typed_value::TypedError) -> ValueError {
    let text = || String::from_utf8_lossy(&obj_bytes(v)).into_owned();
    if error.is_overflow() {
        ValueError::IntegerOverflow
    } else if error.code == crate::typed_value::HELD_DOUBLE_CODE {
        ValueError::DoubleNotInteger(text())
    } else {
        ValueError::NotInteger(text())
    }
}

/// Parse an integer at the native value boundary with the selected grammar.
fn integer_operand(
    value: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
) -> Result<Number, ValueError> {
    match crate::typed_value::scalar_number(value, dialect, true)? {
        Some(integer @ (Number::Int(_) | Number::Big { .. })) => Ok(integer),
        _ => Err(ValueError::NotIntegerBytes(obj_bytes(value))),
    }
}

#[cfg(have_tommath)]
fn integer_object(value: Number) -> *mut TclObj {
    match value {
        Number::Int(value) => obj::new_wide_int_obj(value),
        Number::Big {
            negative,
            radix,
            digits,
        } => crate::bignum::from_big_digits(negative, radix, &digits),
        _ => unreachable!("integer_operand admits only integer representations"),
    }
}

/// Preserve typed Host refusals and selected arithmetic diagnostics at command adapters.
pub(crate) fn integer_error(interp: &mut Interp, error: ValueError) -> crate::interp::Code {
    if interp.host_refusal_pending() {
        return crate::interp::Code::Error;
    }
    if error.native_access_refusal().is_some() {
        return interp.report_cmd_error(error.into());
    }
    let dialect = interp.native_invocation_dialect();
    if error == ValueError::IntegerOverflow
        && dialect.arithmetic() == Some(tcl_dialect::NativeArithmetic::Tcl84Wide)
    {
        interp.error_with_code(
            error.message().as_bytes(),
            b"ARITH IOVERFLOW {integer value too large to represent}",
        )
    } else {
        let message = dialect.integer_error_presentation().map_or_else(
            || error.message().into_bytes(),
            |policy| policy.message(&error),
        );
        interp.set_error(&message)
    }
}

#[cfg(test)]
mod tests {
    use super::str_to_bytes;
    use crate::counters;
    use crate::interp::{Code, Interp};
    use tcl_dialect::TclVersion;

    #[test]
    fn original_value_append_preserves_storage_for_the_next_native_mutation() {
        // naming.list.original-value-append-storage-currency
        // docs/design/analysis/name-resolution-proofs/list-original-value-append-storage-currency.md
        use crate::obj;
        use tcl_syntax::native_string::NativeStringProtocol;
        use tcl_syntax::value::ValueOps;
        for version in [TclVersion::V9_0, TclVersion::V9_1] {
            let mut interp = Interp::new();
            interp.set_runtime_version(version);
            let head = obj::Owned::fresh(obj::new_string_bytes(b"HEAD"));
            let first = obj::Owned::fresh(obj::new_string_bytes(b"FIRST"));
            let second = obj::Owned::fresh(obj::new_string_bytes(b"SECOND"));
            let receiver = obj::Owned::fresh(crate::list::new_list_obj_native(
                &[head.as_ptr()],
                NativeStringProtocol::C(version),
            ));
            let mut original = receiver.as_ptr();
            assert!(interp
                .try_list_append_in_place(&mut original, &first.as_ptr())
                .unwrap());
            assert_eq!(original, receiver.as_ptr());
            crate::list::append_prepared_native_elements(
                original,
                &[second.as_ptr()],
                NativeStringProtocol::C(version),
            )
            .unwrap();
            assert_eq!(
                interp.list_elements(&original).unwrap(),
                vec![head.as_ptr(), first.as_ptr(), second.as_ptr()]
            );
            assert!(!interp.host_refusal_pending());
        }
    }

    #[test]
    fn original_value_append_does_not_rebuild_after_unavailable_storage() {
        // naming.list.original-value-append-storage-currency
        // docs/design/analysis/name-resolution-proofs/list-original-value-append-storage-currency.md
        use crate::obj;
        use tcl_syntax::native_string::NativeStringProtocol;
        use tcl_syntax::value::{ValueError, ValueOps};
        let mut interp = Interp::new();
        interp.set_runtime_version(TclVersion::V9_0);
        let member = obj::Owned::fresh(obj::new_string_bytes(b"MEMBER"));
        let receiver = obj::Owned::fresh(crate::list::new_list_obj_native(
            &[],
            NativeStringProtocol::C(TclVersion::V9_0),
        ));
        // The untyped mutator deliberately removes the native allocation
        // receipt. Equal list elements do not restore that storage authority.
        crate::list::list_append(receiver.as_ptr(), member.as_ptr()).unwrap();
        let mut original = receiver.as_ptr();
        assert_eq!(
            interp.try_list_append_in_place(&mut original, &member.as_ptr()),
            Err(ValueError::CommandProtocolUnavailable(
                "native List allocation extent"
            ))
        );
        assert_eq!(original, receiver.as_ptr());
        assert_eq!(
            interp.list_elements(&original).unwrap(),
            vec![member.as_ptr()]
        );
    }

    #[cfg(have_tommath)]
    #[test]
    fn native_safe_index_preparation_keeps_eager_functions_and_lazy_substitutions() {
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            for source in [
                "0 && absent(1)",
                "1 ? 0 : absent(1)",
                "0 && sqrt()",
                "1 ? 0 : sqrt(1,2)",
                "sqrt(0)",
                "$absent",
                "[error REACHED]",
                "\"$absent\"",
            ] {
                let error =
                    tcl_cmd_core::index::resolve_for_ops(&mut interp, source, 2).expect_err(source);
                assert!(
                    error.native_access_refusal().is_none(),
                    "{engine}: {source}"
                );
                if engine == "jim" {
                    assert_eq!(
                        error.into_byte_details().message,
                        format!("bad index \"{source}\": must be intexpr or end?[+-]intexpr?")
                            .as_bytes(),
                    );
                }
            }
            for source in [
                "0 && sqrt(1)",
                "1 ? 0 : $absent",
                "0 && [error REACHED]",
                "1 ? 0 : \"$absent\"",
                "int(0)",
            ] {
                let result = tcl_cmd_core::index::resolve_for_ops(&mut interp, source, 2);
                if engine == "jim" {
                    assert_eq!(result.unwrap(), 0, "{source}");
                } else {
                    let error = result.expect_err(source);
                    assert!(
                        error.native_access_refusal().is_none(),
                        "{engine}: {source}"
                    );
                }
            }
            let result = tcl_cmd_core::index::resolve_for_ops(&mut interp, "0+1", 2);
            if engine == "tcl8.4" {
                assert!(result.is_err());
            } else {
                assert_eq!(result.unwrap(), 1, "{engine}");
            }
            assert!(!interp.host_refusal_pending(), "{engine}");
        }
    }

    fn run(src: &[u8]) -> (Code, Vec<u8>) {
        run_at(TclVersion::V9_0, src)
    }

    fn run_at(version: TclVersion, src: &[u8]) -> (Code, Vec<u8>) {
        counters::reset();
        let (code, bytes);
        {
            let mut i = Interp::new();
            i.set_runtime_version(version);
            code = i.eval_str(src);
            bytes = i.result_bytes();
        }
        assert_eq!(
            counters::finalize(),
            0,
            "leak: {} objs {} bufs",
            counters::live_objs(),
            counters::live_bufs()
        );
        assert_eq!(counters::double_free_count(), 0);
        (code, bytes)
    }
    fn ok(src: &[u8]) -> Vec<u8> {
        let (c, b) = run(src);
        assert_eq!(c, Code::Ok, "result={:?}", String::from_utf8_lossy(&b));
        b
    }

    #[test]
    fn physical_list_length_matches_all_204_native_storage_rows() {
        // Native proof: naming.list.stock-length-same-original-storage
        // docs/design/analysis/name-resolution-proofs/list.stock-length-same-original-storage.md

        use crate::obj;
        use tcl_registry::native_stock_list::NativeStockListInputClass as Class;
        use tcl_syntax::value::ValueOps;
        const FIXTURES: [&str; 6] = [
            include_str!(
                "../../../rust/tcl-syntax/tests/data/native_list_methods/stock_length/8.4.20.txt"
            ),
            include_str!(
                "../../../rust/tcl-syntax/tests/data/native_list_methods/stock_length/8.5.19.txt"
            ),
            include_str!(
                "../../../rust/tcl-syntax/tests/data/native_list_methods/stock_length/8.6.18.txt"
            ),
            include_str!(
                "../../../rust/tcl-syntax/tests/data/native_list_methods/stock_length/9.0.4.txt"
            ),
            include_str!(
                "../../../rust/tcl-syntax/tests/data/native_list_methods/stock_length/9.1.0.txt"
            ),
            include_str!(
                "../../../rust/tcl-syntax/tests/data/native_list_methods/stock_length/jim.txt"
            ),
        ];
        fn field<'a>(row: &'a str, key: &str) -> &'a str {
            row.split_whitespace()
                .find_map(|item| {
                    item.split_once('=')
                        .filter(|(name, _)| *name == key)
                        .map(|(_, value)| value)
                })
                .expect("measured native field")
        }
        fn class(name: &str) -> Class {
            match name {
                "string" => Class::String,
                "list" => Class::List,
                "dict" => Class::Dictionary,
                "bytearray" => Class::ByteArray,
                "boolean" | "booleanString" => Class::Boolean,
                "int" | "wideInt" | "double" => Class::Numeric,
                _ => panic!("unknown measured class {name}"),
            }
        }
        let mut rows = 0;
        for (index, fixture) in FIXTURES.iter().enumerate() {
            let mut interp = Interp::new();
            if let Some(version) = TclVersion::ALL.get(index) {
                interp.set_runtime_version(*version);
            } else {
                interp.set_dialect_profile(crate::environment::profile_for_dialect("jim"));
            }
            let dialect = interp.native_invocation_dialect();
            for row in fixture.lines() {
                let mode = field(row, "mode").parse::<u8>().unwrap();
                let before = field(row, "before");
                let string: &[u8] = match mode {
                    0 | 7 | 9 | 11 | 13 => b"",
                    2 => b"{",
                    1 | 3 | 5 => b"0",
                    12 => b"2 3",
                    16 => b"true",
                    _ => b"2",
                };
                let value = match class(before) {
                    Class::String => {
                        obj::new_string_bytes(if index == 0 && matches!(mode, 11..=13) {
                            b""
                        } else {
                            string
                        })
                    }
                    Class::List => {
                        let elements = if mode == 8 {
                            vec![obj::new_string_bytes(b"2")]
                        } else {
                            Vec::new()
                        };
                        crate::list::new_list_obj(&elements)
                    }
                    Class::Dictionary => {
                        let pairs = if mode == 12 {
                            vec![(obj::new_string_bytes(b"2"), obj::new_string_bytes(b"3"))]
                        } else {
                            Vec::new()
                        };
                        crate::dict::new_dict_obj(&pairs)
                    }
                    Class::ByteArray => crate::bytearray::new_byte_array(
                        if mode == 9 { b"" } else { b"2" },
                        dialect.byte_array_string_recipe(None).unwrap(),
                    ),
                    Class::Boolean => {
                        let value = obj::new_string_bytes(string);
                        crate::typed_value::native_boolean(value, dialect).unwrap();
                        value
                    }
                    Class::Numeric => {
                        if before == "double" {
                            obj::new_double_obj(if mode == 5 { 0.0 } else { 2.0 })
                        } else {
                            obj::new_wide_int_obj(if mode == 3 {
                                0
                            } else if mode == 16 {
                                1
                            } else {
                                2
                            })
                        }
                    }
                    Class::Unknown
                    | Class::LegacyExtension
                    | Class::PropertyName
                    | Class::MethodName
                    | Class::InstructionName
                    | Class::CommandName
                    | Class::JimLookup
                    | Class::JimSource
                    | Class::ArraySearch
                    | Class::NamespaceName
                    | Class::ParsedVariableName
                    | Class::LocalVariableName
                    | Class::Index => unreachable!("original constructor inventory"),
                };
                let value = obj::Owned::fresh(value);
                if field(row, "beforestring") == "1" && !obj::has_string_rep(value.as_ptr()) {
                    unsafe { obj::set_string_rep(value.as_ptr(), string) };
                }
                assert_eq!(
                    obj::stock_list_input_class(value.as_ptr()),
                    class(before),
                    "{row}"
                );
                assert_eq!(
                    obj::has_string_rep(value.as_ptr()),
                    field(row, "beforestring") == "1",
                    "{row}"
                );
                let result = interp.list_len(&value.as_ptr());
                assert_eq!(
                    result.is_ok(),
                    field(row, "code") == "0",
                    "engine {index}: {row}: {result:?}"
                );
                if let Ok(length) = result {
                    let result_bytes: Vec<_> = field(row, "result")
                        .as_bytes()
                        .chunks_exact(2)
                        .map(|pair| {
                            u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap()
                        })
                        .collect();
                    assert_eq!(length.to_string().as_bytes(), result_bytes, "{row}");
                }
                assert_eq!(
                    obj::stock_list_input_class(value.as_ptr()),
                    class(field(row, "after")),
                    "engine {index}: {row}"
                );
                assert_eq!(
                    obj::has_string_rep(value.as_ptr()),
                    field(row, "afterstring") == "1",
                    "engine {index}: {row}"
                );
                rows += 1;
            }
        }
        assert_eq!(rows, 204);
    }

    #[test]
    fn native_length_preserves_nan_payload_and_dictionary_member_identity() {
        use crate::obj;
        use tcl_syntax::value::ValueOps;
        let mut interp = Interp::new();
        interp.set_runtime_version(TclVersion::V9_0);
        let bits = 0x7ff8_0000_0000_0023;
        let nan = obj::Owned::fresh(obj::new_double_obj(f64::from_bits(bits)));
        assert_eq!(interp.list_len(&nan.as_ptr()).unwrap(), 1);
        assert!(!obj::has_string_rep(nan.as_ptr()));
        assert_eq!(obj::double_of(nan.as_ptr()).to_bits(), bits);
        let key = obj::Owned::fresh(obj::new_string_bytes(b"k\0\xc0\x80\xff"));
        let member = obj::Owned::fresh(obj::new_wide_int_obj(23));
        let dictionary = obj::Owned::fresh(crate::dict::new_dict_obj(&[(
            key.as_ptr(),
            member.as_ptr(),
        )]));
        assert_eq!(interp.list_len(&dictionary.as_ptr()).unwrap(), 2);
        assert!(!obj::has_string_rep(dictionary.as_ptr()));
        assert!(crate::list::is_pure_list(dictionary.as_ptr()));
        let elements = interp.list_elements(&dictionary.as_ptr()).unwrap();
        assert_eq!(elements, vec![key.as_ptr(), member.as_ptr()]);
        assert!(!obj::has_string_rep(member.as_ptr()));
    }

    #[test]
    fn resident_dictionary_spelling_preserves_duplicate_keys_for_list_conversion() {
        use crate::obj;
        use tcl_syntax::value::ValueOps;
        for environment in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let mut interp = Interp::new();
            interp.set_dialect_profile(crate::environment::profile_for_dialect(environment));
            let value = obj::Owned::fresh(obj::new_string_bytes(b"a 1 a 2"));
            assert_eq!(interp.dict_pairs(&value.as_ptr()).unwrap().len(), 1);
            assert_eq!(
                obj::obj_type_ptr(value.as_ptr()),
                &crate::dict::TCL_DICT_TYPE as *const _
            );
            assert_eq!(
                interp.list_len(&value.as_ptr()).unwrap(),
                4,
                "{environment}"
            );
            assert_eq!(obj::bytes_of(value.as_ptr()), b"a 1 a 2");
            let elements = interp.list_elements(&value.as_ptr()).unwrap();
            assert_eq!(
                elements
                    .iter()
                    .map(|&item| obj::bytes_of(item))
                    .collect::<Vec<_>>(),
                [b"a".to_vec(), b"1".to_vec(), b"a".to_vec(), b"2".to_vec()]
            );
        }
    }

    #[cfg(have_tommath)]
    #[test]
    fn tcl9_length_preserves_complete_bignum_storage_without_materializing() {
        use crate::obj;
        use tcl_syntax::value::ValueOps;
        let value = obj::Owned::fresh(crate::bignum::from_big_digits(
            false,
            tcl_syntax::number::Radix::Dec,
            "18446744073709551617000000000000000000000000000000000000003",
        ));
        let before = obj::native_scalar_cache(value.as_ptr()).unwrap();
        for version in [TclVersion::V9_0, TclVersion::V9_1] {
            let mut interp = Interp::new();
            interp.set_runtime_version(version);
            assert_eq!(interp.list_len(&value.as_ptr()).unwrap(), 1);
            assert!(!obj::has_string_rep(value.as_ptr()));
            assert_eq!(obj::native_scalar_cache(value.as_ptr()).unwrap(), before);
        }
    }

    // bytes_to_str / str_to_bytes unit coverage

    /// Original invalid bytes use the checked selected Native Unicode entry.
    #[test]
    fn bytes_to_str_falls_back_on_invalid_utf8() {
        use crate::obj;
        use tcl_syntax::value::ValueOps;
        for environment in ["tcl8.6", "tcl9.0"] {
            let mut interpreter = Interp::with_native_core(
                crate::interp::default_host(),
                tcl_registry::model::resolve_environment(environment).unit_profile(),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let value = obj::Owned::fresh(obj::new_string_bytes(&[0x41, 0xFF, 0x42]));
            let s = ValueOps::checked_character_view(&mut interpreter, &value.as_ptr()).unwrap();
            assert_eq!(s.chars().count(), 3);
            assert_eq!(s.chars().nth(1), Some('\u{00ff}'));
        }
    }

    /// Raw input cannot enter a Unicode-only operation, and retains exact bytes.
    #[test]
    fn unicode_door_refuses_invalid_utf8_without_replacing_bytes() {
        let value = tcl_syntax::raw_string::RawString::from_bytes(&b"A\xffB"[..]);
        assert_eq!(value.unicode().unwrap_err().valid_up_to, 1);
        assert_eq!(&*value.bytes(), b"A\xffB");
    }

    /// TN: genuine valid UTF-8 (including non-ASCII Latin-1-supplement text)
    /// remains its real characters, preserving Unicode string operations.
    #[test]
    fn bytes_to_str_preserves_valid_utf8() {
        let s = tcl_syntax::raw_string::RawString::from_unicode("café")
            .unicode()
            .unwrap();
        assert_eq!(&*s, "café");
        assert_eq!(s.chars().count(), 4);
        assert_eq!(s.chars().nth(3), Some('\u{00e9}'));
    }

    /// String construction remains ordinary UTF-8; byte conversion is deferred
    /// to `Interp::binary_bytes`, where it can consult the Tcl version.
    #[test]
    fn str_to_bytes_keeps_unicode_string_representation() {
        assert_eq!(str_to_bytes("A\u{00ff}B"), "A\u{00ff}B".as_bytes());
    }

    #[test]
    fn jim_raw_string_operations_preserve_native_extents_and_cached_count() {
        counters::reset();
        {
            let mut interp = Interp::new();
            interp.set_dialect_profile(crate::environment::profile_for_dialect("jim"));
            let source = crate::obj::new_string_bytes(b"\xffA\xed\xa0\x80\xc3\xa9");
            interp
                .var_set(b"s", source)
                .expect("plain source slot accepts the value");
            for (script, expected) in [
                (b"string length $s".as_slice(), b"4".as_slice()),
                (b"string index $s 0", b"\xff"),
                (b"string index $s 2", b"\xed\xa0\x80"),
                (b"string range $s 1 end", b"A\xed\xa0\x80\xc3\xa9"),
                (b"string reverse $s", b"\xc3\xa9\xed\xa0\x80A\xff"),
                (
                    b"string repeat $s 2",
                    b"\xffA\xed\xa0\x80\xc3\xa9\xffA\xed\xa0\x80\xc3\xa9",
                ),
                (b"string index $s -1", b""),
                (b"string range $s 2 1", b""),
            ] {
                assert_eq!(interp.eval_str(script), Code::Ok, "script={script:?}");
                assert_eq!(interp.result_bytes(), expected, "script={script:?}");
                assert!(!interp.host_refusal_pending());
            }
            let source = crate::obj::new_string_bytes(b"\xc3A\xc3\xa9");
            interp
                .var_set(b"s", source)
                .expect("plain source slot accepts the value");
            assert_eq!(interp.eval_str(b"set r [string range $s 0 1]"), Code::Ok);
            assert_eq!(interp.result_bytes(), b"\xc3A\xc3\xa9");
            assert_eq!(interp.eval_str(b"string length $r"), Code::Ok);
            assert_eq!(interp.result_bytes(), b"2");
            // A reconstructed string has its own native byte-count recipe.
            assert_eq!(interp.eval_str(b"string length \"$r\""), Code::Ok);
            assert_eq!(interp.result_bytes(), b"3");
            assert_eq!(interp.eval_str(b"set r [string range $s 1 end]"), Code::Ok);
            assert_eq!(interp.result_bytes(), b"\xc3\xa9\0");
            assert_eq!(interp.eval_str(b"string length $r"), Code::Ok);
            assert_eq!(interp.result_bytes(), b"2");
            // Dictionary/list coercion withdraws a previous native string count.
            assert_eq!(interp.eval_str(b"llength $r"), Code::Ok);
            assert_eq!(
                crate::obj::jim_string_count(interp.var_get(b"r").unwrap()),
                None
            );
        }
        assert_eq!(
            counters::finalize(),
            0,
            "objs={} bufs={}",
            counters::live_objs(),
            counters::live_bufs()
        );
        assert_eq!(counters::double_free_count(), 0);
    }

    /// FP guard: genuine Latin-1-supplement text (codepoints `<= 0xFF` but
    /// *not* escaped, e.g. 'é') round-trips as real UTF-8, not a raw byte —
    /// the exact case a naive "codepoint fits a byte" heuristic gets wrong.
    #[test]
    fn str_to_bytes_keeps_utf8_for_real_latin1_text() {
        assert_eq!(str_to_bytes("café"), "café".as_bytes());
    }

    /// FP guard: a string with a genuine wide character (e.g. CJK) is encoded
    /// as real UTF-8, not corrupted.
    #[test]
    fn str_to_bytes_keeps_utf8_for_wide_chars() {
        assert_eq!(str_to_bytes("a\u{65e5}b"), "a\u{65e5}b".as_bytes().to_vec());
    }

    // end-to-end: byte-array dual ports, driven through `string`/`binary`

    /// TP: `string index`/`range`/`replace`/`length` on a `binary format` value
    /// preserve binary bytes exactly in both C Tcl 8.6 and 9.0.
    #[test]
    fn string_subcommands_preserve_binary_bytes() {
        let script = br#"
            set b [binary format H* 41ff42]
            list [string length $b] \
                 [binary encode hex [string index $b 1]] \
                 [binary encode hex [string range $b 0 2]] \
                 [binary encode hex [string replace $b 0 0 X]]
        "#;
        assert_eq!(ok(script), b"3 ff 41ff42 58ff42");
    }

    /// TP: every byte-producing command constructs the same typed byte-array
    /// representation, so a later string read followed by `binary encode` sees
    /// the original payload. This covers decode, scan assignment, and zlib's
    /// decompression result rather than only `binary format`.
    #[test]
    fn byte_producing_commands_share_the_dual_port_representation() {
        let script = br#"
            set decoded [binary decode hex 41ff42]
            binary scan [binary format H* 41ff42] a* scanned
            set inflated [zlib decompress [zlib compress [binary format H* 41ff42]]]
            list \
                [binary encode hex [string range $decoded 0 2]] \
                [binary encode hex [string range $scanned 0 2]] \
                [binary encode hex [string range $inflated 0 2]]
        "#;
        assert_eq!(ok(script), b"41ff42 41ff42 41ff42");
    }

    /// TP/FN: a byte-array string shimmer is version-independent, but turning
    /// a changed Unicode string back into bytes is release-defined. These
    /// values are pinned to C Tcl 8.6.18 and 9.0.4.
    #[test]
    fn string_case_mapping_of_a_byte_array_uses_the_tcl_release_byte_policy() {
        let script = br#"binary encode hex [string toupper [binary format H* 41ff42]]"#;
        let (old_code, old_result) = run_at(TclVersion::V8_6, script);
        assert_eq!(old_code, Code::Ok);
        assert_eq!(old_result, b"417842");

        let (modern_code, modern_result) = run_at(TclVersion::V9_0, script);
        assert_eq!(modern_code, Code::Error);
        assert_eq!(
            modern_result,
            b"expected code point values below 0xff but value at byte offset 1 was 0x178"
        );
    }

    /// FN: a byte array has a real Unicode string representation, so its case
    /// mapping is not silently a no-op. Tcl 9 rejects the final conversion to
    /// bytes because `Ÿ` is outside the checked byte domain.
    #[test]
    fn string_toupper_on_binary_uses_the_checked_tcl9_byte_conversion() {
        let (code, message) =
            run(br#"binary encode hex [string toupper [binary format H* 41ff42]]"#);
        assert_eq!(code, Code::Error);
        assert_eq!(
            message,
            b"expected code point values below 0xff but value at byte offset 1 was 0x178"
        );
    }

    /// TN: plain ASCII through the same subcommands is unaffected.
    #[test]
    fn string_subcommands_ascii_unaffected() {
        assert_eq!(ok(b"string range hello 1 3"), b"ell");
        assert_eq!(ok(b"string toupper hello"), b"HELLO");
        assert_eq!(ok(b"string index hello 0"), b"h");
    }

    /// TN: genuine multi-byte Unicode text is still handled by character, not
    /// by byte, through the same subcommands the binary fix touches.
    #[test]
    fn string_subcommands_wide_unicode_unaffected() {
        // U+65E5 U+672C U+8A9E ("nihongo" in kanji) — 3 characters, 9 bytes.
        let script = "list [string length \u{65e5}\u{672c}\u{8a9e}] [string index \u{65e5}\u{672c}\u{8a9e} 1]";
        assert_eq!(ok(script.as_bytes()), "3 \u{672c}".as_bytes());
    }
}

#[cfg(test)]
mod legacy_extension_length_tests;
