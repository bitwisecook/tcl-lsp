// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Registry-owned, typed edits for lifecycle deprecations.
//!
//! A lifecycle fact says *when* syntax is deprecated. This module supplies
//! hooks that a generic consumer resolves against the selected dialect and
//! surrounding invocation words. The hooks name syntax locations, never
//! commands, so one consumer serves commands, subcommands, options, literal
//! values, and shape replacements.

/// One invocation word exposed to a lifecycle-deprecation resolver.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeprecationFixWord<'a> {
    /// The source spelling after Tcl word parsing.
    pub spelling: &'a str,
    /// Whether the spelling is statically literal rather than computed.
    pub literal: bool,
}

/// Generic context supplied to a registry lifecycle-deprecation resolver.
#[derive(Debug, Clone, Copy)]
pub struct DeprecationFixContext<'a> {
    /// Invocation words, including the command head at index zero.
    pub words: &'a [DeprecationFixWord<'a>],
    /// Index of the literal word carrying the lifecycle declaration.
    pub matched_word_index: usize,
    /// Active dialect/profile name, when the caller resolved one.
    pub dialect: Option<&'a str>,
    /// Resolved lifecycle-version floor on the selected axis.
    pub effective_version: Option<&'a str>,
}

/// The source range a resolved lifecycle-deprecation edit replaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeprecationFixTarget {
    /// A specific invocation word, indexed from the command head.
    Word(usize),
    /// The complete invocation, for a shape-changing replacement.
    Invocation,
}

/// Safety classification attached to a registry deprecation rewrite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeprecationFixSafety {
    /// The replacement preserves the invocation's semantics and is safe for
    /// an editor's bulk safe-fix action.
    SemanticsEquivalent,
    /// The rewrite is well-formed but needs user review.
    RequiresReview,
}

/// An owned edit plan returned by a lifecycle-deprecation hook.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeprecationFixPlan {
    /// Which invocation range the consumer must replace.
    pub target: DeprecationFixTarget,
    /// Replacement Tcl source text.
    pub new_text: String,
    /// User-facing code-action description.
    pub description: String,
    /// Whether automatic application is safe.
    pub safety: DeprecationFixSafety,
}

/// Typed callback for a lifecycle-deprecation edit resolver.
///
/// It receives only generic syntax/context facts. A callback may abstain when
/// dynamic words or an unfamiliar shape make an automatic edit unsound.
pub type DeprecationFixResolver =
    for<'a> fn(DeprecationFixContext<'a>) -> Option<DeprecationFixPlan>;

/// Typed resolver hook for a lifecycle-deprecation quick fix.
#[allow(unpredictable_function_pointer_comparisons)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeprecationFixHook {
    /// Replace the literal word that matched the deprecated lifecycle entry.
    ReplaceMatchedWord {
        /// Replacement spelling.
        replacement: &'static str,
        /// Code-action label.
        description: &'static str,
        /// Automatic-application safety.
        safety: DeprecationFixSafety,
    },
    /// Replace a positional word after the matched lifecycle word.
    ReplaceArgument {
        /// Zero-based word index after the matched lifecycle word.
        index: u8,
        /// Replacement Tcl source text.
        replacement: &'static str,
        /// Code-action label.
        description: &'static str,
        /// Automatic-application safety.
        safety: DeprecationFixSafety,
    },
    /// Replace the complete invocation with a registry-provided shape.
    ReplaceInvocation {
        /// Replacement Tcl source text.
        replacement: &'static str,
        /// Code-action label.
        description: &'static str,
        /// Automatic-application safety.
        safety: DeprecationFixSafety,
    },
    /// Invoke a registry callback that can inspect generic surrounding words,
    /// the selected dialect, and the resolved version before planning an edit.
    Custom {
        /// Registry-selected callback.
        resolver: DeprecationFixResolver,
    },
}

impl DeprecationFixHook {
    /// Resolve this hook to an owned edit plan for a generic syntax consumer.
    #[must_use]
    pub fn resolve(self, context: DeprecationFixContext<'_>) -> Option<DeprecationFixPlan> {
        let (target, replacement, description, safety) = match self {
            Self::ReplaceMatchedWord {
                replacement,
                description,
                safety,
            } => (
                DeprecationFixTarget::Word(context.matched_word_index),
                replacement,
                description,
                safety,
            ),
            Self::ReplaceArgument {
                index,
                replacement,
                description,
                safety,
            } => (
                DeprecationFixTarget::Word(context.matched_word_index + 1 + usize::from(index)),
                replacement,
                description,
                safety,
            ),
            Self::ReplaceInvocation {
                replacement,
                description,
                safety,
            } => (
                DeprecationFixTarget::Invocation,
                replacement,
                description,
                safety,
            ),
            Self::Custom { resolver } => return resolver(context),
        };
        context.words.get(match target {
            DeprecationFixTarget::Word(index) => index,
            DeprecationFixTarget::Invocation => context.matched_word_index,
        })?;
        Some(DeprecationFixPlan {
            target,
            new_text: replacement.to_owned(),
            description: description.to_owned(),
            safety,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DeprecationFixContext, DeprecationFixHook, DeprecationFixPlan, DeprecationFixSafety,
        DeprecationFixTarget, DeprecationFixWord,
    };

    fn only_when_followed_by_keep(
        context: DeprecationFixContext<'_>,
    ) -> Option<DeprecationFixPlan> {
        (context.dialect == Some("tcl9.0")
            && context.effective_version == Some("9.0")
            && context.words.get(context.matched_word_index + 1)?.spelling == "keep")
            .then(|| DeprecationFixPlan {
                target: DeprecationFixTarget::Word(context.matched_word_index),
                new_text: "modern".to_owned(),
                description: "Use modern with keep".to_owned(),
                safety: DeprecationFixSafety::SemanticsEquivalent,
            })
    }

    #[test]
    fn typed_hooks_cover_word_argument_shape_and_contextual_edits() {
        let words = [
            DeprecationFixWord {
                spelling: "cmd",
                literal: true,
            },
            DeprecationFixWord {
                spelling: "old",
                literal: true,
            },
            DeprecationFixWord {
                spelling: "keep",
                literal: true,
            },
        ];
        let context = DeprecationFixContext {
            words: &words,
            matched_word_index: 1,
            dialect: Some("tcl9.0"),
            effective_version: Some("9.0"),
        };
        let word = DeprecationFixHook::ReplaceMatchedWord {
            replacement: "new",
            description: "Use new",
            safety: DeprecationFixSafety::SemanticsEquivalent,
        }
        .resolve(context)
        .unwrap();
        assert_eq!(word.target, DeprecationFixTarget::Word(1));
        let argument = DeprecationFixHook::ReplaceArgument {
            index: 0,
            replacement: "new-value",
            description: "Use new value",
            safety: DeprecationFixSafety::RequiresReview,
        }
        .resolve(context)
        .unwrap();
        assert_eq!(argument.target, DeprecationFixTarget::Word(2));
        let shape = DeprecationFixHook::ReplaceInvocation {
            replacement: "new shape",
            description: "Use new shape",
            safety: DeprecationFixSafety::RequiresReview,
        }
        .resolve(context)
        .unwrap();
        assert_eq!(shape.target, DeprecationFixTarget::Invocation);
        let custom = DeprecationFixHook::Custom {
            resolver: only_when_followed_by_keep,
        }
        .resolve(context)
        .unwrap();
        assert_eq!(custom.new_text, "modern");
    }
}
