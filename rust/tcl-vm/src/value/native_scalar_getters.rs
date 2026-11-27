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

//! Native scalar conversion on the original object: probes, primitive getters and their failure presentation, the C 8.4 expression integer and the increment stores.

use super::{IntRep, NativeScalarProbeRecipe, Value};
use std::cell::Cell;
use std::rc::Rc;
use tcl_syntax::native_string::{NativeStringProtocol, NativeStringStorageIdentity};
use tcl_syntax::number::Number;
use tcl_syntax::scalar_getter::{
    NativeScalarCache, NativeScalarGetterKind, NativeScalarGetterProtocol, NativeScalarGetterValue,
};
use tcl_syntax::value::ValueError;

impl Value {
    /// Probe an original shared object without rendering guest failure.
    /// Full cache changes apply on failure. An error-neutral native probe does
    /// not materialise a cached object's string merely to describe its failure.
    ///
    /// # Errors
    /// Returns a typed missing-capability refusal outside the native outcome.
    pub fn native_scalar_probe(
        &self,
        dialect: tcl_native_model::InvocationDialect,
        kind: NativeScalarGetterKind,
    ) -> Result<
        Result<NativeScalarGetterValue, tcl_syntax::scalar_getter::NativeScalarGetterFailure>,
        tcl_syntax::value::ValueError,
    > {
        self.native_scalar_probe_with_environment(dialect, kind, None)
    }

    pub(crate) fn native_scalar_probe_with_environment(
        &self,
        dialect: tcl_native_model::InvocationDialect,
        kind: NativeScalarGetterKind,
        environment: Option<&dyn tcl_platform::NumericEnvironment>,
    ) -> Result<
        Result<NativeScalarGetterValue, tcl_syntax::scalar_getter::NativeScalarGetterFailure>,
        ValueError,
    > {
        self.native_scalar_probe_using(dialect, kind, environment, &mut None, &mut None)
    }

    #[cfg(test)]
    pub(crate) fn native_scalar_probe_checked<E: From<ValueError>>(
        &self,
        dialect: tcl_native_model::InvocationDialect,
        kind: NativeScalarGetterKind,
        environment: Option<&dyn tcl_platform::NumericEnvironment>,
        check: &mut dyn FnMut() -> Result<(), E>,
    ) -> Result<
        Result<NativeScalarGetterValue, tcl_syntax::scalar_getter::NativeScalarGetterFailure>,
        E,
    > {
        self.native_scalar_probe_using(dialect, kind, environment, &mut Some(check), &mut None)
    }

    pub(crate) fn native_scalar_probe_checked_with_string<E: From<ValueError>>(
        &self,
        dialect: tcl_native_model::InvocationDialect,
        kind: NativeScalarGetterKind,
        environment: Option<&dyn tcl_platform::NumericEnvironment>,
        check: &mut dyn FnMut() -> Result<(), E>,
        string: &mut dyn FnMut() -> Result<Rc<[u8]>, E>,
    ) -> Result<
        Result<NativeScalarGetterValue, tcl_syntax::scalar_getter::NativeScalarGetterFailure>,
        E,
    > {
        self.native_scalar_probe_using(
            dialect,
            kind,
            environment,
            &mut Some(check),
            &mut Some(string),
        )
    }

    fn native_scalar_probe_using<E: From<ValueError>>(
        &self,
        dialect: tcl_native_model::InvocationDialect,
        kind: NativeScalarGetterKind,
        environment: Option<&dyn tcl_platform::NumericEnvironment>,
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
        string_provider: &mut Option<&mut dyn FnMut() -> Result<Rc<[u8]>, E>>,
    ) -> Result<
        Result<NativeScalarGetterValue, tcl_syntax::scalar_getter::NativeScalarGetterFailure>,
        E,
    > {
        Self::native_stage_check(check)?;
        self.check_native_header()?;
        let (protocol, string_protocol) = self.native_scalar_probe_protocols(dialect)?;
        let jim_abi = Cell::new(None);
        let checked_jim = protocol.is_jim084() && check.is_some();
        let current = self.native_scalar_cache();
        if let (Some(environment), Some(cache)) = (environment, current.as_ref()) {
            if checked_jim {
                let context = Self::native_stage_result(self.native_jim_context(), check)?;
                context.numeric_stage_using(check, &jim_abi, |actual| {
                    tcl_cmd_core::native_numeric::scalar_getter_recipe_abi(
                        protocol,
                        kind,
                        tcl_syntax::scalar_getter::NativeScalarGetterStage::Cached(cache),
                        actual,
                    )
                })?;
            } else {
                let reached = tcl_cmd_core::native_numeric::scalar_getter_recipe_abi(
                    protocol,
                    kind,
                    tcl_syntax::scalar_getter::NativeScalarGetterStage::Cached(cache),
                    environment,
                );
                Self::native_stage_result(reached, check)?;
            }
        }
        let target = if protocol.requires_target(kind, current.as_ref()) {
            Some(if checked_jim {
                let context = Self::native_stage_result(self.native_jim_context(), check)?;
                context.scalar_getter_target_using(check, &jim_abi)?
            } else if let Some(environment) = environment {
                Self::native_stage_result(
                    tcl_cmd_core::native_numeric::scalar_getter_target(environment),
                    check,
                )?
            } else if protocol.is_jim084() {
                let context = Self::native_stage_result(self.native_jim_context(), check)?;
                if check.is_some() {
                    context.scalar_getter_target_using(check, &jim_abi)?
                } else {
                    context.scalar_getter_target()?
                }
            } else {
                return Err(ValueError::ScalarNumericInputUnavailable.into());
            })
        } else {
            None
        };
        let cached = current
            .as_ref()
            .map(|cache| protocol.cached_conversion(kind, cache, target))
            .transpose()
            .map_err(|_| ValueError::ScalarNumericInputUnavailable)?
            .flatten();
        let conversion = if let Some(conversion) = cached {
            conversion
        } else {
            self.fresh_native_scalar_conversion_using(
                NativeScalarProbeRecipe {
                    protocol,
                    kind,
                    string_protocol,
                    environment,
                    target,
                    checked_jim,
                    jim_abi: &jim_abi,
                },
                check,
                string_provider,
            )?
        };
        let (materialize, cache, outcome) = conversion.into_parts();
        if materialize {
            let reached = if let Some(provider) = string_provider {
                provider()
            } else {
                self.native_string_bytes(string_protocol)
                    .map_err(tcl_syntax::raw_string::NativeStringAccessError::Unavailable)
                    .map_err(ValueError::from)
                    .map_err(E::from)
            };
            Self::native_stage_check(check)?;
            reached?;
        }
        if let Some(cache) = cache {
            self.adopt_native_scalar_cache_using(cache, protocol, dialect, check)?;
        }
        Self::native_stage_check(check)?;
        Ok(outcome)
    }

    fn native_scalar_probe_protocols(
        &self,
        dialect: tcl_native_model::InvocationDialect,
    ) -> Result<(NativeScalarGetterProtocol, NativeStringProtocol), ValueError> {
        let protocol = dialect
            .native_scalar_getter_protocol()
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        if self
            .native_scalar_descriptor_origin()
            .is_some_and(|version| Some(version) != protocol.tcl_version())
        {
            return Err(ValueError::ScalarNumericInputUnavailable);
        }
        let string_protocol = dialect
            .native_string_protocol()
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        Ok((protocol, string_protocol))
    }

    fn fresh_native_scalar_conversion_using<E: From<ValueError>>(
        &self,
        recipe: NativeScalarProbeRecipe<'_>,
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
        string_provider: &mut Option<&mut dyn FnMut() -> Result<Rc<[u8]>, E>>,
    ) -> Result<tcl_syntax::scalar_getter::NativeScalarGetterConversion, E> {
        let NativeScalarProbeRecipe {
            protocol,
            kind,
            string_protocol,
            environment,
            target,
            checked_jim,
            jim_abi,
        } = recipe;
        Ok({
            if kind != NativeScalarGetterKind::Boolean
                && let Some(environment) = environment
            {
                if checked_jim {
                    let context = Self::native_stage_result(self.native_jim_context(), check)?;
                    context.numeric_stage_using(check, jim_abi, |actual| {
                        tcl_cmd_core::native_numeric::scalar_getter_recipe_abi(
                            protocol,
                            kind,
                            tcl_syntax::scalar_getter::NativeScalarGetterStage::Fresh,
                            actual,
                        )
                    })?;
                } else {
                    let reached = tcl_cmd_core::native_numeric::scalar_getter_recipe_abi(
                        protocol,
                        kind,
                        tcl_syntax::scalar_getter::NativeScalarGetterStage::Fresh,
                        environment,
                    );
                    Self::native_stage_result(reached, check)?;
                }
            }
            let reached = if let Some(provider) = string_provider {
                provider()
            } else {
                self.native_string_bytes(string_protocol)
                    .map_err(tcl_syntax::raw_string::NativeStringAccessError::Unavailable)
                    .map_err(ValueError::from)
                    .map_err(E::from)
            };
            Self::native_stage_check(check)?;
            let original = reached?;
            if protocol.is_jim084()
                && matches!(
                    kind,
                    NativeScalarGetterKind::Wide
                        | NativeScalarGetterKind::Long
                        | NativeScalarGetterKind::Double
                )
            {
                let context = self
                    .native_jim_context()
                    .map_err(|_| ValueError::ScalarNumericInputUnavailable)?;
                if check.is_some() {
                    context
                        .fresh_numeric_conversion_using(protocol, kind, &original, check, jim_abi)?
                } else {
                    context.fresh_numeric_conversion(protocol, kind, &original)?
                }
            } else if protocol.tcl_version() == Some(tcl_dialect::TclVersion::V8_4)
                && let Some(environment) = environment
            {
                let reached = tcl_cmd_core::native_numeric::fresh_c84_conversion(
                    protocol,
                    kind,
                    &original,
                    environment,
                );
                Self::native_stage_result(reached, check)?
            } else {
                protocol
                    .fresh_conversion_with_target(kind, &original, target)
                    .map_err(|_| ValueError::ScalarNumericInputUnavailable)?
                    .ok_or(ValueError::ScalarNumericInputUnavailable)?
            }
        })
    }

    pub(crate) fn prepare_native_expression_integer84_with_environment(
        &self,
        dialect: tcl_native_model::InvocationDialect,
        environment: Option<&dyn tcl_platform::NumericEnvironment>,
    ) -> Result<bool, ValueError> {
        self.prepare_native_expression_integer84_using::<ValueError>(
            dialect,
            environment,
            &mut None,
            &mut None,
        )
    }

    pub(crate) fn prepare_native_expression_integer84_checked<E: From<ValueError>>(
        &self,
        dialect: tcl_native_model::InvocationDialect,
        environment: Option<&dyn tcl_platform::NumericEnvironment>,
        check: &mut dyn FnMut() -> Result<(), E>,
        string: &mut dyn FnMut() -> Result<Rc<[u8]>, E>,
    ) -> Result<bool, E> {
        self.prepare_native_expression_integer84_using(
            dialect,
            environment,
            &mut Some(check),
            &mut Some(string),
        )
    }

    fn arithmetic_string_using<E: From<ValueError>>(
        &self,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
        string: &mut Option<&mut dyn FnMut() -> Result<Rc<[u8]>, E>>,
    ) -> Result<Rc<[u8]>, E> {
        Self::native_stage_check(check)?;
        let reached = if let Some(string) = string {
            string()
        } else {
            self.native_string_bytes(protocol)
                .map_err(|_| ValueError::ScalarNumericInputUnavailable.into())
        };
        Self::native_stage_check(check)?;
        reached
    }

    fn prepare_native_expression_integer84_using<E: From<ValueError>>(
        &self,
        dialect: tcl_native_model::InvocationDialect,
        environment: Option<&dyn tcl_platform::NumericEnvironment>,
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
        string: &mut Option<&mut dyn FnMut() -> Result<Rc<[u8]>, E>>,
    ) -> Result<bool, E> {
        Self::native_stage_check(check)?;
        self.check_native_header()?;
        let protocol = dialect
            .native_scalar_getter_protocol()
            .filter(|protocol| protocol.tcl_version() == Some(tcl_dialect::TclVersion::V8_4))
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        if self
            .native_scalar_descriptor_origin()
            .is_some_and(|version| version != tcl_dialect::TclVersion::V8_4)
        {
            return Err(ValueError::ScalarNumericInputUnavailable.into());
        }
        let current = self.native_scalar_cache();
        if let Some(NativeScalarCache::WordBoolean(boolean)) = current
            .as_ref()
            .filter(|_| self.resident_string_bytes().is_none())
        {
            self.adopt_native_scalar_cache_using(
                NativeScalarCache::Tcl84Long(i64::from(*boolean)),
                protocol,
                dialect,
                check,
            )?;
            Self::native_stage_check(check)?;
            return Ok(true);
        }
        if matches!(
            &current,
            Some(NativeScalarCache::Tcl84Long(_) | NativeScalarCache::Number(Number::Int(_)))
        ) {
            Self::native_stage_check(check)?;
            return Ok(true);
        }
        if matches!(
            &current,
            Some(NativeScalarCache::Number(
                Number::Double(_) | Number::Nan { .. }
            ))
        ) && self.resident_string_bytes().is_none()
        {
            Self::native_stage_check(check)?;
            return Ok(true);
        }
        let string_protocol = dialect
            .native_string_protocol()
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        let original = self.arithmetic_string_using(string_protocol, check, string)?;
        let cached_wide = current
            .as_ref()
            .map(|cache| protocol.cached_conversion(NativeScalarGetterKind::Wide, cache, None))
            .transpose()
            .map_err(|_| ValueError::ScalarNumericInputUnavailable)?
            .flatten();
        if protocol.expression_integer_spelling84(&original)
            && cached_wide.is_none()
            && let Some(environment) = environment
        {
            let reached = tcl_cmd_core::native_numeric::fresh_c84_conversion(
                protocol,
                NativeScalarGetterKind::Wide,
                &original,
                environment,
            );
            Self::native_stage_result(reached, check)?;
        }
        if let Some(conversion) =
            protocol.expression_integer_conversion84(current.as_ref(), &original)
        {
            let (_, cache, outcome) = conversion.into_parts();
            if let Some(cache) = cache {
                self.adopt_native_scalar_cache_using(cache, protocol, dialect, check)?;
            }
            Self::native_stage_check(check)?;
            return Ok(outcome.is_ok());
        }
        Self::native_stage_check(check)?;
        Ok(true)
    }

    /// C8.4's expression setter reuses an actually unshared arithmetic
    /// operand, installs a native-long primary and withdraws its string.
    pub(crate) fn store_native_expression_long84(
        &self,
        integer: i64,
        dialect: tcl_native_model::InvocationDialect,
    ) -> Result<(), ValueError> {
        self.store_native_expression_long84_using::<ValueError>(integer, dialect, &mut None)
    }

    pub(crate) fn store_native_expression_long84_checked<E: From<ValueError>>(
        &self,
        integer: i64,
        dialect: tcl_native_model::InvocationDialect,
        check: &mut dyn FnMut() -> Result<(), E>,
    ) -> Result<(), E> {
        self.store_native_expression_long84_using(integer, dialect, &mut Some(check))
    }

    fn store_native_expression_long84_using<E: From<ValueError>>(
        &self,
        integer: i64,
        dialect: tcl_native_model::InvocationDialect,
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
    ) -> Result<(), E> {
        Self::native_stage_check(check)?;
        self.check_native_header()?;
        let protocol = dialect
            .native_scalar_getter_protocol()
            .filter(|protocol| protocol.tcl_version() == Some(tcl_dialect::TclVersion::V8_4))
            .ok_or(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable)?;
        if self.native_object_is_shared() {
            return Err(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable.into());
        }
        self.adopt_native_scalar_cache_using(
            NativeScalarCache::Tcl84Long(integer),
            protocol,
            dialect,
            check,
        )?;
        *self.0.string.borrow_mut() = None;
        self.0
            .string_storage
            .set(NativeStringStorageIdentity::Unknown);
        *self.0.source_location.borrow_mut() = None;
        Self::native_stage_check(check)
    }

    pub(crate) fn store_native_increment_number(
        &self,
        number: Number,
        dialect: tcl_native_model::InvocationDialect,
    ) -> Result<(), ValueError> {
        self.store_native_increment_number_using(number, dialect, &mut None)
    }

    pub(crate) fn store_native_increment_number_checked<E: From<ValueError>>(
        &self,
        number: Number,
        dialect: tcl_native_model::InvocationDialect,
        check: &mut dyn FnMut() -> Result<(), E>,
    ) -> Result<(), E> {
        self.store_native_increment_number_using(number, dialect, &mut Some(check))
    }

    fn store_native_increment_number_using<E: From<ValueError>>(
        &self,
        number: Number,
        dialect: tcl_native_model::InvocationDialect,
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
    ) -> Result<(), E> {
        Self::native_stage_check(check)?;
        let protocol = dialect
            .native_scalar_getter_protocol()
            .filter(|protocol| protocol.supports_number_getter())
            .ok_or(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable)?;
        self.adopt_native_scalar_cache_using(
            NativeScalarCache::Number(number),
            protocol,
            dialect,
            check,
        )?;
        Self::native_stage_check(check)?;
        *self.0.string.borrow_mut() = None;
        self.0
            .string_storage
            .set(NativeStringStorageIdentity::Unknown);
        *self.0.source_location.borrow_mut() = None;
        Self::native_stage_check(check)
    }

    /// Probe C's full Number or copying Bignum getter on this original object.
    /// Numeric magnitude and category remain independent of bounded extraction.
    ///
    /// # Errors
    /// Refuses engines without the selected primitive or unavailable original storage.
    pub fn native_number_probe(
        &self,
        dialect: tcl_native_model::InvocationDialect,
        kind: tcl_syntax::scalar_getter::NativeNumberGetterKind,
    ) -> Result<Result<Number, tcl_syntax::scalar_getter::NativeScalarGetterFailure>, ValueError>
    {
        self.native_number_probe_using(dialect, kind, &mut None, &mut None)
    }

    #[cfg(test)]
    pub(crate) fn native_number_probe_checked<E: From<ValueError>>(
        &self,
        dialect: tcl_native_model::InvocationDialect,
        kind: tcl_syntax::scalar_getter::NativeNumberGetterKind,
        check: &mut dyn FnMut() -> Result<(), E>,
    ) -> Result<Result<Number, tcl_syntax::scalar_getter::NativeScalarGetterFailure>, E> {
        self.native_number_probe_using(dialect, kind, &mut Some(check), &mut None)
    }

    pub(crate) fn native_number_probe_checked_with_string<E: From<ValueError>>(
        &self,
        dialect: tcl_native_model::InvocationDialect,
        kind: tcl_syntax::scalar_getter::NativeNumberGetterKind,
        check: &mut dyn FnMut() -> Result<(), E>,
        string: &mut dyn FnMut() -> Result<Rc<[u8]>, E>,
    ) -> Result<Result<Number, tcl_syntax::scalar_getter::NativeScalarGetterFailure>, E> {
        self.native_number_probe_using(dialect, kind, &mut Some(check), &mut Some(string))
    }

    fn native_number_probe_using<E: From<ValueError>>(
        &self,
        dialect: tcl_native_model::InvocationDialect,
        kind: tcl_syntax::scalar_getter::NativeNumberGetterKind,
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
        string_provider: &mut Option<&mut dyn FnMut() -> Result<Rc<[u8]>, E>>,
    ) -> Result<Result<Number, tcl_syntax::scalar_getter::NativeScalarGetterFailure>, E> {
        Self::native_stage_check(check)?;
        self.check_native_header()?;
        let protocol = dialect
            .native_scalar_getter_protocol()
            .filter(|protocol| protocol.supports_number_getter())
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        if self
            .native_scalar_descriptor_origin()
            .is_some_and(|version| Some(version) != protocol.tcl_version())
        {
            return Err(ValueError::ScalarNumericInputUnavailable.into());
        }
        let current = self.native_scalar_cache();
        let cached = if kind == tcl_syntax::scalar_getter::NativeNumberGetterKind::IncrementNumber {
            protocol.increment_number_preflight(
                current.as_ref(),
                self.resident_string_bytes().map(|bytes| bytes.len()),
                matches!(*self.0.intrep.borrow(), IntRep::Str),
            )
        } else {
            current
                .as_ref()
                .and_then(|cache| protocol.cached_number_conversion(kind, cache))
        };
        let conversion = if let Some(conversion) = cached {
            conversion
        } else {
            let string_protocol = dialect
                .native_string_protocol()
                .ok_or(ValueError::ScalarNumericInputUnavailable)?;
            let reached = if let Some(provider) = string_provider {
                provider()
            } else {
                self.native_string_bytes(string_protocol)
                    .map_err(|_| ValueError::ScalarNumericInputUnavailable.into())
            };
            Self::native_stage_check(check)?;
            let original = reached?;
            protocol
                .fresh_number_conversion(kind, &original)
                .ok_or(ValueError::ScalarNumericInputUnavailable)?
        };
        let (cache, outcome) = conversion.into_parts();
        if let Some(cache) = cache {
            self.adopt_native_scalar_cache_using(cache, protocol, dialect, check)?;
        }
        Self::native_stage_check(check)?;
        Ok(outcome)
    }

    /// Execute an actual primitive getter and render its reached guest failure.
    /// Interpreter publication belongs to the consuming command adapter.
    ///
    /// # Errors
    /// Returns the exact primitive guest receipt or a typed capability refusal.
    pub fn native_scalar_getter(
        &self,
        dialect: tcl_native_model::InvocationDialect,
        kind: NativeScalarGetterKind,
    ) -> Result<NativeScalarGetterValue, tcl_syntax::value::ValueError> {
        self.native_scalar_getter_with_environment(dialect, kind, None)
    }

    pub(crate) fn native_scalar_getter_with_environment(
        &self,
        dialect: tcl_native_model::InvocationDialect,
        kind: NativeScalarGetterKind,
        environment: Option<&dyn tcl_platform::NumericEnvironment>,
    ) -> Result<NativeScalarGetterValue, ValueError> {
        self.native_scalar_getter_using(dialect, kind, environment, &mut None, &mut None)
    }

    pub(crate) fn native_scalar_getter_checked_with_string<E: From<ValueError>>(
        &self,
        dialect: tcl_native_model::InvocationDialect,
        kind: NativeScalarGetterKind,
        environment: Option<&dyn tcl_platform::NumericEnvironment>,
        check: &mut dyn FnMut() -> Result<(), E>,
        string: &mut dyn FnMut() -> Result<Rc<[u8]>, E>,
    ) -> Result<NativeScalarGetterValue, E> {
        self.native_scalar_getter_using(
            dialect,
            kind,
            environment,
            &mut Some(check),
            &mut Some(string),
        )
    }

    fn native_scalar_getter_using<E: From<ValueError>>(
        &self,
        dialect: tcl_native_model::InvocationDialect,
        kind: NativeScalarGetterKind,
        environment: Option<&dyn tcl_platform::NumericEnvironment>,
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
        string_provider: &mut Option<&mut dyn FnMut() -> Result<Rc<[u8]>, E>>,
    ) -> Result<NativeScalarGetterValue, E> {
        match self.native_scalar_probe_using(dialect, kind, environment, check, string_provider)? {
            Ok(value) => Ok(value),
            Err(failure) => {
                let record = self.native_scalar_failure_presentation_using(
                    dialect,
                    kind,
                    failure,
                    check,
                    string_provider,
                )?;
                Err(ValueError::NativeScalarGetter(Box::new(record)).into())
            }
        }
    }

    /// Render an already reached primitive failure on this original header.
    /// No second conversion or context-free String access is performed.
    #[cfg(test)]
    pub(crate) fn native_scalar_failure_presentation(
        &self,
        dialect: tcl_native_model::InvocationDialect,
        kind: NativeScalarGetterKind,
        failure: tcl_syntax::scalar_getter::NativeScalarGetterFailure,
    ) -> Result<tcl_syntax::scalar_getter::NativeScalarGetterError, ValueError> {
        self.native_scalar_failure_presentation_using(dialect, kind, failure, &mut None, &mut None)
    }

    pub(crate) fn native_scalar_failure_presentation_checked_with_string<E: From<ValueError>>(
        &self,
        dialect: tcl_native_model::InvocationDialect,
        kind: NativeScalarGetterKind,
        failure: tcl_syntax::scalar_getter::NativeScalarGetterFailure,
        check: &mut dyn FnMut() -> Result<(), E>,
        string: &mut dyn FnMut() -> Result<Rc<[u8]>, E>,
    ) -> Result<tcl_syntax::scalar_getter::NativeScalarGetterError, E> {
        self.native_scalar_failure_presentation_using(
            dialect,
            kind,
            failure,
            &mut Some(check),
            &mut Some(string),
        )
    }

    fn native_scalar_failure_presentation_using<E: From<ValueError>>(
        &self,
        dialect: tcl_native_model::InvocationDialect,
        kind: NativeScalarGetterKind,
        failure: tcl_syntax::scalar_getter::NativeScalarGetterFailure,
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
        string_provider: &mut Option<&mut dyn FnMut() -> Result<Rc<[u8]>, E>>,
    ) -> Result<tcl_syntax::scalar_getter::NativeScalarGetterError, E> {
        Self::native_stage_check(check)?;
        self.check_native_header()?;
        let protocol = dialect
            .native_scalar_getter_protocol()
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        let requires_string = protocol
            .failure_requires_original_string(kind, failure)
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        let record = if requires_string {
            let string = dialect
                .native_string_protocol()
                .ok_or(ValueError::ScalarNumericInputUnavailable)?;
            let reached = if let Some(provider) = string_provider {
                provider()
            } else {
                self.native_string_bytes(string)
                    .map_err(tcl_syntax::raw_string::NativeStringAccessError::Unavailable)
                    .map_err(ValueError::from)
                    .map_err(E::from)
            };
            Self::native_stage_check(check)?;
            let original = reached?;
            protocol.failure_presentation(kind, failure, &original)
        } else {
            protocol.failure_presentation_without_original_string(kind, failure)
        };
        Self::native_stage_check(check)?;
        record
            .ok_or(ValueError::ScalarNumericInputUnavailable)
            .map_err(Into::into)
    }
}
