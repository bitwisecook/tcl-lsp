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

//! Selection over original byte words, one method per compiler family (string slices, values, bindings, control, namespaces).

use super::grammar_selection::clock_reading_selection;
use super::introspection_grammars::original_namespace_which_grammar;
use super::string_grammars::NamedCompilerOperandView;
use super::variable_and_list_grammars::variable_append_grammar_bytes;
use super::{
    NativeAppendKind, NativeBodyCompilation, NativeCompilationContext, NativeCompilationFrame,
    NativeCompilationGrammar, NativeCompilationGuard, NativeCompilationSelection,
    NativeCompilationSpec, NativeCompilationWordShape,
};
use crate::{InvocationDialect, InvocationWords};
use tcl_dialect::TclVersion;

impl NativeCompilationSpec {
    fn select_original_string_slice_words(
        self,
        words: &crate::native_compiler_words::NativeCompilerWords<'_>,
        operand_from: usize,
        version: TclVersion,
        context: NativeCompilationContext,
    ) -> Option<NativeCompilationSelection> {
        match self.grammar {
            NativeCompilationGrammar::StringReplace(scope) => {
                Some(crate::native_string_replace_compilation::select_original(
                    words,
                    operand_from,
                    scope,
                    version,
                ))
            }
            NativeCompilationGrammar::StringInsert(scope) => {
                Some(crate::native_string_insert_compilation::select_original(
                    words,
                    operand_from,
                    scope,
                    version,
                ))
            }
            NativeCompilationGrammar::StringRange(scope) => {
                Some(crate::native_string_range_compilation::select_original(
                    words,
                    operand_from,
                    scope,
                    version,
                    self,
                    context,
                ))
            }
            NativeCompilationGrammar::ListReplacement => {
                Some(crate::native_list_replace_compilation::select_original(
                    words,
                    operand_from,
                    version,
                ))
            }
            _ => None,
        }
    }

    pub(super) fn select_original_registered_words(
        self,
        words: &crate::native_compiler_words::NativeCompilerWords<'_>,
        operand_from: usize,
        shapes: &[NativeCompilationWordShape],
        dialect: InvocationDialect,
        context: NativeCompilationContext,
        version: TclVersion,
    ) -> NativeCompilationSelection {
        if let Some(selection) =
            self.select_original_value_words(words, operand_from, version, context)
        {
            return selection;
        }
        if let Some(selection) =
            self.select_original_binding_words(words, operand_from, version, context)
        {
            return selection;
        }
        if matches!(
            self.grammar,
            NativeCompilationGrammar::Error
                | NativeCompilationGrammar::Throw
                | NativeCompilationGrammar::Conditional
                | NativeCompilationGrammar::ForLoop
                | NativeCompilationGrammar::Catch
                | NativeCompilationGrammar::WhileLoop
                | NativeCompilationGrammar::Foreach
                | NativeCompilationGrammar::Try
        ) {
            return self.select_original_control_words(
                words,
                operand_from,
                dialect,
                context,
                version,
            );
        }
        if matches!(
            self.grammar,
            NativeCompilationGrammar::TclOoHelper(_)
                | NativeCompilationGrammar::Tailcall
                | NativeCompilationGrammar::Switch
                | NativeCompilationGrammar::GlobalBindings
                | NativeCompilationGrammar::NamespaceVariableBindings
        ) || (self.grammar == NativeCompilationGrammar::CoroutineRelay
            && version >= TclVersion::V9_1)
        {
            return self.select_original_special_words(
                words,
                operand_from,
                shapes,
                context,
                version,
            );
        }
        self.select_original_operand_words(words, operand_from, shapes, dialect, context, version)
    }

    fn select_original_value_words(
        self,
        words: &crate::native_compiler_words::NativeCompilerWords<'_>,
        operand_from: usize,
        version: TclVersion,
        context: NativeCompilationContext,
    ) -> Option<NativeCompilationSelection> {
        use NativeCompilationSelection as Selection;
        if let NativeCompilationGrammar::ClockReading(reading) = self.grammar {
            return Some(clock_reading_selection(
                reading,
                words.literal(operand_from),
                &words.shapes()[operand_from..],
                version,
                self.operation,
            ));
        }
        if self.grammar == NativeCompilationGrammar::FormatStringConcat {
            if crate::native_format_compilation::dynamic_format_word_declines(
                words,
                operand_from,
                version,
                context,
                self.operation,
                self.body,
            ) {
                return Some(NativeCompilationSelection::Generic);
            }
            return Some(crate::native_format_compilation::select_with_values(
                &|index| words.literal(operand_from + index).map(<[u8]>::to_vec),
                &words.shapes()[operand_from..],
                version,
                context,
                self.operation,
            ));
        }
        if let Some(selection) =
            self.select_original_string_slice_words(words, operand_from, version, context)
        {
            return Some(selection);
        }
        if let NativeCompilationGrammar::MathOperator(operator) = self.grammar {
            return Some(
                match crate::native_mathop_compilation::compile_native_mathop(
                    words,
                    operand_from,
                    operator,
                    version,
                    context,
                ) {
                    Ok(Some(_)) => self.selected_grammar_result(true, version),
                    Ok(None) => Selection::Generic,
                    Err(_) => Selection::Unknown,
                },
            );
        }
        if let NativeCompilationGrammar::NamespaceString(operation) = self.grammar {
            return Some(
                if crate::native_namespace_string_compilation::compile_native_namespace_string(
                    words,
                    operand_from,
                    operation,
                    version,
                )
                .is_some()
                {
                    self.selected_grammar_result(true, version)
                } else {
                    Selection::Generic
                },
            );
        }
        if let Some(selection) =
            self.select_original_string_words(words, operand_from, version, context)
        {
            return Some(selection);
        }

        if let Some(kind) = self.introspection_compilation()
            && crate::native_introspection_compilation::compile_native_introspection(
                words,
                operand_from,
                kind,
                version,
            )
            .is_some()
        {
            return Some(Selection::Inline {
                operation: self.operation,
                guard: NativeCompilationGuard::BeforeArguments,
            });
        }
        if let Some((operation, scope)) = self.scalar_compilation() {
            return Some(crate::native_scalar_compilation::select_original(
                words,
                operand_from,
                operation,
                scope,
                version,
                self.operation,
            ));
        }

        None
    }

    fn select_original_string_words(
        self,
        words: &crate::native_compiler_words::NativeCompilerWords<'_>,
        operand_from: usize,
        version: TclVersion,
        context: NativeCompilationContext,
    ) -> Option<NativeCompilationSelection> {
        if let NativeCompilationGrammar::StringTrim { scope, .. } = self.grammar {
            return Some(crate::native_string_trim_compilation::select_original(
                words,
                operand_from,
                scope,
                version,
                self.operation,
            ));
        }
        if let NativeCompilationGrammar::StringIsAscii(scope) = self.grammar {
            if crate::native_string_is_compilation::private_original_declines(
                words,
                operand_from,
                scope,
                version,
                self.operation,
                self.body,
                context,
            ) {
                return Some(NativeCompilationSelection::Generic);
            }
            // Only the stock member's own compiler context is modelled.
            if self.body != NativeBodyCompilation::Inherit {
                return Some(NativeCompilationSelection::Unknown);
            }
            return Some(crate::native_string_is_compilation::select_original(
                words,
                operand_from,
                scope,
                version,
                self.operation,
            ));
        }
        if let NativeCompilationGrammar::StringMatch(scope) = self.grammar {
            return Some(crate::native_string_compilation::select_original(
                words,
                operand_from,
                scope,
                version,
                self.operation,
            ));
        }
        if self.grammar == NativeCompilationGrammar::Regexp
            && crate::native_regexp_compilation::modelled(version)
        {
            return Some(crate::native_regexp_compilation::select_original(
                words,
                operand_from,
                version,
                self.operation,
            ));
        }
        if let NativeCompilationGrammar::StringMap(scope) = self.grammar {
            return Some(crate::native_string_map_compilation::select_original(
                words,
                operand_from,
                scope,
                version,
            ));
        }
        if let NativeCompilationGrammar::StringOperand { scope, operation } = self.grammar {
            return Some(crate::native_string_operand_compilation::select_original(
                words,
                operand_from,
                operation,
                scope,
                version,
                self.operation,
            ));
        }

        None
    }

    fn select_original_binding_words(
        self,
        words: &crate::native_compiler_words::NativeCompilerWords<'_>,
        operand_from: usize,
        version: TclVersion,
        context: NativeCompilationContext,
    ) -> Option<NativeCompilationSelection> {
        use NativeCompilationSelection as Selection;
        if self.grammar == NativeCompilationGrammar::InfoExists {
            if version < TclVersion::V8_5 {
                return Some(Selection::Generic);
            }
            return Some(match crate::native_info_exists_compilation::compile_native_info_exists(
                words,
                operand_from,
                version,
            ) {
                Ok(_) => self.selected_grammar_result(true, version),
                Err(
                    crate::native_info_exists_compilation::NativeInfoExistsUnavailable::Geometry
                    | crate::native_info_exists_compilation::NativeInfoExistsUnavailable::Version,
                ) => Selection::Generic,
            });
        }

        if self.grammar == NativeCompilationGrammar::ListIndex {
            return Some(crate::native_list_index_compilation::select_original(
                words,
                operand_from,
                version,
                self,
            ));
        }

        if self.grammar == NativeCompilationGrammar::Upvar {
            if version == TclVersion::V8_4 || context.frame == NativeCompilationFrame::ScriptCode {
                return Some(Selection::Generic);
            }
            return Some(
                match crate::native_upvar_compilation::compile_native_upvar(
                    words,
                    operand_from,
                    version,
                    context,
                ) {
                    Ok(_) => self.selected_grammar_result(true, version),
                    Err(crate::native_upvar_compilation::NativeUpvarUnavailable::Geometry) => {
                        Selection::Generic
                    }
                    Err(_) => Selection::Unknown,
                },
            );
        }
        if self.grammar == NativeCompilationGrammar::NamespaceUpvarBindings {
            return Some(match crate::native_namespace_upvar_compilation::compile_native_namespace_upvar_worker(
                words, operand_from, version, context,
            ) {
                Ok(recipe) => self.namespace_binding_selection(recipe.outcome),
                Err(_) => Selection::Unknown,
            });
        }
        if self.grammar == NativeCompilationGrammar::NamespaceLegacy {
            return Some(self.select_original_namespace_legacy(
                words,
                operand_from,
                version,
                context,
            ));
        }
        if self.grammar == NativeCompilationGrammar::LiteralUnset {
            return Some(
                match crate::native_unset_compilation::compile_native_unset(
                    words,
                    operand_from,
                    version,
                ) {
                    Ok(Some(_)) => self.selected_grammar_result(true, version),
                    Ok(None) => Selection::Generic,
                    Err(_) => Selection::Unknown,
                },
            );
        }
        None
    }

    fn select_original_control_outcome<T>(
        self,
        outcome: &crate::native_control_compilation::NativeControlOutcome<T>,
        version: TclVersion,
    ) -> NativeCompilationSelection {
        use crate::native_control_compilation::NativeControlOutcome;
        match outcome {
            NativeControlOutcome::Inline(_) => self.selected_grammar_result(true, version),
            NativeControlOutcome::Generic => NativeCompilationSelection::Generic,
            NativeControlOutcome::Rejected(_) => NativeCompilationSelection::CompileError,
        }
    }

    fn select_original_namespace_legacy(
        self,
        words: &crate::native_compiler_words::NativeCompilerWords<'_>,
        operand_from: usize,
        version: TclVersion,
        context: NativeCompilationContext,
    ) -> NativeCompilationSelection {
        use crate::native_namespace_binding_compilation::NativeNamespaceBindingOutcome;
        match crate::native_namespace_upvar_compilation::compile_native_namespace_upvar(
            words,
            operand_from,
            version,
            context,
        ) {
            Ok(recipe) => match recipe.outcome {
                NativeNamespaceBindingOutcome::Inline => NativeCompilationSelection::Inline {
                    operation: self.operation,
                    guard: NativeCompilationGuard::BeforeArguments,
                },
                NativeNamespaceBindingOutcome::Generic => NativeCompilationSelection::Generic,
                NativeNamespaceBindingOutcome::Unknown => NativeCompilationSelection::Unknown,
            },
            Err(_) => NativeCompilationSelection::Unknown,
        }
    }

    fn select_original_control_words(
        self,
        words: &crate::native_compiler_words::NativeCompilerWords<'_>,
        operand_from: usize,
        dialect: InvocationDialect,
        context: NativeCompilationContext,
        version: TclVersion,
    ) -> NativeCompilationSelection {
        use NativeCompilationSelection as Selection;
        if matches!(
            self.grammar,
            NativeCompilationGrammar::Error | NativeCompilationGrammar::Throw
        ) {
            let compiled = if self.grammar == NativeCompilationGrammar::Throw {
                crate::native_error_compilation::compile_native_throw(words, operand_from, version)
            } else {
                crate::native_error_compilation::compile_native_error(words, operand_from, version)
            };
            return match compiled {
                Ok(_) => self.selected_grammar_result(true, version),
                Err(
                    crate::native_error_compilation::NativeErrorCompilationUnavailable::Generic,
                ) => Selection::Generic,
                Err(
                    crate::native_error_compilation::NativeErrorCompilationUnavailable::Geometry,
                ) => Selection::Unknown,
            };
        }
        if matches!(
            self.grammar,
            NativeCompilationGrammar::Conditional
                | NativeCompilationGrammar::ForLoop
                | NativeCompilationGrammar::Catch
                | NativeCompilationGrammar::WhileLoop
        ) {
            return match crate::native_control_instructions::native_control_instruction(
                self.grammar,
                words,
                operand_from,
                dialect,
                context,
            ) {
                Ok(recipe) => self.select_original_control_outcome(&recipe.outcome, version),
                Err(_) => Selection::Unknown,
            };
        }
        if self.grammar == NativeCompilationGrammar::Foreach {
            let Some(collection) = self.each_collection() else {
                return Selection::Unknown;
            };
            return match crate::native_each_compilation::compile_native_each(
                words,
                operand_from,
                version,
                context,
                collection,
            ) {
                Ok(recipe) => self.select_original_control_outcome(&recipe.outcome, version),
                Err(_) => Selection::Unknown,
            };
        }
        if self.grammar == NativeCompilationGrammar::Try {
            return match crate::native_try_compilation::compile_native_try(
                words,
                operand_from,
                version,
                context,
            ) {
                Ok(recipe) => self.select_original_control_outcome(&recipe.outcome, version),
                Err(_) => Selection::Unknown,
            };
        }
        unreachable!("selected original control grammar")
    }

    fn select_original_special_words(
        self,
        words: &crate::native_compiler_words::NativeCompilerWords<'_>,
        operand_from: usize,
        shapes: &[NativeCompilationWordShape],
        context: NativeCompilationContext,
        version: TclVersion,
    ) -> NativeCompilationSelection {
        use NativeCompilationSelection as Selection;
        if let NativeCompilationGrammar::TclOoHelper(helper) = self.grammar {
            return crate::native_tcloo_compilation::select_original(
                helper,
                words,
                operand_from,
                version,
                self.operation,
            );
        }
        if self.grammar == NativeCompilationGrammar::Tailcall {
            return self.select_tailcall(shapes, version, context);
        }
        if self.grammar == NativeCompilationGrammar::CoroutineRelay && version >= TclVersion::V9_1 {
            return self.selected_grammar_result(true, version);
        }
        if self.grammar == NativeCompilationGrammar::Switch {
            return match crate::native_switch_compilation::native_switch_instruction(
                words,
                operand_from,
                version,
            ) {
                Ok(_) => self.selected_grammar_result(true, version),
                Err(crate::native_switch_compilation::NativeSwitchUnavailable::Generic) => {
                    Selection::Generic
                }
                Err(_) => Selection::Unknown,
            };
        }
        if let Some(kind) = self.namespace_binding_kind() {
            return match crate::native_namespace_binding_compilation::compile_native_namespace_bindings(
                words, operand_from, version, context, kind,
            ) {
                Ok(recipe) => self.namespace_binding_selection(recipe.outcome),
                Err(_) => Selection::Unknown,
            };
        }
        unreachable!("selected original special grammar")
    }

    fn select_original_operand_words(
        self,
        words: &crate::native_compiler_words::NativeCompilerWords<'_>,
        operand_from: usize,
        shapes: &[NativeCompilationWordShape],
        dialect: InvocationDialect,
        context: NativeCompilationContext,
        version: TclVersion,
    ) -> NativeCompilationSelection {
        use NativeCompilationSelection as Selection;
        let expanded_list_append = self.grammar
            == NativeCompilationGrammar::VariableAppend(NativeAppendKind::List)
            && version >= TclVersion::V9_1;
        if let NativeCompilationGrammar::Dictionary { command, ensemble } = self.grammar
            && let Some(selection) =
                command.select_original_compilation(words, operand_from, ensemble, version, context)
        {
            return selection;
        }
        if let NativeCompilationGrammar::ArgumentConcatFrom(first) = self.grammar {
            if version < first {
                return Selection::Generic;
            }
            let Ok(projected) =
                crate::native_compiler_word_projection::project_native_compiler_words(
                    words, version,
                )
            else {
                return Selection::Unknown;
            };
            let Some(operands) = projected.get(operand_from..) else {
                return Selection::Unknown;
            };
            return self.selected_grammar_result(
                !operands
                    .iter()
                    .any(|word| word.shape == NativeCompilationWordShape::Expanded),
                version,
            );
        }
        if !expanded_list_append && shapes.contains(&NativeCompilationWordShape::Expanded) {
            // A static expansion may have been flattened by TclParseCommand
            // before the compiler sees its tokens. It needs that projection.
            return if (operand_from..words.shapes().len()).any(|index| {
                words.shapes()[index] == NativeCompilationWordShape::Expanded
                    && words.literal(index).is_some()
            }) {
                Selection::Unknown
            } else {
                Selection::Generic
            };
        }
        if self.grammar == NativeCompilationGrammar::NamespaceWhich {
            return original_namespace_which_grammar(words, operand_from, version, self.operation);
        }
        if let Some(selection) = self.select_shape_grammar(shapes, version, context) {
            return selection;
        }
        if let NativeCompilationGrammar::VariableAppend(kind) = self.grammar {
            return variable_append_grammar_bytes(
                kind,
                self.operation,
                words.literal(operand_from),
                shapes,
                version,
                context,
            );
        }
        if self.grammar == NativeCompilationGrammar::Return {
            return match crate::native_return_compilation::native_return_instruction(
                words,
                operand_from,
                version,
                context,
            ) {
                Ok(_) => self.selected_grammar_result(true, version),
                Err(
                    crate::native_return_compilation::NativeReturnCompilationUnavailable::Generic,
                ) => Selection::Generic,
                Err(_) => Selection::Unknown,
            };
        }
        let Some(projected) = words.checked_words() else {
            return Selection::Unknown;
        };
        self.select_with_named_operand_view(
            InvocationWords::structured(projected[0], &projected[operand_from..]),
            shapes,
            Some(dialect),
            context,
            NamedCompilerOperandView::SelectedWorker,
        )
    }
}
