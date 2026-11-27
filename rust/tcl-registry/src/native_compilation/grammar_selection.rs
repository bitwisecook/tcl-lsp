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

//! Dispatch from a `NativeCompilationGrammar` to its selection, with the control-flow, arity-gated and clock grammars it applies directly.

use super::introspection_grammars::{
    info_commands_grammar, info_level_grammar, namespace_code_grammar, namespace_current_grammar,
    namespace_origin_grammar, namespace_which_selection,
};
use super::string_grammars::{
    NamedCompilerOperandView, STRING_EQUAL_IMPLEMENTATION, STRING_LENGTH_IMPLEMENTATION,
    named_arity_at, named_ensemble_grammar, regexp_grammar, regsub_grammar, string_match_grammar,
    string_member_arity_grammar, string_trim_grammar, substitution_template_compiler_grammar,
};
use super::variable_and_list_grammars::{
    array_compilation_grammar, list_assignment_grammar, list_insertion_grammar, list_range_grammar,
    literal_unset_grammar, upvar_grammar, variable_append_grammar,
};
use super::{
    NativeClockReading, NativeCompilationContext, NativeCompilationFrame, NativeCompilationGrammar,
    NativeCompilationGuard, NativeCompilationSelection, NativeCompilationSpec,
    NativeCompilationWordShape, literal_shape,
};
use crate::{InvocationWords, SemanticOperationId};
use tcl_dialect::TclVersion;

impl NativeCompilationSpec {
    pub(super) fn select_grammar(
        self,
        words: InvocationWords<'_>,
        shapes: &[NativeCompilationWordShape],
        version: TclVersion,
        context: NativeCompilationContext,
        named_view: NamedCompilerOperandView,
    ) -> NativeCompilationSelection {
        use NativeCompilationSelection as Selection;
        if let Some(selection) =
            self.select_delegated_grammar(words, shapes, version, context, named_view)
        {
            return selection;
        }
        if let Some(selection) = self.select_shape_grammar(shapes, version, context) {
            return selection;
        }
        let valid = match self.grammar {
            NativeCompilationGrammar::Catch => match catch_grammar(words, shapes, version, context)
            {
                Ok(valid) => valid,
                Err(selection) => return selection,
            },
            NativeCompilationGrammar::Conditional => {
                match conditional_grammar(words, shapes, version) {
                    Ok(valid) => valid,
                    Err(selection) => return selection,
                }
            }
            NativeCompilationGrammar::Upvar => match upvar_grammar(words, shapes, version, context)
            {
                Ok(valid) => valid,
                Err(selection) => return selection,
            },
            NativeCompilationGrammar::Return => {
                match return_grammar(words, shapes, version, context) {
                    Ok(valid) => valid,
                    Err(selection) => return selection,
                }
            }
            _ => return Selection::Unknown,
        };
        self.selected_grammar_result(valid, version)
    }

    /// Shared native token/count grammar. Neither byte values nor unchanged
    /// Unicode views are required by these compiler descriptors.
    pub(super) fn select_shape_grammar(
        self,
        shapes: &[NativeCompilationWordShape],
        version: TclVersion,
        context: NativeCompilationContext,
    ) -> Option<NativeCompilationSelection> {
        use NativeCompilationSelection as Selection;
        let valid = match self.grammar {
            NativeCompilationGrammar::CheckedArity { arity, .. } => {
                u16::try_from(shapes.len()).is_ok_and(|count| arity.accepts(count))
            }
            NativeCompilationGrammar::MathOperator(operator) => {
                return Some(match operator.accepts(shapes.len(), version, context) {
                    Some(true) => self.selected_grammar_result(true, version),
                    Some(false) => Selection::Generic,
                    None => Selection::Unknown,
                });
            }
            NativeCompilationGrammar::NamespaceString(_) => {
                if version < TclVersion::V8_6 {
                    return Some(Selection::Generic);
                }
                shapes.len() == 1 && !shapes.contains(&NativeCompilationWordShape::Expanded)
            }
            NativeCompilationGrammar::ListLength => shapes.len() == 1,
            NativeCompilationGrammar::ListAssignment => {
                // Native proof: naming.list.assignment-literal-versus-alias-target-evaluation
                // docs/design/analysis/name-resolution-proofs/assignment-literal-versus-alias-target-evaluation.md
                // This compiler inspects token count and shape; opaque static
                // target bytes do not need a Unicode argument projection.
                return Some(list_assignment_grammar(shapes, version, self.operation));
            }
            NativeCompilationGrammar::VariableLoadStore | NativeCompilationGrammar::Increment => {
                matches!(shapes.len(), 1 | 2)
            }
            NativeCompilationGrammar::Return if shapes.len() <= 1 => {
                // Zero/one-result Return compilers inspect the original count,
                // not result bytes. C8.4 additionally requires an uncaught
                // procedure compiler context.
                if version == TclVersion::V8_4 {
                    if context.frame != NativeCompilationFrame::ProcedureCode {
                        return Some(Selection::Generic);
                    }
                    match context.catch_depth {
                        Some(0) => {}
                        Some(_) => return Some(Selection::Generic),
                        None => return Some(Selection::Unknown),
                    }
                }
                true
            }
            NativeCompilationGrammar::Catch if shapes.len() == 1 => true,
            NativeCompilationGrammar::LoopControl
            | NativeCompilationGrammar::Break
            | NativeCompilationGrammar::Continue => shapes.is_empty(),
            NativeCompilationGrammar::Expression => !shapes.is_empty(),
            NativeCompilationGrammar::ForLoop => match for_loop_grammar(shapes) {
                Ok(valid) => valid,
                Err(selection) => return Some(selection),
            },
            NativeCompilationGrammar::LiteralUnset => {
                match literal_unset_grammar(shapes, version) {
                    Ok(valid) => valid,
                    Err(selection) => return Some(selection),
                }
            }
            NativeCompilationGrammar::InfoExists => {
                if version < TclVersion::V8_5 {
                    return Some(Selection::Generic);
                }
                shapes.len() == 1
            }
            NativeCompilationGrammar::ListIndex => {
                if shapes.is_empty() {
                    return Some(Selection::Generic);
                }
                u32::try_from(shapes.len()).is_ok_and(|count| count < u32::MAX)
            }
            NativeCompilationGrammar::ArgumentList => {
                if version <= TclVersion::V8_5 {
                    match context.frame {
                        NativeCompilationFrame::ScriptCode => return Some(Selection::Generic),
                        NativeCompilationFrame::Unknown => return Some(Selection::Unknown),
                        NativeCompilationFrame::ProcedureCode => {}
                    }
                }
                true
            }
            NativeCompilationGrammar::ArgumentListFrom(first)
            | NativeCompilationGrammar::ArgumentConcatFrom(first) => {
                if version < first {
                    return Some(Selection::Generic);
                }
                true
            }
            NativeCompilationGrammar::LiteralOperands { count } => {
                match literal_operands_grammar(shapes, count) {
                    Ok(valid) => valid,
                    Err(selection) => return Some(selection),
                }
            }
            NativeCompilationGrammar::WhileLoop => match literal_operands_grammar(shapes, 2) {
                Ok(valid) => valid,
                Err(selection) => return Some(selection),
            },
            _ => return None,
        };
        Some(self.selected_grammar_result(valid, version))
    }

    fn select_uplevel(
        self,
        words: InvocationWords<'_>,
        shapes: &[NativeCompilationWordShape],
        version: TclVersion,
        context: NativeCompilationContext,
    ) -> NativeCompilationSelection {
        use NativeCompilationSelection as Selection;
        if version < TclVersion::V9_1 || context.frame == NativeCompilationFrame::ScriptCode {
            Selection::Generic
        } else if context.frame != NativeCompilationFrame::ProcedureCode {
            Selection::Unknown
        } else if !shapes.first().is_some_and(|shape| {
            matches!(
                shape,
                NativeCompilationWordShape::Literal
                    | NativeCompilationWordShape::QuotedLiteral
                    | NativeCompilationWordShape::BracedLiteral
                    | NativeCompilationWordShape::BackslashLiteral
            )
        }) || self.uplevel_operands(words.arguments()).is_none()
        {
            Selection::Generic
        } else {
            Selection::Inline {
                operation: self.operation,
                guard: NativeCompilationGuard::BeforeArguments,
            }
        }
    }

    fn select_format_string_concat(
        self,
        words: InvocationWords<'_>,
        shapes: &[NativeCompilationWordShape],
        version: TclVersion,
        context: NativeCompilationContext,
    ) -> NativeCompilationSelection {
        crate::native_format_compilation::select_with_values(
            &|index| {
                words
                    .arguments()
                    .literal_at(index)
                    .map(|word| word.as_bytes().to_vec())
            },
            shapes,
            version,
            context,
            self.operation,
        )
    }

    fn select_delegated_grammar(
        self,
        words: InvocationWords<'_>,
        shapes: &[NativeCompilationWordShape],
        version: TclVersion,
        context: NativeCompilationContext,
        named_view: NamedCompilerOperandView,
    ) -> Option<NativeCompilationSelection> {
        use NativeCompilationSelection as Selection;
        Some(match self.grammar {
            NativeCompilationGrammar::VariableAppend(kind) => {
                variable_append_grammar(kind, self.operation, words, shapes, version, context)
            }
            NativeCompilationGrammar::Array { command, lookup } => array_compilation_grammar(
                self.operation,
                command,
                lookup,
                words,
                shapes,
                version,
                context,
            ),
            NativeCompilationGrammar::Foreach => {
                self.select_foreach(words, shapes, version, context)
            }
            NativeCompilationGrammar::Dictionary { command, ensemble } => {
                command.select(ensemble, words, shapes, version, context)
            }
            NativeCompilationGrammar::NamedEnsembleInvocation {
                lookup,
                hook_from,
                arity,
                later_arity,
                ..
            } => named_ensemble_grammar(
                lookup,
                hook_from,
                named_arity_at(arity, later_arity, version),
                named_view,
                shapes,
                version,
            ),
            NativeCompilationGrammar::Try => {
                crate::native_try::select(self.operation, words, shapes, version, context)
            }
            NativeCompilationGrammar::Uplevel => {
                self.select_uplevel(words, shapes, version, context)
            }
            NativeCompilationGrammar::Unresolved => Selection::Unknown,
            NativeCompilationGrammar::Constant => constant_selection(
                words.arguments().literal_at(0).map(str::as_bytes),
                shapes,
                version,
                self.operation,
            ),
            NativeCompilationGrammar::GlobalBindings
            | NativeCompilationGrammar::NamespaceVariableBindings => self
                .namespace_binding_selection(
                    crate::native_namespace_binding_compilation::namespace_binding_shape_outcome(
                        words,
                        shapes,
                        version,
                        context,
                        self.namespace_binding_kind()?,
                    ),
                ),
            NativeCompilationGrammar::SubstitutionTemplate => {
                substitution_template_compiler_grammar(words, shapes, version, self.operation)
            }
            NativeCompilationGrammar::FormatStringConcat => {
                self.select_format_string_concat(words, shapes, version, context)
            }
            NativeCompilationGrammar::ClockReading(reading) => clock_reading_selection(
                reading,
                words.arguments().literal_at(0).map(str::as_bytes),
                shapes,
                version,
                self.operation,
            ),
            NativeCompilationGrammar::HookFrom(first) => unmodelled_hook_selection(version, first),
            NativeCompilationGrammar::Regsub => regsub_grammar(words, shapes, version),
            NativeCompilationGrammar::NamespaceWhich => namespace_which_selection(
                shapes,
                words.arguments().literal_at(0).map(str::as_bytes),
                version,
                self.operation,
            ),
            NativeCompilationGrammar::NamespaceUpvarBindings => {
                generic_before_or_in_script(TclVersion::V8_6, version, context.frame)
            }
            NativeCompilationGrammar::NamespaceLegacy => {
                generic_before_or_in_script(TclVersion::V8_5, version, context.frame)
            }
            NativeCompilationGrammar::ProcedureHookFrom(first) => {
                procedure_hook_selection(version, first, context.frame)
            }
            _ => match self.select_delegated_arity_grammar(shapes, version) {
                Some(selection) => selection,
                None => return self.select_delegated_word_grammar(words, shapes, version),
            },
        })
    }

    fn select_delegated_arity_grammar(
        self,
        shapes: &[NativeCompilationWordShape],
        version: TclVersion,
    ) -> Option<NativeCompilationSelection> {
        use NativeCompilationSelection as Selection;
        match self.grammar {
            NativeCompilationGrammar::Error => Some(gated_arity_grammar(
                TclVersion::V8_6,
                crate::Arity::new(1, 3),
                shapes,
                version,
                self.operation,
            )),
            NativeCompilationGrammar::Throw => Some(gated_arity_grammar(
                TclVersion::V8_6,
                crate::Arity::exact(2),
                shapes,
                version,
                self.operation,
            )),
            NativeCompilationGrammar::ArityFrom { first, arity } => Some(gated_arity_grammar(
                first,
                arity,
                shapes,
                version,
                self.operation,
            )),
            NativeCompilationGrammar::InfoCoroutine => Some(gated_arity_grammar(
                TclVersion::V8_6,
                crate::Arity::exact(0),
                shapes,
                version,
                self.operation,
            )),
            NativeCompilationGrammar::CoroutineYield => Some(gated_arity_grammar(
                TclVersion::V8_6,
                crate::Arity::new(0, 1),
                shapes,
                version,
                self.operation,
            )),
            NativeCompilationGrammar::CoroutineRelay => Some(gated_arity_grammar(
                TclVersion::V8_6,
                crate::Arity::at_least(1),
                shapes,
                version,
                self.operation,
            )),
            NativeCompilationGrammar::NoHook => Some(Selection::Generic),
            _ => None,
        }
    }

    fn select_delegated_word_grammar(
        self,
        words: InvocationWords<'_>,
        shapes: &[NativeCompilationWordShape],
        version: TclVersion,
    ) -> Option<NativeCompilationSelection> {
        Some(match self.grammar {
            NativeCompilationGrammar::InfoLevel => {
                info_level_grammar(words, shapes, version, self.operation)
            }
            NativeCompilationGrammar::NamespaceCurrent => {
                namespace_current_grammar(words, shapes, version, self.operation)
            }
            NativeCompilationGrammar::NamespaceOrigin => {
                namespace_origin_grammar(words, shapes, version, self.operation)
            }
            NativeCompilationGrammar::NamespaceCode => {
                namespace_code_grammar(words, shapes, version, self.operation)
            }
            NativeCompilationGrammar::InfoCommands => {
                info_commands_grammar(words, shapes, version, self.operation)
            }
            NativeCompilationGrammar::StringIsAscii(scope) => {
                crate::native_string_is_compilation::select_shapes(
                    shapes,
                    &|index| {
                        words
                            .arguments()
                            .literal_at(index)
                            .map(|word| word.as_bytes().to_vec())
                    },
                    scope,
                    version,
                    self.operation,
                )
            }
            NativeCompilationGrammar::StringEqual(scope) => string_member_arity_grammar(
                words,
                shapes,
                version,
                self.operation,
                &STRING_EQUAL_IMPLEMENTATION,
                scope,
                2,
            ),
            NativeCompilationGrammar::StringTrim { scope, .. } => {
                string_trim_grammar(shapes, version, self.operation, scope)
            }
            NativeCompilationGrammar::StringMatch(scope) => {
                string_match_grammar(words, shapes, version, self.operation, scope)
            }
            NativeCompilationGrammar::StringLength(scope) => string_member_arity_grammar(
                words,
                shapes,
                version,
                self.operation,
                &STRING_LENGTH_IMPLEMENTATION,
                scope,
                1,
            ),
            // The map probe requires the exact retained compiler words.
            NativeCompilationGrammar::StringMap(_)
            | NativeCompilationGrammar::StringRange(_)
            | NativeCompilationGrammar::StringReplace(_)
            | NativeCompilationGrammar::StringInsert(_)
            | NativeCompilationGrammar::ListReplacement => NativeCompilationSelection::Unknown,
            NativeCompilationGrammar::StringOperand { scope, operation } => {
                crate::native_string_operand_compilation::select_shapes(
                    shapes,
                    words.arguments().literal_at(0).map(str::as_bytes),
                    operation,
                    scope,
                    version,
                    self.operation,
                )
            }
            NativeCompilationGrammar::ListRange => {
                list_range_grammar(words, shapes, version, self.operation)
            }
            NativeCompilationGrammar::ListInsertion => {
                list_insertion_grammar(words, shapes, version, self.operation)
            }
            NativeCompilationGrammar::Regexp => {
                regexp_grammar(words, shapes, version, self.operation)
            }
            _ => return None,
        })
    }

    pub(super) fn select_tailcall(
        self,
        shapes: &[NativeCompilationWordShape],
        version: TclVersion,
        context: NativeCompilationContext,
    ) -> NativeCompilationSelection {
        use NativeCompilationSelection as Selection;
        if version < TclVersion::V8_6 || context.frame == NativeCompilationFrame::ScriptCode {
            return Selection::Generic;
        }
        if context.frame != NativeCompilationFrame::ProcedureCode
            || shapes.contains(&NativeCompilationWordShape::Opaque)
        {
            return Selection::Unknown;
        }
        if version < TclVersion::V9_1
            && (shapes.is_empty()
                || shapes.len() >= 255
                || shapes.contains(&NativeCompilationWordShape::Expanded))
        {
            return Selection::Generic;
        }
        self.selected_grammar_result(true, version)
    }

    pub(super) fn selected_grammar_result(
        self,
        valid: bool,
        version: TclVersion,
    ) -> NativeCompilationSelection {
        use NativeCompilationSelection as Selection;
        if !valid {
            return if version == TclVersion::V8_4 {
                Selection::CompileError
            } else {
                Selection::Generic
            };
        }
        Selection::Inline {
            operation: self.operation,
            guard: NativeCompilationGuard::for_c_version(version),
        }
    }

    fn select_foreach(
        self,
        words: InvocationWords<'_>,
        shapes: &[NativeCompilationWordShape],
        version: TclVersion,
        context: NativeCompilationContext,
    ) -> NativeCompilationSelection {
        use NativeCompilationSelection as Selection;
        if context.frame != NativeCompilationFrame::ProcedureCode {
            return if context.frame == NativeCompilationFrame::ScriptCode {
                Selection::Generic
            } else {
                Selection::Unknown
            };
        }
        if shapes.len() < 3 || shapes.len().is_multiple_of(2) {
            return if version == TclVersion::V8_4 {
                Selection::CompileError
            } else {
                Selection::Generic
            };
        }
        if !literal_shape(*shapes.last().unwrap()) {
            return Selection::Generic;
        }
        for index in (0..shapes.len() - 1).step_by(2) {
            let Some(var_list) = words.arguments().literal_at(index) else {
                return Selection::Generic;
            };
            let Ok(names) =
                tcl_native_model::InvocationDialect::for_version(version).read_list(var_list)
            else {
                return Selection::Unknown;
            };
            if names.is_empty() {
                return Selection::Unknown;
            }
            if names.iter().any(|name| {
                tcl_syntax::naming::is_qualified(name.as_bytes())
                    || tcl_syntax::naming::split_element_ref(name).is_some()
            }) {
                return Selection::Generic;
            }
        }
        Selection::Inline {
            operation: self.operation,
            guard: NativeCompilationGuard::for_c_version(version),
        }
    }
}

/// `TclCompileConstCmd` over the written name: an array element declines.
fn constant_selection(
    name: Option<&[u8]>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    operation: SemanticOperationId,
) -> NativeCompilationSelection {
    let element =
        name.is_some_and(|name| tcl_syntax::naming::split_element_ref_bytes(name).is_some());
    if element {
        return NativeCompilationSelection::Generic;
    }
    gated_arity_grammar(
        TclVersion::V9_0,
        crate::Arity::exact(2),
        shapes,
        version,
        operation,
    )
}

fn unmodelled_hook_selection(version: TclVersion, first: TclVersion) -> NativeCompilationSelection {
    if version < first {
        NativeCompilationSelection::Generic
    } else {
        NativeCompilationSelection::Unknown
    }
}

fn procedure_hook_selection(
    version: TclVersion,
    first: TclVersion,
    frame: NativeCompilationFrame,
) -> NativeCompilationSelection {
    if frame == NativeCompilationFrame::ScriptCode {
        NativeCompilationSelection::Generic
    } else {
        unmodelled_hook_selection(version, first)
    }
}

fn for_loop_grammar(
    shapes: &[NativeCompilationWordShape],
) -> Result<bool, NativeCompilationSelection> {
    if shapes.len() != 4 {
        Ok(false)
    } else if shapes[1..].iter().all(|shape| literal_shape(*shape)) {
        Ok(true)
    } else {
        Err(NativeCompilationSelection::Generic)
    }
}

/// Generic before release `first` or in script code, otherwise unknown.
fn generic_before_or_in_script(
    first: TclVersion,
    version: TclVersion,
    frame: NativeCompilationFrame,
) -> NativeCompilationSelection {
    if version < first || frame == NativeCompilationFrame::ScriptCode {
        NativeCompilationSelection::Generic
    } else {
        NativeCompilationSelection::Unknown
    }
}

pub(crate) fn clock_reading_value(
    reading: NativeClockReading,
    argument: Option<&[u8]>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
) -> Option<u8> {
    use NativeClockReading as Reading;
    // tclsh 8.6.18, 9.0.4 and 9.1.0 disassemble `clock clicks`, `clock
    // microseconds`, `clock milliseconds` and `clock seconds` to `clockRead 0`
    // to `3`, and `clock clicks -mic` / `-mil` to `clockRead 1` / `2`. C9.1's
    // `IS_TOKEN_PREFIX(tokenPtr, 3, …)` also takes `-mi` as microseconds.
    let shortest_option = match version {
        TclVersion::V8_6 | TclVersion::V9_0 => 4,
        TclVersion::V9_1 => 3,
        _ => return None,
    };
    match (reading, shapes) {
        (Reading::Clicks, []) => Some(0),
        (Reading::Microseconds, []) => Some(1),
        (Reading::Milliseconds, []) => Some(2),
        (Reading::Seconds, []) => Some(3),
        (Reading::Clicks, [shape]) => {
            let bytes = crate::native_regexp_compilation::simple_word_bytes(*shape, argument)?;
            if !(shortest_option..=13).contains(&bytes.len()) {
                return None;
            }
            if b"-microseconds".starts_with(bytes) {
                Some(1)
            } else if b"-milliseconds".starts_with(bytes) {
                Some(2)
            } else {
                None
            }
        }
        _ => None,
    }
}

pub(super) fn clock_reading_selection(
    reading: NativeClockReading,
    argument: Option<&[u8]>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    operation: SemanticOperationId,
) -> NativeCompilationSelection {
    if operation != SemanticOperationId::Invoke {
        return NativeCompilationSelection::Unknown;
    }
    if clock_reading_value(reading, argument, shapes, version).is_some() {
        NativeCompilationSelection::Inline {
            operation,
            guard: NativeCompilationGuard::BeforeArguments,
        }
    } else if matches!(
        version,
        TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1
    ) {
        // The hook returns TCL_ERROR on any other word count or option, and
        // the command compiles as an invocation: tclsh 9.0.4 disassembles
        // `clock clicks -x` and `clock seconds x` to `invokeReplace 3`.
        NativeCompilationSelection::Generic
    } else {
        NativeCompilationSelection::Unknown
    }
}

fn literal_operands_grammar(
    shapes: &[NativeCompilationWordShape],
    count: usize,
) -> Result<bool, NativeCompilationSelection> {
    if shapes.len() != count {
        Ok(false)
    } else if shapes.iter().all(|shape| literal_shape(*shape)) {
        Ok(true)
    } else {
        Err(NativeCompilationSelection::Generic)
    }
}

fn conditional_grammar(
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
) -> Result<bool, NativeCompilationSelection> {
    use NativeCompilationSelection as Selection;
    if !shapes.iter().all(|shape| literal_shape(*shape)) {
        return Err(Selection::Generic);
    }
    let Some(arguments) = words.arguments().literal_values() else {
        return Err(Selection::Unknown);
    };
    let Some(conditional) = crate::installed().conditional_clauses() else {
        return Err(Selection::Unknown);
    };
    if (conditional.shape_error)(&arguments).is_some() {
        return Err(if version == TclVersion::V8_4 {
            Selection::CompileError
        } else {
            Selection::Generic
        });
    }
    Ok(true)
}

fn gated_arity_grammar(
    first: TclVersion,
    arity: crate::Arity,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    operation: SemanticOperationId,
) -> NativeCompilationSelection {
    use NativeCompilationSelection as Selection;
    if version < first {
        return Selection::Generic;
    }
    let Ok(count) = u16::try_from(shapes.len()) else {
        return Selection::Unknown;
    };
    if !arity.accepts(count) {
        return Selection::Generic;
    }
    Selection::Inline {
        operation,
        guard: NativeCompilationGuard::for_c_version(version),
    }
}

fn return_grammar(
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    context: NativeCompilationContext,
) -> Result<bool, NativeCompilationSelection> {
    use NativeCompilationSelection as Selection;
    if shapes.len() > 1 {
        if version == TclVersion::V8_4 {
            return Err(Selection::Generic);
        }
        let pair_count = shapes.len() & !1;
        if shapes.len() == 3
            && words.arguments().literal_at(0) == Some("-options")
            && matches!(
                shapes[0],
                NativeCompilationWordShape::Literal
                    | NativeCompilationWordShape::QuotedLiteral
                    | NativeCompilationWordShape::BracedLiteral
            )
        {
            return Ok(true);
        }
        let Some(arguments) = (0..pair_count)
            .map(|index| {
                words
                    .arguments()
                    .literal_at(index)
                    .map(|word| word.as_bytes().to_vec())
            })
            .collect::<Option<Vec<_>>>()
        else {
            return if version >= TclVersion::V8_6 {
                Ok(true)
            } else {
                Err(Selection::Generic)
            };
        };
        return match crate::native_return_compilation::static_controls(&arguments, version) {
            Ok(_) => Ok(true),
            Err(error) if error.native_access_refusal().is_some() => Err(Selection::Unknown),
            Err(_) => Err(Selection::Generic),
        };
    }
    if version != TclVersion::V8_4 {
        return Ok(true);
    }
    if context.frame != NativeCompilationFrame::ProcedureCode {
        return Err(Selection::Generic);
    }
    match context.catch_depth {
        Some(0) => Ok(true),
        Some(_) => Err(Selection::Generic),
        None => Err(Selection::Unknown),
    }
}

fn catch_grammar(
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    context: NativeCompilationContext,
) -> Result<bool, NativeCompilationSelection> {
    use NativeCompilationSelection as Selection;
    let max = if version == TclVersion::V8_4 { 2 } else { 3 };
    if shapes.is_empty() || shapes.len() > max {
        return Ok(false);
    }
    if shapes.len() == 1 {
        return Ok(true);
    }
    match context.frame {
        NativeCompilationFrame::ScriptCode => return Err(Selection::Generic),
        NativeCompilationFrame::Unknown => return Err(Selection::Unknown),
        NativeCompilationFrame::ProcedureCode => {}
    }
    for index in 1..shapes.len() {
        let Some(name) = words.arguments().literal_at(index) else {
            return Err(Selection::Generic);
        };
        if tcl_syntax::naming::is_qualified(name.as_bytes())
            || tcl_syntax::naming::split_element_ref(name).is_some()
        {
            return Err(Selection::Generic);
        }
    }
    Ok(true)
}
