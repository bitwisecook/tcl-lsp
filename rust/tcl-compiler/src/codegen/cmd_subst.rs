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

//! Command substitution parsing and inline emission.
//!
//! Extends [`CodegenCtx`] with methods for parsing `[cmd arg ...]`
//! substitutions and emitting specialised bytecode sequences for
//! common Tcl commands (expr, incr, string, list, dict, etc.).

use tcl_registry::hooks::InlineCodegenHookId;

use super::helpers::{SubstPart, parse_subst_template, regexp_to_glob};
use super::values::{is_qualified, parse_simple_var_ref, split_array_ref};
use super::{CodegenCtx, INDEX_END, Op, Operand, bytecode_imm, parse_tcl_index, str_class_id};

// Free functions — pure parsing, no emission state needed

/// Whether a [`parse_tcl_index`] result is encodable as a `*_IMM` index
/// operand: a non-negative index, or an `end` / `end-N` index
/// (`<= INDEX_END`). An `end+N` index encodes as `INDEX_END + N`
/// (> `INDEX_END`) — neither — so it must fall back to the non-immediate
/// opcode rather than be emitted as a garbage immediate. This is
/// the same guard `lindex` already applies.
const fn imm_index_ok(idx: i32) -> bool {
    idx >= 0 || idx <= INDEX_END
}

/// Unroll `[set y [set z 42]]` into `["y", "z", "42"]`.
///
/// Returns the variable names followed by the innermost value,
/// or `None` if `value` is not a chain of nested `set` commands.
#[must_use]
pub fn unroll_nested_set(value: &str) -> Option<Vec<String>> {
    let mut chain = Vec::new();
    let mut v = value;
    while v.starts_with("[set ") && v.ends_with(']') {
        let inner = &v[1..v.len() - 1]; // strip [ ]
        let mut parts = inner.splitn(3, char::is_whitespace);
        let cmd = parts.next()?;
        if cmd != "set" {
            return None;
        }
        let var = parts.next()?;
        let rest = parts.next()?;
        if rest.is_empty() {
            return None;
        }
        chain.push(var.to_owned());
        v = rest;
    }
    if chain.is_empty() {
        return None;
    }
    chain.push(v.to_owned()); // innermost value
    Some(chain)
}

/// Return `true` if `value` is a single balanced `[cmd ...]`.
#[must_use]
pub fn is_pure_cmd_subst(value: &str) -> bool {
    if !value.starts_with('[') || !value.ends_with(']') {
        return false;
    }
    let mut depth: i32 = 0;
    let bytes = value.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let ch = bytes[i];
        if ch == b'\\' {
            i += 2; // skip escaped char (crude but sufficient)
            continue;
        }
        if ch == b'[' {
            depth += 1;
        } else if ch == b']' {
            depth -= 1;
        }
        if depth == 0 {
            return i == bytes.len() - 1;
        }
        i += 1;
    }
    false
}

/// Return `true` if `text` contains `;` or newline outside quotes/braces.
///
/// Used to detect multi-command scripts in `[...]` substitutions so
/// they can be deferred to runtime `EVAL_STK` instead of being
/// inlined as a single command call.
#[must_use]
pub fn has_command_separator(text: &str) -> bool {
    let inner = if text.starts_with('[') && text.ends_with(']') {
        &text[1..text.len() - 1]
    } else {
        text
    };
    let bytes = inner.as_bytes();
    let mut in_quotes = false;
    let mut brace_depth: i32 = 0;
    let mut bracket_depth: i32 = 0;
    let mut i = 0;
    while i < bytes.len() {
        let ch = bytes[i];
        if ch == b'\\' && i + 1 < bytes.len() {
            i += 2; // skip escaped character
            continue;
        }
        if ch == b'"' && brace_depth == 0 {
            in_quotes = !in_quotes;
        } else if !in_quotes {
            if ch == b'{' {
                brace_depth += 1;
            } else if ch == b'}' && brace_depth > 0 {
                brace_depth -= 1;
            } else if ch == b'[' {
                bracket_depth += 1;
            } else if ch == b']' && bracket_depth > 0 {
                bracket_depth -= 1;
            } else if (ch == b';' || ch == b'\n') && brace_depth == 0 && bracket_depth == 0 {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// Split `[cmd arg1 ...]` into `(text, was_braced)` tuples.
///
/// `was_braced` is `true` when the original argument was wrapped
/// in `{…}` (braces are stripped from the returned text). This
/// lets the caller decide whether to re-wrap the value in braces.
// Sequential character-walk parser; the brace / quote / bracket / escape
/// Skip past a balanced `[...]` substitution starting at *i*.
/// Returns the new cursor position past the matching `]`.
fn skip_cmd_subst(bytes: &[u8], n: usize, mut i: usize) -> usize {
    let mut depth: i32 = 0;
    while i < n {
        if bytes[i] == b'[' {
            depth += 1;
        } else if bytes[i] == b']' {
            depth -= 1;
            if depth == 0 {
                i += 1;
                break;
            }
        }
        i += 1;
    }
    i
}

/// Parse one quoted-string `"..."` part starting at *i* (which
/// points at the opening `"`).  Returns `(part_text, new_i)`.
fn parse_quoted_part(text: &str, bytes: &[u8], n: usize, mut i: usize) -> (String, usize) {
    i += 1;
    let start = i;
    while i < n && bytes[i] != b'"' {
        if bytes[i] == b'\\' {
            i += 1;
        }
        i += 1;
    }
    let part = text[start..i].to_owned();
    if i < n {
        i += 1; // skip closing "
    }
    (part, i)
}

/// Parse one braced `{...}` part starting at *i*.
fn parse_braced_part(text: &str, bytes: &[u8], n: usize, mut i: usize) -> (String, usize) {
    let mut depth: i32 = 1;
    i += 1;
    let start = i;
    while i < n && depth > 0 {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if bytes[i] == b'{' {
            depth += 1;
        } else if bytes[i] == b'}' {
            depth -= 1;
        }
        if depth > 0 {
            i += 1;
        }
    }
    let part = text[start..i].to_owned();
    if i < n {
        i += 1; // skip closing }
    }
    (part, i)
}

/// Parse one bare word or `[cmd]`-substitution-rooted word starting
/// at *i*.  Bare-word handling reads non-whitespace bytes;
/// `[cmd]` mid-word is consumed as an opaque substitution.
fn parse_bareword_part(text: &str, bytes: &[u8], n: usize, mut i: usize) -> (String, usize) {
    let start = i;
    while i < n && bytes[i] != b' ' && bytes[i] != b'\t' {
        if bytes[i] == b'[' {
            i = skip_cmd_subst(bytes, n, i);
        } else {
            i += 1;
        }
    }
    (text[start..i].to_owned(), i)
}

/// Advance past inter-word separators inside a single command: horizontal
/// whitespace (` `/`\t`) and a `\<newline>` line continuation (backslash, then
/// `\n` or `\r\n`/`\r`, then any leading horizontal whitespace of the next
/// line). A continuation is a word separator in Tcl — without skipping it the
/// tokenizer mis-split a multi-line command's words (e.g. `string range $x \`
/// <newline> `$i $j`), dropping an argument and raising a spurious
/// "wrong # args" (tcltest's `SubstArguments` → info / lrepeat / lseq, and
/// every test file using the `{-body … -result …}` dict form).
fn skip_word_seps(bytes: &[u8], n: usize, mut i: usize) -> usize {
    loop {
        if i < n && (bytes[i] == b' ' || bytes[i] == b'\t') {
            i += 1;
        } else if i + 1 < n && bytes[i] == b'\\' && (bytes[i + 1] == b'\n' || bytes[i + 1] == b'\r')
        {
            i += 2;
            if i < n && bytes[i - 1] == b'\r' && bytes[i] == b'\n' {
                i += 1; // `\r\n`
            }
        } else {
            break;
        }
    }
    i
}

/// Parse a command-substitution body into `(text, braced)` parts.
/// `braced=true` means the text was a `{...}` literal — the caller
/// should not interpolate it.  Strips the outer `[...]` if present.
#[must_use]
pub fn parse_cmd_parts(text: &str) -> Vec<(String, bool)> {
    let text = text.trim();
    let text = if text.starts_with('[') && text.ends_with(']') {
        text[1..text.len() - 1].trim()
    } else {
        text
    };
    let bytes = text.as_bytes();
    let n = bytes.len();
    let mut parts = Vec::new();
    let mut i = 0;

    while i < n {
        i = skip_word_seps(bytes, n, i);
        if i >= n {
            break;
        }
        match bytes[i] {
            b'"' => {
                let (part, new_i) = parse_quoted_part(text, bytes, n, i);
                parts.push((part, false));
                i = new_i;
            }
            b'{' => {
                let (part, new_i) = parse_braced_part(text, bytes, n, i);
                parts.push((part, true));
                i = new_i;
            }
            b'[' => {
                let start = i;
                i = skip_cmd_subst(bytes, n, i);
                // Continue past trailing non-ws (e.g. ``[ns current]::foo``).
                while i < n && bytes[i] != b' ' && bytes[i] != b'\t' {
                    if bytes[i] == b'[' {
                        i = skip_cmd_subst(bytes, n, i);
                    } else {
                        i += 1;
                    }
                }
                parts.push((text[start..i].to_owned(), false));
            }
            _ => {
                let (part, new_i) = parse_bareword_part(text, bytes, n, i);
                parts.push((part, false));
                i = new_i;
            }
        }
    }
    parts
}

/// Like [`parse_cmd_parts`] but also flags `{*}`-expansion words. Each tuple is
/// `(text, braced, expand)`: `expand` is true when the word carried a leading
/// `{*}` prefix (stripped from `text`). A `{*}` *not* immediately followed by
/// word content (`{*}` then a space / end) is a literal braced `*`, not an
/// expansion — matching Tcl's `{*}` rule.
#[must_use]
pub fn parse_cmd_parts_expand(text: &str) -> Vec<(String, bool, bool)> {
    let text = text.trim();
    let text = if text.starts_with('[') && text.ends_with(']') {
        text[1..text.len() - 1].trim()
    } else {
        text
    };
    let bytes = text.as_bytes();
    let n = bytes.len();
    let mut parts = Vec::new();
    let mut i = 0;

    while i < n {
        i = skip_word_seps(bytes, n, i);
        if i >= n {
            break;
        }
        // `{*}` immediately followed by non-whitespace word content → expansion.
        let mut expand = false;
        if i + 3 < n && &bytes[i..i + 3] == b"{*}" && bytes[i + 3] != b' ' && bytes[i + 3] != b'\t'
        {
            expand = true;
            i += 3;
        }
        let (part, braced, new_i) = match bytes[i] {
            b'"' => {
                let (p, ni) = parse_quoted_part(text, bytes, n, i);
                (p, false, ni)
            }
            b'{' => {
                let (p, ni) = parse_braced_part(text, bytes, n, i);
                (p, true, ni)
            }
            b'[' => {
                let start = i;
                let mut j = skip_cmd_subst(bytes, n, i);
                while j < n && bytes[j] != b' ' && bytes[j] != b'\t' {
                    if bytes[j] == b'[' {
                        j = skip_cmd_subst(bytes, n, j);
                    } else {
                        j += 1;
                    }
                }
                (text[start..j].to_owned(), false, j)
            }
            _ => {
                let (p, ni) = parse_bareword_part(text, bytes, n, i);
                (p, false, ni)
            }
        };
        parts.push((part, braced, expand));
        i = new_i;
    }
    parts
}

// CodegenCtx methods — emission helpers for command substitutions

impl CodegenCtx<'_> {
    /// Emit a single arg from a parsed command substitution.
    ///
    /// Handles: `$var` loads, `$={name}` braced scalars, `[cmd]`
    /// nested substitutions, braced args with `$`/`[`, interpolated
    /// strings, backslash escapes, and plain literals.
    pub fn emit_cmd_subst_arg(&mut self, arg: &str, braced: bool) {
        // A composite unbraced arg with an embedded substitution (`$opt*`,
        // `x$y`, `${a}b`, `pre[cmd]post`) decomposes into more than one part:
        // build the string by concatenating the substituted parts rather than
        // pushing the raw text as a literal (which would leave `$opt*` /
        // `pre[cmd]` un-substituted). A pure `$var` / `${var}` / `$arr(i)` /
        // `[cmd]` is a single part and falls through to the fast paths below.
        if !braced
            && (arg.contains('$') || arg.contains('['))
            && let Some(parts) = parse_subst_template(arg, self.escapes, self.braced_var)
            && parts.len() > 1
        {
            for part in &parts {
                match part {
                    SubstPart::Lit(text) => self.push_lit(text),
                    SubstPart::Cmd(cmd) => self.emit_inline_cmd_subst(cmd),
                    SubstPart::Var(name) => self.load_var(name),
                }
            }
            self.emit(
                Op::STR_CONCAT1,
                vec![Operand::Imm(bytecode_imm(parts.len()))],
            );
            return;
        }
        if !braced && arg.starts_with('$') {
            // ${var} form
            if let Some(var_name) = parse_simple_var_ref(arg, self.braced_var) {
                self.load_var(var_name);
                return;
            }
            // Bare $varname form (not normalised to ${var}), including a
            // namespace-qualified name (`$::x`, `$ns::v`): a whole-word variable
            // reference whose name is alphanumerics/`_` joined only by `::`
            // separators. `is_bare_var_name` enforces the `::`-pair rule, so a
            // lone trailing/interior colon (`$action:` = `$action` then literal
            // `:`) is *not* swallowed into the name (it would `load_var
            // "action:"`); such interpolated words fall through to `emit_value`.
            // Loading the qualified form here also fixes `$::x` measuring the
            // literal `$::x` (the runtime `subst_word` only substitutes `${…}`).
            let rest = &arg[1..];
            if tcl_syntax::naming::is_bare_var_name(rest) {
                self.load_var(rest);
                return;
            }
            // Bare $name(index) array ref form
            if !rest.is_empty() && split_array_ref(rest).is_some() {
                self.load_var(rest);
                return;
            }
            // Fallback: push as literal
            self.push_lit(arg);
        } else if !braced && arg.starts_with('[') && arg.ends_with(']') {
            // Nested command substitution — compile inline. Everything
            // except an `expr` body gets a startCommand wrap: `expr`'s
            // inline emitter (the registry-stamped
            // `InlineCodegenHookId::Expr`) compiles to pure stack ops
            // with no command boundary, so tclsh emits no startCommand
            // for it.
            let inner_parts = parse_cmd_parts(arg);
            let needs_sc = self.is_proc
                && !inner_parts.is_empty()
                && self.inline_cmd_subst_hook(&inner_parts[0].0, &inner_parts[1..])
                    != Some(InlineCodegenHookId::Expr);
            if needs_sc {
                let sc_end = self.fresh_label("cmd_end");
                self.emit_comment(
                    Op::START_CMD,
                    vec![Operand::Label(sc_end.clone()), Operand::Imm(1)],
                    "",
                );
                self.cmd_index += 1;
                self.emit_inline_cmd_subst(arg);
                self.place_label(&sc_end);
            } else {
                self.emit_inline_cmd_subst(arg);
            }
        } else if braced && (arg.contains('$') || arg.contains('[')) {
            // Braced arg with substitution markers — re-wrap in braces
            self.push_lit(&format!("{{{arg}}}"));
        } else if !braced && (arg.contains('$') || arg.contains('[')) {
            // Interpolated string — delegate to emit_value with interpolation
            self.emit_value(arg, true);
        } else if !braced && arg.contains('\\') {
            let processed = tcl_lexer::backslash_subst_in(arg, self.escapes);
            if processed.contains('$') || (processed.contains('[') && processed.contains(']')) {
                // After backslash processing, still has subst markers — push raw
                self.push_lit(&processed);
            } else {
                self.push_lit(&processed);
            }
        } else {
            self.push_lit(arg);
        }
    }

    /// Emit a generic command substitution as `push cmd; <args>; invokeStk`.
    pub fn emit_generic_cmd_subst(&mut self, cmd: &str, args: &[(String, bool)]) {
        // The command name itself may be a substitution (`$Verify($opt) ...`,
        // `[lookup] ...`), so it is emitted through the same per-word path as
        // the arguments rather than as a bare literal.
        self.emit_cmd_word(cmd, false);
        for (arg, braced) in args {
            self.emit_cmd_word(arg, *braced);
        }
        let arg_count = bytecode_imm(1 + args.len());
        let op = if arg_count < 256 {
            Op::INVOKE_STK1
        } else {
            Op::INVOKE_STK4
        };
        self.emit(op, vec![Operand::Imm(arg_count)]);
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
            if let Some(var_name) = parse_simple_var_ref(word, self.braced_var) {
                self.load_var(var_name);
            } else {
                // Bare `$name` / `$name(idx)` — load the variable rather than
                // pushing the unsubstituted literal. A namespace-qualified name
                // (`$::x`, `$ns::v`) counts as bare, but only with `::`-pair
                // separators (`is_bare_var_name`): a lone colon ends the name, so
                // `$action:` is `$action` then a literal `:`, not a variable
                // `action:`. Without the qualified case a `$::x` fell through to
                // `emit_value` → `push_lit("$::x")`, which the runtime leaves
                // unsubstituted (only `${…}` is a subst trigger).
                let rest = &word[1..];
                let is_bare = tcl_syntax::naming::is_bare_var_name(rest);
                if is_bare || (!rest.is_empty() && split_array_ref(rest).is_some()) {
                    self.load_var(rest);
                } else {
                    // A leading `$` followed by more text (`$i.x`, `$a$b`) is an
                    // interpolated word, not a whole-variable reference: decompose
                    // it into its `$var` / literal parts and concatenate, so the
                    // variable is substituted. Pushing it raw left the inline
                    // command-substitution path (e.g. `set m [concat a "$i.x"]`)
                    // with the literal `$i.x`.
                    self.emit_value(word, true);
                }
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
            let processed = tcl_lexer::backslash_subst_in(word, self.escapes);
            self.push_lit(&processed);
        } else if braced {
            // A braced word is already de-braced here, so its content is the
            // finished value: push it verbatim or the VM's `subst_word` strips
            // a *second* brace layer — `proc p {} { set {{loc}} L ; return [set
            // {{loc}}] }` read the local `loc` while the store had created
            // `{loc}` (issue #1602; tclsh 8.6.14 / 9.0.4 return `L`).
            self.push_lit_verbatim(word);
        } else {
            self.push_lit(word);
        }
    }

    /// Inline compile `[list {*}$a {*}$b]` as `load a; load b; listConcat`.
    ///
    /// Only matches the exact two-argument form — `[list {*}$x]` (one
    /// argument) and three-or-more-argument variants fall back to the
    /// generic path. The matched pattern is: `[list` followed by two
    /// `{*}$name` tokens separated by whitespace, closed by `]`.
    ///
    /// Returns `true` if the pattern matched and the bytecode was
    /// emitted.
    pub fn try_list_expand_concat(&mut self, value: &str) -> bool {
        let Some(inner) = value
            .strip_prefix("[list")
            .and_then(|s| s.strip_suffix(']'))
        else {
            return false;
        };
        let mut names: Vec<&str> = Vec::new();
        for tok in inner.split_whitespace() {
            let Some(name) = tok.strip_prefix("{*}$") else {
                return false;
            };
            if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                return false;
            }
            names.push(name);
        }
        if names.len() != 2 {
            return false;
        }
        self.load_var(names[0]);
        self.load_var(names[1]);
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
        if !(value.starts_with("[list ") && value.ends_with(']')) {
            return false;
        }
        let parts = parse_cmd_parts(value);
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

    /// Emit a value with full interpolation support.
    ///
    /// Extends the simplified `emit_value_interpolated` with command
    /// substitution inlining and `parse_subst_template` decomposition.
    pub fn emit_value(&mut self, value: &str, interpolate: bool) {
        // Variable reference: ${var} → load
        if let Some(var_name) = parse_simple_var_ref(value, self.braced_var) {
            self.load_var(var_name);
            return;
        }
        // The `[list …]` / `[format …]` / `[dict create …]` folds and the two
        // `list` inlinings — shared with `emit_value_interpolated`, which
        // carried an identical copy of them (issues #1427 / #1585).
        if self.try_emit_constant_fold(value, super::values::FoldedLiteral::NoDedup) {
            return;
        }
        // Whole-word variable-index array element `$arr($idx)`: route through
        // `load_var` (base + substituted key). The runtime literal/`subst_word`
        // path cannot resolve a bare `$idx` inside the index, so this must be
        // decomposed at compile time into a `loadArray` (or `loadArrayStk`).
        if interpolate
            && value.starts_with('$')
            && value.ends_with(')')
            && let Some(parts) = parse_subst_template(value, self.escapes, self.braced_var)
            && parts.len() == 1
            && let SubstPart::Var(name) = &parts[0]
            && split_array_ref(name).is_some()
        {
            self.load_var(name);
            return;
        }
        // Interpolated string: decompose $var and [cmd] parts
        if interpolate
            && (value.contains('$') || value.contains('['))
            && let Some(parts) = parse_subst_template(value, self.escapes, self.braced_var)
            && parts.len() > 1
        {
            for part in &parts {
                match part {
                    SubstPart::Lit(text) => self.push_lit(text),
                    SubstPart::Cmd(cmd_text) => {
                        self.emit_inline_cmd_subst(cmd_text);
                    }
                    SubstPart::Var(name) => {
                        self.load_var(name);
                    }
                }
            }
            self.emit(
                Op::STR_CONCAT1,
                vec![Operand::Imm(bytecode_imm(parts.len()))],
            );
            return;
        }
        // A whole-word command substitution compiles inline (on the explicit
        // stack) rather than via the runtime `subst_word` fallback, so a
        // `[yield]`/`[cmd]` inside it stays yieldable in a coroutine.
        if interpolate && self.try_emit_whole_cmd_subst(value) {
            return;
        }
        // Default: push as literal
        self.push_lit(value);
    }

    /// Resolve the registry-stamped [`InlineCodegenHookId`] for a
    /// command-substitution head word, mirroring how
    /// `emitter::bytecoded::try_bytecoded` resolves statement-position
    /// hooks (`resolve_call`, so a subcommand-keyed hook such as
    /// `dict get` / `info exists` wins over the command level).
    ///
    /// The spec-name equality check keeps qualified spellings
    /// (`[::expr …]`) on the generic-invoke path: `CommandRegistry::get`
    /// resolves a leading `::` to the bare spec, but the historical
    /// dispatch keyed on the raw head word and the emitted bytecode
    /// must not change under the registry-driven dispatch.
    fn inline_cmd_subst_hook(
        &self,
        cmd: &str,
        args: &[(String, bool)],
    ) -> Option<InlineCodegenHookId> {
        // A name this unit renames, aliases, or shadows with a proc no longer
        // denotes the builtin whose emitter the hook names, so there is
        // nothing to specialise — fall through to the generic invoke and let
        // the runtime dispatch on whatever the name holds (issue #1585).
        if !self.trusts_builtin(cmd) {
            return None;
        }
        let arg_refs: Vec<&str> = args.iter().map(|(a, _)| a.as_str()).collect();
        // The registry's own point — see
        // `emitter::bytecoded::try_bytecoded` (issues #1462/#1463).
        let resolved =
            self.registry
                .resolve_call(cmd, &arg_refs, self.registry.own_surface_query())?;
        if resolved.spec.name != cmd {
            return None;
        }
        resolved.inline_codegen_hook
    }

    /// Inline-compile a `[cmd arg ...]` command substitution.
    ///
    /// Handles `[expr {...}]` specially by parsing and inlining the
    /// expression body. Other commands are compiled as
    /// `push cmd; <args>; invokeStk1 N`.
    ///
    /// Multi-command scripts (containing `;` or newlines outside
    /// quotes/braces) fall back to runtime `EVAL_STK`.
    ///
    /// Which commands get a specialised inline emitter is registry
    /// data, not compiler code: the dispatch resolves the head word
    /// via [`Self::inline_cmd_subst_hook`] and matches the returned
    /// [`InlineCodegenHookId`]. The compiler keeps the per-variant
    /// emitters and their applicability guards (arity / shape /
    /// proc-context); a call whose guard fails — or whose hook this
    /// value-position dispatcher does not specialise (e.g.
    /// `Return` / `Break`, which only the catch-body dispatcher in
    /// `control_flow` emits inline) — falls back to the generic
    /// invoke.
    pub fn emit_inline_cmd_subst(&mut self, text: &str) {
        // Multi-command scripts (a `;`/newline separator outside quotes/braces)
        // fall back to runtime eval — checked *before* the `{*}` form below so a
        // body that has both (`[set y 1; list {*}$a]`) runs as two commands
        // rather than being mis-parsed as one expanded command.
        if has_command_separator(text) {
            let inner = if text.starts_with('[') && text.ends_with(']') {
                &text[1..text.len() - 1]
            } else {
                text
            };
            self.push_lit(&format!("{{{inner}}}"));
            self.emit(Op::EVAL_STK, vec![]);
            return;
        }
        // A `{*}`-expanded command substitution in value position compiles to the
        // `expandStart … expandStkTop N; invokeExpanded` form (tclsh's), leaving
        // the result on the stack (no trailing `pop`, unlike the statement form).
        if text.contains("{*}") {
            let parts = parse_cmd_parts_expand(text);
            if parts.iter().any(|(_, _, expand)| *expand) {
                self.emit_expanded_cmd_subst(&parts);
                return;
            }
        }

        let parts = parse_cmd_parts(text);
        if parts.is_empty() {
            self.push_lit("");
            return;
        }

        let cmd = &parts[0].0;
        let args = &parts[1..];

        // Registry-driven dispatch: the hook ID names the emitter, the
        // guards are each emitter's applicability conditions. A
        // subcommand-keyed hook (`InfoExists`, `DictGet`) only resolves
        // when the subcommand word matched exactly, so those arms need
        // no re-check of the subcommand text.
        match self.inline_cmd_subst_hook(cmd, args) {
            Some(InlineCodegenHookId::Expr) if args.len() == 1 => {
                let expr_body = &args[0].0;
                // Re-parsed under the compile's own dialect, exactly as the
                // lowering pass parses a statement-position `expr` — parsing
                // it dialect-blind here left a dialect-only operator
                // (`$x contains "a"`) unrecognised and pushed as a raw string
                // (issue #1435).
                let node = self.parse_compile_expr(expr_body);
                self.emit_expr(&node);
            }
            Some(InlineCodegenHookId::Incr) if (1..=2).contains(&args.len()) => {
                self.emit_inline_incr(args);
            }
            Some(InlineCodegenHookId::InfoExists) if args.len() == 2 => {
                self.emit_inline_info_exists(args);
            }
            Some(InlineCodegenHookId::String) if args.len() >= 2 => {
                self.emit_inline_string(args);
            }
            Some(InlineCodegenHookId::Lindex) if args.len() >= 2 => {
                self.emit_inline_lindex(args);
            }
            Some(InlineCodegenHookId::Lrange) if args.len() == 3 => {
                self.emit_inline_lrange(args);
            }
            Some(InlineCodegenHookId::Lreplace) if args.len() >= 3 => {
                self.emit_inline_lreplace(args);
            }
            Some(InlineCodegenHookId::Linsert) if args.len() >= 2 => {
                self.emit_inline_linsert(args);
            }
            Some(InlineCodegenHookId::Regexp) if args.len() >= 2 => {
                self.emit_inline_regexp(args);
            }
            Some(InlineCodegenHookId::List) if !args.is_empty() && !text.contains("{*}") => {
                self.used_inline_cmd_subst = true;
                for (a, b) in args {
                    self.emit_cmd_subst_arg(a, *b);
                }
                self.emit(Op::LIST, vec![Operand::Imm(bytecode_imm(args.len()))]);
            }
            Some(InlineCodegenHookId::Array) if args.len() >= 2 => {
                self.emit_inline_array(args);
            }
            Some(InlineCodegenHookId::DictGet) if args.len() >= 3 => {
                self.emit_inline_dict_get(args);
            }
            Some(InlineCodegenHookId::Catch) if self.is_proc && (1..=3).contains(&args.len()) => {
                let result_var = args.get(1).map(|(s, _)| s.as_str());
                if result_var.is_some_and(|v| v.starts_with("::")) {
                    self.used_inline_cmd_subst = false;
                    self.emit_generic_cmd_subst(cmd, args);
                } else {
                    self.used_inline_cmd_subst = true;
                    let body_text = &args[0].0;
                    let options_var = args.get(2).map(|(s, _)| s.as_str());
                    self.emit_catch_inline(body_text, result_var, options_var);
                }
            }
            _ => {
                self.used_inline_cmd_subst = false;
                self.emit_generic_cmd_subst(cmd, args);
            }
        }
    }

    /// Emit a `{*}`-expanded command substitution in value position:
    /// `expandStart`, each word (an expanded word followed by `expandStkTop N`
    /// where N is the running word count), then `invokeExpanded` — leaving the
    /// result on the stack. Mirrors [`Self::emit_expanded_call`] without the
    /// trailing `pop`.
    fn emit_expanded_cmd_subst(&mut self, parts: &[(String, bool, bool)]) {
        self.used_inline_cmd_subst = true;
        self.emit_comment(Op::EXPAND_START, vec![], "(expanded)");
        let mut word_count: u32 = 0;
        for (part, braced, expand) in parts {
            if *braced {
                // A braced expanded word splits its *list* elements without
                // substitution, so push it verbatim.
                self.push_lit_verbatim(part);
            } else {
                self.emit_cmd_subst_arg(part, false);
            }
            word_count += 1;
            if *expand {
                self.emit(
                    Op::EXPAND_STKTOP,
                    vec![Operand::Imm(
                        i32::try_from(word_count).expect("word count fits in i32"),
                    )],
                );
            }
        }
        self.emit_comment(Op::INVOKE_EXPANDED, vec![], "");
    }

    // -- Private inline helpers for emit_inline_cmd_subst --

    fn emit_inline_incr(&mut self, args: &[(String, bool)]) {
        let var_name = &args[0].0;
        if self.is_proc && !is_qualified(var_name) {
            let slot = bytecode_imm(self.lvt.intern(var_name));
            if args.len() == 1 {
                self.emit_comment(
                    Op::INCR_SCALAR1_IMM,
                    vec![Operand::Imm(slot), Operand::Imm(1)],
                    &format!("var \"{var_name}\""),
                );
            } else {
                let amt_str = &args[1].0;
                if let Some(amt) = self.parse_int_operand(amt_str) {
                    if (-128..=127).contains(&amt) {
                        self.emit_comment(
                            Op::INCR_SCALAR1_IMM,
                            vec![
                                Operand::Imm(slot),
                                Operand::Imm(
                                    i32::try_from(amt)
                                        .expect("incr literal fits in i32 after range check"),
                                ),
                            ],
                            &format!("var \"{var_name}\""),
                        );
                    } else {
                        self.push_lit(amt_str);
                        self.emit_comment(
                            Op::INCR_SCALAR1,
                            vec![Operand::Imm(slot)],
                            &format!("var \"{var_name}\""),
                        );
                    }
                } else {
                    let var_ref = amt_str.strip_prefix('$').unwrap_or(amt_str);
                    self.load_var(var_ref);
                    self.emit_comment(
                        Op::INCR_SCALAR1,
                        vec![Operand::Imm(slot)],
                        &format!("var \"{var_name}\""),
                    );
                }
            }
        } else {
            self.push_lit(var_name);
            if args.len() == 1 {
                self.emit(Op::INCR_STK_IMM, vec![Operand::Imm(1)]);
            } else {
                let amt_str = &args[1].0;
                // `INCR_STK_IMM` carries a 1-byte signed operand, so it
                // must be range-checked exactly like the proc-local
                // `INCR_SCALAR1_IMM` branch above. Without the check,
                // `[incr ::g 200]` overflowed the operand, and an amount
                // outside `i32` (e.g. `3000000000`) fell through to a
                // `load_var` of a variable *named* after the number — a
                // phantom-variable read. Parse as `i64` and fall back to the
                // full `INCR_STK` for anything outside the 1-byte range.
                if let Some(amt) = self.parse_int_operand(amt_str) {
                    if (-128..=127).contains(&amt) {
                        self.emit(
                            Op::INCR_STK_IMM,
                            vec![Operand::Imm(
                                i32::try_from(amt)
                                    .expect("incr literal fits in i32 after range check"),
                            )],
                        );
                    } else {
                        self.push_lit(amt_str);
                        self.emit(Op::INCR_STK, vec![]);
                    }
                } else {
                    let var_ref = amt_str.strip_prefix('$').unwrap_or(amt_str);
                    self.load_var(var_ref);
                    self.emit(Op::INCR_STK, vec![]);
                }
            }
        }
    }

    fn emit_inline_info_exists(&mut self, args: &[(String, bool)]) {
        self.used_inline_cmd_subst = true;
        let var_name = &args[1].0;
        if self.is_proc && !is_qualified(var_name) {
            let slot = bytecode_imm(self.lvt.intern(var_name));
            self.emit_comment(
                Op::EXIST_SCALAR,
                vec![Operand::Imm(slot)],
                &format!("var \"{var_name}\""),
            );
            self.emit(Op::NOP, vec![]);
        } else {
            self.push_lit(var_name);
            self.emit(Op::EXIST_STK, vec![]);
        }
    }

    /// Emit the `string equal {-nocase}? a b` / `string compare
    /// {-nocase|-length}? ...` forms via `INVOKE_REPLACE` against
    /// the FQN sub-emitter.  `flag_args` is the per-form prefix
    /// pushed before the operand args.  `total_argc` is the total
    /// argument count including the operand args; helper computes
    /// `INVOKE_REPLACE 0 1` operands as `(total_argc, 2)`.
    fn emit_inline_string_invoke_replace(
        &mut self,
        prev_inline: bool,
        target_subcmd: &str,
        flag_args: &[&str],
        operand_args: &[(String, bool)],
    ) {
        self.used_inline_cmd_subst = prev_inline;
        let sc_end = self.fresh_label("subcmd_end");
        self.emit_comment(
            Op::START_CMD,
            vec![Operand::Label(sc_end.clone()), Operand::Imm(1)],
            "",
        );
        self.push_lit("string");
        self.push_lit(target_subcmd);
        for f in flag_args {
            self.push_lit(f);
        }
        for (a, b) in operand_args {
            self.emit_cmd_subst_arg(a, *b);
        }
        self.push_lit(&format!("::tcl::string::{target_subcmd}"));
        let argc = bytecode_imm(2 + flag_args.len() + operand_args.len());
        self.emit(
            Op::INVOKE_REPLACE,
            vec![Operand::Imm(argc), Operand::Imm(2)],
        );
        self.place_label(&sc_end);
        self.seen_generic_invoke = true;
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
        self.emit_cmd_subst_arg(&sargs[0].0, sargs[0].1);
        self.emit_cmd_subst_arg(&sargs[1].0, sargs[1].1);
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
        let fqn = format!("::tcl::string::{subcmd}");
        self.push_lit(&fqn);
        for (a, b) in sargs {
            self.emit_cmd_subst_arg(a, *b);
        }
        let argc = bytecode_imm(1 + sargs.len());
        let invoke_op = if argc < 256 {
            Op::INVOKE_STK1
        } else {
            Op::INVOKE_STK4
        };
        self.emit(invoke_op, vec![Operand::Imm(argc)]);
        self.place_label(&sc_end);
        self.seen_generic_invoke = true;
    }

    fn emit_inline_string(&mut self, args: &[(String, bool)]) {
        let subcmd = &args[0].0;
        let sargs = &args[1..];
        let prev_inline = self.used_inline_cmd_subst;
        self.used_inline_cmd_subst = true;

        match subcmd.as_str() {
            "index" if sargs.len() == 2 => {
                self.emit_cmd_subst_arg(&sargs[0].0, sargs[0].1);
                self.emit_cmd_subst_arg(&sargs[1].0, sargs[1].1);
                self.emit(Op::STR_INDEX, vec![]);
            }
            "range" if sargs.len() == 3 => {
                self.emit_cmd_subst_arg(&sargs[0].0, sargs[0].1);
                let start_idx = parse_tcl_index(&sargs[1].0);
                let end_idx = parse_tcl_index(&sargs[2].0);
                if let (Some(s), Some(e)) = (start_idx, end_idx)
                    && imm_index_ok(s)
                    && imm_index_ok(e)
                {
                    self.emit(Op::STR_RANGE_IMM, vec![Operand::Imm(s), Operand::Imm(e)]);
                } else {
                    self.emit_cmd_subst_arg(&sargs[1].0, sargs[1].1);
                    self.emit_cmd_subst_arg(&sargs[2].0, sargs[2].1);
                    self.emit(Op::STR_RANGE, vec![]);
                }
            }
            "equal" if sargs.len() == 2 => {
                self.emit_inline_string_2arg_op(prev_inline, sargs, Op::STR_EQ);
            }
            "equal" if sargs.len() == 3 && sargs[0].0 == "-nocase" => {
                self.emit_inline_string_invoke_replace(
                    prev_inline,
                    "equal",
                    &["-nocase"],
                    &sargs[1..],
                );
            }
            "compare" if sargs.len() == 2 => {
                self.emit_inline_string_2arg_op(prev_inline, sargs, Op::STR_CMP);
            }
            "compare" if sargs.len() == 3 && sargs[0].0 == "-nocase" => {
                self.emit_inline_string_invoke_replace(
                    prev_inline,
                    "compare",
                    &["-nocase"],
                    &sargs[1..],
                );
            }
            "compare" if sargs.len() == 4 && sargs[0].0 == "-length" => {
                self.emit_inline_string_invoke_replace(
                    prev_inline,
                    "compare",
                    &["-length"],
                    &sargs[1..],
                );
            }
            "replace" if sargs.len() == 4 => {
                self.emit_inline_string_replace(sargs);
            }
            "length" if sargs.len() == 1 => {
                self.emit_cmd_subst_arg(&sargs[0].0, sargs[0].1);
                self.emit(Op::STR_LEN, vec![]);
            }
            "is" if sargs.len() >= 2 => {
                self.emit_inline_string_is(sargs);
            }
            _ => {
                self.emit_inline_string_fqn_invoke(prev_inline, subcmd, sargs);
            }
        }
    }

    fn emit_inline_string_replace(&mut self, sargs: &[(String, bool)]) {
        let first_lit = &sargs[1].0;
        let last_lit = &sargs[2].0;
        if first_lit == "0"
            && let Ok(last_int) = last_lit.parse::<i32>()
            && last_int >= 0
            // `last_int + 1` is the start index; guard the i32::MAX
            // overflow (which would wrap to a negative garbage index) with a
            // checked add and fall back to `strreplace` when it doesn't fit.
            && let Some(start) = last_int.checked_add(1)
        {
            self.emit_cmd_subst_arg(&sargs[0].0, sargs[0].1);
            self.emit_cmd_subst_arg(&sargs[3].0, sargs[3].1);
            self.emit(Op::REVERSE, vec![Operand::Imm(2)]);
            self.emit(
                Op::STR_RANGE_IMM,
                vec![Operand::Imm(start), Operand::Imm(INDEX_END)],
            );
            self.emit(Op::STR_CONCAT1, vec![Operand::Imm(2)]);
            return;
        }
        // Fallback: strreplace
        self.emit_cmd_subst_arg(&sargs[0].0, sargs[0].1);
        self.emit_cmd_subst_arg(&sargs[1].0, sargs[1].1);
        self.emit_cmd_subst_arg(&sargs[2].0, sargs[2].1);
        self.emit_cmd_subst_arg(&sargs[3].0, sargs[3].1);
        self.emit(Op::STR_REPLACE, vec![]);
    }

    fn emit_inline_string_is(&mut self, sargs: &[(String, bool)]) {
        // Only two shapes can be specialised inline: `CLASS value` and
        // `CLASS -strict value`. Anything else carries an option this path does
        // not model — above all `-failindex var`, which has to *write a
        // variable*. The dispatch that reaches here gates on arity alone, and
        // this function used to take `sargs.last()` as the value and ignore
        // everything before it, so `string is integer -failindex fi 1.5`
        // computed the correct answer and silently never wrote `fi`
        // (tclsh writes 1). A 2-word form whose second word is `-strict` is a
        // missing-value arity error, which the generic path reports properly.
        let specialisable = match sargs.len() {
            2 => sargs[1].0 != "-strict",
            3 => sargs[1].0 == "-strict",
            _ => false,
        };
        if !specialisable {
            self.emit_string_is_generic(sargs);
            return;
        }
        let class_name = &sargs[0].0;
        // Detect -strict flag and value
        let (strict, val_arg) = if sargs.len() == 3 && sargs[1].0 == "-strict" {
            (true, &sargs[2])
        } else {
            (false, &sargs[sargs.len() - 1])
        };

        if let Some(class_id) = str_class_id(class_name) {
            if strict {
                // `STR_CLASS` reports the empty string as a member, so it cannot
                // honour `-strict` (under which the empty string is a non-member
                // for character classes); defer to the generic command.
                self.emit_string_is_generic(sargs);
            } else {
                self.emit_cmd_subst_arg(&val_arg.0, val_arg.1);
                self.emit(Op::STR_CLASS, vec![Operand::Imm(i32::from(class_id))]);
            }
        } else if class_name == "integer" {
            self.emit_cmd_subst_arg(&val_arg.0, val_arg.1);
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
            self.emit_cmd_subst_arg(&val_arg.0, val_arg.1);
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
            self.emit_cmd_subst_arg(&val_arg.0, val_arg.1);
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
            self.emit_string_is_generic(sargs);
        }
    }

    /// `string is CLASS ?args…?` via the generic command path, **keeping** the
    /// `is` subcommand word. Used when a `string is` form cannot be specialised
    /// inline (unknown class, `-strict` char class, extra options). The previous
    /// fallback dropped `is` (it prefixed `string` then sliced it back off),
    /// producing the invalid `string CLASS …`.
    fn emit_string_is_generic(&mut self, sargs: &[(String, bool)]) {
        self.used_inline_cmd_subst = false;
        let mut all_args = vec![("is".to_owned(), false)];
        all_args.extend_from_slice(sargs);
        self.emit_generic_cmd_subst("string", &all_args);
    }

    fn emit_inline_lindex(&mut self, args: &[(String, bool)]) {
        self.used_inline_cmd_subst = true;
        self.emit_cmd_subst_arg(&args[0].0, args[0].1); // list
        if args.len() == 2 {
            let idx = parse_tcl_index(&args[1].0);
            if let Some(idx) = idx {
                if idx >= 0 || idx <= INDEX_END {
                    self.emit(Op::LIST_INDEX_IMM, vec![Operand::Imm(idx)]);
                } else {
                    self.emit_cmd_subst_arg(&args[1].0, args[1].1);
                    self.emit(Op::LIST_INDEX, vec![]);
                }
            } else {
                self.emit_cmd_subst_arg(&args[1].0, args[1].1);
                self.emit(Op::LIST_INDEX, vec![]);
            }
        } else {
            for a in &args[1..] {
                self.emit_cmd_subst_arg(&a.0, a.1);
            }
            self.emit(
                Op::LINDEX_MULTI,
                vec![Operand::Imm(bytecode_imm(args.len()))],
            );
        }
    }

    fn emit_inline_lrange(&mut self, args: &[(String, bool)]) {
        // Decide between LIST_RANGE_IMM and the generic fallback
        // *before* emitting any arguments — otherwise the fallback
        // would push the list a second time, leaving an extra value
        // on the stack.
        let start_idx = parse_tcl_index(&args[1].0);
        let end_idx = parse_tcl_index(&args[2].0);
        if let (Some(s), Some(e)) = (start_idx, end_idx)
            && imm_index_ok(s)
            && imm_index_ok(e)
        {
            self.used_inline_cmd_subst = true;
            self.emit_cmd_subst_arg(&args[0].0, args[0].1);
            self.emit(Op::LIST_RANGE_IMM, vec![Operand::Imm(s), Operand::Imm(e)]);
        } else {
            self.used_inline_cmd_subst = false;
            self.emit_generic_cmd_subst("lrange", args);
        }
    }

    fn emit_inline_lreplace(&mut self, args: &[(String, bool)]) {
        self.used_inline_cmd_subst = true;
        self.emit_cmd_subst_arg(&args[0].0, args[0].1);
        for a in &args[1..] {
            self.emit_cmd_subst_arg(&a.0, a.1);
        }
        self.emit(
            Op::LREPLACE4,
            vec![Operand::Imm(bytecode_imm(args.len())), Operand::Imm(1)],
        );
    }

    fn emit_inline_linsert(&mut self, args: &[(String, bool)]) {
        self.used_inline_cmd_subst = true;
        self.emit_cmd_subst_arg(&args[0].0, args[0].1);
        for a in &args[1..] {
            self.emit_cmd_subst_arg(&a.0, a.1);
        }
        self.emit(
            Op::LREPLACE4,
            vec![Operand::Imm(bytecode_imm(args.len())), Operand::Imm(2)],
        );
    }

    fn emit_inline_regexp(&mut self, args: &[(String, bool)]) {
        let mut rargs: Vec<&(String, bool)> = args.iter().collect();
        let mut nocase = false;
        if !rargs.is_empty() && rargs[0].0 == "-nocase" {
            nocase = true;
            rargs.remove(0);
        }
        if !rargs.is_empty() && rargs[0].0 == "--" {
            rargs.remove(0);
        }
        if rargs.len() == 2 && nocase {
            if let Some(glob) = regexp_to_glob(&rargs[0].0) {
                self.used_inline_cmd_subst = true;
                self.push_lit(&glob);
                self.emit_cmd_subst_arg(&rargs[1].0, rargs[1].1);
                self.emit(Op::STR_MATCH, vec![Operand::Imm(1)]);
            } else {
                self.used_inline_cmd_subst = false;
                self.emit_generic_cmd_subst("regexp", args);
            }
        } else if rargs.len() == 2 && !nocase {
            // Push only the cleaned pattern + subject (the `--` / `-nocase`
            // option words are consumed at compile time); the `REGEXP` opcode
            // pops exactly those two. Operand is the compile flags: tclsh's
            // default `TCL_REG_ADVANCED` (3). `-nocase` is handled by the glob
            // path above, so the NOCASE bit is never set here.
            self.used_inline_cmd_subst = true;
            for arg in &rargs {
                self.emit_cmd_subst_arg(&arg.0, arg.1);
            }
            self.emit(Op::REGEXP, vec![Operand::Imm(3)]);
        } else {
            self.used_inline_cmd_subst = false;
            self.emit_generic_cmd_subst("regexp", args);
        }
    }

    fn emit_inline_array(&mut self, args: &[(String, bool)]) {
        let sub = &args[0].0;
        let rest = &args[1..];
        if sub == "exists" && rest.len() == 1 && self.is_proc && !is_qualified(&rest[0].0) {
            self.used_inline_cmd_subst = true;
            let slot = bytecode_imm(self.lvt.intern(&rest[0].0));
            self.emit_comment(
                Op::ARRAY_EXISTS_IMM,
                vec![Operand::Imm(slot)],
                &format!("var \"{}\"", rest[0].0),
            );
        } else if (sub == "names" || sub == "size") && !rest.is_empty() {
            let sc_end = self.fresh_label("subcmd_end");
            self.emit_comment(
                Op::START_CMD,
                vec![Operand::Label(sc_end.clone()), Operand::Imm(1)],
                "",
            );
            let fqn = format!("::tcl::array::{sub}");
            self.push_lit(&fqn);
            for (a, b) in rest {
                self.emit_cmd_subst_arg(a, *b);
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
            self.emit_generic_cmd_subst("array", args);
        }
    }

    fn emit_inline_dict_get(&mut self, args: &[(String, bool)]) {
        self.used_inline_cmd_subst = true;
        let dict_args = &args[1..]; // skip "get"
        self.emit_cmd_subst_arg(&dict_args[0].0, dict_args[0].1); // dict value
        let keys = &dict_args[1..];
        for (k, b) in keys {
            self.emit_cmd_subst_arg(k, *b);
        }
        self.emit(Op::DICT_GET, vec![Operand::Imm(bytecode_imm(keys.len()))]);
    }
}

// Tests

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_registry::CommandRegistry;

    /// A value-position `[expr {…}]` is re-parsed here, and until issue #1435
    /// it was re-parsed dialect-blind: an iRules word operator lexed as a
    /// function name, the parse fell back to `ExprNode::Raw`, and codegen
    /// pushed the source text for a second dialect-blind parse in the VM —
    /// which returned the text itself rather than evaluating the operator.
    #[test]
    fn inline_expr_subst_parses_under_the_compile_dialect() {
        let profile = tcl_dialect::DialectProfile::irules();
        let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
        let mut ctx = CodegenCtx::new(true, &["x"], registry);
        ctx.dialect = Some(
            tcl_registry::model::ingress::resolve_environment(profile.name).analyser_profile(),
        );
        ctx.emit_inline_cmd_subst("[expr {$x contains \"a\"}]");
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::IRULE_CONTAINS), "{ops:?}");
    }

    /// The same site's release axis: an operator the target release lacks is
    /// not specialised at all, so the VM raises the interpreter's own
    /// diagnostic instead of executing an opcode 8.4 has no grammar for.
    #[test]
    fn inline_expr_subst_respects_the_target_release() {
        let registry = CommandRegistry::build_default();

        let mut old = CodegenCtx::new(true, &[], &registry);
        old.dialect =
            Some(tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile());
        old.emit_inline_cmd_subst("[expr {2 ** 3}]");
        let ops: Vec<Op> = old.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::EXPR_STK), "{ops:?}");
        assert!(old.literals.entries().iter().any(|l| l == "2 ** 3"));

        let mut modern = CodegenCtx::new(true, &[], &registry);
        modern.dialect =
            Some(tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile());
        modern.emit_inline_cmd_subst("[expr {2 ** 3}]");
        let ops: Vec<Op> = modern.instructions.iter().map(|i| i.op).collect();
        assert!(!ops.contains(&Op::EXPR_STK), "{ops:?}");
        assert!(modern.literals.entries().iter().any(|l| l == "8"));
    }

    // -- unroll_nested_set --

    #[test]
    fn unroll_simple() {
        let r = unroll_nested_set("[set y [set z 42]]").unwrap();
        assert_eq!(r, vec!["y", "z", "42"]);
    }

    #[test]
    fn unroll_not_nested() {
        assert!(unroll_nested_set("hello").is_none());
    }

    // -- is_pure_cmd_subst --

    #[test]
    fn pure_cmd_subst_simple() {
        assert!(is_pure_cmd_subst("[expr {1+2}]"));
    }

    #[test]
    fn pure_cmd_subst_nested() {
        assert!(is_pure_cmd_subst("[set x [set y 1]]"));
    }

    #[test]
    fn pure_cmd_subst_not() {
        assert!(!is_pure_cmd_subst("hello"));
        assert!(!is_pure_cmd_subst("[a] [b]"));
    }

    // -- has_command_separator --

    #[test]
    fn separator_semicolon() {
        assert!(has_command_separator("set x 1; set y 2"));
    }

    #[test]
    fn separator_newline() {
        assert!(has_command_separator("set x 1\nset y 2"));
    }

    #[test]
    fn separator_in_braces_ignored() {
        assert!(!has_command_separator("{set x 1; set y 2}"));
    }

    #[test]
    fn separator_in_quotes_ignored() {
        assert!(!has_command_separator("\"set x 1; set y 2\""));
    }

    #[test]
    fn no_separator() {
        assert!(!has_command_separator("set x 1"));
    }

    // -- parse_cmd_parts --

    #[test]
    fn parse_simple_cmd() {
        let parts = parse_cmd_parts("[set x 42]");
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], ("set".into(), false));
        assert_eq!(parts[1], ("x".into(), false));
        assert_eq!(parts[2], ("42".into(), false));
    }

    #[test]
    fn parse_braced_arg() {
        let parts = parse_cmd_parts("[expr {1+2}]");
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], ("expr".into(), false));
        assert_eq!(parts[1], ("1+2".into(), true));
    }

    #[test]
    fn parse_quoted_arg() {
        let parts = parse_cmd_parts("[puts \"hello world\"]");
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], ("puts".into(), false));
        assert_eq!(parts[1], ("hello world".into(), false));
    }

    #[test]
    fn parse_nested_cmd() {
        let parts = parse_cmd_parts("[set x [expr {1+2}]]");
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], ("set".into(), false));
        assert_eq!(parts[1], ("x".into(), false));
        assert_eq!(parts[2], ("[expr {1+2}]".into(), false));
    }

    // -- emit_cmd_subst_arg --

    #[test]
    fn emit_arg_literal() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        ctx.emit_cmd_subst_arg("hello", false);
        assert_eq!(ctx.instructions[0].op, Op::PUSH1);
    }

    #[test]
    fn emit_arg_var_ref() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &["x"], &registry);
        ctx.emit_cmd_subst_arg("${x}", false);
        assert_eq!(ctx.instructions[0].op, Op::LOAD_SCALAR1);
    }

    // -- emit_generic_cmd_subst --

    #[test]
    fn emit_generic_simple() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        ctx.emit_generic_cmd_subst("puts", &[("hello".into(), false)]);
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert_eq!(ops, vec![Op::PUSH1, Op::PUSH1, Op::INVOKE_STK1]);
    }

    // -- emit_inline_cmd_subst --

    #[test]
    fn inline_expr() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        // A variable operand keeps the ADD opcode; a constant `1+2` would be
        // folded to a single push by the codegen-time expr const-folder.
        ctx.emit_inline_cmd_subst("[expr {$x+2}]");
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::ADD));
    }

    #[test]
    fn inline_incr_proc() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &["x"], &registry);
        ctx.emit_inline_cmd_subst("[incr x]");
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::INCR_SCALAR1_IMM));
    }

    #[test]
    fn inline_string_length() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.emit_inline_cmd_subst("[string length ${x}]");
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::STR_LEN));
    }

    #[test]
    fn inline_list() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.emit_inline_cmd_subst("[list a b c]");
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::LIST));
    }

    #[test]
    fn inline_multicommand_falls_back() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.emit_inline_cmd_subst("[set x 1; set y 2]");
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::EVAL_STK));
    }

    // -- regression: label reconstruction in string equal/compare --

    /// `string equal` in non-proc context with a nested command
    /// substitution in one arg. The nested substitution allocates
    /// labels (via `fresh_label`), so if the fast-path reconstructed
    /// the end-label name from `label_counter - 1` it would resolve
    /// to a different label.
    #[test]
    fn inline_string_equal_nested_cmd_label_resolves() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        // Nested `[expr ...]` does not allocate labels, but nested
        // `[array names ...]` wraps in a startCommand, which does.
        ctx.emit_inline_cmd_subst("[string equal [array names a] foo]");
        // Build expected: the startCommand end label should resolve
        // to the `STR_EQ` position (or later), not a dangling index.
        let has_str_eq = ctx.instructions.iter().any(|i| i.op == Op::STR_EQ);
        assert!(has_str_eq, "expected STR_EQ in output");
        // All Label operands must resolve (i.e. exist in label_positions).
        for instr in &ctx.instructions {
            for op in &instr.operands {
                if let Operand::Label(l) = op {
                    assert!(
                        ctx.label_positions.contains_key(l),
                        "unresolved label {l:?} in {:?}",
                        instr.op
                    );
                }
            }
        }
    }

    #[test]
    fn inline_string_compare_nested_cmd_label_resolves() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        ctx.emit_inline_cmd_subst("[string compare [array names a] foo]");
        let has_str_cmp = ctx.instructions.iter().any(|i| i.op == Op::STR_CMP);
        assert!(has_str_cmp, "expected STR_CMP in output");
        for instr in &ctx.instructions {
            for op in &instr.operands {
                if let Operand::Label(l) = op {
                    assert!(
                        ctx.label_positions.contains_key(l),
                        "unresolved label {l:?} in {:?}",
                        instr.op
                    );
                }
            }
        }
    }

    // -- specialised value-emission paths --

    #[test]
    fn try_list_expand_concat_matches_two_vars() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        assert!(ctx.try_list_expand_concat("[list {*}$a {*}$b]"));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        // push "a"; loadStk; push "b"; loadStk; listConcat
        assert_eq!(
            ops,
            vec![
                Op::PUSH1,
                Op::LOAD_STK,
                Op::PUSH1,
                Op::LOAD_STK,
                Op::LIST_CONCAT,
            ]
        );
    }

    #[test]
    fn try_list_expand_concat_rejects_single_var() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        assert!(!ctx.try_list_expand_concat("[list {*}$a]"));
        assert!(ctx.instructions.is_empty());
    }

    #[test]
    fn try_list_expand_concat_rejects_three_vars() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        assert!(!ctx.try_list_expand_concat("[list {*}$a {*}$b {*}$c]"));
    }

    #[test]
    fn try_list_expand_concat_rejects_non_expanded_arg() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        // Literal first arg without {*} prefix — falls back to generic path.
        assert!(!ctx.try_list_expand_concat("[list a {*}$b]"));
    }

    #[test]
    fn try_inline_list_without_target_emits_break_as_literal() {
        // When `[list ... [break] ...]` appears without
        // a loop target in scope, the pattern still claims the value
        // and emits `[break]` as a literal list element. The generic
        // fallback is never reached.
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        assert!(ctx.try_inline_list_with_break_continue("[list a [break] c]"));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        // No JUMP4 since no break target.
        assert!(!ops.contains(&Op::JUMP4));
        // Still emits LIST N at the end.
        assert!(ops.contains(&Op::LIST));
    }

    #[test]
    fn try_inline_list_with_break_emits_jump_to_target() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.break_target = Some("loop_break_1".into());
        assert!(ctx.try_inline_list_with_break_continue("[list a [break] c]"));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        // push "a"; startCommand; pop; jump4 break_target; push "c"; list 3
        assert!(ops.contains(&Op::JUMP4), "expected JUMP4, got {ops:?}");
        assert!(
            ops.contains(&Op::START_CMD),
            "expected START_CMD, got {ops:?}"
        );
        assert!(ops.contains(&Op::LIST), "expected LIST, got {ops:?}");
    }

    #[test]
    fn try_inline_list_without_break_returns_false() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.break_target = Some("loop_break_1".into());
        // No [break]/[continue] inside — should not match.
        assert!(!ctx.try_inline_list_with_break_continue("[list a b c]"));
    }

    /// `lrange` with non-literal indices must not push the list arg
    /// twice when falling back to the generic invoke path.
    #[test]
    fn inline_lrange_variable_indices_no_double_push() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &["lst", "a", "b"], &registry);
        ctx.emit_inline_cmd_subst("[lrange ${lst} ${a} ${b}]");
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        // Should be: push "lrange"; load lst; load a; load b; invokeStk1 4
        // → 1 push, 3 loads, 1 invoke. NOT: load lst; push "lrange"; ...
        let load_count = ops
            .iter()
            .filter(|o| matches!(o, Op::LOAD_SCALAR1 | Op::LOAD_SCALAR4))
            .count();
        assert_eq!(load_count, 3, "expected 3 var loads, got {ops:?}");
        let invoke_count = ops.iter().filter(|o| **o == Op::INVOKE_STK1).count();
        assert_eq!(invoke_count, 1, "expected one invokeStk1, got {ops:?}");
    }

    #[test]
    fn inline_lrange_end_plus_n_falls_back_to_non_imm() {
        // `end+1` encodes as INDEX_END+1, a garbage immediate, so the
        // emitter must not use LIST_RANGE_IMM — it falls back to the generic
        // path.
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &["lst"], &registry);
        ctx.emit_inline_cmd_subst("[lrange ${lst} 0 end+1]");
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(
            !ops.contains(&Op::LIST_RANGE_IMM),
            "end+1 must not emit LIST_RANGE_IMM, got {ops:?}",
        );
    }

    #[test]
    fn inline_lrange_end_uses_imm() {
        // A plain `end` index is valid and still takes the fast immediate path.
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &["lst"], &registry);
        ctx.emit_inline_cmd_subst("[lrange ${lst} 0 end]");
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(
            ops.contains(&Op::LIST_RANGE_IMM),
            "end should use LIST_RANGE_IMM, got {ops:?}",
        );
    }

    // -- registry drift: inline codegen hook stamping --

    /// The registry-stamped inline-hook set must equal the command set
    /// the retired hardcoded `match cmd.as_str()` dispatch (plus the
    /// catch-body dispatch in `control_flow`) special-cased — so a
    /// future spec change is a conscious decision, not a silent
    /// bytecode change.
    #[test]
    fn registry_inline_hook_stamping_matches_previous_hardcoded_dispatch() {
        use tcl_registry::hooks::InlineCodegenHookId as H;
        let registry = CommandRegistry::build_default();
        let expected: &[(&str, H)] = &[
            ("expr", H::Expr),
            ("incr", H::Incr),
            ("string", H::String),
            ("lindex", H::Lindex),
            ("lrange", H::Lrange),
            ("lreplace", H::Lreplace),
            ("linsert", H::Linsert),
            ("regexp", H::Regexp),
            ("list", H::List),
            ("array", H::Array),
            ("catch", H::Catch),
            ("return", H::Return),
            ("error", H::Error),
            ("break", H::Break),
            ("continue", H::Continue),
            ("try", H::Try),
        ];
        for (name, hook) in expected {
            assert_eq!(
                registry.get(name).and_then(|s| s.inline_codegen_hook),
                Some(*hook),
                "{name} must carry the {hook:?} inline hook"
            );
        }
        // Subcommand-keyed hooks.
        assert_eq!(
            registry
                .get("info")
                .and_then(|s| s.subcommand("exists"))
                .and_then(|s| s.inline_codegen_hook),
            Some(H::InfoExists)
        );
        assert_eq!(
            registry
                .get("dict")
                .and_then(|s| s.subcommand("get"))
                .and_then(|s| s.inline_codegen_hook),
            Some(H::DictGet)
        );
        // …and nothing else: stamping an inline hook on any further
        // spec or subcommand must fail here first.
        let expected_cmds: std::collections::HashSet<&str> =
            expected.iter().map(|(n, _)| *n).collect();
        let names: Vec<String> = registry.command_names().map(str::to_owned).collect();
        for name in &names {
            let Some(spec) = registry.get(name) else {
                continue;
            };
            if spec.inline_codegen_hook.is_some() {
                assert!(
                    expected_cmds.contains(name.as_str()),
                    "unexpected command-level inline hook on {name}"
                );
            }
            for sub in spec.subcommands {
                if sub.inline_codegen_hook.is_some() {
                    assert!(
                        (name == "info" && sub.name == "exists")
                            || (name == "dict" && sub.name == "get"),
                        "unexpected subcommand-level inline hook on {name} {}",
                        sub.name
                    );
                }
            }
        }
    }

    /// A `::`-qualified head word keeps the generic-invoke path — the
    /// retired dispatch keyed on the raw word, so `[::expr …]` never
    /// took the inline expression emitter and must not start to.
    #[test]
    fn qualified_head_word_stays_generic() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.emit_inline_cmd_subst("[::expr {$x+2}]");
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(!ops.contains(&Op::ADD), "qualified ::expr must not inline");
        assert!(
            ops.contains(&Op::INVOKE_STK1),
            "qualified ::expr must invoke generically, got {ops:?}"
        );
    }

    /// Hooks the value-position dispatcher does not specialise
    /// (`Break` is catch-body-only) fall to the generic invoke, as the
    /// retired dispatch did for `[break]`.
    #[test]
    fn catch_body_only_hooks_stay_generic_in_value_position() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.emit_inline_cmd_subst("[break]");
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(
            !ops.contains(&Op::BREAK),
            "no inline break in value position"
        );
        assert!(ops.contains(&Op::INVOKE_STK1), "generic invoke expected");
    }
}
