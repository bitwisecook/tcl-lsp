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

//! The `incr` adapters over original objects: the legacy C 8.4 and Jim owners, their amount getters, and the modern integer update.

#[cfg(have_tommath)]
use super::integer_object;
use super::{IncrementContext, RuntimeAppendValue, RuntimeIncrementObjects};
use crate::interp::Interp;
use crate::obj;
use crate::obj::TclObj;
use std::rc::Rc;
use tcl_registry::native_rmw::LegacyIncrementDialect;
use tcl_syntax::number::Number;
use tcl_syntax::value::ValueError;

/// Actual C8.4/Jim original-object increment, with native probe/COW order.
pub(crate) struct RuntimeLegacyIncrementObjects {
    pub(super) dialect: tcl_native_model::InvocationDialect,
    recipe: tcl_syntax::scalar_getter::NativeLegacyIncrementRecipe,
    jim_context: Option<Rc<crate::native_source::NativeJimObjectContext>>,
    pub(super) context: IncrementContext,
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

impl RuntimeIncrementObjects {
    pub(crate) fn with_cached_sum(mut self, cached_sum: Option<(i64, i64)>) -> Self {
        self.cached_sum = cached_sum;
        self
    }

    pub(crate) fn selected(interp: &Interp) -> Result<Self, tcl_cmd_core::CmdError> {
        let context = IncrementContext::selected(interp)?;
        let dialect = interp.native_invocation_dialect();
        dialect
            .native_scalar_getter_protocol()
            .filter(|protocol| protocol.supports_number_getter())
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        Ok(Self {
            dialect,
            context,
            cached_sum: None,
        })
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
        let cached = self.cached_sum.filter(|&(old, sum)| {
            matches!((&current, &amount), (Number::Int(actual), Number::Int(1))
                if *actual == old && actual.checked_add(1) == Some(sum))
        });
        if let Some((_, sum)) = cached {
            self.context.check()?;
            return Ok(Number::Int(sum));
        }
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
