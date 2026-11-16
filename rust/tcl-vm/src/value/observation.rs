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

//! Observing an object without changing it: the native snapshot, `tcl::unsupported::representation` and measured size.

use super::{IntRep, NativeBytecodeContext, Value};
use std::rc::Rc;
#[cfg(test)]
use tcl_syntax::native_string::NativeStringProtocol;
use tcl_syntax::raw_string::RawString;

impl Value {
    /// Original physical descriptor name for native conformance observers.
    #[cfg(test)]
    pub(crate) fn native_object_type_name(&self) -> &'static str {
        match &*self.0.intrep.borrow() {
            IntRep::Str => "none",
            IntRep::NativeString { .. } | IntRep::JimString(_) => "string",
            IntRep::List { .. } => "list",
            IntRep::Dict(_) => "dict",
            IntRep::ByteArray(_) => "bytearray",
            IntRep::NativeBytecode(cache) => {
                if matches!(cache.context, NativeBytecodeContext::Substitution(_)) {
                    "substcode"
                } else {
                    "bytecode"
                }
            }
            IntRep::NativeRegexp(_) | IntRep::JimRegexp(_) => "regexp",
            IntRep::JimIndex(_) => "index",
            IntRep::NativeEndOffset(_) => "end-offset",
            IntRep::NativePropertyName(_) => "tcl::oo property name",
            IntRep::NativeInstructionName(_) => "instname",
            IntRep::FrameLevel { .. } => "levelReference",
            IntRep::NativeMethodName(_) => "TclOO method name",
            IntRep::JimSource(_) => "source",
            IntRep::JimScript(_) => "script",
            IntRep::JimDictionarySubstitution { .. } => "dict-substitution",
            IntRep::JimInterpolated(_) => "interpolated",
            IntRep::JimVariable(_) => "variable",
            IntRep::JimSubcommand(_) => "subcmd-lookup",
            IntRep::JimOption(tcl_core_types::NativeJimOptionCache::Enum { .. }) => "get-enum",
            IntRep::JimOption(tcl_core_types::NativeJimOptionCache::ComparedString { .. }) => {
                "compared-string"
            }
            IntRep::JimCommand(_) => "command",
            IntRep::Expression(cache) => {
                if cache.dialect.native_string_protocol() == Some(NativeStringProtocol::Jim084) {
                    "expression"
                } else {
                    "expr"
                }
            }
            IntRep::NativeCommandName(_) | IntRep::NativeCommandNameUnresolved(_) => "cmdName",
            IntRep::NativeParsedVariableName(_) => "parsedVarName",
            IntRep::NativeLocalVariableName(_) => "localVarName",
            IntRep::NativeNamespaceName(_) => "nsName",
            IntRep::Tcl84Long(_) | IntRep::Int(_) => "int",
            IntRep::Tcl84Wide(_) => "wideInt",
            IntRep::Big { .. } => "bignum",
            IntRep::Double(_) | IntRep::CoercedDouble(_) => "double",
            _ => "opaque",
        }
    }

    /// Snapshot original primary cache and storage without materialization.
    /// Object identity and selected operation authority remain separate inputs.
    #[must_use]
    pub fn native_object_snapshot(&self) -> tcl_syntax::native_object::NativeObjectSnapshot {
        use tcl_syntax::native_object::{NativeObjectCacheSnapshot as Cache, NativeObjectSnapshot};
        let resident = self.resident_string_bytes();
        let cache = match &*self.0.intrep.borrow() {
            IntRep::Str => Cache::None,
            IntRep::NativeString {
                protocol,
                num_chars,
                unicode,
            } => Cache::String {
                protocol: *protocol,
                num_chars: *num_chars,
                unicode: unicode.clone(),
            },
            IntRep::JimString(count) => Cache::JimString { num_chars: *count },
            IntRep::ByteArray(bytes) => Cache::ByteArray {
                bytes: Rc::clone(&bytes.bytes),
                proper: bytes.proper,
            },
            IntRep::WordBoolean { value, version } => Cache::WordBoolean {
                value: *value,
                version: *version,
            },
            IntRep::List {
                items, canonical, ..
            } => Cache::List {
                length: items.len(),
                canonical: canonical.get(),
            },
            IntRep::Dict(dict) => Cache::Dictionary {
                size: dict.with_pairs(<[(Value, Value)]>::len),
                pure: resident.is_none(),
            },
            IntRep::Int(_)
            | IntRep::Tcl84Long(_)
            | IntRep::Tcl84Wide(_)
            | IntRep::Big { .. }
            | IntRep::Double(_)
            | IntRep::CoercedDouble(_)
            | IntRep::Bool(_) => Cache::Numeric(
                self.native_scalar_cache()
                    .expect("matched numeric primary representation"),
            ),
            _ => self.native_named_primary_snapshot(),
        };
        NativeObjectSnapshot {
            storage: self.resident_string_storage_identity(),
            resident,
            cache,
        }
    }

    fn native_named_primary_snapshot(
        &self,
    ) -> tcl_syntax::native_object::NativeObjectCacheSnapshot {
        use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
        match &*self.0.intrep.borrow() {
            IntRep::NativeCommandName(cache) => Cache::CommandName {
                version: cache.version,
                resolved: true,
            },
            IntRep::NativeCommandNameUnresolved(version) => Cache::CommandName {
                version: *version,
                resolved: false,
            },
            IntRep::JimCommand(cache) => Cache::JimCommand {
                procedure_epoch: cache.epoch,
            },
            IntRep::JimVariable(cache) => Cache::JimVariable {
                frame: cache.frame,
                global: cache.global,
            },
            IntRep::JimOption(tcl_core_types::NativeJimOptionCache::Enum { entry, flags }) => {
                Cache::JimEnum {
                    flags: *flags,
                    index: entry.index(),
                }
            }
            IntRep::JimOption(tcl_core_types::NativeJimOptionCache::ComparedString { .. }) => {
                Cache::JimComparedString
            }
            IntRep::JimSubcommand(cache) => Cache::JimSubcommand {
                index: cache.index(),
            },
            IntRep::NativeParsedVariableName(cache) => Cache::ParsedVariableName {
                version: cache.protocol.version(),
                array: cache.array.is_some(),
            },
            IntRep::NativeLocalVariableName(cache) => Cache::LocalVariableName {
                version: cache.protocol.version(),
                index: cache.index,
            },
            IntRep::NativeNamespaceName(cache) => Cache::NamespaceName {
                version: cache.version(),
                resolved: cache.namespace().is_some(),
            },
            IntRep::NativeBytecode(cache) => {
                if matches!(cache.context, NativeBytecodeContext::Substitution(_)) {
                    Cache::Other
                } else {
                    Cache::Bytecode {
                        version: cache.version,
                    }
                }
            }
            IntRep::JimScript(header) => Cache::JimScript {
                flags: header.0.storage.flags(),
                tokens: header.0.storage.len(),
            },
            IntRep::JimDictionarySubstitution { .. } => Cache::JimDictionarySubstitution,
            IntRep::JimInterpolated(_) => Cache::JimInterpolated,
            IntRep::NativeInstructionName(name) => Cache::InstructionName {
                version: name.version(),
                opcode: name.opcode(),
            },
            IntRep::FrameLevel { cache, version } => {
                let (relative, level) = match cache {
                    tcl_registry::NativeFrameLevelCache::Relative(level) => (true, *level),
                    tcl_registry::NativeFrameLevelCache::Absolute(level) => (false, *level),
                };
                Cache::FrameReference {
                    version: *version,
                    relative,
                    level,
                }
            }
            IntRep::NativeIndex { cache, version } => Cache::Index {
                version: *version,
                index: cache.index(),
                stride: cache.stride(),
            },
            _ => Cache::Other,
        }
    }
}

impl Value {
    /// `tcl::unsupported::representation` of this value under `release`: its
    /// rep as C names the type, its handle count and its address
    /// ([`tcl_cmd_core::representation::describe`]). A boolean read from a
    /// word (`yes`) is C's `boolean` (`booleanString` before 9.0); one a
    /// comparison made is C's `int`.
    pub(crate) fn representation(&self, release: tcl_dialect::TclVersion) -> Vec<u8> {
        use tcl_cmd_core::representation::{Representation, describe};
        let string = self.0.string.borrow().as_ref().map(RawString::bytes);
        let address = |p: *const ()| p as usize;
        let modern = release >= tcl_dialect::TclVersion::V9_0;
        let (type_name, internal) =
            Self::representation_payload(&self.0.intrep.borrow(), string.as_deref(), modern);
        describe(
            &Representation {
                type_name,
                ref_count: Rc::strong_count(&self.0),
                address: address(Rc::as_ptr(&self.0).cast()),
                internal,
                string: string.as_deref(),
            },
            release,
        )
    }

    fn representation_payload(
        intrep: &IntRep,
        string: Option<&[u8]>,
        modern: bool,
    ) -> (Option<&'static str>, tcl_cmd_core::representation::Internal) {
        use tcl_cmd_core::representation::Internal;
        let address = |p: *const ()| p as usize;
        let boolean_word = |word: bool| match (word, modern) {
            (false, _) => "int",
            (true, true) => "boolean",
            (true, false) => "booleanString",
        };
        match intrep {
            IntRep::Str => (None, Internal::None),
            IntRep::NativeString { .. } | IntRep::JimString(_) => {
                (Some("string"), Internal::Words(0, 0))
            }
            IntRep::ByteArray(rep) => (
                Some(if modern {
                    "proper bytearray"
                } else {
                    "bytearray"
                }),
                Internal::Words(address(Rc::as_ptr(rep).cast()), 0),
            ),
            IntRep::JimSource(_) => (Some("source"), Internal::Words(0, 0)),
            IntRep::JimScript(_) => (Some("script"), Internal::Words(0, 0)),
            IntRep::JimDictionarySubstitution { .. } => {
                (Some("dict-substitution"), Internal::Words(0, 0))
            }
            IntRep::JimInterpolated(_) => (Some("interpolated"), Internal::Words(0, 0)),
            IntRep::JimScriptLine { argc, line } => (
                Some("scriptline"),
                Internal::Words(
                    usize::try_from(*argc).unwrap_or(0),
                    usize::try_from(*line).unwrap_or(0),
                ),
            ),
            IntRep::Expression(rep) => (
                Some("exprcode"),
                Internal::Words(address(Rc::as_ptr(rep).cast()), 0),
            ),
            IntRep::NativeRegexp(_) | IntRep::JimRegexp(_) => {
                (Some("regexp"), Internal::Words(0, 0))
            }
            IntRep::JimIndex(_) => (Some("index"), Internal::Words(0, 0)),
            IntRep::NativeEndOffset(_) => (Some("end-offset"), Internal::Words(0, 0)),
            IntRep::NativePropertyName(_) => (Some("TclOO property name"), Internal::Words(0, 0)),
            IntRep::NativeInstructionName(_) => (Some("instname"), Internal::Words(0, 0)),
            IntRep::NativeMethodName(rep) => (
                Some("TclOO method name"),
                Internal::Words(address(Rc::as_ptr(rep).cast()), 0),
            ),
            IntRep::NativeBytecode(rep) => (
                Some("bytecode"),
                Internal::Words(address(Rc::as_ptr(rep).cast()), 0),
            ),
            IntRep::Int(n) | IntRep::Tcl84Long(n) => {
                (Some("int"), Internal::Bits(n.cast_unsigned()))
            }
            IntRep::Tcl84Wide(n) => (Some("wideInt"), Internal::Bits(n.cast_unsigned())),
            IntRep::Big { .. } => (Some("bignum"), Internal::Words(0, 0)),
            IntRep::Double(x) => (Some("double"), Internal::Double(*x)),
            #[allow(clippy::cast_precision_loss)]
            IntRep::CoercedDouble(n) => (Some("coerced-double"), Internal::Double(*n as f64)),
            IntRep::Bool(b) => {
                let word = string.is_some_and(|s| s != b"0".as_slice() && s != b"1".as_slice());
                (
                    Some(boolean_word(word)),
                    Internal::Words(usize::from(*b), 0),
                )
            }
            IntRep::WordBoolean { value, .. } => (
                Some(boolean_word(true)),
                Internal::Words(usize::from(*value), 0),
            ),
            IntRep::CompletionCode(_) | IntRep::NativeIndex { .. } => {
                (Some("index"), Internal::Words(0, 0))
            }
            IntRep::FrameLevel { .. } => (Some("levelReference"), Internal::Words(0, 0)),
            IntRep::NativeArraySearch { .. } => (Some("array search"), Internal::Words(0, 0)),
            IntRep::NativeCommandName(_) | IntRep::NativeCommandNameUnresolved(_) => {
                (Some("cmdName"), Internal::Words(0, 0))
            }
            IntRep::JimCommand(_) => (Some("command"), Internal::Words(0, 0)),
            IntRep::JimVariable(_) => (Some("variable"), Internal::Words(0, 0)),
            IntRep::JimOption(_) => (Some("enum"), Internal::Words(0, 0)),
            IntRep::JimSubcommand(_) => (Some("subcmd"), Internal::Words(0, 0)),
            IntRep::NativeParsedVariableName(_) => (Some("parsedVarName"), Internal::Words(0, 0)),
            IntRep::NativeLocalVariableName(_) => (Some("localVarName"), Internal::Words(0, 0)),
            IntRep::NativeNamespaceName(_) => (Some("nsName"), Internal::Words(0, 0)),
            IntRep::List { canonical, .. } => (
                Some("list"),
                Internal::Words(address(Rc::as_ptr(canonical).cast()), 0),
            ),
            IntRep::Dict(rep) => (
                Some("dict"),
                Internal::Words(address(Rc::as_ptr(rep).cast()), 0),
            ),
        }
    }
}

impl Value {
    /// The bytes this value measures to without rendering it, stopping once
    /// past `cap`: its string where it has one, a number's digits, and a
    /// list's or dict's elements plus a separator between each (the quoting
    /// rendering may add is not counted). A list is measured through its
    /// elements rather than rendered, so a list holding itself twice, nested
    /// a few dozen deep, costs one pass instead of more bytes than any host
    /// has (the value-size budget, `charge_value_size`).
    pub(crate) fn measured_bytes(&self, cap: u64) -> u64 {
        self.measured_bytes_at_depth(cap, 0)
    }

    fn measured_bytes_at_depth(&self, cap: u64, depth: u32) -> u64 {
        const MAX_DEPTH: u32 = 1000;
        if let Some(s) = self.0.string.borrow().as_ref() {
            return s.bytes().len() as u64;
        }
        let measured = {
            let rep = self.0.intrep.borrow();
            match &*rep {
                IntRep::Str => Some(0),
                IntRep::Bool(_) => Some(1),
                IntRep::Int(n) | IntRep::Tcl84Long(n) | IntRep::Tcl84Wide(n) => Some(
                    u64::from(n.unsigned_abs().checked_ilog10().unwrap_or(0) + 1)
                        + u64::from(*n < 0),
                ),
                _ if depth >= MAX_DEPTH => Some(1),
                IntRep::List { items, .. } => Some(measure_elements(items.iter(), cap, depth)),
                _ => None,
            }
        };
        measured.unwrap_or_else(|| self.string_bytes().len() as u64)
    }
}

/// The bytes `items` measure to with a separator between each, stopping once
/// past `cap`.
fn measure_elements<'a>(items: impl Iterator<Item = &'a Value>, cap: u64, depth: u32) -> u64 {
    let mut total = 0u64;
    for (i, item) in items.enumerate() {
        total = total
            .saturating_add(u64::from(i > 0))
            .saturating_add(item.measured_bytes_at_depth(cap.saturating_sub(total), depth + 1));
        if total > cap {
            break;
        }
    }
    total
}
