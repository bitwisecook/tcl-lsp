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

use crate::hooks::{CodegenHookId, InlineCodegenHookId};

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
}

impl IntrinsicId {
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
}
