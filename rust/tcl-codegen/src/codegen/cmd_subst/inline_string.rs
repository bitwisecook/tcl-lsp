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

//! Inline emitters for `string` subcommands and `regexp`: comparison, `string is`, ranges, match and trim.

use super::super::{CodegenCtx, Op, Operand, bytecode_imm, str_class_id};
use super::is_literal_word;
use tcl_cmd_core::index::IndexCompiler;
use tcl_registry::hooks::InlineCodegenHookId;

impl CodegenCtx<'_> {
    /// Emit the `string equal {-nocase}? a b` / `string compare
    /// {-nocase|-length}? ...` forms via `INVOKE_REPLACE` against
    /// the FQN sub-emitter.  `flag_args` is the per-form prefix
    /// pushed before the operand args.  `total_argc` is the total
    /// argument count including the operand args; helper computes
    /// `INVOKE_REPLACE 0 1` operands as `(total_argc, 2)`.
    fn emit_inline_string_invoke_replace(
        &mut self,
        prev_inline: bool,
        entered: (&str, &tcl_runtime_api::CommandBindingIdentity),
        wrap_start_command: bool,
        target: (&str, &[u8]),
        flag_args: &[&str],
        operand_args: &[(String, bool)],
    ) {
        let (entered_command, entered_binding) = entered;
        let (target_subcmd, target_address) = target;
        self.used_inline_cmd_subst = prev_inline;
        let sc_end = wrap_start_command.then(|| {
            let label = self.fresh_label("subcmd_end");
            self.emit_comment(
                Op::START_CMD,
                vec![Operand::Label(label.clone()), Operand::Imm(1)],
                "",
            );
            label
        });
        let body_start = self.instructions.len();
        self.push_lit(entered_command);
        self.push_lit(target_subcmd);
        for f in flag_args {
            self.push_lit(f);
        }
        for (index, (a, b)) in operand_args.iter().enumerate() {
            self.emit_native_argument_word(1 + flag_args.len() + index, a, *b);
        }
        self.push_lit_bytes_exact(target_address);
        let argc = bytecode_imm(2 + flag_args.len() + operand_args.len());
        self.emit(
            Op::INVOKE_REPLACE,
            vec![Operand::Imm(argc), Operand::Imm(2)],
        );
        if let Some(sc_end) = sc_end {
            self.place_label(&sc_end);
        }
        self.retain_entered_command(body_start, Some(entered_binding.clone()));
        self.seen_generic_invoke = true;
    }

    /// Emit one of String's option-bearing `INVOKE_REPLACE` shapes.
    ///
    /// Both value and statement position call this one guard. The registry
    /// selects `InlineCodegenHookId::String`; this hook-owned dispatcher then
    /// selects the exact subcommand/option shape the opcode can represent.
    pub(super) fn try_emit_inline_string_invoke_replace(
        &mut self,
        prev_inline: bool,
        entered_command: &str,
        entered_binding: &tcl_runtime_api::CommandBindingIdentity,
        args: &[(String, bool)],
        wrap_start_command: bool,
    ) -> bool {
        let Some(((subcmd, _), sargs)) = args.split_first() else {
            return false;
        };
        let Some(target_address) = self
            .registry
            .profile()
            .map(tcl_registry::InvocationDialect::of_profile)
            .and_then(|dialect| {
                dialect.ensemble_implementation_command_address(
                    tcl_registry::EnsembleImplementationFamily::String,
                    subcmd.as_bytes(),
                )
            })
        else {
            return false;
        };
        match subcmd.as_str() {
            "equal" if sargs.len() == 3 && sargs[0].0 == "-nocase" => {
                self.emit_inline_string_invoke_replace(
                    prev_inline,
                    (entered_command, entered_binding),
                    wrap_start_command,
                    ("equal", &target_address),
                    &["-nocase"],
                    &sargs[1..],
                );
                true
            }
            "compare" if sargs.len() == 3 && sargs[0].0 == "-nocase" => {
                self.emit_inline_string_invoke_replace(
                    prev_inline,
                    (entered_command, entered_binding),
                    wrap_start_command,
                    ("compare", &target_address),
                    &["-nocase"],
                    &sargs[1..],
                );
                true
            }
            "compare" if sargs.len() == 4 && sargs[0].0 == "-length" => {
                self.emit_inline_string_invoke_replace(
                    prev_inline,
                    (entered_command, entered_binding),
                    wrap_start_command,
                    ("compare", &target_address),
                    &["-length"],
                    &sargs[1..],
                );
                true
            }
            _ => false,
        }
    }

    /// Emit a 2-arg `string equal a b` / `string compare a b` —
    /// shares the `is_proc ? no-START_CMD : with-START_CMD-wrap`
    /// scaffold.  `op` is the resulting bytecode opcode.
    fn emit_inline_string_2arg_op(&mut self, prev_inline: bool, sargs: &[(String, bool)], op: Op) {
        let sc_end = if self.is_proc {
            None
        } else {
            self.used_inline_cmd_subst = prev_inline;
            let label = self.fresh_label("subcmd_end");
            self.emit_comment(
                Op::START_CMD,
                vec![Operand::Label(label.clone()), Operand::Imm(1)],
                "",
            );
            Some(label)
        };
        self.emit_native_argument_word(1, &sargs[0].0, sargs[0].1);
        self.emit_native_argument_word(2, &sargs[1].0, sargs[1].1);
        self.emit(op, vec![]);
        if let Some(label) = sc_end {
            self.place_label(&label);
        }
    }

    /// Fall-through path: invoke `::tcl::string::<subcmd>` via
    /// `INVOKE_STK1`/`STK4`.
    fn emit_inline_string_fqn_invoke(
        &mut self,
        prev_inline: bool,
        entered_command: &str,
        subcmd: &str,
        sargs: &[(String, bool)],
    ) {
        self.used_inline_cmd_subst = prev_inline;
        let sc_end = self.fresh_label("subcmd_end");
        self.emit_comment(
            Op::START_CMD,
            vec![Operand::Label(sc_end.clone()), Operand::Imm(1)],
            "",
        );
        let target_address = self
            .registry
            .profile()
            .map(tcl_registry::InvocationDialect::of_profile)
            .and_then(|dialect| {
                dialect.ensemble_implementation_command_address(
                    tcl_registry::EnsembleImplementationFamily::String,
                    subcmd.as_bytes(),
                )
            });
        if let Some(address) = target_address.as_deref() {
            self.push_lit_bytes_exact(address);
        } else {
            self.push_lit(entered_command);
            self.push_lit(subcmd);
        }
        for (index, (a, b)) in sargs.iter().enumerate() {
            self.emit_native_argument_word(index + 1, a, *b);
        }
        let argc = bytecode_imm(usize::from(target_address.is_none()) + 1 + sargs.len());
        let invoke_op = if argc < 256 {
            Op::INVOKE_STK1
        } else {
            Op::INVOKE_STK4
        };
        self.emit(invoke_op, vec![Operand::Imm(argc)]);
        self.place_label(&sc_end);
        self.seen_generic_invoke = true;
    }

    pub(super) fn emit_inline_string(
        &mut self,
        entered_command: &str,
        entered_binding: &tcl_runtime_api::CommandBindingIdentity,
        args: &[(String, bool)],
    ) {
        let subcmd = &args[0].0;
        let sargs = &args[1..];
        let prev_inline = self.used_inline_cmd_subst;
        self.used_inline_cmd_subst = true;

        if self.try_emit_inline_string_invoke_replace(
            prev_inline,
            entered_command,
            entered_binding,
            args,
            true,
        ) {
            return;
        }

        match subcmd.as_str() {
            "index" if sargs.len() == 2 => {
                self.emit_native_argument_word(1, &sargs[0].0, sargs[0].1);
                self.emit_native_argument_word(2, &sargs[1].0, sargs[1].1);
                self.emit(Op::STR_INDEX, vec![]);
            }
            "range" if sargs.len() == 3 => {
                if !self.try_emit_string_range_imm(sargs) {
                    self.emit_native_argument_word(1, &sargs[0].0, sargs[0].1);
                    self.emit_index_words(IndexCompiler::StringRange, 2, &sargs[1..]);
                    self.emit(Op::STR_RANGE, vec![]);
                }
            }
            "equal" if sargs.len() == 2 => {
                self.emit_inline_string_2arg_op(prev_inline, sargs, Op::STR_EQ);
            }
            "compare" if sargs.len() == 2 => {
                self.emit_inline_string_2arg_op(prev_inline, sargs, Op::STR_CMP);
            }
            "length" if sargs.len() == 1 => {
                self.emit_native_argument_word(1, &sargs[0].0, sargs[0].1);
                self.emit(Op::STR_LEN, vec![]);
            }
            // `string is` and `string replace` compile only through their
            // own release-stamped hooks; under the `string` hook the command
            // runs them.
            "is" => {
                self.emit_string_is_generic(entered_command, entered_binding, sargs);
            }
            _ => {
                self.emit_inline_string_fqn_invoke(prev_inline, entered_command, subcmd, sargs);
            }
        }
    }

    /// Emit `string range` whose indices are both literal words as
    /// `STR_RANGE_IMM`, as C compiles it in value and statement position
    /// alike, so a literal index reads as the release compiles it.
    fn try_emit_string_range_imm(&mut self, sargs: &[(String, bool)]) -> bool {
        let [string, first, last] = sargs else {
            return false;
        };
        let (Some(first), Some(last)) = (
            self.literal_index_imm(&first.0),
            self.literal_index_imm(&last.0),
        ) else {
            return false;
        };
        self.emit_native_argument_word(1, &string.0, string.1);
        self.emit_str_range_imm(first, last);
        true
    }

    /// Emit `string is` as the release compiles it, or nothing where the
    /// command runs it.
    fn try_emit_inline_string_is(&mut self, sargs: &[(String, bool)]) -> bool {
        // Only two shapes can be specialised inline: `CLASS value` and
        // `CLASS -strict value`. Anything else carries an option this path does
        // not model — above all `-failindex var`, which has to *write a
        // variable*. The dispatch that reaches here gates on arity alone, so
        // taking `sargs.last()` as the value and ignoring everything before it
        // would make `string is integer -failindex fi 1.5` compute the correct
        // answer and silently never write `fi` (tclsh writes 1). A 2-word form
        // whose second word is `-strict` is a
        // missing-value arity error, which the generic path reports properly.
        let specialisable = match sargs.len() {
            2 => sargs[1].0 != "-strict",
            3 => sargs[1].0 == "-strict",
            _ => false,
        };
        if !specialisable {
            return false;
        }
        let class_name = &sargs[0].0;
        // Detect -strict flag and value
        let (strict, val_arg) = if sargs.len() == 3 && sargs[1].0 == "-strict" {
            (true, &sargs[2])
        } else {
            (false, &sargs[sargs.len() - 1])
        };

        if let Some(class_id) = str_class_id(class_name) {
            // `STR_CLASS` reports the empty string as a member, so it cannot
            // honour `-strict` (under which the empty string is a non-member
            // for character classes), and it reads the value by character,
            // which the command evaluated directly does not.
            if strict || self.in_direct_command() {
                return false;
            }
            self.emit_native_argument_word(sargs.len(), &val_arg.0, val_arg.1);
            self.emit(Op::STR_CLASS, vec![Operand::Imm(i32::from(class_id))]);
        } else if class_name == "integer" {
            self.emit_native_argument_word(sargs.len(), &val_arg.0, val_arg.1);
            if strict {
                self.emit(Op::NUMERIC_TYPE, vec![]);
                self.emit(Op::DUP, vec![]);
                let end_lbl = self.fresh_label("si_end");
                self.emit(Op::JUMP_FALSE1, vec![Operand::Label(end_lbl.clone())]);
                self.push_lit("3");
                self.emit(Op::LE, vec![]);
                self.place_label(&end_lbl);
            } else {
                self.emit(Op::DUP, vec![]);
                self.emit(Op::NUMERIC_TYPE, vec![]);
                self.emit(Op::DUP, vec![]);
                let has_type = self.fresh_label("si_has_type");
                self.emit(Op::JUMP_TRUE1, vec![Operand::Label(has_type.clone())]);
                self.emit(Op::POP, vec![]);
                self.push_lit("");
                self.emit(Op::STR_EQ, vec![]);
                let end_lbl = self.fresh_label("si_end");
                self.emit(Op::JUMP1, vec![Operand::Label(end_lbl.clone())]);
                self.place_label(&has_type);
                self.emit(Op::REVERSE, vec![Operand::Imm(2)]);
                self.emit(Op::POP, vec![]);
                self.push_lit("3");
                self.emit(Op::LE, vec![]);
                self.place_label(&end_lbl);
            }
        } else if class_name == "double" {
            self.emit_native_argument_word(sargs.len(), &val_arg.0, val_arg.1);
            if strict {
                self.emit(Op::NUMERIC_TYPE, vec![]);
                let true_lbl = self.fresh_label("si_true");
                self.emit(Op::JUMP_TRUE1, vec![Operand::Label(true_lbl.clone())]);
                self.push_lit("0");
                let end_lbl = self.fresh_label("si_end");
                self.emit(Op::JUMP1, vec![Operand::Label(end_lbl.clone())]);
                self.place_label(&true_lbl);
                self.push_lit("1");
                self.place_label(&end_lbl);
            } else {
                self.emit(Op::DUP, vec![]);
                self.push_lit("");
                self.emit(Op::STR_EQ, vec![]);
                let true_lbl = self.fresh_label("si_true");
                self.emit(Op::JUMP_TRUE1, vec![Operand::Label(true_lbl.clone())]);
                self.emit(Op::NUMERIC_TYPE, vec![]);
                let has_type = self.fresh_label("si_has_type");
                self.emit(Op::JUMP_TRUE1, vec![Operand::Label(has_type.clone())]);
                self.push_lit("0");
                let end_lbl = self.fresh_label("si_end");
                self.emit(Op::JUMP1, vec![Operand::Label(end_lbl.clone())]);
                self.place_label(&true_lbl);
                self.emit(Op::POP, vec![]);
                self.place_label(&has_type);
                self.push_lit("1");
                self.place_label(&end_lbl);
            }
        } else if class_name == "boolean" {
            self.emit_native_argument_word(sargs.len(), &val_arg.0, val_arg.1);
            self.emit(Op::TRY_CVT_TO_BOOLEAN, vec![]);
            let true_lbl = self.fresh_label("si_true");
            self.emit(Op::JUMP_TRUE1, vec![Operand::Label(true_lbl.clone())]);
            self.push_lit("");
            self.emit(Op::STR_EQ, vec![]);
            let end_lbl = self.fresh_label("si_end");
            self.emit(Op::JUMP1, vec![Operand::Label(end_lbl.clone())]);
            self.place_label(&true_lbl);
            self.emit(Op::POP, vec![]);
            self.push_lit("1");
            self.place_label(&end_lbl);
        } else {
            return false;
        }
        true
    }

    /// `string is CLASS ?args…?` via the generic command path, **keeping** the
    /// `is` subcommand word. Used when a `string is` form cannot be specialised
    /// inline (unknown class, `-strict` char class, extra options). The previous
    /// fallback dropped `is` (it prefixed `string` then sliced it back off),
    /// producing the invalid `string CLASS …`.
    pub(super) fn emit_string_is_generic(
        &mut self,
        entered_command: &str,
        entered_binding: &tcl_runtime_api::CommandBindingIdentity,
        sargs: &[(String, bool)],
    ) {
        self.used_inline_cmd_subst = false;
        let mut all_args = vec![("is".to_owned(), false)];
        all_args.extend_from_slice(sargs);
        self.emit_generic_cmd_subst_with_binding(
            entered_command,
            &all_args,
            Some(entered_binding.clone()),
        );
    }

    /// `regexp ?switches? pattern string` in a value, compiled where the
    /// release compiles it ([`Self::try_emit_compiled_regexp`]) and the
    /// command's otherwise.
    pub(super) fn emit_inline_regexp(&mut self, args: &[(String, bool)]) {
        if !self.try_emit_compiled_regexp(args) {
            self.used_inline_cmd_subst = false;
            self.emit_generic_cmd_subst("regexp", args);
        }
    }

    /// `regexp ?switches? pattern string` as `TclCompileRegexpCmd` compiles
    /// it: only `-nocase` (or a prefix of it of two characters or more) and
    /// `--` may precede the two words, each a literal; a literal pattern that
    /// is no switch, unless `--` came first, and that `TclReToGlob` converts
    /// becomes `INST_STR_EQ` (an exact comparison, without `-nocase`) or
    /// `INST_STR_MATCH` over the glob, and any other pattern `INST_REGEXP`.
    /// Emits nothing where the command runs it, as it does every command the
    /// release evaluates directly. The pattern and subject keep their
    /// original argument indices past the consumed switches.
    pub(super) fn try_emit_compiled_regexp(&mut self, args: &[(String, bool)]) -> bool {
        const TCL_REG_ADVANCED: i32 = 3;
        const TCL_REG_NOCASE: i32 = 0o10;
        if self.in_direct_command() {
            return false;
        }
        let mut nocase = false;
        let mut saw_last = false;
        let mut i = 0;
        while i + 2 < args.len() {
            let word = &args[i];
            if !is_literal_word(word) {
                return false;
            }
            if word.0 == "--" {
                saw_last = true;
                i += 1;
                break;
            }
            if !(word.0.len() > 1 && "-nocase".starts_with(word.0.as_str())) {
                return false;
            }
            nocase = true;
            i += 1;
        }
        let [pattern, subject] = &args[i..] else {
            return false;
        };
        let glob = if is_literal_word(pattern) {
            if pattern.0.starts_with('-') && !saw_last {
                return false;
            }
            tcl_syntax::glob::re_to_glob(pattern.0.as_bytes())
        } else {
            None
        };
        self.used_inline_cmd_subst = true;
        if let Some((glob, exact)) = glob {
            self.push_lit_bytes_exact(&glob);
            self.emit_native_argument_word(i + 1, &subject.0, subject.1);
            if exact && !nocase {
                self.emit(Op::STR_EQ, vec![]);
            } else {
                self.emit(Op::STR_MATCH, vec![Operand::Imm(i32::from(nocase))]);
            }
        } else {
            self.emit_native_argument_word(i, &pattern.0, pattern.1);
            self.emit_native_argument_word(i + 1, &subject.0, subject.1);
            let cflags = TCL_REG_ADVANCED | if nocase { TCL_REG_NOCASE } else { 0 };
            self.emit(Op::REGEXP, vec![Operand::Imm(cflags)]);
        }
        true
    }

    /// Emit a `string` subcommand whose own hook says how the release
    /// compiles it, or nothing where the command runs it.
    pub(super) fn try_emit_string_subcommand(
        &mut self,
        hook: InlineCodegenHookId,
        args: &[(String, bool)],
    ) -> bool {
        match hook {
            InlineCodegenHookId::StringIs if args.len() >= 3 => {
                self.used_inline_cmd_subst = true;
                self.try_emit_inline_string_is(&args[1..])
            }
            InlineCodegenHookId::StringMatch => self.try_emit_string_match(args),
            InlineCodegenHookId::StringReplace => self.try_emit_string_replace(args),
            InlineCodegenHookId::StringTrim => self.try_emit_string_trim(args, Op::STR_TRIM),
            InlineCodegenHookId::StringTrimLeft => {
                self.try_emit_string_trim(args, Op::STR_TRIM_LEFT)
            }
            InlineCodegenHookId::StringTrimRight => {
                self.try_emit_string_trim(args, Op::STR_TRIM_RIGHT)
            }
            _ => false,
        }
    }

    /// `string match ?-nocase? pattern string` as `TclCompileStringMatchCmd`
    /// compiles it: `INST_STR_EQ` for a literal pattern with no glob
    /// character (without `-nocase`), else `INST_STR_MATCH`. A first word
    /// that is no literal prefix of `-nocase`, or another count, is the
    /// command's, as is every command the release evaluates directly.
    fn try_emit_string_match(&mut self, args: &[(String, bool)]) -> bool {
        if self.in_direct_command() {
            return false;
        }
        let (nocase, at) = match args {
            [_, _, _] => (false, 1),
            [_, option, _, _]
                if is_literal_word(option)
                    && option.0.len() > 1
                    && "-nocase".starts_with(option.0.as_str()) =>
            {
                (true, 2)
            }
            _ => return false,
        };
        let (pattern, string) = (&args[at], &args[at + 1]);
        let exact =
            !nocase && is_literal_word(pattern) && !pattern.0.contains(['*', '?', '[', '\\']);
        self.used_inline_cmd_subst = true;
        self.emit_native_argument_word(at, &pattern.0, pattern.1);
        self.emit_native_argument_word(at + 1, &string.0, string.1);
        if exact {
            self.emit(Op::STR_EQ, vec![]);
        } else {
            self.emit(Op::STR_MATCH, vec![Operand::Imm(i32::from(nocase))]);
        }
        true
    }

    /// `string trim|trimleft|trimright string ?chars?` as
    /// `TclCompileStringTrimCmd` and its one-sided siblings compile it, to
    /// `op` over the string and the characters, or the default set where none
    /// are given. Emits nothing for another count, or where the release
    /// evaluates the command directly.
    fn try_emit_string_trim(&mut self, args: &[(String, bool)], op: Op) -> bool {
        if self.in_direct_command() {
            return false;
        }
        let (string, chars) = match args {
            [_, string] => (string, None),
            [_, string, chars] => (string, Some(chars)),
            _ => return false,
        };
        self.used_inline_cmd_subst = true;
        self.emit_native_argument_word(1, &string.0, string.1);
        match chars {
            Some((chars, braced)) => self.emit_native_argument_word(2, chars, *braced),
            None => self.push_lit_exact(&tcl_cmd_core::string::default_trim_set()),
        }
        self.emit(op, vec![]);
        true
    }
}
