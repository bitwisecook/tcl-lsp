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

//! The generic invocation of a substituted command: the entered command head, its words and `{*}` expansion.

use super::super::{CodegenCtx, Op, Operand, bytecode_imm};
use tcl_bytecode::EnteredCommandSite;

impl CodegenCtx<'_> {
    /// Literal command whose registry-described bytecode specialisation this
    /// generic value-position surrogate stands in for.
    ///
    /// Ordinary `invokeStk` commands resolve after argument substitution in C
    /// Tcl, so a literal head alone is not enough. The entered-token sidecar is
    /// reserved for an operation selected by the native compiler grammar.
    /// A Rust emission hook alone does not establish native selection timing.
    pub(in super::super) fn entered_command_surrogate(
        &mut self,
        command: &str,
    ) -> Option<tcl_runtime_api::CommandBindingIdentity> {
        use tcl_registry::native_compilation::NativeCompilationSelection;
        if self.plain_command_dispatch {
            return None;
        }
        let (identity, guard) = if let Some(proof) = self
            .invocation_tokens
            .as_deref()
            .and_then(|tokens| tokens.source_binding.as_ref())
        {
            let compiled = proof.admitted_inline_invocation()?;
            if !compiled.target.prepended.is_empty() {
                return None;
            }
            (compiled.target.command.clone(), compiled.guard)
        } else {
            let tokens = self.invocation_tokens.as_deref()?;
            let dialect = tcl_compiler::environment_ingress::authoring_invocation_dialect(
                self.registry,
                self.dialect,
                self.lexer_config(),
            );
            let (facts, selection) = tcl_compiler::registry_invocation::native_compilation_syntax(
                self.registry,
                tokens,
                dialect,
                self.native_compilation,
            )?;
            let NativeCompilationSelection::Inline { guard, .. } = selection else {
                return None;
            };
            (facts.canonical_command, guard)
        };
        let guard = tcl_compiler::registry_invocation::native_command_binding_guard(guard);
        let binding = self
            .command_binding_identity(command, identity)
            .with_guard(guard);
        self.require_command_binding(&binding);
        Some(binding)
    }

    /// Attach one literal head to its own push and carry it through the
    /// fixed-arity generic invoke just emitted. Capturing at the head push is
    /// important: an earlier substitution in the first argument may mutate the
    /// command table before any nested `START_CMD` is reached. Expanded calls
    /// deliberately resolve after expansion, matching Tcl's distinct
    /// command-token timing.
    pub(in super::super) fn retain_entered_command(
        &mut self,
        body_start: usize,
        entered_binding: Option<tcl_runtime_api::CommandBindingIdentity>,
    ) {
        // Tcl's DONT_COMPILE_CMDS_INLINE / execution-trace path deliberately
        // resolves the command after substitution. Plain-dispatch bytecode is
        // that path in this compiler, so it must not carry an entered token.
        if self.plain_command_dispatch {
            return;
        }
        let Some(binding) = entered_binding else {
            return;
        };
        let end = self.fresh_label("entered_cmd_end");
        let Some(head) = self.instructions.get_mut(body_start) else {
            return;
        };
        debug_assert!(matches!(head.op, Op::PUSH1 | Op::PUSH4));
        head.entered_command = Some(EnteredCommandSite {
            binding,
            end: end.clone(),
        });
        self.place_label(&end);
    }

    /// Emit a generic command substitution as `push cmd; <args>; invokeStk`.
    pub fn emit_generic_cmd_subst(&mut self, cmd: &str, args: &[(String, bool)]) {
        let entered_binding = self.entered_command_surrogate(cmd);
        // The command C compiles in its place makes these locals.
        if entered_binding.is_some() {
            let words: Vec<String> = args.iter().map(|(arg, _)| arg.clone()).collect();
            self.intern_entered_command_locals(cmd, &words);
        }
        let original = self.original_hook_operands(cmd, args).to_vec();
        self.emit_generic_cmd_subst_with_binding(cmd, &original, entered_binding);
    }

    /// Emit the generic-invoke body with an entered binding already selected
    /// by the registry hook dispatcher.
    ///
    /// This is distinct from [`Self::emit_generic_cmd_subst`]: a specialised
    /// emitter can discover that its precise argument shape needs the generic
    /// path only after registry resolution. Re-resolving here can substitute a
    /// stock command identity for the source command whose hook selected that
    /// emitter.
    pub(super) fn emit_generic_cmd_subst_with_binding(
        &mut self,
        cmd: &str,
        args: &[(String, bool)],
        entered_binding: Option<tcl_runtime_api::CommandBindingIdentity>,
    ) {
        let body_start = self.instructions.len();
        let words: Vec<(&str, bool)> = args
            .iter()
            .map(|(arg, braced)| (arg.as_str(), *braced))
            .collect();
        let release_compiled = self.release_compiled(cmd, &words);
        let release_binding = self.release_invoke_binding(cmd, &words);
        // The command name itself may be a substitution (`$Verify($opt) ...`,
        // `[lookup] ...`), so it is emitted through the same per-word path as
        // the arguments rather than as a bare literal.
        if !self.try_emit_original_command_head(cmd) {
            self.emit_cmd_word(cmd, false);
        }
        let tokens = self.invocation_tokens.clone();
        for (index, (arg, braced)) in args.iter().enumerate() {
            let word = tokens.as_deref().and_then(|tokens| {
                (tokens.words().len() == args.len() + 1
                    && tokens
                        .argv_texts
                        .get(index + 1)
                        .is_some_and(|text| text == arg))
                .then(|| tokens.words().get(index + 1))
                .flatten()
            });
            if let Some(word) = word {
                self.emit_word_from_source(arg, *braced, Some(word));
            } else {
                self.emit_cmd_word(arg, *braced);
            }
        }
        let arg_count = bytecode_imm(1 + args.len());
        let op = if arg_count < 256 {
            Op::INVOKE_STK1
        } else {
            Op::INVOKE_STK4
        };
        let invoke = self.emit(op, vec![Operand::Imm(arg_count)]);
        self.instructions[invoke].release_compiled = release_compiled;
        self.instructions[invoke].release_binding = release_binding;
        // Resolve the literal head when it is pushed, before any argument word
        // can substitute, and keep it until this generic invoke.
        self.retain_entered_command(body_start, entered_binding);
    }

    /// Emit one word of a generic command invocation (the command name or an
    /// argument): inline command substitutions, load `$`-variables (scalar,
    /// `${name}`, `$=`-braced scalar, or `$arr(idx)` array element), keep braced
    /// words that still hold substitutions wrapped for runtime handling, and
    /// otherwise push the literal (backslash-decoded).
    pub(crate) fn emit_cmd_word(&mut self, word: &str, braced: bool) {
        if !braced && word.starts_with('[') && word.ends_with(']') {
            let end_label = self.fresh_label("cmd_end");
            self.emit_comment(
                Op::START_CMD,
                vec![Operand::Label(end_label.clone()), Operand::Imm(1)],
                "",
            );
            self.emit_inline_cmd_subst(word);
            self.place_label(&end_label);
        } else if !braced && word.starts_with('$') {
            if !self.emit_variable_reference(word) {
                self.emit_value(word, true);
            }
        } else if braced && (word.contains('$') || word.contains('[')) {
            self.push_lit(&format!("{{{word}}}"));
        } else if !braced && (word.contains('$') || word.contains('[')) {
            // Interpolated word with an *embedded* (non-leading) substitution
            // — e.g. an array-element reference `be(a:$a)` used as a command
            // argument, or `x$item`. The leading-`$` whole-reference forms are
            // handled above; here `emit_value` decomposes the word into its
            // literal / `$var` / `[cmd]` parts and concatenates them, so the
            // `$a` inside the key is substituted. Without this the word was
            // pushed raw and a nested `[set be(a:$a)]` read the literal element
            // `be(a:$a)` (set-1.26).
            self.emit_value(word, true);
        } else if !braced && word.contains('\\') {
            // Decoded here, so the result is this word's value — the same arm,
            // and the same rule, as `emit_cmd_subst_arg`'s. Left substituting,
            // the decoded `\{\}` is read back as a braced literal and stripped
            // to nothing: `set w [dict get [dict create k \{\}] k]` then
            // measures 0 where both oracles say 2.
            self.push_decoded_literal(word);
        } else if braced {
            // A braced word is already de-braced here, so its content is the
            // finished value: push it verbatim or the VM's `subst_word` strips
            // a *second* brace layer — `proc p {} { set {{loc}} L ; return [set
            // {{loc}}] }` would read the local `loc` while the store created
            // `{loc}` (tclsh 8.6.14 / 9.0.4 return `L`).
            self.push_lit_verbatim(word);
        } else {
            // Not braced, and every substitution marker was routed above: this
            // word's text is already its value, so it goes out unsubstituted
            // for the same reason the braced arm does — see `push_compiled_word`.
            self.push_lit_exact(word);
        }
    }

    /// Emit the ordered words of one `{*}` invocation.
    ///
    /// The command head and argument tail deliberately share this loop: Tcl's
    /// expansion marker belongs to a *word position*, including position zero,
    /// and every position must finish its substitutions before that value is
    /// split as a list.  Callers supply words as `(text, braced, expanded)`;
    /// [`Self::emit_cmd_word`] remains the single value-emission path for both
    /// statement and command-substitution invocations.
    pub(crate) fn emit_expanded_words<'w>(
        &mut self,
        words: impl IntoIterator<Item = (&'w str, bool, bool)>,
        comment: &str,
    ) {
        self.emit_comment(Op::EXPAND_START, vec![], comment);
        for (index, (word, braced, expanded)) in words.into_iter().enumerate() {
            if index == 0 {
                if !self.try_emit_original_command_head(word) {
                    self.emit_cmd_word(word, braced);
                }
            } else {
                self.emit_native_argument_word(index - 1, word, braced);
            }
            if expanded {
                self.emit(
                    Op::EXPAND_STKTOP,
                    vec![Operand::Imm(
                        i32::try_from(index + 1).expect("word count fits in i32"),
                    )],
                );
            }
        }
        self.emit_comment(Op::INVOKE_EXPANDED, vec![], comment);
    }
}
