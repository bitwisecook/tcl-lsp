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

//! Shared Tcl value-shape helpers used across compiler passes.

use tcl_lexer::{Lexer, Span, TokenType};

/// Parse one authored Tcl word through the canonical word model. Multiple
/// words or commands do not supply a value shape. The authored word is an
/// extracted operand; its local parser keeps a leading BOM as content while
/// the word model retains the complete parent configuration.
#[must_use]
pub fn value_word_with_config(
    text: &str,
    config: tcl_lexer::LexerConfig,
) -> Option<crate::ir::WordExpr> {
    let mut commands = crate::segmenter::segment_commands_with_offset_and_config(
        text,
        0,
        tcl_lexer::SourceSegmentationPurpose::NestedScript.config(config),
    );
    if commands.len() != 1 {
        return None;
    }
    let command = commands.pop()?;
    let tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(text),
        config,
        &command,
    );
    (tokens.words().len() == 1).then(|| tokens.words()[0].clone())
}

/// Retain the exact lookup evidence of commands inside a sole substitution.
/// An unrecorded nested site inherits explicit uncertainty from its parent.
#[must_use]
pub fn command_substitution_tokens(
    word: &crate::ir::WordExpr,
    parent: Option<&crate::ir::CommandTokens>,
    config: tcl_lexer::LexerConfig,
) -> Option<Vec<crate::ir::CommandTokens>> {
    let (spelling, site) = word.sole_command_substitution()?;
    let inner = spelling.strip_prefix('[')?.strip_suffix(']')?;
    let base = site.span.start().checked_add(1)?;
    let map = tcl_lexer::SourceMap::new(inner).with_base(base, 0, 0);
    Some(
        crate::segmenter::segment_commands_with_offset_and_config(inner, base, config.nested())
            .iter()
            .map(|command| {
                let mut tokens =
                    crate::ir::CommandTokens::from_segmented(&map, config.nested(), command);
                if let Some(parent) = parent {
                    tokens.inherit_nested_bindings(parent);
                }
                tokens
            })
            .collect(),
    )
}

/// Whether the complete fragment is one variable reference under its source grammar.
#[must_use]
pub fn is_pure_var_ref(text: &str, config: tcl_lexer::LexerConfig) -> bool {
    tcl_syntax::naming::is_whole_variable_reference(text, config)
}

/// The literal scalar name of one whole reference under its source grammar.
///
/// ```
/// use tcl_compiler::value_shapes::whole_word_scalar_var_name;
/// let config = tcl_lexer::LexerConfig::default();
/// assert_eq!(whole_word_scalar_var_name("$cmd", config), Some("cmd"));
/// assert_eq!(whole_word_scalar_var_name("${cmd}", config), Some("cmd"));
/// assert_eq!(whole_word_scalar_var_name("$ns::cmd", config), Some("ns::cmd"));
/// assert_eq!(whole_word_scalar_var_name("$a(k)", config), None);
/// assert_eq!(whole_word_scalar_var_name("pre$cmd", config), None);
/// assert_eq!(whole_word_scalar_var_name("cmd", config), None);
/// ```
/// This projection supplies no cell, read value or source applicability.
#[must_use]
pub fn whole_word_scalar_var_name(text: &str, config: tcl_lexer::LexerConfig) -> Option<&str> {
    tcl_syntax::naming::whole_scalar_variable_reference_name(text, config)
}

/// Extract command name and args from `[cmd arg1 arg2 …]`.
///
/// Returns `None` when `text` is not bracket-wrapped, or when the
/// inside is empty. Arguments are split on whitespace at the *top* nesting
/// level only: `[...]`, `{...}` and `"..."` groups keep their inner
/// whitespace, so a nested command substitution like `[list [read $fd]]`
/// yields the single arg `[read $fd]` rather than the two broken halves
/// `[read` and `$fd]` (which would hide a taint source nested in an
/// argument). Callers that need full Tcl list quoting handle it upstream.
#[must_use]
pub fn parse_command_substitution_with_config(
    text: &str,
    config: tcl_lexer::LexerConfig,
) -> Option<(String, Vec<String>)> {
    let (command, args) = parse_command_substitution_with_spans_and_config(text, config)?;
    Some((command, args.into_iter().map(|(text, _)| text).collect()))
}

/// Like [`parse_command_substitution_with_config`], but additionally returns each
/// argument's byte span *within `text`* (the caller adds its own base
/// offset to get an absolute source position) — for diagnostics that need
/// to anchor on the specific offending argument rather than the whole
/// substitution.
///
/// Uses the real [`Lexer`] rather than a hand-rolled bracket/brace
/// counter, so word boundaries follow Tcl's actual quoting rules (this
/// also makes it correct on inputs a naive counter gets wrong, e.g. a
/// `#` comment or an escaped delimiter at the top level). Returns `None`
/// for the same shapes as
/// [`parse_command_substitution_with_config`], plus when the bracket body doesn't lex
/// as a single command (an embedded `;`/newline followed by more words).
#[must_use]
pub fn parse_command_substitution_with_spans_and_config(
    text: &str,
    config: tcl_lexer::LexerConfig,
) -> Option<(String, Vec<(String, Span)>)> {
    let leading_ws = u32::try_from(text.len() - text.trim_start().len()).unwrap_or(u32::MAX);
    let stripped = text.trim();
    let inner = stripped.strip_prefix('[')?.strip_suffix(']')?;
    // Offset of `inner`'s first byte within `text`: leading whitespace + `[`.
    let base = leading_ws + 1;

    let tokens = Lexer::with_config(inner, config.nested())
        .tokenise_all()
        .ok()?;
    // One `SourceMap` for the whole scan — `widened_token_text` needs the
    // lexer's content geometry to widen a word over its closing delimiter,
    // and building a map per token would re-index `inner`'s lines each time.
    let sm = tcl_lexer::SourceMap::new(inner);
    let mut words: Vec<(String, Span)> = Vec::new();
    let mut prev_kind = TokenType::Eol;
    let mut saw_terminator = false;

    for (idx, tok) in tokens.iter().enumerate() {
        match tok.kind {
            TokenType::Comment | TokenType::Sep => {
                prev_kind = tok.kind;
                continue;
            }
            TokenType::Eol => {
                if !words.is_empty() {
                    saw_terminator = true;
                }
                prev_kind = tok.kind;
                continue;
            }
            TokenType::Eof => break,
            _ => {}
        }
        if saw_terminator {
            return None;
        }
        let (word_text, tok_end) = widened_token_text(inner, &sm, tok)?;
        // Two closing delimiters the raw token stream can leave
        // unclaimed by any token's span, needing manual recovery here:
        //
        // - The `in_quote` closing `"` — unlike `Str`/`Cmd`, there is no
        //   dedicated token kind for a double-quoted word to attach
        //   `group_closer()` to, so a trailing `"` immediately after the
        //   last fragment of a quoted word (e.g. `"/api/*"`, or `"a[foo]"`
        //   right after the nested command substitution's own widened
        //   `]`) is silently dropped.
        // - A non-degenerate braced-var's closing `}` (`${p}`) — `Var`
        //   also covers the unbraced `$name` form, which has no closer at
        //   all, so `group_closer()` can't unconditionally claim `}` for
        //   the whole kind the way it does for `Str`/`Cmd`.
        //
        // Only claim the byte when the *next* token doesn't already start
        // there (e.g. `"a$b"` emits the closing `"` as its own adjacent
        // token — nothing orphaned to recover in that shape, and claiming
        // it here too would double it).
        let next_start = tokens.get(idx + 1).map(|t| t.span.start());
        let orphaned_closer = match inner.as_bytes().get(tok_end as usize) {
            Some(b'"') => true,
            Some(b'}') if tok.kind == TokenType::Var => true,
            _ => false,
        } && next_start != Some(tok_end);
        let (word_text, tok_end) = if orphaned_closer {
            let start = usize::try_from(tok.span.start()).ok()?;
            let widened_end = tok_end as usize + 1;
            (
                inner.get(start..widened_end)?.to_owned(),
                u32::try_from(widened_end).unwrap_or(u32::MAX),
            )
        } else {
            (word_text, tok_end)
        };
        let abs_span = Span::new(base + tok.span.start(), base + tok_end);
        // segmentation-drift-ok: a private word grouper over the `[...]`
        // interior, kept because it also recovers the two orphaned closers
        // documented above, which `WordSpan` does not carry.  See the tcl-lexer
        // owner section of shared-utility-contracts-rust.md.
        if matches!(prev_kind, TokenType::Sep | TokenType::Eol) || words.is_empty() {
            words.push((word_text, abs_span));
        } else {
            // Adjacent tokens with no separator concatenate onto the
            // previous word (e.g. `$a$b` or `foo$x`).
            //
            // Extend by slicing `inner` from where the word so far ends,
            // never by appending this token's own raw text: raw token spans
            // *overlap* when a quoted word opens directly on a substitution.
            // `parse_quoted`'s documented empty-content clamp extends a
            // zero-content `Esc` one byte over the `$`/`[` that stopped the
            // scan — so `"[lindex $x 0]"` lexes `Esc 5..7` (`"[`) followed by
            // `Cmd 6..18`, and both claim the `[`. The clamp is the lexer's
            // convention, resolved there by `SourceMap::token_text`; a
            // consumer that wants the raw source spelling has to reconcile
            // it. Appending raw text emitted the shared byte twice
            // (`"[[lindex $x 0]"`), leaving the text disagreeing with its own
            // (correct) span. Slicing the gap keeps the two in lockstep by
            // construction: the word's text is always exactly the source
            // covered by the word's span.
            let (last_text, last_span) =
                words.last_mut().expect("words is non-empty in this branch");
            let joined_end = abs_span.end().max(last_span.end());
            if joined_end > last_span.end() {
                let from = usize::try_from(last_span.end() - base).ok()?;
                let to = usize::try_from(joined_end - base).ok()?;
                last_text.push_str(inner.get(from..to)?);
            }
            *last_span = Span::new(last_span.start(), joined_end);
        }
        prev_kind = tok.kind;
    }

    let mut it = words.into_iter();
    let (cmd, _cmd_span) = it.next()?;
    Some((cmd, it.collect()))
}

/// Return `tok`'s text and exclusive end offset (both relative to `inner`),
/// widened to include a `{…}` / `[…]` word's closing delimiter when the raw
/// token span omits it.
///
/// The widening itself — and the inner-end convention it compensates for —
/// is [`tcl_lexer::word_span`]'s single authoritative answer; this only
/// slices `inner` with it.  A local copy of the arithmetic, discriminating on
/// "does the extracted text already end with the closer?", silently drops the
/// real closer of a word whose last *inner* byte is one (`{a\}}`).  `sm` must map `inner`, the same frame `tok` is in.
///
/// The braced/bracketed type gate is kept: a quoted `"…"` word's closing
/// quote is recovered by the caller's `orphaned_closer` step instead, which
/// also handles the closer of a braced *variable* (`${p}`) — neither is
/// derivable from the token *kind*, so neither belongs to this arm.
fn widened_token_text(
    inner: &str,
    sm: &tcl_lexer::SourceMap<'_>,
    tok: &tcl_lexer::Token,
) -> Option<(String, u32)> {
    let span = if tok.kind.group_closer().is_some() {
        tcl_lexer::word_span(sm, *tok)
    } else {
        tok.span
    };
    let text = inner.get(span.start() as usize..span.end() as usize)?;
    Some((text.to_owned(), span.end()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_bracket_projection_keeps_body_bom_and_original_child_coordinates() {
        // naming.source.original-configured-script-completeness

        // Genuine original parent/child word geometry; no evaluated child result/admission.
        let mut prepared = crate::analyser::Analyser::new();
        let input = prepared.prepare_analysis_input("tcl9.1").unwrap();
        let source = "puts [\u{feff}unknown argument]";
        let identities =
            crate::realm::document_realm_bindings_with_resolved_input(source, &input, None);
        let command = crate::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            input.lexer_config(),
        )
        .pop()
        .unwrap();
        let mut parent = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            input.lexer_config(),
            &command,
        );
        identities
            .source_bindings_ref()
            .stamp_original_tokens(&mut parent);
        let child =
            command_substitution_tokens(&parent.words()[1], Some(&parent), input.lexer_config())
                .unwrap();
        assert_eq!(child.len(), 1);
        assert_eq!(child[0].argv_texts, ["\u{feff}unknown", "argument"]);
        assert_eq!(child[0].argv[0].start(), 6);
        assert_eq!(
            source.get(child[0].argv[0].as_range()),
            Some("\u{feff}unknown")
        );
        assert_eq!(child[0].nested_bindings, parent.nested_bindings);
        let site = child[0]
            .source_binding
            .as_ref()
            .unwrap()
            .invocation_site()
            .unwrap();
        assert_eq!(site.offset, 6);
        assert_eq!(site.source.source_image().bytes(), source.as_bytes());
        let (head, arguments) = parse_command_substitution_with_spans_and_config(
            "[\u{feff}unknown argument]",
            input.lexer_config(),
        )
        .unwrap();
        assert_eq!(head, "\u{feff}unknown");
        assert_eq!(arguments[0].0, "argument");
        assert_eq!(arguments[0].1.start(), 12);
    }

    #[test]
    fn pure_var_bare_and_braced() {
        assert!(is_pure_var_ref("$x", tcl_lexer::LexerConfig::default()));
        assert!(is_pure_var_ref("${x}", tcl_lexer::LexerConfig::default()));
        assert!(is_pure_var_ref(
            "$foo::bar",
            tcl_lexer::LexerConfig::default()
        ));
    }

    #[test]
    fn pure_var_rejects_extras() {
        assert!(!is_pure_var_ref(
            "$x extra",
            tcl_lexer::LexerConfig::default()
        ));
        assert!(!is_pure_var_ref(
            "hello$x",
            tcl_lexer::LexerConfig::default()
        ));
        assert!(!is_pure_var_ref(
            "$x[nested]",
            tcl_lexer::LexerConfig::default()
        ));
        assert!(!is_pure_var_ref(
            "\"$x\"",
            tcl_lexer::LexerConfig::default()
        ));
        assert!(!is_pure_var_ref("x", tcl_lexer::LexerConfig::default()));
    }

    #[test]
    fn pure_var_rejects_unbalanced_braces() {
        assert!(!is_pure_var_ref("${x}y", tcl_lexer::LexerConfig::default()));
    }

    #[test]
    fn pure_var_accepts_array_index() {
        assert!(is_pure_var_ref(
            "$arr(idx)",
            tcl_lexer::LexerConfig::default()
        ));
        assert!(is_pure_var_ref(
            "$arr(key)",
            tcl_lexer::LexerConfig::default()
        ));
        assert!(is_pure_var_ref(
            "${a(1)}",
            tcl_lexer::LexerConfig::default()
        ));
        // Backslash-escaped close paren stays inside the one index.
        assert!(is_pure_var_ref(
            "$a(x\\)y)",
            tcl_lexer::LexerConfig::default()
        ));
    }

    #[test]
    fn fp_nab_12_escaped_paren_array_index_companions() {
        // Escaped close parentheses remain inside the original index.
        assert!(is_pure_var_ref(
            r"$a(x\)y)",
            tcl_lexer::LexerConfig::default()
        ));
        // Companion controls locking in overall parser correctness.
        assert!(is_pure_var_ref("$x", tcl_lexer::LexerConfig::default()));
        assert!(is_pure_var_ref(
            "${some name}",
            tcl_lexer::LexerConfig::default()
        ));
        assert!(is_pure_var_ref("$ns::x", tcl_lexer::LexerConfig::default()));
        assert!(is_pure_var_ref(
            "$arr(plain)",
            tcl_lexer::LexerConfig::default()
        ));
        // Inverse: an *unescaped* `)` terminates the index, leaving trailing
        // `y` as concatenated literal — NOT one pure var ref.
        assert!(!is_pure_var_ref(
            "$a(x)y",
            tcl_lexer::LexerConfig::default()
        ));
    }

    #[test]
    fn pure_var_rejects_concatenation_and_trailing_text() {
        // Over-permissive cases a byte-set heuristic would accept.
        assert!(!is_pure_var_ref("$x$y", tcl_lexer::LexerConfig::default()));
        assert!(!is_pure_var_ref(
            "$x.foo",
            tcl_lexer::LexerConfig::default()
        ));
        assert!(!is_pure_var_ref("$x_$y", tcl_lexer::LexerConfig::default()));
        // Unterminated array index is not a pure reference.
        assert!(!is_pure_var_ref(
            "$arr(idx",
            tcl_lexer::LexerConfig::default()
        ));
    }

    #[test]
    fn braced_whole_name_array_ref_detection() {
        let is_braced_whole_name_array_ref = |text: &str| {
            text.starts_with("${")
                && tcl_lexer::word_parts::whole_var_ref(
                    text.as_bytes(),
                    tcl_lexer::LexerConfig::default(),
                )
                .ok()
                .flatten()
                .is_some_and(|reference| {
                    reference.index.is_none()
                        && tcl_syntax::naming::split_element_ref_bytes(reference.name).is_some()
                })
        };
        assert!(is_braced_whole_name_array_ref("${a(1)}"));
        assert!(is_braced_whole_name_array_ref("${arr(key)}"));
        // Plain braced scalar is not an array-shaped ref.
        assert!(!is_braced_whole_name_array_ref("${foo}"));
        assert!(!is_braced_whole_name_array_ref("$arr(1)"));
        assert!(!is_braced_whole_name_array_ref("${a(1)}x"));
    }

    #[test]
    fn parse_command_substitution_basic() {
        let (cmd, args) = parse_command_substitution_with_config(
            "[llength $x]",
            tcl_lexer::LexerConfig::default(),
        )
        .unwrap();
        assert_eq!(cmd, "llength");
        assert_eq!(args, vec!["$x".to_string()]);
    }

    #[test]
    fn parse_command_substitution_with_whitespace() {
        let (cmd, args) = parse_command_substitution_with_config(
            "  [ set x 42 ]  ",
            tcl_lexer::LexerConfig::default(),
        )
        .unwrap();
        assert_eq!(cmd, "set");
        assert_eq!(args, vec!["x".to_string(), "42".into()]);
    }

    #[test]
    fn parse_command_substitution_rejects_non_bracketed() {
        assert!(
            parse_command_substitution_with_config("llength $x", tcl_lexer::LexerConfig::default())
                .is_none()
        );
        assert!(
            parse_command_substitution_with_config("[empty ", tcl_lexer::LexerConfig::default())
                .is_none()
        );
        assert!(
            parse_command_substitution_with_config("[]", tcl_lexer::LexerConfig::default())
                .is_none()
        );
    }

    #[test]
    fn parse_command_substitution_no_args() {
        let (cmd, args) =
            parse_command_substitution_with_config("[pwd]", tcl_lexer::LexerConfig::default())
                .unwrap();
        assert_eq!(cmd, "pwd");
        assert_eq!(args, [] as [std::string::String; 0]);
    }

    #[test]
    fn spans_basic_arg_offsets_are_exact() {
        let text = "[lindex $x 0]";
        let (cmd, args) = parse_command_substitution_with_spans_and_config(
            text,
            tcl_lexer::LexerConfig::default(),
        )
        .unwrap();
        assert_eq!(cmd, "lindex");
        assert_eq!(args.len(), 2);
        let (x_text, x_span) = &args[0];
        assert_eq!(x_text, "$x");
        assert_eq!(&text[x_span.start() as usize..x_span.end() as usize], "$x");
        let (zero_text, zero_span) = &args[1];
        assert_eq!(zero_text, "0");
        assert_eq!(
            &text[zero_span.start() as usize..zero_span.end() as usize],
            "0"
        );
    }

    #[test]
    fn spans_account_for_leading_whitespace_and_inner_padding() {
        let text = "  [ set x 42 ]  ";
        let (cmd, args) = parse_command_substitution_with_spans_and_config(
            text,
            tcl_lexer::LexerConfig::default(),
        )
        .unwrap();
        assert_eq!(cmd, "set");
        let (x_text, x_span) = &args[0];
        assert_eq!(x_text, "x");
        assert_eq!(&text[x_span.start() as usize..x_span.end() as usize], "x");
        let (v_text, v_span) = &args[1];
        assert_eq!(v_text, "42");
        assert_eq!(&text[v_span.start() as usize..v_span.end() as usize], "42");
    }

    #[test]
    fn spans_none_for_non_bracketed_or_empty() {
        assert!(
            parse_command_substitution_with_spans_and_config(
                "llength $x",
                tcl_lexer::LexerConfig::default()
            )
            .is_none()
        );
        assert!(
            parse_command_substitution_with_spans_and_config(
                "[]",
                tcl_lexer::LexerConfig::default()
            )
            .is_none()
        );
    }

    #[test]
    fn spans_none_for_multiple_top_level_commands() {
        // A second command after a newline/`;` inside the substitution body
        // is not a single-command shape this helper models.
        assert!(
            parse_command_substitution_with_spans_and_config(
                "[set x 1; set y 2]",
                tcl_lexer::LexerConfig::default()
            )
            .is_none()
        );
    }

    #[test]
    fn spans_nested_command_substitution_keeps_full_bracket_text() {
        let text = "[list [read $fd]]";
        let (cmd, args) = parse_command_substitution_with_spans_and_config(
            text,
            tcl_lexer::LexerConfig::default(),
        )
        .unwrap();
        assert_eq!(cmd, "list");
        assert_eq!(args.len(), 1);
        let (nested_text, nested_span) = &args[0];
        assert_eq!(nested_text, "[read $fd]");
        assert_eq!(
            &text[nested_span.start() as usize..nested_span.end() as usize],
            "[read $fd]"
        );
    }

    #[test]
    fn spans_braced_arg_keeps_full_brace_text() {
        let text = "[list {a b c}]";
        let (cmd, args) = parse_command_substitution_with_spans_and_config(
            text,
            tcl_lexer::LexerConfig::default(),
        )
        .unwrap();
        assert_eq!(cmd, "list");
        let (braced_text, braced_span) = &args[0];
        assert_eq!(braced_text, "{a b c}");
        assert_eq!(
            &text[braced_span.start() as usize..braced_span.end() as usize],
            "{a b c}"
        );
    }

    #[test]
    fn spans_empty_group_arg_not_overwidened() {
        let text = "[list {} x]";
        let (cmd, args) = parse_command_substitution_with_spans_and_config(
            text,
            tcl_lexer::LexerConfig::default(),
        )
        .unwrap();
        assert_eq!(cmd, "list");
        assert_eq!(args.len(), 2);
        let (empty_text, empty_span) = &args[0];
        assert_eq!(empty_text, "{}");
        assert_eq!(
            &text[empty_span.start() as usize..empty_span.end() as usize],
            "{}"
        );
        let (x_text, x_span) = &args[1];
        assert_eq!(x_text, "x");
        assert_eq!(&text[x_span.start() as usize..x_span.end() as usize], "x");
    }

    /// A non-degenerate braced variable reference (`${p}`, real name
    /// inside the braces) keeps its closing `}` — the raw `Var` token's
    /// span excludes it (same inner-end convention as `Str`/`Cmd`), and
    /// `Var` has no `group_closer()` entry since the *unbraced* `$name`
    /// form has no closer at all to unconditionally claim.
    #[test]
    fn spans_braced_var_keeps_closing_brace() {
        let text = "[file normalize ${p}]";
        let (cmd, args) = parse_command_substitution_with_spans_and_config(
            text,
            tcl_lexer::LexerConfig::default(),
        )
        .unwrap();
        assert_eq!(cmd, "file");
        assert_eq!(args[1].0, "${p}");
        assert_eq!(
            &text[args[1].1.start() as usize..args[1].1.end() as usize],
            "${p}"
        );
    }

    /// Control: an unbraced `$name` reference must not gain a spurious
    /// trailing `}` from the fix above.
    #[test]
    fn spans_unbraced_var_unaffected() {
        let text = "[file normalize $p]";
        let (cmd, args) = parse_command_substitution_with_spans_and_config(
            text,
            tcl_lexer::LexerConfig::default(),
        )
        .unwrap();
        assert_eq!(cmd, "file");
        assert_eq!(args[1].0, "$p");
    }

    /// A double-quoted word with no embedded substitution keeps its
    /// closing `"` in the returned text — the lexer's raw token span for
    /// the final content fragment excludes it (there is no dedicated
    /// token kind for a quoted word to attach `group_closer()` to, unlike
    /// `Str`/`Cmd`), so it must be recovered from the one-byte gap that
    /// widening otherwise leaves unclaimed.
    #[test]
    fn spans_quoted_word_keeps_closing_quote() {
        let text = r#"[string match "/api/*" $uri]"#;
        let (cmd, args) = parse_command_substitution_with_spans_and_config(
            text,
            tcl_lexer::LexerConfig::default(),
        )
        .unwrap();
        assert_eq!(cmd, "string");
        assert_eq!(args[1].0, "\"/api/*\"");
        assert_eq!(
            &text[args[1].1.start() as usize..args[1].1.end() as usize],
            "\"/api/*\""
        );
    }

    /// Same recovery applies when the quoted word's last fragment follows
    /// an embedded command substitution rather than being the whole word.
    #[test]
    fn spans_quoted_word_with_nested_command_sub_keeps_closing_quote() {
        let text = r#"[string match "a[foo]b" $c]"#;
        let (cmd, args) = parse_command_substitution_with_spans_and_config(
            text,
            tcl_lexer::LexerConfig::default(),
        )
        .unwrap();
        assert_eq!(cmd, "string");
        assert_eq!(args[1].0, "\"a[foo]b\"");
        assert_eq!(
            &text[args[1].1.start() as usize..args[1].1.end() as usize],
            "\"a[foo]b\""
        );
    }

    /// Control: when the closing quote directly follows an embedded
    /// `$var` substitution, the lexer already emits it as its own
    /// adjacent token (no gap) — confirms the fix above doesn't
    /// double-append in that shape.
    #[test]
    fn spans_quoted_word_after_var_substitution_keeps_closing_quote() {
        let text = r#"[string match "a$b" $c]"#;
        let (cmd, args) = parse_command_substitution_with_spans_and_config(
            text,
            tcl_lexer::LexerConfig::default(),
        )
        .unwrap();
        assert_eq!(cmd, "string");
        assert_eq!(args[1].0, "\"a$b\"");
        assert_eq!(
            &text[args[1].1.start() as usize..args[1].1.end() as usize],
            "\"a$b\""
        );
    }

    /// A quoted word that opens *directly* on a command substitution must not
    /// come back with a doubled `[` and no closing `]` (`"[[lindex $x 0]"`).
    /// The raw token spans overlap in this shape — `parse_quoted`'s
    /// empty-content clamp extends the opening `Esc` one byte over the `[`
    /// that stopped its scan, and the `Cmd` token starts on that same `[` —
    /// so concatenating raw token text emits the shared byte twice.
    #[test]
    fn spans_quoted_word_opening_on_command_sub_is_not_doubled() {
        let text = r#"[list "[lindex $x 0]"]"#;
        let (cmd, args) = parse_command_substitution_with_spans_and_config(
            text,
            tcl_lexer::LexerConfig::default(),
        )
        .unwrap();
        assert_eq!(cmd, "list");
        assert_eq!(args.len(), 1);
        assert_eq!(args[0].0, "\"[lindex $x 0]\"");
        assert_eq!(
            &text[args[0].1.start() as usize..args[0].1.end() as usize],
            "\"[lindex $x 0]\""
        );
    }

    /// The same clamp extends the opening `Esc` over a `$` introducer, so a
    /// quoted word opening directly on a variable substitution is the other
    /// half of the overlap family.
    #[test]
    fn spans_quoted_word_opening_on_var_sub_is_not_doubled() {
        let text = r#"[list "$x"]"#;
        let (cmd, args) = parse_command_substitution_with_spans_and_config(
            text,
            tcl_lexer::LexerConfig::default(),
        )
        .unwrap();
        assert_eq!(cmd, "list");
        assert_eq!(args[0].0, "\"$x\"");
        assert_eq!(
            &text[args[0].1.start() as usize..args[0].1.end() as usize],
            "\"$x\""
        );
    }

    /// The overlap-family table: the shape that overlaps plus its three
    /// neighbours, each pinned on the invariant at stake — a returned
    /// argument's text is exactly the source its own span covers, so the two
    /// cannot disagree.
    #[test]
    fn spans_argument_text_always_matches_its_own_span() {
        for text in [
            r#"[list "[lindex $x 0]"]"#,
            r#"[list "a[foo]"]"#,
            r"[list a[lindex $x 0]b]",
            r"[list ${p}]",
            r#"[list "$x"]"#,
            r#"[list "a$b" {c d} [e]]"#,
        ] {
            let (_cmd, args) = parse_command_substitution_with_spans_and_config(
                text,
                tcl_lexer::LexerConfig::default(),
            )
            .unwrap_or_else(|| panic!("{text} should parse as one command substitution"));
            for (arg_text, span) in &args {
                assert_eq!(
                    arg_text.as_str(),
                    &text[span.start() as usize..span.end() as usize],
                    "argument text and span disagree for {text}"
                );
            }
        }
    }
}

#[cfg(test)]
mod original_value_word_purpose_tests {
    use super::*;

    #[test]
    fn original_authored_value_word_keeps_bom_and_parent_variable_grammar() {
        // naming.source.original-configured-script-completeness

        // Authored operand syntax, without a source entry or evaluated value receipt.
        let config = tcl_lexer::LexerConfig {
            leading_bom: tcl_lexer::LeadingBom::Skip,
            braced_var: tcl_lexer::BracedVarStyle::FirstClose,
            strict_quoting: true,
            ..tcl_lexer::LexerConfig::default()
        };
        let text = "\u{feff}VALUE";
        let word = value_word_with_config(text, config).unwrap();
        let crate::ir::WordExpr::Literal {
            text: value,
            source,
        } = word
        else {
            panic!("one genuine literal operand");
        };
        assert_eq!(value, text);
        assert_eq!(
            source.span,
            Span::new(0, u32::try_from(text.len()).unwrap())
        );
        let whole = crate::segmenter::segment_commands_with_offset_and_config(text, 0, config);
        assert_eq!(whole[0].name(), "VALUE");
        assert_eq!(whole[0].argv[0].span.start(), 3);
        let variable = value_word_with_config("${a{b}", config).unwrap();
        assert_eq!(variable.sole_variable_substitution().unwrap().0, "${a{b}");
        assert!(value_word_with_config("VALUE other", config).is_none());
        assert!(value_word_with_config("VALUE; other", config).is_none());
        let content = tcl_lexer::LexerConfig {
            leading_bom: tcl_lexer::LeadingBom::Content,
            ..config
        };
        assert_eq!(
            value_word_with_config(text, content).unwrap().legacy_text(),
            text
        );
    }
}
