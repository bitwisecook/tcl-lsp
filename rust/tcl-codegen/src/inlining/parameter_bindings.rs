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

//! The statements that bind a spliced body's parameters: positional words, declared defaults and verbatim-splice safety.

use std::collections::HashMap;
use tcl_compiler::ir::{CommandTokens, Procedure, Statement};
use tcl_lexer::{LexerConfig, Span};
use tcl_syntax::word_rules::WordValueRules;

// parameter bindings

/// Build the parameter-binding statement list for a v3 call. Returns
/// `(cid, rename, bindings)` on success, or `None` to decline.
pub(super) fn build_param_bindings(
    call: &Statement,
    proc: &Procedure,
    rules: WordValueRules,
    config: LexerConfig,
    counter: &mut usize,
) -> Option<(usize, HashMap<String, String>, Vec<Statement>)> {
    let Statement::Call {
        args, tokens, span, ..
    } = call
    else {
        return None;
    };
    let span = *span;

    // `{*}` expansion changes call arity at runtime — decline.
    if let Some(t) = tokens
        && let Some(expand) = &t.expand_word
        && expand.iter().skip(1).any(|&b| b)
    {
        return None;
    }

    *counter += 1;
    let cid = *counter;
    let mut rename: HashMap<String, String> = HashMap::new();
    for p in &proc.params {
        rename.insert(p.clone(), format!("{}{p}", super::local_prefix(cid)));
    }

    let params = &proc.params;
    let has_variadic = params.last().is_some_and(|p| p == "args");
    let tokens = tokens.as_ref();

    let mut bound: Vec<String> = Vec::new();
    let mut bound_is_literal: Vec<bool> = Vec::new();
    // The call-site half of a default-filling binding, should either branch
    // below find the call short of its formals.
    let mut site = DefaultBindingSite {
        args,
        tokens,
        positional_count: params.len(),
        has_variadic,
    };

    if has_variadic {
        let positional_count = params.len() - 1;
        if args.len() < positional_count {
            site.positional_count = positional_count;
            return build_with_defaults(cid, &rename, proc, &site, rules);
        }
        for (i, a) in args.iter().enumerate().take(positional_count) {
            bound.push(a.clone());
            bound_is_literal.push(arg_is_braced_literal(tokens, i));
        }
        let extras = &args[positional_count..];
        if extras.is_empty() {
            bound.push(String::new());
            bound_is_literal.push(true);
        } else {
            for i in positional_count..args.len() {
                if arg_is_braced_literal(tokens, i) {
                    return None;
                }
            }
            for w in extras {
                // An empty extra word (e.g. the quoted `""` in `v ""`) would
                // collapse inside `[list …]` — `[list ]` is a zero-element
                // list, dropping the original one-element value. Decline so
                // the call falls back to runtime dispatch with correct arity.
                if w.is_empty() || !list_clean_for_splice(w, config) {
                    return None;
                }
            }
            bound.push(format!("[list {}]", extras.join(" ")));
            bound_is_literal.push(false);
        }
    } else {
        if args.len() > params.len() {
            return None;
        }
        if args.len() < params.len() {
            return build_with_defaults(cid, &rename, proc, &site, rules);
        }
        for (i, a) in args.iter().enumerate() {
            bound.push(a.clone());
            bound_is_literal.push(arg_is_braced_literal(tokens, i));
        }
    }

    let mut bindings: Vec<Statement> = Vec::with_capacity(params.len());
    for ((p, v), is_literal) in params.iter().zip(&bound).zip(&bound_is_literal) {
        let name = rename[p].clone();
        if *is_literal {
            bindings.push(Statement::AssignConst {
                span,
                name,
                name_braced: false,
                value: v.clone(),
                value_span: None,
            });
        } else {
            bindings.push(Statement::AssignValue {
                span,
                name,
                name_braced: false,
                value: v.clone(),
                value_needs_backsubst: v.contains('\\'),
                tokens: None,
            });
        }
    }
    Some((cid, rename, bindings))
}

/// Whether the call-site word at `args[idx]` was a braced literal (`{…}`)
/// in source — so it must bind as an [`Statement::AssignConst`], preserving
/// embedded `$`/`[` rather than re-substituting them at the inlined site.
///
/// Thin wrapper over the shared [`CommandTokens::arg_is_braced_literal`] for
/// the `Option<&CommandTokens>` the inliner carries.
pub(super) fn arg_is_braced_literal(tokens: Option<&CommandTokens>, idx: usize) -> bool {
    tokens.is_some_and(|t| t.arg_is_braced_literal(idx))
}

/// Try to fill missing positional args from declared defaults.
///
/// Matches the non-default path's literal binding: a call-site word that
/// was a braced literal binds as [`Statement::AssignConst`], and so does a
/// declared default — Tcl proc defaults are literal (`proc p {{a $x}} …; p`
/// binds `a` to the string `$x`, never the caller's `$x`). Without this the
/// default path would re-substitute both in the caller frame.
/// The call-site half of a default-filling parameter binding: the words the
/// caller wrote, their token shapes, and the arity facts derived from the
/// callee's formals.
struct DefaultBindingSite<'a> {
    pub(super) args: &'a [String],
    pub(super) tokens: Option<&'a CommandTokens>,
    positional_count: usize,
    has_variadic: bool,
}

fn build_with_defaults(
    cid: usize,
    rename: &HashMap<String, String>,
    proc: &Procedure,
    site: &DefaultBindingSite<'_>,
    rules: WordValueRules,
) -> Option<(usize, HashMap<String, String>, Vec<Statement>)> {
    let DefaultBindingSite {
        args,
        tokens,
        positional_count,
        has_variadic,
    } = *site;
    if proc.params_raw.is_empty() {
        return None;
    }
    let parsed = parse_params_with_defaults(&proc.params_raw, rules)?;
    if parsed.len() != proc.params.len() {
        return None; // parse drift — bail safely
    }

    let mut bound: Vec<String> = Vec::new();
    let mut bound_is_literal: Vec<bool> = Vec::new();
    for i in 0..positional_count {
        if i < args.len() {
            bound.push(args[i].clone());
            bound_is_literal.push(arg_is_braced_literal(tokens, i));
            continue;
        }
        let default = parsed[i].1.as_ref()?;
        bound.push(default.clone());
        bound_is_literal.push(true); // defaults are literal
    }
    if has_variadic {
        bound.push(String::new());
        bound_is_literal.push(true); // empty literal
    }

    let bindings: Vec<Statement> = proc
        .params
        .iter()
        .zip(&bound)
        .zip(&bound_is_literal)
        .map(|((p, v), is_literal)| {
            let name = rename[p].clone();
            if *is_literal {
                Statement::AssignConst {
                    span: proc.span,
                    name,
                    name_braced: false,
                    value: v.clone(),
                    value_span: None,
                }
            } else {
                Statement::AssignValue {
                    span: proc.span,
                    name,
                    name_braced: false,
                    value: v.clone(),
                    value_needs_backsubst: v.contains('\\'),
                    tokens: None,
                }
            }
        })
        .collect();
    Some((cid, rename.clone(), bindings))
}

/// Parse a valid Tcl proc `param_str` into `(name, default_or_None)` pairs.
///
/// Inlining is an execution transform, so it uses the shared strict proc
/// grammar.  A malformed declaration declines the transform and remains on the
/// ordinary runtime path, where Tcl reports the precise creation error.
pub(super) fn parse_params_with_defaults(
    param_str: &str,
    rules: WordValueRules,
) -> Option<Vec<(String, Option<String>)>> {
    tcl_compiler::signature_scan::params::parse_param_list_strict_in(
        param_str,
        rules,
        tcl_dialect::ParameterGrammar::Tcl,
    )
    .ok()
    .map(|parameters| {
        parameters
            .into_iter()
            .map(|parameter| (parameter.name, parameter.default))
            .collect()
    })
}

/// Return True iff `text` can be safely spliced verbatim into a Tcl
/// `[list <e1> <e2> …]` synth as a single element.
pub(super) fn list_clean_for_splice(text: &str, config: LexerConfig) -> bool {
    use tcl_lexer::word_parts::{SubstFlags, WordPart, decompose_spanned_checked};

    let Ok(parts) = decompose_spanned_checked(text.as_bytes(), SubstFlags::default(), config)
    else {
        return false;
    };
    if parts
        .iter()
        .any(|part| matches!(part.part, WordPart::ParseError(_)))
    {
        return false;
    }
    // This generated word is lexical emission advice. The placeholder command
    // selects no worker, source lookup or Native compilation entry.
    let spelling = format!("__inline_list_argument {text}");
    let Ok(end) = u32::try_from(spelling.len()) else {
        return false;
    };
    let Ok(plan) = tcl_lexer::native_script_words_in(
        tcl_lexer::SourceImage::document(&spelling),
        Span::new(0, end),
        config,
    ) else {
        return false;
    };
    let [command] = plan.commands.as_slice() else {
        return false;
    };
    let [_, word] = command.words.as_slice() else {
        return false;
    };
    plan.fatal_tail.is_none()
        && word.written_bytes() == text.as_bytes()
        && word.span() == word.word_span()
        && word.tokens().iter().all(|token| {
            token.kind != tcl_lexer::TokenType::Str
                && !token.in_quote
                && !(token.kind == tcl_lexer::TokenType::Esc && token.content_offset != 0)
        })
}
