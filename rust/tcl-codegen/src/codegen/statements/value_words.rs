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

//! Values in word position: interpolation, retained templates and expressions, and whole-word command substitutions.

use super::super::cmd_subst::{is_pure_cmd_subst_with_config, parse_cmd_parts_with_config};
use super::super::helpers::{SubstPart, parse_subst_template};
use super::super::values::split_array_ref;
use super::super::{CodegenCtx, Op, Operand};
use super::has_escaped_subst_marker;
use tcl_compiler::ir::WordExpr;
use tcl_compiler::word_subst::whole_word_command_tokens;

impl CodegenCtx<'_> {
    /// Emit a simple value — handles `${var}` references and literals.
    ///
    /// This is a simplified value emitter that handles the common cases.
    /// Full interpolation with command substitution requires the main
    /// emitter pipeline.
    pub fn emit_value_interpolated(&mut self, value: &str) {
        self.emit_value_interpolated_from_word(value, None);
    }

    fn emit_variable_index(&mut self, value: &str) -> bool {
        // Whole-word variable-index array element `$arr($idx)`: route through
        // `load_var` (base + substituted key) rather than the literal/runtime
        // subst path, which cannot resolve the bare `$idx` inside the index.
        if value.starts_with('$')
            && value.ends_with(')')
            && let Some(parts) = parse_subst_template(value, self.lexer_config())
            && parts.len() == 1
            && let SubstPart::Var(name) = &parts[0]
            && split_array_ref(name).is_some()
        {
            self.load_var(name);
            return true;
        }
        false
    }

    /// Emit a value while retaining a source word only for an aligned nested
    /// command substitution. Every other value follows the established path.
    pub(super) fn emit_value_interpolated_from_word(
        &mut self,
        value: &str,
        word: Option<&WordExpr>,
    ) {
        if word.is_some_and(|word| self.emit_retained_expression(value, word)) {
            return;
        }
        if word.is_some_and(|word| self.emit_retained_template(value, word)) {
            return;
        }
        if let Some(WordExpr::Variable { spelling, .. }) = word
            && word.is_some_and(|word| value == word.legacy_text())
            && self.emit_variable_reference(spelling)
        {
            return;
        }
        if self.emit_variable_reference(value) {
            return;
        }
        // The `[list …]` / `[format …]` / `[dict create …]` folds and the two
        // `list` inlinings — one copy, shared with `emit_value`.
        if self.try_emit_constant_fold(value) {
            return;
        }
        if self.emit_variable_index(value) {
            return;
        }
        // Interpolated string containing an array-element variable
        // (`"…$arr($idx)…"`): the runtime `subst_word` fallback can substitute a
        // normalised `${name}` but not an array element with a substituted
        // index, so decompose the word at compile time. Plain `$scalar`
        // interpolation keeps its existing (deferred-literal) lowering.
        // Also decompose a `{…}`-wrapped value that still carries a live
        // substitution (e.g. `"{[list …]}"`, `"{$z}"`): the braces are literal
        // word content and the `[…]` / `${…}` must run, but pushing the value
        // raw would let the runtime `subst_word` mistake it for a braced
        // literal, strip the braces and hand back the *unsubstituted* inside:
        // `puts "{$z}"` would print `${z}`, and `set q "{$z}"` would store four
        // bytes where both oracles store three. This is the marker-carrying half of
        // the value rule `push_compiled_word` states: a value the runtime still
        // owns cannot simply be frozen, so it is resolved here instead. A
        // genuine braced argument never reaches here (it is emitted by the
        // braced-word path), so decomposing is safe.
        //
        // The brace test is the VM's own rule, through the same owner the VM
        // asks (`whole_braced_word`) — this arm exists precisely because
        // `subst_word` *will* strip, so anything looser decomposes words it
        // would not have touched and anything two-byte repeats the bug this
        // change is about. `{}${z}` is the discriminating case: it is not a
        // braced word, the VM's general scan substitutes it correctly, and it
        // stays on the literal path.
        if (value.contains('$') || value.contains('['))
            && let Some(parts) = parse_subst_template(value, self.lexer_config())
            && parts.len() > 1
            && (parts.iter().any(|p| {
                matches!(p, SubstPart::LiteralElement { .. })
                    || matches!(p, SubstPart::Var(n) if split_array_ref(n).is_some())
            }) || (tcl_syntax::word_rules::whole_braced_word(value).is_some()
                && parts.iter().any(|p| {
                    matches!(
                        p,
                        SubstPart::Cmd(_) | SubstPart::Var(_) | SubstPart::LiteralElement { .. }
                    )
                })))
        {
            for part in &parts {
                match part {
                    // A decoded fragment is finished text, never source — see
                    // `push_compiled_word`, whose fragment rule this is.
                    SubstPart::Lit(text) => self.push_lit_exact(text),
                    SubstPart::ByteLit(bytes) => self.push_lit_bytes_exact(bytes),
                    SubstPart::Cmd(cmd_text) => self.emit_inline_cmd_subst(cmd_text),
                    SubstPart::Var(name) => self.load_var(name),
                    SubstPart::LiteralElement { base, key } => self.load_literal_element(base, key),
                    SubstPart::Expression(expression) => {
                        self.emit_expression_substitution(expression);
                    }
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
            self.push_decoded_literal(value);
            return;
        }
        // A whole-word command substitution compiles inline (on the explicit
        // stack) rather than via the runtime `subst_word` fallback, so a
        // `[yield]`/`[cmd]` inside it stays yieldable in a coroutine.
        if self.emit_inline_cmd_subst_from_word(value, word) {
            return;
        }
        // A word carrying an *escaped* marker cannot be deferred at all, even
        // when it also carries a live one. The VM reads a compiled literal's
        // backslashes as ordinary characters, so an undecoded `\$\{x}` handed
        // over raw is read back as a live `${x}`. Resolve the word here
        // instead: the template parser decodes each literal run and separates
        // the substitutions that really are live, so the escaped marker ends up
        // as data and the `[…]` beside it still runs.
        if has_escaped_subst_marker(value)
            && let Some(parts) = parse_subst_template(value, self.lexer_config())
        {
            for part in &parts {
                match part {
                    SubstPart::Lit(text) => self.push_lit_exact(text),
                    SubstPart::ByteLit(bytes) => self.push_lit_bytes_exact(bytes),
                    SubstPart::Cmd(cmd_text) => self.emit_inline_cmd_subst(cmd_text),
                    SubstPart::Var(name) => self.load_var(name),
                    SubstPart::LiteralElement { base, key } => self.load_literal_element(base, key),
                    SubstPart::Expression(expression) => {
                        self.emit_expression_substitution(expression);
                    }
                }
            }
            if parts.len() > 1 {
                self.emit(
                    Op::STR_CONCAT1,
                    vec![Operand::Imm(
                        i32::try_from(parts.len()).expect("part count fits in i32"),
                    )],
                );
            } else if parts.is_empty() {
                self.push_lit_exact("");
            }
            return;
        }
        // Default: the word's own text is its value — push it unsubstituted
        // unless a `${…}` / `[…]` the runtime still owns survived the arms
        // above (`push_compiled_word`). Substituting a finished value strips a
        // brace layer that was never source: `set v "{}"` stored the empty
        // string where both oracles store the two-byte `{}`.
        self.push_compiled_word(value);
    }

    /// Evaluate the retained lexical components of an unchanged template.
    /// Command components retain their own source proof; decoded text is data.
    fn emit_retained_template(&mut self, value: &str, word: &WordExpr) -> bool {
        use tcl_compiler::ir::WordPart;
        let WordExpr::Template { parts, .. } = word else {
            return false;
        };
        if value != word.legacy_text() {
            return false;
        }
        let mut operands = Vec::with_capacity(parts.len());
        for part in parts {
            let operand = match part {
                WordPart::Text { text, .. } => {
                    (SubstPart::decoded_literal(text, self.escapes), None)
                }
                WordPart::Variable { spelling, .. } => {
                    let Some(parsed) = parse_subst_template(spelling, self.lexer_config()) else {
                        return false;
                    };
                    let [variable @ (SubstPart::Var(_) | SubstPart::LiteralElement { .. })] =
                        parsed.as_slice()
                    else {
                        return false;
                    };
                    (variable.clone(), None)
                }
                WordPart::CommandSubstitution { spelling, source } => (
                    SubstPart::Cmd(spelling.clone()),
                    Some(WordExpr::CommandSubstitution {
                        spelling: spelling.clone(),
                        source: source.clone(),
                    }),
                ),
                WordPart::Opaque { text, .. } => {
                    let Some(parsed) = parse_subst_template(text, self.lexer_config()) else {
                        return false;
                    };
                    let [expression @ SubstPart::Expression(_)] = parsed.as_slice() else {
                        return false;
                    };
                    (expression.clone(), None)
                }
            };
            operands.push(operand);
        }
        for (operand, source) in &operands {
            match operand {
                SubstPart::Lit(text) => self.push_lit_exact(text),
                SubstPart::ByteLit(bytes) => self.push_lit_bytes_exact(bytes),
                SubstPart::Var(name) => self.load_var(name),
                SubstPart::LiteralElement { base, key } => self.load_literal_element(base, key),
                SubstPart::Expression(expression) => self.emit_expression_substitution(expression),
                SubstPart::Cmd(text) => {
                    self.emit_value_interpolated_from_word(text, source.as_ref());
                }
            }
        }
        // A lexical Template containing one substitution is a quoted word
        // (bare single substitutions have their own WordExpr variants). Jim
        // copies that word's bytes instead of retaining its source intrep.
        let copies_single = operands.len() == 1
            && matches!(
                operands[0].0,
                SubstPart::Var(_)
                    | SubstPart::LiteralElement { .. }
                    | SubstPart::Cmd(_)
                    | SubstPart::Expression(_)
            )
            && self.native_hook_dialect().is_some_and(|dialect| {
                dialect.quoted_substitution_preserves_object() == Some(false)
            });
        if operands.len() > 1 || copies_single {
            self.emit(
                Op::STR_CONCAT1,
                vec![Operand::Imm(super::super::bytecode_imm(operands.len()))],
            );
        } else if operands.is_empty() {
            self.push_lit_exact("");
        }
        true
    }

    /// Retain the native expression token for the VM's direct word evaluator.
    pub(in super::super) fn emit_expression_substitution(&mut self, expression: &str) {
        self.push_lit(&format!("${expression}"));
    }

    /// Evaluate an original bare expression token without naming a command.
    fn emit_retained_expression(&mut self, value: &str, word: &WordExpr) -> bool {
        let WordExpr::Opaque {
            text,
            reason: tcl_compiler::ir::WordOpacity::DialectSubstitution,
            ..
        } = word
        else {
            return false;
        };
        if text != value {
            return false;
        }
        let Some(parts) = parse_subst_template(text, self.lexer_config()) else {
            return false;
        };
        let [SubstPart::Expression(expression)] = parts.as_slice() else {
            return false;
        };
        self.emit_expression_substitution(expression);
        true
    }

    /// Whether a `set x VALUE` value should inline-compile its command
    /// substitution (`set x [cmd arg …]`) instead of pushing the raw `[…]` text
    /// and leaning on the runtime `subst_word` fallback. The conditions:
    /// a balanced `[…]` with inner whitespace (a bare `[foo]` with no args
    /// stays a literal). The caller first gives the registry-owned
    /// constant-fold path a chance to consume the value, so this shape
    /// predicate contains no per-command knowledge. The inline command emitter
    /// also owns `{*}` argument expansion in this assignment position; keeping
    /// that path live is what makes `set x [concat a {*}$args b]` invoke the
    /// command rather than store its source text. Command arguments keep the
    /// literal + `subst_word` path, which the inline command-parser cannot
    /// match on escaped brackets.
    pub(super) fn assign_value_inlines_cmd_subst(&self, value: &str) -> bool {
        // `is_pure_cmd_subst`, not "starts and ends with a bracket": the inline
        // emitter strips the outer brackets and word-splits the rest, so
        // `set r [llength $a]:[join $a ,]` — three concatenated parts — would
        // be mangled into one bogus command.
        is_pure_cmd_subst_with_config(value, self.lexer_config())
            && value.len() > 2
            && value[1..value.len() - 1].contains(' ')
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
    /// An aligned original word retains its independently admitted native
    /// compiler operation. Its emitter validates the original operands and
    /// captures the compiler prerequisites before argument execution. Other
    /// substitutions invoke the actual command on the explicit activation stack.
    ///
    /// Conservative on shape: escaped brackets (`\[` / `\]`, which the parser
    /// cannot match — see [`Self::assign_value_inlines_cmd_subst`]), `{*}`
    /// expansion, and multi-command bodies (a `;`/newline separator) keep the
    /// runtime path. Those never carry the coroutine resume-value idiom, so
    /// nothing yieldable is lost.
    pub(crate) fn try_emit_whole_cmd_subst(&mut self, value: &str) -> bool {
        self.try_emit_whole_cmd_subst_with_tokens(value, None)
    }

    pub(in super::super) fn nested_command_tokens(
        &self,
        word: Option<&WordExpr>,
    ) -> Option<tcl_compiler::ir::CommandTokens> {
        let mut nested = whole_word_command_tokens(word?, self.lexer_config())?;
        // Re-parsing preserves the original word's source address, but does
        // not itself carry the selected nested callable. Only the owning
        // invocation can supply that exact-site proof; an omitted site stays
        // unknown through the shared inheritance contract.
        if let Some(parent) = self.invocation_tokens.as_deref() {
            nested.inherit_nested_bindings(parent);
        }
        Some(nested)
    }

    fn emit_inline_cmd_subst_from_word(&mut self, value: &str, word: Option<&WordExpr>) -> bool {
        let nested = self.nested_command_tokens(word);
        self.try_emit_whole_cmd_subst_with_tokens(value, nested.as_ref())
    }

    fn try_emit_whole_cmd_subst_with_tokens(
        &mut self,
        value: &str,
        tokens: Option<&tcl_compiler::ir::CommandTokens>,
    ) -> bool {
        let start = self.instructions.len();
        let emitted = self.with_invocation_tokens(tokens, |ctx| {
            ctx.try_emit_whole_cmd_subst_inner(value, tokens)
        });
        if emitted {
            self.stamp_substitution_error_info(start, value.trim(), tokens);
        }
        emitted
    }

    fn try_emit_whole_cmd_subst_inner(
        &mut self,
        value: &str,
        tokens: Option<&tcl_compiler::ir::CommandTokens>,
    ) -> bool {
        if value.contains("\\[") || value.contains("\\]") || value.contains("{*}") {
            return false;
        }
        if !matches!(
            parse_subst_template(value, self.lexer_config()).as_deref(),
            Some([SubstPart::Cmd(_)])
        ) {
            return false;
        }
        let Some(parts) = parse_cmd_parts_with_config(value, self.lexer_config()) else {
            return false;
        };
        let Some((head, _)) = parts.split_first() else {
            return false;
        };
        let args = &parts[1..];
        let source_aware = tokens.is_some_and(|tokens| {
            tokens.words_align_with_argv_text()
                && tokens.argv_texts.len() == parts.len()
                && tokens
                    .argv_texts
                    .iter()
                    .zip(&parts)
                    .all(|(word, (text, _))| word == text)
        });
        if source_aware && self.emit_native_original_preparation_invocation(tokens) {
            return true;
        }
        if source_aware
            && self
                .inline_cmd_subst_hook_candidate(&head.0, args)
                .is_some()
        {
            self.emit_inline_cmd_subst_with_tokens(value, tokens);
            return true;
        }
        if self.try_emit_compiled_index_command(&head.0, args) {
            return true;
        }
        // A source snapshot may be unavailable for a compatibility caller.
        // Preserve its established generic value emission rather than
        // guessing from flattened text.
        self.emit_generic_cmd_subst(&head.0, &parts[1..]);
        true
    }
}
