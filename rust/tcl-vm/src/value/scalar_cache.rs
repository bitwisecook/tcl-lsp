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

//! The numeric cache of an object: inspecting it, reconstructing it from a transported cache, and adopting a conversion.

use super::{DoubleFormatContext, IntRep, Value};
use std::rc::Rc;
use tcl_syntax::native_string::NativeStringStorageIdentity;
use tcl_syntax::number::Number;
use tcl_syntax::scalar_getter::{NativeScalarCache, NativeScalarGetterProtocol};
use tcl_syntax::value::ValueError;

impl Value {
    /// Inspect the primary physical scalar cache without generating its string.
    /// This describes storage and grants no native execution permission.
    #[must_use]
    pub fn native_scalar_cache(&self) -> Option<NativeScalarCache> {
        match &*self.0.intrep.borrow() {
            IntRep::Int(value) | IntRep::Tcl84Wide(value) => {
                Some(NativeScalarCache::Number(Number::Int(*value)))
            }
            IntRep::Tcl84Long(value) => Some(NativeScalarCache::Tcl84Long(*value)),
            IntRep::Bool(value) => Some(NativeScalarCache::Number(Number::Int(i64::from(*value)))),
            IntRep::Big {
                negative,
                radix,
                digits,
            } => Some(NativeScalarCache::Number(Number::Big {
                negative: *negative,
                radix: *radix,
                digits: digits.clone(),
            })),
            IntRep::Double(value) => Some(NativeScalarCache::Number(Number::Double(*value))),
            IntRep::CoercedDouble(value) => Some(NativeScalarCache::JimCoercedInteger(*value)),
            IntRep::WordBoolean { value, .. } => Some(NativeScalarCache::WordBoolean(*value)),
            _ => None,
        }
    }

    /// Tcl's constant compiler transfers its numeric primary only into an
    /// untyped registered header; an earlier typed literal remains untouched.
    pub(crate) fn adopt_native_expression_number_if_untyped(
        &self,
        number: Number,
        version: tcl_dialect::TclVersion,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        self.check_native_header()?;
        if version < tcl_dialect::TclVersion::V8_5 || self.resident_string_bytes().is_none() {
            return Err(ValueError::CommandProtocolUnavailable(
                "native constant cache transfer",
            ));
        }
        if matches!(*self.0.intrep.borrow(), IntRep::Str) {
            self.adopt_native_scalar_cache(
                NativeScalarCache::Number(number),
                tcl_native_model::InvocationDialect::for_version(version)
                    .native_scalar_getter_protocol()
                    .ok_or(ValueError::ScalarNumericInputUnavailable)?,
                tcl_native_model::InvocationDialect::for_version(version),
            )?;
        }
        Ok(())
    }

    /// Inspect the live exact integer primary without a getter or display projection.
    /// This query describes storage; the calling purpose selects its actual issuer.
    pub(crate) fn native_integer_primary(
        &self,
    ) -> Result<Option<i64>, tcl_syntax::value::ValueError> {
        self.check_native_header()?;
        Ok(match &*self.0.intrep.borrow() {
            IntRep::Int(value) => Some(*value),
            _ => None,
        })
    }

    /// Exact scalar descriptor origin; retained metadata grants no live issuer.
    #[must_use]
    pub fn native_scalar_descriptor_origin(&self) -> Option<tcl_dialect::TclVersion> {
        if matches!(&*self.0.intrep.borrow(), IntRep::Tcl84Wide(_)) {
            Some(tcl_dialect::TclVersion::V8_4)
        } else {
            self.native_word_boolean_version()
        }
    }

    /// Actual C release retained with a native word-Boolean descriptor.
    #[must_use]
    pub fn native_word_boolean_version(&self) -> Option<tcl_dialect::TclVersion> {
        match *self.0.intrep.borrow() {
            IntRep::WordBoolean { version, .. } => Some(version),
            _ => None,
        }
    }

    /// Current full numeric cache, excluding native word-Boolean descriptors.
    pub(crate) fn number_representation(&self) -> Option<Number> {
        match self.native_scalar_cache()? {
            NativeScalarCache::Number(number) => Some(number),
            NativeScalarCache::Tcl84Long(value) => Some(Number::Int(value)),
            NativeScalarCache::JimCoercedInteger(integer) => Some(Number::Double(integer as f64)),
            NativeScalarCache::WordBoolean(_) => None,
        }
    }

    /// Reconstruct a transported physical cache without invoking a getter.
    /// Cache origin and independently selected actual engine must agree.
    ///
    /// # Errors
    /// Refuses unavailable engines, foreign descriptor origins or malformed Big storage.
    pub fn from_native_scalar_cache(
        cache: NativeScalarCache,
        word_boolean_origin: Option<tcl_dialect::TclVersion>,
        dialect: tcl_native_model::InvocationDialect,
    ) -> Result<Self, tcl_syntax::value::ValueError> {
        Self::from_native_scalar_cache_with_storage(cache, word_boolean_origin, dialect, None)
    }

    /// Reconstruct a full transported cache and its independently resident string.
    /// Modern native word-Boolean descriptors require that resident string.
    /// Explicit C84 Wide data also require the actual scalar ingress receipt.
    ///
    /// # Errors
    /// Refuses missing authority, inconsistent storage or a foreign cache origin.
    pub fn from_native_scalar_cache_with_storage(
        cache: NativeScalarCache,
        word_boolean_origin: Option<tcl_dialect::TclVersion>,
        dialect: tcl_native_model::InvocationDialect,
        resident: Option<(Rc<[u8]>, NativeStringStorageIdentity)>,
    ) -> Result<Self, ValueError> {
        Self::from_native_scalar_cache_with_storage_using(
            cache,
            word_boolean_origin,
            dialect,
            resident,
            None,
        )
    }

    pub(crate) fn from_native_scalar_cache_with_ingress(
        cache: NativeScalarCache,
        origin: Option<tcl_dialect::TclVersion>,
        ingress: &crate::NativeScalarIngress,
        resident: Option<(Rc<[u8]>, NativeStringStorageIdentity)>,
    ) -> Result<Self, ValueError> {
        Self::from_native_scalar_cache_with_storage_using(
            cache,
            origin,
            ingress.dialect(),
            resident,
            Some(ingress),
        )
    }

    fn from_native_scalar_cache_with_storage_using(
        cache: NativeScalarCache,
        word_boolean_origin: Option<tcl_dialect::TclVersion>,
        dialect: tcl_native_model::InvocationDialect,
        resident: Option<(Rc<[u8]>, NativeStringStorageIdentity)>,
        ingress: Option<&crate::NativeScalarIngress>,
    ) -> Result<Self, ValueError> {
        let protocol = dialect
            .native_scalar_getter_protocol()
            .ok_or(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable)?;
        if matches!(cache, NativeScalarCache::Number(Number::Int(_)))
            && word_boolean_origin.is_some()
            && ingress.is_none()
        {
            return Err(ValueError::CommandProtocolUnavailable(
                "original C84 Wide descriptor ingress unavailable",
            ));
        }
        if word_boolean_origin.is_some_and(|origin| Some(origin) != protocol.tcl_version())
            || (matches!(cache, NativeScalarCache::WordBoolean(_)) && word_boolean_origin.is_none())
        {
            return Err(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable);
        }
        let mut value = Self::empty();
        if let Some((bytes, storage)) = resident {
            value = value.with_resident_string_bytes_and_storage(bytes, storage)?;
        } else {
            *value.0.string.borrow_mut() = None;
            value
                .0
                .string_storage
                .set(NativeStringStorageIdentity::Unknown);
        }
        if let Some(ingress) = ingress {
            ingress.validate_cache(&cache)?;
            let mut geometry = || ingress.validate_cache(&cache);
            let mut stage: Option<&mut dyn FnMut() -> Result<(), ValueError>> = Some(&mut geometry);
            value.adopt_native_scalar_cache_using(cache.clone(), protocol, dialect, &mut stage)?;
        } else {
            value.adopt_native_scalar_cache(cache, protocol, dialect)?;
        }
        Ok(value)
    }

    /// Store a reached legacy increment cache on the original working object.
    ///
    /// # Errors
    /// Refuses caches outside authentic C 8.4 and Jim integer increment storage.
    #[cfg(test)]
    pub(crate) fn store_native_legacy_increment_cache(
        &self,
        cache: NativeScalarCache,
        dialect: tcl_native_model::InvocationDialect,
    ) -> Result<(), ValueError> {
        self.store_native_legacy_increment_cache_using(cache, dialect, &mut None)
    }

    pub(crate) fn store_native_legacy_increment_cache_checked<E: From<ValueError>>(
        &self,
        cache: NativeScalarCache,
        dialect: tcl_native_model::InvocationDialect,
        check: &mut dyn FnMut() -> Result<(), E>,
    ) -> Result<(), E> {
        self.store_native_legacy_increment_cache_using(cache, dialect, &mut Some(check))
    }

    fn store_native_legacy_increment_cache_using<E: From<ValueError>>(
        &self,
        cache: NativeScalarCache,
        dialect: tcl_native_model::InvocationDialect,
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
    ) -> Result<(), E> {
        Self::native_stage_check(check)?;
        let protocol = dialect
            .native_scalar_getter_protocol()
            .ok_or(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable)?;
        let admitted = matches!(
            (protocol.tcl_version(), protocol.is_jim084(), &cache),
            (
                Some(tcl_dialect::TclVersion::V8_4),
                false,
                NativeScalarCache::Tcl84Long(_) | NativeScalarCache::Number(Number::Int(_))
            ) | (None, true, NativeScalarCache::Number(Number::Int(_)))
        );
        if !admitted {
            return Err(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable.into());
        }
        self.adopt_native_scalar_cache_using(cache, protocol, dialect, check)?;
        Self::native_stage_check(check)?;
        *self.0.string.borrow_mut() = None;
        self.0
            .string_storage
            .set(NativeStringStorageIdentity::Unknown);
        *self.0.source_location.borrow_mut() = None;
        Self::native_stage_check(check)
    }

    fn adopt_native_scalar_cache(
        &self,
        cache: NativeScalarCache,
        protocol: NativeScalarGetterProtocol,
        dialect: tcl_native_model::InvocationDialect,
    ) -> Result<(), ValueError> {
        self.adopt_native_scalar_cache_using(cache, protocol, dialect, &mut None)
    }

    pub(super) fn adopt_native_scalar_cache_using<E: From<ValueError>>(
        &self,
        cache: NativeScalarCache,
        protocol: NativeScalarGetterProtocol,
        dialect: tcl_native_model::InvocationDialect,
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
    ) -> Result<(), E> {
        Self::native_stage_check(check)?;
        let rep = match cache {
            NativeScalarCache::Tcl84Long(value) => {
                if protocol.tcl_version() != Some(tcl_dialect::TclVersion::V8_4) {
                    return Err(ValueError::ScalarNumericInputUnavailable.into());
                }
                IntRep::Tcl84Long(value)
            }
            NativeScalarCache::Number(Number::Int(value)) => {
                if protocol.tcl_version() == Some(tcl_dialect::TclVersion::V8_4) && check.is_some()
                {
                    IntRep::Tcl84Wide(value)
                } else {
                    IntRep::Int(value)
                }
            }
            NativeScalarCache::Number(Number::Double(value)) => IntRep::Double(value),
            NativeScalarCache::Number(Number::Nan { negative, payload }) => {
                IntRep::Double(f64::from_bits(
                    (u64::from(negative) << 63)
                        | 0x7ff8_0000_0000_0000
                        | (payload.unwrap_or(0) & 0x0007_ffff_ffff_ffff),
                ))
            }
            NativeScalarCache::Number(Number::Big {
                negative,
                radix,
                digits,
            }) => {
                if num_bigint::BigInt::parse_bytes(digits.as_bytes(), radix as u32).is_none() {
                    return Err(ValueError::ScalarNumericInputUnavailable.into());
                }
                IntRep::Big {
                    negative,
                    radix,
                    digits,
                }
            }
            NativeScalarCache::WordBoolean(value) => {
                let version = protocol
                    .tcl_version()
                    .ok_or(ValueError::ScalarNumericInputUnavailable)?;
                if version != tcl_dialect::TclVersion::V8_4 && self.0.string.borrow().is_none() {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "native word-Boolean resident string",
                    )
                    .into());
                }
                IntRep::WordBoolean { value, version }
            }
            NativeScalarCache::JimCoercedInteger(value) => {
                if !protocol.is_jim084() {
                    return Err(ValueError::ScalarNumericInputUnavailable.into());
                }
                IntRep::CoercedDouble(value)
            }
        };
        self.replace_primary_using(rep, check)?;
        Self::native_stage_check(check)?;
        *self.0.double_format.borrow_mut() = DoubleFormatContext::for_dialect(dialect);
        Self::native_stage_check(check)
    }

    /// The truth of a plain wide-integer or Boolean representation, without
    /// consulting its string.
    pub(crate) fn plain_truth(&self) -> Option<bool> {
        match *self.0.intrep.borrow() {
            IntRep::Int(value) => Some(value != 0),
            IntRep::Bool(value) => Some(value),
            _ => None,
        }
    }

    /// The payload of a plain (Tcl 8.5+) wide-integer representation only.
    pub(crate) fn plain_integer_representation(&self) -> Option<i64> {
        match *self.0.intrep.borrow() {
            IntRep::Int(value) => Some(value),
            _ => None,
        }
    }

    /// Inspect a native integer conversion without reparsing retained source bytes.
    pub(crate) fn integer_representation(&self) -> Option<i64> {
        match *self.0.intrep.borrow() {
            IntRep::Int(value) | IntRep::Tcl84Long(value) | IntRep::Tcl84Wide(value) => Some(value),
            IntRep::Bool(value) => Some(i64::from(value)),
            _ => None,
        }
    }

    pub(crate) fn stock_list_input_class(
        &self,
    ) -> tcl_registry::native_stock_list::NativeStockListInputClass {
        use tcl_registry::native_stock_list::NativeStockListInputClass as Class;
        match &*self.0.intrep.borrow() {
            IntRep::Str | IntRep::NativeString { .. } | IntRep::JimString(_) => Class::String,
            IntRep::List { .. } => Class::List,
            IntRep::Dict(_) => Class::Dictionary,
            IntRep::ByteArray(_) => Class::ByteArray,
            IntRep::Int(_)
            | IntRep::Tcl84Long(_)
            | IntRep::Tcl84Wide(_)
            | IntRep::Big { .. }
            | IntRep::Double(_)
            | IntRep::CoercedDouble(_)
            | IntRep::Bool(_) => Class::Numeric,
            IntRep::WordBoolean { .. } => Class::Boolean,
            IntRep::NativeCommandName(_) | IntRep::NativeCommandNameUnresolved(_) => {
                Class::CommandName
            }
            IntRep::NativeArraySearch { .. } => Class::ArraySearch,
            IntRep::NativeNamespaceName(_) => Class::NamespaceName,
            IntRep::JimCommand(_)
            | IntRep::JimVariable(_)
            | IntRep::JimOption(_)
            | IntRep::JimSubcommand(_) => Class::JimLookup,
            IntRep::NativeParsedVariableName(_) => Class::ParsedVariableName,
            IntRep::NativeLocalVariableName(_) => Class::LocalVariableName,
            IntRep::JimSource(_) => Class::JimSource,
            IntRep::Expression(_)
            | IntRep::NativeBytecode(_)
            | IntRep::CompletionCode(_)
            | IntRep::FrameLevel { .. }
            | IntRep::NativeRegexp(_) => Class::CompiledCache,
            IntRep::JimScript(_)
            | IntRep::JimDictionarySubstitution { .. }
            | IntRep::JimInterpolated(_)
            | IntRep::JimScriptLine { .. }
            | IntRep::JimRegexp(_)
            | IntRep::JimIndex(_) => Class::Unknown,
            IntRep::NativePropertyName(_) => Class::PropertyName,
            IntRep::NativeInstructionName(_) => Class::InstructionName,
            IntRep::NativeMethodName(_) => Class::MethodName,
            IntRep::NativeIndex { .. } | IntRep::NativeEndOffset(_) => Class::Index,
        }
    }

    pub(super) fn restore_coerced_integer(&self) -> Option<i64> {
        let integer = match *self.0.intrep.borrow() {
            IntRep::CoercedDouble(value) => Some(value),
            _ => None,
        };
        if let Some(value) = integer {
            *self.0.intrep.borrow_mut() = IntRep::Int(value);
        }
        integer
    }

    /// Inspect a double without materialising its string representation.
    pub(crate) fn double_representation(&self) -> Option<f64> {
        match *self.0.intrep.borrow() {
            IntRep::Double(value) => Some(value),
            IntRep::CoercedDouble(value) => Some(value as f64),
            _ => None,
        }
    }

    /// Retain a successfully converted native operand on the shared object.
    /// Conversion preserves its existing source bytes, including list spelling.
    pub(crate) fn cache_integer_representation(&self, value: i64) {
        let _ = self.cache_numeric_representation_using::<ValueError>(
            Number::Int(value),
            &mut None,
            &mut None,
        );
    }

    /// Retain a successfully converted double without changing its source bytes.
    pub(crate) fn cache_double_representation(&self, value: f64) {
        let _ = self.cache_numeric_representation_using::<ValueError>(
            Number::Double(value),
            &mut None,
            &mut None,
        );
    }

    pub(crate) fn cache_numeric_representation_checked<E: From<ValueError>>(
        &self,
        value: Number,
        check: &mut dyn FnMut() -> Result<(), E>,
        string: &mut dyn FnMut() -> Result<Rc<[u8]>, E>,
    ) -> Result<(), E> {
        self.cache_numeric_representation_using(value, &mut Some(check), &mut Some(string))
    }

    pub(super) fn cache_numeric_representation_using<E: From<ValueError>>(
        &self,
        value: Number,
        check: &mut Option<&mut dyn FnMut() -> Result<(), E>>,
        string: &mut Option<&mut dyn FnMut() -> Result<Rc<[u8]>, E>>,
    ) -> Result<(), E> {
        Self::native_stage_check(check)?;
        let replacement = match value {
            Number::Int(integer) => IntRep::Int(integer),
            Number::Double(double) => IntRep::Double(double),
            _ => return Err(ValueError::ScalarNumericInputUnavailable.into()),
        };
        let reached = if let Some(string) = string {
            string()
        } else {
            Ok(self.string_bytes())
        };
        Self::native_stage_check(check)?;
        reached?;
        if check.is_none() {
            self.replace_primary(replacement);
        } else {
            self.replace_primary_using(replacement, check)?;
        }
        let reached = Self::native_stage_check(check);
        drop(value);
        reached
    }
}
