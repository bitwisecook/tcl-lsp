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

//! The selection entry points: from source words, native byte words or resolved invocation facts to a compiler selection or its failure.

use super::string_grammars::NamedCompilerOperandView;
use super::variable_and_list_grammars::{array_compilation_grammar_for, variable_append_grammar};
use super::{
    NativeAppendKind, NativeCompilationContext, NativeCompilationFailure, NativeCompilationGrammar,
    NativeCompilationGuard, NativeCompilationMode, NativeCompilationSelection,
    NativeCompilationSpec, NativeCompilationWordShape, NativeNamedInvocationProtocol,
};
use crate::{InvocationDialect, InvocationWords};
use tcl_dialect::TclVersion;
use tcl_dialect::model::Family;

impl NativeCompilationSpec {
    /// Resolve native compilation using source shape, never computed argv text.
    #[must_use]
    pub fn select(
        self,
        words: InvocationWords<'_>,
        shapes: &[NativeCompilationWordShape],
        dialect: Option<InvocationDialect>,
        context: NativeCompilationContext,
    ) -> NativeCompilationSelection {
        self.select_with_named_operand_view(
            words,
            shapes,
            dialect,
            context,
            NamedCompilerOperandView::Unknown,
        )
    }

    /// Select with an independently retained metadata operand layout. This
    /// neither resolves an original head nor supplies native compiler authority.
    #[must_use]
    pub fn select_with_named_operand_view(
        self,
        words: InvocationWords<'_>,
        shapes: &[NativeCompilationWordShape],
        dialect: Option<InvocationDialect>,
        context: NativeCompilationContext,
        named_view: NamedCompilerOperandView,
    ) -> NativeCompilationSelection {
        use NativeCompilationSelection as Selection;
        if !self.operation_matches_grammar() {
            return Selection::Unknown;
        }
        if let NativeCompilationGrammar::WithImplementationPath { compiler, .. } = self.grammar {
            if dialect.is_none_or(|dialect| self.implementation_prerequisites(dialect).is_none()) {
                return Selection::Unknown;
            }
            return compiler
                .select_with_named_operand_view(words, shapes, dialect, context, named_view);
        }
        if context.mode == NativeCompilationMode::Direct {
            return Selection::Generic;
        }
        let Some(dialect) = dialect else {
            return Selection::Unknown;
        };
        if dialect.family() != Some(Family::Tcl) {
            return if dialect.family() == Some(Family::Jim) {
                Selection::Generic
            } else {
                Selection::Unknown
            };
        }
        let Some(version) = dialect.tcl_version else {
            return Selection::Unknown;
        };
        // The selected registration has no compiler to execute in either
        // evaluation mode. This preserves ordinary argv evaluation without
        // certifying the handler, its body, or a successful invocation.
        if self.compiler_hook_presence(dialect) == Some(false) {
            return Selection::Generic;
        }
        if context.mode != NativeCompilationMode::BytecodeObject {
            return Selection::Unknown;
        }
        if words.head().literal().is_none() {
            return Selection::Generic;
        }
        // Tcl 9.1 registers CMD_COMPILES_EXPANDED on this compiler. Its
        // namespace-prefixed list builder consumes original source words;
        // the expanded runtime argv length is deliberately not a prerequisite.
        if self.grammar == NativeCompilationGrammar::CoroutineRelay && version >= TclVersion::V9_1 {
            return if shapes.contains(&NativeCompilationWordShape::Opaque) {
                Selection::Unknown
            } else {
                Selection::Inline {
                    operation: self.operation,
                    guard: NativeCompilationGuard::BeforeArguments,
                }
            };
        }
        if let NativeCompilationGrammar::TclOoHelper(helper) = self.grammar {
            return crate::native_tcloo_compilation::select(
                helper,
                shapes,
                words.arguments().literal_at(0).map(str::as_bytes),
                version,
                self.operation,
            );
        }
        if self.grammar == NativeCompilationGrammar::Tailcall {
            return self.select_tailcall(shapes, version, context);
        }
        if self.grammar == NativeCompilationGrammar::VariableAppend(NativeAppendKind::List)
            && version >= TclVersion::V9_1
        {
            return variable_append_grammar(
                NativeAppendKind::List,
                self.operation,
                words.with_dialect(dialect),
                shapes,
                version,
                context,
            );
        }
        let Some(count) = words.arguments().exact_argv_len() else {
            return Selection::Generic;
        };
        if shapes.len() != count || shapes.contains(&NativeCompilationWordShape::Opaque) {
            return Selection::Unknown;
        }
        if shapes.contains(&NativeCompilationWordShape::Expanded) {
            return Selection::Generic;
        }
        self.select_grammar(
            words.with_dialect(dialect),
            shapes,
            version,
            context,
            named_view,
        )
    }

    /// Select an independently registered compiler using original byte words.
    /// `operand_from` addresses the complete original vector including its
    /// head; it is not a runtime argc or a written-only role offset. Shape-only
    /// descriptors do not require Unicode. Value-dependent descriptors use an
    /// unchanged checked view or abstain; opaque literals never become dynamic.
    /// This selection does not attest registration or implementation dependencies.
    #[must_use]
    pub fn select_native_words(
        self,
        words: &crate::native_compiler_words::NativeCompilerWords<'_>,
        operand_from: usize,
        dialect: Option<InvocationDialect>,
        context: NativeCompilationContext,
    ) -> NativeCompilationSelection {
        use NativeCompilationSelection as Selection;
        if !self.operation_matches_grammar() {
            return Selection::Unknown;
        }
        if let NativeCompilationGrammar::WithImplementationPath { compiler, .. } = self.grammar {
            if dialect.is_none_or(|dialect| self.implementation_prerequisites(dialect).is_none()) {
                return Selection::Unknown;
            }
            return compiler.select_native_words(words, operand_from, dialect, context);
        }
        if context.mode == NativeCompilationMode::Direct {
            return Selection::Generic;
        }
        let Some(dialect) = dialect else {
            return Selection::Unknown;
        };
        if dialect.family() != Some(Family::Tcl) {
            return if dialect.family() == Some(Family::Jim) {
                Selection::Generic
            } else {
                Selection::Unknown
            };
        }
        let Some(version) = dialect.tcl_version else {
            return Selection::Unknown;
        };
        if operand_from == 0 {
            return Selection::Unknown;
        }
        let Some(shapes) = words.shapes().get(operand_from..) else {
            return Selection::Unknown;
        };
        if words.shapes()[0] == NativeCompilationWordShape::Expanded {
            return Selection::Unknown;
        }
        match words.shapes()[0].compiler_head(Some(dialect)) {
            Some(false) => return Selection::Generic,
            None => return Selection::Unknown,
            Some(true) => {}
        }
        // A selected NULL compileProc supplies no pre-argv compiler effects.
        // The original vector and head checks above remain independent of
        // runtime argc and successful handler entry.
        if self.compiler_hook_presence(dialect) == Some(false) {
            return Selection::Generic;
        }
        if context.mode != NativeCompilationMode::BytecodeObject {
            return Selection::Unknown;
        }
        self.select_original_registered_words(
            words,
            operand_from,
            shapes,
            dialect,
            context,
            version,
        )
    }

    /// The private member command a basic named-invocation compiler names.
    #[must_use]
    pub const fn named_member_slot(self) -> Option<&'static str> {
        match self.grammar {
            NativeCompilationGrammar::NamedEnsembleInvocation { lookup, .. } => Some(lookup.slot),
            NativeCompilationGrammar::WithImplementationPath { compiler, .. } => {
                compiler.named_member_slot()
            }
            _ => None,
        }
    }

    /// Whether this actual descriptor is a basic named-invocation compiler.
    /// An ensemble-shaped runtime handler does not change its CPP kind.
    #[must_use]
    pub const fn is_named_invocation_compiler(self) -> bool {
        match self.grammar {
            NativeCompilationGrammar::NamedEnsembleInvocation { .. } => true,
            NativeCompilationGrammar::WithImplementationPath { compiler, .. } => {
                compiler.is_named_invocation_compiler()
            }
            _ => false,
        }
    }

    /// Select the actual installed worker's grammar, independently of the
    /// original public command's spelling. This grants no registration proof.
    #[must_use]
    pub fn select_registered_worker_native_words(
        self,
        words: &crate::native_compiler_words::NativeCompilerWords<'_>,
        operand_from: usize,
        dialect: Option<InvocationDialect>,
        context: NativeCompilationContext,
    ) -> NativeCompilationSelection {
        if self.grammar == NativeCompilationGrammar::InfoCommands {
            // TclAttemptCompileProc shifts the parse head to the last selected
            // member; this compiler's source grammar includes that original
            // selector before its zero or one actual pattern operand.
            let Some(selector) = operand_from.checked_sub(1) else {
                return NativeCompilationSelection::Unknown;
            };
            return self.select_native_words(words, selector, dialect, context);
        }
        if let NativeCompilationGrammar::Array { command, lookup } = self.grammar {
            let Some(dialect) = dialect.filter(|dialect| dialect.family() == Some(Family::Tcl))
            else {
                return NativeCompilationSelection::Unknown;
            };
            let Some(version) = dialect.tcl_version else {
                return NativeCompilationSelection::Unknown;
            };
            if context.mode != NativeCompilationMode::BytecodeObject {
                return self.select_native_words(words, operand_from, Some(dialect), context);
            }
            let Some(projected) = words.checked_words() else {
                return NativeCompilationSelection::Unknown;
            };
            let Some(arguments) = projected.get(operand_from..) else {
                return NativeCompilationSelection::Unknown;
            };
            let Some(shapes) = words.shapes().get(operand_from..) else {
                return NativeCompilationSelection::Unknown;
            };
            return array_compilation_grammar_for(
                (self.operation, command, lookup, true),
                InvocationWords::structured(projected[0], arguments),
                shapes,
                version,
                context,
            );
        }
        let selection = self.select_native_words(words, operand_from, dialect, context);
        if matches!(
            self.grammar,
            NativeCompilationGrammar::InfoLevel
                | NativeCompilationGrammar::NamespaceCurrent
                | NativeCompilationGrammar::NamespaceOrigin
                | NativeCompilationGrammar::NamespaceCode
        ) && matches!(
            selection,
            NativeCompilationSelection::NamedInvocation {
                protocol: NativeNamedInvocationProtocol::EnsembleRewrite,
                ..
            }
        ) {
            NativeCompilationSelection::Generic
        } else {
            selection
        }
    }

    /// First compiler operand in a complete original vector, including its head.
    ///
    /// Runtime member roles begin after their selector. A public String
    /// compiler retains that original selector, while a private worker starts
    /// at its own first operand. This layout supplies no registration, live
    /// lookup, argument execution or successful completion evidence.
    #[must_use]
    pub fn original_operand_from_for_facts(self, facts: &crate::InvocationFacts) -> Option<usize> {
        if let NativeCompilationGrammar::WithImplementationPath { compiler, .. } = self.grammar {
            return compiler.original_operand_from_for_facts(facts);
        }
        let retains_member = matches!(
            self.grammar,
            NativeCompilationGrammar::Dictionary { ensemble: true, .. }
                | NativeCompilationGrammar::StringIsAscii(
                    crate::native_scalar_compilation::NativeScalarScope::PublicMember
                )
                | NativeCompilationGrammar::StringEqual(
                    crate::native_scalar_compilation::NativeScalarScope::PublicMember
                )
                | NativeCompilationGrammar::StringLength(
                    crate::native_scalar_compilation::NativeScalarScope::PublicMember
                )
                | NativeCompilationGrammar::StringTrim {
                    scope: crate::native_scalar_compilation::NativeScalarScope::PublicMember,
                    ..
                }
                | NativeCompilationGrammar::StringMatch(
                    crate::native_string_compilation::NativeStringMatchScope::PublicMember
                )
                | NativeCompilationGrammar::StringMap(
                    crate::native_scalar_compilation::NativeScalarScope::PublicMember
                )
                | NativeCompilationGrammar::StringRange(
                    crate::native_scalar_compilation::NativeScalarScope::PublicMember
                )
                | NativeCompilationGrammar::StringReplace(
                    crate::native_scalar_compilation::NativeScalarScope::PublicMember
                )
                | NativeCompilationGrammar::StringInsert(
                    crate::native_scalar_compilation::NativeScalarScope::PublicMember
                )
                | NativeCompilationGrammar::StringOperand {
                    scope: crate::native_scalar_compilation::NativeScalarScope::PublicMember,
                    ..
                }
        );
        facts
            .argument_offset
            .checked_sub(usize::from(retains_member))?
            .checked_add(1)
    }

    /// Select using the resolved member's operand view while retaining every
    /// original compiler token shape. Primitive descriptors consume their own
    /// operands; ensemble-wide grammars retain the public member word.
    #[must_use]
    pub fn select_for_facts(
        self,
        words: InvocationWords<'_>,
        shapes: &[NativeCompilationWordShape],
        facts: &crate::InvocationFacts,
        dialect: Option<InvocationDialect>,
        context: NativeCompilationContext,
    ) -> NativeCompilationSelection {
        if self.is_named_invocation_compiler() {
            // The member invoked by its own private name has no selector to
            // retain: `TclCompileBasicNArgCmd` pushes the written words and
            // emits `invokeStk`, and so does the fallback when it declines.
            // tclsh 8.6.18 and 9.0.4: `::tcl::array::get a` is
            // `push ::tcl::array::get; push a; invokeStk1 2`.
            return if facts.argument_offset == 0
                && self.named_member_slot().is_some_and(|slot| {
                    words.head_literal().is_some_and(|head| {
                        tcl_syntax::naming::unroot_rooted_key(head).unwrap_or(head)
                            == tcl_syntax::naming::unroot_rooted_key(slot).unwrap_or(slot)
                    })
                }) {
                NativeCompilationSelection::Generic
            } else {
                NativeCompilationSelection::Unknown
            };
        }
        let offset = if matches!(
            self.grammar,
            NativeCompilationGrammar::InfoExists
                | NativeCompilationGrammar::InfoLevel
                | NativeCompilationGrammar::InfoCoroutine
                | NativeCompilationGrammar::NamespaceCurrent
                | NativeCompilationGrammar::NamespaceOrigin
                | NativeCompilationGrammar::NamespaceCode
                | NativeCompilationGrammar::NamespaceString(_)
                | NativeCompilationGrammar::NamespaceWhich
        ) || matches!(
            self.grammar,
            // The ensemble compiler hands the member's compile procedure the
            // words after the member, as the private name receives them.
            NativeCompilationGrammar::WithImplementationPath { compiler, .. }
                if matches!(compiler.grammar, NativeCompilationGrammar::ClockReading(_))
        ) {
            let private = dialect.and_then(|dialect| self.implementation_lookup(dialect));
            if private.is_some_and(|lookup| {
                words.head_literal().is_some_and(|head| {
                    tcl_syntax::naming::unroot_rooted_key(head).unwrap_or(head)
                        == tcl_syntax::naming::unroot_rooted_key(lookup.slot).unwrap_or(lookup.slot)
                })
            }) {
                0
            } else {
                facts.argument_offset
            }
        } else {
            0
        };
        let Some(shapes) = shapes.get(offset..) else {
            return NativeCompilationSelection::Unknown;
        };
        self.select_with_named_operand_view(
            InvocationWords::from_arguments(words.head(), words.arguments().slice_from(offset)),
            shapes,
            dialect,
            context,
            NamedCompilerOperandView::OriginalArguments {
                selectors_retained: facts.argument_offset,
            },
        )
    }

    /// Select from resolved facts for a caller that retains the authenticated
    /// original command vector. A named ensemble compiler then emits its
    /// private-name invocation after the retained selector words; every other
    /// grammar selects exactly as [`Self::select_for_facts`], which refuses a
    /// named compiler because it cannot see that the original words survive.
    #[must_use]
    pub fn select_for_original_facts(
        self,
        words: InvocationWords<'_>,
        shapes: &[NativeCompilationWordShape],
        facts: &crate::InvocationFacts,
        dialect: Option<InvocationDialect>,
        context: NativeCompilationContext,
    ) -> NativeCompilationSelection {
        if !self.is_named_invocation_compiler() {
            return self.select_for_facts(words, shapes, facts, dialect, context);
        }
        self.select_with_named_operand_view(
            words,
            shapes,
            dialect,
            context,
            NamedCompilerOperandView::OriginalArguments {
                selectors_retained: facts.argument_offset,
            },
        )
    }

    /// Project native compiler failures through authored grammar, never the
    /// invoked token's spelling. C Tcl 8.4's fixed arity errors name the native
    /// primitive even when the command object was renamed.
    #[must_use]
    pub fn failure_for_selection(
        self,
        selection: NativeCompilationSelection,
        _words: InvocationWords<'_>,
        facts: &crate::InvocationFacts,
        dialect: Option<InvocationDialect>,
    ) -> Option<NativeCompilationFailure> {
        if selection != NativeCompilationSelection::CompileError
            || dialect
                .filter(|dialect| dialect.family() == Some(Family::Tcl))
                .and_then(|dialect| dialect.tcl_version)
                != Some(TclVersion::V8_4)
        {
            return None;
        }
        let usage = match self.grammar {
            NativeCompilationGrammar::CheckedArity { usage, .. } => Some(usage),
            NativeCompilationGrammar::ListLength => Some("llength list"),
            NativeCompilationGrammar::VariableLoadStore => Some("set varName ?newValue?"),
            NativeCompilationGrammar::Increment => Some("incr varName ?increment?"),
            NativeCompilationGrammar::VariableAppend(NativeAppendKind::String) => {
                Some("append varName ?value value ...?")
            }
            NativeCompilationGrammar::VariableAppend(NativeAppendKind::List) => {
                Some("lappend varName ?value value ...?")
            }
            NativeCompilationGrammar::Catch => Some("catch command ?varName?"),
            NativeCompilationGrammar::Expression => Some("expr arg ?arg ...?"),
            NativeCompilationGrammar::ForLoop => Some("for start test next command"),
            NativeCompilationGrammar::Foreach => {
                Some("foreach varList list ?varList list ...? command")
            }
            NativeCompilationGrammar::LoopControl
            | NativeCompilationGrammar::Break
            | NativeCompilationGrammar::Continue => match facts.completion.codes {
                crate::completion::CompletionCodeDomain::Exact(
                    [crate::completion::CompletionCode::Break],
                ) => Some("break"),
                crate::completion::CompletionCodeDomain::Exact(
                    [crate::completion::CompletionCode::Continue],
                ) => Some("continue"),
                _ => None,
            },
            NativeCompilationGrammar::WhileLoop => Some("while test command"),
            _ => None,
        };
        Some(NativeCompilationFailure {
            message: usage.map(|usage| format!("wrong # args: should be \"{usage}\"")),
            error_code: Some("NONE".into()),
            error_info: None,
        })
    }

    /// Present a compiler rejection with independently retained source words.
    /// Raw spellings are needed when a missing script follows a quoted or
    /// braced expression; decoded values alone cannot prove those bytes.
    #[must_use]
    pub fn failure_for_selection_with_source(
        self,
        selection: NativeCompilationSelection,
        words: InvocationWords<'_>,
        facts: &crate::InvocationFacts,
        dialect: Option<InvocationDialect>,
        written_head: &str,
        operand_spellings: &[&str],
    ) -> Option<NativeCompilationFailure> {
        let mut failure = self.failure_for_selection(selection, words, facts, dialect)?;
        if matches!(self.grammar, NativeCompilationGrammar::Conditional) {
            let values = words.arguments().literal_values()?;
            if values.len() != operand_spellings.len() {
                return None;
            }
            failure.message = crate::commands::tcl::native_if_compile_shape_message(
                &values,
                written_head,
                operand_spellings,
            );
        }
        Some(failure)
    }
}
