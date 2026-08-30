// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Live variable- and command-substitution spans in a Tcl expression.
//!
//! The expression lexer intentionally makes a double-quoted operand one
//! opaque `String` token.  That is right for expression parsing, but a
//! consumer that needs to execute or inventory the embedded Tcl commands must
//! still see substitutions in that operand.  This module owns that bridge: it
//! keeps expression-level braces and comments inert, while delegating every
//! actual `[…]` closer to the lexer range owner.

use tcl_dialect::DialectProfile;
use tcl_lexer::{
    Lexer, LexerConfig, Span, TokenType, backslash_escape_end, command_substitution_end,
};

use super::{ExprNode, parse_expr};

/// The direct substitutions that a valid Tcl expression evaluates.
///
/// Both vectors contain byte-exact, half-open ranges relative to the supplied
/// expression text. Variables inside a returned command span are deliberately
/// absent from [`Self::variables`]: that command is a Tcl-script boundary and
/// its script consumer must recurse into it exactly once.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LiveExpressionSubstitutions {
    /// Variable references evaluated by the expression itself.
    pub variables: Vec<Span>,
    /// Complete outermost command substitutions evaluated by the expression.
    pub commands: Vec<Span>,
}

/// Return every variable and complete command substitution directly evaluated
/// by `source` as a Tcl expression.
///
/// This is the shared expression-to-script bridge. It parses through the
/// canonical profile-aware expression lexer/parser first, then gates the
/// result through the caller's resolved runtime surface. Therefore a malformed
/// expression, a Tcl 8.x expression comment, an inert quoted/braced operand,
/// or a runtime-rejected operator contributes no execution facts.
///
/// Array-index variable substitutions are returned in addition to their
/// enclosing array variable. The index is scanned through the shared Tcl lexer
/// in its substitution-only mode, so a nested `[…]` command remains opaque and
/// is not misreported as an outer expression variable.
#[must_use]
pub fn live_expression_substitutions(
    source: &str,
    profile: &'static DialectProfile,
    lexer_config: LexerConfig,
    surface_accepts: impl FnOnce(&ExprNode) -> bool,
) -> LiveExpressionSubstitutions {
    if !lexer_config_matches_profile(lexer_config, profile) {
        return LiveExpressionSubstitutions::default();
    }
    // `parse_expr` is the canonical expression lexer/parser seam. In
    // particular, it rejects a Tcl 8.x `#`, dangling operators, unbalanced
    // delimiters, and incomplete variable forms before we expose any
    // otherwise well-formed substitution range.
    let parsed = parse_expr(source, Some(profile.name));
    if matches!(parsed, ExprNode::Raw { .. }) || !surface_accepts(&parsed) {
        return LiveExpressionSubstitutions::default();
    }

    let mut variables: Vec<Span> = parsed
        .variable_spans()
        .into_iter()
        .filter_map(|(start, end)| expression_span(source, start, end))
        .collect();
    // Direct AST variables include the whole `$array(index)` form. The array
    // key evaluates as a Tcl substitution context of its own, so recover its
    // direct variables through the lexer owner. Commands inside the index stay
    // opaque here and are recovered from `commands` by the script recursion.
    let direct_variables = variables.clone();
    for variable in direct_variables {
        append_array_index_variable_spans(source, variable, lexer_config, &mut variables);
    }
    // The expression AST deliberately keeps string operands opaque. A
    // double-quoted operand nevertheless evaluates Tcl substitutions at expr
    // time, unlike a braced operand. Re-enter only the raw quoted interiors
    // through the shared lexer; scanning decoded string text would lose exact
    // escapes and can turn inert source into a false live variable.
    for (start, end) in parsed.quoted_string_spans() {
        append_quoted_string_variable_spans(source, start, end, lexer_config, &mut variables);
    }
    variables.sort_unstable_by_key(|span| (span.start(), span.end()));
    variables.dedup();

    let comments = profile.grammar.expr_comments.comments();
    let mut commands = Vec::new();
    scan_expression(source, 0, source.len(), comments, &mut commands);

    LiveExpressionSubstitutions {
        variables,
        commands,
    }
}

/// Return the complete, outermost command substitutions directly evaluated by
/// `source` as a Tcl expression.
///
/// Each returned [`Span`] is a byte-exact, half-open range including both
/// brackets.  An escaped bracket, a bracket inside a braced literal, and an
/// unterminated command substitution are absent.  Substitutions in a
/// double-quoted expression operand are included.  A substitution nested in a
/// returned command is intentionally not repeated: it belongs to that
/// command's Tcl-script body and its script consumer will recurse into it.
///
/// `profile` controls TIP 582 expression comments and every other expression
/// grammar axis; `lexer_config` must be derived from the same profile and
/// carries the script-side grammar used to consume returned spans.  The API
/// fails closed if those two resolved inputs disagree. `surface_accepts` is the
/// caller's resolved runtime expression surface: syntax alone cannot decide
/// whether a parsed operator is executable for that runtime without making
/// this foundational crate depend on the registry. A malformed expression, or
/// one the runtime surface rejects, returns no spans. This is deliberately
/// all-or-nothing: a syntactically invalid expression executes no embedded
/// commands.
///
/// The closer itself is owned by
/// [`tcl_lexer::command_substitution_end`], so quotes, braces, nested command
/// substitutions, escapes, and command-position comments *inside* a returned
/// command use Tcl script grammar rather than a duplicate expression scan.
#[must_use]
pub fn command_substitution_spans(
    source: &str,
    profile: &'static DialectProfile,
    lexer_config: LexerConfig,
    surface_accepts: impl FnOnce(&ExprNode) -> bool,
) -> Vec<Span> {
    live_expression_substitutions(source, profile, lexer_config, surface_accepts).commands
}

/// Convert an inclusive expression-lexer pair into the public half-open span
/// contract, only after proving it still slices the original UTF-8 source.
fn expression_span(source: &str, start: u32, end: u32) -> Option<Span> {
    let start = usize::try_from(start).ok()?;
    let end = usize::try_from(end).ok()?.checked_add(1)?;
    source.get(start..end)?;
    Some(Span::new(
        u32::try_from(start).ok()?,
        u32::try_from(end).ok()?,
    ))
}

/// Add direct variable substitutions from a `$name(index)` key. The enclosing
/// expression parser has already proved the full variable syntax valid; this
/// function only decomposes the index's Tcl substitution context.
fn append_array_index_variable_spans(
    source: &str,
    variable: Span,
    lexer_config: LexerConfig,
    out: &mut Vec<Span>,
) {
    let Some(raw) = source.get(variable.as_range()) else {
        return;
    };
    let Some(index_range) = array_index_content_range(raw) else {
        return;
    };
    let Some(index) = raw.get(index_range.clone()) else {
        return;
    };
    let index_start = variable.start() as usize + index_range.start;
    let Ok(tokens) = Lexer::with_config(index, lexer_config)
        .as_quoted_body()
        .tokenise_all()
    else {
        return;
    };
    for token in tokens
        .into_iter()
        .filter(|token| token.kind == TokenType::Var)
    {
        let start = index_start + token.span.start() as usize;
        let end = index_start + token.span.end() as usize;
        let (Ok(start), Ok(end)) = (u32::try_from(start), u32::try_from(end)) else {
            continue;
        };
        let nested = Span::new(start, end);
        out.push(nested);
        append_array_index_variable_spans(source, nested, lexer_config, out);
    }
}

/// Add direct variable substitutions from one quoted expression operand.
///
/// `start` and `end` use the expression lexer's inclusive convention and
/// include the quote delimiters. The lexer sees only the raw interior in
/// quoted-body mode, preserving Tcl escape, array-index, and command-boundary
/// semantics without decoding the expression string.
fn append_quoted_string_variable_spans(
    source: &str,
    start: u32,
    end: u32,
    lexer_config: LexerConfig,
    out: &mut Vec<Span>,
) {
    let Some(quoted) = expression_span(source, start, end) else {
        return;
    };
    let quoted_start = quoted.start() as usize;
    let quoted_end = quoted.end() as usize;
    let Some(interior_start) = quoted_start.checked_add(1) else {
        return;
    };
    let Some(interior_end) = quoted_end.checked_sub(1) else {
        return;
    };
    let Some(interior) = source.get(interior_start..interior_end) else {
        return;
    };
    let Ok(tokens) = Lexer::with_config(interior, lexer_config)
        .as_quoted_body()
        .tokenise_all()
    else {
        return;
    };
    for token in tokens
        .into_iter()
        .filter(|token| token.kind == TokenType::Var)
    {
        let start = interior_start + token.span.start() as usize;
        let end = interior_start + token.span.end() as usize;
        let (Ok(start), Ok(end)) = (u32::try_from(start), u32::try_from(end)) else {
            continue;
        };
        let variable = Span::new(start, end);
        out.push(variable);
        append_array_index_variable_spans(source, variable, lexer_config, out);
    }
}

/// The content range of a non-braced `$name(index)` variable spelling.
///
/// The byte scan is the same name grammar as the expression lexer: ASCII word
/// characters plus complete `::` namespace separators. It runs only on a
/// complete variable token accepted by that lexer; parsing the index itself is
/// delegated to [`Lexer::as_quoted_body`], never a consumer regex.
fn array_index_content_range(variable: &str) -> Option<std::ops::Range<usize>> {
    let bytes = variable.as_bytes();
    if bytes.first() != Some(&b'$') || bytes.get(1) == Some(&b'{') || bytes.last() != Some(&b')') {
        return None;
    }
    let mut pos = 1;
    while let Some(&byte) = bytes.get(pos) {
        if byte.is_ascii_alphanumeric() || byte == b'_' {
            pos += 1;
        } else if byte == b':' && bytes.get(pos + 1) == Some(&b':') {
            pos += 2;
        } else {
            break;
        }
    }
    if bytes.get(pos) != Some(&b'(') {
        return None;
    }
    let start = pos.checked_add(1)?;
    let end = bytes.len().checked_sub(1)?;
    (start <= end).then_some(start..end)
}

/// The caller may change offsets, strictness, or file-vs-nested BOM handling,
/// but not the dialect-derived script grammar while using this expression
/// profile.  `command_substitution_end` is release-blind because no supported
/// release's escape *width* changes a `[`/`]` boundary; checking the resolved
/// config here still prevents a caller from mixing Tcl 8/iRules word grammar
/// with a Tcl 9 expression parse.
fn lexer_config_matches_profile(config: LexerConfig, profile: &DialectProfile) -> bool {
    let expected = LexerConfig::from_grammar(profile.grammar);
    config.expand_syntax == expected.expand_syntax
        && config.irules_brace_separator == expected.irules_brace_separator
        && config.brace_line_continuation == expected.brace_line_continuation
        && config.braced_var == expected.braced_var
        && config.escapes == expected.escapes
}

fn scan_expression(
    source: &str,
    mut pos: usize,
    end: usize,
    comments: bool,
    spans: &mut Vec<Span>,
) {
    let bytes = source.as_bytes();
    while pos < end {
        match bytes[pos] {
            b'\\' => pos = backslash_escape_end(source, pos).min(end),
            b'#' if comments => {
                pos = source[pos..end]
                    .find('\n')
                    .map_or(end, |relative| pos + relative + 1);
            }
            b'{' => match braced_literal_end(source, pos, end) {
                Some(next) => pos = next,
                // A malformed braced operand gives us no sound outer grammar
                // in which to prove a later `[` live.  Fail closed.
                None => return,
            },
            b'"' => {
                pos = scan_quoted_operand(source, pos, end, spans).unwrap_or(end);
            }
            b'[' => match command_substitution_end(source, pos).filter(|&next| next <= end) {
                Some(next) => {
                    spans.push(span(pos, next));
                    pos = next;
                }
                // Do not manufacture a span for an editor-buffer fragment
                // whose `]` has not yet been written.
                None => return,
            },
            _ => pos += 1,
        }
    }
}

/// Skip a braced expression operand.  Braces are literal syntax: neither
/// substitutions nor comments run in them.  This mirrors the brace balance
/// rule used by the expression lexer.
fn braced_literal_end(source: &str, open: usize, end: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut pos = open + 1;
    let mut depth = 1_u32;
    while pos < end {
        match bytes[pos] {
            b'\\' => pos = backslash_escape_end(source, pos).min(end),
            b'{' => {
                depth += 1;
                pos += 1;
            }
            b'}' => {
                depth -= 1;
                pos += 1;
                if depth == 0 {
                    return Some(pos);
                }
            }
            _ => pos += 1,
        }
    }
    None
}

/// Scan a quoted expression operand for substitutions; braces and `#` are
/// ordinary characters here.  Spans are committed only after the closing
/// quote, so an incomplete quoted operand does not claim that its inner
/// command can execute.
fn scan_quoted_operand(
    source: &str,
    open: usize,
    end: usize,
    spans: &mut Vec<Span>,
) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut pos = open + 1;
    let mut quoted_spans = Vec::new();
    while pos < end {
        match bytes[pos] {
            b'\\' => pos = backslash_escape_end(source, pos).min(end),
            b'"' => {
                spans.extend(quoted_spans);
                return Some(pos + 1);
            }
            b'[' => {
                let next = command_substitution_end(source, pos).filter(|&next| next <= end)?;
                quoted_spans.push(span(pos, next));
                pos = next;
            }
            _ => pos += 1,
        }
    }
    None
}

fn span(start: usize, end: usize) -> Span {
    Span::new(
        u32::try_from(start).expect("expression offset fits u32"),
        u32::try_from(end).expect("expression offset fits u32"),
    )
}

#[cfg(test)]
mod tests {
    use super::{command_substitution_spans, live_expression_substitutions};
    use tcl_dialect::DialectProfile;
    use tcl_lexer::LexerConfig;

    fn texts(source: &str, dialect: Option<&str>) -> Vec<String> {
        let profile = dialect
            .and_then(DialectProfile::find)
            .unwrap_or_else(DialectProfile::plain_tcl);
        command_substitution_spans(
            source,
            profile,
            LexerConfig::from_grammar(profile.grammar),
            |_| true,
        )
        .into_iter()
        .map(|span| source[span.as_range()].to_owned())
        .collect()
    }

    fn substitutions(source: &str, dialect: Option<&str>) -> super::LiveExpressionSubstitutions {
        let profile = dialect
            .and_then(DialectProfile::find)
            .unwrap_or_else(DialectProfile::plain_tcl);
        live_expression_substitutions(
            source,
            profile,
            LexerConfig::from_grammar(profile.grammar),
            |_| true,
        )
    }

    fn variable_texts(source: &str, dialect: Option<&str>) -> Vec<String> {
        substitutions(source, dialect)
            .variables
            .into_iter()
            .map(|span| source[span.as_range()].to_owned())
            .collect()
    }

    #[test]
    fn reports_direct_live_substitutions_with_exact_unicode_offsets() {
        let source = "\"☃[HTTP::host]\" eq \"host\" || [HTTP::uri] eq \"/\"";
        let profile = DialectProfile::irules();
        let spans = command_substitution_spans(
            source,
            profile,
            LexerConfig::from_grammar(profile.grammar),
            |_| true,
        );
        assert_eq!(
            texts(source, Some("f5-irules")),
            ["[HTTP::host]", "[HTTP::uri]"]
        );
        assert_eq!(
            spans[0].start() as usize,
            source.find("[HTTP::host]").unwrap()
        );
        assert_eq!(
            spans[0].end() as usize,
            source.find("[HTTP::host]").unwrap() + 12
        );
    }

    #[test]
    fn reports_valid_expression_variables_with_exact_nested_ranges() {
        let source = concat!(
            "\"\u{2603}\" ne \"\" && $static::maintenance && ",
            "$static::table($static::slot) && ${static::braced} && [HTTP::host] ne \"\"",
        );
        let facts = substitutions(source, Some("f5-irules"));
        let variables: Vec<_> = facts
            .variables
            .iter()
            .map(|span| source[span.as_range()].to_owned())
            .collect();
        assert_eq!(
            variables,
            [
                "$static::maintenance",
                "$static::table($static::slot)",
                "$static::slot",
                "${static::braced}",
            ],
        );
        assert_eq!(
            facts
                .commands
                .iter()
                .map(|span| source[span.as_range()].to_owned())
                .collect::<Vec<_>>(),
            ["[HTTP::host]"],
        );
        let outer = facts.variables[1];
        assert_eq!(
            outer.start() as usize,
            source
                .find("$static::table")
                .expect("outer variable position"),
            "the public span stays absolute-byte compatible after Unicode"
        );
    }

    #[test]
    fn quoted_operands_keep_live_substitutions_while_braced_operands_stay_inert() {
        assert_eq!(
            variable_texts(
                "\"$static::quoted\" eq {${static::braced}} || $static::live",
                Some("f5-irules"),
            ),
            ["$static::quoted", "$static::live"],
            "only the quoted operand evaluates its Tcl variable substitution"
        );

        let source = "$static::array([set static::inside]) ne \"\"";
        let facts = substitutions(source, Some("f5-irules"));
        assert_eq!(
            facts
                .variables
                .iter()
                .map(|span| source[span.as_range()].to_owned())
                .collect::<Vec<_>>(),
            ["$static::array([set static::inside])"],
            "the command body owns its own variable facts"
        );
        assert_eq!(
            facts
                .commands
                .iter()
                .map(|span| source[span.as_range()].to_owned())
                .collect::<Vec<_>>(),
            ["[set static::inside]"],
        );
    }

    #[test]
    fn quoted_operands_use_tcl_substitution_grammar_in_tcl86_and_tcl90() {
        let source = concat!(
            "\"$static::quoted($static::slot) [set static::inside 1] \\",
            "$static::escaped\" eq {${static::braced_data}}",
        );
        for dialect in ["tcl8.6", "tcl9.0"] {
            let facts = substitutions(source, Some(dialect));
            assert_eq!(
                facts
                    .variables
                    .iter()
                    .map(|span| source[span.as_range()].to_owned())
                    .collect::<Vec<_>>(),
                ["$static::quoted($static::slot)", "$static::slot"],
                "{dialect} keeps quoted array substitutions but no escaped/braced text"
            );
            assert_eq!(
                facts
                    .commands
                    .iter()
                    .map(|span| source[span.as_range()].to_owned())
                    .collect::<Vec<_>>(),
                ["[set static::inside 1]"],
                "{dialect} keeps a quoted command substitution at its raw span"
            );
        }
    }

    #[test]
    fn malformed_tcl84_expression_exposes_neither_variables_nor_commands() {
        for source in [
            "$static::maintenance +",
            "$static::array($static::slot",
            "${static::maintenance",
            "$static::maintenance # $static::comment",
        ] {
            let facts = substitutions(source, Some("f5-irules"));
            assert!(
                facts.variables.is_empty() && facts.commands.is_empty(),
                "malformed Tcl 8.4 iRules expression leaked facts: {source:?} -> {facts:?}"
            );
        }
    }

    #[test]
    fn excludes_escaped_braced_and_unterminated_brackets() {
        assert_eq!(
            texts(r#""\[escaped]" eq "[live]""#, Some("f5-irules")),
            ["[live]"]
        );
        assert_eq!(texts("{[braced]} eq [live]", Some("f5-irules")), ["[live]"]);
        assert!(texts("[unterminated", Some("f5-irules")).is_empty());
        assert!(texts("\"[complete_but_unquoted]", Some("f5-irules")).is_empty());
    }

    #[test]
    fn respects_expression_and_nested_script_comments() {
        let source = "# [inert]\n[set x 1; # ] in command\nHTTP::host]";
        assert_eq!(
            texts(source, Some("tcl9.0")),
            ["[set x 1; # ] in command\nHTTP::host]"]
        );
        assert_eq!(
            texts(source, Some("tcl8.6")),
            Vec::<String>::new(),
            "Tcl 8 treats the expression-level comment marker as invalid"
        );
    }

    #[test]
    fn nested_command_is_owned_by_its_outer_script() {
        assert_eq!(
            texts("[expr {[HTTP::host] ne \"\"}]", Some("f5-irules")),
            ["[expr {[HTTP::host] ne \"\"}]"]
        );
    }

    #[test]
    fn fails_closed_when_the_complete_expression_is_invalid() {
        for source in [
            "[live] +",
            "([live]",
            "[live] # Tcl 8 treats this as an invalid character",
        ] {
            assert!(
                texts(source, Some("tcl8.6")).is_empty(),
                "invalid Tcl 8 expression leaked a command span: {source:?}"
            );
        }

        assert_eq!(
            texts("[live] # Tcl 9 comment", Some("tcl9.0")),
            ["[live]"],
            "TIP 582 comments stay valid in Tcl 9"
        );
    }

    #[test]
    fn fails_closed_when_the_runtime_surface_rejects_the_expression() {
        let profile = DialectProfile::find("tcl9.0").expect("catalogue profile");
        assert!(
            command_substitution_spans(
                "[live]",
                profile,
                LexerConfig::from_grammar(profile.grammar),
                |_| false,
            )
            .is_empty(),
            "the caller's runtime expression surface gates execution"
        );
    }

    #[test]
    fn fails_closed_when_the_script_config_is_for_another_profile() {
        let profile = DialectProfile::find("tcl9.0").expect("catalogue profile");
        assert!(
            command_substitution_spans(
                "[live]",
                profile,
                LexerConfig::from_grammar(
                    DialectProfile::find("tcl8.6")
                        .expect("catalogue profile")
                        .grammar
                ),
                |_| true,
            )
            .is_empty(),
            "mixed profile/config inputs must not execute a span"
        );
    }
}
