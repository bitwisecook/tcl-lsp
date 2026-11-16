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

//! Statements the registry already lowered structurally: name places, original operands and native constants.

use super::super::cells::{
    CellPlace, cell_place, has_substitution, name_data_place, whole_reference,
};
use super::super::ir::{NativeOp, NativeType, NativeValueId};
use super::super::representation::Representation;
use super::super::{NativeLoweringDecline, StatementOutcome};
use super::Lowerer;
use super::words::variable_place;
use crate::executable_ir::LoweredOperation;
use crate::ir::{CommandTokens, SourceSite, Statement, WordExpr};
use tcl_core_types::Code as CompletionCode;
use tcl_syntax::number::Number;

impl Lowerer<'_> {
    pub(super) fn source_absent_model(&self) -> bool {
        self.input.module.source.is_empty()
            && self.input.module.retained_source_bindings.is_none()
            && self.input.module.source_metadata_input.is_none()
            && self.input.module.source_entry.native_entry.is_none()
            && self
                .input
                .module
                .source_entry
                .metadata_context
                .is_standalone()
            && !self.input.module.source_entry.unknown_entry
    }

    fn lowered_name_place(
        &self,
        operation: &LoweredOperation,
        name: &str,
        braced: bool,
    ) -> Option<CellPlace> {
        if let Some(tokens) = operation.original_tokens.as_deref() {
            (operation.source.provenance == crate::ir::Provenance::Source).then_some(())?;
            let (command, _) = tokens
                .source_binding
                .as_ref()?
                .original_recorded_command()?;
            (command.span == operation.source.span).then_some(())?;
            (tokens.argv_texts.get(1).map(String::as_str) == Some(name)).then_some(())?;
            let value = crate::registry_invocation::original_literal_argument_data_for_module(
                self.input.registry,
                self.input.module,
                tokens,
                0,
            )?;
            return name_data_place(&value);
        }
        self.source_absent_model().then_some(())?;
        cell_place(name, braced)
    }

    fn original_lowered_argument<'b>(
        &self,
        operation: &'b LoweredOperation,
        argument: usize,
        expected: &str,
    ) -> Option<crate::registry_invocation::OriginalSourceArgument<'b>> {
        let tokens = operation.original_tokens.as_deref()?;
        (operation.source.provenance == crate::ir::Provenance::Source).then_some(())?;
        let (command, _) = tokens
            .source_binding
            .as_ref()?
            .original_recorded_command()?;
        (command.span == operation.source.span).then_some(())?;
        (tokens
            .argv_texts
            .get(argument.checked_add(1)?)
            .map(String::as_str)
            == Some(expected))
        .then_some(())?;
        crate::registry_invocation::original_source_argument_for_module(
            self.input.registry,
            self.input.module,
            tokens,
            argument,
        )
    }

    fn lower_original_operand(
        &mut self,
        operation: &LoweredOperation,
        text: &str,
    ) -> Option<NativeValueId> {
        if self.source_absent_model() {
            return self.lower_operand_word(text);
        }
        let argument = self.original_lowered_argument(operation, 1, text)?;
        if let Some(text) = argument.literal_data() {
            if self.representation
                && let Some(Number::Int(value)) = self.numbers.parse_whole(&text)
                && value.to_string() == text
            {
                return Some(self.const_int(value));
            }
            return Some(self.const_str(&text));
        }
        let (spelling, source) = argument.word().sole_variable_substitution()?;
        tcl_syntax::naming::whole_scalar_variable_reference_name(spelling, self.lexer_config)?;
        let place = variable_place(spelling, source, self.lexer_config).ok()?;
        let value = self.read_cell(&place);
        Some(self.as_int_operand(value))
    }

    // Already-lowered operations.

    #[allow(clippy::too_many_lines)]
    pub(super) fn lower_lowered(&mut self, operation: &LoweredOperation) -> StatementOutcome {
        match &operation.statement {
            Statement::AssignConst {
                name,
                name_braced,
                value,
                ..
            } => {
                let Some(place) = self.lowered_name_place(operation, name, *name_braced) else {
                    return self
                        .decline(&operation.source, NativeLoweringDecline::ComputedCellName);
                };
                let original;
                let value = if self.source_absent_model() {
                    value.as_str()
                } else {
                    let Some(argument) = self.original_lowered_argument(operation, 1, value) else {
                        return self.decline(
                            &operation.source,
                            NativeLoweringDecline::MissingCommandTokens,
                        );
                    };
                    let Some(value) = argument.literal_data() else {
                        return self
                            .decline(&operation.source, NativeLoweringDecline::SubstitutedOperand);
                    };
                    original = value;
                    &original
                };
                let boxed = self.const_str(value);
                let shadow = self.native_constant(value).unwrap_or(boxed);
                self.write_cell(&place, boxed, shadow);
                StatementOutcome::Native
            }
            Statement::AssignValue {
                name,
                name_braced,
                value,
                tokens,
                ..
            } => {
                let Some(place) = self.lowered_name_place(operation, name, *name_braced) else {
                    return self
                        .decline(&operation.source, NativeLoweringDecline::ComputedCellName);
                };
                let word = if self.source_absent_model() {
                    value_word(tokens.as_ref()).cloned()
                } else {
                    let Some(argument) = self.original_lowered_argument(operation, 1, value) else {
                        return self.decline(
                            &operation.source,
                            NativeLoweringDecline::MissingCommandTokens,
                        );
                    };
                    if let Some(value) = argument.literal_data() {
                        let boxed = self.const_str(&value);
                        let shadow = self.native_constant(&value).unwrap_or(boxed);
                        self.write_cell(&place, boxed, shadow);
                        return StatementOutcome::Native;
                    }
                    if matches!(
                        argument.word(),
                        WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. }
                    ) {
                        return self
                            .decline(&operation.source, NativeLoweringDecline::SubstitutedOperand);
                    }
                    Some(argument.word().clone())
                };
                let Some(word) = word else {
                    return self.decline(
                        &operation.source,
                        NativeLoweringDecline::MissingCommandTokens,
                    );
                };
                if let WordExpr::Literal { text, .. } | WordExpr::BracedLiteral { text, .. } = &word
                    && !text.contains('\\')
                {
                    let boxed = self.const_str(text);
                    let shadow = self.native_constant(text).unwrap_or(boxed);
                    self.write_cell(&place, boxed, shadow);
                    return StatementOutcome::Native;
                }
                match self.attempt(|this| this.lower_word(&word, 0)) {
                    Ok(value) => {
                        self.write_cell(&place, value, value);
                        StatementOutcome::Native
                    }
                    Err(reason) => self.decline(&operation.source, reason),
                }
            }
            Statement::AssignExpr {
                name,
                name_braced,
                expr,
                ..
            } => {
                let Some(place) = self.lowered_name_place(operation, name, *name_braced) else {
                    return self
                        .decline(&operation.source, NativeLoweringDecline::ComputedCellName);
                };
                let value = self.lower_expression(expr);
                let boxed = self.boxed(value);
                self.write_cell(&place, boxed, value);
                StatementOutcome::Native
            }
            Statement::Incr {
                name,
                name_braced,
                amount,
                ..
            } => {
                let Some(place) = self.lowered_name_place(operation, name, *name_braced) else {
                    return self
                        .decline(&operation.source, NativeLoweringDecline::ComputedCellName);
                };
                let delta = match amount.as_deref() {
                    None => self.const_int(1),
                    Some(text) => match self.lower_original_operand(operation, text) {
                        Some(value) => value,
                        None => {
                            return self.decline(
                                &operation.source,
                                NativeLoweringDecline::SubstitutedOperand,
                            );
                        }
                    },
                };
                self.lower_incr(&place, delta, amount.is_none());
                StatementOutcome::Native
            }
            Statement::ExprEval { expr, .. } => {
                let value = self.lower_expression(expr);
                let boxed = self.boxed(value);
                self.emit(NativeOp::Complete {
                    code: CompletionCode::Ok,
                    result: Some(boxed),
                });
                StatementOutcome::Native
            }
            Statement::Return {
                value,
                expr,
                braced,
                ..
            } => {
                let result = match (expr, value) {
                    (Some(expr), _) => {
                        let value = self.lower_expression(expr);
                        Some(self.boxed(value))
                    }
                    (None, Some(text)) if *braced || !has_substitution(text) => {
                        Some(self.const_str(text))
                    }
                    (None, Some(_)) => {
                        return self
                            .decline(&operation.source, NativeLoweringDecline::SubstitutedOperand);
                    }
                    (None, None) => None,
                };
                self.emit(NativeOp::Complete {
                    code: CompletionCode::Return,
                    result,
                });
                StatementOutcome::Native
            }
            _ => self.decline(
                &operation.source,
                NativeLoweringDecline::OpaqueRegion(Some(operation.descriptor)),
            ),
        }
    }

    pub(super) fn decline(
        &mut self,
        source: &SourceSite,
        reason: NativeLoweringDecline,
    ) -> StatementOutcome {
        // Drop anything the statement emitted before declining.
        if let Some(buffer) = self.ops.last_mut() {
            buffer.clear();
        }
        self.eval_source(source, reason);
        StatementOutcome::EvalSource(reason)
    }

    /// The native constant a literal value text denotes, when boxing that
    /// native value would print exactly the text again.
    fn native_constant(&mut self, text: &str) -> Option<NativeValueId> {
        if !self.representation {
            return None;
        }
        match self.numbers.parse_whole(text)? {
            Number::Int(value) if value.to_string() == text => Some(self.const_int(value)),
            Number::Double(value)
                if value.is_finite() && tcl_syntax::number::format_double(value) == text =>
            {
                let dst = self.new_value(
                    NativeType::F64,
                    Representation::NativeDouble { finite: true },
                );
                self.emit(NativeOp::ConstDouble { dst, value });
                Some(dst)
            }
            _ => None,
        }
    }

    /// A single retained operand word (an `incr` amount): a literal, or one
    /// bare scalar variable reference.
    fn lower_operand_word(&mut self, text: &str) -> Option<NativeValueId> {
        if text.starts_with('$') {
            // A retained `$name` word carries no lexical extent here, so the
            // owner's name reading decides it: an element access is not one
            // scalar operand.
            let place = name_data_place(whole_reference(text, self.lexer_config)?)?;
            if matches!(place, CellPlace::Element { .. }) {
                return None;
            }
            let value = self.read_cell(&place);
            return Some(self.as_int_operand(value));
        }
        if has_substitution(text) {
            return None;
        }
        if let Some(Number::Int(value)) = self.numbers.parse_whole(text)
            && self.representation
        {
            return Some(self.const_int(value));
        }
        Some(self.const_str(text))
    }

    /// The `i64` form of an integer operand when it is native, else its
    /// boxed form (the runtime then owns the conversion and its error).
    fn as_int_operand(&mut self, value: NativeValueId) -> NativeValueId {
        match self.ty(value) {
            NativeType::I64 | NativeType::Obj => value,
            NativeType::Bool => self.bool_to_int(value),
            NativeType::F64 => self.boxed(value),
        }
    }
}

/// The value word of a `set name value` statement's retained tokens.
fn value_word(tokens: Option<&CommandTokens>) -> Option<&WordExpr> {
    let tokens = tokens?;
    if !tokens.words_align_with_argv_text() || tokens.words().len() != 3 {
        return None;
    }
    tokens.words().get(2)
}
