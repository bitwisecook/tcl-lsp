// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional source index advice, independently of native dispatch or values.

/// Literal container/index relationship declared by a selected source schema.
/// The descriptor grants no handler identity, successful result or current value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceIndexBounds {
    /// Descend through a literal list using each original index operand.
    ListIndex,
    /// Select a list slice using first and last indices.
    ListRange,
    /// Replace a list slice using first and last indices.
    ListReplace,
    /// Select one character from a literal string.
    StringIndex,
    /// Select a string slice using first and last indices.
    StringRange,
    /// Replace a string slice using first and last indices.
    StringReplace,
    /// Insert into a string using one index and one payload operand.
    StringInsert,
}

impl SourceIndexBounds {
    /// Complete authoring vocabulary.
    pub const ALL: &'static [Self] = &[
        Self::ListIndex,
        Self::ListRange,
        Self::ListReplace,
        Self::StringIndex,
        Self::StringRange,
        Self::StringReplace,
        Self::StringInsert,
    ];

    /// Exact source shapes on this operation's post-selector count axis.
    /// Runtime argument acceptance remains independent.
    #[must_use]
    pub const fn accepts_argument_count(self, count: usize) -> bool {
        match self {
            Self::ListIndex => count >= 2,
            Self::ListRange | Self::StringRange | Self::StringInsert => count == 3,
            Self::ListReplace => count >= 3,
            Self::StringIndex => count == 2,
            Self::StringReplace => count == 3 || count == 4,
        }
    }

    pub(crate) const fn from_operation(operation: crate::SemanticOperationId) -> Option<Self> {
        use crate::{IntrinsicId as I, SemanticOperationId as O};
        match operation {
            O::Intrinsic(I::ListIndex) => Some(Self::ListIndex),
            O::Intrinsic(I::ListRange) => Some(Self::ListRange),
            O::Intrinsic(I::ListReplace) => Some(Self::ListReplace),
            O::Intrinsic(I::StringIndex) => Some(Self::StringIndex),
            O::Intrinsic(I::StringRange) => Some(Self::StringRange),
            O::Intrinsic(I::StringReplace) => Some(Self::StringReplace),
            _ => None,
        }
    }
}

/// Selected source index relationship and its original effective operands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceIndexBoundsInvocation {
    /// Descriptor-selected conditional relationship.
    pub operation: SourceIndexBounds,
    /// Original post-head operands after any selected member selector.
    pub arguments: std::ops::Range<usize>,
    /// Independently retained grammar and character rules, if supplied.
    pub dialect: Option<crate::InvocationDialect>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authored_index_bounds_keep_selected_operations_and_effective_count_axes() {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        let context = crate::model::ingress::static_context_for("tcl9.0");
        let registry = context.commands();
        for (head, args, operation, range) in [
            (
                "lindex",
                vec!["a b", "9"],
                SourceIndexBounds::ListIndex,
                0..2,
            ),
            (
                "lrange",
                vec!["a b", "8", "9"],
                SourceIndexBounds::ListRange,
                0..3,
            ),
            (
                "lreplace",
                vec!["a b", "8", "9", "x"],
                SourceIndexBounds::ListReplace,
                0..4,
            ),
            (
                "string",
                vec!["index", "abc", "9"],
                SourceIndexBounds::StringIndex,
                1..3,
            ),
            (
                "string",
                vec!["range", "abc", "8", "9"],
                SourceIndexBounds::StringRange,
                1..4,
            ),
            (
                "string",
                vec!["replace", "abc", "8", "9", "x"],
                SourceIndexBounds::StringReplace,
                1..5,
            ),
            (
                "string",
                vec!["insert", "abc", "-1", "x"],
                SourceIndexBounds::StringInsert,
                1..4,
            ),
        ] {
            let selected = crate::model::assembly::resolve_invocation_in_context(
                registry,
                Some(context.context()),
                head,
                &args,
            )
            .unwrap();
            let index = selected.authored_source_index_bounds().unwrap();
            assert_eq!(index.operation, operation, "{head} {args:?}");
            assert_eq!(index.arguments, range);
        }
        for (head, args) in [
            ("lindex", vec!["a b"]),
            ("string", vec!["length", "abc"]),
            ("string", vec!["index", "abc", "9", "extra"]),
        ] {
            assert!(
                crate::model::assembly::resolve_invocation_in_context(
                    registry,
                    Some(context.context()),
                    head,
                    &args
                )
                .unwrap()
                .authored_source_index_bounds()
                .is_none()
            );
        }
        let args = [
            crate::InvocationWord::Literal("a b"),
            crate::InvocationWord::Expanded,
        ];
        let words =
            crate::InvocationWords::structured(crate::InvocationWord::Literal("lindex"), &args);
        assert!(
            registry
                .resolve_structured_invocation(words, Some(context.context().authoring_query()))
                .resolved()
                .unwrap()
                .authored_source_index_bounds()
                .is_none()
        );
    }
}
