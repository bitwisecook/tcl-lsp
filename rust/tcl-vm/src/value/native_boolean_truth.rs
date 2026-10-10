// SPDX-License-Identifier: AGPL-3.0-or-later
//! Checked physical operations for the shared original expression-result worker.

use super::{IntRep, NativeStringStorageIdentity, Value};
use tcl_registry::InvocationDialect;
use tcl_syntax::{
    number::Number,
    scalar_getter::{
        NativeScalarCache as Cache, NativeScalarGetterFailure as Failure,
        NativeScalarGetterKind as Getter, NativeScalarGetterValue as GetterValue,
    },
    value::ValueError,
};

impl Value {
    pub(crate) fn original_expression_integer84(
        &self,
        dialect: InvocationDialect,
        environment: Option<&dyn tcl_platform::NumericEnvironment>,
    ) -> Result<Result<GetterValue, Failure>, ValueError> {
        self.check_native_header()?;
        let scalar = dialect
            .native_scalar_getter_protocol()
            .filter(|scalar| scalar.tcl_version() == Some(tcl_dialect::TclVersion::V8_4))
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        let string = dialect
            .native_string_protocol()
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        let current = self.native_scalar_cache();
        let original = self
            .native_string_bytes(string)
            .map_err(tcl_syntax::raw_string::NativeStringAccessError::Unavailable)
            .map_err(ValueError::from)?;
        if let Some(environment) = environment {
            let before = environment
                .c_integer_abi()
                .map_err(|_| ValueError::ScalarNumericInputUnavailable)?;
            tcl_cmd_core::native_numeric::scalar_getter_target(environment)?;
            tcl_cmd_core::native_numeric::fresh_c84_conversion(
                scalar,
                Getter::Wide,
                &original,
                environment,
            )?;
            let after = environment
                .c_integer_abi()
                .map_err(|_| ValueError::ScalarNumericInputUnavailable)?;
            if before != after {
                return Err(ValueError::ScalarNumericInputUnavailable);
            }
        } else if !matches!(
            current,
            Some(Cache::Tcl84Long(_) | Cache::Number(Number::Int(_)))
        ) {
            return Err(ValueError::ScalarNumericInputUnavailable);
        }
        let conversion = scalar
            .expression_integer_conversion84(current.as_ref(), &original)
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        let (materialize, cache, outcome) = conversion.into_parts();
        if materialize {
            self.native_string_bytes(string)
                .map_err(tcl_syntax::raw_string::NativeStringAccessError::Unavailable)
                .map_err(ValueError::from)?;
        }
        if let Some(cache) = cache {
            self.adopt_native_scalar_cache(cache, scalar, dialect)?;
        }
        Ok(outcome)
    }

    pub(crate) fn original_expression_word_boolean84(
        &self,
        dialect: InvocationDialect,
        boolean: bool,
    ) -> Result<GetterValue, ValueError> {
        self.check_native_header()?;
        let scalar = dialect
            .native_scalar_getter_protocol()
            .filter(|scalar| scalar.tcl_version() == Some(tcl_dialect::TclVersion::V8_4))
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        if self.resident_string_bytes().is_some()
            || self.native_scalar_cache() != Some(Cache::WordBoolean(boolean))
        {
            return Err(ValueError::ScalarNumericInputUnavailable);
        }
        self.adopt_native_scalar_cache(Cache::Tcl84Long(i64::from(boolean)), scalar, dialect)?;
        Ok(GetterValue::Wide(i64::from(boolean)))
    }

    pub(crate) fn original_expression_numeric_invalidate(&self) -> Result<(), ValueError> {
        self.check_native_header()?;
        if self.native_object_is_shared()
            || !matches!(
                self.native_scalar_cache(),
                Some(Cache::Tcl84Long(_) | Cache::Number(_))
            )
        {
            return Err(ValueError::ScalarNumericInputUnavailable);
        }
        *self.0.string.borrow_mut() = None;
        self.0
            .string_storage
            .set(NativeStringStorageIdentity::Unknown);
        *self.0.source_location.borrow_mut() = None;
        Ok(())
    }

    pub(crate) fn original_expression_cached_lengths(
        &self,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Result<(Option<usize>, Option<usize>), ValueError> {
        self.check_native_header()?;
        self.seal_compound_string_protocol(protocol)
            .map_err(tcl_syntax::raw_string::NativeStringAccessError::Unavailable)
            .map_err(ValueError::from)?;
        match &*self.0.intrep.borrow() {
            IntRep::List { items, .. } => Ok((Some(items.len()), None)),
            IntRep::Dict(dict) => Ok((None, Some(dict.contents.borrow().pairs.len()))),
            _ => Ok((None, None)),
        }
    }
}
