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

//! Generating the string of a typed value: `to_str`, `string_bytes` and the renderer for scalars, byte arrays, lists and native caches.

use super::{
    IntRep, MAX_LIST_TO_STR_DEPTH, NativeValueCheck, TOO_DEEPLY_NESTED_PLACEHOLDER, Value,
};
use std::rc::Rc;
use tcl_syntax::native_string::NativeStringStorageIdentity;
use tcl_syntax::raw_string::{RawString, UnicodeAccessError};
use tcl_syntax::value::ValueError;
use tcl_syntax::{list, number};

impl Value {
    /// The string representation, generating and caching it from the typed rep
    /// on first call (Tcl's `Tcl_GetString` / lazy `updateStringProc`).
    #[must_use]
    pub fn to_str(&self) -> Rc<str> {
        self.try_to_str()
            .expect("the legacy Unicode accessor requires a checked value")
    }

    /// Exact string bytes, generating a typed representation without Unicode
    /// replacement. A list retains the original byte values of its elements.
    #[must_use]
    pub fn string_bytes(&self) -> Rc<[u8]> {
        self.to_raw_string_at_depth(0).0.bytes()
    }

    /// Checked host Unicode projection of the actual string representation.
    /// Conversion failure preserves the original bytes and typed object.
    pub fn try_to_str(&self) -> Result<Rc<str>, UnicodeAccessError> {
        self.to_raw_string_at_depth(0).0.unicode()
    }

    /// [`Value::to_str`]'s recursive engine, with an explicit nesting-depth
    /// parameter (see [`MAX_LIST_TO_STR_DEPTH`]). `depth` is
    /// this value's nesting level within the *current* top-level `to_str()`
    /// call (0 at the root). Returns the string alongside whether rendering
    /// it anywhere in this subtree hit the depth cap — when it did, the
    /// result must not be cached (`self.0.string`): the same shared `Rc<Obj>`
    /// may also be reachable directly (or at a shallower offset) from
    /// elsewhere, where a fresh call starting back at depth 0 could
    /// legitimately render it in full, and caching the truncated result
    /// here would wrongly leak into that unrelated call. A value entirely
    /// within the cap is unaffected either way — same output, same caching
    /// — as the pre-fix implementation.
    pub(super) fn to_raw_string_at_depth(&self, depth: u32) -> (RawString, bool) {
        self.to_raw_string_at_depth_using::<ValueError>(depth, &mut None, None)
            .expect("neutral String representation")
    }

    /// Reach the same explicit-independent String worker under original checks.
    /// Native retained compounds keep their own recipe and NULL formatter;
    /// neutral values retain their existing rendering and UTF8 semantics.
    pub(crate) fn string_bytes_checked_for_value_retaining<E: From<ValueError>>(
        &self,
        check: &mut NativeValueCheck<'_, E>,
        retain: &dyn Fn(&ValueError),
    ) -> Result<Rc<[u8]>, E> {
        let mut original = |current: &Value| {
            check(current)?;
            for value in [self, current] {
                if let Err(error) = value.check_native_header() {
                    retain(&error);
                    return Err(error.into());
                }
            }
            Ok(())
        };
        self.to_raw_string_at_depth_using(0, &mut Some(&mut original), Some(retain))
            .map(|(raw, _)| raw.bytes())
    }

    fn to_raw_string_at_depth_using<E: From<ValueError>>(
        &self,
        depth: u32,
        check: &mut Option<&mut NativeValueCheck<'_, E>>,
        retain: Option<&dyn Fn(&ValueError)>,
    ) -> Result<(RawString, bool), E> {
        enum Backing {
            List(crate::NativeListItems),
            Dictionary(Rc<Vec<(Value, Value)>>),
        }
        self.native_string_stage_check(check)?;
        let unavailable = || {
            let error = ValueError::from(
                tcl_syntax::raw_string::NativeStringAccessError::Unavailable(
                    tcl_syntax::native_string::NativeStringUnavailable::StringUpdater,
                ),
            );
            if let Some(retain) = retain {
                retain(&error);
            }
            E::from(error)
        };
        if let Some(s) = self.0.string.borrow().as_ref() {
            return Ok((s.clone(), false));
        }
        let selected = match &*self.0.intrep.borrow() {
            IntRep::List {
                string_protocol, ..
            } => string_protocol.get(),
            IntRep::Dict(dict) => dict.string_protocol.get(),
            _ => None,
        };
        if let Some(protocol) = selected {
            let bytes = if let Some(stage) = check.as_deref_mut() {
                self.native_string_bytes_checked_for_value_using(protocol, None, stage, retain)?
            } else {
                self.native_string_bytes(protocol)
                    .expect("selected compound updater requires checked child storage")
            };
            self.native_string_stage_check(check)?;
            return Ok((RawString::from_bytes(bytes), false));
        }
        // Checked recursion keeps the actual backing allocation under a short
        // borrow, without acquiring new native member roles or cloning a parser.
        let backing = if check.is_some() {
            match &*self.0.intrep.borrow() {
                IntRep::List { items, .. } => Some(Backing::List(items.lifetime_view())),
                IntRep::Dict(dict) => Some(Backing::Dictionary(dict.pairs_backing())),
                _ => None,
            }
        } else {
            None
        };
        let (generated, past_cap) = if let Some(backing) = backing {
            let reached = match &backing {
                Backing::List(items) => {
                    Self::render_list_bytes_using(items.iter(), depth, check, retain)
                }
                Backing::Dictionary(pairs) => Self::render_list_bytes_using(
                    pairs.iter().flat_map(|(key, value)| [key, value]),
                    depth,
                    check,
                    retain,
                ),
            };
            // Intrinsic/native errors and original checker failures are already
            // retained inside the reached child worker before this lease retires.
            drop(backing);
            self.native_string_stage_check(check)?;
            reached?
        } else {
            self.render_independent_primary_using(
                &self.0.intrep.borrow(),
                depth,
                check,
                retain,
                &unavailable,
            )?
        };
        self.native_string_stage_check(check)?;
        if !past_cap {
            self.cache_generated_raw_string(&generated);
        }
        self.native_string_stage_check(check)?;
        Ok((generated, past_cap))
    }

    fn render_independent_primary_using<E: From<ValueError>>(
        &self,
        cache: &IntRep,
        depth: u32,
        check: &mut Option<&mut NativeValueCheck<'_, E>>,
        retain: Option<&dyn Fn(&ValueError)>,
        unavailable: &dyn Fn() -> E,
    ) -> Result<(RawString, bool), E> {
        Ok(match cache {
            IntRep::Str
            | IntRep::JimString(_)
            | IntRep::JimSource(_)
            | IntRep::JimScript(_)
            | IntRep::JimDictionarySubstitution { .. }
            | IntRep::JimInterpolated(_)
            | IntRep::JimScriptLine { .. }
            | IntRep::Expression(_) => (RawString::from_unicode(""), false),
            IntRep::NativeBytecode(_) => {
                if check.is_some() {
                    return Err(unavailable());
                }
                unreachable!("native Bytecode has resident source and no string updater")
            }
            IntRep::NativeString {
                protocol,
                unicode: Some(unicode),
                ..
            } => {
                let version = protocol
                    .tcl_version()
                    .expect("native C String unit backing");
                let bytes = tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(version)
                    .encode_units(unicode)
                    .expect("validated native Unicode units");
                (RawString::from_bytes(bytes), false)
            }
            IntRep::NativeString {
                num_chars: Some(0),
                unicode: None,
                ..
            } => (RawString::from_bytes(b"".as_slice()), false),
            IntRep::NativeString { unicode: None, .. } => {
                if check.is_some() {
                    return Err(unavailable());
                }
                panic!("native String has neither resident bytes nor Unicode backing")
            }
            IntRep::ByteArray(bytes) => (Self::raw_byte_array_string(&bytes.bytes), false),
            cache @ (IntRep::Int(_)
            | IntRep::Tcl84Long(_)
            | IntRep::Tcl84Wide(_)
            | IntRep::CoercedDouble(_)
            | IntRep::Big { .. }
            | IntRep::Double(_)
            | IntRep::Bool(_)
            | IntRep::WordBoolean { .. }
            | IntRep::NativeIndex { .. }
            | IntRep::NativeNamespaceName(_)
            | IntRep::JimIndex(_)
            | IntRep::NativeEndOffset(_)
            | IntRep::NativeInstructionName(_)) => {
                self.render_independent_scalar_cache(cache, check.is_some(), unavailable)?
            }
            IntRep::CompletionCode(_)
            | IntRep::NativeArraySearch { .. }
            | IntRep::FrameLevel { .. }
            | IntRep::NativePropertyName(_)
            | IntRep::NativeMethodName(_)
            | IntRep::NativeRegexp(_)
            | IntRep::JimRegexp(_)
            | IntRep::NativeCommandName(_)
            | IntRep::NativeCommandNameUnresolved(_)
            | IntRep::JimCommand(_)
            | IntRep::JimVariable(_)
            | IntRep::JimOption(_)
            | IntRep::JimSubcommand(_)
            | IntRep::NativeParsedVariableName(_)
            | IntRep::NativeLocalVariableName(_) => {
                if check.is_some() {
                    return Err(unavailable());
                }
                panic!(
                    "completion-code cache has no native string updater; use checked native string access"
                )
            }
            IntRep::List { items, .. } => {
                Self::render_list_bytes_using(items.iter(), depth, check, retain)?
            }
            IntRep::Dict(dict) => Self::render_list_bytes_using(
                dict.pairs_backing()
                    .iter()
                    .flat_map(|(key, value)| [key, value]),
                depth,
                check,
                retain,
            )?,
        })
    }

    fn render_independent_scalar_cache<E>(
        &self,
        cache: &IntRep,
        checked: bool,
        unavailable: &dyn Fn() -> E,
    ) -> Result<(RawString, bool), E> {
        if checked {
            let has_updater = match cache {
                IntRep::NativeEndOffset(offset) => offset.string_update().is_some(),
                IntRep::NativeIndex { cache, .. } => cache.word().is_ok(),
                IntRep::NativeNamespaceName(cache) => cache.string_update_bytes(
                    tcl_syntax::native_namespace_name::NativeNamespaceNameRecipe::for_tcl_version(cache.version()),
                ).is_some(),
                IntRep::WordBoolean { version, .. } => *version == tcl_dialect::TclVersion::V8_4,
                _ => true,
            };
            if !has_updater {
                return Err(unavailable());
            }
        }
        Ok((self.render_native_cache_bytes(cache), false))
    }

    fn raw_byte_array_string(bytes: &[u8]) -> RawString {
        RawString::from_unicode(
            bytes
                .iter()
                .map(|&byte| char::from(byte))
                .collect::<String>(),
        )
    }

    fn cache_generated_raw_string(&self, generated: &RawString) {
        *self.0.string.borrow_mut() = Some(generated.clone());
        if let IntRep::List { canonical, .. } = &mut *self.0.intrep.borrow_mut() {
            canonical.set(true);
        }
        self.0.string_storage.set(
            if generated.bytes().is_empty()
                && matches!(*self.0.intrep.borrow(), IntRep::NativeNamespaceName(_))
            {
                NativeStringStorageIdentity::CanonicalEmpty
            } else {
                NativeStringStorageIdentity::Allocated
            },
        );
    }

    fn render_native_cache_bytes(&self, cache: &IntRep) -> RawString {
        let (bytes, _) = match cache {
            IntRep::NativeEndOffset(offset) => (
                RawString::from_bytes(offset.string_update().expect("legacy end-offset updater")),
                false,
            ),
            IntRep::Int(n) | IntRep::Tcl84Long(n) | IntRep::Tcl84Wide(n) => (
                RawString::from_unicode(tcl_syntax::number::native_integer_string(
                    *n,
                    tcl_syntax::number::runtime_release()
                        .is_some_and(tcl_dialect::TclVersion::garbles_wide_min),
                )),
                false,
            ),
            IntRep::CoercedDouble(n) => (RawString::from_unicode(n.to_string()), false),
            IntRep::Big {
                negative,
                radix,
                digits,
            } => {
                let magnitude = num_bigint::BigInt::parse_bytes(digits.as_bytes(), *radix as u32)
                    .expect("validated native Big magnitude");
                let integer = if *negative { -magnitude } else { magnitude };
                (RawString::from_unicode(integer.to_string()), false)
            }
            IntRep::Double(f) => {
                let context = self.0.double_format.borrow();
                let rendered = match context.as_ref() {
                    Some(context) => {
                        number::format_double_native_selected(*f, context.policy, context.format())
                    }
                    None => number::format_double(*f),
                };
                (RawString::from_unicode(rendered), false)
            }
            IntRep::Bool(b) => (RawString::from_unicode(if *b { "1" } else { "0" }), false),
            IntRep::WordBoolean { value, version } => {
                assert_eq!(
                    *version,
                    tcl_dialect::TclVersion::V8_4,
                    "native word-Boolean descriptor has no string updater"
                );
                (
                    RawString::from_unicode(if *value { "1" } else { "0" }),
                    false,
                )
            }
            IntRep::NativeIndex { cache, .. } => (
                RawString::from_bytes(cache.word().expect("checked native Index table updater")),
                false,
            ),
            IntRep::NativeNamespaceName(cache) => {
                let recipe =
                    tcl_syntax::native_namespace_name::NativeNamespaceNameRecipe::for_tcl_version(
                        cache.version(),
                    );
                (
                    RawString::from_bytes(
                        cache
                            .string_update_bytes(recipe)
                            .expect("namespace primary without updater retains original string"),
                    ),
                    false,
                )
            }
            IntRep::JimIndex(index) => (RawString::from_bytes(index.string_bytes()), false),
            IntRep::NativeInstructionName(name) => (
                RawString::from_bytes(Rc::<[u8]>::from(name.string_bytes())),
                false,
            ),
            _ => unreachable!("scalar cache renderer selected for another primary"),
        };
        bytes
    }

    fn render_list_bytes_using<'a, E: From<ValueError>>(
        values: impl Iterator<Item = &'a Self>,
        depth: u32,
        check: &mut Option<&mut NativeValueCheck<'_, E>>,
        retain: Option<&dyn Fn(&ValueError)>,
    ) -> Result<(RawString, bool), E> {
        if MAX_LIST_TO_STR_DEPTH.exceeded(depth) {
            return Ok((RawString::from_unicode(TOO_DEEPLY_NESTED_PLACEHOLDER), true));
        }
        let mut past_cap = false;
        let mut bytes = Vec::new();
        for (index, value) in values.enumerate() {
            if index != 0 {
                bytes.push(b' ');
            }
            let (element, capped) = value.to_raw_string_at_depth_using(depth + 1, check, retain)?;
            past_cap |= capped;
            list::append_list_element(&mut bytes, &element.bytes(), index == 0);
        }
        Ok((RawString::from_bytes(bytes), past_cap))
    }
}
