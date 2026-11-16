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

//! Inline emitters for `info commands`, `info level`, `info exists`, `namespace origin` and `namespace code`.

use super::super::values::{is_qualified, split_array_ref};
use super::super::{CodegenCtx, Op, Operand, bytecode_imm};
use tcl_compiler::ir::{CommandTokens, WordExpr};

impl CodegenCtx<'_> {
    pub(super) fn emit_inline_info_commands_resolve(&mut self, args: &[(String, bool)]) {
        self.used_inline_cmd_subst = true;
        self.emit_native_argument_word(1, &args[1].0, args[1].1);
        self.emit(Op::RESOLVE_CMD, vec![]);
        self.emit(Op::DUP, vec![]);
        self.emit(Op::STR_LEN, vec![]);
        let end = self.fresh_label("info_commands_end");
        self.emit(Op::JUMP_FALSE1, vec![Operand::Label(end.clone())]);
        self.emit(Op::LIST, vec![Operand::Imm(1)]);
        self.place_label(&end);
    }

    /// The original compiler captures its opcode before argv; the operand
    /// still evaluates once and resolves its command in the entered namespace.
    pub(in crate::codegen) fn emit_inline_namespace_origin(
        &mut self,
        tokens: Option<&CommandTokens>,
    ) -> bool {
        let Some(tokens) = tokens else {
            return false;
        };
        let Some(invocation) =
            tcl_compiler::registry_invocation::admitted_native_compiler_invocation(
                self.registry,
                None,
                tokens,
            )
        else {
            return false;
        };
        let Some([word]) = invocation
            .effective_words()
            .words
            .get(1 + invocation.argument_offset()..)
        else {
            return false;
        };
        self.emit_word_from_source(
            &word.legacy_text(),
            matches!(word, WordExpr::BracedLiteral { .. }),
            Some(word),
        );
        self.emit(Op::ORIGIN_CMD, vec![]);
        self.used_inline_cmd_subst = true;
        true
    }

    /// The original literal builder captures the runtime namespace, rather
    /// than evaluating the script or baking in a compiler namespace name.
    pub(in crate::codegen) fn emit_inline_namespace_code(
        &mut self,
        tokens: Option<&CommandTokens>,
    ) -> bool {
        let Some(invocation) = tokens.and_then(|tokens| {
            tcl_compiler::registry_invocation::admitted_native_compiler_invocation(
                self.registry,
                None,
                tokens,
            )
        }) else {
            return false;
        };
        let Some([word]) = invocation
            .effective_words()
            .words
            .get(1 + invocation.argument_offset()..)
        else {
            return false;
        };
        self.push_lit("::namespace");
        self.push_lit("inscope");
        self.emit(Op::CURRENT_NAMESPACE, vec![]);
        self.emit_word_from_source(
            &word.legacy_text(),
            matches!(word, WordExpr::BracedLiteral { .. }),
            Some(word),
        );
        self.emit(Op::LIST, vec![Operand::Imm(4)]);
        self.used_inline_cmd_subst = true;
        true
    }

    pub(super) fn emit_inline_info_level(&mut self, tokens: Option<&CommandTokens>) -> bool {
        let Some(tokens) = tokens else {
            return false;
        };
        let Some(invocation) =
            tcl_compiler::registry_invocation::admitted_native_compiler_invocation(
                self.registry,
                None,
                tokens,
            )
        else {
            return false;
        };
        let Some(operands) = invocation
            .effective_words()
            .words
            .get(1 + invocation.argument_offset()..)
        else {
            return false;
        };
        match operands {
            [] => {
                self.emit(Op::INFO_LEVEL_NUM, vec![]);
            }
            [word] => {
                self.emit_word_from_source(
                    &word.legacy_text(),
                    matches!(word, WordExpr::BracedLiteral { .. }),
                    Some(word),
                );
                self.emit(Op::INFO_LEVEL_ARGS, vec![]);
            }
            _ => return false,
        }
        self.used_inline_cmd_subst = true;
        true
    }

    /// Emit the selected exists operand from its canonical source coordinate.
    /// A computed name evaluates once before the stack opcode; only a source
    /// word eligible for a direct local slot bypasses substitution.
    pub(super) fn emit_inline_info_exists(&mut self, tokens: Option<&CommandTokens>) -> bool {
        let Some(tokens) = tokens else {
            return false;
        };
        let Some(invocation) =
            tcl_compiler::registry_invocation::admitted_native_compiler_invocation(
                self.registry,
                None,
                tokens,
            )
        else {
            return false;
        };
        let Some(word) = invocation
            .effective_words()
            .words
            .get(1 + invocation.argument_offset())
        else {
            return false;
        };
        self.used_inline_cmd_subst = true;
        if let Some(slot) = self.emit_original_array_exists_key(word) {
            // `TclCompileInfoExistsCmd` names the array's compiled local and
            // pushes the key.
            self.emit(Op::EXIST_ARRAY, vec![Operand::Imm(bytecode_imm(slot))]);
        } else if let Some(slot) = self.original_scalar_operand_slot(word) {
            self.emit(Op::EXIST_SCALAR, vec![Operand::Imm(bytecode_imm(slot))]);
            self.emit(Op::NOP, vec![]);
        } else {
            let spelling = word.legacy_text();
            self.emit_word_from_source(
                &spelling,
                matches!(word, WordExpr::BracedLiteral { .. }),
                Some(word),
            );
            self.emit(Op::EXIST_STK, vec![]);
        }
        true
    }

    /// Push the key of an array-element operand whose array is a compiled
    /// local, returning that local's slot; emit nothing and answer `None`
    /// for any other operand. The original operand layout decides the key:
    /// a literal key is pushed as is, a compound one is evaluated. A
    /// hand-built context without that layout reads the word's text.
    fn emit_original_array_exists_key(&mut self, word: &WordExpr) -> Option<usize> {
        use tcl_syntax::native_variable_words::NativeVariableWordOperand;
        match self.original_variable_operand(word) {
            Some(NativeVariableWordOperand::Literal {
                name,
                index: Some(index),
                ..
            }) => {
                let slot = self.command_variable_slot(&name)?;
                self.push_lit_bytes_exact(&index);
                Some(slot)
            }
            Some(NativeVariableWordOperand::CompoundArray { name, index, .. }) => {
                let slot = self.command_variable_slot(&name)?;
                self.emit_executable_arena(&index);
                Some(slot)
            }
            None if self.native_entry.is_none()
                && self
                    .invocation_tokens
                    .as_deref()
                    .is_none_or(|tokens| tokens.source_binding.is_none()) =>
            {
                let spelling = word.legacy_text();
                let (base, key) = split_array_ref(&spelling)?;
                if !self.compiles_locals() || is_qualified(base) {
                    return None;
                }
                let slot = self.lvt.intern(base);
                self.push_array_key(key);
                Some(slot)
            }
            Some(_) | None => None,
        }
    }
}
