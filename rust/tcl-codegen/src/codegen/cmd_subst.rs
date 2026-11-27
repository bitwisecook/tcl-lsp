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

use tcl_compiler::ir::CommandTokens;
use tcl_compiler::registry_invocation::compiled_local_name_value;

use super::helpers::{SubstPart, parse_subst_template};
use super::values::split_array_ref;
use super::{CodegenCtx, Op, Operand, bytecode_imm};

mod generic_invoke;
mod index_words;
mod inline_collections;
mod inline_hooks;
mod inline_introspection;
mod inline_string;
mod native_calls;

/// Whether a word is a literal the compiler reads as written: braced, or
/// with nothing to substitute (C's `TCL_TOKEN_SIMPLE_WORD`).
fn is_literal_word((word, braced): &(String, bool)) -> bool {
    *braced || !word.contains(['$', '[', '\\'])
}

// Free functions — pure parsing, no emission state needed

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

/// Standalone default-grammar query for one whole command substitution.
#[must_use]
pub fn is_pure_cmd_subst(value: &str) -> bool {
    // dialect-drift-ok: standalone default-grammar helper for callers with no document (tests, tools); document paths use the _with_config form
    is_pure_cmd_subst_with_config(value, tcl_lexer::LexerConfig::default())
}

pub(super) fn is_pure_cmd_subst_with_config(value: &str, config: tcl_lexer::LexerConfig) -> bool {
    tcl_lexer::word_parts::is_whole_command_substitution(value.as_bytes(), config)
}

/// Standalone default-grammar query for a script command terminator.
#[must_use]
pub fn has_command_separator(text: &str) -> bool {
    // dialect-drift-ok: standalone default-grammar helper for callers with no document (tests, tools); document paths use the _with_config form
    has_command_separator_with_config(text, tcl_lexer::LexerConfig::default())
}

pub(super) fn has_command_separator_with_config(
    text: &str,
    config: tcl_lexer::LexerConfig,
) -> bool {
    let Ok(end) = u32::try_from(text.len()) else {
        return true;
    };
    let region = if is_pure_cmd_subst_with_config(text, config) {
        tcl_lexer::Span::new(1, end - 1)
    } else {
        tcl_lexer::Span::new(0, end)
    };
    let image = tcl_lexer::SourceImage::document(text);
    // Body lexical purpose prevents a local leading BOM from becoming a prologue.
    let config = tcl_lexer::SourceSegmentationPurpose::NestedScript.config(config);
    tcl_lexer::source_region_tokens_in(&image, region, config).map_or(true, |tokens| {
        tokens
            .iter()
            .any(|token| token.kind == tcl_lexer::TokenType::Eol && !token.span.is_empty())
    })
}

/// Whether a script contains at most one complete, unexpanded command.
#[must_use]
pub fn is_single_command_body(body: &str, config: tcl_lexer::LexerConfig) -> bool {
    !has_command_separator_with_config(body, config)
        && tcl_compiler::word_subst::single_command_source_parts(
            body,
            config,
            tcl_compiler::word_subst::SingleCommandSourcePurpose::Script,
        )
        .is_some_and(|parts| parts.iter().all(|part| !part.expand))
}

/// Default Tcl lexical projection for standalone callers.
#[must_use]
pub fn parse_cmd_parts(text: &str) -> Vec<(String, bool)> {
    // dialect-drift-ok: standalone default-grammar helper for callers with no document (tests, tools); document paths use the _with_config form
    parse_cmd_parts_with_config(text.trim(), tcl_lexer::LexerConfig::default()).unwrap_or_default()
}

pub(super) fn parse_cmd_parts_with_config(
    text: &str,
    config: tcl_lexer::LexerConfig,
) -> Option<Vec<(String, bool)>> {
    let parts = tcl_compiler::word_subst::single_command_source_parts(
        text,
        config,
        tcl_compiler::word_subst::SingleCommandSourcePurpose::CommandSubstitution,
    )?;
    if parts.iter().any(|part| part.expand) {
        return None;
    }
    Some(
        parts
            .into_iter()
            .map(|part| (part.text, part.braced))
            .collect(),
    )
}

pub(super) fn parse_script_parts_with_config(
    text: &str,
    config: tcl_lexer::LexerConfig,
) -> Option<Vec<(String, bool)>> {
    let parts = tcl_compiler::word_subst::single_command_source_parts(
        text,
        config,
        tcl_compiler::word_subst::SingleCommandSourcePurpose::Script,
    )?;
    if parts.iter().any(|part| part.expand) {
        return None;
    }
    Some(
        parts
            .into_iter()
            .map(|part| (part.text, part.braced))
            .collect(),
    )
}

pub(super) fn parse_script_parts_expand_with_config(
    text: &str,
    config: tcl_lexer::LexerConfig,
) -> Option<Vec<(String, bool, bool)>> {
    Some(
        tcl_compiler::word_subst::single_command_source_parts(
            text,
            config,
            tcl_compiler::word_subst::SingleCommandSourcePurpose::Script,
        )?
        .into_iter()
        .map(|part| (part.text, part.braced, part.expand))
        .collect(),
    )
}

/// Whether `args` is one `expr` word that is its own expression text: braced,
/// or free of substitutions. Any other word is substituted first and its value
/// is what `expr` evaluates (C's `TclCompileExprWords`), so `[expr $c]` with
/// `c` set to `1+2` is `3`, not the text of `c`.
pub(super) fn lone_spelled_expression(args: &[(String, bool)]) -> bool {
    matches!(args, [(word, braced)]
        if *braced || !(tcl_syntax::naming::word_is_dynamic(word, false) || word.contains('\\')))
}

/// Default Tcl lexical projection preserving expansion markers.
#[must_use]
pub fn parse_cmd_parts_expand(text: &str) -> Vec<(String, bool, bool)> {
    // dialect-drift-ok: standalone default-grammar helper for callers with no document (tests, tools); document paths use the _with_config form
    parse_cmd_parts_expand_with_config(text.trim(), tcl_lexer::LexerConfig::default())
        .unwrap_or_default()
}

pub(super) fn parse_cmd_parts_expand_with_config(
    text: &str,
    config: tcl_lexer::LexerConfig,
) -> Option<Vec<(String, bool, bool)>> {
    Some(
        tcl_compiler::word_subst::single_command_source_parts(
            text,
            config,
            tcl_compiler::word_subst::SingleCommandSourcePurpose::CommandSubstitution,
        )?
        .into_iter()
        .map(|part| (part.text, part.braced, part.expand))
        .collect(),
    )
}

/// The words of a command once the parser expands its `{*}` words, as
/// `Tcl_ParseCommand` expands a word of literal text whose list elements are
/// all literal into a word per element (an empty list into none). `None`
/// where a `{*}` word is left to expand at run time, or an element or braced
/// word would read as a substitution where a word is emitted (`$`, `[`,
/// `\`). `words` are `(text, braced, expanded)`.
pub(crate) fn parse_time_expansion<'w>(
    words: impl IntoIterator<Item = (&'w str, bool, bool)>,
) -> Option<Vec<(String, bool)>> {
    let substitutes = |text: &str| text.contains(['$', '[', '\\']);
    let mut out = Vec::new();
    let mut expanded_any = false;
    for (text, braced, expanded) in words {
        if !expanded {
            // A braced word keeps its bracedness only where its text could
            // not read otherwise.
            if braced && substitutes(text) {
                return None;
            }
            out.push((text.to_owned(), braced));
            continue;
        }
        if !braced && substitutes(text) {
            return None;
        }
        expanded_any = true;
        let mut at = 0;
        while let Some(element) = tcl_syntax::list::find_element(text, at).ok()? {
            let value = &text[element.value.clone()];
            if !element.literal || substitutes(value) {
                return None;
            }
            out.push((value.to_owned(), true));
            at = element.next;
        }
    }
    expanded_any.then_some(out)
}

/// Whether the command word `cmd` names the builtin `name`: its own spelling,
/// or a global one, which a release compiles alike. A root marker is any run
/// of two or more colons, so `:::set` is `::set`.
pub(super) fn names_builtin(cmd: &str, name: &str) -> bool {
    cmd == name
        || (tcl_syntax::naming::is_absolute(cmd)
            && !tcl_syntax::naming::is_qualified(name.as_bytes())
            && tcl_syntax::naming::unroot_rooted_key(
                &tcl_syntax::naming::canonical_written_command(cmd),
            ) == Some(name))
}

/// Evaluate an argument source word after proving it eligible for a direct
/// local-name opcode. This preserves brace grouping and quoted-word decoding
/// instead of passing a compatibility spelling to the bytecode emitter.
pub(crate) fn source_direct_local_name_value(
    tokens: Option<&CommandTokens>,
    arg_index: usize,
    escapes: tcl_dialect::EscapeSyntax,
    word_rules: tcl_syntax::word_rules::WordValueRules,
) -> Option<String> {
    tokens
        .and_then(|tokens| tokens.words().get(arg_index + 1))
        .and_then(|word| compiled_local_name_value(word, escapes, word_rules))
}

/// Source availability and evaluated value for a nested local-name argument.
enum NestedLocalName {
    /// The caller did not retain a compatible structured source snapshot.
    Unavailable,
    /// An aligned source word is dynamic or requires backslash decoding.
    Stack,
    /// An aligned source word is direct and supplies this evaluated name.
    Direct(String),
}

/// Evaluate a nested invocation's local-name argument when its canonical word
/// snapshot still matches the compatibility parser's argv values. A missing or
/// mismatched snapshot leaves the established value emitter unchanged; an
/// aligned dynamic word selects the generic fallback, while an aligned direct
/// word supplies its evaluated literal name.
fn nested_local_name_value(
    tokens: Option<&CommandTokens>,
    parts: &[(String, bool)],
    arg_index: usize,
    escapes: tcl_dialect::EscapeSyntax,
    word_rules: tcl_syntax::word_rules::WordValueRules,
) -> NestedLocalName {
    let Some(tokens) = tokens else {
        return NestedLocalName::Unavailable;
    };
    if !tokens.words_align_with_argv_text()
        || tokens.argv_texts.len() != parts.len()
        || !tokens
            .argv_texts
            .iter()
            .zip(parts)
            .all(|(word, (text, _))| word == text)
    {
        return NestedLocalName::Unavailable;
    }
    match source_direct_local_name_value(Some(tokens), arg_index, escapes, word_rules) {
        Some(name) => NestedLocalName::Direct(name),
        None => NestedLocalName::Stack,
    }
}

// CodegenCtx methods — emission helpers for command substitutions

impl CodegenCtx<'_> {
    /// Emit a single arg from a parsed command substitution.
    ///
    /// Handles: `$var` loads, `$={name}` braced scalars, `[cmd]`
    /// nested substitutions, braced args with `$`/`[`, interpolated
    /// strings, backslash escapes, and plain literals.
    pub fn emit_cmd_subst_arg(&mut self, arg: &str, braced: bool) {
        // `parse_cmd_parts` has already removed a braced word's grouping
        // braces, leaving its exact value here. Keep that value on the
        // compiler-owned verbatim literal path: an ordinary PUSH asks the VM
        // to perform word substitution and would reinterpret a value such as
        // `{}` as fresh grouping syntax (and therefore push the empty string).
        // The verbatim helper also owns Tcl's one permitted transformation in
        // braces, backslash-newline collapse.
        if braced {
            self.push_lit_verbatim(arg);
            return;
        }
        // A composite unbraced arg with an embedded substitution (`$opt*`,
        // `x$y`, `${a}b`, `pre[cmd]post`) decomposes into more than one part:
        // build the string by concatenating the substituted parts rather than
        // pushing the raw text as a literal (which would leave `$opt*` /
        // `pre[cmd]` un-substituted). A pure `$var` / `${var}` / `$arr(i)` /
        // `[cmd]` is a single part and falls through to the fast paths below.
        if !braced
            && (arg.contains('$') || arg.contains('['))
            && let Some(parts) = parse_subst_template(arg, self.lexer_config())
            && parts.len() > 1
        {
            for part in &parts {
                match part {
                    // A decoded fragment is finished text, never source — see
                    // `push_compiled_word`, whose fragment rule this is.
                    SubstPart::Lit(text) => self.push_lit_exact(text),
                    SubstPart::ByteLit(bytes) => self.push_lit_bytes_exact(bytes),
                    SubstPart::Cmd(cmd) => self.emit_inline_cmd_subst(cmd),
                    SubstPart::Var(name) => self.load_var(name),
                    SubstPart::LiteralElement { base, key } => self.load_literal_element(base, key),
                    SubstPart::Expression(expression) => {
                        self.emit_expression_substitution(expression);
                    }
                }
            }
            self.emit(
                Op::STR_CONCAT1,
                vec![Operand::Imm(bytecode_imm(parts.len()))],
            );
            return;
        }
        if !braced && arg.starts_with('$') {
            if !self.emit_variable_reference(arg) {
                self.emit_value(arg, true);
            }
        } else if !braced && arg.starts_with('[') && arg.ends_with(']') {
            // Nested command substitution — compile inline. Everything
            // except an `expr` body gets a startCommand wrap: `expr`'s
            // inline emitter (the registry-stamped
            // `InlineCodegenHookId::Expr`) compiles to pure stack ops
            // with no command boundary, so tclsh emits no startCommand
            // for it.
            let inner_parts = parse_cmd_parts_with_config(arg, self.lexer_config());
            let needs_sc = self.is_proc
                && inner_parts.as_ref().is_some_and(|parts| {
                    !parts.is_empty()
                        && (self.inline_cmd_subst_hook(&parts[0].0, &parts[1..])
                            != Some(InlineCodegenHookId::Expr)
                            || (parts.len() == 2 && !lone_spelled_expression(&parts[1..])))
                });
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
        } else if arg.contains('$') || arg.contains('[') {
            // Interpolated string — delegate to emit_value with interpolation
            self.emit_value(arg, true);
        } else if !braced && arg.contains('\\') {
            // This branch has no substitution markers and decodes its source
            // escapes once. The result is finished data, not compiled-word text.
            self.push_decoded_literal(arg);
        } else {
            // A de-quoted or plain arg is the finished value, so it must not be
            // re-substituted: `[string index "{}" 0]` answered empty because
            // the value `{}` was pushed substituting and `subst_word` stripped
            // the braces a second time (`push_compiled_word`).
            self.push_lit_exact(arg);
        }
    }

    /// Emit a value with full interpolation support.
    ///
    /// Extends the simplified `emit_value_interpolated` with command
    /// substitution inlining and `parse_subst_template` decomposition.
    pub fn emit_value(&mut self, value: &str, interpolate: bool) {
        if self.emit_variable_reference(value) {
            return;
        }
        // The `[list …]` / `[format …]` / `[dict create …]` folds and the two
        // `list` inlinings — one copy, shared with `emit_value_interpolated`.
        if self.try_emit_constant_fold(value) {
            return;
        }
        // Whole-word variable-index array element `$arr($idx)`: route through
        // `load_var` (base + substituted key). The runtime literal/`subst_word`
        // path cannot resolve a bare `$idx` inside the index, so this must be
        // decomposed at compile time into a `loadArray` (or `loadArrayStk`).
        if interpolate
            && value.starts_with('$')
            && value.ends_with(')')
            && let Some(parts) = parse_subst_template(value, self.lexer_config())
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
            && let Some(parts) = parse_subst_template(value, self.lexer_config())
            && parts.len() > 1
        {
            for part in &parts {
                match part {
                    // A decoded fragment is finished text, never source — see
                    // `push_compiled_word`, whose fragment rule this is.
                    SubstPart::Lit(text) => self.push_lit_exact(text),
                    SubstPart::ByteLit(bytes) => self.push_lit_bytes_exact(bytes),
                    SubstPart::Cmd(cmd_text) => {
                        self.emit_inline_cmd_subst(cmd_text);
                    }
                    SubstPart::Var(name) => {
                        self.load_var(name);
                    }
                    SubstPart::LiteralElement { base, key } => self.load_literal_element(base, key),
                    SubstPart::Expression(expression) => {
                        self.emit_expression_substitution(expression);
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
        // A word whose markers are *escaped* is still source, not a value: the
        // escapes have to be decoded before anything is pushed, and deferring
        // it to the VM hands over backslashes that its compiled-word
        // convention reads as ordinary characters. The template parser decodes
        // each literal run, so `"\$\{x}"` becomes the four-character value
        // `${x}` rather than its six-character spelling. The twin of the same
        // arm in `emit_value_interpolated`. A source word with any other
        // backslash escape (`return \101`) is decoded the same way: tclsh
        // pushes its value `A`, never its four-character spelling (#2291).
        if (crate::codegen::statements::has_escaped_subst_marker(value)
            || (interpolate && value.contains('\\')))
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
                    vec![Operand::Imm(bytecode_imm(parts.len()))],
                );
            } else if parts.is_empty() {
                self.push_lit_exact("");
            }
            return;
        }
        // Preserve the word's value, including literal braces. Any remaining
        // substitution stays with the runtime rather than becoming literal text.
        self.push_compiled_word(value);
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
        self.emit_inline_cmd_subst_with_tokens(text, None);
    }

    /// Emit one command substitution using an aligned canonical word snapshot
    /// when its enclosing value preserved one.
    ///
    /// The aligned original word owns operand geometry, native compiler
    /// admission, and child body coordinates. The shared hook dispatcher keeps
    /// these receipts when a substitution is consumed as one value.
    pub fn emit_inline_cmd_subst_with_tokens(
        &mut self,
        text: &str,
        tokens: Option<&CommandTokens>,
    ) {
        let syntax = tokens
            .is_none()
            .then(|| self.original_inline_syntax(text))
            .flatten();
        let tokens = tokens.or(syntax.as_ref());
        let start = self.instructions.len();
        self.with_invocation_tokens(tokens, |ctx| ctx.emit_inline_cmd_subst_inner(text, tokens));
        if has_command_separator_with_config(text, self.lexer_config()) {
            self.stamp_substitution_script(start);
        } else {
            self.stamp_substitution_error_info(start, text, tokens);
        }
    }

    /// The words of the substituted script `text`, and whether a `{*}` of a
    /// literal word was expanded as the parser expands it; `None` once the
    /// substitution has been emitted otherwise (a run-time expansion as the
    /// expanded generic form, or a script the configured projection cannot
    /// split as one evaluated script).
    fn cmd_subst_parts(&mut self, text: &str) -> Option<(Vec<(String, bool)>, bool)> {
        if self.recognises_expand_syntax() && text.contains("{*}") {
            let Some(expand_parts) = parse_cmd_parts_expand_with_config(text, self.lexer_config())
            else {
                self.emit_source_script_eval(text);
                return None;
            };
            if expand_parts.iter().any(|(_, _, expand)| *expand) {
                let words = expand_parts
                    .iter()
                    .map(|(word, braced, expand)| (word.as_str(), *braced, *expand));
                if let Some(expanded) = parse_time_expansion(words) {
                    return Some((expanded, true));
                }
                self.emit_expanded_cmd_subst(&expand_parts);
                return None;
            }
        }
        let Some(parts) = parse_cmd_parts_with_config(text, self.lexer_config()) else {
            self.emit_source_script_eval(text);
            return None;
        };
        Some((parts, false))
    }

    /// An inline `[expr word]`. A simple word is the expression and compiles
    /// inline; a word with substitutions is substituted first and its value
    /// parsed at run time (`TclCompileExprWords`): `expr $x` evaluates a
    /// `[cmd]` inside `x`'s value, and `\"` reaches `expr` as `"`.
    fn emit_inline_expr(&mut self, word: &(String, bool)) {
        if !lone_spelled_expression(std::slice::from_ref(word)) {
            self.emit_native_argument_word(0, &word.0, word.1);
            self.emit(Op::EXPR_STK, vec![]);
            return;
        }
        // Re-parsed under the compile's own dialect, exactly as the lowering
        // pass parses a statement-position `expr` — parsing it dialect-blind
        // here would leave a dialect-only operator (`$x contains "a"`)
        // unrecognised and push it as a raw string.
        let node = self.parse_compile_expr(&word.0);
        if !self.emit_expr(&node) {
            self.emit(Op::TRY_CVT_TO_NUMERIC, vec![]);
        }
    }

    pub(super) fn original_inline_syntax(&self, text: &str) -> Option<CommandTokens> {
        let wrapped;
        let spelling = if text.starts_with('[') && text.ends_with(']') {
            text
        } else {
            wrapped = format!("[{text}]");
            &wrapped
        };
        tcl_compiler::word_subst::nested_command_words(
            spelling,
            &tcl_source::words::SourceSite::opaque(tcl_lexer::Span::new(0, 0)),
            self.lexer_config(),
        )
        .ok()
    }

    fn emit_inline_cmd_subst_inner(&mut self, text: &str, tokens: Option<&CommandTokens>) {
        if self.emit_native_original_preparation_invocation(tokens) {
            return;
        }
        if self.emit_native_procedure_noop(tokens) || self.emit_native_named_invocation(tokens) {
            return;
        }
        // Multi-command scripts (a `;`/newline separator outside quotes/braces)
        // fall back to runtime eval — checked *before* the `{*}` form below so a
        // body that has both (`[set y 1; list {*}$a]`) runs as two commands
        // rather than being mis-parsed as one expanded command.
        if has_command_separator_with_config(text, self.lexer_config()) {
            let inner = if text.starts_with('[') && text.ends_with(']') {
                &text[1..text.len() - 1]
            } else {
                text
            };
            self.push_lit(&format!("{{{inner}}}"));
            self.emit(Op::EVAL_STK, vec![]);
            return;
        }
        let Some((mut parts, expanded)) = self.cmd_subst_parts(text) else {
            return;
        };
        // Words the parser expanded have no aligned original tokens.
        let tokens = if expanded { None } else { tokens };
        if parts.is_empty() {
            self.push_lit("");
            return;
        }

        if self.emit_dynamic_local_introspection(tokens, &mut parts) {
            return;
        }
        let cmd = &parts[0].0;
        let args = &parts[1..];

        self.emit_inline_cmd_subst_parts(text, tokens, cmd, args);
    }

    pub(super) fn emit_source_script_eval(&mut self, text: &str) {
        let inner = text
            .strip_prefix('[')
            .and_then(|text| text.strip_suffix(']'))
            .unwrap_or(text);
        self.emit_script_eval(inner);
    }

    pub(super) fn emit_script_eval(&mut self, text: &str) {
        self.push_lit_exact(text);
        self.emit(Op::EVAL_STK, vec![]);
    }

    fn emit_inline_cmd_subst_parts(
        &mut self,
        text: &str,
        tokens: Option<&CommandTokens>,
        cmd: &str,
        args: &[(String, bool)],
    ) {
        let operands = args.to_vec();
        self.with_native_hook_operands(cmd, &operands, |ctx, logical| {
            ctx.emit_inline_hook_operands(text, cmd, logical, tokens);
        });
    }

    fn emit_inline_or_generic_cmd_subst(
        &mut self,
        cmd: &str,
        args: &[(String, bool)],
        emit_inline: impl FnOnce(&mut Self) -> bool,
    ) {
        if !emit_inline(self) {
            self.used_inline_cmd_subst = false;
            self.emit_generic_cmd_subst(cmd, args);
        }
    }

    fn emit_selected_inline_catch(&mut self, args: &[(String, bool)]) {
        self.used_inline_cmd_subst = true;
        let body_text = &args[0].0;
        let result_var = args.get(1).map(|(s, _)| s.as_str());
        let options_var = args.get(2).map(|(s, _)| s.as_str());
        let emitted = self.emit_catch_inline(body_text, result_var, options_var);
        debug_assert!(emitted, "the same retained catch output proof was selected");
    }

    fn emit_inline_hook_operands(
        &mut self,
        text: &str,
        cmd: &str,
        args: &[(String, bool)],
        tokens: Option<&CommandTokens>,
    ) {
        if super::emitter::bytecoded::try_value_bytecoded(self, cmd, args) {
            return;
        }
        // Registry-driven dispatch: the hook ID names the emitter, the
        // guards are each emitter's applicability conditions. A
        // subcommand-keyed hook (`InfoExists`, `DictGet`) only resolves
        // when the subcommand word matched exactly, so those arms need
        // no re-check of the subcommand text.
        match self.inline_cmd_subst_resolution(cmd, args) {
            Some((InlineCodegenHookId::Error, _)) if (1..=3).contains(&args.len()) => {
                self.emit_inline_or_generic_cmd_subst(cmd, args, |ctx| {
                    ctx.emit_inline_error(args, text)
                });
            }
            Some((InlineCodegenHookId::NamespaceCurrent, _)) => {
                self.emit(Op::CURRENT_NAMESPACE, vec![]);
            }
            Some((InlineCodegenHookId::NamespaceOrigin, _)) => {
                self.emit_inline_or_generic_cmd_subst(cmd, args, |ctx| {
                    ctx.emit_inline_namespace_origin(tokens)
                });
            }
            Some((InlineCodegenHookId::NamespaceCode, _)) => {
                self.emit_inline_or_generic_cmd_subst(cmd, args, |ctx| {
                    ctx.emit_inline_namespace_code(tokens)
                });
            }
            Some((InlineCodegenHookId::Yield, _)) if args.len() <= 1 => {
                self.emit_inline_yield(args);
            }
            Some((InlineCodegenHookId::YieldTo, _)) => {
                self.emit_inline_yield_to(
                    args.iter()
                        .map(|(word, braced)| (word.as_str(), *braced, false)),
                );
            }
            Some((InlineCodegenHookId::Expr, _)) if args.len() == 1 => {
                self.emit_inline_expr(&args[0]);
            }
            Some((InlineCodegenHookId::Incr, _)) if (1..=2).contains(&args.len()) => {
                self.emit_inline_incr(args);
            }
            Some((InlineCodegenHookId::InfoLevel, _)) => {
                self.emit_inline_or_generic_cmd_subst(cmd, args, |ctx| {
                    ctx.emit_inline_info_level(tokens)
                });
            }
            Some((InlineCodegenHookId::InfoExists, _)) => {
                self.emit_inline_or_generic_cmd_subst(cmd, args, |ctx| {
                    ctx.emit_inline_info_exists(tokens)
                });
            }
            Some((InlineCodegenHookId::InfoCommandsResolve, _)) if args.len() == 2 => {
                self.emit_inline_info_commands_resolve(args);
            }
            Some((InlineCodegenHookId::Set, _)) if self.inline_set_load_is_supported(args) => {
                // `TclCompileSetCmd`: the one-word form is a load.
                self.used_inline_cmd_subst = true;
                self.load_var(&args[0].0);
            }
            Some((InlineCodegenHookId::String, binding)) if args.len() >= 2 => {
                self.emit_canonical_inline_string(cmd, &binding, args);
            }
            Some((InlineCodegenHookId::Lindex, _)) => {
                self.emit_inline_or_generic_cmd_subst(cmd, args, |ctx| {
                    ctx.emit_inline_lindex(tokens)
                });
            }
            Some((InlineCodegenHookId::Lrange, _)) if args.len() == 3 => {
                self.emit_inline_lrange(args);
            }
            Some((InlineCodegenHookId::Lreplace, _)) if args.len() >= 3 => {
                self.emit_inline_lreplace(args);
            }
            Some((InlineCodegenHookId::Linsert, _)) if args.len() >= 2 => {
                self.emit_inline_linsert(args);
            }
            Some((InlineCodegenHookId::Regexp, _)) if args.len() >= 2 => {
                self.emit_inline_regexp(args);
            }
            Some((hook, _)) if self.try_emit_string_subcommand(hook, args) => {}
            Some((InlineCodegenHookId::StringIs, binding)) if args.len() >= 3 => {
                self.emit_string_is_generic(cmd, &binding, &args[1..]);
            }
            Some((InlineCodegenHookId::List, _)) if !args.is_empty() && !text.contains("{*}") => {
                self.emit_inline_list(args);
            }
            Some((InlineCodegenHookId::Array, _)) if args.len() >= 2 => {
                self.emit_inline_array(cmd, args);
            }
            Some((InlineCodegenHookId::DictGet, _)) if args.len() >= 3 => {
                self.emit_inline_dict_get(args);
            }
            Some((InlineCodegenHookId::Catch, _))
                if self.is_proc
                    && self.release_admits_arity(cmd, args.len())
                    && self.catch_inline_shape_is_supported(args) =>
            {
                self.emit_selected_inline_catch(args);
            }
            _ => {
                self.used_inline_cmd_subst = false;
                self.emit_generic_cmd_subst(cmd, args);
            }
        }
    }

    fn inline_set_load_is_supported(&self, args: &[(String, bool)]) -> bool {
        matches!(args, [(name, braced)]
            if !self.in_direct_command()
                && !tcl_syntax::naming::word_is_dynamic(name, *braced)
                && !name.contains('\\'))
    }

    fn emit_canonical_inline_string(
        &mut self,
        cmd: &str,
        binding: &tcl_runtime_api::CommandBindingIdentity,
        args: &[(String, bool)],
    ) {
        if let Some(canonical) = self.canonical_subcommand_args(cmd, args) {
            self.emit_inline_string(cmd, binding, &canonical);
        } else {
            self.used_inline_cmd_subst = false;
            self.emit_generic_cmd_subst(cmd, args);
        }
    }

    fn emit_inline_list(&mut self, args: &[(String, bool)]) {
        self.used_inline_cmd_subst = true;
        for (index, (word, braced)) in args.iter().enumerate() {
            self.emit_native_argument_word(index, word, *braced);
        }
        self.emit(Op::LIST, vec![Operand::Imm(bytecode_imm(args.len()))]);
    }

    /// Emit a `{*}`-expanded command substitution in value position:
    /// `expandStart`, each word (an expanded word followed by `expandStkTop N`
    /// where N is the running word count), then `invokeExpanded` — leaving the
    /// result on the stack. Mirrors [`Self::emit_expanded_call`] without the
    /// trailing `pop`.
    pub(super) fn emit_expanded_cmd_subst(&mut self, parts: &[(String, bool, bool)]) {
        if self.try_emit_expanded_native_call(parts) {
            return;
        }
        self.used_inline_cmd_subst = true;
        self.emit_expanded_words(
            parts
                .iter()
                .map(|(word, braced, expanded)| (word.as_str(), *braced, *expanded)),
            "(expanded)",
        );
    }
}

// Tests

#[cfg(test)]
mod tests;
