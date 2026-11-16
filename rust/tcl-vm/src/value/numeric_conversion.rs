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

//! The value as a number or boolean: `as_int`, `as_wide`, `as_i128`, `as_double`, `as_bool` and the native integer and boolean getters.

use super::{IntRep, Value};
use crate::error::TclError;
use std::rc::Rc;
use tcl_syntax::number::Number;
use tcl_syntax::value::ValueError;
use tcl_syntax::{list, number};

impl Value {
    fn numeric_conversion_current(
        &self,
        original: Option<crate::expr::NumericContext<'_>>,
    ) -> Result<(), TclError> {
        original.map_or(Ok(()), |context| context.current_for_value(self))
    }

    pub(crate) fn numeric_conversion_parse(
        text: &str,
        original: Option<crate::expr::NumericContext<'_>>,
    ) -> Option<Number> {
        match original {
            Some(context) => {
                number::parse_whole_with(text, number::ParseFlags::for_syntax(context.numbers))
            }
            None => number::parse_whole(text),
        }
    }

    pub(crate) fn numeric_conversion_text(
        &self,
        original: Option<crate::expr::NumericContext<'_>>,
        invalid: fn(Vec<u8>) -> ValueError,
    ) -> Result<Rc<str>, TclError> {
        let Some(context) = original else {
            return self
                .try_to_str()
                .map_err(|_| invalid(self.string_bytes().to_vec()).into());
        };
        context.current_for_value(self)?;
        let reached = context.string_for_value(self);
        context.current_for_value(self)?;
        let bytes = reached?;
        std::str::from_utf8(&bytes)
            .map(Rc::from)
            .map_err(|_| invalid(bytes.to_vec()).into())
    }

    pub(crate) fn numeric_conversion_optional_text(
        &self,
        original: Option<crate::expr::NumericContext<'_>>,
    ) -> Result<Option<Rc<str>>, TclError> {
        if original.is_none() {
            return Ok(self.try_to_str().ok());
        }
        match self.numeric_conversion_text(original, ValueError::NotIntegerBytes) {
            Ok(text) => Ok(Some(text)),
            Err(error @ TclError::Host(_)) => Err(error),
            Err(TclError::Guest(_)) => Ok(None),
        }
    }

    /// The value as a wide integer (`Tcl_GetWideIntFromObj`), caching the typed
    /// rep when it parses from the string side.
    pub fn as_int(&self) -> Result<i64, TclError> {
        self.as_int_with_stages(None)
    }

    pub(crate) fn as_int_with_stages(
        &self,
        original: Option<crate::expr::NumericContext<'_>>,
    ) -> Result<i64, TclError> {
        self.numeric_conversion_current(original)?;
        if let Some(value) = self.restore_coerced_integer() {
            self.numeric_conversion_current(original)?;
            return Ok(value);
        }
        match &*self.0.intrep.borrow() {
            IntRep::Int(n) | IntRep::Tcl84Long(n) | IntRep::Tcl84Wide(n) => return Ok(*n),
            IntRep::Bool(b) => return Ok(i64::from(*b)),
            IntRep::Str
            | IntRep::JimString(_)
            | IntRep::NativeString { .. }
            | IntRep::ByteArray(_)
            | IntRep::Big { .. }
            | IntRep::WordBoolean { .. }
            | IntRep::CompletionCode(_)
            | IntRep::NativeArraySearch { .. }
            | IntRep::FrameLevel { .. }
            | IntRep::NativePropertyName(_)
            | IntRep::NativeInstructionName(_)
            | IntRep::NativeMethodName(_)
            | IntRep::NativeRegexp(_)
            | IntRep::JimRegexp(_)
            | IntRep::JimIndex(_)
            | IntRep::NativeEndOffset(_)
            | IntRep::NativeIndex { .. }
            | IntRep::NativeCommandName(_)
            | IntRep::NativeCommandNameUnresolved(_)
            | IntRep::JimCommand(_)
            | IntRep::JimVariable(_)
            | IntRep::JimOption(_)
            | IntRep::JimSubcommand(_)
            | IntRep::NativeParsedVariableName(_)
            | IntRep::NativeLocalVariableName(_)
            | IntRep::NativeNamespaceName(_)
            | IntRep::JimSource(_)
            | IntRep::JimScript(_)
            | IntRep::JimDictionarySubstitution { .. }
            | IntRep::JimInterpolated(_)
            | IntRep::JimScriptLine { .. }
            | IntRep::Expression(_)
            | IntRep::NativeBytecode(_)
            | IntRep::Double(_)
            | IntRep::CoercedDouble(_)
            | IntRep::List { .. }
            | IntRep::Dict(_) => {}
        }
        let s = self.numeric_conversion_text(original, ValueError::NotIntegerBytes)?;
        match Self::numeric_conversion_parse(&s, original) {
            Some(Number::Int(n)) => {
                if let Some(original) = original {
                    original.cache(self, Number::Int(n))?;
                } else {
                    *self.0.intrep.borrow_mut() = IntRep::Int(n);
                }
                self.numeric_conversion_current(original)?;
                Ok(n)
            }
            // Every C release reads a bignum spelling as an overflow.
            Some(Number::Big { .. }) => Err(ValueError::IntegerOverflow.into()),
            _ => Err(TclError::new(format!(
                "expected integer but got {}",
                list::describe_bad_value(&s)
            ))),
        }
    }

    /// Convert the original object under an explicit native input extent and
    /// numeral grammar. Successful conversion changes this object's intrep;
    /// the complete original string, including bytes after NUL, is retained.
    pub(crate) fn native_int(
        &self,
        input: number::NativeScalarNumericInputPolicy,
        syntax: tcl_dialect::NumberSyntax,
    ) -> Result<i64, ValueError> {
        self.native_int_using::<ValueError>(input, syntax, &mut None, &mut None)
    }

    pub(crate) fn native_int_checked<E: From<ValueError>>(
        &self,
        input: number::NativeScalarNumericInputPolicy,
        syntax: tcl_dialect::NumberSyntax,
        check: &mut dyn FnMut() -> Result<(), E>,
        string: &mut dyn FnMut() -> Result<Rc<[u8]>, E>,
    ) -> Result<i64, E> {
        self.native_int_using(input, syntax, &mut Some(check), &mut Some(string))
    }

    fn native_int_using<E: From<ValueError>>(
        &self,
        input: number::NativeScalarNumericInputPolicy,
        syntax: tcl_dialect::NumberSyntax,
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
        string: &mut Option<&mut dyn FnMut() -> Result<Rc<[u8]>, E>>,
    ) -> Result<i64, E> {
        Self::native_stage_check(check)?;
        if let Some(value) = self.restore_coerced_integer() {
            Self::native_stage_check(check)?;
            return Ok(value);
        }
        match *self.0.intrep.borrow() {
            IntRep::Int(value) | IntRep::Tcl84Long(value) | IntRep::Tcl84Wide(value) => {
                return Ok(value);
            }
            IntRep::Bool(value) => return Ok(i64::from(value)),
            _ => {}
        }
        let reached = if let Some(string) = string {
            string()
        } else {
            Ok(self.string_bytes())
        };
        Self::native_stage_check(check)?;
        let original = reached?;
        let text = std::str::from_utf8(input.input_bytes(&original))
            .map_err(|_| ValueError::NotIntegerBytes(original.to_vec()))?;
        let mut flags = number::ParseFlags::for_syntax(syntax);
        flags.integer_only = true;
        match number::parse_whole_with(text, flags) {
            Some(Number::Int(value)) => {
                self.cache_numeric_representation_using(Number::Int(value), check, string)?;
                Ok(value)
            }
            _ => Err(ValueError::NotIntegerBytes(original.to_vec()).into()),
        }
    }

    /// Native boolean getter. Jim accepts its boolean words after the scalar
    /// C-string boundary; expression numeric truth conversion is a separate door.
    pub(crate) fn native_bool(
        &self,
        input: number::NativeScalarNumericInputPolicy,
        syntax: tcl_dialect::NumberSyntax,
    ) -> Result<bool, ValueError> {
        self.native_bool_using::<ValueError>(input, syntax, &mut None, &mut None)
    }

    pub(crate) fn native_bool_checked<E: From<ValueError>>(
        &self,
        input: number::NativeScalarNumericInputPolicy,
        syntax: tcl_dialect::NumberSyntax,
        check: &mut dyn FnMut() -> Result<(), E>,
        string: &mut dyn FnMut() -> Result<Rc<[u8]>, E>,
    ) -> Result<bool, E> {
        self.native_bool_using(input, syntax, &mut Some(check), &mut Some(string))
    }

    fn native_bool_using<E: From<ValueError>>(
        &self,
        input: number::NativeScalarNumericInputPolicy,
        syntax: tcl_dialect::NumberSyntax,
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
        string: &mut Option<&mut dyn FnMut() -> Result<Rc<[u8]>, E>>,
    ) -> Result<bool, E> {
        Self::native_stage_check(check)?;
        match *self.0.intrep.borrow() {
            IntRep::Int(value) | IntRep::Tcl84Long(value) | IntRep::Tcl84Wide(value) => {
                return Ok(value != 0);
            }
            IntRep::Bool(value) => return Ok(value),
            IntRep::Double(value)
                if input == number::NativeScalarNumericInputPolicy::LengthDelimited =>
            {
                return Ok(value != 0.0);
            }
            _ => {}
        }
        let reached = if let Some(string) = string {
            string()
        } else {
            Ok(self.string_bytes())
        };
        Self::native_stage_check(check)?;
        let original = reached?;
        let text = std::str::from_utf8(input.input_bytes(&original))
            .map_err(|_| ValueError::NotBooleanBytes(original.to_vec()))?;
        if input == number::NativeScalarNumericInputPolicy::LengthDelimited
            && let Some(number) =
                number::parse_whole_with(text, number::ParseFlags::for_syntax(syntax))
        {
            return match number {
                Number::Int(value) => Ok(value != 0),
                Number::Double(value) => Ok(value != 0.0),
                Number::Big { .. } => Ok(true),
                Number::Nan { .. } => Err(ValueError::NotBooleanBytes(original.to_vec()).into()),
            };
        }
        let value = tcl_syntax::boolean::parse_boolean_word(text)
            .ok_or_else(|| ValueError::NotBooleanBytes(original.to_vec()))?;
        if input == number::NativeScalarNumericInputPolicy::NulTerminatedJim084 {
            self.cache_numeric_representation_using(Number::Int(i64::from(value)), check, string)?;
        }
        Ok(value)
    }

    /// The value as a C `int` (`Tcl_GetIntFromObj`): [`Self::as_int`] narrowed
    /// as C narrows it ([`tcl_syntax::value::narrow_to_c_int`]), so
    /// `4294967295` reads -1 and `4294967296` is the overflow.
    pub fn as_c_int(&self) -> Result<i32, TclError> {
        tcl_syntax::value::narrow_to_c_int(self.as_int()?).map_err(|_| {
            TclError::with_error_code(
                tcl_syntax::expr::errors::IOVERFLOW_MESSAGE,
                tcl_syntax::expr::errors::IOVERFLOW_CODE,
            )
        })
    }

    /// The value truncated to a 64-bit wide (`wide()`): an in-range integer
    /// as-is, else a bignum literal reduced modulo 2^64 and bit-cast to `i64`
    /// (two's-complement wrap — what C's `wide()` does, e.g.
    /// `wide(0x8000000000000000)` is `-9223372036854775808`). The VM has no
    /// arbitrary-precision rep, so this is the truncating window onto one.
    pub fn as_wide(&self) -> Result<i64, TclError> {
        self.as_wide_with_stages(None)
    }

    pub(crate) fn as_wide_with_stages(
        &self,
        original: Option<crate::expr::NumericContext<'_>>,
    ) -> Result<i64, TclError> {
        self.numeric_conversion_current(original)?;
        let reached = self.as_int_with_stages(original);
        self.numeric_conversion_current(original)?;
        match reached {
            Ok(n) => return Ok(n),
            Err(error @ TclError::Host(_)) => return Err(error),
            Err(TclError::Guest(_)) => {}
        }
        let s = self.numeric_conversion_text(original, ValueError::NotIntegerBytes)?;
        if let Some(Number::Big {
            negative,
            radix,
            digits,
        }) = Self::numeric_conversion_parse(&s, original)
        {
            let base = radix as u32;
            let mut acc: u64 = 0;
            for ch in digits.chars() {
                let d = ch.to_digit(base).unwrap_or(0);
                acc = acc.wrapping_mul(u64::from(base)).wrapping_add(u64::from(d));
            }
            let signed = acc.cast_signed();
            return Ok(if negative {
                signed.wrapping_neg()
            } else {
                signed
            });
        }
        Err(TclError::new(format!("expected integer but got \"{s}\"")))
    }

    /// The value as a 128-bit integer, parsing an out-of-`i64`-range integer
    /// literal whose magnitude fits `i128`. This is the VM's bounded stand-in
    /// for Tcl's arbitrary-precision integers: it covers the common large-value
    /// range (`2**70`, `10**21`, …) so `expr`/`mathop` arithmetic promotes on
    /// `i64` overflow instead of wrapping. `None` for non-integers or magnitudes
    /// beyond `i128`.
    #[must_use]
    pub fn as_i128(&self) -> Option<i128> {
        self.as_i128_with_stages(None).ok().flatten()
    }

    pub(crate) fn as_i128_with_stages(
        &self,
        original: Option<crate::expr::NumericContext<'_>>,
    ) -> Result<Option<i128>, TclError> {
        self.numeric_conversion_current(original)?;
        let reached = self.as_int_with_stages(original);
        self.numeric_conversion_current(original)?;
        match reached {
            Ok(n) => return Ok(Some(i128::from(n))),
            Err(error @ TclError::Host(_)) => return Err(error),
            Err(TclError::Guest(_)) => {}
        }
        let Some(s) = self.numeric_conversion_optional_text(original)? else {
            return Ok(None);
        };
        if let Some(Number::Big {
            negative,
            radix,
            digits,
        }) = Self::numeric_conversion_parse(&s, original)
        {
            let base = radix as u32;
            let mut acc: u128 = 0;
            for ch in digits.chars() {
                let Some(d) = ch.to_digit(base) else {
                    return Ok(None);
                };
                let Some(next) = acc
                    .checked_mul(u128::from(base))
                    .and_then(|acc| acc.checked_add(u128::from(d)))
                else {
                    return Ok(None);
                };
                acc = next;
            }
            let Ok(mag) = i128::try_from(acc) else {
                return Ok(None);
            };
            return Ok(Some(if negative { -mag } else { mag }));
        }
        Ok(None)
    }

    /// The value as a double (`Tcl_GetDoubleFromObj`).
    pub fn as_double(&self) -> Result<f64, TclError> {
        self.as_double_with_stages(None)
    }

    pub(crate) fn as_double_with_stages(
        &self,
        original: Option<crate::expr::NumericContext<'_>>,
    ) -> Result<f64, TclError> {
        self.numeric_conversion_current(original)?;
        match &*self.0.intrep.borrow() {
            IntRep::Int(n)
            | IntRep::Tcl84Long(n)
            | IntRep::Tcl84Wide(n)
            | IntRep::CoercedDouble(n) => {
                return Ok(*n as f64);
            }
            IntRep::Double(f) => return Ok(*f),
            IntRep::Bool(b) => return Ok(f64::from(i32::from(*b))),
            IntRep::Str
            | IntRep::JimString(_)
            | IntRep::NativeString { .. }
            | IntRep::ByteArray(_)
            | IntRep::Big { .. }
            | IntRep::WordBoolean { .. }
            | IntRep::CompletionCode(_)
            | IntRep::NativeArraySearch { .. }
            | IntRep::FrameLevel { .. }
            | IntRep::NativePropertyName(_)
            | IntRep::NativeInstructionName(_)
            | IntRep::NativeMethodName(_)
            | IntRep::NativeRegexp(_)
            | IntRep::JimRegexp(_)
            | IntRep::JimIndex(_)
            | IntRep::NativeEndOffset(_)
            | IntRep::NativeIndex { .. }
            | IntRep::NativeCommandName(_)
            | IntRep::NativeCommandNameUnresolved(_)
            | IntRep::JimCommand(_)
            | IntRep::JimVariable(_)
            | IntRep::JimOption(_)
            | IntRep::JimSubcommand(_)
            | IntRep::NativeParsedVariableName(_)
            | IntRep::NativeLocalVariableName(_)
            | IntRep::NativeNamespaceName(_)
            | IntRep::JimSource(_)
            | IntRep::JimScript(_)
            | IntRep::JimDictionarySubstitution { .. }
            | IntRep::JimInterpolated(_)
            | IntRep::JimScriptLine { .. }
            | IntRep::Expression(_)
            | IntRep::NativeBytecode(_)
            | IntRep::List { .. }
            | IntRep::Dict(_) => {}
        }
        let s = self.numeric_conversion_text(original, ValueError::NotDoubleBytes)?;
        match Self::numeric_conversion_parse(&s, original) {
            Some(Number::Int(n)) => Ok(n as f64),
            Some(Number::Double(f)) => Ok(f),
            _ => Err(TclError::new(format!(
                "expected floating-point number but got {}",
                list::describe_bad_value(&s)
            ))),
        }
    }

    /// The value as a boolean (`Tcl_GetBooleanFromObj`).
    pub fn as_bool(&self) -> Result<bool, TclError> {
        self.as_bool_with_stages(None)
    }

    pub(crate) fn as_bool_with_stages(
        &self,
        original: Option<crate::expr::NumericContext<'_>>,
    ) -> Result<bool, TclError> {
        self.numeric_conversion_current(original)?;
        match &*self.0.intrep.borrow() {
            IntRep::Bool(b) => return Ok(*b),
            IntRep::Int(n)
            | IntRep::Tcl84Long(n)
            | IntRep::Tcl84Wide(n)
            | IntRep::CoercedDouble(n) => return Ok(*n != 0),
            IntRep::Double(f) => return Ok(*f != 0.0),
            IntRep::Str
            | IntRep::JimString(_)
            | IntRep::NativeString { .. }
            | IntRep::ByteArray(_)
            | IntRep::Big { .. }
            | IntRep::WordBoolean { .. }
            | IntRep::CompletionCode(_)
            | IntRep::NativeArraySearch { .. }
            | IntRep::FrameLevel { .. }
            | IntRep::NativePropertyName(_)
            | IntRep::NativeInstructionName(_)
            | IntRep::NativeMethodName(_)
            | IntRep::NativeRegexp(_)
            | IntRep::JimRegexp(_)
            | IntRep::JimIndex(_)
            | IntRep::NativeEndOffset(_)
            | IntRep::NativeIndex { .. }
            | IntRep::NativeCommandName(_)
            | IntRep::NativeCommandNameUnresolved(_)
            | IntRep::JimCommand(_)
            | IntRep::JimVariable(_)
            | IntRep::JimOption(_)
            | IntRep::JimSubcommand(_)
            | IntRep::NativeParsedVariableName(_)
            | IntRep::NativeLocalVariableName(_)
            | IntRep::NativeNamespaceName(_)
            | IntRep::JimSource(_)
            | IntRep::JimScript(_)
            | IntRep::JimDictionarySubstitution { .. }
            | IntRep::JimInterpolated(_)
            | IntRep::JimScriptLine { .. }
            | IntRep::Expression(_)
            | IntRep::NativeBytecode(_)
            | IntRep::List { .. }
            | IntRep::Dict(_) => {}
        }
        let s = self.numeric_conversion_text(original, ValueError::NotBooleanBytes)?;
        if let Some(num) = Self::numeric_conversion_parse(&s, original) {
            return match num {
                Number::Int(n) => Ok(n != 0),
                Number::Double(f) => Ok(f != 0.0),
                // A parsed `Big` is beyond `i64`, hence never zero.
                Number::Big { .. } => Ok(true),
                // NaN is a domain error in a boolean context (tclsh 8.6/9.0:
                // "floating point value is Not a Number"), never truthy.
                Number::Nan { .. } => Err(TclError::new("floating point value is Not a Number")),
            };
        }
        // The canonical word acceptor (`ParseBoolean`, tclObj.c): any
        // unambiguous case-insensitive prefix of the six boolean words —
        // one home in `tcl_syntax::boolean`, oracle-table-pinned.
        match tcl_syntax::boolean::parse_boolean_word(&s) {
            Some(b) => Ok(b),
            None => Err(TclError::new(format!(
                "expected boolean value but got {}",
                list::describe_bad_value(&s)
            ))),
        }
    }
}
