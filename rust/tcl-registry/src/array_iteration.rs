// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Caller-frame array iteration arguments, separately from compiler selection.

/// Actual C Tcl 9 private iterator worker required by the stock public ensemble.
pub const IMPLEMENTATION_LOOKUP: crate::native_compilation::NativeCompilerImplementationLookup =
    crate::native_compilation::NativeCompilerImplementationLookup {
        ensemble: "::array",
        member: "for",
        slot: "::tcl::array::for",
        command: "array",
        prepended: &["for"],
    };

/// Frozen names and body position accepted by the selected array iterator.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArrayIterationPlan {
    /// Original binding-list operand in effective argv, excluding the head.
    pub bindings_at: usize,
    /// Original array-name operand.
    pub array_at: usize,
    /// Original body operand.
    pub body_at: usize,
    /// Key destination, resolved anew before each iteration's first store.
    pub key_name: String,
    /// Value destination, resolved after the preceding key store's observers.
    pub value_name: String,
    /// Array name whose physical array lookup must succeed before body entry.
    pub array_name: String,
}

/// Argument selection does not establish the array's existence or body entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArrayIterationSelection {
    /// The native positional/list grammar is closed.
    Valid(ArrayIterationPlan),
    /// Known arguments fail before any iteration bindings or body entry.
    Invalid,
    /// Unknown argument bytes, cardinality or native policy retain uncertainty.
    Unknown,
}

/// Select C Tcl 9's native array iteration grammar from frozen operands.
#[must_use]
pub fn select(arguments: crate::InvocationArguments<'_>, offset: usize) -> ArrayIterationSelection {
    use ArrayIterationSelection as Selection;
    let Some(dialect) = arguments.dialect() else {
        return Selection::Unknown;
    };
    if dialect.family() != Some(tcl_dialect::model::Family::Tcl)
        || dialect
            .tcl_version
            .is_none_or(|version| version < tcl_dialect::TclVersion::V9_0)
    {
        return Selection::Unknown;
    }
    let Some(count) = arguments.exact_argv_len() else {
        return Selection::Unknown;
    };
    if count.checked_sub(offset) != Some(3) {
        return Selection::Invalid;
    }
    let Some(bindings) = arguments.literal_at(offset) else {
        return Selection::Unknown;
    };
    let Ok(names) = tcl_syntax::list::split_list(bindings) else {
        return Selection::Invalid;
    };
    let [key, value] = names.as_slice() else {
        return Selection::Invalid;
    };
    let Some(array) = arguments.literal_at(offset + 1) else {
        return Selection::Unknown;
    };
    Selection::Valid(ArrayIterationPlan {
        bindings_at: offset,
        array_at: offset + 1,
        body_at: offset + 2,
        key_name: key.to_string(),
        value_name: value.to_string(),
        array_name: array.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn array_iteration_requires_two_frozen_names_before_body_entry() {
        let dialect = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0);
        for (bindings, valid) in [
            ("k v", true),
            ("k", false),
            ("k v extra", false),
            ("{", false),
        ] {
            let arguments = ["for", bindings, "a", "body"];
            let words = crate::InvocationWords::literals("array", &arguments).with_dialect(dialect);
            assert_eq!(
                matches!(
                    select(words.arguments(), 1),
                    ArrayIterationSelection::Valid(_)
                ),
                valid
            );
        }
    }
}
