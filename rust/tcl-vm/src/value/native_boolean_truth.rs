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
    pub(crate) fn original_expression_integer84_checked<E: From<ValueError>>(
        &self,
        dialect: InvocationDialect,
        environment: Option<&dyn tcl_platform::NumericEnvironment>,
        check: &mut dyn FnMut() -> Result<(), E>,
    ) -> Result<Result<GetterValue, Failure>, E> {
        self.original_expression_integer84_using(dialect, environment, &mut Some(check))
    }

    fn original_expression_integer84_using<E: From<ValueError>>(
        &self,
        dialect: InvocationDialect,
        environment: Option<&dyn tcl_platform::NumericEnvironment>,
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
    ) -> Result<Result<GetterValue, Failure>, E> {
        Self::native_stage_check(check)?;
        self.check_native_header()?;
        let scalar = dialect
            .native_scalar_getter_protocol()
            .filter(|scalar| scalar.tcl_version() == Some(tcl_dialect::TclVersion::V8_4))
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        let string = dialect
            .native_string_protocol()
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        let current = self.native_scalar_cache();
        let reached = self
            .native_string_bytes(string)
            .map_err(tcl_syntax::raw_string::NativeStringAccessError::Unavailable)
            .map_err(ValueError::from);
        let original = Self::native_stage_result(reached, check)?;
        if let Some(environment) = environment {
            let reached = environment
                .c_integer_abi()
                .map_err(|_| ValueError::ScalarNumericInputUnavailable);
            let before = Self::native_stage_result(reached, check)?;
            let reached = tcl_cmd_core::native_numeric::scalar_getter_target(environment);
            Self::native_stage_result(reached, check)?;
            let reached = tcl_cmd_core::native_numeric::fresh_c84_conversion(
                scalar,
                Getter::Wide,
                &original,
                environment,
            );
            Self::native_stage_result(reached, check)?;
            let reached = environment
                .c_integer_abi()
                .map_err(|_| ValueError::ScalarNumericInputUnavailable);
            let after = Self::native_stage_result(reached, check)?;
            if before != after {
                return Err(ValueError::ScalarNumericInputUnavailable.into());
            }
        } else if !matches!(
            current,
            Some(Cache::Tcl84Long(_) | Cache::Number(Number::Int(_)))
        ) {
            return Err(ValueError::ScalarNumericInputUnavailable.into());
        }
        let conversion = scalar
            .expression_integer_conversion84(current.as_ref(), &original)
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        let (materialize, cache, outcome) = conversion.into_parts();
        if materialize {
            let reached = self
                .native_string_bytes(string)
                .map_err(tcl_syntax::raw_string::NativeStringAccessError::Unavailable)
                .map_err(ValueError::from);
            Self::native_stage_result(reached, check)?;
        }
        if let Some(cache) = cache {
            self.adopt_native_scalar_cache_using(cache, scalar, dialect, check)?;
        }
        Self::native_stage_check(check)?;
        Ok(outcome)
    }

    pub(crate) fn original_expression_word_boolean84_checked<E: From<ValueError>>(
        &self,
        dialect: InvocationDialect,
        boolean: bool,
        check: &mut dyn FnMut() -> Result<(), E>,
    ) -> Result<GetterValue, E> {
        self.original_expression_word_boolean84_using(dialect, boolean, &mut Some(check))
    }

    fn original_expression_word_boolean84_using<E: From<ValueError>>(
        &self,
        dialect: InvocationDialect,
        boolean: bool,
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
    ) -> Result<GetterValue, E> {
        Self::native_stage_check(check)?;
        self.check_native_header()?;
        let scalar = dialect
            .native_scalar_getter_protocol()
            .filter(|scalar| scalar.tcl_version() == Some(tcl_dialect::TclVersion::V8_4))
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        if self.resident_string_bytes().is_some()
            || self.native_scalar_cache() != Some(Cache::WordBoolean(boolean))
        {
            return Err(ValueError::ScalarNumericInputUnavailable.into());
        }
        self.adopt_native_scalar_cache_using(
            Cache::Tcl84Long(i64::from(boolean)),
            scalar,
            dialect,
            check,
        )?;
        Self::native_stage_check(check)?;
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
