// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original-object integer updates with full Number/Bignum probe ordering.

use crate::CmdError;
use tcl_syntax::number::Number;
use tcl_syntax::scalar_getter::{NativeNumberGetterKind, NativeScalarGetterFailure};

/// Add accepted integer payloads without constructing a temporary Tcl object.
/// The backend supplies exact arbitrary-precision arithmetic; this owner keeps
/// wide addition and promotion identical across object adapters.
///
/// # Errors
/// Rejects noninteger payloads and preserves backend capability refusals.
pub fn add_integer_numbers(
    current: Number,
    amount: Number,
    full_add: impl FnOnce(Number, Number) -> Result<Number, CmdError>,
) -> Result<Number, CmdError> {
    if !matches!(current, Number::Int(_) | Number::Big { .. })
        || !matches!(amount, Number::Int(_) | Number::Big { .. })
    {
        return Err(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable.into());
    }
    if let (Number::Int(current), Number::Int(amount)) = (&current, &amount)
        && let Some(sum) = current.checked_add(*amount)
    {
        return Ok(Number::Int(sum));
    }
    full_add(current, amount)
}

/// Backend operations on one already selected physical increment receiver.
/// Implementations retain the selected actual engine independently of source grammar.
pub trait NativeIncrementObjects {
    /// Original physical object handle.
    type Value: Clone;
    /// Receiver after its original sharing check, before numeric conversion.
    type Prepared;

    /// Apply receiver COW before taking any working getter reference.
    fn prepare(&self, original: Option<&Self::Value>) -> Result<Self::Prepared, CmdError>;
    /// Borrow the prepared current object without changing its sharing observation.
    fn current<'a>(&self, prepared: &'a Self::Prepared) -> &'a Self::Value;
    /// Probe the selected original numeric primitive, applying cache effects on failure.
    fn probe(
        &self,
        value: &Self::Value,
        kind: NativeNumberGetterKind,
    ) -> Result<Result<Number, NativeScalarGetterFailure>, CmdError>;
    /// Run the actual integer failure presenter after the Number probe.
    fn integer_failure(
        &self,
        value: &Self::Value,
        kind: NativeNumberGetterKind,
    ) -> Result<CmdError, CmdError>;
    /// Add full integer magnitudes using the selected arithmetic owner.
    fn add(&self, current: Number, amount: Number) -> Result<Number, CmdError>;
    /// Replace the prepared object's integer payload and invalidate its resident string.
    fn store(&self, prepared: &mut Self::Prepared, sum: Number) -> Result<(), CmdError>;
    /// Transfer the prepared original object after its mutation.
    fn finish(&self, prepared: Self::Prepared) -> Self::Value;
}

fn reading_increment(error: CmdError) -> CmdError {
    if error.native_access_refusal().is_some() {
        return error;
    }
    let mut details = error.into_byte_details();
    let info = details
        .error_info
        .get_or_insert_with(|| details.message.clone());
    info.extend_from_slice(b"\n    (reading increment)");
    CmdError::from_byte_details(details)
}

fn number<O: NativeIncrementObjects>(ops: &O, value: &O::Value) -> Result<Number, CmdError> {
    match ops.probe(value, NativeNumberGetterKind::IncrementNumber)? {
        Ok(number) => Ok(number),
        Err(_) => Err(ops.integer_failure(value, NativeNumberGetterKind::IncrementNumber)?),
    }
}

/// Increment one physical receiver using C's full-number probe order.
/// A missing ordinary receiver starts at zero. Dictionary missing-member
/// construction is separately selected by [`missing_dictionary_member`].
///
/// # Errors
/// Retains native cache effects, guest integer diagnostics and outer host refusals.
pub fn increment<O: NativeIncrementObjects>(
    ops: &O,
    original: Option<&O::Value>,
    amount: &O::Value,
) -> Result<O::Value, CmdError> {
    let mut prepared = ops.prepare(original)?;
    let current = number(ops, ops.current(&prepared))?;
    let amount_number = number(ops, amount).map_err(reading_increment)?;
    if !matches!(current, Number::Int(_) | Number::Big { .. }) {
        return Err(ops.integer_failure(
            ops.current(&prepared),
            NativeNumberGetterKind::IncrementNumber,
        )?);
    }
    if !matches!(amount_number, Number::Int(_) | Number::Big { .. }) {
        return Err(reading_increment(ops.integer_failure(
            amount,
            NativeNumberGetterKind::IncrementNumber,
        )?));
    }
    let sum = ops.add(current, amount_number)?;
    ops.store(&mut prepared, sum)?;
    Ok(ops.finish(prepared))
}

/// Validate a generic dictionary increment and retain the exact original amount.
/// The copying Bignum getter's fresh integer-only parse differs from `GetNumber`.
///
/// # Errors
/// Preserves the actual Bignum failure and the reading-increment error trace.
pub fn missing_dictionary_member<O: NativeIncrementObjects>(
    ops: &O,
    amount: &O::Value,
) -> Result<O::Value, CmdError> {
    match ops.probe(amount, NativeNumberGetterKind::Bignum)? {
        Ok(_) => Ok(amount.clone()),
        Err(_) => Err(reading_increment(
            ops.integer_failure(amount, NativeNumberGetterKind::Bignum)?,
        )),
    }
}

/// Original amount conversion before variable lookup or read callbacks.
pub trait LegacyIncrementAmountOps {
    /// Original operand handle, never replaced to obtain its numeric acceptance.
    type Value;
    /// Actual C8.4 `GetLong` conversion with original cache/error effects.
    fn c84_long(&mut self, original: &Self::Value) -> Result<i64, CmdError>;
    /// Actual Jim `GetWideExpr` safe-expression conversion on the original object.
    fn jim_wide_expression(&mut self, original: &Self::Value) -> Result<i64, CmdError>;
}

/// Accepted original amount, without a manufactured object reference.
#[derive(Debug, Clone, Copy)]
pub struct PreparedLegacyIncrementAmount {
    recipe: tcl_syntax::scalar_getter::NativeLegacyIncrementRecipe,
    value: i64,
}

/// Prepare the amount at its native pre-lookup frontier.
///
/// # Errors
/// Preserves original primitive cache effects, guest failures and host refusals.
pub fn prepare_legacy_amount<O: LegacyIncrementAmountOps>(
    ops: &mut O,
    recipe: tcl_syntax::scalar_getter::NativeLegacyIncrementRecipe,
    original: &O::Value,
) -> Result<PreparedLegacyIncrementAmount, CmdError> {
    use tcl_syntax::scalar_getter::NativeLegacyIncrementRecipe as Recipe;
    let value = match recipe {
        Recipe::Tcl84 => ops.c84_long(original).map_err(reading_increment)?,
        Recipe::Jim084 => ops.jim_wide_expression(original)?,
    };
    Ok(PreparedLegacyIncrementAmount { recipe, value })
}

/// Native implicit increment is one, with no manufactured operand object.
#[must_use]
pub fn default_legacy_amount(
    recipe: tcl_syntax::scalar_getter::NativeLegacyIncrementRecipe,
) -> PreparedLegacyIncrementAmount {
    PreparedLegacyIncrementAmount { recipe, value: 1 }
}

/// Physical operations after the selected legacy receiver has been read.
pub trait LegacyIncrementObjects {
    /// Original native object handle.
    type Value;
    /// Working object after its reached COW decision.
    type Prepared;
    /// Independently selected actual handler recipe of this object adapter.
    fn recipe(&self) -> tcl_syntax::scalar_getter::NativeLegacyIncrementRecipe;
    /// Inspect the primary cache without string or numeric conversion.
    fn cache(
        &self,
        value: &Self::Value,
    ) -> Result<Option<tcl_syntax::scalar_getter::NativeScalarCache>, CmdError>;
    /// Actual Wide getter on the reached original/working object.
    fn wide(&self, value: &Self::Value) -> Result<i64, CmdError>;
    /// Duplicate only when the borrowed original is actually shared.
    fn prepare(&self, value: Option<&Self::Value>) -> Result<Self::Prepared, CmdError>;
    /// Borrow the prepared payload without adding a reference.
    fn current<'a>(&self, prepared: &'a Self::Prepared) -> &'a Self::Value;
    /// Publish the selected Long or Wide cache and invalidate resident storage.
    fn store(
        &self,
        prepared: &mut Self::Prepared,
        cache: tcl_syntax::scalar_getter::NativeScalarCache,
    ) -> Result<(), CmdError>;
    /// Transfer the working original object after mutation.
    fn finish(&self, prepared: Self::Prepared) -> Self::Value;
}

/// Apply legacy native increment to an already-read receiver.
/// C8.4 copies before current conversion; Jim converts current before copying.
/// Native signed endpoint arithmetic wraps and preserves C8.4 Long/Wide class.
///
/// # Errors
/// Missing C8.4 contents must be rejected by the physical read owner. Numeric
/// failures and fatal/capability refusals retain their separate channels.
pub fn increment_legacy<O: LegacyIncrementObjects>(
    ops: &O,
    original: Option<&O::Value>,
    amount: PreparedLegacyIncrementAmount,
) -> Result<O::Value, CmdError> {
    use tcl_syntax::scalar_getter::{
        NativeLegacyIncrementRecipe as Recipe, NativeScalarCache as Cache,
    };
    if ops.recipe() != amount.recipe {
        return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "legacy increment amount belongs to another native handler",
        )
        .into());
    }
    let (mut prepared, current, long) = match amount.recipe {
        Recipe::Tcl84 => {
            let original = original.ok_or_else(|| {
                CmdError::from(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "C8.4 increment missing read receipt",
                ))
            })?;
            let prepared = ops.prepare(Some(original))?;
            let (current, long) = match ops.cache(ops.current(&prepared))? {
                Some(Cache::Tcl84Long(value)) => (value, true),
                Some(Cache::Number(Number::Int(value))) => (value, false),
                _ => (ops.wide(ops.current(&prepared))?, true),
            };
            (prepared, current, long)
        }
        Recipe::Jim084 => {
            let current = original
                .map(|value| ops.wide(value))
                .transpose()?
                .unwrap_or(0);
            (ops.prepare(original)?, current, false)
        }
    };
    let sum = current.wrapping_add(amount.value);
    let cache = if long {
        Cache::Tcl84Long(sum)
    } else {
        Cache::Number(Number::Int(sum))
    };
    ops.store(&mut prepared, cache)?;
    Ok(ops.finish(prepared))
}
