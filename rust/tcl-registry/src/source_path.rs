// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Typed conditional source path algebra, independent of native execution.

/// Authored source operation supported by the shared navigation algebra.
/// Its descriptor supplies no runtime handler, current filename, filesystem
/// state or successful evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourcePathOperation {
    /// Join argument values in the explicit slash-form path algebra.
    Join,
    /// Select the parent directory of one argument value.
    Dirname,
    /// Lexically normalise one anchored argument value.
    Normalize,
    /// Use the document filename independently supplied by the consumer.
    ScriptPath,
}

impl SourcePathOperation {
    /// Complete authoring vocabulary.
    pub const ALL: &'static [Self] =
        &[Self::Join, Self::Dirname, Self::Normalize, Self::ScriptPath];

    /// Exact argument cardinality supported by the source algebra.
    /// Runtime arity and evaluation remain independent.
    #[must_use]
    pub const fn accepts_argument_count(self, count: usize) -> bool {
        match self {
            Self::Join => count >= 1,
            Self::Dirname | Self::Normalize => count == 1,
            Self::ScriptPath => count == 0,
        }
    }
}

/// Selected operation and effective post-head values from one source schema.
/// The range includes genuine bound/prefix values after its member selector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcePathInvocation {
    /// The descriptor's conditional source algebra operation.
    pub operation: SourcePathOperation,
    /// Exact effective post-head argument ordinals, excluding the selector.
    pub arguments: std::ops::Range<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authored_source_path_operations_keep_selected_members_and_exact_effective_argv() {
        // naming.navigation.retained-path-source-inventory
        // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
        let registry = crate::CommandRegistry::build_default();
        let query = |head: &str, args: &[&str]| {
            registry
                .resolve_invocation(head, args, None)?
                .authored_source_path_operation()
        };
        assert_eq!(
            query("file", &["join", "/ROOT", "sub"]),
            Some(SourcePathInvocation {
                operation: SourcePathOperation::Join,
                arguments: 1..3,
            })
        );
        assert_eq!(
            query("file", &["dirn", "/ROOT/sub"]),
            Some(SourcePathInvocation {
                operation: SourcePathOperation::Dirname,
                arguments: 1..2,
            })
        );
        assert_eq!(
            query("info", &["script"]),
            Some(SourcePathInvocation {
                operation: SourcePathOperation::ScriptPath,
                arguments: 1..1,
            })
        );
        assert_eq!(query("list", &["join", "/ROOT", "sub"]), None);
        assert_eq!(query("file", &["join"]), None);
        assert_eq!(query("info", &["script", "/override"]), None);
        let args = [
            crate::InvocationWord::Literal("join"),
            crate::InvocationWord::Expanded,
        ];
        let words =
            crate::InvocationWords::structured(crate::InvocationWord::Literal("file"), &args);
        assert!(
            registry
                .resolve_structured_invocation(words, None)
                .resolved()
                .and_then(|invocation| invocation.authored_source_path_operation())
                .is_none()
        );
    }
}
