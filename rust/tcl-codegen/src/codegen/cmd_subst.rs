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

use tcl_bytecode::EnteredCommandSite;
use tcl_cmd_core::index::{Encoded, IndexCompiler};
use tcl_registry::hooks::InlineCodegenHookId;

use tcl_compiler::ir::{CommandTokens, WordExpr};
use tcl_compiler::registry_invocation::compiled_local_name_value;

use super::helpers::{SubstPart, parse_subst_template};
use super::values::{is_qualified, split_array_ref};
use super::{CodegenCtx, INDEX_END, Op, Operand, bytecode_imm, end_relative_imm, str_class_id};

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
    pub(super) fn vm_release(&self) -> tcl_dialect::TclVersion {
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
    pub(super) fn compiled_index_words(
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
    fn emit_index_words(
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
    pub(super) fn try_emit_compiled_index_command(
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
    fn try_emit_string_replace(&mut self, args: &[(String, bool)]) -> bool {
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

    fn emit_str_range_imm(&mut self, first: i32, last: i32) {
        self.emit(
            Op::STR_RANGE_IMM,
            vec![Operand::Imm(first), Operand::Imm(last)],
        );
    }
}

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

    /// Literal command whose registry-described bytecode specialisation this
    /// generic value-position surrogate stands in for.
    ///
    /// Ordinary `invokeStk` commands resolve after argument substitution in C
    /// Tcl, so a literal head alone is not enough. The entered-token sidecar is
    /// reserved for an operation selected by the native compiler grammar.
    /// A Rust emission hook alone does not establish native selection timing.
    pub(super) fn entered_command_surrogate(
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
    pub(super) fn retain_entered_command(
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
    fn emit_generic_cmd_subst_with_binding(
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

    /// Wrapping metadata follows the same actual selected invocation as hooks.
    pub(super) fn selected_invocation_traits(
        &self,
        command: &str,
        args: &[(String, bool)],
    ) -> Option<tcl_registry::Traits> {
        if !self.invocation_specialisation_proved() {
            return None;
        }
        if let Some(tokens) = self.invocation_tokens.as_deref()
            && tokens.source_binding.is_some()
        {
            if !tokens.words_align_with_argv_text()
                || tokens.argv_texts.first().is_none_or(|head| head != command)
                || tokens.argv_texts.len() != args.len().checked_add(1)?
                || tokens
                    .argv_texts
                    .iter()
                    .skip(1)
                    .zip(args)
                    .any(|(original, (text, _))| original != text)
            {
                return None;
            }
            return Some(
                tcl_compiler::registry_invocation::admitted_native_compiler_invocation(
                    self.registry,
                    None,
                    tokens,
                )?
                .traits(),
            );
        }
        let arguments = args
            .iter()
            .map(|(text, _)| text.as_str())
            .collect::<Vec<_>>();
        let selected =
            self.registry
                .resolve_call(command, &arguments, self.invocation_surface_query())?;
        names_builtin(command, selected.spec.name).then_some(selected.spec.traits)
    }

    /// Resolve the registry-stamped [`InlineCodegenHookId`] for a
    /// command-substitution head word, mirroring how
    /// `emitter::bytecoded::try_bytecoded` resolves statement-position
    /// hooks (`resolve_call`, so a subcommand-keyed hook such as
    /// `dict get` / `info exists` wins over the command level).
    ///
    /// The head must name the builtin itself ([`names_builtin`]): its own
    /// spelling or the global one (`[::expr …]`), which a release compiles
    /// alike. Any other qualified spelling stays on the generic-invoke path.
    pub(crate) fn inline_cmd_subst_hook(
        &mut self,
        cmd: &str,
        args: &[(String, bool)],
    ) -> Option<InlineCodegenHookId> {
        self.inline_cmd_subst_resolution(cmd, args)
            .map(|(hook, _binding)| hook)
    }

    /// Read a registry hook without retaining a command-binding dependency.
    ///
    /// This only gates a later source-form decision. The caller that actually
    /// emits a specialised opcode must enter through
    /// [`Self::inline_cmd_subst_resolution`] and retain its binding there.
    pub(crate) fn inline_cmd_subst_hook_candidate(
        &self,
        cmd: &str,
        args: &[(String, bool)],
    ) -> Option<InlineCodegenHookId> {
        if self.plain_command_dispatch || !self.invocation_specialisation_proved() {
            return None;
        }
        if let Some(tokens) = self.invocation_tokens.as_deref()
            && tokens.source_binding.is_some()
        {
            return tcl_compiler::registry_invocation::admitted_native_compiler_invocation(
                self.registry,
                None,
                tokens,
            )?
            .inline_codegen_hook();
        }
        let arg_refs: Vec<&str> = args.iter().map(|(arg, _)| arg.as_str()).collect();
        let resolved =
            self.registry
                .resolve_call(cmd, &arg_refs, self.invocation_surface_query())?;
        names_builtin(cmd, resolved.spec.name).then_some(resolved.inline_codegen_hook?)
    }

    /// `args` with the subcommand word of `cmd`, an ensemble, spelt as the
    /// subcommand it selects in the emulated release: an ensemble compiles a
    /// unique prefix as the subcommand itself (`TclCompileEnsemble`). `None`
    /// where the word is not literal or selects no one subcommand, which
    /// leaves the command to run. The subcommand resolves under the
    /// invocation's own surface, as the hook did.
    fn canonical_subcommand_args(
        &self,
        cmd: &str,
        args: &[(String, bool)],
    ) -> Option<Vec<(String, bool)>> {
        let ((word, braced), rest) = args.split_first()?;
        if tcl_syntax::naming::word_is_dynamic(word, *braced) {
            return None;
        }
        let arg_refs: Vec<&str> = args.iter().map(|(a, _)| a.as_str()).collect();
        let query = self.invocation_surface_query();
        let resolved = self.registry.resolve_call(cmd, &arg_refs, query)?;
        if resolved.spec.subcommands.is_empty() {
            return Some(args.to_vec());
        }
        let sub = resolved
            .sub
            .or_else(|| resolved.spec.resolve_subcommand_for_dialect(word, query))?;
        Some(
            std::iter::once((sub.name.to_owned(), true))
                .chain(rest.iter().cloned())
                .collect(),
        )
    }

    /// Resolve and retain the hook together with the exact entered command
    /// identity selected for this source invocation.
    fn inline_cmd_subst_resolution(
        &mut self,
        cmd: &str,
        args: &[(String, bool)],
    ) -> Option<(InlineCodegenHookId, tcl_runtime_api::CommandBindingIdentity)> {
        let arg_refs: Vec<&str> = args.iter().map(|(a, _)| a.as_str()).collect();
        let (hook, site) = self.inline_codegen_resolution(cmd, &arg_refs)?;
        let shape = self.invocation_shape(&arg_refs, |index| args[index].1);
        self.require_site_binding(&site, &shape);
        Some((hook, site.binding))
    }

    /// Resolve and retain one registry-described inline specialisation.
    ///
    /// All inline contexts use this owner: ordinary command substitutions,
    /// catch/try bodies, and structured try emitters. A specialised emitter
    /// therefore cannot consume a command head without also retaining the
    /// source spelling and canonical implementation it assumed.
    pub(crate) fn inline_codegen_hook(
        &mut self,
        cmd: &str,
        args: &[&str],
    ) -> Option<InlineCodegenHookId> {
        let (hook, site) = self.inline_codegen_resolution(cmd, args)?;
        self.require_site_binding(&site, &self.invocation_shape(args, |_| false));
        Some(hook)
    }

    /// Resolve the hook and live command identity assumed by an inline
    /// specialisation. Emitters which retain an entered token use the same
    /// registry decision as the ordinary hook dispatcher.
    fn inline_codegen_resolution(
        &self,
        cmd: &str,
        args: &[&str],
    ) -> Option<(InlineCodegenHookId, super::SiteBinding)> {
        if self.plain_command_dispatch || !self.invocation_specialisation_proved() {
            return None;
        }
        if let Some(tokens) = self.invocation_tokens.as_deref()
            && tokens.source_binding.is_some()
        {
            let admitted = tcl_compiler::registry_invocation::admitted_native_compiler_invocation(
                self.registry,
                None,
                tokens,
            )?;
            return Some((
                admitted.inline_codegen_hook()?,
                super::SiteBinding {
                    binding: self
                        .command_binding_identity(cmd, admitted.canonical_registration_name()),
                    claim: None,
                },
            ));
        }
        // Whole-unit mutation discovery is deliberately not an admission
        // decision here: a later proc body may mutate this name only after the
        // current invocation has entered it. Retain the typed binding below;
        // the VM validates it at each actual execution boundary and recompiles
        // stale future invocations through plain dispatch.
        // The registry's own point — see `emitter::bytecoded::try_bytecoded`.
        let resolved = self
            .registry
            .resolve_call(cmd, args, self.invocation_surface_query())?;
        if !names_builtin(cmd, resolved.spec.name) {
            return None;
        }
        let hook = resolved.inline_codegen_hook?;
        if hook == InlineCodegenHookId::InfoCommandsResolve
            && !self
                .invocation_tokens
                .as_deref()
                .and_then(|tokens| tokens.source_binding.as_ref())
                .is_some_and(|binding| {
                    matches!(
                        binding.native_compilation_admission_selection(),
                        tcl_registry::native_compilation::NativeCompilationSelection::Inline {
                            operation: tcl_registry::SemanticOperationId::Intrinsic(
                                tcl_registry::IntrinsicId::InfoCommandsResolve
                            ),
                            ..
                        }
                    )
                })
        {
            return None;
        }
        // As `registry_codegen_hook`: the `alias_of` target's identity, and
        // the claim behind it, where the hook is the target's own; the
        // command's own name otherwise.
        let site = self.stamped_binding(
            cmd,
            &resolved,
            tcl_registry::codegen_stamp::CodegenStamp::InlineCodegen(hook),
        );
        Some((hook, site))
    }

    /// Preserve source-proven local-name semantics before value-position
    /// specialisation. Returns whether it emitted the conservative fallback.
    fn emit_dynamic_local_introspection(
        &mut self,
        tokens: Option<&CommandTokens>,
        parts: &mut [(String, bool)],
    ) -> bool {
        let source_local_name =
            nested_local_name_value(tokens, parts, 1, self.escapes, self.word_rules);
        let source_hook = self.inline_cmd_subst_hook_candidate(&parts[0].0, &parts[1..]);
        let source_is_local_introspection = match source_hook {
            Some(InlineCodegenHookId::InfoExists) => parts.len() == 3,
            Some(InlineCodegenHookId::Array) => parts.len() == 3 && parts[1].0 == "exists",
            _ => false,
        };
        if !source_is_local_introspection {
            return false;
        }
        match source_local_name {
            NestedLocalName::Direct(name) => {
                parts[2].0 = name;
                false
            }
            // The specialised emitters place their final name value on the
            // stack verbatim. Without an aligned source word that proves the
            // value is already final, that would suppress a live `$` or
            // command substitution (notably a Return value, whose
            // compatibility path retains no CommandTokens).
            NestedLocalName::Stack if source_hook == Some(InlineCodegenHookId::InfoExists) => false,
            NestedLocalName::Stack | NestedLocalName::Unavailable => {
                self.used_inline_cmd_subst = false;
                self.emit_generic_cmd_subst(&parts[0].0, &parts[1..]);
                true
            }
        }
    }

    /// Resolve one fully-consuming inline specialisation whose source name
    /// must also remain untouched throughout this compilation unit.
    ///
    /// Unlike entered-token emitters, the list-concat and inline loop-control
    /// paths below have no ordinary invocation left to redispatch. They need
    /// both the whole-unit trust fact and the active registry's typed hook;
    /// the returned binding is the exact dependency their bytecode consumes.
    fn trusted_inline_codegen_binding(
        &self,
        command: &str,
        args: &[&str],
        expected: InlineCodegenHookId,
    ) -> Option<tcl_runtime_api::CommandBindingIdentity> {
        if !self.trusts_builtin(command) {
            return None;
        }
        let (hook, site) = self.inline_codegen_resolution(command, args)?;
        // These callers name shipped commands, whose bindings carry no
        // pack claim; one that did would need it recorded, so it declines.
        (hook == expected && site.claim.is_none()).then_some(site.binding)
    }

    /// Try an inline emitter whose registry hook is also valid for an ordinary
    /// command statement.
    ///
    /// Inline hooks normally produce one stack value for a command
    /// substitution. String's `INVOKE_REPLACE` forms have the same argument
    /// substitution and command-entry semantics in statement position, so
    /// reuse their single applicability guard without the nested-substitution
    /// `START_CMD`, then discard the result just as the generic statement
    /// invoke does. Hook selection and the retained command identity still
    /// come from the active registry/profile; this bridge contains no command
    /// identity lookup.
    pub(crate) fn try_inline_statement_codegen(
        &mut self,
        cmd: &str,
        args: &[String],
        tokens: Option<&CommandTokens>,
        used_generic_invoke: &mut bool,
    ) -> bool {
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let Some((hook, site)) = self.inline_codegen_resolution(cmd, &arg_refs) else {
            return false;
        };
        let mut inline_args: Vec<(String, bool)> = args
            .iter()
            .enumerate()
            .map(|(i, arg)| {
                (
                    arg.clone(),
                    self.cmd_arg_braced.get(i).copied().unwrap_or(false),
                )
            })
            .collect();

        let local_name = self.original_hook_argument(1).and_then(|index| {
            source_direct_local_name_value(tokens, index, self.escapes, self.word_rules)
        });

        // These hooks are also valid for a complete command statement. The
        // statement dispatch checks the source-word facts, then reuses the
        // value emitter's local opcode sequence instead of recognising command
        // names itself. The result is discarded just as a normal statement
        // invoke would discard it.
        match hook {
            InlineCodegenHookId::Error if (1..=3).contains(&inline_args.len()) => {
                if !self.emit_inline_error(&inline_args, "") {
                    return false;
                }
            }
            InlineCodegenHookId::NamespaceCurrent => {
                self.emit(Op::CURRENT_NAMESPACE, vec![]);
            }
            InlineCodegenHookId::NamespaceOrigin => {
                if !self.emit_inline_namespace_origin(tokens) {
                    return false;
                }
            }
            InlineCodegenHookId::NamespaceCode => {
                if !self.emit_inline_namespace_code(tokens) {
                    return false;
                }
            }
            InlineCodegenHookId::Yield if inline_args.len() <= 1 => {
                self.emit_inline_yield(&inline_args);
            }
            InlineCodegenHookId::YieldTo => {
                self.emit_inline_yield_to(
                    inline_args
                        .iter()
                        .map(|(word, braced)| (word.as_str(), *braced, false)),
                );
            }
            InlineCodegenHookId::Lindex => {
                if !self.emit_inline_lindex(tokens) {
                    return false;
                }
            }
            InlineCodegenHookId::InfoLevel => {
                if !self.emit_inline_info_level(tokens) {
                    return false;
                }
            }
            InlineCodegenHookId::InfoExists => {
                if !self.emit_inline_info_exists(tokens) {
                    return false;
                }
            }
            InlineCodegenHookId::InfoCommandsResolve if inline_args.len() == 2 => {
                self.emit_inline_info_commands_resolve(&inline_args);
            }
            InlineCodegenHookId::Array
                if inline_args.len() == 2
                    && inline_args[0].0 == "exists"
                    && local_name.is_some() =>
            {
                inline_args[1].0 = local_name.expect("guard proved a source local name");
                self.emit_inline_array(cmd, &inline_args);
            }
            InlineCodegenHookId::String => {
                if !self.try_inline_statement_string(cmd, &site.binding, &inline_args) {
                    return false;
                }
            }
            InlineCodegenHookId::Regexp if self.try_emit_compiled_regexp(&inline_args) => {}
            hook if self.try_emit_string_subcommand(hook, &inline_args) => {}
            InlineCodegenHookId::Lreplace
                if self.try_emit_compiled_index_command(cmd, &inline_args) => {}
            _ => return false,
        }
        self.emit(Op::POP, vec![]);
        *used_generic_invoke = true;
        let shape = self.invocation_shape(args, |index| {
            self.cmd_arg_braced.get(index).copied().unwrap_or(false)
        });
        self.require_site_binding(&site, &shape);
        true
    }

    fn try_inline_statement_string(
        &mut self,
        cmd: &str,
        binding: &tcl_runtime_api::CommandBindingIdentity,
        args: &[(String, bool)],
    ) -> bool {
        let previous_inline = self.used_inline_cmd_subst;
        let Some(canonical) = self.canonical_subcommand_args(cmd, args) else {
            return false;
        };
        self.try_emit_inline_string_invoke_replace(previous_inline, cmd, binding, &canonical, false)
            || self.try_emit_compiled_index_command(cmd, args)
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
            &tcl_compiler::ir::SourceSite::opaque(tcl_lexer::Span::new(0, 0)),
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

    // Private inline helpers for emit_inline_cmd_subst.

    /// Native Error uses the same return-options stack protocol as returnImm,
    /// preserving optional errorInfo/errorCode argv and their evaluation order.
    pub(super) fn emit_inline_error(
        &mut self,
        args: &[(String, bool)],
        source_command: &str,
    ) -> bool {
        use tcl_registry::native_error_compilation::{NativeErrorStep, native_error_instruction};
        let Some(version) = self
            .native_hook_dialect()
            .and_then(|dialect| dialect.tcl_version)
        else {
            return false;
        };
        let operands = (0..args.len())
            .map(tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand::Original)
            .collect::<Vec<_>>();
        let Ok(recipe) = native_error_instruction(&operands, version) else {
            return false;
        };
        self.used_inline_cmd_subst = true;
        for step in recipe.steps {
            match step {
                NativeErrorStep::Word(tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand::Original(index)) => self.emit_native_argument_word(index, &args[index].0, args[index].1),
                NativeErrorStep::Word(_) => unreachable!("selected original argv operands"),
                NativeErrorStep::Literal(value) => { self.push_lit_bytes_exact(&value); }
                NativeErrorStep::List(count) => { self.emit(Op::LIST, vec![Operand::Imm(i32::try_from(count).expect("native Error pair count"))]); }
                NativeErrorStep::DictionaryPut => { self.emit(Op::DICT_PUT, vec![]); }
                NativeErrorStep::ReturnError => self.emit_native_error_return(source_command),
                NativeErrorStep::Pop => { self.emit(Op::POP, vec![]); }
                NativeErrorStep::ThrowTypeCheck => unreachable!("Error recipes carry no throw type check"),
            }
        }
        true
    }

    pub(super) fn emit_inline_yield(&mut self, args: &[(String, bool)]) {
        self.used_inline_cmd_subst = true;
        if let Some((word, braced)) = args.first() {
            self.emit_native_argument_word(0, word, *braced);
        } else {
            self.push_lit_exact("");
        }
        self.emit(Op::YIELD, vec![]);
    }

    /// Native relay list construction preserves namespace capture before argv
    /// evaluation and Tcl 9.1's expansion boundaries.
    pub(super) fn emit_inline_yield_to<'a>(
        &mut self,
        words: impl IntoIterator<Item = (&'a str, bool, bool)>,
    ) {
        self.used_inline_cmd_subst = true;
        self.emit(Op::CURRENT_NAMESPACE, vec![]);
        self.emit_native_argument_list(words, 0, 1);
        self.emit(Op::YIELD_TO_INVOKE, vec![]);
    }

    /// Build a native list from original argument words and expansion segments.
    /// `arguments_from` retains the hook's operand map; `prefix_count` counts
    /// already evaluated stack values included before those arguments.
    pub(super) fn emit_native_argument_list<'a>(
        &mut self,
        words: impl IntoIterator<Item = (&'a str, bool, bool)>,
        arguments_from: usize,
        prefix_count: usize,
    ) {
        let segment_limit = self.native_hook_dialect().and_then(
            tcl_registry::native_compilation::NativeTailcallStack::argument_list_segment_limit,
        );
        let mut pending = prefix_count;
        let mut concatenated = false;
        for (index, (word, braced, expanded)) in words.into_iter().enumerate() {
            if expanded && pending > 0 {
                self.emit(Op::LIST, vec![Operand::Imm(bytecode_imm(pending))]);
                if concatenated {
                    self.emit(Op::LIST_CONCAT, vec![]);
                }
                pending = 0;
                concatenated = true;
            }
            self.emit_native_argument_word(arguments_from + index, word, braced);
            if expanded {
                if concatenated {
                    self.emit(Op::LIST_CONCAT, vec![]);
                }
                concatenated = true;
            } else {
                pending += 1;
                if segment_limit.is_some_and(|limit| pending > limit) {
                    self.emit(Op::LIST, vec![Operand::Imm(bytecode_imm(pending))]);
                    if concatenated {
                        self.emit(Op::LIST_CONCAT, vec![]);
                    }
                    pending = 0;
                    concatenated = true;
                }
            }
        }
        if pending > 0 {
            self.emit(Op::LIST, vec![Operand::Imm(bytecode_imm(pending))]);
            if concatenated {
                self.emit(Op::LIST_CONCAT, vec![]);
            }
        }
        if pending == 0 && !concatenated {
            self.emit(Op::LIST, vec![Operand::Imm(0)]);
        }
    }

    pub(super) fn emit_native_argument_word(&mut self, index: usize, text: &str, braced: bool) {
        let word = self.original_hook_argument(index).and_then(|original| {
            self.invocation_tokens
                .as_deref()?
                .words()
                .get(original + 1)
                .cloned()
        });
        // `text` is the written word as the substituted script spells it
        // (`{}$z`); the original word's compatibility text spells its
        // variables `${z}`. The compiled-word convention reads a bare `$` as
        // data, so the written spelling must never reach the word path: there
        // `string length "{}$z"` pushed the four bytes `{}$z` unsubstituted.
        // Use the original word only when both spellings name the same parts,
        // and then in its own convention; otherwise the written text compiles
        // through the substituted-script word path.
        let word = word.filter(|word| {
            let legacy = word.legacy_text();
            legacy == text
                || (!braced
                    && parse_subst_template(&legacy, self.lexer_config()).is_some_and(|parts| {
                        parse_subst_template(text, self.lexer_config()).as_ref() == Some(&parts)
                    }))
        });
        if let Some(word) = word {
            self.emit_word_from_source(&word.legacy_text(), braced, Some(&word));
        } else {
            self.emit_cmd_subst_arg(text, braced);
        }
    }

    pub(super) fn try_emit_expanded_native_call(&mut self, parts: &[(String, bool, bool)]) -> bool {
        let Some(tokens) = self.invocation_tokens.as_deref() else {
            return false;
        };
        if tokens.source_binding.is_none() {
            let dialect = tcl_compiler::environment_ingress::authoring_invocation_dialect(
                self.registry,
                self.dialect,
                self.lexer_config(),
            );
            let Some((
                _,
                tcl_registry::native_compilation::NativeCompilationSelection::Inline { .. },
            )) = tcl_compiler::registry_invocation::native_compilation_syntax(
                self.registry,
                tokens,
                dialect,
                self.native_compilation,
            )
            else {
                return false;
            };
        }
        let Some((head, _, false)) = parts.first() else {
            return false;
        };
        let args = parts[1..]
            .iter()
            .map(|(word, _, _)| word.as_str())
            .collect::<Vec<_>>();
        if super::emitter::bytecoded::try_expanded_tailcall(self, head, &parts[1..]) {
            return true;
        }
        if super::emitter::bytecoded::try_expanded_lappend(self, head, &parts[1..]) {
            self.used_inline_cmd_subst = true;
            return true;
        }
        let Some((InlineCodegenHookId::YieldTo, binding)) =
            self.inline_codegen_resolution(head, &args)
        else {
            return false;
        };
        self.emit_inline_yield_to(
            parts[1..]
                .iter()
                .map(|(word, braced, expanded)| (word.as_str(), *braced, *expanded)),
        );
        // A run-time expansion leaves the words after the command unknown.
        self.require_site_binding(
            &binding,
            &tcl_registry::release_compilation::InvocationShape::default(),
        );
        true
    }

    fn emit_inline_incr(&mut self, args: &[(String, bool)]) {
        let Some((name, braced)) = args.first() else {
            return;
        };
        let amount = args.get(1);
        self.emit_incr(
            name,
            *braced,
            amount.map(|(amount, _)| amount.as_str()),
            amount.is_some_and(|(_, braced)| *braced),
        );
    }

    /// One retained compiler selection and original-command replay boundary.
    fn emit_native_compiler_selection(
        &mut self,
        source: &str,
        namespace: &str,
        namespace_context: Option<tcl_runtime_api::CompiledNamespaceContext>,
        prerequisite: tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite,
    ) -> String {
        let selection = self.emit(Op::NOP, vec![]);
        let end = self.fresh_label("native_selection_end");
        self.instructions[selection].source_cmd_text =
            tcl_lexer::SourceImage::from_bytes(source.as_bytes(), self.source_image().channel());
        self.instructions[selection].source_command_namespace =
            namespace_context.as_ref().map_or_else(
                || super::namespace_path_from_constructed_key(namespace),
                |context| context.path().clone(),
            );
        self.instructions[selection].source_command_namespace_context = namespace_context;
        self.instructions[selection].source_command_boundary =
            tcl_bytecode::SourceCommandBoundary::Start;
        self.instructions[selection].no_fold = true;
        self.instructions[selection].native_compiler_selection =
            Some(tcl_bytecode::NativeCompilerSelectionSite {
                prerequisite,
                end: end.clone(),
            });
        end
    }

    /// Execute every original operand, then produce the result selected by the
    /// actual native empty variadic procedure compiler. No procedure frame or
    /// late invocation is introduced by Tcl's `NoOp` compiler.
    pub(super) fn emit_native_procedure_noop(&mut self, tokens: Option<&CommandTokens>) -> bool {
        if self.plain_command_dispatch {
            return false;
        }
        let Some(tokens) = tokens else { return false };
        let Some(plan) = tcl_compiler::registry_invocation::native_procedure_noop_plan(tokens)
        else {
            return false;
        };
        let end = self.emit_native_compiler_selection(
            &plan.source,
            &plan.namespace,
            plan.namespace_context,
            tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite::ProcedureHeader(Box::new(plan.prerequisite)),
        );
        for (index, word) in tokens.words().iter().enumerate().skip(1) {
            let braced = tokens.argv_kinds.get(index) == Some(&tcl_lexer::TokenType::Str)
                && tokens
                    .single_token_word
                    .get(index)
                    .copied()
                    .unwrap_or(false);
            self.emit_word_from_source(&tokens.argv_texts[index], braced, Some(word));
            self.emit(Op::POP, vec![]);
        }
        self.push_lit_exact("");
        self.place_label(&end);
        self.used_inline_cmd_subst = true;
        true
    }

    /// Emit a captured private name followed by late ordinary handler lookup.
    /// Source substitutions execute once, in original order, without capturing
    /// an entered token or substituting a canonical handler for the private name.
    pub(super) fn emit_native_named_invocation(&mut self, tokens: Option<&CommandTokens>) -> bool {
        if self.plain_command_dispatch {
            return false;
        }
        let Some(tokens) = tokens else { return false };
        let Some(binding) = tokens.source_binding.as_ref() else {
            return false;
        };
        let Some(named) = binding.admitted_named_invocation() else {
            return false;
        };
        let Some((image, originals, offset)) = binding.original_compiler_source(tokens) else {
            return false;
        };
        let Some(dialect) = binding.native_compiler_dialect() else {
            return false;
        };
        let Some(protocol) = binding.compiler_source_protocol() else {
            return false;
        };
        let Some(originals) = tcl_compiler::registry_invocation::original_native_compiler_words(
            image,
            originals,
            offset,
            tokens.native_lexer_config(self.lexer_config()),
        ) else {
            return false;
        };
        let Ok(captured) =
            tcl_registry::native_compiler_words::NativeCompilerWords::capture(&originals, protocol)
        else {
            return false;
        };
        let replacements: Vec<_> = named
            .replacement_words
            .iter()
            .map(|word| word.as_bytes().to_vec())
            .collect();
        let Ok(recipe) = tcl_registry::native_instruction_plan::native_named_invocation_instruction(
            &captured,
            dialect,
            named.captured_name().as_bytes(),
            named.arguments_from,
            named.protocol,
            &replacements,
        ) else {
            return false;
        };
        // INVOKE_REPLACE has a fixed original word count. The genuine compiler
        // selects ordinary dispatch when a rewrite still has dynamic expansion.
        if recipe.protocol
            == tcl_registry::native_compilation::NativeNamedInvocationProtocol::EnsembleRewrite
            && recipe.expanded.iter().any(|expanded| *expanded)
        {
            return false;
        }
        let selection_end = if let Some(required) = &named.compiler_prerequisite {
            self.retain_entry_named_compiler_prerequisite(required);
            let Some(source) = tcl_compiler::registry_invocation::native_compiler_replay_source(
                tokens,
                &named.compilation_site,
            ) else {
                return false;
            };
            let end = self.emit_native_compiler_selection(
                &source,
                &tokens.source_binding.as_ref().unwrap().lookup_namespace,
                tcl_compiler::registry_invocation::compiled_namespace_context(tokens),
                tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite::from_command_registration(
                    std::sync::Arc::clone(required),
                ),
            );
            Some(end)
        } else {
            None
        };
        if !self.emit_native_original_preparation_receipt(&named.preparations) {
            return false;
        }
        self.emit_named_instruction(tokens, named, &recipe);
        if let Some(end) = selection_end {
            self.place_label(&end);
        }
        self.used_inline_cmd_subst = false;
        true
    }

    fn emit_named_instruction(
        &mut self,
        tokens: &CommandTokens,
        named: &tcl_compiler::command_binding::SourceNamedInvocationProof,
        recipe: &tcl_registry::native_instruction_plan::NativeNamedInvocationInstruction,
    ) {
        use tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand;
        use tcl_registry::native_instruction_plan::NativeNamedInvocationWord;
        let rewrite = recipe.protocol
            == tcl_registry::native_compilation::NativeNamedInvocationProtocol::EnsembleRewrite;
        let expanding = recipe.expanded.iter().any(|expanded| *expanded);
        if expanding {
            self.emit(Op::EXPAND_START, vec![]);
        }
        if !rewrite {
            self.emit_selected_native_command_name(tokens, named);
        }
        for (position, (word, expanded)) in recipe.words.iter().zip(&recipe.expanded).enumerate() {
            match word {
                NativeNamedInvocationWord::Replacement(bytes)
                | NativeNamedInvocationWord::Original(
                    NativeCompilerWordOperand::LiteralExpansion { value: bytes, .. },
                ) => self.push_lit_bytes_exact(bytes),
                NativeNamedInvocationWord::Original(NativeCompilerWordOperand::Original(index)) => {
                    let original = &tokens.words()[*index];
                    let payload = match original {
                        WordExpr::Expand { word, .. } => word.as_ref(),
                        original => original,
                    };
                    let braced = tokens.argv_kinds.get(*index) == Some(&tcl_lexer::TokenType::Str)
                        && tokens
                            .single_token_word
                            .get(*index)
                            .copied()
                            .unwrap_or(false);
                    self.emit_word_from_source(&tokens.argv_texts[*index], braced, Some(payload));
                }
            }
            if *expanded {
                self.emit(
                    Op::EXPAND_STKTOP,
                    vec![Operand::Imm(bytecode_imm(position + 2))],
                );
            }
        }
        if rewrite {
            self.emit_selected_native_command_name(tokens, named);
            self.emit(
                Op::INVOKE_REPLACE,
                vec![
                    Operand::Imm(bytecode_imm(recipe.words.len())),
                    Operand::Imm(bytecode_imm(recipe.arguments_from + 1)),
                ],
            );
        } else if expanding {
            self.emit(Op::INVOKE_EXPANDED, vec![]);
        } else {
            let argc = bytecode_imm(recipe.words.len() + 1);
            self.emit(
                if argc < 256 {
                    Op::INVOKE_STK1
                } else {
                    Op::INVOKE_STK4
                },
                vec![Operand::Imm(argc)],
            );
        }
    }

    fn emit_inline_info_commands_resolve(&mut self, args: &[(String, bool)]) {
        self.used_inline_cmd_subst = true;
        self.emit_native_argument_word(1, &args[1].0, args[1].1);
        self.emit(Op::RESOLVE_CMD, vec![]);
        self.emit(Op::DUP, vec![]);
        self.emit(Op::STR_LEN, vec![]);
        let end = self.fresh_label("info_commands_end");
        self.emit(Op::JUMP_FALSE1, vec![Operand::Label(end.clone())]);
        self.emit(Op::LIST, vec![Operand::Imm(1)]);
        self.place_label(&end);
    }

    /// The original compiler captures its opcode before argv; the operand
    /// still evaluates once and resolves its command in the entered namespace.
    pub(in crate::codegen) fn emit_inline_namespace_origin(
        &mut self,
        tokens: Option<&CommandTokens>,
    ) -> bool {
        let Some(tokens) = tokens else {
            return false;
        };
        let Some(invocation) =
            tcl_compiler::registry_invocation::admitted_native_compiler_invocation(
                self.registry,
                None,
                tokens,
            )
        else {
            return false;
        };
        let Some([word]) = invocation
            .effective_words()
            .words
            .get(1 + invocation.argument_offset()..)
        else {
            return false;
        };
        self.emit_word_from_source(
            &word.legacy_text(),
            matches!(word, WordExpr::BracedLiteral { .. }),
            Some(word),
        );
        self.emit(Op::ORIGIN_CMD, vec![]);
        self.used_inline_cmd_subst = true;
        true
    }

    /// The original literal builder captures the runtime namespace, rather
    /// than evaluating the script or baking in a compiler namespace name.
    pub(in crate::codegen) fn emit_inline_namespace_code(
        &mut self,
        tokens: Option<&CommandTokens>,
    ) -> bool {
        let Some(invocation) = tokens.and_then(|tokens| {
            tcl_compiler::registry_invocation::admitted_native_compiler_invocation(
                self.registry,
                None,
                tokens,
            )
        }) else {
            return false;
        };
        let Some([word]) = invocation
            .effective_words()
            .words
            .get(1 + invocation.argument_offset()..)
        else {
            return false;
        };
        self.push_lit("::namespace");
        self.push_lit("inscope");
        self.emit(Op::CURRENT_NAMESPACE, vec![]);
        self.emit_word_from_source(
            &word.legacy_text(),
            matches!(word, WordExpr::BracedLiteral { .. }),
            Some(word),
        );
        self.emit(Op::LIST, vec![Operand::Imm(4)]);
        self.used_inline_cmd_subst = true;
        true
    }

    fn emit_inline_info_level(&mut self, tokens: Option<&CommandTokens>) -> bool {
        let Some(tokens) = tokens else {
            return false;
        };
        let Some(invocation) =
            tcl_compiler::registry_invocation::admitted_native_compiler_invocation(
                self.registry,
                None,
                tokens,
            )
        else {
            return false;
        };
        let Some(operands) = invocation
            .effective_words()
            .words
            .get(1 + invocation.argument_offset()..)
        else {
            return false;
        };
        match operands {
            [] => {
                self.emit(Op::INFO_LEVEL_NUM, vec![]);
            }
            [word] => {
                self.emit_word_from_source(
                    &word.legacy_text(),
                    matches!(word, WordExpr::BracedLiteral { .. }),
                    Some(word),
                );
                self.emit(Op::INFO_LEVEL_ARGS, vec![]);
            }
            _ => return false,
        }
        self.used_inline_cmd_subst = true;
        true
    }

    /// Emit the selected exists operand from its canonical source coordinate.
    /// A computed name evaluates once before the stack opcode; only a source
    /// word eligible for a direct local slot bypasses substitution.
    fn emit_inline_info_exists(&mut self, tokens: Option<&CommandTokens>) -> bool {
        let Some(tokens) = tokens else {
            return false;
        };
        let Some(invocation) =
            tcl_compiler::registry_invocation::admitted_native_compiler_invocation(
                self.registry,
                None,
                tokens,
            )
        else {
            return false;
        };
        let Some(word) = invocation
            .effective_words()
            .words
            .get(1 + invocation.argument_offset())
        else {
            return false;
        };
        self.used_inline_cmd_subst = true;
        if let Some(slot) = self.emit_original_array_exists_key(word) {
            // `TclCompileInfoExistsCmd` names the array's compiled local and
            // pushes the key.
            self.emit(Op::EXIST_ARRAY, vec![Operand::Imm(bytecode_imm(slot))]);
        } else if let Some(slot) = self.original_scalar_operand_slot(word) {
            self.emit(Op::EXIST_SCALAR, vec![Operand::Imm(bytecode_imm(slot))]);
            self.emit(Op::NOP, vec![]);
        } else {
            let spelling = word.legacy_text();
            self.emit_word_from_source(
                &spelling,
                matches!(word, WordExpr::BracedLiteral { .. }),
                Some(word),
            );
            self.emit(Op::EXIST_STK, vec![]);
        }
        true
    }

    /// Push the key of an array-element operand whose array is a compiled
    /// local, returning that local's slot; emit nothing and answer `None`
    /// for any other operand. The original operand layout decides the key:
    /// a literal key is pushed as is, a compound one is evaluated. A
    /// hand-built context without that layout reads the word's text.
    fn emit_original_array_exists_key(&mut self, word: &WordExpr) -> Option<usize> {
        use tcl_syntax::native_variable_words::NativeVariableWordOperand;
        match self.original_variable_operand(word) {
            Some(NativeVariableWordOperand::Literal {
                name,
                index: Some(index),
                ..
            }) => {
                let slot = self.command_variable_slot(&name)?;
                self.push_lit_bytes_exact(&index);
                Some(slot)
            }
            Some(NativeVariableWordOperand::CompoundArray { name, index, .. }) => {
                let slot = self.command_variable_slot(&name)?;
                self.emit_executable_arena(&index);
                Some(slot)
            }
            None if self.native_entry.is_none()
                && self
                    .invocation_tokens
                    .as_deref()
                    .is_none_or(|tokens| tokens.source_binding.is_none()) =>
            {
                let spelling = word.legacy_text();
                let (base, key) = split_array_ref(&spelling)?;
                if !self.compiles_locals() || is_qualified(base) {
                    return None;
                }
                let slot = self.lvt.intern(base);
                self.push_array_key(key);
                Some(slot)
            }
            Some(_) | None => None,
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
    fn try_emit_inline_string_invoke_replace(
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

    fn emit_inline_string(
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
    fn emit_string_is_generic(
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

    /// Emit only retained original operands of the selected list-index operation.
    fn emit_inline_lindex(&mut self, tokens: Option<&CommandTokens>) -> bool {
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

    fn emit_inline_lrange(&mut self, args: &[(String, bool)]) {
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
    pub(super) fn try_emit_lrange_imm(&mut self, args: &[(String, bool)]) -> bool {
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

    fn emit_inline_lreplace(&mut self, args: &[(String, bool)]) {
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

    fn emit_inline_linsert(&mut self, args: &[(String, bool)]) {
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

    /// `regexp ?switches? pattern string` in a value, compiled where the
    /// release compiles it ([`Self::try_emit_compiled_regexp`]) and the
    /// command's otherwise.
    fn emit_inline_regexp(&mut self, args: &[(String, bool)]) {
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
    fn try_emit_compiled_regexp(&mut self, args: &[(String, bool)]) -> bool {
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
    fn try_emit_string_subcommand(
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

    fn emit_inline_array(&mut self, entered_command: &str, args: &[(String, bool)]) {
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
                .map(tcl_registry::InvocationDialect::of_profile)
                .and_then(|dialect| {
                    dialect.ensemble_implementation_command_address(
                        tcl_registry::EnsembleImplementationFamily::Array,
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

    fn emit_inline_dict_get(&mut self, args: &[(String, bool)]) {
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

// Tests

#[cfg(test)]
mod tests;
