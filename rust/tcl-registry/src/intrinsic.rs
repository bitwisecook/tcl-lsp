// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Target-neutral identities for registry-declared intrinsic operations.
//!
//! An intrinsic identifies semantics that more than one target may implement.
//! It is not a promise that specialisation is legal: live command binding,
//! trace, interpreter, effect, and representation proofs remain separate.

use tcl_dialect::{StringCharacterModel, TclVersion};

use crate::hooks::{CodegenHookId, InlineCodegenHookId, LoweringHookId};
use crate::semantic_operation::SemanticOperationId;

const RUNTIME_INVARIANT_SEMANTICS: u32 = 0;
const TCL8_UTF16_STRING_SEMANTICS: u32 = 1;
const TCL9_SCALAR_STRING_SEMANTICS: u32 = 2;
const INVARIANT_SEMANTICS: &[u32] = &[RUNTIME_INVARIANT_SEMANTICS];
const VERSIONED_STRING_SEMANTICS: &[u32] =
    &[TCL8_UTF16_STRING_SEMANTICS, TCL9_SCALAR_STRING_SEMANTICS];

/// Target-neutral identity of a registry-described intrinsic operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum IntrinsicId {
    /// Assign list elements to variables.
    ListAssign,
    /// Return a list's length.
    ListLength,
    /// Select one or more list elements by index.
    ListIndex,
    /// Select an inclusive list range.
    ListRange,
    /// Replace an inclusive list range.
    ListReplace,
    /// Insert elements into a list.
    ListInsert,
    /// Update a list held in a variable.
    ListSet,
    /// Construct a Tcl list from arguments.
    ListConstruct,
    /// Read a nested dictionary key path.
    DictGet,
    /// Set a nested dictionary key path in a variable.
    DictSet,
    /// Remove a nested dictionary key path from a variable.
    DictUnset,
    /// Increment a dictionary entry held in a variable.
    DictIncr,
    /// Append text to a dictionary entry held in a variable.
    DictAppend,
    /// Append a list element to a dictionary entry held in a variable.
    DictListAppend,
    /// Select one character from a string.
    StringIndex,
    /// Select an inclusive string range.
    StringRange,
    /// Compare strings for equality.
    StringEqual,
    /// Lexicographically compare strings.
    StringCompare,
    /// Replace an inclusive string range.
    StringReplace,
    /// Return a string's character length.
    StringLength,
    /// Test whether a string belongs to a declared character class.
    StringIs,
    /// Match a regular expression.
    Regexp,
    /// Test whether a variable exists.
    InfoExists,
    /// Test whether an array variable exists.
    ArrayExists,
    /// Return array element names.
    ArrayNames,
    /// Return an array's element count.
    ArraySize,
    /// Concatenate lists.
    Concat,
    /// Write a value to a Tcl channel.
    ///
    /// The operation covers the complete `puts` semantic surface. Backends must
    /// still select only invocation forms whose option and channel shape they
    /// implement directly.
    ChannelWrite,
}

impl IntrinsicId {
    /// Every intrinsic in stable-ID order.
    pub const ALL: &'static [Self] = &[
        Self::ListAssign,
        Self::ListLength,
        Self::ListIndex,
        Self::ListRange,
        Self::ListReplace,
        Self::ListInsert,
        Self::ListSet,
        Self::ListConstruct,
        Self::DictGet,
        Self::DictSet,
        Self::DictUnset,
        Self::DictIncr,
        Self::DictAppend,
        Self::DictListAppend,
        Self::StringIndex,
        Self::StringRange,
        Self::StringEqual,
        Self::StringCompare,
        Self::StringReplace,
        Self::StringLength,
        Self::StringIs,
        Self::Regexp,
        Self::InfoExists,
        Self::ArrayExists,
        Self::ArrayNames,
        Self::ArraySize,
        Self::Concat,
        Self::ChannelWrite,
    ];

    /// Explicit stable scalar identity for offline artefacts and runtime guards.
    ///
    /// These values are intentionally grouped and matched, never derived from
    /// enum declaration order or an integer cast.
    #[must_use]
    pub const fn stable_id(self) -> u32 {
        match self {
            Self::ListAssign => 0x0101,
            Self::ListLength => 0x0102,
            Self::ListIndex => 0x0103,
            Self::ListRange => 0x0104,
            Self::ListReplace => 0x0105,
            Self::ListInsert => 0x0106,
            Self::ListSet => 0x0107,
            Self::ListConstruct => 0x0108,
            Self::DictGet => 0x0201,
            Self::DictSet => 0x0202,
            Self::DictUnset => 0x0203,
            Self::DictIncr => 0x0204,
            Self::DictAppend => 0x0205,
            Self::DictListAppend => 0x0206,
            Self::StringIndex => 0x0301,
            Self::StringRange => 0x0302,
            Self::StringEqual => 0x0303,
            Self::StringCompare => 0x0304,
            Self::StringReplace => 0x0305,
            Self::StringLength => 0x0306,
            Self::StringIs => 0x0307,
            Self::Regexp => 0x0401,
            Self::InfoExists => 0x0501,
            Self::ArrayExists => 0x0601,
            Self::ArrayNames => 0x0602,
            Self::ArraySize => 0x0603,
            Self::Concat => 0x0701,
            Self::ChannelWrite => 0x0801,
        }
    }

    /// Stable runtime-semantics key included in guarded implementation identity.
    ///
    /// Most intrinsics currently have one release-invariant implementation
    /// contract. `StringLength` is versioned because Tcl 8 counts UTF-16-style
    /// `Tcl_UniChar` units while Tcl 9 counts Unicode scalar values.
    #[must_use]
    pub const fn guard_semantics_key(self, runtime: TclVersion) -> u32 {
        match self {
            Self::StringLength => match runtime.string_character_model() {
                StringCharacterModel::Utf16CodeUnits => TCL8_UTF16_STRING_SEMANTICS,
                StringCharacterModel::UnicodeScalars => TCL9_SCALAR_STRING_SEMANTICS,
            },
            _ => RUNTIME_INVARIANT_SEMANTICS,
        }
    }

    /// Every guarded runtime-semantics key this intrinsic may attest.
    #[must_use]
    pub const fn guard_semantics_variants(self) -> &'static [u32] {
        match self {
            Self::StringLength => VERSIONED_STRING_SEMANTICS,
            _ => INVARIANT_SEMANTICS,
        }
    }

    /// Recover an intrinsic from its explicit stable scalar identity.
    #[must_use]
    pub const fn from_stable_id(id: u32) -> Option<Self> {
        let mut index = 0;
        while index < Self::ALL.len() {
            let intrinsic = Self::ALL[index];
            if intrinsic.stable_id() == id {
                return Some(intrinsic);
            }
            index += 1;
        }
        None
    }

    pub(crate) const fn from_descriptors(
        semantic: Option<SemanticOperationId>,
        lowering: Option<LoweringHookId>,
        codegen: Option<CodegenHookId>,
        inline_codegen: Option<InlineCodegenHookId>,
    ) -> Option<Self> {
        match semantic {
            Some(SemanticOperationId::Intrinsic(id)) => Some(id),
            Some(_) => None,
            None if lowering.is_some() => None,
            None => {
                let inline = match inline_codegen {
                    Some(hook) => Self::from_legacy_inline_codegen(hook),
                    None => None,
                };
                match inline {
                    Some(id) => Some(id),
                    None => match codegen {
                        Some(hook) => Self::from_legacy_codegen(hook),
                        None => None,
                    },
                }
            }
        }
    }

    /// Stable compiler and Explorer spelling for this semantic identity.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ListAssign => "list-assign",
            Self::ListLength => "list-length",
            Self::ListIndex => "list-index",
            Self::ListRange => "list-range",
            Self::ListReplace => "list-replace",
            Self::ListInsert => "list-insert",
            Self::ListSet => "list-set",
            Self::ListConstruct => "list-construct",
            Self::DictGet => "dict-get",
            Self::DictSet => "dict-set",
            Self::DictUnset => "dict-unset",
            Self::DictIncr => "dict-incr",
            Self::DictAppend => "dict-append",
            Self::DictListAppend => "dict-list-append",
            Self::StringIndex => "string-index",
            Self::StringRange => "string-range",
            Self::StringEqual => "string-equal",
            Self::StringCompare => "string-compare",
            Self::StringReplace => "string-replace",
            Self::StringLength => "string-length",
            Self::StringIs => "string-is",
            Self::Regexp => "regexp",
            Self::InfoExists => "info-exists",
            Self::ArrayExists => "array-exists",
            Self::ArrayNames => "array-names",
            Self::ArraySize => "array-size",
            Self::Concat => "concat",
            Self::ChannelWrite => "channel-write",
        }
    }

    /// Compatibility projection from a legacy statement-position `TclVM` hook.
    ///
    /// This mapping temporarily lets existing registry specs declare common
    /// intrinsic identity without duplicating every hook stamp. New specs
    /// should prefer an explicit [`crate::SemanticOperationId::Intrinsic`]
    /// declaration; the legacy hook remains only for bytecode emission.
    #[must_use]
    pub const fn from_legacy_codegen(hook: CodegenHookId) -> Option<Self> {
        match hook {
            CodegenHookId::Lassign => Some(Self::ListAssign),
            CodegenHookId::Llength => Some(Self::ListLength),
            CodegenHookId::Lrange => Some(Self::ListRange),
            CodegenHookId::Linsert => Some(Self::ListInsert),
            CodegenHookId::Lset => Some(Self::ListSet),
            CodegenHookId::Concat => Some(Self::Concat),
            CodegenHookId::Append
            | CodegenHookId::Lappend
            | CodegenHookId::Unset
            | CodegenHookId::Tailcall
            | CodegenHookId::Global
            | CodegenHookId::Upvar
            | CodegenHookId::Dict
            | CodegenHookId::Array
            | CodegenHookId::Namespace => None,
        }
    }

    /// Compatibility projection from a legacy value/catch-position `TclVM` hook.
    ///
    /// See [`Self::from_legacy_codegen`] for the migration contract.
    #[must_use]
    pub const fn from_legacy_inline_codegen(hook: InlineCodegenHookId) -> Option<Self> {
        match hook {
            InlineCodegenHookId::InfoExists => Some(Self::InfoExists),
            InlineCodegenHookId::Lindex => Some(Self::ListIndex),
            InlineCodegenHookId::Lrange => Some(Self::ListRange),
            InlineCodegenHookId::Lreplace => Some(Self::ListReplace),
            InlineCodegenHookId::Linsert => Some(Self::ListInsert),
            InlineCodegenHookId::Regexp => Some(Self::Regexp),
            InlineCodegenHookId::List => Some(Self::ListConstruct),
            InlineCodegenHookId::DictGet => Some(Self::DictGet),
            InlineCodegenHookId::Expr
            | InlineCodegenHookId::Incr
            | InlineCodegenHookId::Catch
            | InlineCodegenHookId::Return
            | InlineCodegenHookId::Error
            | InlineCodegenHookId::Break
            | InlineCodegenHookId::Continue
            | InlineCodegenHookId::Try
            | InlineCodegenHookId::String
            | InlineCodegenHookId::Array => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlapping_legacy_hooks_share_one_intrinsic_identity() {
        assert_eq!(
            IntrinsicId::from_legacy_codegen(CodegenHookId::Lrange),
            IntrinsicId::from_legacy_inline_codegen(InlineCodegenHookId::Lrange)
        );
        assert_eq!(
            IntrinsicId::from_legacy_codegen(CodegenHookId::Linsert),
            IntrinsicId::from_legacy_inline_codegen(InlineCodegenHookId::Linsert)
        );
        assert_eq!(IntrinsicId::from_legacy_codegen(CodegenHookId::Array), None);
        assert_eq!(
            IntrinsicId::from_legacy_inline_codegen(InlineCodegenHookId::String),
            None
        );
    }

    #[test]
    fn stable_ids_round_trip_without_ordinal_dependence() {
        let mut ids = std::collections::BTreeSet::new();
        for &intrinsic in IntrinsicId::ALL {
            assert!(ids.insert(intrinsic.stable_id()));
            assert_eq!(
                IntrinsicId::from_stable_id(intrinsic.stable_id()),
                Some(intrinsic)
            );
        }
        assert_eq!(IntrinsicId::StringLength.stable_id(), 0x0306);
        assert_eq!(IntrinsicId::from_stable_id(0), None);
    }

    #[test]
    fn string_length_guard_identity_is_runtime_semantics_versioned() {
        assert_ne!(
            IntrinsicId::StringLength.guard_semantics_key(tcl_dialect::TclVersion::V8_6),
            IntrinsicId::StringLength.guard_semantics_key(tcl_dialect::TclVersion::V9_0)
        );
        assert_eq!(
            IntrinsicId::ListLength.guard_semantics_key(tcl_dialect::TclVersion::V8_6),
            IntrinsicId::ListLength.guard_semantics_key(tcl_dialect::TclVersion::V9_0)
        );
    }

    #[test]
    fn command_spec_collects_intrinsics_from_its_subcommands() {
        let registry = crate::CommandRegistry::build_default();
        let string = registry.get("string").expect("string spec");
        let ids = string.intrinsic_ids();
        for expected in [
            IntrinsicId::StringCompare,
            IntrinsicId::StringEqual,
            IntrinsicId::StringIndex,
            IntrinsicId::StringIs,
            IntrinsicId::StringLength,
            IntrinsicId::StringRange,
            IntrinsicId::StringReplace,
        ] {
            assert!(ids.contains(&expected), "missing {expected:?}");
        }
    }
}
