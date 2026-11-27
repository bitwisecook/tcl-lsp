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

//! Assignments, increments and expression-result statements: the static store target and the shared `set` storage path.

use super::super::values::split_array_ref;
use super::super::{CodegenCtx, Op, Operand};
use tcl_compiler::codegen_helpers::has_unescaped_subst;
use tcl_compiler::ir::Statement;
use tcl_source::words::WordExpr;

/// A static store's name word, resolved by
/// [`CodegenCtx::store_target`] — see that method for the rule and its oracle.
pub(super) struct StoreTarget {
    /// The variable name the store must use: the name word's *value*, with the
    /// element key still in source form when [`Self::key_is_literal`] is false.
    pub(super) name: String,
    /// Whether the element key (if this names one) is already a finished
    /// literal, so codegen pushes it verbatim instead of substituting it.
    /// True for a braced word and for any word whose only substitutions were
    /// backslash escapes; false while a live `$` / `[` remains in the key.
    key_is_literal: bool,
}

impl CodegenCtx<'_> {
    /// Resolve a *static store*'s name word for codegen.
    ///
    /// Tcl substitutes a command word **whole** and only then scans the result
    /// for the `(`…`)` that makes it an array element. The IR carries the
    /// word's *source spelling*, so codegen owns both halves; it resolves the
    /// two shapes a static store can take:
    ///
    /// * **Fully static** — a braced word (which substitutes nothing) or one
    ///   carrying only backslash escapes. The word's value is known here:
    ///   decode it, and both the name and any element key are finished
    ///   literals that must be pushed *verbatim* so the VM's `subst_word`
    ///   never touches them again.
    /// * **Live key** — the word still holds an unescaped `$` / `[`, which can
    ///   only be inside the element key: `lower_set` keeps a computed *base*
    ///   (`set $x v`, `set a[f] v`) a generic `Call` whose `STORE_STK` operand
    ///   is the substituted word. The split then runs on the source spelling
    ///   and `push_array_key` resolves the key at run time; the escape-only
    ///   base is still decoded.
    ///
    /// Two divergences close here. Not decoding puts the variable in the table
    /// under its source spelling, so a read through one spelling misses a write
    /// through the other; not pushing the resolved name verbatim lets
    /// `subst_word` substitute it a *second* time — stripping a name's outer
    /// braces, reading a `${…}` inside it, and running a `[…]` inside it.
    ///
    /// Identical on tclsh 8.4.20, 8.5.19, 8.6.14, 9.0.4 and 9.1:
    ///
    /// ```text
    /// # the name is the word's value, not its spelling
    /// set "z1\\" A ; set "z2\}" B ; set "z3\ x" C ; set z4\\ D ; set {z5\\} E
    /// set "z6\x41" F ; set "z7\t" G
    /// foreach n [lsort [info vars z*]] { puts "$n len=[string length $n]" }
    /// ->  z1\ len=3      (quoted `\\`   -> one backslash)
    ///     z2} len=3      (quoted `\}`   -> a brace)
    ///     z3 x len=4     (quoted `\ `   -> a space)
    ///     z4\ len=3      (bare   `\\`   -> one backslash)
    ///     z5\\ len=4     (BRACED `\\`   -> kept verbatim: the negative case)
    ///     z6A len=3      (quoted `\x41`)
    ///     z7<TAB> len=3  (quoted `\t`)
    ///
    /// # the resolved name is never substituted again
    /// set {{a}} V ; set {a[bogus]} V ; set {${x}} V ; set arr({k}) V
    /// ->  variables `{a}` (len 3), `a[bogus]` (len 8) and `${x}` (len 4),
    ///     plus `arr` with the single key `{k}` (len 3) — no command runs,
    ///     no `x` is read, no braces are stripped.
    ///
    /// # the two halves together: escapes decide where the element starts, and
    /// # an escaped `$` inside a key stays a literal `$`
    /// set i IDX ; set q\(b\) 1 ; set r(\$i) 2 ; set s($i) 3
    /// ->  array q key b ; array r key {$i} ; array s key IDX
    /// ```
    pub(super) fn store_target(&mut self, name: &str, name_braced: bool) -> Option<StoreTarget> {
        // Braces suppressed every substitution *except* the one they permit:
        // a `\<newline>` continuation (and the whitespace after it) collapses
        // to a single space. Doing it here rather than at each push site keeps
        // the name final for the local-variable table too — tclsh 8.4.20 /
        // 8.5.19 / 8.6.16 / 9.0.4 / 9.1 all make `set {z1\<newline>y} B` the
        // 4-byte name `z1 y` (hex `7a312079`).
        if name_braced {
            return Some(StoreTarget {
                name: self.word_rules.collapse_braced_word(name).into_owned(),
                key_is_literal: true,
            });
        }
        if has_unescaped_subst(name) {
            // Live key: keep the key's source spelling so `push_array_key`
            // substitutes it, but resolve the escape-only base as Tcl does.
            let name = match split_array_ref(name) {
                Some((base, elem)) => format!("{}({elem})", self.decoded_name(base)?),
                None => name.to_owned(),
            };
            return Some(StoreTarget {
                name,
                key_is_literal: false,
            });
        }
        Some(StoreTarget {
            name: self.decoded_name(name)?,
            key_is_literal: true,
        })
    }

    /// A native name needs its own represented byte lookup protocol.
    /// Decline emission when that protocol cannot accept the decoded bytes.
    fn decoded_name(&mut self, source: &str) -> Option<String> {
        let bytes = tcl_lexer::backslash_subst_bytes_in(source.as_bytes(), self.escapes);
        if let Ok(name) = std::str::from_utf8(&bytes) {
            Some(name.to_owned())
        } else {
            self.refuse_native_dependency();
            None
        }
    }

    /// Emit the retained fused assignment with its nested expression boundary.
    fn emit_assign_expression(&mut self, stmt: &Statement) {
        let Statement::AssignExpr {
            name,
            name_braced,
            expr,
            command_binding,
            fallback_value,
            ..
        } = stmt
        else {
            unreachable!("selected expression assignment");
        };
        // The fused command retains the exact source binding it
        // consumed. Runtime revalidation checks this dependency at the
        // nested START_CMD boundary, so whole-unit mutation discovery
        // must not prematurely discard a still-valid specialisation.
        if let Some(command_binding) = command_binding {
            self.require_command_binding(command_binding);
        }
        let Some(target) = self.store_target(name, *name_braced) else {
            return;
        };
        let name = target.name.as_str();
        if self.target_needs_stack(name) {
            self.push_var_ref(name, target.key_is_literal);
        }
        let inner_end = self.fresh_label("cmd_end");
        self.emit_comment(
            Op::START_CMD,
            vec![Operand::Label(inner_end.clone()), Operand::Imm(1)],
            "",
        );
        let parent = self.invocation_tokens.clone();
        let nested =
            self.nested_command_tokens(parent.as_deref().and_then(|tokens| tokens.words().get(2)));
        let expr_start = self.instructions.len();
        self.with_invocation_tokens(nested.as_ref(), |ctx| {
            if !ctx.emit_retained_native_expression() && !ctx.emit_expr(expr) {
                ctx.emit(Op::TRY_CVT_TO_NUMERIC, vec![]);
            }
        });
        self.stamp_substitution_error_info(expr_start, fallback_value.trim(), None);
        self.place_label(&inner_end);
        self.store_var(name);
        self.emit(Op::POP, vec![]);
    }

    /// Emit assignments, increments and expression-result statements.
    pub(super) fn emit_assign_or_incr(&mut self, stmt: &Statement) -> bool {
        match stmt {
            Statement::AssignConst {
                name,
                name_braced,
                value,
                ..
            } => {
                let Some(target) = self.store_target(name, *name_braced) else {
                    return true;
                };
                let name = target.name.as_str();
                if self.target_needs_stack(name) {
                    self.push_var_ref(name, target.key_is_literal);
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
                tokens,
                ..
            } => {
                self.emit_assign_value(
                    name,
                    *name_braced,
                    value,
                    *value_needs_backsubst,
                    tokens.as_ref().and_then(|tokens| tokens.words().get(2)),
                );
                true
            }
            Statement::AssignExpr { .. } => {
                self.emit_assign_expression(stmt);
                true
            }
            Statement::Incr {
                name,
                name_braced,
                amount,
                amount_braced,
                ..
            } => {
                if self.try_emit_original_increment() {
                    self.emit(Op::POP, vec![]);
                    return true;
                }
                let Some(target) = self.store_target(name, *name_braced) else {
                    return true;
                };
                self.emit_incr(
                    &target.name,
                    target.key_is_literal,
                    amount.as_deref(),
                    *amount_braced,
                );
                self.emit(Op::POP, vec![]);
                true
            }
            Statement::ExprEval {
                command_binding,
                expr,
                ..
            } => {
                self.require_command_binding(command_binding);
                if self.emit_retained_native_expression() {
                    self.emit(Op::POP, vec![]);
                    return true;
                }
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

    /// Emit the shared `set name VALUE` storage path. `AssignExpr` uses this
    /// too when its nested `expr` command was rebound after lowering fused it.
    fn emit_assign_value(
        &mut self,
        name: &str,
        name_braced: bool,
        value: &str,
        value_needs_backsubst: bool,
        value_word: Option<&WordExpr>,
    ) {
        // Whether this word's escapes were decoded *here*. A decoded value is
        // finished: every marker left in it came from an escape and is data, so
        // it must not be read as source again. `set v "\$\{x}"` decodes to the
        // four characters `${x}`, and re-reading that as a variable reference —
        // which is what the value emitter's `${…}` fast path does — loaded `x`
        // and stored one character instead.
        let decoded_here = value_needs_backsubst && !value.contains('[') && !value.contains("${");
        let value = if decoded_here {
            value.to_owned()
        } else {
            // A value carrying a `[` or `${` (an escaped `\[` / `\${` or
            // a real substitution) is left raw: the runtime `subst_word`
            // does backslash decoding plus command / variable substitution in
            // one left-to-right pass. Pre-decoding would turn escaped markers
            // into live substitutions and would double-decode other escapes.
            value.to_owned()
        };
        let inline = self.assign_value_inlines_cmd_subst(&value);
        let Some(target) = self.store_target(name, name_braced) else {
            return;
        };
        let name = target.name.as_str();
        if self.target_needs_stack(name) {
            self.push_var_ref(name, target.key_is_literal);
        }

        // Let the registry-owned fold have first refusal. If the active
        // command surface declines the fold, dispatch the same whole-word
        // substitution through that surface's inline hook rather than a
        // command-name allow/deny list in this consumer.
        if decoded_here {
            // Finished above: byte-exact, with no further reading of the text.
            self.push_decoded_literal(&value);
        } else if self.try_emit_constant_fold(&value) {
            // The shared fold emitted the value.
        } else if inline {
            // Assignment already owns this shape's direct inline dispatcher:
            // it handles expanded and multi-command bodies as well as every
            // established specialised hook.  Carry source tokens only as an
            // optional fact for its `info` / `array` local-name decision.
            let nested = self.nested_command_tokens(value_word);
            self.emit_inline_cmd_subst_with_tokens(&value, nested.as_ref());
        } else {
            self.emit_value_interpolated_from_word(&value, value_word);
        }
        self.store_var(name);
        self.emit(Op::POP, vec![]);
    }
}
