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

//! Command calls: variable references for stores, argument words, `{*}` expansion and the call itself.

use super::super::values::split_array_ref;
use super::super::{CodegenCtx, Op, Operand};
use tcl_compiler::codegen_helpers::has_unescaped_subst;
use tcl_compiler::ir::WordExpr;

impl CodegenCtx<'_> {
    /// Push variable reference for store operations (name/key on stack).
    ///
    /// `name` is a [resolved store name](Self::store_target) — the name word's
    /// *value*, not its source spelling — so every literal half goes out
    /// **verbatim and byte-exact**
    /// ([`push_lit_exact`](Self::push_lit_exact)): the VM must not run word
    /// substitution over a name it has already resolved, or it strips a name's
    /// outer braces (`set {{a}} V` creating `a` instead of `{a}`), reads a
    /// `${…}` inside one, and runs a `[…]` inside one. The
    /// literal bytes are unchanged — only the out-of-band `literal_rep`
    /// differs — so the disassembly is byte-stable.
    ///
    /// `key_is_literal` says whether an array-element key is likewise finished
    /// (a braced word — `set {a($x)} v` keys on the literal `$x` — or one whose
    /// only substitutions were backslash escapes). A key that still substitutes
    /// (`set a($i) v`) keeps the [`push_array_key`](Self::push_array_key) path.
    pub fn push_var_ref(&mut self, name: &str, key_is_literal: bool) {
        let Some((base, elem)) = split_array_ref(name) else {
            self.push_lit_exact(name);
            return;
        };
        // In a proc the base is the LVT slot `store_var` names; only the key
        // goes on the stack.
        if self.command_variable_slot(base.as_bytes()).is_none() {
            self.push_lit_exact(base);
        }
        if key_is_literal {
            self.push_lit_exact(elem);
        } else {
            self.push_array_key(elem);
        }
    }

    /// Emit a command call with `{*}` expansion on marked arguments.
    /// Emit one command word: a braced single-token word (`{…}`) is pushed
    /// verbatim (substitution suppressed — a `proc` body, or a `{*}`-expanded
    /// braced list whose `[…]`/`$…` are data); a word whose only substitution
    /// markers are backslash-escaped is decoded; otherwise it is interpolated.
    /// Shared by the plain ([`emit_call`](Self::emit_call)) and `{*}`-expanded
    /// ([`emit_expanded_call`](Self::emit_expanded_call)) word loops.
    /// Emit the `i`-th argument of the command currently dispatching to a
    /// codegen hook, honouring whether that word was braced (see
    /// [`Self::cmd_arg_braced`]). Hooks that emit a word as a runtime value
    /// (`concat`, `llength`, …) call this instead of
    /// [`Self::emit_value_interpolated`] so a non-braced literal's backslash
    /// escapes are collapsed exactly like the generic per-word path — a braced
    /// word stays verbatim. Falls back to non-braced when no flags are recorded
    /// (hand-built test contexts).
    pub(crate) fn emit_word_arg(&mut self, i: usize, a: &str) {
        let braced = self.cmd_arg_braced.get(i).copied().unwrap_or(false);
        let word = self.original_hook_argument(i).and_then(|original| {
            self.invocation_tokens
                .as_deref()?
                .words()
                .get(original + 1)
                .cloned()
        });
        self.emit_word_from_source(a, braced, word.as_ref());
    }

    pub(in super::super) fn emit_word_from_source(
        &mut self,
        a: &str,
        braced: bool,
        word: Option<&WordExpr>,
    ) {
        let first = self.instructions.len();
        if word.is_some_and(|word| self.try_emit_original_operand_word(a, word)) {
            self.retain_literal_source_value_line(first, word);
            return;
        }
        if braced {
            self.push_lit_verbatim(a);
        } else if a.contains('\\') && !has_unescaped_subst(a) {
            // A non-braced word whose only substitution markers are
            // backslash-escaped (`\{`, `a\{b`, `\ x`, `x\$y`). If decoding it
            // would (re)introduce a `[` / `${` — the word carries an escaped
            // `\[` or `\${` — leave it raw so the runtime `subst_word` decodes
            // it in one left-to-right pass; pre-decoding here would create a
            // bare `[` the VM then mis-reads as a command substitution and
            // double-decodes. Otherwise it is a pure compile-time literal:
            // backslash-substitute it so the escapes don't reach the VM raw.
            if a.contains('[') || a.contains("${") {
                self.push_lit(a);
            } else {
                self.push_decoded_literal(a);
            }
        } else {
            self.emit_value_interpolated_from_word(a, word);
        }
        self.retain_literal_source_value_line(first, word);
    }

    /// Stamp a retained literal value independently of its command boundary.
    /// Captured, transformed and unlocated values retain no invented location.
    pub(in super::super) fn retain_literal_source_value_line(
        &mut self,
        first: usize,
        word: Option<&WordExpr>,
    ) {
        let Some(word @ (WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. })) = word else {
            return;
        };
        let site = word.source();
        if site.provenance != tcl_compiler::ir::Provenance::Source
            || self.source.is_empty()
            || self.source.get(site.span.as_range()).is_none()
        {
            return;
        }
        let line = self.source_line(site.span);
        let Some([instruction]) = self.instructions.get_mut(first..) else {
            return;
        };
        if matches!(instruction.op, Op::PUSH1 | Op::PUSH4) {
            instruction.source_value_line = Some(line);
        }
    }

    /// Emit a `{*}`-expanded command call: push each word (braced words stay
    /// verbatim — see [`emit_word_from_source`](Self::emit_word_from_source)), `EXPAND_STKTOP`-splitting
    /// the ones flagged in `expand_word`, then `INVOKE_EXPANDED`.
    pub fn emit_expanded_call(
        &mut self,
        cmd: &str,
        args: &[String],
        expand_word: &[bool],
        tokens: Option<&tcl_compiler::ir::CommandTokens>,
    ) {
        if self.emit_native_original_preparation_invocation(tokens) {
            self.emit(Op::POP, vec![]);
            return;
        }
        let parts = std::iter::once(cmd)
            .chain(args.iter().map(String::as_str))
            .enumerate()
            .map(|(index, word)| {
                (
                    word.to_owned(),
                    tokens
                        .is_some_and(|tokens| tokens.arg_is_braced_literal(index.wrapping_sub(1))),
                    expand_word.get(index).copied().unwrap_or(false),
                )
            })
            .collect::<Vec<_>>();
        let relayed =
            self.with_invocation_tokens(tokens, |ctx| ctx.try_emit_expanded_native_call(&parts));
        if relayed {
            self.emit(Op::POP, vec![]);
            return;
        }
        let words = std::iter::once(cmd)
            .chain(args.iter().map(String::as_str))
            .enumerate()
            .map(|(index, word)| {
                // A braced single-token word stays verbatim even when
                // expanded: the split operates on its list value without
                // substituting `[…]` / `$…` inside it. This applies equally to
                // the command head (index zero) and to every argument.
                let braced = tokens.is_some_and(|t| {
                    t.argv_kinds.get(index) == Some(&tcl_lexer::TokenType::Str)
                        && t.single_token_word.get(index).copied().unwrap_or(false)
                });
                let expanded = expand_word.get(index).copied().unwrap_or(false);
                (word, braced, expanded)
            });
        self.with_invocation_tokens(tokens, |ctx| {
            ctx.emit_expanded_words(words, &format!("{cmd} (expanded)"));
        });
        self.emit(Op::POP, vec![]);
    }

    /// Emit a regular command call.
    pub fn emit_call(
        &mut self,
        cmd: &str,
        args: &[String],
        tokens: Option<&tcl_compiler::ir::CommandTokens>,
        used_generic_invoke: &mut bool,
    ) {
        self.with_invocation_tokens(tokens, |ctx| {
            ctx.emit_call_inner(cmd, args, tokens, used_generic_invoke);
        });
    }

    fn emit_call_inner(
        &mut self,
        cmd: &str,
        args: &[String],
        tokens: Option<&tcl_compiler::ir::CommandTokens>,
        used_generic_invoke: &mut bool,
    ) {
        if self.emit_native_original_preparation_invocation(tokens) {
            self.emit(Op::POP, vec![]);
            *used_generic_invoke = true;
            return;
        }
        if self.emit_native_procedure_noop(tokens) || self.emit_native_named_invocation(tokens) {
            self.emit(Op::POP, vec![]);
            *used_generic_invoke = true;
            return;
        }
        if self.emit_lexical_loop_jump(cmd, args) {
            return;
        }

        // A braced single-token word (`{…}`, lexed as `TokenType::Str`) is a
        // verbatim literal — e.g. a `proc` body — so it is pushed as-is and its
        // backslashes are NOT collapsed; a non-braced word's escapes are. `argv[0]`
        // is the command word, so arg `i` maps to `argv_kinds[i + 1]`. Computed
        // once and stashed so a codegen hook can collapse its list args the same
        // way the generic per-word path below does (concat-1.4, llength-2.3).
        let braced_flags: Vec<bool> = (0..args.len())
            .map(|i| {
                tokens.is_some_and(|t| {
                    t.argv_kinds.get(i + 1) == Some(&tcl_lexer::TokenType::Str)
                        && t.single_token_word.get(i + 1).copied().unwrap_or(false)
                })
            })
            .collect();

        // Try a registered per-command codegen hook before the
        // generic invoke fallback.
        self.cmd_arg_braced = braced_flags;
        if super::super::emitter::bytecoded::try_bytecoded_with_tokens(
            self,
            cmd,
            args,
            tokens,
            used_generic_invoke,
        ) {
            self.cmd_arg_braced = Vec::new();
            return;
        }

        // Native compilation can select a token even when this backend uses
        // a generic surrogate. Capture that token before any argument executes.
        let entered_binding = self.entered_command_surrogate(cmd);
        // The command C compiles in its place makes these locals.
        if entered_binding.is_some() {
            self.intern_entered_command_locals(cmd, args);
        }
        let body_start = self.instructions.len();
        let words: Vec<(&str, bool)> = args
            .iter()
            .enumerate()
            .map(|(i, a)| {
                (
                    a.as_str(),
                    self.cmd_arg_braced.get(i).copied().unwrap_or(false),
                )
            })
            .collect();
        let release_compiled = self.release_compiled(cmd, &words);
        let release_binding = self.release_invoke_binding(cmd, &words);
        // The original head owns both its lexical evaluation and cache actions.
        if !self.try_emit_original_command_head(cmd) {
            self.emit_cmd_word(cmd, false);
        }
        for (i, a) in args.iter().enumerate() {
            let braced = self.cmd_arg_braced.get(i).copied().unwrap_or(false);
            let word = tokens.and_then(|tokens| tokens.words().get(i + 1));
            self.emit_word_from_source(a, braced, word);
        }
        self.cmd_arg_braced = Vec::new();
        let arg_count = i32::try_from(1 + args.len())
            .expect("invoke argument count fits in i32 (bytecode limit)");
        let op = if arg_count < 256 {
            Op::INVOKE_STK1
        } else {
            Op::INVOKE_STK4
        };
        let invoke = self.emit_comment(op, vec![Operand::Imm(arg_count)], cmd);
        self.instructions[invoke].release_compiled = release_compiled;
        self.instructions[invoke].release_binding = release_binding;
        self.retain_entered_command(body_start, entered_binding);
        self.emit(Op::POP, vec![]);
        *used_generic_invoke = true;
    }
}
