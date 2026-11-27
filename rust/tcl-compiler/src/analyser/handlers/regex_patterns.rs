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

//! The pattern arguments of regular-expression commands.

use super::super::state::Analyser;
use tcl_lexer::{Token, TokenType};

impl Analyser {
    /// Registry-owned pattern layout over original typed argument words.
    pub(in crate::analyser) fn regex_pattern_source_index(
        &self,
        command: &str,
        arg_tokens: &[Token],
        command_token: Token,
    ) -> Option<usize> {
        let context = self.analysis_context();
        let mut dialect = self
            .source_analysis_entry
            .as_ref()
            .and_then(|entry| entry.invocation_dialect)
            .unwrap_or_else(|| tcl_native_model::InvocationDialect::of_profile(self.profile));
        dialect.lexer_grammar = self.grammar();
        dialect.word_values = self.word_rules();
        let source_map = self.cached_source_map();
        let end = tcl_lexer::word_span(&source_map, *arg_tokens.last()?).end();
        let start = command_token.span.start();
        let text = self.source.get(start as usize..end as usize)?;
        let segmented = tcl_source::segment::segment_commands_with_offset_and_config(
            text,
            start,
            self.file_lexer_config().nested(),
        )
        .into_iter()
        .next()?;
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &source_map,
            self.file_lexer_config(),
            &segmented,
        );
        self.head_identities
            .source_bindings_ref()
            .stamp_original_tokens(&mut tokens);
        if self.result.allows_lexical_declaration_advice() {
            let words = crate::registry_invocation::source_structure::source_registry_words(
                &self.source,
                &self.result,
                &segmented,
            )?;
            let patterns = words.with_source_schema(&context, |schema| {
                schema.authored_source_pattern_arguments()
            })??;
            let mut selected = patterns
                .into_iter()
                .filter(|pattern| pattern.kind == tcl_registry::patterns::PatternType::Regex);
            let pattern = selected.next()?;
            if selected.next().is_some() {
                return None;
            }
            let crate::registry_invocation::InvocationWordOrigin::Written(written) = words
                .origins()
                .get(usize::from(pattern.index).checked_add(1)?)?
            else {
                return None;
            };
            return written.checked_sub(1);
        }
        if tokens.source_binding.is_some() {
            return crate::registry_invocation::normal_representation_invocation_in_context(
                &context, &tokens,
            )?
            .pattern_source_argument_index(context.commands());
        }
        crate::regex_source::source_pattern_index(
            &self.source,
            context.commands(),
            command,
            arg_tokens,
            self.file_lexer_config(),
            dialect,
        )
    }

    /// Record the pattern arguments of regex-pattern commands
    /// (`PatternType::Regex` specs — `regexp` / `regsub`) for syntax
    /// highlighting.
    ///
    /// Literal patterns (`Esc` / `Str` tokens) are recorded
    /// verbatim; variable patterns use checked Logical declaration and value
    /// provenance to retain the original value and its source geometry.
    /// `Cmd`-substitution patterns are skipped — runtime-computed
    /// patterns can't be statically resolved.
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::RegexPatternCapture`]
    /// (stamped on both `regexp` and `regsub`); `cmd_name` is data —
    /// it labels the recorded pattern — not a guard.
    pub fn handle_regex_pattern_capture(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
        command_token: Token,
    ) {
        if args.is_empty() {
            return;
        }
        let Some(idx) = self.regex_pattern_source_index(cmd_name, arg_tokens, command_token) else {
            return;
        };
        if idx >= arg_tokens.len() {
            return;
        }
        let tok = arg_tokens[idx];
        self.record_regex_pattern_token(&args[idx], tok, cmd_name, scope_path);
    }

    /// Record one regex-pattern token for `regexp` / `regsub` /
    /// `switch -regexp`. Variables use checked original value provenance;
    /// command substitutions supply no static pattern, and literals retain their bytes.
    pub(super) fn record_regex_pattern_token(
        &mut self,
        text: &str,
        tok: Token,
        command: &str,
        scope_path: &[usize],
    ) {
        match tok.kind {
            TokenType::Cmd => {
                // Runtime-computed patterns can't be statically
                // resolved — skip.  Only ``Var`` tokens are
                // resolved as a substitution branch.
            }
            TokenType::Var => {
                let sm = Analyser::source_map(
                    &self.source,
                    &self.cached_line_index,
                    self.cached_line_index_source_len,
                );
                let var_name = sm.token_text(tok).to_string();
                let constant = self
                    .lookup_logical_const_substitution(tok)
                    .map(|(value, span)| (value.to_owned(), span))
                    .or_else(|| self.frozen_regex_pattern_definition(tok, &var_name, scope_path));
                if let Some((pattern, def_span)) = constant {
                    // Record the use site (the `$var` token).
                    self.result
                        .regex_patterns
                        .push(super::super::types::RegexPattern {
                            range: tok.span,
                            pattern: pattern.clone(),
                            command: command.to_string(),
                        });
                    // Also record the defining ``set`` value's
                    // range so the semantic-token provider can
                    // highlight the literal.
                    self.result
                        .regex_patterns
                        .push(super::super::types::RegexPattern {
                            range: def_span,
                            pattern,
                            command: command.to_string(),
                        });
                    self.regex_vars.insert((scope_path.to_vec(), var_name));
                }
            }
            _ => {
                self.result
                    .regex_patterns
                    .push(super::super::types::RegexPattern {
                        range: tok.span,
                        pattern: text.to_string(),
                        command: command.to_string(),
                    });
            }
        }
    }

    /// On a profile whose words the interpretation evaluates, the value it
    /// froze for a plain `$name` pattern word, with the span of the
    /// dominating constant assignment in the same scope that wrote exactly
    /// that value before the read.
    fn frozen_regex_pattern_definition(
        &self,
        tok: Token,
        var_name: &str,
        scope_path: &[usize],
    ) -> Option<(String, tcl_lexer::Span)> {
        let name =
            tcl_syntax::naming::var_reference_for_style(var_name, self.lexer_config().braced_var);
        if tcl_syntax::naming::split_array_name(name).1.is_some()
            || !tcl_syntax::naming::source_variable_name_is_frame_local_for_all_policies(
                name.as_bytes(),
            )
        {
            return None;
        }
        let value = self.frozen_value_word(tok)?;
        if self
            .nondominating_consts
            .get(scope_path)
            .is_some_and(|names| names.contains(name))
        {
            return None;
        }
        let (written, span) = self.const_strings.get(scope_path)?.get(name)?;
        (*written == value && span.end() < tok.span.start()).then_some((value, *span))
    }
}
