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

//! Inline emitters for list, array and dict commands: `list` with expansion, `lindex`, `lrange`, `lreplace`, `linsert`, `array` and `dict get`.

use super::super::{CodegenCtx, Op, Operand, bytecode_imm, end_relative_imm};
use super::parse_cmd_parts_with_config;
use tcl_cmd_core::index::{Encoded, IndexCompiler};
use tcl_compiler::ir::CommandTokens;
use tcl_registry::hooks::InlineCodegenHookId;
use tcl_source::words::WordExpr;

impl CodegenCtx<'_> {
    /// Inline compile `[list {*}$a {*}$b]` as `load a; load b; listConcat`.
    ///
    /// Only two expanded whole scalar references are eligible. The configured
    /// original word owner supplies command boundaries and expansion markers;
    /// the shared variable scanner supplies the complete reference shape.
    /// Registry and retained command-binding checks remain independent.
    pub fn try_list_expand_concat(&mut self, value: &str) -> bool {
        if self.plain_command_dispatch || !value.starts_with('[') || !value.ends_with(']') {
            return false;
        }
        let Ok(end) = u32::try_from(value.len()) else {
            return false;
        };
        let config = self.lexer_config();
        let Ok(plan) = tcl_lexer::native_script_words_in_with_purpose(
            tcl_lexer::SourceImage::document(value),
            tcl_lexer::Span::new(1, end - 1),
            config,
            tcl_lexer::SourceSegmentationPurpose::NestedScript,
        ) else {
            return false;
        };
        if plan.fatal_tail.is_some() || plan.commands.len() != 1 {
            return false;
        }
        let [head, left, right] = plan.commands[0].words.as_slice() else {
            return false;
        };
        if head.written_bytes() != b"list" {
            return false;
        }
        let mut references = Vec::new();
        let mut call_args = Vec::new();
        for word in [left, right] {
            if !word.group().expand || word.group().kind != tcl_lexer::WordKind::Bare {
                return false;
            }
            let Ok(reference) = std::str::from_utf8(word.bytes()) else {
                return false;
            };
            if tcl_syntax::naming::whole_scalar_variable_reference_name(reference, config).is_none()
            {
                return false;
            }
            let Ok(written) = std::str::from_utf8(word.written_bytes()) else {
                return false;
            };
            references.push(reference);
            call_args.push(written);
        }
        let Some(binding) =
            self.trusted_inline_codegen_binding("list", &call_args, InlineCodegenHookId::List)
        else {
            return false;
        };
        self.require_command_binding(&binding);
        for reference in references {
            assert!(self.emit_variable_reference(reference));
        }
        self.emit(Op::LIST_CONCAT, vec![]);
        true
    }

    /// Inline compile `[list arg ... [break] ...]` or
    /// `[list arg ... [continue] ...]` when a loop-break/continue
    /// target is in scope.
    ///
    /// tclsh 9.0 compiles `break`/`continue` inside a `list` command
    /// substitution as an inline jump with stack cleanup: values
    /// pushed for earlier list arguments are popped, then a `jump4`
    /// goes to the loop target. The trailing `list N` and any outer
    /// command instructions become dead code that the bytecode layout
    /// still lays out.
    ///
    /// Returns `true` if the pattern matched and was emitted.
    pub fn try_inline_list_with_break_continue(&mut self, value: &str) -> bool {
        if self.plain_command_dispatch || !(value.starts_with("[list ") && value.ends_with(']')) {
            return false;
        }
        let Some(parts) = parse_cmd_parts_with_config(value, self.lexer_config()) else {
            return false;
        };
        if parts.first().map(|(n, _)| n.as_str()) != Some("list") {
            return false;
        }
        let args = &parts[1..];
        let has_bc = args
            .iter()
            .any(|(a, _)| a == "[break]" || a == "[continue]");
        if !has_bc {
            return false;
        }

        let inline_break = self.break_target.is_some() && args.iter().any(|(a, _)| a == "[break]");
        let inline_continue =
            self.continue_target.is_some() && args.iter().any(|(a, _)| a == "[continue]");
        let arg_refs: Vec<&str> = args.iter().map(|(arg, _)| arg.as_str()).collect();
        let Some(list_binding) =
            self.trusted_inline_codegen_binding("list", &arg_refs, InlineCodegenHookId::List)
        else {
            return false;
        };
        let break_binding = if inline_break {
            let Some(binding) =
                self.trusted_inline_codegen_binding("break", &[], InlineCodegenHookId::Break)
            else {
                return false;
            };
            Some(binding)
        } else {
            None
        };
        let continue_binding = if inline_continue {
            let Some(binding) =
                self.trusted_inline_codegen_binding("continue", &[], InlineCodegenHookId::Continue)
            else {
                return false;
            };
            Some(binding)
        } else {
            None
        };
        self.require_command_binding(&list_binding);
        if let Some(binding) = &break_binding {
            self.require_command_binding(binding);
        }
        if let Some(binding) = &continue_binding {
            self.require_command_binding(binding);
        }

        let mut n_pushed: usize = 0;
        for (arg, _braced) in args {
            if arg == "[break]" && self.break_target.is_some() {
                let target = self.break_target.clone().unwrap();
                let end_label = self.fresh_label("cmd_end");
                self.emit(
                    Op::START_CMD,
                    vec![Operand::Label(end_label.clone()), Operand::Imm(1)],
                );
                self.cmd_index += 1;
                for _ in 0..n_pushed {
                    self.emit(Op::POP, vec![]);
                }
                self.emit_comment(Op::JUMP4, vec![Operand::Label(target)], "break");
                self.place_label(&end_label);
                n_pushed += 1; // dead-code placeholder
            } else if arg == "[continue]" && self.continue_target.is_some() {
                let target = self.continue_target.clone().unwrap();
                let end_label = self.fresh_label("cmd_end");
                self.emit(
                    Op::START_CMD,
                    vec![Operand::Label(end_label.clone()), Operand::Imm(1)],
                );
                self.cmd_index += 1;
                for _ in 0..n_pushed {
                    self.emit(Op::POP, vec![]);
                }
                self.emit_comment(Op::JUMP4, vec![Operand::Label(target)], "continue");
                self.place_label(&end_label);
                n_pushed += 1;
            } else {
                self.emit_value(arg, false);
                n_pushed += 1;
            }
        }
        let arg_count = i32::try_from(args.len()).unwrap_or(i32::MAX);
        self.emit(Op::LIST, vec![Operand::Imm(arg_count)]);
        true
    }

    /// Emit only retained original operands of the selected list-index operation.
    pub(super) fn emit_inline_lindex(&mut self, tokens: Option<&CommandTokens>) -> bool {
        let Some(invocation) = tokens.and_then(|tokens| {
            tcl_compiler::registry_invocation::admitted_native_compiler_invocation(
                self.registry,
                None,
                tokens,
            )
        }) else {
            return false;
        };
        let Some(operands) = invocation
            .effective_words()
            .words
            .get(1 + invocation.argument_offset()..)
            .filter(|operands| !operands.is_empty())
        else {
            return false;
        };
        let immediate = (operands.len() == 2)
            .then(|| {
                invocation.with_argument_words(|words| {
                    let arguments = words.arguments();
                    let dialect = arguments.dialect()?;
                    let release = dialect.tcl_version?;
                    if release < tcl_dialect::TclVersion::V8_5 {
                        return None;
                    }
                    let text = arguments.literal_at(1)?;
                    // The literal index as the invocation's release compiles it
                    // (8.6's `TclIndexEncode`), or as it reads it running when
                    // the release evaluates the command directly.
                    match tcl_cmd_core::index::encode(text, release, !self.in_direct_command())? {
                        Encoded::Index(index) => i32::try_from(index).ok(),
                        Encoded::FromEnd(offset) if offset <= 0 => end_relative_imm(offset),
                        Encoded::FromEnd(_) => None,
                        Encoded::Before => Some(-1),
                        Encoded::After => Some(i32::MAX),
                    }
                })
            })
            .flatten();
        for (index, word) in operands.iter().enumerate() {
            if index == 1 && immediate.is_some() {
                continue;
            }
            self.emit_word_from_source(
                &word.legacy_text(),
                matches!(word, WordExpr::BracedLiteral { .. }),
                Some(word),
            );
        }
        if let Some(index) = immediate {
            self.emit(Op::LIST_INDEX_IMM, vec![Operand::Imm(index)]);
        } else if operands.len() == 2 {
            self.emit(Op::LIST_INDEX, vec![]);
        } else {
            self.emit(
                Op::LINDEX_MULTI,
                vec![Operand::Imm(bytecode_imm(operands.len()))],
            );
        }
        self.used_inline_cmd_subst = true;
        true
    }

    pub(super) fn emit_inline_lrange(&mut self, args: &[(String, bool)]) {
        if self.try_emit_lrange_imm(args) {
            self.used_inline_cmd_subst = true;
        } else {
            self.used_inline_cmd_subst = false;
            self.emit_generic_cmd_subst("lrange", args);
        }
    }

    /// Emit `lrange list first last` whose indices are both literal words as
    /// `LIST_RANGE_IMM`, deciding before any argument is pushed so a caller
    /// that falls back pushes the list once.
    pub(in super::super) fn try_emit_lrange_imm(&mut self, args: &[(String, bool)]) -> bool {
        let [value, first, last] = args else {
            return false;
        };
        let (Some(first), Some(last)) = (
            self.literal_index_imm(&first.0),
            self.literal_index_imm(&last.0),
        ) else {
            return false;
        };
        self.emit_native_argument_word(0, &value.0, value.1);
        self.emit(
            Op::LIST_RANGE_IMM,
            vec![Operand::Imm(first), Operand::Imm(last)],
        );
        true
    }

    pub(super) fn emit_inline_lreplace(&mut self, args: &[(String, bool)]) {
        self.used_inline_cmd_subst = true;
        self.emit_native_argument_word(0, &args[0].0, args[0].1);
        let indices = args.len().min(3);
        self.emit_index_words(IndexCompiler::Lreplace, 1, &args[1..indices]);
        for (index, a) in args.iter().enumerate().skip(indices) {
            self.emit_native_argument_word(index, &a.0, a.1);
        }
        self.emit(
            Op::LREPLACE4,
            vec![Operand::Imm(bytecode_imm(args.len())), Operand::Imm(1)],
        );
    }

    pub(super) fn emit_inline_linsert(&mut self, args: &[(String, bool)]) {
        self.used_inline_cmd_subst = true;
        self.emit_native_argument_word(0, &args[0].0, args[0].1);
        let indices = args.len().min(2);
        self.emit_index_words(IndexCompiler::Linsert, 1, &args[1..indices]);
        for (index, a) in args.iter().enumerate().skip(indices) {
            self.emit_native_argument_word(index, &a.0, a.1);
        }
        self.emit(
            Op::LREPLACE4,
            vec![Operand::Imm(bytecode_imm(args.len())), Operand::Imm(2)],
        );
    }

    pub(super) fn emit_inline_array(&mut self, entered_command: &str, args: &[(String, bool)]) {
        let sub = &args[0].0;
        let rest = &args[1..];
        if sub == "exists" && rest.len() == 1 {
            let tokens = self.invocation_tokens.clone();
            let source_word = tokens.as_deref().and_then(|tokens| {
                tcl_compiler::registry_invocation::admitted_native_compiler_invocation(
                    self.registry,
                    None,
                    tokens,
                )
                .and_then(|invocation| {
                    invocation
                        .effective_words()
                        .words
                        .get(1 + invocation.argument_offset())
                        .cloned()
                })
                .or_else(|| {
                    (self.native_entry.is_none() && tokens.source_binding.is_none())
                        .then(|| tokens.words().get(2).cloned())
                        .flatten()
                })
            });
            let slot = source_word
                .as_ref()
                .and_then(|word| self.original_scalar_operand_slot(word));
            if let Some(slot) = slot {
                self.used_inline_cmd_subst = true;
                self.emit(Op::ARRAY_EXISTS_IMM, vec![Operand::Imm(bytecode_imm(slot))]);
            } else {
                self.used_inline_cmd_subst = false;
                self.emit_generic_cmd_subst(entered_command, args);
            }
        } else if (sub == "names" || sub == "size") && !rest.is_empty() {
            let Some(target_address) = self
                .registry
                .profile()
                .map(tcl_native_model::InvocationDialect::of_profile)
                .and_then(|dialect| {
                    dialect.ensemble_implementation_command_address(
                        tcl_native_model::EnsembleImplementationFamily::Array,
                        sub.as_bytes(),
                    )
                })
            else {
                self.used_inline_cmd_subst = false;
                self.emit_generic_cmd_subst(entered_command, args);
                return;
            };
            let sc_end = self.fresh_label("subcmd_end");
            self.emit_comment(
                Op::START_CMD,
                vec![Operand::Label(sc_end.clone()), Operand::Imm(1)],
                "",
            );
            self.push_lit_bytes_exact(&target_address);
            for (index, (a, b)) in rest.iter().enumerate() {
                self.emit_native_argument_word(index + 1, a, *b);
            }
            let arg_count = bytecode_imm(1 + rest.len());
            let invoke_op = if arg_count < 256 {
                Op::INVOKE_STK1
            } else {
                Op::INVOKE_STK4
            };
            self.emit(invoke_op, vec![Operand::Imm(arg_count)]);
            self.place_label(&sc_end);
            self.seen_generic_invoke = true;
        } else {
            self.used_inline_cmd_subst = false;
            self.emit_generic_cmd_subst(entered_command, args);
        }
    }

    pub(super) fn emit_inline_dict_get(&mut self, args: &[(String, bool)]) {
        self.used_inline_cmd_subst = true;
        let dict_args = &args[1..]; // skip "get"
        self.emit_native_argument_word(1, &dict_args[0].0, dict_args[0].1); // dict value
        let keys = &dict_args[1..];
        for (index, (k, b)) in keys.iter().enumerate() {
            self.emit_native_argument_word(index + 2, k, *b);
        }
        self.emit(Op::DICT_GET, vec![Operand::Imm(bytecode_imm(keys.len()))]);
    }
}
