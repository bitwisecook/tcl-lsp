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

//! Literal index words as the release's compiler encodes them: `string range`, `string replace`, `lrange`, `linsert` and `lreplace`.

use super::super::{CodegenCtx, INDEX_END, Op, Operand, end_relative_imm};
use tcl_cmd_core::index::IndexCompiler;
use tcl_registry::hooks::InlineCodegenHookId;

impl CodegenCtx<'_> {
    /// The `*_IMM` operand for the literal index word `word`, as the target
    /// release reads it whatever the length: compiled with its command, or,
    /// in a command the release evaluates without compiling it, read as the
    /// command runs. `None` leaves the word to the opcode that reads it at
    /// run time: a word whose reading turns on the length, an `end+N` the
    /// release does not settle after the end, and an `end-N` past
    /// [`tcl_bytecode::INDEX_END_BAND`].
    pub(crate) fn literal_index_imm(&self, word: &str) -> Option<i32> {
        use tcl_cmd_core::index::Encoded;
        match tcl_cmd_core::index::encode(word, self.vm_release(), !self.in_direct_command())? {
            Encoded::Index(index) => i32::try_from(index).ok(),
            Encoded::FromEnd(offset) if offset <= 0 => end_relative_imm(offset),
            Encoded::FromEnd(_) => None,
            Encoded::Before => Some(-1),
            Encoded::After => Some(i32::MAX),
        }
    }

    /// The release the VM runs this compile under.
    pub(in super::super) fn vm_release(&self) -> tcl_dialect::TclVersion {
        self.dialect
            .map_or(tcl_dialect::TclVersion::V9_0, |profile| {
                profile.vm_runtime_version
            })
    }

    /// The index words of a command `compiler` compiles, each respelt where
    /// the release compiles it to a reading apart from its command's
    /// ([`tcl_cmd_core::index::compiled_index_words`]), `None` where it is
    /// pushed as written. A command the release evaluates without compiling
    /// it reads every word as written.
    pub(in super::super) fn compiled_index_words(
        &self,
        compiler: IndexCompiler,
        words: &[&str],
    ) -> Vec<Option<&'static str>> {
        if self.in_direct_command() {
            return vec![None; words.len()];
        }
        let words: Vec<Option<&str>> = words.iter().copied().map(Some).collect();
        tcl_cmd_core::index::compiled_index_words(compiler, &words, self.vm_release())
    }

    /// Push the index words of a command whose opcode reads them at run time,
    /// as the release compiles them ([`Self::compiled_index_words`]). A word
    /// pushed as written keeps its original operand, argument `first_index`
    /// onwards.
    pub(super) fn emit_index_words(
        &mut self,
        compiler: IndexCompiler,
        first_index: usize,
        words: &[(String, bool)],
    ) {
        let texts: Vec<&str> = words.iter().map(|(word, _)| word.as_str()).collect();
        let respelt = self.compiled_index_words(compiler, &texts);
        for (offset, ((word, braced), respelt)) in words.iter().zip(respelt).enumerate() {
            match respelt {
                Some(spelling) => self.push_lit_exact(spelling),
                None => self.emit_native_argument_word(first_index + offset, word, *braced),
            }
        }
    }

    /// Whether any of `words`, the index words of a command `compiler`
    /// compiles, is respelt ([`Self::compiled_index_words`]).
    fn respells_index_words(&self, compiler: IndexCompiler, words: &[(String, bool)]) -> bool {
        let texts: Vec<&str> = words.iter().map(|(word, _)| word.as_str()).collect();
        self.compiled_index_words(compiler, &texts)
            .iter()
            .any(Option::is_some)
    }

    /// Emit `string range`, `string replace`, `lrange`, `linsert` or
    /// `lreplace` through its specialised opcode where one of its literal
    /// indices reads apart compiled and as the command runs, so a command the
    /// release compiles reads it compiled where this compiler would otherwise
    /// keep the generic invoke.
    pub(in super::super) fn try_emit_compiled_index_command(
        &mut self,
        cmd: &str,
        args: &[(String, bool)],
    ) -> bool {
        let apart = |ctx: &Self, words: &[(String, bool)]| {
            !ctx.in_direct_command()
                && words
                    .iter()
                    .any(|(word, _)| tcl_cmd_core::index::encodes_apart(word, ctx.vm_release()))
        };
        let emitted = match (self.inline_cmd_subst_hook_candidate(cmd, args), args) {
            (Some(InlineCodegenHookId::String), [_, _, indices @ ..]) if apart(self, indices) => {
                // The hook's own subcommand dispatch settles which opcode.
                let Some(canonical) = self.canonical_subcommand_args(cmd, args) else {
                    return false;
                };
                let Some((_, binding)) = self.inline_cmd_subst_resolution(cmd, args) else {
                    return false;
                };
                self.emit_inline_string(cmd, &binding, &canonical);
                return true;
            }
            (Some(InlineCodegenHookId::StringReplace), [_, _, indices @ ..])
                if apart(self, indices) =>
            {
                self.try_emit_string_replace(args)
            }
            (Some(InlineCodegenHookId::Lrange), [_, indices @ ..]) if apart(self, indices) => {
                self.try_emit_lrange_imm(args)
            }
            (Some(InlineCodegenHookId::Linsert), [_, index, _, ..])
                if self
                    .respells_index_words(IndexCompiler::Linsert, std::slice::from_ref(index)) =>
            {
                self.emit_inline_linsert(args);
                true
            }
            (Some(InlineCodegenHookId::Lreplace), [_, _, _, ..])
                if self.respells_index_words(IndexCompiler::Lreplace, &args[1..3]) =>
            {
                self.emit_inline_lreplace(args);
                true
            }
            _ => false,
        };
        if emitted {
            self.inline_cmd_subst_resolution(cmd, args);
        }
        emitted
    }

    /// `string replace string first last ?newString?` as 8.6's
    /// `TclCompileStringReplaceCmd` compiles it. With both indices literal it
    /// settles a no-op to the string itself, and some ranges to slices of it
    /// joined with the replacement; anything else is `INST_STR_REPLACE` over
    /// the words as written. Emits nothing for another count, or where the
    /// release evaluates the command directly. `args` starts at the
    /// subcommand word, so each operand keeps its original argument index.
    pub(super) fn try_emit_string_replace(&mut self, args: &[(String, bool)]) -> bool {
        const BEFORE: i32 = -1;
        const AFTER: i32 = i32::MAX;
        let ([_, string, first, last] | [_, string, first, last, _]) = args else {
            return false;
        };
        if self.in_direct_command() {
            return false;
        }
        let replacement = args.get(4);
        self.used_inline_cmd_subst = true;
        self.emit_native_argument_word(1, &string.0, string.1);
        let settled = self
            .literal_index_imm(&first.0)
            .zip(self.literal_index_imm(&last.0))
            .and_then(|(first, last)| {
                let at_end = |index: i32| index <= INDEX_END;
                let no_op = last == BEFORE
                    || first == AFTER
                    || (at_end(first) && at_end(last) && last < first)
                    || (first >= 0 && last >= 0 && last < first);
                if no_op {
                    if let Some((word, braced)) = replacement {
                        self.emit_native_argument_word(4, word, *braced);
                        self.emit(Op::POP, vec![]);
                    }
                    return Some(());
                }
                match replacement {
                    Some((word, braced)) if first == BEFORE && last >= 0 => {
                        self.emit_native_argument_word(4, word, *braced);
                        self.emit(Op::REVERSE, vec![Operand::Imm(2)]);
                        if last == AFTER {
                            self.emit(Op::POP, vec![]);
                        } else {
                            self.emit_str_range_imm(last + 1, INDEX_END);
                            self.emit(Op::STR_CONCAT1, vec![Operand::Imm(2)]);
                        }
                    }
                    Some((word, braced)) if last == AFTER && at_end(first) => {
                        self.emit_str_range_imm(0, first - 1);
                        self.emit_native_argument_word(4, word, *braced);
                        self.emit(Op::STR_CONCAT1, vec![Operand::Imm(2)]);
                    }
                    Some(_) => return None,
                    None if first == BEFORE || first == 0 => {
                        if last == INDEX_END || last == AFTER {
                            self.emit(Op::POP, vec![]);
                            self.push_lit("");
                        } else {
                            self.emit_str_range_imm(last + 1, INDEX_END);
                        }
                    }
                    None if last == INDEX_END || last == AFTER => {
                        self.emit_str_range_imm(0, first - 1);
                    }
                    None => {
                        self.emit(Op::DUP, vec![]);
                        self.emit_str_range_imm(0, first - 1);
                        self.emit(Op::REVERSE, vec![Operand::Imm(2)]);
                        self.emit_str_range_imm(last + 1, INDEX_END);
                        self.emit(Op::STR_CONCAT1, vec![Operand::Imm(2)]);
                    }
                }
                Some(())
            });
        if settled.is_none() {
            self.emit_native_argument_word(2, &first.0, first.1);
            self.emit_native_argument_word(3, &last.0, last.1);
            match replacement {
                Some((word, braced)) => self.emit_native_argument_word(4, word, *braced),
                None => self.push_lit(""),
            }
            self.emit(Op::STR_REPLACE, vec![]);
        }
        true
    }

    pub(super) fn emit_str_range_imm(&mut self, first: i32, last: i32) {
        self.emit(
            Op::STR_RANGE_IMM,
            vec![Operand::Imm(first), Operand::Imm(last)],
        );
    }
}
