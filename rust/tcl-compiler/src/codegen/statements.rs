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

//! Statement-level bytecode emission.
//!
//! Extends [`CodegenCtx`] with methods for emitting IR statements
//! (assignments, calls, returns, barriers) with `startCommand`
//! wrapping.

use crate::ir::Statement;

use super::cmd_subst::{has_command_separator, parse_cmd_parts};
use super::helpers::{SubstPart, parse_subst_template};
use super::values::{is_qualified, needs_stk_var_ref, parse_simple_var_ref, split_array_ref};
use super::{CodegenCtx, Op, Operand};

/// Tag used to identify `startCommand` instructions wrapping generic
/// invokes so the peephole pass can selectively remove them.
pub(crate) const SC_GENERIC_TAG: &str = "sc:generic";

/// Whether `s` contains a `$` or `[` that is *not* backslash-escaped, i.e. a
/// real variable / command substitution that must be resolved at runtime.
///
/// A word with only escaped markers (`\$`, `\[`) and other backslash escapes is
/// a pure compile-time literal: it can be backslash-substituted and pushed
/// directly. Without this distinction a word like `x\$y` or list-1.11's
/// `list e\n} f\$}` would keep its backslashes (real Tcl delivers `x$y` /
/// `list e\n} f$}`).
pub(crate) fn has_unescaped_subst(s: &str) -> bool {
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            // A backslash escapes the next byte (`\$`, `\[`, `\\`); skip both.
            b'\\' => i += 2,
            b'$' | b'[' => return true,
            _ => i += 1,
        }
    }
    false
}

/// Tag appended to no-dedup literal comments.
pub(crate) const NO_DEDUP_TAG: &str = " #nodedup";

impl CodegenCtx<'_> {
    /// Emit a statement, wrapping with `startCommand` if needed.
    ///
    /// `count_override` overrides the default count of 1 (e.g. 2 when
    /// a compound command and its init share the same bytecode offset).
    /// `deferred_end_label` uses a caller-provided label instead of
    /// creating and placing one automatically.
    pub fn emit_stmt_with_start_cmd(
        &mut self,
        stmt: &Statement,
        count_override: Option<u32>,
        deferred_end_label: Option<&str>,
    ) {
        // The wrapping startCommand shares the statement's source span.
        self.current_span = Some(stmt.span());
        let count = count_override.unwrap_or(1);
        let emit_sc = self.cmd_index > 0;
        let mut end_label: Option<String> = None;
        let mut sc_idx: Option<usize> = None;

        if emit_sc {
            let label = match deferred_end_label {
                Some(l) => l.to_owned(),
                None => self.fresh_label("cmd_end"),
            };
            sc_idx = Some(self.emit_comment(
                Op::START_CMD,
                vec![
                    Operand::Label(label.clone()),
                    Operand::Imm(i32::try_from(count).expect("startCommand count fits in i32")),
                ],
                "",
            ));
            end_label = Some(label);
        }

        let mut used_generic_invoke = false;
        self.emit_stmt(stmt, &mut used_generic_invoke);

        if used_generic_invoke {
            // Tag this startCommand as wrapping a generic invoke
            if let Some(idx) = sc_idx
                && !self.is_proc
            {
                SC_GENERIC_TAG.clone_into(&mut self.instructions[idx].comment);
            }
        }

        if let Some(ref label) = end_label
            && deferred_end_label.is_none()
        {
            // Place end label before the trailing pop (or at end)
            let pos = if self.instructions.last().is_some_and(|i| i.op == Op::POP) {
                self.instructions.len() - 1
            } else {
                self.instructions.len()
            };
            self.label_positions.insert(label.clone(), pos);
        }

        self.cmd_index += 1;
    }

    /// Emit `Statement::AssignConst` / `AssignValue` / `AssignExpr`
    /// / `Incr` / `ExprEval`.  Extracted from [`Self::emit_stmt`] to
    /// keep the dispatcher under threshold.
    fn emit_assign_or_incr(&mut self, stmt: &Statement) -> bool {
        match stmt {
            Statement::AssignConst {
                name,
                name_braced,
                value,
                ..
            } => {
                if needs_stk_var_ref(name, self.is_proc) {
                    self.push_var_ref(name, *name_braced);
                }
                // A constant value is verbatim: push it as-is so the VM does
                // not run word substitution on any `[…]` / `$` it contains
                // (e.g. `set x {a [b] $c}`).
                self.push_lit_verbatim(value);
                self.store_var(name);
                self.emit(Op::POP, vec![]);
                true
            }
            Statement::AssignValue {
                name,
                name_braced,
                value,
                value_needs_backsubst,
                ..
            } => {
                let value =
                    if *value_needs_backsubst && !value.contains('[') && !value.contains("${") {
                        tcl_lexer::backslash_subst_in(value, self.escapes).into_owned()
                    } else {
                        // A value carrying a `[` or `${` (an escaped `\[` / `\${` or
                        // a real substitution) is left raw: the runtime `subst_word`
                        // does backslash decoding plus command / variable
                        // substitution in a single left-to-right pass. Pre-decoding
                        // here would turn `\[` / `\${` into a bare `[` / `${` that
                        // `subst_word` then mis-reads as a substitution start, and
                        // would double-decode the escapes (e.g. `"x\[y z\\]"` →
                        // `x[y z]` instead of `x[y z\]`, and `"\${x}"` → the value of
                        // `x` instead of the literal `${x}`).
                        value.clone()
                    };
                let inline = Self::assign_value_inlines_cmd_subst(&value);
                if needs_stk_var_ref(name, self.is_proc) {
                    self.push_var_ref(name, *name_braced);
                } else if inline && self.is_proc && !is_qualified(name) {
                    // Pre-intern the target so it gets a lower LVT slot than any
                    // variable introduced inside the substitution (catch result
                    // var, etc.) — matches tclsh slot order.
                    self.lvt.intern(name);
                }
                if inline {
                    self.emit_inline_cmd_subst(&value);
                } else {
                    self.emit_value_interpolated(&value);
                }
                self.store_var(name);
                self.emit(Op::POP, vec![]);
                true
            }
            Statement::AssignExpr {
                name,
                name_braced,
                expr,
                ..
            } => {
                if needs_stk_var_ref(name, self.is_proc) {
                    self.push_var_ref(name, *name_braced);
                }
                let inner_end = self.fresh_label("cmd_end");
                self.emit_comment(
                    Op::START_CMD,
                    vec![Operand::Label(inner_end.clone()), Operand::Imm(1)],
                    "",
                );
                let guaranteed_numeric = self.emit_expr(expr);
                if !guaranteed_numeric {
                    self.emit(Op::TRY_CVT_TO_NUMERIC, vec![]);
                }
                self.place_label(&inner_end);
                self.store_var(name);
                self.emit(Op::POP, vec![]);
                true
            }
            Statement::Incr { name, amount, .. } => {
                self.emit_incr(name, amount.as_deref());
                self.emit(Op::POP, vec![]);
                true
            }
            Statement::ExprEval { expr, .. } => {
                let guaranteed_numeric = self.emit_expr(expr);
                if !guaranteed_numeric {
                    self.emit(Op::TRY_CVT_TO_NUMERIC, vec![]);
                }
                self.emit(Op::POP, vec![]);
                true
            }
            _ => false,
        }
    }

    /// Emit `Statement::Call` — handles the CFG placeholder names
    /// (`<cond>` / `<empty_clause>`), `{*}` expansion, and the
    /// generic call path.
    fn emit_call_stmt(
        &mut self,
        command: &str,
        args: &[String],
        tokens: Option<&crate::ir::CommandTokens>,
        used_generic_invoke: &mut bool,
    ) {
        if command == "<cond>" {
            return;
        }
        // `<upvar-invalidate>` is a synthetic marker the CFG builder prepends to
        // carry caller-side / embedded-substitution `defs` so the optimiser
        // doesn't propagate a stale value past a mutation. It is not a real
        // command — emit nothing (it must never reach the VM, which would reject
        // it as an invalid command name).
        if command == "<upvar-invalidate>" {
            return;
        }
        if command == "<empty_clause>" {
            self.literals.intern("");
            self.emit(Op::NOP, vec![]);
            self.emit(Op::NOP, vec![]);
            self.emit(Op::NOP, vec![]);
            return;
        }
        let has_expand = tokens
            .and_then(|t| t.expand_word.as_ref())
            .is_some_and(|ew| ew.iter().any(|&e| e));
        if has_expand {
            let ew = tokens.and_then(|t| t.expand_word.as_ref()).unwrap();
            self.emit_expanded_call(command, args, ew, tokens);
            *used_generic_invoke = true;
        } else {
            self.emit_call(command, args, tokens, used_generic_invoke);
        }
    }

    /// Dispatch a statement to the appropriate emission handler.
    pub fn emit_stmt(&mut self, stmt: &Statement, used_generic_invoke: &mut bool) {
        // Stamp every instruction this statement lowers to with its source
        // span so the explorer can map each op back to source.
        self.current_span = Some(stmt.span());
        if self.emit_assign_or_incr(stmt) {
            return;
        }
        match stmt {
            Statement::Call {
                command,
                args,
                tokens,
                ..
            } => self.emit_call_stmt(command, args, tokens.as_ref(), used_generic_invoke),

            Statement::Barrier {
                command,
                args,
                reason,
                tokens,
                ..
            } => {
                if command.is_empty() {
                    self.emit_comment(Op::NOP, vec![], &format!("barrier: {reason}"));
                } else {
                    self.emit_call_stmt(command, args, tokens.as_ref(), used_generic_invoke);
                }
            }

            // Opaque (glob/regexp/fall-through) switch: the CFG builder keeps
            // it as a single statement rather than expanding arm blocks, so
            // emit a generic `switch` invoke — tclsh 9.0's un-compiled approach
            // for these modes.
            Statement::Switch {
                raw_args,
                patterns_braced,
                ..
            } => {
                // The single-braced arm form `switch … { pat body … }` passes
                // the whole arm block as one braced-literal word: its patterns
                // and bodies are data, so a `[…]`/`$…` inside (e.g. a `-regexp`
                // character class `{[0-9]+}`) must not be substituted. Mark that
                // trailing word `Str` so it is pushed verbatim; the leading
                // option/subject words still interpolate (the subject may be
                // `$s`). The multi-word form keeps the all-interpolated path.
                if *patterns_braced && !raw_args.is_empty() {
                    let n = raw_args.len();
                    let mut kinds = vec![tcl_lexer::TokenType::Esc; n + 1];
                    kinds[n] = tcl_lexer::TokenType::Str;
                    let toks = crate::ir::CommandTokens::from_lossy_parts(
                        Vec::new(),
                        Vec::new(),
                        kinds,
                        vec![true; n + 1],
                        Vec::new(),
                        None,
                    );
                    self.emit_call_stmt("switch", raw_args, Some(&toks), used_generic_invoke);
                } else {
                    self.emit_call_stmt("switch", raw_args, None, used_generic_invoke);
                }
            }

            Statement::Return { value, .. } => {
                let val = value.as_deref().unwrap_or("");
                self.emit_value_interpolated(val);
                // A top-level `RETURN_IMM` pops the result *and* an options dict;
                // push the empty options so the pair sits at the top of the
                // stack regardless of any leftover from a preceding statement
                // (e.g. an `if` with no `else`). Proc bodies fold this to a
                // `DONE`, which returns the single top value, so no options push.
                if !self.is_proc {
                    self.push_lit("");
                }
                // A plain `return` is `(code 0, level 1)`: C's
                // `TclMergeReturnOptions` defaults `-level` to 1, and
                // `CompileReturnInternal` puts the merged pair on the operands
                // (`tclCompCmdsGR.c:2410`). `(0, 0)` would mean "push the result
                // and fall through", not "return". Proc bodies re-key on the same
                // pair in `fold_tail_return_to_done`.
                self.emit(Op::RETURN_IMM, vec![Operand::Imm(0), Operand::Imm(1)]);
            }

            // Static-body uplevel: `UpFrame` is the structured IR form the
            // analysers consume (interproc purity, code-sinking, var-escape, the
            // inline_uplevel pass). The whole-callee inline_uplevel splice
            // rewrites passthrough call-sites into `Statement::Block` (emitted
            // inline below); any `UpFrame` that reaches codegen unspliced is run
            // through the runtime `uplevel` builtin by re-emitting the original
            // `uplevel <level> {body}` invoke from its preserved command tokens.
            // This is byte-identical to the pre-static-lowering barrier path and
            // keeps correct frame semantics without needing frame-shift opcodes.
            Statement::UpFrame { tokens, .. } => {
                if let Some(ct) = tokens
                    && ct.argv_texts.len() >= 2
                {
                    let command = ct.argv_texts[0].clone();
                    let args: Vec<String> = ct.argv_texts[1..].to_vec();
                    self.emit_call_stmt(&command, &args, Some(ct), used_generic_invoke);
                } else {
                    // No preserved tokens (synthetic UpFrame) — nothing to
                    // dispatch; mark it rather than silently dropping a body.
                    self.emit_comment(Op::NOP, vec![], "unhandled: Statement::UpFrame (no tokens)");
                }
            }

            // Inline block: the inline_uplevel pass replaces
            // matching call-sites with ``Statement::Block { body, .. }``
            // — emit the body's statements as if they were inline at
            // this point. The block has no scope of its own at the
            // bytecode level (the body already ran in the caller's
            // frame in the original ``uplevel`` semantics).
            Statement::Block { body, .. } => {
                for inner in &body.statements {
                    self.emit_stmt(inner, used_generic_invoke);
                }
            }

            // Structured control flow is handled by the main emitter loop;
            // if we reach these here, emit a NOP placeholder.
            _ => {
                self.emit_comment(Op::NOP, vec![], "unhandled statement");
            }
        }
    }

    /// Emit a simple value — handles `${var}` references and literals.
    ///
    /// This is a simplified value emitter that handles the common cases.
    /// Full interpolation with command substitution requires the main
    /// emitter pipeline.
    pub fn emit_value_interpolated(&mut self, value: &str) {
        // Braced scalar marker: $={name} → push + loadStk
        if let Some(name) = super::values::parse_braced_scalar_ref(value) {
            self.push_lit(name);
            self.emit(Op::LOAD_STK, vec![]);
            return;
        }
        // Variable reference: ${var} → load
        if let Some(var_name) = parse_simple_var_ref(value, self.braced_var) {
            self.load_var(var_name);
            return;
        }
        // Constant-fold [list arg1 arg2 ...]
        if let Some(folded) = super::helpers::fold_list_cmd(value) {
            self.push_lit_no_dedup_verbatim(&folded);
            return;
        }
        // Inline [list {*}$a {*}$b] → load a, load b, listConcat.
        if self.try_list_expand_concat(value) {
            return;
        }
        // Inline [list arg ... [break] ...] / [list arg ... [continue] ...].
        if self.try_inline_list_with_break_continue(value) {
            return;
        }
        // Constant-fold [format "..." arg ...].
        if let Some(folded) = super::helpers::try_format_fold(value) {
            self.push_lit_no_dedup_verbatim(&folded);
            return;
        }
        // Constant-fold [dict create k v ...] — gated on the emulated release
        // actually having `dict`; see the twin site in `cmd_subst.rs` (#1427).
        if self.registry.has_command_in_this_dialect("dict")
            && let Some(folded) = super::helpers::fold_dict_create_cmd(value)
        {
            self.push_lit(&folded);
            self.emit(Op::DUP, vec![]);
            self.emit(Op::VERIFY_DICT, vec![]);
            return;
        }
        // Whole-word variable-index array element `$arr($idx)`: route through
        // `load_var` (base + substituted key) rather than the literal/runtime
        // subst path, which cannot resolve the bare `$idx` inside the index.
        if value.starts_with('$')
            && value.ends_with(')')
            && let Some(parts) = parse_subst_template(value, self.escapes, self.braced_var)
            && parts.len() == 1
            && let SubstPart::Var(name) = &parts[0]
            && split_array_ref(name).is_some()
        {
            self.load_var(name);
            return;
        }
        // Interpolated string containing an array-element variable
        // (`"…$arr($idx)…"`): the runtime `subst_word` fallback can substitute a
        // normalised `${name}` but not an array element with a substituted
        // index, so decompose the word at compile time. Plain `$scalar`
        // interpolation keeps its existing (deferred-literal) lowering.
        // Also decompose a `{…}`-wrapped value that contains a command
        // substitution (e.g. `"{[list …]}"`): the braces are literal word
        // content and the `[…]` must run, but pushing the value raw would let
        // the runtime `subst_word` mistake it for a braced literal and strip the
        // braces. A genuine braced argument never reaches here (it is emitted by
        // the braced-word path), so decomposing is safe.
        if (value.contains('$') || value.contains('['))
            && let Some(parts) = parse_subst_template(value, self.escapes, self.braced_var)
            && parts.len() > 1
            && (parts
                .iter()
                .any(|p| matches!(p, SubstPart::Var(n) if split_array_ref(n).is_some()))
                || (value.starts_with('{')
                    && value.ends_with('}')
                    && parts.iter().any(|p| matches!(p, SubstPart::Cmd(_)))))
        {
            for part in &parts {
                match part {
                    SubstPart::Lit(text) => self.push_lit(text),
                    SubstPart::Cmd(cmd_text) => self.emit_inline_cmd_subst(cmd_text),
                    SubstPart::Scalar(name) => {
                        self.push_lit(name);
                        self.emit(Op::LOAD_STK, vec![]);
                    }
                    SubstPart::Var(name) => self.load_var(name),
                }
            }
            self.emit(
                Op::STR_CONCAT1,
                vec![Operand::Imm(
                    i32::try_from(parts.len()).expect("part count fits in i32"),
                )],
            );
            return;
        }
        // A value with backslash escapes and a *bare* `$` (e.g. `f\$}` / the
        // `$}` in `list e\n} f\$}`) but no real `${…}` reference or `[…]`
        // command substitution is a pure literal — the bare `$` stays literal
        // because the codegen normalises genuine variables to `${name}`. Decode
        // its escapes once at compile time. Without this the value is pushed raw
        // and the runtime `subst_word` (which only decodes words carrying a real
        // `${`/`[`) leaves the `\\` escapes intact. Values the assign path
        // already backslash-decoded never reach here with a `$`, so there is no
        // double decode.
        if value.contains('\\')
            && value.contains('$')
            && !value.contains("${")
            && !value.contains('[')
        {
            self.push_lit(&tcl_lexer::backslash_subst_in(value, self.escapes));
            return;
        }
        // A whole-word command substitution compiles inline (on the explicit
        // stack) rather than via the runtime `subst_word` fallback, so a
        // `[yield]`/`[cmd]` inside it stays yieldable in a coroutine.
        if self.try_emit_whole_cmd_subst(value) {
            return;
        }
        // Default: push as literal
        self.push_lit(value);
    }

    /// Whether a `set x VALUE` value should inline-compile its command
    /// substitution (`set x [cmd arg …]`) instead of pushing the raw `[…]` text
    /// and leaning on the runtime `subst_word` fallback. The conditions:
    /// a balanced `[…]` with inner whitespace (a bare
    /// `[foo]` with no args stays a literal), no `{*}` expansion, and excluding
    /// the commands the constant-fold / special branches in
    /// [`Self::emit_value_interpolated`] own (`list` / `dict create` / `set` /
    /// `format`). Only used in the assignment value position — command arguments
    /// keep the literal + `subst_word` path, which the inline command-parser
    /// cannot match on escaped brackets.
    fn assign_value_inlines_cmd_subst(value: &str) -> bool {
        value.starts_with('[')
            && value.ends_with(']')
            && value.len() > 2
            && value[1..value.len() - 1].contains(' ')
            && !value.starts_with("[list ")
            && !value.starts_with("[dict create ")
            && !value.starts_with("[set ")
            && !value.starts_with("[format ")
    }

    /// If `value` is a *single whole-word command substitution* (`[cmd …]` and
    /// nothing else) of a shape the command-parser handles faithfully, compile
    /// it as a generic `INVOKE` on the explicit activation stack and return
    /// `true`; otherwise return `false` and leave the caller to fall back to its
    /// literal / `subst_word` path.
    ///
    /// C Tcl never runs a whole-word substitution through a recursive
    /// `Tcl_EvalObjEx`; it compiles it to `INST_INVOKE`. Matching that keeps a
    /// `[yield]`/`[cmd]` inside the substitution **yieldable** in a coroutine —
    /// the runtime `subst_word` fallback re-enters the evaluator on the *host*
    /// stack, a boundary a `yield` cannot cross (`cannot yield: C stack busy`).
    ///
    /// The generic `INVOKE` runs the real command (`string`, a user proc,
    /// `yield`, …) exactly as `subst_word` would, only on the explicit stack —
    /// so the change is behaviour-preserving apart from yieldability. It is used
    /// in preference to the specialised inline forms (`STR_LEN`, `STR_CLASS`, …)
    /// deliberately: those are pure builtins that never yield, so they need no
    /// inlining here, and running the real command sidesteps their opcode edge
    /// cases. The folded forms (`[list …]` / `[format …]` / `[dict create …]`)
    /// and the specialised assignment path still own their contexts; this only
    /// covers a whole-word substitution that would otherwise reach `push_lit`.
    ///
    /// Conservative on shape: escaped brackets (`\[` / `\]`, which the parser
    /// cannot match — see [`Self::assign_value_inlines_cmd_subst`]), `{*}`
    /// expansion, and multi-command bodies (a `;`/newline separator) keep the
    /// runtime path. Those never carry the coroutine resume-value idiom, so
    /// nothing yieldable is lost.
    pub(crate) fn try_emit_whole_cmd_subst(&mut self, value: &str) -> bool {
        if value.contains("\\[")
            || value.contains("\\]")
            || value.contains("{*}")
            || has_command_separator(value)
        {
            return false;
        }
        if !matches!(
            parse_subst_template(value, self.escapes, self.braced_var).as_deref(),
            Some([SubstPart::Cmd(_)])
        ) {
            return false;
        }
        let parts = parse_cmd_parts(value);
        let Some((head, _)) = parts.split_first() else {
            return false;
        };
        self.emit_generic_cmd_subst(&head.0, &parts[1..]);
        true
    }

    /// Push variable reference for store operations (name/key on stack).
    ///
    /// When `name_braced` is `true` the target word was a brace-string
    /// literal (`set {a($x)} v`); Tcl braces suppress substitution, so an
    /// array-element key is pushed LITERALLY (`$x` stays `$x`) instead of
    /// being substituted. A scalar target is unaffected.
    pub fn push_var_ref(&mut self, name: &str, name_braced: bool) {
        if let Some((base, elem)) = split_array_ref(name) {
            if self.is_proc && !is_qualified(name) {
                if name_braced {
                    self.push_lit(elem);
                } else {
                    self.push_array_key(elem);
                }
            } else {
                self.push_lit(base);
                if name_braced {
                    self.push_lit(elem);
                } else {
                    self.push_array_key(elem);
                }
            }
        } else {
            self.push_lit(name);
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
        self.emit_word(a, braced);
    }

    fn emit_word(&mut self, a: &str, braced: bool) {
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
                self.push_lit(&tcl_lexer::backslash_subst_in(a, self.escapes));
            }
        } else {
            self.emit_value_interpolated(a);
        }
    }

    /// Emit a `{*}`-expanded command call: push each word (braced words stay
    /// verbatim — see [`emit_word`](Self::emit_word)), `EXPAND_STKTOP`-splitting
    /// the ones flagged in `expand_word`, then `INVOKE_EXPANDED`.
    pub fn emit_expanded_call(
        &mut self,
        cmd: &str,
        args: &[String],
        expand_word: &[bool],
        tokens: Option<&crate::ir::CommandTokens>,
    ) {
        self.emit_comment(Op::EXPAND_START, vec![], &format!("{cmd} (expanded)"));
        // Build full word list: [cmd, *args]
        self.emit_value_interpolated(cmd);
        let mut word_count: u32 = 1;
        for (i, arg) in args.iter().enumerate() {
            // A braced single-token word stays verbatim even when expanded: the
            // `{*}` splits its *list* elements without substituting them, so a
            // `[…]`/`$…` inside `{*}{… [x] …}` is literal data (matches C, which
            // expands the parsed list elements, not a re-substituted string).
            let braced = tokens.is_some_and(|t| {
                t.argv_kinds.get(i + 1) == Some(&tcl_lexer::TokenType::Str)
                    && t.single_token_word.get(i + 1).copied().unwrap_or(false)
            });
            self.emit_word(arg, braced);
            word_count += 1;
            // expand_word[0] is the command itself, args start at [1]
            if expand_word.get(i + 1).copied().unwrap_or(false) {
                self.emit(
                    Op::EXPAND_STKTOP,
                    vec![Operand::Imm(
                        i32::try_from(word_count).expect("word_count fits in i32"),
                    )],
                );
            }
        }
        self.emit_comment(Op::INVOKE_EXPANDED, vec![], cmd);
        self.emit(Op::POP, vec![]);
    }

    /// Emit a regular command call.
    pub fn emit_call(
        &mut self,
        cmd: &str,
        args: &[String],
        tokens: Option<&crate::ir::CommandTokens>,
        used_generic_invoke: &mut bool,
    ) {
        // break/continue inside loops: emit jump instead of invokeStk. Only the
        // bare forms take the fast path — `break foo` / `continue foo` carry
        // arguments and must reach the builtin to raise `wrong # args`
        // (for-2.1 / for-3.1).
        if cmd == "continue"
            && args.is_empty()
            && let Some(cont_lbl) = self.continue_target.clone()
        {
            self.emit_comment(Op::JUMP4, vec![Operand::Label(cont_lbl)], "continue");
            return;
        }
        if cmd == "break"
            && args.is_empty()
            && let Some(brk_lbl) = self.break_target.clone()
        {
            self.emit_comment(Op::JUMP4, vec![Operand::Label(brk_lbl)], "break");
            return;
        }

        // Compiled `dict for`/`dict map {k v} DICT {body}` (the ensemble-rewrite
        // barriers `::tcl::dict::for`/`::tcl::dict::map` cfg_builder produced):
        // emit C Tcl's inline dict-iteration bytecode when the shape is
        // compilable, else fall through to the runtime invoke.
        if args.len() == 3 {
            let inlined = match cmd {
                "::tcl::dict::for" => self.emit_dict_for(&args[0], &args[1], &args[2]),
                "::tcl::dict::map" => self.emit_dict_map(&args[0], &args[1], &args[2]),
                _ => false,
            };
            if inlined {
                *used_generic_invoke = true;
                return;
            }
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
        if super::emitter::bytecoded::try_bytecoded(self, cmd, args, used_generic_invoke) {
            self.cmd_arg_braced = Vec::new();
            return;
        }

        // The command name may itself be a substitution (`$Verify($opt) …`,
        // `[lookup] …`); emit it through the per-word path. A plain literal name
        // still lowers to a single `push`, so normal calls are unchanged.
        self.emit_cmd_word(cmd, false);
        for (i, a) in args.iter().enumerate() {
            let braced = self.cmd_arg_braced.get(i).copied().unwrap_or(false);
            self.emit_word(a, braced);
        }
        self.cmd_arg_braced = Vec::new();
        let arg_count = i32::try_from(1 + args.len())
            .expect("invoke argument count fits in i32 (bytecode limit)");
        let op = if arg_count < 256 {
            Op::INVOKE_STK1
        } else {
            Op::INVOKE_STK4
        };
        self.emit_comment(op, vec![Operand::Imm(arg_count)], cmd);
        self.emit(Op::POP, vec![]);
        *used_generic_invoke = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::CodegenCtx;
    use crate::expr_ast::ExprNode;
    use tcl_lexer::Span;
    use tcl_registry::CommandRegistry;

    fn opcodes(ctx: &CodegenCtx) -> Vec<Op> {
        ctx.instructions.iter().map(|i| i.op).collect()
    }

    fn sp() -> Span {
        Span::new(0, 0)
    }

    #[test]
    fn emit_assign_const_proc() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let stmt = Statement::AssignConst {
            span: sp(),
            name: "x".into(),
            value: "42".into(),
            name_braced: false,
            value_span: None,
        };
        let mut ugi = false;
        ctx.emit_stmt(&stmt, &mut ugi);
        assert!(!ugi);
        assert_eq!(opcodes(&ctx), vec![Op::PUSH1, Op::STORE_SCALAR1, Op::POP]);
    }

    #[test]
    fn emit_assign_const_toplevel() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let stmt = Statement::AssignConst {
            span: sp(),
            name: "x".into(),
            value: "42".into(),
            name_braced: false,
            value_span: None,
        };
        let mut ugi = false;
        ctx.emit_stmt(&stmt, &mut ugi);
        assert_eq!(
            opcodes(&ctx),
            vec![Op::PUSH1, Op::PUSH1, Op::STORE_STK, Op::POP]
        );
    }

    #[test]
    fn emit_incr_stmt() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &["x"], &registry);
        let stmt = Statement::Incr {
            span: sp(),
            name: "x".into(),
            name_braced: false,
            amount: None,
            safe_on_uninit: false,
        };
        let mut ugi = false;
        ctx.emit_stmt(&stmt, &mut ugi);
        assert_eq!(opcodes(&ctx), vec![Op::INCR_SCALAR1_IMM, Op::POP]);
    }

    #[test]
    fn emit_call_generic() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let stmt = Statement::Call {
            span: sp(),
            command: "puts".into(),
            canonical_command: None,
            args: vec!["hello".into()],
            defs: vec![],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        };
        let mut ugi = false;
        ctx.emit_stmt(&stmt, &mut ugi);
        assert!(ugi);
        assert!(opcodes(&ctx).contains(&Op::INVOKE_STK1));
    }

    #[test]
    fn emit_call_break_in_loop() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.break_target = Some("loop_end_0".into());
        let stmt = Statement::Call {
            span: sp(),
            command: "break".into(),
            canonical_command: None,
            args: vec![],
            defs: vec![],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        };
        let mut ugi = false;
        ctx.emit_stmt(&stmt, &mut ugi);
        assert!(!ugi);
        assert_eq!(opcodes(&ctx), vec![Op::JUMP4]);
    }

    #[test]
    fn emit_return_empty() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let stmt = Statement::Return {
            span: sp(),
            value: None,
            expr: None,
            braced: false,
        };
        let mut ugi = false;
        ctx.emit_stmt(&stmt, &mut ugi);
        assert!(opcodes(&ctx).contains(&Op::RETURN_IMM));
    }

    #[test]
    fn emit_stmt_with_start_cmd_numbering() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let stmt = Statement::AssignConst {
            span: sp(),
            name: "x".into(),
            value: "1".into(),
            name_braced: false,
            value_span: None,
        };
        ctx.emit_stmt_with_start_cmd(&stmt, None, None);
        assert!(!opcodes(&ctx).contains(&Op::START_CMD));
        assert_eq!(ctx.cmd_index, 1);

        let stmt2 = Statement::AssignConst {
            span: sp(),
            name: "y".into(),
            value: "2".into(),
            name_braced: false,
            value_span: None,
        };
        ctx.emit_stmt_with_start_cmd(&stmt2, None, None);
        assert!(opcodes(&ctx).contains(&Op::START_CMD));
        assert_eq!(ctx.cmd_index, 2);
    }

    #[test]
    fn emit_empty_clause() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let stmt = Statement::Call {
            span: sp(),
            command: "<empty_clause>".into(),
            canonical_command: None,
            args: vec![],
            defs: vec![],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        };
        let mut ugi = false;
        ctx.emit_stmt(&stmt, &mut ugi);
        assert_eq!(opcodes(&ctx), vec![Op::NOP, Op::NOP, Op::NOP]);
    }

    #[test]
    fn emit_barrier_with_command() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let stmt = Statement::Barrier {
            span: sp(),
            reason: "test".into(),
            command: "eval".into(),
            canonical_command: None,
            args: vec!["script".into()],
            tokens: None,
        };
        let mut ugi = false;
        ctx.emit_stmt(&stmt, &mut ugi);
        assert!(ugi);
        assert!(opcodes(&ctx).contains(&Op::INVOKE_STK1));
    }

    #[test]
    fn emit_barrier_without_command() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let stmt = Statement::Barrier {
            span: sp(),
            reason: "side-effect".into(),
            command: String::new(),
            canonical_command: None,
            args: vec![],
            tokens: None,
        };
        let mut ugi = false;
        ctx.emit_stmt(&stmt, &mut ugi);
        assert!(!ugi);
        assert_eq!(opcodes(&ctx), vec![Op::NOP]);
    }

    #[test]
    fn emit_assign_expr() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &["x"], &registry);
        let stmt = Statement::AssignExpr {
            span: sp(),
            name: "x".into(),
            name_braced: false,
            expr: ExprNode::Literal {
                text: "42".into(),
                start: 0,
                end: 2,
            },
            expr_base: None,
        };
        let mut ugi = false;
        ctx.emit_stmt(&stmt, &mut ugi);
        assert!(opcodes(&ctx).contains(&Op::START_CMD));
        assert!(opcodes(&ctx).contains(&Op::STORE_SCALAR1));
    }

    #[test]
    fn emit_expr_eval() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let stmt = Statement::ExprEval {
            span: sp(),
            expr: ExprNode::Literal {
                text: "1".into(),
                start: 0,
                end: 1,
            },
            expr_base: None,
        };
        let mut ugi = false;
        ctx.emit_stmt(&stmt, &mut ugi);
        assert!(opcodes(&ctx).contains(&Op::POP));
    }

    #[test]
    fn emit_expanded_call_basic() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        ctx.emit_expanded_call("puts", &["hello".into()], &[false, true], None);
        let ops = opcodes(&ctx);
        assert!(ops.contains(&Op::EXPAND_START));
        assert!(ops.contains(&Op::EXPAND_STKTOP));
        assert!(ops.contains(&Op::INVOKE_EXPANDED));
    }
}
