// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Audited native try compiler selection, sharing the runtime clause grammar.
//!
//! Source: pinned C8.6.18/9.0.4/9.1.0 `TclCompileTryCmd` and `IssueTry*Instructions`.
//! Handler clauses allocate anonymous locals; finally-only compilation does not.

use crate::native_compilation::{
    NativeCompilationContext, NativeCompilationFailureScope, NativeCompilationFrame,
    NativeCompilationGuard, NativeCompilationSelection, NativeCompilationWordShape,
    NativeCompiledBodies, NativeCompiledBodyContext, NativeCompiledBodyErrorContext,
    NativeCompiledBodyOperand,
};
use crate::{InvocationDialect, InvocationWords, SemanticOperationId, TryClauseKind};
use tcl_dialect::TclVersion;

/// Clause whose required arguments are absent before its selector is parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeTryClauseArgument {
    /// `on code variableList script`.
    On,
    /// `trap pattern variableList script`.
    Trap,
    /// `finally script`.
    Finally,
}

/// Native clause failure reached after its argument count was validated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeTryClauseFailure {
    /// A finally clause has trailing words.
    FinallyNonterminal,
    /// A final handler has the native fallthrough marker.
    BadFallthrough,
    /// A trap prefix is not a native list.
    TrapPrefixFormat,
}

impl InvocationDialect {
    /// Error tuple from the actual C try clause producer.
    #[must_use]
    pub fn try_clause_failure_error_code(
        self,
        failure: NativeTryClauseFailure,
    ) -> Option<&'static [u8]> {
        let version = self.native_string_protocol()?.tcl_version()?;
        if version < TclVersion::V8_6 {
            return None;
        }
        Some(match failure {
            NativeTryClauseFailure::FinallyNonterminal => b"TCL OPERATION TRY FINALLY NONTERMINAL",
            NativeTryClauseFailure::BadFallthrough => b"TCL OPERATION TRY BADFALLTHROUGH",
            NativeTryClauseFailure::TrapPrefixFormat => b"TCL OPERATION TRY TRAP EXNFORMAT",
        })
    }

    /// Actual native `try` clause argument error metadata, independent of text.
    /// Releases without native `try` and unsupported engines abstain.
    #[must_use]
    pub fn try_clause_argument_error_code(
        self,
        clause: NativeTryClauseArgument,
    ) -> Option<&'static [u8]> {
        let version = self.native_string_protocol()?.tcl_version()?;
        if version < TclVersion::V8_6 {
            return None;
        }
        Some(match clause {
            NativeTryClauseArgument::On => b"TCL OPERATION TRY ON ARGUMENT",
            NativeTryClauseArgument::Trap => b"TCL OPERATION TRY TRAP ARGUMENT",
            NativeTryClauseArgument::Finally => b"TCL OPERATION TRY FINALLY ARGUMENT",
        })
    }
}

fn layout(
    words: InvocationWords<'_>,
    dialect: InvocationDialect,
) -> Option<crate::TryControlInvocation> {
    let values = words.arguments().literal_values()?;
    let plan = crate::commands::tcl::NATIVE_TRY_GRAMMAR
        .walk_arguments(
            words.arguments().with_dialect(dialect),
            &[],
            dialect.authoring_query(),
        )?
        .ok()?;
    crate::registry::parse_try_control_invocation(
        &plan,
        &values,
        tcl_syntax::number::Numbers::Target(dialect.numbers),
        dialect.completion_code_policy(),
    )
}

pub(crate) fn select(
    operation: SemanticOperationId,
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    context: NativeCompilationContext,
) -> NativeCompilationSelection {
    use NativeCompilationSelection as Selection;
    if version < TclVersion::V8_6 || shapes.is_empty() {
        return Selection::Generic;
    }
    if shapes.len() == 1 {
        return Selection::Inline {
            operation,
            guard: NativeCompilationGuard::BeforeArguments,
        };
    }
    let Some(dialect) = words.arguments().dialect() else {
        return Selection::Unknown;
    };
    let Some(layout) = layout(words, dialect) else {
        // The runtime parser intentionally accepts a narrower proof domain than
        // TclWordKnownAtCompileTime. Absence is not proof of compiler refusal.
        return Selection::Unknown;
    };
    let handlers = layout
        .clauses
        .iter()
        .any(|clause| clause.kind != TryClauseKind::Finally);
    if handlers && context.frame != NativeCompilationFrame::ProcedureCode {
        return if context.frame == NativeCompilationFrame::ScriptCode {
            Selection::Generic
        } else {
            Selection::Unknown
        };
    }
    for clause in &layout.clauses {
        if clause.kind == TryClauseKind::Trap
            && clause
                .selector_index
                .and_then(|index| words.arguments().literal_at(index))
                .and_then(|text| dialect.word_values.split_list(text).ok())
                .is_some_and(|list| list.is_empty())
        {
            return Selection::Generic;
        }
        if let Some(index) = clause.variable_list_index {
            let Some(names) = words
                .arguments()
                .literal_at(index)
                .and_then(|text| dialect.word_values.split_list(text).ok())
            else {
                return Selection::Unknown;
            };
            if names.iter().any(|name| {
                tcl_syntax::naming::is_qualified(name.as_bytes())
                    || tcl_syntax::naming::split_element_ref(name).is_some()
            }) {
                return Selection::Generic;
            }
        }
        if !matches!(
            shapes.get(clause.body_index),
            Some(
                NativeCompilationWordShape::Literal
                    | NativeCompilationWordShape::QuotedLiteral
                    | NativeCompilationWordShape::BracedLiteral
            )
        ) {
            return Selection::Generic;
        }
    }
    Selection::Inline {
        operation,
        guard: NativeCompilationGuard::BeforeArguments,
    }
}

pub(crate) fn compiled_bodies(
    words: InvocationWords<'_>,
    dialect: InvocationDialect,
) -> NativeCompiledBodies {
    let Some(layout) = layout(words, dialect) else {
        return NativeCompiledBodies::Unknown;
    };
    let context = if layout.clauses.is_empty() {
        NativeCompiledBodyContext::Inherit
    } else {
        NativeCompiledBodyContext::ExceptionRange
    };
    NativeCompiledBodies::Known(
        std::iter::once(layout.body_index)
            .chain(
                layout
                    .clauses
                    .iter()
                    .filter(|clause| !clause.fallthrough)
                    .map(|clause| clause.body_index),
            )
            .map(|argument| NativeCompiledBodyOperand {
                argument,
                context,
                error_context: NativeCompiledBodyErrorContext::None,
                failure_scope: NativeCompilationFailureScope::FallbackToGeneric,
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_compilation::{
        NativeBodyCompilation, NativeCompilationMode, NativeCompilationSpec,
    };

    #[test]
    fn try_compiler_shares_clauses_and_protected_child_contexts() {
        let dialect = InvocationDialect::for_version(TclVersion::V8_6);
        let spec = NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::Try,
            operation: SemanticOperationId::StructuredLowering(crate::hooks::LoweringHookId::Try),
            body: NativeBodyCompilation::Inherit,
        };
        let context = NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            frame: NativeCompilationFrame::ProcedureCode,
            catch_depth: Some(2),
            loop_depth: 0,
        };
        let args = [
            "expr {1+2}",
            "on",
            "error",
            "m o",
            "set m handled",
            "finally",
            "set done 1",
        ];
        let words = InvocationWords::literals("try", &args).with_dialect(dialect);
        let shapes = [NativeCompilationWordShape::BracedLiteral; 7];
        assert!(matches!(
            spec.select(words, &shapes, Some(dialect), context),
            NativeCompilationSelection::Inline { .. }
        ));
        let NativeCompiledBodies::Known(bodies) = compiled_bodies(words, dialect) else {
            panic!("closed clauses");
        };
        assert_eq!(
            bodies.iter().map(|body| body.argument).collect::<Vec<_>>(),
            [0, 4, 6]
        );
        assert!(
            bodies
                .iter()
                .all(|body| body.entered_context(context).unwrap().catch_depth == Some(3))
        );
        assert_eq!(
            spec.select(
                words,
                &shapes,
                Some(dialect),
                NativeCompilationContext {
                    frame: NativeCompilationFrame::ScriptCode,
                    ..context
                }
            ),
            NativeCompilationSelection::Generic
        );
        let finally = InvocationWords::literals("try", &["return value", "finally", "set done 1"])
            .with_dialect(dialect);
        assert!(matches!(
            spec.select(
                finally,
                &shapes[..3],
                Some(dialect),
                NativeCompilationContext {
                    frame: NativeCompilationFrame::ScriptCode,
                    ..context
                }
            ),
            NativeCompilationSelection::Inline { .. }
        ));
    }
}
