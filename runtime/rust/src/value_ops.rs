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
use tcl_syntax::value::{ValueError, ValueOps};

use crate::interp::{obj_bytes, Interp};
use crate::list;
use crate::obj::{self, TclObj};

#[cfg(have_tommath)]
fn index_expression_error(source: &str, error: &crate::expr_error::ExprError) -> ValueError {
    error.native_access_refusal.map_or_else(
        || ValueError::NotInteger(source.to_owned()),
        ValueError::from,
    )
}

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
            tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(cause) => {
                cause.clone().into()
            }
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

/// Actual C8.4/Jim original-object increment, with native probe/COW order.
pub(crate) struct RuntimeLegacyIncrementObjects {
    dialect: tcl_registry::InvocationDialect,
    recipe: tcl_syntax::scalar_getter::NativeLegacyIncrementRecipe,
    jim_context: Option<Rc<crate::native_source::NativeJimObjectContext>>,
    context: IncrementContext,
}
impl RuntimeLegacyIncrementObjects {
    pub(crate) fn selected(interp: &Interp) -> Result<Self, tcl_cmd_core::CmdError> {
        let context = IncrementContext::selected(interp)?;
        let dialect = interp.native_invocation_dialect();
        let recipe = dialect
            .native_legacy_increment_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native legacy increment",
            ))?
            .recipe();
        let jim_context = if dialect.native_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            Some(interp.native_jim_object_context()?)
        } else {
            None
        };
        context.check()?;
        Ok(Self {
            dialect,
            recipe,
            context,
            jim_context,
        })
    }
}
impl tcl_cmd_core::native_increment::LegacyIncrementObjects for RuntimeLegacyIncrementObjects {
    type Value = RuntimeAppendValue;
    type Prepared = RuntimeAppendValue;
    fn recipe(&self) -> tcl_syntax::scalar_getter::NativeLegacyIncrementRecipe {
        self.recipe
    }
    fn cache(
        &self,
        value: &Self::Value,
    ) -> Result<Option<tcl_syntax::scalar_getter::NativeScalarCache>, tcl_cmd_core::CmdError> {
        self.context.check()?;
        self.context.after(obj::native_scalar_cache(value.as_ptr()))
    }
    fn wide(&self, value: &Self::Value) -> Result<i64, tcl_cmd_core::CmdError> {
        self.context.check()?;
        if let Some(context) = &self.jim_context {
            self.context
                .after(crate::native_source::bind_context(value.as_ptr(), context))?;
        }
        match self.context.scalar(
            value.as_ptr(),
            self.dialect,
            tcl_syntax::scalar_getter::NativeScalarGetterKind::Wide,
        )? {
            tcl_syntax::scalar_getter::NativeScalarGetterValue::Wide(value) => Ok(value),
            _ => Err(ValueError::ScalarNumericInputUnavailable.into()),
        }
    }
    fn prepare(
        &self,
        original: Option<&Self::Value>,
    ) -> Result<Self::Prepared, tcl_cmd_core::CmdError> {
        self.context.prepare(original)
    }
    fn current<'a>(&self, prepared: &'a Self::Prepared) -> &'a Self::Value {
        prepared
    }
    fn store(
        &self,
        prepared: &mut Self::Prepared,
        cache: tcl_syntax::scalar_getter::NativeScalarCache,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        self.context.check()?;
        let valid = match self.recipe {
            tcl_syntax::scalar_getter::NativeLegacyIncrementRecipe::Tcl84 => matches!(
                cache,
                tcl_syntax::scalar_getter::NativeScalarCache::Tcl84Long(_)
                    | tcl_syntax::scalar_getter::NativeScalarCache::Number(Number::Int(_))
            ),
            tcl_syntax::scalar_getter::NativeLegacyIncrementRecipe::Jim084 => matches!(
                cache,
                tcl_syntax::scalar_getter::NativeScalarCache::Number(Number::Int(_))
            ),
        };
        if !valid {
            return Err(ValueError::ScalarNumericInputUnavailable.into());
        }
        self.context.store(prepared.as_ptr(), cache, self.dialect)
    }
    fn finish(&self, prepared: Self::Prepared) -> Self::Value {
        prepared
    }
}

pub(crate) struct RuntimeLegacyIncrementAmountOps<'a>(pub(crate) &'a mut Interp);
impl tcl_cmd_core::native_increment::LegacyIncrementAmountOps
    for RuntimeLegacyIncrementAmountOps<'_>
{
    type Value = *mut TclObj;
    fn c84_long(&mut self, original: &Self::Value) -> Result<i64, tcl_cmd_core::CmdError> {
        let context = IncrementContext::selected(self.0)?;
        match context.scalar(
            *original,
            self.0.native_invocation_dialect(),
            tcl_syntax::scalar_getter::NativeScalarGetterKind::Long,
        )? {
            tcl_syntax::scalar_getter::NativeScalarGetterValue::Wide(value) => Ok(value),
            _ => Err(ValueError::ScalarNumericInputUnavailable.into()),
        }
    }
    fn jim_wide_expression(
        &mut self,
        original: &Self::Value,
    ) -> Result<i64, tcl_cmd_core::CmdError> {
        let context = IncrementContext::selected(self.0)?;
        let result = crate::builtins::native_jim_wide_expression(self.0, *original);
        context.check()?;
        result
    }
}

/// Selected modern C integer update operations on original physical objects.
pub(crate) struct RuntimeIncrementObjects {
    dialect: tcl_registry::InvocationDialect,
    context: IncrementContext,
}
impl RuntimeIncrementObjects {
    pub(crate) fn selected(interp: &Interp) -> Result<Self, tcl_cmd_core::CmdError> {
        let context = IncrementContext::selected(interp)?;
        let dialect = interp.native_invocation_dialect();
        dialect
            .native_scalar_getter_protocol()
            .filter(|protocol| protocol.supports_number_getter())
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        Ok(Self { dialect, context })
    }
}
impl tcl_cmd_core::native_increment::NativeIncrementObjects for RuntimeIncrementObjects {
    type Value = RuntimeAppendValue;
    type Prepared = RuntimeAppendValue;
    fn prepare(
        &self,
        original: Option<&Self::Value>,
    ) -> Result<Self::Prepared, tcl_cmd_core::CmdError> {
        self.context.prepare(original)
    }
    fn retain_increment_failure(&self, first: &tcl_cmd_core::CmdError) {
        self.context.retain(first.clone());
    }
    fn current<'a>(&self, prepared: &'a Self::Prepared) -> &'a Self::Value {
        prepared
    }
    fn probe(
        &self,
        value: &Self::Value,
        kind: tcl_syntax::scalar_getter::NativeNumberGetterKind,
    ) -> Result<
        Result<Number, tcl_syntax::scalar_getter::NativeScalarGetterFailure>,
        tcl_cmd_core::CmdError,
    > {
        self.context.check()?;
        let stage = || self.context.stage(value.as_ptr());
        self.context
            .after(crate::typed_value::native_number_probe_with_stage_check(
                value.as_ptr(),
                self.dialect,
                kind,
                Some(&self.context.currency),
                self.context
                    .original
                    .as_ref()
                    .map(|_| &stage as &dyn Fn() -> Result<(), ValueError>),
            ))
    }
    fn integer_failure(
        &self,
        value: &Self::Value,
        kind: tcl_syntax::scalar_getter::NativeNumberGetterKind,
    ) -> Result<tcl_cmd_core::CmdError, tcl_cmd_core::CmdError> {
        let getter = if kind == tcl_syntax::scalar_getter::NativeNumberGetterKind::Bignum {
            tcl_syntax::scalar_getter::NativeScalarGetterKind::Wide
        } else {
            tcl_syntax::scalar_getter::NativeScalarGetterKind::Int
        };
        match self.context.scalar(value.as_ptr(), self.dialect, getter) {
            Err(error) => Ok(error),
            Ok(_) => Err(ValueError::CommandProtocolUnavailable(
                "native increment integer failure stage",
            )
            .into()),
        }
    }
    fn add(&self, current: Number, amount: Number) -> Result<Number, tcl_cmd_core::CmdError> {
        self.context.check()?;
        let result = tcl_cmd_core::native_increment::add_integer_numbers(
            current,
            amount,
            |current, amount| {
                #[cfg(have_tommath)]
                {
                    let current = obj::Owned::fresh(integer_object(current));
                    let amount = obj::Owned::fresh(integer_object(amount));
                    let result = crate::bignum::add(current.as_ptr(), amount.as_ptr())
                        .map_err(|_| ValueError::ScalarNumericInputUnavailable)?;
                    let result = obj::Owned::fresh(result);
                    match obj::native_scalar_cache(result.as_ptr())? {
                        Some(tcl_syntax::scalar_getter::NativeScalarCache::Number(number)) => {
                            Ok(number)
                        }
                        _ => Err(ValueError::ScalarNumericInputUnavailable.into()),
                    }
                }
                #[cfg(not(have_tommath))]
                {
                    let _ = (current, amount);
                    Err(ValueError::ScalarNumericInputUnavailable.into())
                }
            },
        );
        self.context.check()?;
        result
    }
    fn store(
        &self,
        prepared: &mut Self::Prepared,
        sum: Number,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        self.context.store(
            prepared.as_ptr(),
            tcl_syntax::scalar_getter::NativeScalarCache::Number(sum),
            self.dialect,
        )
    }
    fn finish(&self, prepared: Self::Prepared) -> Self::Value {
        prepared
    }
}

/// Physical dictionary operations over borrowed original native objects.
pub(crate) struct RuntimeDictionaryObjects {
    pub(crate) protocol: tcl_syntax::native_string::NativeStringProtocol,
    names: tcl_syntax::naming::NamePolicyProtocol,
    preparation: tcl_cmd_core::native_dictionary::NativeDictionaryPreparation,
    original: Option<std::rc::Rc<RuntimeDictionaryChecks>>,
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
impl RuntimeAppendObjects {
    /// The actual append caller supplies its original receiver/frame checks;
    /// this uses the existing shared String/current/error owner without issuing
    /// a dictionary, variable, operation or Native reference role.
    pub(crate) fn with_original_checks(
        self,
        current: impl Fn() -> Result<(), tcl_cmd_core::CmdError> + 'static,
        string: impl Fn(
                *mut TclObj,
                &dyn Fn() -> Result<(), tcl_cmd_core::CmdError>,
            ) -> Result<std::rc::Rc<[u8]>, tcl_cmd_core::CmdError>
            + 'static,
        refuse: impl Fn(ValueError) -> tcl_cmd_core::CmdError + 'static,
        retain: impl Fn(tcl_cmd_core::CmdError) -> tcl_cmd_core::CmdError + 'static,
    ) -> OriginalRuntimeAppendObjects {
        OriginalRuntimeAppendObjects::new(
            self,
            RuntimeDictionaryChecks::new(current, string, refuse, retain),
        )
    }
}
impl RuntimeDictionaryObjects {
    pub(crate) fn selected(interp: &Interp) -> Result<Self, tcl_cmd_core::CmdError> {
        let protocol = interp.native_invocation_dialect().native_string_materialization(
            Some(tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation),
        ).ok_or(ValueError::CommandProtocolUnavailable("native dictionary object protocol"))?.protocol();
        let names = interp
            .name_policy_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native dictionary diagnostic protocol",
            ))?;
        Ok(Self {
            protocol,
            names,
            preparation:
                tcl_cmd_core::native_dictionary::NativeDictionaryPreparation::CopyBeforeConversion,
            original: None,
        })
    }
    /// The caller's actual entered operation/receiver and original String
    /// projection. This adds no owning Native role or conversion purpose.
    pub(crate) fn with_original_checks(
        mut self,
        current: impl Fn() -> Result<(), tcl_cmd_core::CmdError> + 'static,
        string: impl Fn(
                *mut TclObj,
                &dyn Fn() -> Result<(), tcl_cmd_core::CmdError>,
            ) -> Result<std::rc::Rc<[u8]>, tcl_cmd_core::CmdError>
            + 'static,
        refuse: impl Fn(ValueError) -> tcl_cmd_core::CmdError + 'static,
        retain: impl Fn(tcl_cmd_core::CmdError) -> tcl_cmd_core::CmdError + 'static,
    ) -> Self {
        self.original = Some(RuntimeDictionaryChecks::new(
            current, string, refuse, retain,
        ));
        self
    }
    pub(crate) fn original_check(&self) -> Result<(), tcl_cmd_core::CmdError> {
        self.original
            .as_ref()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original Dictionary/Return owner",
            ))?
            .check()
    }
    pub(crate) fn original_refuse(&self, error: ValueError) -> tcl_cmd_core::CmdError {
        match self.original.as_ref() {
            Some(owner) => (owner.refuse)(error),
            None => error.into(),
        }
    }
    pub(crate) fn original_bytes(
        &self,
        value: *mut TclObj,
    ) -> Result<std::rc::Rc<[u8]>, tcl_cmd_core::CmdError> {
        self.original
            .as_ref()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original Dictionary/Return owner",
            ))?
            .bytes(value)
    }
    /// The same original String worker, with an independently captured
    /// return-options/private-field backing check inside reached callbacks.
    pub(crate) fn original_bytes_using(
        &self,
        value: *mut TclObj,
        extra: &dyn Fn() -> Result<(), tcl_cmd_core::CmdError>,
    ) -> Result<std::rc::Rc<[u8]>, tcl_cmd_core::CmdError> {
        self.original
            .as_ref()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original Dictionary/Return owner",
            ))?
            .bytes_using(value, extra)
    }
    pub(crate) fn original_retain(&self, error: tcl_cmd_core::CmdError) -> tcl_cmd_core::CmdError {
        match self.original.as_ref() {
            Some(owner) => (owner.retain)(error),
            None => error,
        }
    }
    pub(crate) fn original_list(
        &self,
        value: *mut TclObj,
    ) -> Result<Vec<*mut TclObj>, tcl_cmd_core::CmdError> {
        crate::list::list_elements_native_using(
            value,
            self.protocol,
            &|| self.original_check(),
            &|error| self.original_refuse(error),
        )
    }
    pub(crate) fn original_pairs(
        &self,
        value: *mut TclObj,
    ) -> Result<Vec<(*mut TclObj, *mut TclObj)>, tcl_cmd_core::CmdError> {
        crate::dict::native_dict_pairs_using(
            value,
            self.protocol,
            &|| self.original_check(),
            &|error| self.original_refuse(error),
        )
    }
    /// Reuse this genuine original dictionary/frame/check owner for the same
    /// physical append/Cat backend. No scope or Native role is manufactured.
    pub(crate) fn original_append_objects(
        &self,
        base: RuntimeAppendObjects,
    ) -> Result<OriginalRuntimeAppendObjects, tcl_cmd_core::CmdError> {
        self.original_check()?;
        Ok(OriginalRuntimeAppendObjects::new(
            base,
            self.original
                .as_ref()
                .expect("checked original dictionary owner")
                .clone(),
        ))
    }
    pub(crate) fn original_increment_objects(
        &self,
        actual: &Interp,
    ) -> Result<RuntimeIncrementObjects, tcl_cmd_core::CmdError> {
        self.original_check()?;
        let mut increment = RuntimeIncrementObjects::selected(actual)?;
        increment.context.original = Some(
            self.original
                .as_ref()
                .expect("checked original dictionary owner")
                .clone(),
        );
        increment.context.check()?;
        Ok(increment)
    }
    pub(crate) fn with_preparation(
        mut self,
        preparation: tcl_cmd_core::native_dictionary::NativeDictionaryPreparation,
    ) -> Self {
        self.preparation = preparation;
        self
    }
}
impl tcl_cmd_core::native_dictionary::NativeDictionaryObjects for RuntimeDictionaryObjects {
    type Value = RuntimeAppendValue;
    type Prepared = crate::dict::PreparedNativeDictionary;
    fn prepare(
        &self,
        original: Option<&Self::Value>,
    ) -> Result<Self::Prepared, tcl_cmd_core::CmdError> {
        use tcl_cmd_core::native_dictionary::NativeDictionaryPreparation as Preparation;
        let original = original.map(RuntimeAppendValue::as_ptr);
        if let Some(checked) = &self.original {
            return Self::Prepared::prepare_selected_using(
                original,
                self.protocol,
                self.preparation,
                &|| checked.check(),
                &|error| (checked.refuse)(error),
            );
        }
        match self.preparation {
            Preparation::CopyBeforeConversion => Self::Prepared::prepare(original, self.protocol),
            Preparation::ConvertBeforeCopy => {
                Self::Prepared::prepare_after_conversion(original, self.protocol)
            }
            Preparation::IncrementCommand => {
                Self::Prepared::prepare_for_increment(original, self.protocol)
            }
        }
        .map_err(Into::into)
    }
    fn prepare_child(
        &self,
        original: Option<&Self::Value>,
    ) -> Result<Self::Prepared, tcl_cmd_core::CmdError> {
        if let Some(checked) = &self.original {
            return Self::Prepared::prepare_selected_using(
                original.map(RuntimeAppendValue::as_ptr),
                self.protocol,
                tcl_cmd_core::native_dictionary::NativeDictionaryPreparation::ConvertBeforeCopy,
                &|| checked.check(),
                &|error| (checked.refuse)(error),
            );
        }
        crate::dict::PreparedNativeDictionary::prepare_after_conversion(
            original.map(RuntimeAppendValue::as_ptr),
            self.protocol,
        )
        .map_err(Into::into)
    }
    fn with_member<R>(
        &self,
        dictionary: &Self::Prepared,
        key: &Self::Value,
        operation: impl FnOnce(Option<&Self::Value>) -> Result<R, tcl_cmd_core::CmdError>,
    ) -> Result<R, tcl_cmd_core::CmdError> {
        if let Some(checked) = &self.original {
            let observer = dictionary
                .observe_prepared()
                .map_err(|error| (checked.refuse)(error))?;
            checked.parents.borrow_mut().push(observer);
            let reached = dictionary.with_member_using(
                key.as_ptr(),
                &|| checked.check(),
                &|error| (checked.refuse)(error),
                &|value, current| checked.bytes_using(value, current),
                |member| {
                    let member = member.map(RuntimeAppendValue::borrowed);
                    operation(member.as_ref())
                },
            );
            // Extract the nonowning guard before releasing it. No callback
            // occurs under the parent-observation RefCell borrow.
            let observer = checked.parents.borrow_mut().pop();
            drop(observer);
            checked.check()?;
            return reached;
        }
        dictionary.with_member(key.as_ptr(), |member| {
            let member = member.map(RuntimeAppendValue::borrowed);
            operation(member.as_ref())
        })?
    }
    fn set_member(
        &self,
        dictionary: &mut Self::Prepared,
        key: &Self::Value,
        value: Self::Value,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        if let Some(checked) = &self.original {
            let reached = dictionary.set_member_using(
                key.as_ptr(),
                value.as_ptr(),
                &|| checked.check(),
                &|error| (checked.refuse)(error),
                &|original, current| checked.bytes_using(original, current),
            );
            drop(value);
            checked.check()?;
            return reached;
        }
        dictionary
            .set_member(key.as_ptr(), value.as_ptr())
            .map_err(Into::into)
    }
    fn remove_member(
        &self,
        dictionary: &mut Self::Prepared,
        key: &Self::Value,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        if let Some(checked) = &self.original {
            return dictionary
                .remove_member_using(
                    key.as_ptr(),
                    &|| checked.check(),
                    &|error| (checked.refuse)(error),
                    &|original, current| checked.bytes_using(original, current),
                )
                .map(|_| ());
        }
        dictionary
            .remove_member(key.as_ptr())
            .map(|_| ())
            .map_err(Into::into)
    }
    fn missing_key(&self, key: &Self::Value) -> tcl_cmd_core::CmdError {
        let bytes = if let Some(checked) = &self.original {
            checked
                .check()
                .and_then(|()| checked.bytes(key.as_ptr()))
                .map(|bytes| bytes.to_vec())
        } else {
            crate::dict::native_object_bytes(key.as_ptr(), self.protocol).map_err(Into::into)
        };
        let original = match bytes {
            Ok(bytes) => bytes,
            Err(first) => return first,
        };
        match tcl_syntax::naming::report_native_dictionary_missing_key(
            self.names.recipe(),
            tcl_syntax::naming::NativeDictionaryMissingKeyOperation::UnsetIntermediate,
            &original,
        ) {
            Ok(report) => match report.error_code {
                Some(code) => tcl_cmd_core::CmdError::with_error_code_bytes(report.message, code),
                None => tcl_cmd_core::CmdError::new_bytes(report.message),
            },
            Err(_) => {
                ValueError::CommandProtocolUnavailable("native dictionary missing-key diagnostic")
                    .into()
            }
        }
    }
    fn finish(&self, dictionary: Self::Prepared) -> Self::Value {
        let owned = dictionary.into_value();
        RuntimeAppendValue::retain(owned.as_ptr())
    }
}

#[path = "value_ops/original_append.rs"]
mod original_append;
pub(crate) use original_append::OriginalRuntimeAppendObjects;

/// Physical append adapter with independently retained issuer and binary updater capability.
pub(crate) struct RuntimeAppendObjects {
    pub(crate) dialect: tcl_registry::InvocationDialect,
    pub(crate) binary_recipe:
        Option<tcl_registry::native_string_materialization::ByteArrayStringRecipe>,
}

impl tcl_cmd_core::native_append::NativeAppendObjects for RuntimeAppendObjects {
    type Value = RuntimeAppendValue;
    fn snapshot(
        &self,
        value: &Self::Value,
    ) -> Result<tcl_syntax::native_object::NativeObjectSnapshot, ValueError> {
        obj::native_object_snapshot(value.as_ptr())
    }
    fn prepare_receiver(
        &self,
        value: &Self::Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Self::Value {
        RuntimeAppendValue::retain(if obj::is_shared(value.as_ptr()) {
            let duplicate = obj::duplicate(value.as_ptr());
            crate::list::duplicate_native_backing(value.as_ptr(), duplicate, protocol);
            duplicate
        } else {
            value.as_ptr()
        })
    }
    fn duplicate_into(&self, receiver: &Self::Value, source: &Self::Value) {
        obj::duplicate_into(receiver.as_ptr(), source.as_ptr());
    }
    fn string(
        &self,
        value: &Self::Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<Rc<[u8]>, ValueError> {
        if !obj::native_string_available(value.as_ptr()) {
            return Err(ValueError::CommandProtocolUnavailable(
                "native append string updater",
            ));
        }
        if !obj::has_string_rep(value.as_ptr())
            && obj::obj_type_ptr(value.as_ptr()) == &crate::bytearray::TCL_BYTE_ARRAY_TYPE
        {
            let recipe = self
                .binary_recipe
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "native append binary string recipe",
                ))?;
            if recipe.protocol() != protocol {
                return Err(ValueError::CommandProtocolUnavailable(
                    "native append binary recipe origin",
                ));
            }
        }
        Ok(Rc::from(obj::bytes_of(value.as_ptr())))
    }
    fn unicode(
        &self,
        value: &Self::Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<Rc<[u32]>, ValueError> {
        obj::native_unicode_units(value.as_ptr(), protocol)
    }
    fn decode_unicode_source(
        &self,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        bytes: &[u8],
    ) -> Result<Rc<[u32]>, ValueError> {
        let version = protocol
            .tcl_version()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original append C Unicode source units",
            ))?;
        Ok(Rc::from(
            tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(version).decode_storage_units(
                bytes,
                tcl_runtime_api::native_c_string_abi::supported_backend_plain_char(),
            ),
        ))
    }
    fn set_string(
        &self,
        value: &Self::Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        resident: Option<(
            Rc<[u8]>,
            tcl_syntax::native_string::NativeStringStorageIdentity,
        )>,
        count: Option<usize>,
        unicode: Option<Rc<[u32]>>,
    ) -> Result<(), ValueError> {
        if resident.is_none() && self.dialect.native_string_protocol() == Some(protocol) {
            if let Some(unicode) = &unicode {
                let fresh = obj::Owned::fresh(obj::new_native_unicode_obj(
                    Rc::clone(unicode),
                    self.dialect,
                )?);
                obj::duplicate_into(value.as_ptr(), fresh.as_ptr());
                return Ok(());
            }
        }
        obj::set_native_append_string(value.as_ptr(), protocol, resident, count, unicode)
    }
    fn set_binary(
        &self,
        value: &Self::Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        bytes: Rc<[u8]>,
    ) -> Result<(), ValueError> {
        let recipe = self
            .binary_recipe
            .filter(|recipe| recipe.protocol() == protocol)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native append binary updater recipe",
            ))?;
        crate::bytearray::set_native_append_bytes(value.as_ptr(), &bytes, recipe);
        Ok(())
    }
    fn new_string(&self, bytes: Rc<[u8]>) -> Self::Value {
        RuntimeAppendValue::retain(crate::interp::new_string(&bytes))
    }
}

impl tcl_cmd_core::native_cat::NativeCatObjects for RuntimeAppendObjects {
    fn empty_binary(
        &self,
        value: &Self::Value,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<Rc<[u8]>, ValueError> {
        let version = protocol
            .tcl_version()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "C compiled concat binary getter",
            ))?;
        let recipe = self
            .binary_recipe
            .filter(|recipe| recipe.protocol() == protocol)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native compiled concat binary issuer",
            ))?;
        crate::bytearray::native_binary_bytes(
            value.as_ptr(),
            tcl_registry::native_binary_value::NativeBinaryByteConversion::Narrow(
                version.string_character_model(),
            ),
            true,
            recipe,
        )
        .map(Rc::from)
        .map_err(|_| {
            ValueError::CommandProtocolUnavailable("native compiled concat empty binary conversion")
        })
    }
    fn is_shared(&self, value: &Self::Value) -> bool {
        obj::is_shared(value.as_ptr())
    }
    fn set_plain_string(&self, value: &Self::Value, bytes: Rc<[u8]>) -> Result<(), ValueError> {
        unsafe {
            obj::set_string_rep(value.as_ptr(), &bytes);
        }
        obj::discard_native_internal_representation(value.as_ptr())
    }
}

impl Interp {
    /// Validate original primary origin without reaching a String getter.
    pub(crate) fn native_string_primary_admission(
        &self,
        value: *mut TclObj,
    ) -> Result<(), ValueError> {
        obj::check_native_liveness(value)?;
        obj::native_frame_level_cache_in(value, self.native_invocation_dialect())?;
        if obj::native_instruction_name::cache(value).is_some_and(|name| {
            self.native_invocation_dialect().native_string_protocol()
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
        let formatter = host.native_integer_formatter();
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

impl ValueOps for Interp {
    fn string_character_model(&self) -> Option<tcl_dialect::StringCharacterModel> {
        self.native_invocation_dialect().characters
    }
    type Value = *mut TclObj;

    fn name_policy_protocol(&self) -> Option<tcl_syntax::naming::NamePolicyProtocol> {
        Interp::name_policy_protocol(self)
    }

    fn original_option_index(
        &mut self,
        original: &Self::Value,
        words: &'static [&'static str],
        exact: bool,
        noun: &'static str,
    ) -> Result<Option<tcl_syntax::value::OriginalOptionLookup>, ValueError> {
        use tcl_syntax::value::OriginalOptionLookup;
        obj::check_native_liveness(*original)?;
        let dialect = self.native_invocation_dialect();
        if dialect.native_jim_enum_protocol().is_some() {
            let table =
                tcl_registry::native_index_lookup::NativeStaticIndexTable::supported_backend(words);
            let flags = tcl_registry::native_jim_enum::NativeJimEnumFlags::options(exact);
            return Ok(Some(
                match self.native_jim_enum_from_original(
                    *original,
                    &table,
                    flags,
                    Some(noun.as_bytes()),
                )? {
                    Ok(index) => OriginalOptionLookup::Index(index),
                    Err(message) => OriginalOptionLookup::Failure {
                        message: message.unwrap_or_default(),
                        error_code: b"NONE".to_vec(),
                        string_result: None,
                    },
                },
            ));
        }
        let protocol = dialect.native_index_lookup_protocol().ok_or(
            ValueError::CommandProtocolUnavailable("original static option lookup"),
        )?;
        let table =
            tcl_registry::native_index_lookup::NativeStaticIndexTable::supported_backend(words);
        Ok(Some(
            match self.native_index_from_original(*original, &table, exact, noun)? {
                Ok(index) => OriginalOptionLookup::Index(index),
                Err(message) => {
                    let bytes = self.native_string_bytes(original)?;
                    OriginalOptionLookup::Failure {
                        message,
                        error_code: protocol.error_code(noun.as_bytes(), &bytes),
                        string_result: Some(tcl_syntax::native_string::NativeStringProtocol::C(
                            protocol.version(),
                        )),
                    }
                }
            },
        ))
    }

    fn same_object(&self, left: &Self::Value, right: &Self::Value) -> Option<bool> {
        (obj::allocation_is_live(*left) && obj::allocation_is_live(*right))
            .then_some(*left == *right)
    }

    fn concat_policy(&self) -> Option<tcl_dialect::ConcatPolicy> {
        self.native_invocation_dialect().concat_policy()
    }
    fn has_list_representation(&self, value: &Self::Value) -> bool {
        core::ptr::eq(obj::obj_type_ptr(*value), &crate::list::TCL_LIST_TYPE)
    }

    fn native_concat_list_shape(
        &self,
        value: &Self::Value,
    ) -> Result<tcl_syntax::value::NativeConcatListShape, ValueError> {
        native_concat::shape(self, *value)
    }
    fn native_concat_empty_list(&mut self) -> Result<Self::Value, ValueError> {
        native_concat::empty(self)
    }
    fn native_concat_copy_list(&mut self, value: &Self::Value) -> Result<Self::Value, ValueError> {
        native_concat::copy(self, *value)
    }
    fn native_concat_append_list(
        &mut self,
        target: &Self::Value,
        source: &Self::Value,
    ) -> Result<(), ValueError> {
        native_concat::append(self, *target, *source)
    }
    fn native_concat_first_bytes(
        &mut self,
        value: &Self::Value,
    ) -> Result<Option<tcl_syntax::value::NativeConcatFirstElement<Self::Value>>, ValueError> {
        native_concat::first(self, *value)
    }
    fn release_native_concat_first(
        &mut self,
        first: tcl_syntax::value::NativeConcatFirstElement<Self::Value>,
    ) {
        if let Some(original) = first.temporary {
            drop(obj::Owned::fresh(original));
        }
    }
    fn native_concat_string_bytes(&mut self, value: &Self::Value) -> Result<Rc<[u8]>, ValueError> {
        self.native_object_string_bytes(*value)
    }
    fn native_concat_string_result(&mut self, bytes: &[u8]) -> Result<Self::Value, ValueError> {
        native_concat::string(self, bytes)
    }
    fn discard_native_concat_result(&mut self, value: Self::Value) {
        drop(obj::Owned::fresh(value));
    }

    fn native_jim_string_cat(
        &mut self,
        args: &[*mut TclObj],
    ) -> Result<Option<*mut TclObj>, ValueError> {
        use tcl_registry::native_string_materialization::LogicalStringProvider;
        let dialect = self.native_invocation_dialect();
        let policy = self
            .name_policy_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "selected string cat object purpose",
            ))?;
        let provider = (policy.authority()
            == tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation)
            .then_some(LogicalStringProvider::Tcl84CoreSimulation);
        let protocol = dialect
            .native_string_materialization(provider)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "selected string cat object materialization",
            ))?
            .protocol();
        if !protocol.is_jim084() {
            return Ok(None);
        }
        let operation = crate::interp::native_operation_currency::NativeOperationScope::enter(self)
            .map_err(|_| {
                ValueError::CommandProtocolUnavailable("original Jim string cat operation")
            })?;
        let source = self.original_source_construction()?;
        let lifetimes = args
            .iter()
            .map(|&original| {
                obj::check_native_liveness(original)?;
                Ok(obj::NativeObjectLifetime::retain(original))
            })
            .collect::<Result<Vec<_>, ValueError>>()?;
        let check = |original: &RuntimeAppendValue| -> Result<(), ValueError> {
            operation
                .currency()
                .ensure_current_or_refuse()
                .map_err(|_| {
                    ValueError::CommandProtocolUnavailable("original Jim string cat operation")
                })?;
            if let Some(source) = &source {
                source.check(self)?;
            }
            for original in &lifetimes {
                original.checked_ptr()?;
            }
            obj::check_native_liveness(original.as_ptr())
        };
        let originals: Vec<_> = args
            .iter()
            .copied()
            .map(RuntimeAppendValue::borrowed)
            .collect();
        let objects = RuntimeAppendObjects {
            dialect,
            binary_recipe: None,
        };
        let retain = |error: &ValueError| {
            if let Some(refusal) = error.native_access_refusal() {
                operation.currency().retain_refusal(
                    &tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(refusal),
                );
            }
        };
        let result = tcl_cmd_core::native_cat::concatenate_jim_checked(
            &objects,
            protocol,
            &originals,
            &mut |original: &RuntimeAppendValue| {
                self.native_object_string_bytes_using(
                    original.as_ptr(),
                    &|| check(original),
                    &|error| error,
                )
            },
            &mut |original| check(original),
            &mut |error| retain(error),
        )?;
        let construction = OriginalJimStringCatConstruction::completed(
            &lifetimes,
            result.as_ptr(),
            source.clone(),
        );
        let recorded = self.record_original_namespace_info_cat(&construction);
        if let Err(error) = &recorded {
            retain(error);
        }
        recorded?;
        Ok(Some(result.into_native_unowned()))
    }

    fn index_syntax(&self) -> Option<tcl_dialect::IndexSyntax> {
        self.native_invocation_dialect().index_syntax()
    }

    fn index_error_string_protocol(
        &self,
    ) -> Result<Option<tcl_syntax::native_string::NativeStringProtocol>, ValueError> {
        let materialization = self
            .native_invocation_dialect()
            .native_string_materialization(None)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native index error String producer",
            ))?;
        Ok((!materialization.protocol().is_jim084()).then_some(materialization.protocol()))
    }

    #[cfg(not(have_tommath))]
    fn eval_index_expression(&mut self, _source: &str) -> Result<i64, ValueError> {
        Err(ValueError::ExpressionEngineUnavailable)
    }

    #[cfg(have_tommath)]
    fn eval_index_expression(&mut self, source: &str) -> Result<i64, ValueError> {
        let context = self
            .native_invocation_dialect()
            .expression_parse_context(Some(self.dialect_profile()));
        let node =
            if tcl_registry::runtime_expr_validation::requires_fixed_function_preparation(&context)
            {
                crate::builtins::prepare_runtime_fixed_expression(self, source)
                    .map_err(|error| index_expression_error(source, &error))?
            } else {
                let node = tcl_syntax::expr::parser::parse_expr_for_profile(
                    source,
                    Some(self.dialect_profile()),
                );
                tcl_registry::expr_surface::RuntimeExprSurface::for_profile(self.dialect_profile())
                    .validate(&node)
                    .map_err(|_| ValueError::NotInteger(source.to_owned()))?;
                node.map_text(String::into_bytes)
            };
        let value = crate::expr::eval_index_expression(&node, self.dialect_profile())
            .map_err(|error| index_expression_error(source, &error))?;
        match self.as_int(&value.as_ptr()) {
            Ok(value) => Ok(value),
            Err(error)
                if error.native_access_refusal().is_none()
                    && self
                        .native_invocation_dialect()
                        .index_syntax()
                        .is_some_and(|syntax| {
                            syntax.width == tcl_dialect::IndexIntegerWidth::Tcl64
                        }) =>
            {
                let bytes = obj_bytes(value.as_ptr());
                let text = std::str::from_utf8(&bytes)
                    .map_err(|_| ValueError::NotIntegerBytes(bytes.clone()))?;
                match number::parse_whole_with(
                    text,
                    number::ParseFlags::for_syntax(self.dialect_profile().grammar.numbers),
                ) {
                    Some(Number::Big { negative, .. }) => {
                        Ok(if negative { i64::MIN } else { i64::MAX })
                    }
                    _ => Err(error),
                }
            }
            Err(error) => Err(error),
        }
    }

    fn value_size_limit(&self) -> Option<u64> {
        self.value_limit()
    }

    fn measure_value(&mut self, v: &*mut TclObj, cap: u64) -> u64 {
        crate::budget::measured_bytes(*v, cap)
    }

    fn admit_value_bytes(&mut self, bytes: u64) -> bool {
        self.charge_allocation(bytes).is_none()
    }

    fn new_str(&mut self, s: &str) -> *mut TclObj {
        obj::new_string_bytes(&str_to_bytes(s))
    }

    fn new_string(&mut self, s: String) -> *mut TclObj {
        obj::new_string_bytes(&str_to_bytes(&s))
    }

    fn new_int(&mut self, n: i64) -> *mut TclObj {
        obj::new_wide_int_obj(n)
    }

    fn new_double(&mut self, f: f64) -> *mut TclObj {
        obj::new_double_obj(f)
    }

    fn new_jim_string(&mut self, bytes: &[u8], character_count: usize) -> *mut TclObj {
        let value = self.new_bytes(bytes);
        obj::retain_jim_string_count(value, character_count);
        value
    }

    fn jim_string_trim_result(
        &mut self,
        value: &Self::Value,
        plan: tcl_syntax::raw_string::JimStringTrimPlan,
    ) -> Self::Value {
        let bytes = obj_bytes(*value);
        let start = plan.byte_start();
        let end = plan.byte_end();
        if start > 0 {
            // The left-trim producer creates an unshared original working
            // header. Right trim converts and mutates that header in place.
            let working = self.new_bytes(&bytes[start..]);
            if plan.right_conversion() {
                obj::retain_jim_string_representation(working);
                if end == start {
                    crate::interp::drop_fresh(working);
                    return self.new_bytes(b"");
                }
                if end < bytes.len() {
                    obj::trim_jim_string_bytes(working, &bytes[start..end]);
                }
            }
            return working;
        }
        if plan.right_conversion() {
            obj::retain_jim_string_representation(*value);
            if end == 0 {
                return self.new_bytes(b"");
            }
        }
        if end == bytes.len() {
            return *value;
        }
        if end == 0 || obj::is_shared(*value) {
            return self.new_bytes(&bytes[..end]);
        }
        obj::trim_jim_string_bytes(*value, &bytes[..end]);
        *value
    }

    fn array_existence_result(&mut self, present: bool) -> Result<*mut TclObj, ValueError> {
        use tcl_registry::native_array_compilation::{
            native_array_existence_result, NativeArrayExistenceResult,
        };
        if self.observed_name_policy_selected() {
            return Ok(self.new_bool(present));
        }
        match native_array_existence_result(self.native_invocation_dialect()) {
            Some(NativeArrayExistenceResult::ExecutionBooleanConstant) => {
                self.native_execution_boolean_constant(present)
            }
            Some(NativeArrayExistenceResult::FreshInteger) => Ok(self.new_bool(present)),
            None => Err(ValueError::CommandProtocolUnavailable(
                "array existence result producer",
            )),
        }
    }

    fn new_bool(&mut self, b: bool) -> *mut TclObj {
        obj::new_boolean_obj(i32::from(b))
    }

    fn new_list(&mut self, items: Vec<*mut TclObj>) -> *mut TclObj {
        self.new_list_object(&items)
    }

    fn pin_value(&mut self, value: &*mut TclObj) {
        unsafe { obj::incr_ref_count(*value) };
    }

    fn unpin_value(&mut self, value: &*mut TclObj) {
        unsafe { obj::decr_ref_count(*value) };
    }

    fn try_as_str(
        &mut self,
        v: &*mut TclObj,
    ) -> Result<Rc<str>, tcl_syntax::raw_string::UnicodeAccessError> {
        tcl_syntax::raw_string::RawString::from_bytes(obj_bytes(*v)).unicode()
    }

    fn checked_character_view(&mut self, value: &*mut TclObj) -> Result<Rc<str>, ValueError> {
        if !self.has_native_character_view_entry() {
            return self.as_chars(value).map_err(Into::into);
        }
        // A ValueOps value is caller-owned live allocation storage. The lease
        // preserves that storage across callbacks without adding a Native ref.
        obj::check_native_liveness(*value)?;
        let original = obj::NativeObjectLifetime::retain(*value);
        self.native_character_view(&original)
    }

    /// A character-indexed read converts the value to a string (C's
    /// `Tcl_GetUnicodeFromObj`): a canonical list is one no longer.
    fn as_chars(
        &mut self,
        v: &*mut TclObj,
    ) -> Result<Rc<str>, tcl_syntax::raw_string::UnicodeAccessError> {
        list::read_as_string(*v);
        self.try_as_str(v)
    }

    fn is_untyped_or_string(&mut self, v: &*mut TclObj) -> bool {
        list::is_untyped_or_string(*v)
    }

    fn native_char_len(&mut self, value: &Self::Value) -> Result<usize, ValueError> {
        obj::check_native_liveness(*value)?;
        use tcl_registry::native_string_materialization::LogicalStringProvider;
        let dialect = self.native_invocation_dialect();
        let provider = Some(LogicalStringProvider::Tcl84CoreSimulation);
        let protocol = dialect
            .native_string_materialization(provider)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native string length",
            ))?
            .protocol();
        let representation = dialect
            .string_length_representation_with_provider(provider)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native string length representation",
            ))?;
        obj::native_character_count(*value, protocol, representation)
    }

    fn native_object_snapshot(
        &self,
        value: &Self::Value,
    ) -> Result<tcl_syntax::native_object::NativeObjectSnapshot, ValueError> {
        obj::native_object_snapshot(*value)
    }

    fn native_unicode_units(&mut self, value: &Self::Value) -> Result<Rc<[u32]>, ValueError> {
        obj::check_native_liveness(*value)?;
        let protocol = self
            .name_policy_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native Unicode unit recipe",
            ))?
            .string_protocol();
        obj::native_unicode_units(*value, protocol)
    }

    fn native_unicode_string_result(
        &mut self,
        units: Rc<[u32]>,
        version: tcl_dialect::TclVersion,
    ) -> Result<Self::Value, ValueError> {
        let dialect = self.native_invocation_dialect();
        if dialect.tcl_version != Some(version) || dialect.native_error_log_protocol().is_none() {
            return Err(ValueError::CommandProtocolUnavailable(
                "native Unicode result issuer",
            ));
        }
        obj::new_native_unicode_obj(units, dialect)
    }

    fn native_external_utf8_result(
        &mut self,
        bytes: &[u8],
        version: tcl_dialect::TclVersion,
    ) -> Result<Self::Value, ValueError> {
        let dialect = self.native_invocation_dialect();
        let recipe = dialect
            .byte_array_string_recipe(None)
            .filter(|recipe| {
                recipe.protocol() == tcl_syntax::native_string::NativeStringProtocol::C(version)
            })
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native external UTF-8 binary result issuer",
            ))?;
        Ok(crate::bytearray::new_byte_array(bytes, recipe))
    }

    fn discard_native_internal_representation(
        &mut self,
        value: &Self::Value,
    ) -> Result<(), ValueError> {
        obj::discard_native_internal_representation(*value)
    }

    fn as_int(&mut self, v: &*mut TclObj) -> Result<i64, ValueError> {
        let operation = ValueCoercion::enter(self, *v)?;
        match operation.scalar(tcl_syntax::scalar_getter::NativeScalarGetterKind::Wide)? {
            tcl_syntax::scalar_getter::NativeScalarGetterValue::Wide(value) => Ok(value),
            _ => Err(ValueCoercion::value_error(
                self,
                ValueError::ScalarNumericInputUnavailable,
            )),
        }
    }

    /// `Tcl_GetIntFromObj` through the runtime's one typed-read owner
    /// (`typed_value::int`), under the same coercion currency as `as_int`.
    fn as_c_int(&mut self, v: &*mut TclObj) -> Result<i32, ValueError> {
        let operation = ValueCoercion::enter(self, *v)?;
        let read = crate::typed_value::int(*v).map_err(|error| value_error(*v, &error));
        operation.check()?;
        read
    }

    #[cfg(have_tommath)]
    fn integer_magnitude(
        &mut self,
        v: &*mut TclObj,
        radix: Radix,
        syntax: tcl_dialect::NumberSyntax,
    ) -> Result<IntegerMagnitude, ValueError> {
        // A held double is refused before its spelling is read.
        if let Some(error) = crate::typed_value::held_double(*v) {
            return Err(value_error(*v, &error));
        }
        let operation = ValueCoercion::enter(self, *v)?;
        let number = match operation.integer_number()? {
            Some(number @ (Number::Int(_) | Number::Big { .. })) => number,
            _ => return Err(ValueError::NotIntegerBytes(operation.original_string()?)),
        };
        // Render the accepted Number through the existing integer digit kernel.
        // The original's bytes and the caller's format syntax cannot reselect
        // input grammar or install a numeric primary onto a rejected original.
        let integer = integer_object(number);
        if integer.is_null() {
            return Err(ValueCoercion::value_error(
                self,
                ValueError::ScalarNumericInputUnavailable,
            ));
        }
        let integer = obj::Owned::fresh(integer);
        let magnitude = crate::bignum::integer_magnitude(integer.as_ptr(), radix, syntax);
        drop(integer);
        operation.check()?;
        magnitude
            .map(|(negative, digits)| IntegerMagnitude { negative, digits })
            .ok_or_else(|| {
                ValueCoercion::value_error(self, ValueError::ScalarNumericInputUnavailable)
            })
    }

    fn string_compare_length(&mut self, v: &*mut TclObj) -> Result<Option<usize>, ValueError> {
        let operation = ValueCoercion::enter(self, *v)?;
        match operation.integer_number()? {
            Some(Number::Int(integer)) => Ok(usize::try_from(integer).ok()),
            Some(Number::Big { .. }) => Err(ValueError::IntegerOverflow),
            _ => Err(ValueError::NotIntegerBytes(operation.original_string()?)),
        }
    }

    fn as_double(&mut self, v: &*mut TclObj) -> Result<f64, ValueError> {
        let operation = ValueCoercion::enter(self, *v)?;
        // A held double is its own value (`Tcl_GetDoubleFromObj`), not its
        // spelling, which 8.4 rounds to twelve digits.
        if obj::obj_type_ptr(*v) == &obj::TCL_DOUBLE_TYPE && !obj::double_of(*v).is_nan() {
            let value = obj::double_of(*v);
            operation.check()?;
            return Ok(value);
        }
        match operation.scalar(tcl_syntax::scalar_getter::NativeScalarGetterKind::Double)? {
            tcl_syntax::scalar_getter::NativeScalarGetterValue::Double(value) => Ok(value),
            _ => Err(ValueCoercion::value_error(
                self,
                ValueError::ScalarNumericInputUnavailable,
            )),
        }
    }

    fn as_bool(&mut self, v: &*mut TclObj) -> Result<bool, ValueError> {
        let operation = ValueCoercion::enter(self, *v)?;
        match operation.scalar(tcl_syntax::scalar_getter::NativeScalarGetterKind::Boolean)? {
            tcl_syntax::scalar_getter::NativeScalarGetterValue::Boolean(value) => {
                Ok(value.is_true())
            }
            _ => Err(ValueCoercion::value_error(
                self,
                ValueError::ScalarNumericInputUnavailable,
            )),
        }
    }

    /// Integer addition follows the selected native tower, independently of
    /// whether this build has an arbitrary-precision backend.
    fn int_add(
        &mut self,
        a: Option<&*mut TclObj>,
        b: &*mut TclObj,
    ) -> Result<*mut TclObj, ValueError> {
        if let Some(left) = a {
            obj::check_native_liveness(*left)?;
        }
        obj::check_native_liveness(*b)?;
        use tcl_dialect::NativeArithmetic;
        let dialect = self.native_invocation_dialect();
        let policy = dialect.arithmetic().ok_or(ValueError::IntegerOverflow)?;
        // Preserve native coercion order: current value, then increment.
        let left = a
            .map(|value| integer_operand(*value, dialect))
            .transpose()?;
        let right = integer_operand(*b, dialect)?;
        if policy != NativeArithmetic::TclBignum {
            let left = left
                .as_ref()
                .map(|value| tcl_syntax::expr::wide::parsed_literal(policy, value))
                .transpose()
                .map_err(|_| ValueError::IntegerOverflow)?
                .unwrap_or(0);
            let right = tcl_syntax::expr::wide::parsed_literal(policy, &right)
                .map_err(|_| ValueError::IntegerOverflow)?;
            let sum =
                tcl_syntax::expr::wide::binary(policy, tcl_syntax::expr::BinOp::Add, left, right)
                    .map_err(|_| ValueError::IntegerOverflow)?;
            return Ok(obj::new_wide_int_obj(sum));
        }
        #[cfg(have_tommath)]
        {
            let left = integer_object(left.unwrap_or(Number::Int(0)));
            let right = integer_object(right);
            let result = crate::bignum::add(left, right).map_err(|_| ValueError::IntegerOverflow);
            crate::interp::drop_fresh(left);
            crate::interp::drop_fresh(right);
            result
        }
        #[cfg(not(have_tommath))]
        {
            let (Some(Number::Int(left)), Number::Int(right)) =
                (left.or(Some(Number::Int(0))), right)
            else {
                return Err(ValueError::IntegerOverflow);
            };
            left.checked_add(right)
                .map(obj::new_wide_int_obj)
                .ok_or(ValueError::IntegerOverflow)
        }
    }

    fn list_len(&mut self, value: &Self::Value) -> Result<usize, ValueError> {
        native_jim_index::list_length_using(
            self,
            value,
            &mut |_, _| Ok(()),
            |interpreter, original| interpreter.list_elements(original),
        )
    }

    fn list_index(
        &mut self,
        value: &Self::Value,
        index: usize,
    ) -> Result<Option<Self::Value>, ValueError> {
        obj::check_native_liveness(*value)?;
        if crate::native_arithseries::is_series(*value) {
            let protocol = self
                .native_invocation_dialect()
                .native_string_protocol()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "abstract native Index",
                ))?;
            return crate::native_arithseries::index_in(*value, index, protocol);
        }
        Ok(self.list_elements(value)?.into_iter().nth(index))
    }

    fn list_elements(&mut self, v: &*mut TclObj) -> Result<Vec<*mut TclObj>, ValueError> {
        obj::check_native_liveness(*v)?;
        let protocol = self.native_invocation_dialect().native_string_materialization(
            Some(tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation),
        ).ok_or(ValueError::CommandProtocolUnavailable("native object list conversion"))?.protocol();
        if protocol.is_jim084() {
            crate::native_source::bind_context(*v, &self.native_jim_object_context()?)?;
        }
        list::list_elements_native_checked(*v, protocol)
    }

    fn dict_pairs(
        &mut self,
        v: &*mut TclObj,
    ) -> Result<Vec<(*mut TclObj, *mut TclObj)>, ValueError> {
        obj::check_native_liveness(*v)?;
        let protocol = self.native_invocation_dialect().native_string_materialization(
            Some(tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation),
        ).ok_or(ValueError::CommandProtocolUnavailable("native object dictionary conversion"))?.protocol();
        if protocol.is_jim084() {
            crate::native_source::bind_context(*v, &self.native_jim_object_context()?)?;
        }
        crate::dict::native_dict_pairs(*v, protocol)
    }

    fn dict_hash_bucket_count(&mut self, v: &*mut TclObj) -> Result<Option<usize>, ValueError> {
        drop(self.dict_pairs(v)?);
        Ok(Some(
            crate::dict::dict_hash_bucket_count(*v).expect("native Dictionary cache"),
        ))
    }

    fn new_dict_checked(
        &mut self,
        pairs: Vec<(*mut TclObj, *mut TclObj)>,
    ) -> Result<*mut TclObj, ValueError> {
        self.new_dict_with_hash_bucket_count_checked(pairs, 4)
    }

    fn new_dict_with_hash_bucket_count_checked(
        &mut self,
        pairs: Vec<(*mut TclObj, *mut TclObj)>,
        bucket_count: usize,
    ) -> Result<*mut TclObj, ValueError> {
        let recipe = self.native_invocation_dialect().native_string_materialization(Some(tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation)).ok_or(ValueError::CommandProtocolUnavailable("native dictionary construction"))?;
        let buckets = Some(bucket_count);
        crate::dict::new_dict_obj_native(&pairs, buckets, recipe.protocol())
    }

    fn new_dict(&mut self, pairs: Vec<(*mut TclObj, *mut TclObj)>) -> *mut TclObj {
        let value = crate::dict::new_dict_obj(&pairs);
        if let Some(recipe) = self
            .native_invocation_dialect()
            .native_string_materialization(Some(
            tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation,
        )) {
            crate::dict::seal_string_protocol(value, recipe.protocol())
                .expect("fresh native Dictionary recipe");
        }
        value
    }

    fn new_dict_with_hash_bucket_count(
        &mut self,
        pairs: Vec<(*mut TclObj, *mut TclObj)>,
        bucket_count: usize,
    ) -> *mut TclObj {
        let value = crate::dict::new_dict_obj_with_hash_bucket_count(&pairs, Some(bucket_count));
        if let Some(recipe) = self
            .native_invocation_dialect()
            .native_string_materialization(Some(
            tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation,
        )) {
            crate::dict::seal_string_protocol(value, recipe.protocol())
                .expect("fresh native Dictionary recipe");
        }
        value
    }

    /// Byte-exact, and simpler than `as_str`'s Unicode round trip — this is why
    /// `append` (which never needs character semantics) routes through the
    /// shared core without any of `bytes_to_str`/`str_to_bytes`'s trade-offs.
    fn as_bytes(&mut self, v: &*mut TclObj) -> Rc<[u8]> {
        Rc::from(obj_bytes(*v).as_slice())
    }

    fn native_string_bytes(&mut self, value: &Self::Value) -> Result<Rc<[u8]>, ValueError> {
        self.native_object_string_bytes(*value)
    }

    fn native_split(
        &mut self,
        value: &Self::Value,
        chars: Option<&Self::Value>,
    ) -> Result<Option<Self::Value>, ValueError> {
        use tcl_syntax::{naming::NamePolicyAuthority, native_string::NativeStringProtocol};
        let protocol = NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6);
        if !self.name_policy_protocol().is_some_and(|policy| {
            policy.authority() == NamePolicyAuthority::Native
                && policy.string_protocol() == protocol
        }) {
            return Ok(None);
        }
        let original = ValueCoercion::enter(self, *value)?;
        let delimiter = chars
            .map(|&chars| {
                obj::check_native_liveness(chars)?;
                Ok::<_, ValueError>(obj::NativeObjectLifetime::retain(chars))
            })
            .transpose()?;
        let characters = if let Some(delimiter) = &delimiter {
            let bytes = self.native_object_string_bytes(delimiter.as_ptr());
            original.check()?;
            obj::check_native_liveness(delimiter.as_ptr())?;
            bytes?
        } else {
            Rc::from(&b" \n\t\r"[..])
        };
        let source = self.native_object_string_bytes(*value);
        original.check()?;
        if let Some(delimiter) = &delimiter {
            obj::check_native_liveness(delimiter.as_ptr())?;
        }
        let source = source?;
        let parts = tcl_cmd_core::list::plan_c86_split(&source, &characters)?;
        let result = obj::Owned::fresh(obj::new_obj());
        let mut reused = std::collections::HashMap::<u32, *mut TclObj>::new();
        for part in parts {
            let existing = part.reuse.and_then(|key| reused.get(&key).copied());
            let fresh = existing
                .is_none()
                .then(|| obj::Owned::fresh(obj::new_string_bytes(&source[part.bytes])));
            let child =
                existing.unwrap_or_else(|| fresh.as_ref().expect("fresh split child").as_ptr());
            let appended = crate::list::append_native_element_with_currency(
                result.as_ptr(),
                child,
                protocol,
                original.scope.currency(),
            );
            original.check()?;
            appended.map_err(|error| ValueCoercion::value_error(self, error))?;
            if let Some(key) = part.reuse {
                reused.insert(key, child);
            }
        }
        original.check()?;
        Ok(Some(result.into_native_unowned()))
    }

    /// Byte-exact construction (the `obj::new_string_bytes` path).
    fn new_bytes(&mut self, bytes: &[u8]) -> *mut TclObj {
        obj::new_string_bytes(bytes)
    }

    fn native_command_full_name_result(&mut self, bytes: &[u8]) -> Result<*mut TclObj, ValueError> {
        Ok(self
            .native_namespace_origin_result(bytes)?
            .into_native_unowned())
    }

    fn try_append_bytes_in_place(&mut self, v: &mut *mut TclObj, bytes: &[u8]) -> bool {
        // Amortised O(1) growth when the object is an unshared plain string —
        // an in-place path the VM cannot take (it always copies).
        if obj::is_plain_string(*v) && !obj::is_shared(*v) {
            obj::string_append_inplace(*v, bytes);
            true
        } else {
            false
        }
    }

    fn try_list_append_in_place(
        &mut self,
        list: &mut *mut TclObj,
        item: &*mut TclObj,
    ) -> Result<bool, ValueError> {
        // naming.list.original-value-append-storage-currency

        obj::check_native_liveness(*list)?;
        obj::check_native_liveness(*item)?;
        if obj::is_shared(*list) {
            return Ok(false);
        }
        let protocol = self.native_invocation_dialect().native_string_materialization(
            Some(tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation),
        ).ok_or(ValueError::CommandProtocolUnavailable("native object list append"))?.protocol();
        if protocol.is_jim084() {
            crate::native_source::bind_context(*list, &self.native_jim_object_context()?)?;
        }
        list::append_prepared_native_elements(*list, &[*item], protocol)?;
        Ok(true)
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

pub(crate) struct RuntimeDateList(Rc<RuntimeDateListState>);
struct RuntimeDateListState {
    original: obj::NativeObjectLifetime,
    primary: obj::NativePrimaryReceipt,
    backing: Option<crate::list::NativeListBacking>,
    members: Vec<*mut TclObj>,
}

/// One inherited operation, with allocation-only views of reached originals.
pub(crate) struct RuntimeClockDateOps {
    interpreter: Interp,
    scope: crate::interp::native_operation_currency::NativeOperationScope,
    literals: tcl_cmd_core::clock::DateFieldLiterals<*mut TclObj>,
    literal_receipts: Vec<obj::NativePrimaryReceipt>,
    windows: Vec<Rc<RuntimeDateListState>>,
    children: Vec<obj::NativeObjectLifetime>,
}

impl RuntimeClockDateOps {
    pub(crate) fn new(
        interpreter: &Interp,
        literals: tcl_cmd_core::clock::DateFieldLiterals<*mut TclObj>,
    ) -> Result<Self, ValueError> {
        use tcl_syntax::{naming::NamePolicyAuthority, native_string::NativeStringProtocol};
        let scope =
            crate::interp::native_operation_currency::NativeOperationScope::enter(interpreter)
                .map_err(|first| ValueCoercion::execution_error(interpreter, first))?;
        if interpreter
            .native_invocation_dialect()
            .clock_get_date_fields_registration()
            .is_none()
            || !interpreter.name_policy_protocol().is_some_and(|policy| {
                policy.authority() == NamePolicyAuthority::Native
                    && policy.string_protocol()
                        == NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6)
            })
        {
            return Err(ValueCoercion::value_error(
                interpreter,
                ValueError::CommandProtocolUnavailable("native C86 GetDateFields worker"),
            ));
        }
        let literal_receipts = literals
            .keys
            .iter()
            .copied()
            .chain([literals.ce, literals.bce])
            .map(obj::NativePrimaryReceipt::capture)
            .collect::<Result<Vec<_>, _>>()?;
        let result = Self {
            interpreter: interpreter.clone(),
            scope,
            literals,
            literal_receipts,
            windows: Vec::new(),
            children: Vec::new(),
        };
        result.check_value()?;
        Ok(result)
    }

    fn receiver_check(&self) -> Result<(), ValueError> {
        for (receipt, value) in self.literal_receipts.iter().zip(
            self.literals
                .keys
                .iter()
                .copied()
                .chain([self.literals.ce, self.literals.bce]),
        ) {
            receipt.check_for(value, "changed C86 clock literal primary")?;
        }
        for window in &self.windows {
            window.primary.check_for(
                window.original.as_ptr(),
                "changed C86 timezone List primary",
            )?;
            if let Some(backing) = &window.backing {
                backing.check_generation()?;
            }
        }
        for child in &self.children {
            obj::check_native_liveness(child.as_ptr())?;
        }
        Ok(())
    }

    fn check_execution(&self) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        self.scope.currency().ensure_current_or_refuse()?;
        self.receiver_check().map_err(|error| {
            let cause = error
                .native_access_refusal()
                .expect("physical clock receiver guard");
            let first = tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(cause);
            self.interpreter
                .clone()
                .refuse_native_execution(first.clone());
            self.interpreter.native_execution_refusal().unwrap_or(first)
        })
    }

    pub(crate) fn check_value(&self) -> Result<(), ValueError> {
        self.check_execution()
            .map_err(|first| ValueCoercion::execution_error(&self.interpreter, first))
    }

    fn after<T>(&self, result: Result<T, ValueError>) -> Result<T, ValueError> {
        self.check_value()?;
        result.map_err(|error| ValueCoercion::value_error(&self.interpreter, error))
    }

    fn hold(&mut self, value: *mut TclObj) -> Result<*mut TclObj, ValueError> {
        self.check_value()?;
        obj::check_native_liveness(value)
            .map_err(|error| ValueCoercion::value_error(&self.interpreter, error))?;
        self.children.push(obj::NativeObjectLifetime::retain(value));
        Ok(value)
    }
    fn prepare_list(
        &mut self,
        value: &*mut TclObj,
        ordinary: bool,
    ) -> Result<RuntimeDateList, ValueError> {
        use tcl_syntax::native_string::NativeStringProtocol;
        self.check_value()?;
        obj::check_native_liveness(*value)?;
        let original = obj::NativeObjectLifetime::retain(*value);
        let check = || {
            self.check_value()?;
            obj::check_native_liveness(*value)
        };
        let retain = |error| ValueCoercion::value_error(&self.interpreter, error);
        let protocol = NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6);
        let result = if ordinary {
            crate::list::ordinary_list_elements_native_using(*value, protocol, &check, &retain)
        } else {
            crate::list::list_elements_native_with_stage_handlers(
                *value,
                protocol,
                Some(self.scope.currency()),
                Some(&check),
                Some(&retain),
            )
        };
        let members = self.after(result)?;
        // The getter owns its intended coercion; capture the new primary only afterward.
        let primary = obj::NativePrimaryReceipt::capture(*value)?;
        let backing = crate::list::native_list_backing(*value);
        if backing.is_none() && !(members.is_empty() && obj::has_canonical_empty_string(*value)) {
            return Err(ValueCoercion::value_error(
                &self.interpreter,
                ValueError::CommandProtocolUnavailable("original C86 timezone List backing"),
            ));
        }
        let state = Rc::new(RuntimeDateListState {
            original,
            primary,
            backing,
            members,
        });
        self.windows.push(Rc::clone(&state));
        self.check_value()?;
        Ok(RuntimeDateList(state))
    }
}

impl tcl_cmd_core::clock::NativeClockDateOps for RuntimeClockDateOps {
    type Value = *mut TclObj;
    type List = RuntimeDateList;

    fn date_check(&self) -> Result<(), ValueError> {
        self.check_value()
    }

    fn date_wide(&mut self, value: &Self::Value) -> Result<i64, ValueError> {
        self.hold(*value)?;
        let conversion = ValueCoercion::enter(&self.interpreter, *value)?;
        let result = conversion.scalar(tcl_syntax::scalar_getter::NativeScalarGetterKind::Wide);
        let result = self.after(result)?;
        match result {
            tcl_syntax::scalar_getter::NativeScalarGetterValue::Wide(number) => Ok(number),
            _ => Err(ValueCoercion::value_error(
                &self.interpreter,
                ValueError::ScalarNumericInputUnavailable,
            )),
        }
    }

    fn date_int(&mut self, value: &Self::Value) -> Result<i32, ValueError> {
        self.hold(*value)?;
        let conversion = ValueCoercion::enter(&self.interpreter, *value)?;
        let result = conversion.scalar(tcl_syntax::scalar_getter::NativeScalarGetterKind::Int);
        let result = self.after(result)?;
        match result {
            tcl_syntax::scalar_getter::NativeScalarGetterValue::Wide(number) => {
                i32::try_from(number).map_err(|_| {
                    ValueCoercion::value_error(
                        &self.interpreter,
                        ValueError::ScalarNumericInputUnavailable,
                    )
                })
            }
            _ => Err(ValueCoercion::value_error(
                &self.interpreter,
                ValueError::ScalarNumericInputUnavailable,
            )),
        }
    }

    fn date_is_bignum(&mut self, value: &Self::Value) -> Result<bool, ValueError> {
        self.check_value()?;
        let snapshot = self.after(obj::native_object_snapshot(*value))?;
        Ok(matches!(
            snapshot.cache,
            tcl_syntax::native_object::NativeObjectCacheSnapshot::Numeric(
                tcl_syntax::scalar_getter::NativeScalarCache::Number(Number::Big { .. })
            )
        ))
    }

    fn date_elements(&mut self, value: &Self::Value) -> Result<Self::List, ValueError> {
        self.prepare_list(value, false)
    }

    fn date_len(&self, list: &Self::List) -> Result<usize, ValueError> {
        self.check_value()?;
        Ok(list.0.members.len())
    }

    fn date_child(&mut self, list: &Self::List, index: usize) -> Result<Self::Value, ValueError> {
        self.check_value()?;
        let child = list.0.members.get(index).copied().ok_or_else(|| {
            ValueCoercion::value_error(
                &self.interpreter,
                ValueError::CommandProtocolUnavailable("unsafe C86 timezone cell extent"),
            )
        })?;
        self.hold(child)
    }

    fn date_index_zero(&mut self, value: &Self::Value) -> Result<Self::Value, ValueError> {
        let list = self.prepare_list(value, true)?;
        if list.0.members.is_empty() {
            return Err(ValueCoercion::value_error(
                &self.interpreter,
                ValueError::CommandProtocolUnavailable("unsafe C86 first transition cell"),
            ));
        }
        // Actual C86 ordinary ListIndex follows its own reached row conversion.
        let result = crate::list::list_index(*value, 0).map_err(|error| ValueError::ListParse {
            error: error.shared(),
            source: Vec::new(),
        });
        let child = self.after(result)?.ok_or_else(|| {
            ValueCoercion::value_error(
                &self.interpreter,
                ValueError::CommandProtocolUnavailable("unsafe C86 first transition cell"),
            )
        })?;
        self.hold(child)
    }

    fn date_result(
        &mut self,
        fields: &tcl_cmd_core::clock::C86CalendarFields,
        name: &Self::Value,
    ) -> Result<Self::Value, tcl_cmd_core::CmdError> {
        use tcl_cmd_core::clock::C86DateValue;
        self.check_value()?;
        let name_receipt = obj::NativePrimaryReceipt::capture(*name)?;
        let mut fresh = Vec::new();
        let values = tcl_cmd_core::clock::c86_date_values(fields, name, &self.literals)
            .into_iter()
            .map(|role| match role {
                C86DateValue::Original(value) => value,
                C86DateValue::Wide(number) => {
                    fresh.push(obj::Owned::fresh(obj::new_wide_int_obj(number)));
                    fresh.last().expect("fresh date field").as_ptr()
                }
                C86DateValue::Int(number) => {
                    fresh.push(obj::Owned::fresh(crate::capi::Tcl_NewIntObj(number)));
                    fresh.last().expect("fresh date field").as_ptr()
                }
            })
            .collect::<Vec<_>>();
        let pairs = self
            .literals
            .keys
            .iter()
            .copied()
            .zip(values)
            .collect::<Vec<_>>();
        let check = || {
            self.check_execution()?;
            name_receipt
                .check_for(*name, "changed C86 timezone name primary")
                .map_err(|error| {
                    let first = tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                        error
                            .native_access_refusal()
                            .expect("physical clock name guard"),
                    );
                    self.interpreter
                        .clone()
                        .refuse_native_execution(first.clone());
                    self.interpreter.native_execution_refusal().unwrap_or(first)
                })
        };
        let result = crate::dict::new_dict_obj_native_with_stage_check(
            &pairs,
            None,
            tcl_syntax::native_string::NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6),
            &check,
        )
        .map_err(|error| {
            if let Some(first) = error.native_execution_refusal() {
                self.interpreter
                    .clone()
                    .refuse_native_execution(first.clone());
            }
            error
        });
        let result = result.map(obj::Owned::fresh);
        check()?;
        Ok(result?.into_native_unowned())
    }
}

#[cfg(test)]
mod legacy_extension_length_tests {
    use super::*;
    use std::cell::Cell;
    use tcl_registry::native_stock_list::NativeStockListInputClass as Class;
    thread_local! { static READS: Cell<usize> = const { Cell::new(0) }; }
    extern "C" fn update(original: *mut TclObj) {
        READS.with(|reads| reads.set(reads.get() + 1));
        unsafe { obj::set_native_updater_string_rep(original, b"one two", false) };
    }
    static LEGACY: obj::TclObjType = obj::TclObjType {
        name: c"registeredLegacyLength".as_ptr(),
        free_int_rep_proc: None,
        dup_int_rep_proc: None,
        update_string_proc: Some(update),
        set_from_any_proc: None,
    };
    static UNREGISTERED: obj::TclObjType = obj::TclObjType {
        name: c"unregisteredLength".as_ptr(),
        free_int_rep_proc: None,
        dup_int_rep_proc: None,
        update_string_proc: Some(update),
        set_from_any_proc: None,
    };
    #[test]
    fn registered_legacy_length_converts_original_without_admitting_unknown_descriptors() {
        obj::register_legacy_extension_type(&LEGACY);
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            READS.with(|reads| reads.set(0));
            let mut actual = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(profile),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let legacy = obj::Owned::fresh(obj::alloc_typed(&LEGACY, 0));
            let unknown = obj::Owned::fresh(obj::alloc_typed(&UNREGISTERED, 0));
            assert_eq!(
                obj::stock_list_input_class(legacy.as_ptr()),
                Class::LegacyExtension
            );
            assert_eq!(
                obj::stock_list_input_class(unknown.as_ptr()),
                Class::Unknown
            );
            assert_eq!(actual.list_len(&legacy.as_ptr()).unwrap(), 2, "{profile}");
            assert_eq!(READS.with(Cell::get), 1);
            assert!(actual.list_len(&unknown.as_ptr()).is_err());
            assert_eq!(READS.with(Cell::get), 1);
            assert!(!obj::has_string_rep(unknown.as_ptr()));
        }
    }
}
