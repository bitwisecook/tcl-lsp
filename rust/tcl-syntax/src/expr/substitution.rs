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

use super::ExprNode;
use super::parser::{CheckedExprParse, ExprParseContext, parse_expr_checked_with_context};

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

/// Retain compiler-visible command substitutions in a checked expression.
///
/// These are lexical compiler visits, including both lazy branches. They do
/// not prove runtime execution. Rejected or unsupported syntax returns `None`;
/// every range includes the brackets and is half-open in the original source.
#[must_use]
pub fn command_substitutions_in_checked_expression(
    source: &str,
    context: &ExprParseContext,
) -> Option<Vec<Span>> {
    checked_expression_substitutions(source, context).map(|substitutions| substitutions.commands)
}

/// Original lexical substitution visits under the complete retained context.
///
/// Lazy branches are both visited. These ranges provide source topology,
/// never evidence that a variable read, command or conversion completes or
/// runs. The caller retains runtime/compilation purpose independently.
/// Rejected or unsupported syntax returns `None` instead of recovery facts.
#[must_use]
pub fn checked_expression_substitutions(
    source: &str,
    context: &ExprParseContext,
) -> Option<LiveExpressionSubstitutions> {
    let CheckedExprParse::Parsed(parsed) = parse_expr_checked_with_context(source, context) else {
        return None;
    };
    substitutions_from_parsed(source, &parsed, context)
}

/// Complete checked source expression, without a selected Native parser or evaluator.
/// The source bytes, full grammar and immutable tree remain one lexical receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceExpression {
    source: String,
    context: ExprParseContext,
    tree: ExprNode,
    config: LexerConfig,
}
impl OriginalSourceExpression {
    /// Parse complete original bytes with the existing source-only grammar owner.
    /// Recovery trees and incomplete or unavailable grammar return no receipt.
    #[must_use]
    pub fn capture(source: &str, context: &ExprParseContext, config: LexerConfig) -> Option<Self> {
        let tree =
            super::parser::parse_source_expression_checked_with_config(source, context, config)?;
        Some(Self {
            source: source.to_owned(),
            context: *context,
            tree,
            config,
        })
    }
    /// Compare a retained lowered tree with this complete checked source tree.
    /// Equality preserves source offsets and topology; it grants no evaluation,
    /// operand value, successful completion or rewrite equivalence.
    #[must_use]
    pub fn matches_tree(&self, tree: &ExprNode) -> bool {
        &self.tree == tree
    }
    /// Complete original expression bytes, without cooking or trimming.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }
    /// All retained parser axes; source recognition grants no Native policy.
    #[must_use]
    pub const fn context(&self) -> &ExprParseContext {
        &self.context
    }
    /// Checked function identifiers in immutable tree order, including nested calls.
    #[must_use]
    pub fn function_calls(&self) -> Vec<(&str, super::ExprOffset, usize)> {
        self.tree.function_calls()
    }
    /// Complete original script/word configuration, including caller knobs.
    #[must_use]
    pub const fn lexer_config(&self) -> LexerConfig {
        self.config
    }
    /// Conservative opaque/string or substitution operands in the original tree.
    /// A string may contain inert bytes; this supplies no effect closure or read.
    #[must_use]
    pub fn has_opaque_or_substitution_operands(&self) -> bool {
        self.tree.evaluation_nodes().into_iter().any(|node| {
            matches!(
                node,
                ExprNode::Var { .. }
                    | ExprNode::Command { .. }
                    | ExprNode::String { .. }
                    | ExprNode::CompiledWord { .. }
                    | ExprNode::Raw { .. }
            )
        })
    }
}

/// Complete original source substitution topology under the retained grammar.
///
/// The caller must separately retain its source input, configured original
/// word/image/extent and selected Expr role. This accepts no recovery tree and
/// grants no Native syntax error, preparation, result representation, folding
/// or execution proof. Unknown Native syntax stays unknown; lexical failures,
/// unavailable operators and incomplete consumption still decline.
#[must_use]
pub fn checked_source_expression_substitutions(
    source: &str,
    context: &ExprParseContext,
) -> Option<LiveExpressionSubstitutions> {
    let parsed = super::parser::parse_source_expression_checked(source, context)?;
    substitutions_from_parsed(source, &parsed, context)
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
    let context = ExprParseContext::for_profile(profile);
    let CheckedExprParse::Parsed(parsed) = parse_expr_checked_with_context(source, &context) else {
        return LiveExpressionSubstitutions::default();
    };
    if !surface_accepts(&parsed) {
        return LiveExpressionSubstitutions::default();
    }
    substitutions_from_parsed(source, &parsed, &context).unwrap_or_default()
}

fn substitutions_from_parsed(
    source: &str,
    parsed: &ExprNode,
    context: &ExprParseContext,
) -> Option<LiveExpressionSubstitutions> {
    // The checked expression owns the complete quote delimiters. Its extracted
    // interiors deliberately have no closing quote, so quoted-body EOF is not
    // a new syntax failure. Nested syntax was checked before this projection.
    let lexer_config = LexerConfig::from_grammar(context.lexer_grammar);

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
        append_quoted_string_variable_spans(source, start, end, lexer_config, &mut variables)?;
    }
    variables.sort_unstable_by_key(|span| (span.start(), span.end()));
    variables.dedup();

    let comments = context.lexer_grammar.expr_comments.comments();
    let mut commands = Vec::new();
    scan_expression(source, 0, source.len(), comments, &mut commands);

    Some(LiveExpressionSubstitutions {
        variables,
        commands,
    })
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

/// Convert an inclusive expression-lexer pair into a checked half-open span.
///
/// The complete extent must remain within `source` and on UTF-8 boundaries.
/// This is source geometry only: a span supplies no original operand, source
/// ancestry, grammar, selected command or execution receipt.
#[must_use]
pub fn expression_span(source: &str, start: u32, end: u32) -> Option<Span> {
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
    let Some(index_range) = array_index_content_range(raw, lexer_config) else {
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
) -> Option<()> {
    let quoted = expression_span(source, start, end)?;
    let quoted_start = quoted.start() as usize;
    let quoted_end = quoted.end() as usize;
    let interior_start = quoted_start.checked_add(1)?;
    let interior_end = quoted_end.checked_sub(1)?;
    let interior = source.get(interior_start..interior_end)?;
    // The expr parser owns the quotes but treats their interiors as opaque.
    // Validate nested substitution delimiters through the same script scanner;
    // quoted-body EOF alone cannot distinguish a missing array closer.
    let extent = Span::new(0, u32::try_from(interior.len()).ok()?);
    let arena = tcl_lexer::ExecutablePartArena::decompose(
        tcl_lexer::SourceImage::document(interior),
        extent,
        tcl_lexer::word_parts::SubstFlags::default(),
        lexer_config,
    )
    .ok()?;
    if arena
        .all_parts()
        .any(|part| matches!(part.part, tcl_lexer::ExecutablePart::ParseError(_)))
    {
        return None;
    }
    let Ok(tokens) = Lexer::with_config(interior, lexer_config)
        .as_quoted_body()
        .tokenise_all()
    else {
        return None;
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
    Some(())
}

/// The content range of a non-braced `$name(index)` variable spelling.
///
/// The native variable scanner owns both the complete name grammar and index
/// delimiters. Borrowed index geometry preserves Jim high-byte names and empty
/// indices without a second name scan or closer calculation.
fn array_index_content_range(
    variable: &str,
    config: LexerConfig,
) -> Option<std::ops::Range<usize>> {
    let bytes = variable.as_bytes();
    if bytes.first() != Some(&b'$') {
        return None;
    }
    let reference = tcl_lexer::word_parts::scan_var_ref(bytes, 0, config).ok()??;
    (reference.next == bytes.len()).then_some(())?;
    reference.index_range_in(bytes)
}

/// The caller may change offsets, strictness, or file-vs-nested BOM handling,
/// but not the dialect-derived script grammar while using this expression
/// profile.  `command_substitution_end` is release-blind because no supported
/// release's escape *width* changes a `[`/`]` boundary; checking the resolved
/// config here still prevents a caller from mixing Tcl 8/iRules word grammar
/// with a Tcl 9 expression parse.
fn lexer_config_matches_profile(config: LexerConfig, profile: &DialectProfile) -> bool {
    config.grammar_over(profile.grammar) == profile.grammar
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
    use super::{
        checked_expression_substitutions, command_substitution_spans, live_expression_substitutions,
    };
    use crate::expr::parser::ExprParseContext;
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
        assert_eq!(texts("[unterminated", Some("f5-irules")), [] as [String; 0]);
        assert_eq!(
            texts("\"[complete_but_unquoted]", Some("f5-irules")),
            [] as [String; 0]
        );
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

    #[test]
    fn checked_substitutions_retain_overridden_variable_and_comment_grammar() {
        // Implementation contract: naming.grammar.checked-expression-substitution-context
        // docs/design/analysis/name-resolution-proofs/checked-expression-substitution-context.md

        let profile = DialectProfile::find("tcl8.6").unwrap();
        let mut context = ExprParseContext::for_profile(profile);
        context.lexer_grammar.var_syntax = tcl_dialect::VarSyntax::Jim;
        context.lexer_grammar.expr_comments = DialectProfile::find("tcl9.0")
            .unwrap()
            .grammar
            .expr_comments;
        let source = "\"☃$café($clé)\" ne \"\" # [inert]\n || [left] || [right]";
        let substitutions = checked_expression_substitutions(source, &context).unwrap();
        assert_eq!(
            substitutions
                .variables
                .iter()
                .map(|span| &source[span.as_range()])
                .collect::<Vec<_>>(),
            ["$café($clé)", "$clé"],
        );
        assert_eq!(
            substitutions
                .commands
                .iter()
                .map(|span| &source[span.as_range()])
                .collect::<Vec<_>>(),
            ["[left]", "[right]"],
            "both lazy branches are lexical visits, not runtime completion proofs",
        );
        assert!(
            checked_expression_substitutions(source, &ExprParseContext::for_profile(profile))
                .is_none()
        );
        assert!(checked_expression_substitutions("$a +", &context).is_none());
        assert!(checked_expression_substitutions("\"$a(\"", &context).is_none());
    }

    #[test]
    fn checked_substitutions_keep_command_script_ownership_separate() {
        // Implementation contract: naming.grammar.checked-expression-substitution-context
        // docs/design/analysis/name-resolution-proofs/checked-expression-substitution-context.md

        let context = ExprParseContext::for_profile(DialectProfile::find("tcl8.6").unwrap());
        let source = "$a($index[set inside $other]) + [set result $value]";
        let substitutions = checked_expression_substitutions(source, &context).unwrap();
        assert_eq!(
            substitutions
                .variables
                .iter()
                .map(|span| &source[span.as_range()])
                .collect::<Vec<_>>(),
            ["$a($index[set inside $other])", "$index"],
        );
        assert_eq!(
            substitutions
                .commands
                .iter()
                .map(|span| &source[span.as_range()])
                .collect::<Vec<_>>(),
            ["[set inside $other]", "[set result $value]"],
        );
    }
}

#[cfg(test)]
mod original_expression_span_tests {
    use super::expression_span;
    use tcl_lexer::Span;

    #[test]
    fn inclusive_expression_extent_keeps_complete_multibyte_source_and_checked_boundaries() {
        // naming.grammar.checked-expression-substitution-context
        // docs/design/analysis/name-resolution-proofs/checked-expression-substitution-context.md
        // Geometry only; no original operand or command selection is issued.
        let command = "[regexp λ λ out]";
        let end = u32::try_from(command.len())
            .unwrap()
            .checked_sub(1)
            .unwrap();
        let extent = expression_span(command, 0, end).unwrap();
        assert_eq!(extent, Span::new(0, end + 1));
        assert_eq!(command.get(extent.as_range()), Some(command));

        let source = format!("λ prefix {command} suffix");
        let base = u32::try_from(source.find(command).unwrap()).unwrap();
        let extent = expression_span(&source, base, base.checked_add(end).unwrap()).unwrap();
        assert_eq!(extent.start(), base);
        assert_eq!(source.get(extent.as_range()), Some(command));
        let lambda = u32::try_from(command.find('λ').unwrap()).unwrap();
        assert!(expression_span(command, lambda + 1, end).is_none());
        assert!(expression_span(command, 0, lambda).is_none());
        assert!(expression_span(command, 0, u32::MAX).is_none());
        assert!(expression_span(command, end + 2, end).is_none());
        assert!(expression_span(command, 0, end + 1).is_none());
    }
}

#[cfg(test)]
mod original_source_topology_tests {
    use super::super::parser::{CheckedExprParse, ExprParseUnsupported, NativeExprSyntax};
    use super::*;

    #[test]
    fn original_source_topology_keeps_complete_unicode_bom_ranges_without_native_syntax() {
        // naming.grammar.checked-expression-substitution-context
        // docs/design/analysis/name-resolution-proofs/checked-expression-substitution-context.md
        // Complete source topology only; unknown Native syntax remains unavailable.
        let context = ExprParseContext::for_profile(DialectProfile::plain_tcl());
        assert_eq!(context.native_syntax, NativeExprSyntax::Unknown);
        let source = "![\u{feff}match λ λ out] || [right]";
        let substitutions = checked_source_expression_substitutions(source, &context).unwrap();
        assert!(substitutions.variables.is_empty());
        assert_eq!(
            substitutions
                .commands
                .iter()
                .map(|span| &source[span.as_range()])
                .collect::<Vec<_>>(),
            ["[\u{feff}match λ λ out]", "[right]"]
        );
        assert_eq!(substitutions.commands[0].start(), 1);
        assert_eq!(
            substitutions.commands.last().unwrap().end(),
            u32::try_from(source.len()).unwrap()
        );
        assert!(checked_expression_substitutions(source, &context).is_none());
        assert_eq!(
            parse_expr_checked_with_context(source, &context),
            CheckedExprParse::Unsupported(ExprParseUnsupported::UnknownNativeSyntax)
        );
        assert_eq!(context.native_syntax, NativeExprSyntax::Unknown);
    }

    #[test]
    fn original_source_topology_refuses_incomplete_lexing_unsupported_operators_and_unconsumed_suffix()
     {
        // naming.grammar.checked-expression-substitution-context
        // docs/design/analysis/name-resolution-proofs/checked-expression-substitution-context.md
        let mut context = ExprParseContext::for_profile(DialectProfile::plain_tcl());
        for source in [
            "[complete] +",
            "([complete]",
            "[unterminated",
            "\"$array(\"",
            "$array($key",
            "[complete] @",
            "[complete] [later]",
        ] {
            assert!(
                checked_source_expression_substitutions(source, &context).is_none(),
                "{source}"
            );
        }
        context.expr_grammar_base = Some(tcl_dialect::TclVersion::V8_4);
        assert!(checked_source_expression_substitutions("[complete] ** 2", &context).is_none());
        assert!(checked_source_expression_substitutions("[complete] + 2", &context).is_some());
        context.lexer_grammar.expr_comments = DialectProfile::find("tcl8.6")
            .unwrap()
            .grammar
            .expr_comments;
        assert!(
            checked_source_expression_substitutions("[complete] # [inert]", &context).is_none()
        );
        context.lexer_grammar.expr_comments = DialectProfile::find("tcl9.0")
            .unwrap()
            .grammar
            .expr_comments;
        assert_eq!(
            checked_source_expression_substitutions("[complete] # [inert]", &context)
                .unwrap()
                .commands
                .len(),
            1
        );
        context.native_syntax = NativeExprSyntax::Tcl(tcl_dialect::TclVersion::V8_4);
        assert!(checked_expression_substitutions("[complete] ** 2", &context).is_none());
        assert!(checked_source_expression_substitutions("[complete] ** 2", &context).is_none());
    }
}

#[cfg(test)]
mod original_source_expression_tests {
    use super::*;

    #[test]
    fn original_source_expression_retains_complete_configuration_and_untrimmed_offsets() {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let context = ExprParseContext::for_profile(profile);
        let mut config = LexerConfig::from_grammar(context.lexer_grammar);
        config.strict_quoting = !config.strict_quoting;
        let expression =
            OriginalSourceExpression::capture("  max(abs($x), 2)  ", &context, config).unwrap();
        assert_eq!(expression.source(), "  max(abs($x), 2)  ");
        assert_eq!(expression.lexer_config(), config);
        assert_eq!(expression.function_calls(), [("max", 2, 2), ("abs", 6, 1)]);
        assert!(expression.has_opaque_or_substitution_operands());
        let mut foreign = config;
        foreign.expand_syntax = !foreign.expand_syntax;
        assert!(OriginalSourceExpression::capture("max(1,2)", &context, foreign).is_none());
        assert!(OriginalSourceExpression::capture("max(1,2) tail", &context, config).is_none());
    }
}

impl OriginalSourceExpression {
    /// Correspondence for replacing one actual scalar source substitution by
    /// a numeric literal. Complete parsing and original token topology prevent
    /// adjacent numeral/operator text from silently changing the container.
    /// Quoted expression operands retain their existing substitution grammar.
    /// No evaluation, binding or transformation permission follows.
    #[must_use]
    pub fn scalar_numeric_literal_substitution(
        &self,
        reference: Span,
        value: &str,
    ) -> Option<String> {
        use tcl_lexer::ExprTokenType;
        let admits = || -> Option<String> {
            if !crate::number::is_whole_number(value, self.context.lexer_grammar.numbers) {
                return None;
            }
            let tokens = |text: &str| -> Option<Vec<tcl_lexer::ExprToken<Vec<u8>>>> {
                let (tokens, failures) = tcl_lexer::tokenise_expr_bytes_with_config_and_failures(
                    text.as_bytes(),
                    &self.context.lexer_grammar,
                    self.context.expr_grammar_base,
                    self.context.f5_word_grammar,
                    self.config,
                );
                failures.is_empty().then_some(tokens)
            };
            let original = tokens(&self.source)?;
            let mut expected = Vec::new();
            let mut replaced = false;
            let mut fragment = value.to_owned();
            for token in original {
                let end = token.end.checked_add(1)?;
                if token.start <= reference.start() && reference.end() <= end {
                    if replaced {
                        return None;
                    }
                    let token_source = std::str::from_utf8(&token.text).ok()?;
                    let extent = Span::new(0, u32::try_from(token_source.len()).ok()?);
                    let plan = tcl_lexer::native_script_words_in_with_purpose(
                        tcl_lexer::SourceImage::document(token_source),
                        extent,
                        self.config,
                        tcl_lexer::SourceSegmentationPurpose::NestedScript,
                    )
                    .ok()?;
                    let [command] = plan.commands.as_slice() else {
                        return None;
                    };
                    let [word] = command.words.as_slice() else {
                        return None;
                    };
                    let local = Span::new(
                        reference.start().checked_sub(token.start)?,
                        reference.end().checked_sub(token.start)?,
                    );
                    if plan.fatal_tail.is_some()
                        || word.span() != extent
                        || !crate::word_rules::lexical_source_scalar_literal_substitution(
                            word, local, value,
                        )
                    {
                        return None;
                    }
                    if token.kind == ExprTokenType::Variable {
                        let literal = tokens(value)?
                            .into_iter()
                            .filter(|token| token.kind != ExprTokenType::Eof)
                            .collect::<Vec<_>>();
                        if !matches!(literal.as_slice(), [token] if token.kind==ExprTokenType::Number)
                        {
                            // A variable is an atom. Preserve that topology for
                            // signed numbers instead of donating unary precedence.
                            fragment = format!("({value})");
                        }
                    }
                    match token.kind {
                        ExprTokenType::Variable
                            if token.start == reference.start() && end == reference.end() =>
                        {
                            for literal in tokens(&fragment)? {
                                if literal.kind == ExprTokenType::Eof {
                                    continue;
                                }
                                if !matches!(
                                    literal.kind,
                                    ExprTokenType::Number
                                        | ExprTokenType::Operator
                                        | ExprTokenType::ParenOpen
                                        | ExprTokenType::ParenClose
                                ) {
                                    return None;
                                }
                                if literal.kind == ExprTokenType::Operator
                                    && literal.text != b"+"
                                    && literal.text != b"-"
                                {
                                    return None;
                                }
                                expected.push((literal.kind, literal.text));
                            }
                        }
                        ExprTokenType::String if token.text.first() == Some(&b'"') => {
                            let from = usize::try_from(reference.start().checked_sub(token.start)?)
                                .ok()?;
                            let to =
                                usize::try_from(reference.end().checked_sub(token.start)?).ok()?;
                            let mut text = token.text;
                            text.get(from..to)?;
                            drop(text.splice(from..to, value.bytes()));
                            expected.push((token.kind, text));
                        }
                        _ => return None,
                    }
                    replaced = true;
                } else {
                    expected.push((token.kind, token.text));
                }
            }
            if !replaced {
                return None;
            }
            let mut proposal = self.source.clone();
            proposal.get(reference.as_range())?;
            proposal.replace_range(reference.as_range(), &fragment);
            Self::capture(&proposal, &self.context, self.config)?;
            let actual = tokens(&proposal)?
                .into_iter()
                .map(|token| (token.kind, token.text))
                .collect::<Vec<_>>();
            (expected == actual).then_some(fragment)
        };
        admits()
    }
}

#[cfg(test)]
mod numeric_source_substitution_tests {
    use super::*;
    fn expression(source: &str) -> Option<OriginalSourceExpression> {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        OriginalSourceExpression::capture(
            source,
            &ExprParseContext::for_profile(profile),
            LexerConfig::for_profile(Some(profile)),
        )
    }
    #[test]
    fn numeric_atom_and_quoted_scalar_keep_the_original_expression_language() {
        let original = expression("$x + 1").unwrap();
        assert!(
            original
                .scalar_numeric_literal_substitution(Span::new(0, 2), "7")
                .is_some()
        );
        assert_eq!(
            original
                .scalar_numeric_literal_substitution(Span::new(0, 2), "-1")
                .as_deref(),
            Some("(-1)")
        );
        assert!(
            original
                .scalar_numeric_literal_substitution(Span::new(0, 2), "abc")
                .is_none()
        );
        assert!(
            original
                .scalar_numeric_literal_substitution(Span::new(0, 2), "0xZZZ")
                .is_none()
        );
        let original = expression("{${x}} eq \"$x\"").unwrap();
        assert!(
            original
                .scalar_numeric_literal_substitution(Span::new(1, 5), "7")
                .is_none()
        );
        assert!(
            original
                .scalar_numeric_literal_substitution(Span::new(11, 13), "7")
                .is_some()
        );
        assert!(expression("${x}2 + 1").is_none());
        let original = expression("\"$arr($x)\" eq 0").unwrap();
        assert!(
            original
                .scalar_numeric_literal_substitution(Span::new(6, 8), "7")
                .is_none()
        );
    }
}

/// Complete original expression command substitution and its script interior.
/// Coordinates are relative to the retained expression, not a source identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OriginalExpressionCommandRegion {
    /// Complete source extent, including its original brackets.
    pub substitution: Span,
    /// Actual script interior selected by the shared command scanner.
    pub body: Span,
}

impl OriginalSourceExpression {
    /// Direct source command regions under every retained parser/configuration
    /// axis. Quoted operands and variable indices use the same executable-part
    /// owner; inert braced strings and commands inside returned scripts are not
    /// reclassified. Missing geometry or lexical failures withdraw the result.
    /// The caller separately retains its original word/channel/selected role.
    #[must_use]
    pub fn command_regions(&self) -> Option<Vec<OriginalExpressionCommandRegion>> {
        let source = self.source.as_bytes();
        let (tokens, failures) = tcl_lexer::tokenise_expr_bytes_with_config_and_failures(
            source,
            &self.context.lexer_grammar,
            self.context.expr_grammar_base,
            self.context.f5_word_grammar,
            self.config,
        );
        failures.is_empty().then_some(())?;
        let terms = tcl_lexer::expression_terms(source, &tokens, self.config)?;
        let image = tcl_lexer::SourceImage::document(&self.source);
        let mut regions = Vec::new();
        for term in terms {
            let whole = Span::new(term.source.start, term.source.end);
            let content = Span::new(term.value.start, term.value.end);
            let parts = match term.kind {
                tcl_lexer::ExprTermKind::Command => {
                    regions.push(OriginalExpressionCommandRegion {
                        substitution: whole,
                        body: content,
                    });
                    continue;
                }
                tcl_lexer::ExprTermKind::EscapedString => content,
                tcl_lexer::ExprTermKind::Variable | tcl_lexer::ExprTermKind::IndexedVariable => {
                    whole
                }
                _ => continue,
            };
            let arena = tcl_lexer::ExecutablePartArena::decompose(
                image.clone(),
                parts,
                tcl_lexer::word_parts::SubstFlags::default(),
                self.config,
            )
            .ok()?;
            for part in arena.all_parts() {
                match part.part {
                    tcl_lexer::ExecutablePart::ParseError(_) => return None,
                    tcl_lexer::ExecutablePart::Command { body } => {
                        regions.push(OriginalExpressionCommandRegion {
                            substitution: part.span,
                            body,
                        })
                    }
                    _ => {}
                }
            }
        }
        regions.sort_unstable_by_key(|region| {
            (region.substitution.start(), region.substitution.end())
        });
        regions.dedup();
        Some(regions)
    }
}

#[cfg(test)]
mod configured_command_region_controls {
    use super::*;

    #[test]
    fn complete_expression_regions_keep_quoted_and_index_commands_but_not_literal_data() {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let context = ExprParseContext::for_profile(profile);
        let mut config = LexerConfig::for_file_grammar(profile.grammar);
        config.strict_quoting = !config.strict_quoting;
        let text = "\"[quoted]\" eq {[inert]} || $a([get]) + [expr {1}]";
        let original = OriginalSourceExpression::capture(text, &context, config).unwrap();
        let regions = original.command_regions().unwrap();
        assert_eq!(
            regions
                .iter()
                .map(|region| &text[region.substitution.as_range()])
                .collect::<Vec<_>>(),
            ["[quoted]", "[get]", "[expr {1}]"]
        );
        assert_eq!(
            regions
                .iter()
                .map(|region| &text[region.body.as_range()])
                .collect::<Vec<_>>(),
            ["quoted", "get", "expr {1}"]
        );
        assert_eq!(original.lexer_config(), config);
        assert!(OriginalSourceExpression::capture("[unterminated", &context, config).is_none());
    }
}

impl OriginalSourceExpression {
    /// Original operator token of one actual retained binary subtree.
    /// Complete tree/source/configuration ownership is retained. Only a unique
    /// positioned subtree in that retained tree supplies this geometry.
    /// Geometry is separate from operator semantics and rewrite equivalence.
    #[must_use]
    pub fn binary_operator_span(&self, node: &ExprNode) -> Option<Span> {
        let mut selected = self
            .tree
            .evaluation_nodes()
            .into_iter()
            .filter(|candidate| *candidate == node);
        let node = selected.next()?;
        if selected.next().is_some() {
            return None;
        }
        let ExprNode::Binary { op, left, right } = node else {
            return None;
        };
        let (_, left_end) = left.lexical_source_extent()?;
        let (right_start, _) = right.lexical_source_extent()?;
        let gap_start = left_end.checked_add(1)?;
        if gap_start > right_start {
            return None;
        }
        let (tokens, failures) = tcl_lexer::tokenise_expr_bytes_with_config_and_failures(
            self.source.as_bytes(),
            &self.context.lexer_grammar,
            self.context.expr_grammar_base,
            self.context.f5_word_grammar,
            self.config,
        );
        failures.is_empty().then_some(())?;
        let mut operators = tokens.iter().filter(|token| {
            token.kind == tcl_lexer::ExprTokenType::Operator
                && token.start >= gap_start
                && token.end < right_start
                && token.text.as_slice() == op.as_str().as_bytes()
        });
        let token = operators.next()?;
        if operators.next().is_some() {
            return None;
        }
        let span = Span::new(token.start, token.end.checked_add(1)?);
        (self.source.get(span.as_range())? == op.as_str()).then_some(span)
    }
}

#[cfg(test)]
mod original_binary_operator_geometry_controls {
    use super::*;

    #[test]
    fn complete_configured_tree_selects_operator_tokens_not_string_occurrences() {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let context = ExprParseContext::for_profile(profile);
        let mut config = LexerConfig::for_file_grammar(profile.grammar);
        config.strict_quoting = !config.strict_quoting;
        let source = "$x == \"literal == bytes\" && ($y != \"bar\")";
        let original = OriginalSourceExpression::capture(source, &context, config).unwrap();
        let mut spans = original
            .tree
            .evaluation_nodes()
            .into_iter()
            .filter_map(|node| {
                let ExprNode::Binary { op, .. } = node else {
                    return None;
                };
                if matches!(op, super::super::BinOp::Eq | super::super::BinOp::Ne) {
                    Some(original.binary_operator_span(node).unwrap())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        spans.sort_by_key(|span| span.start());
        assert_eq!(spans.len(), 2);
        assert_eq!(source.get(spans[0].as_range()), Some("=="));
        assert_eq!(source.get(spans[1].as_range()), Some("!="));
        assert_eq!(spans[0].start(), 3);
        let foreign =
            OriginalSourceExpression::capture("$other == \"foreign\"", &context, config).unwrap();
        assert!(original.binary_operator_span(&foreign.tree).is_none());
        assert!(OriginalSourceExpression::capture("$x ==", &context, config).is_none());
    }
}

impl OriginalSourceExpression {
    /// Source text for a deliberately proposed Tcl argument corresponding to
    /// one unique complete original expression leaf. The retained source tree,
    /// parser context and full configuration remain the geometry owners.
    /// This checks source shape and substitution phase only: argument values,
    /// lookup, evaluation order, effects and rewrite equivalence remain unknown.
    #[must_use]
    pub fn proposed_tcl_argument_source(&self, node: &ExprNode) -> Option<String> {
        let mut selected = self
            .tree
            .evaluation_nodes()
            .into_iter()
            .filter(|candidate| *candidate == node);
        let node = selected.next()?;
        if selected.next().is_some() {
            return None;
        }
        let (text, start, end) = match node {
            ExprNode::Literal { text, start, end }
            | ExprNode::String { text, start, end }
            | ExprNode::Var {
                text, start, end, ..
            }
            | ExprNode::Command { text, start, end } => (text, *start, *end),
            _ => return None,
        };
        let span = super::substitution::expression_span(&self.source, start, end)?;
        let source = self.source.get(span.as_range())?;
        if source != text {
            return None;
        }
        if matches!(node, ExprNode::Var { .. })
            && node.variable_source_span(0, self.config) != Some(span)
        {
            return None;
        }
        let image = tcl_lexer::SourceImage::document(source);
        let local_end = u32::try_from(source.len()).ok()?;
        let plan = tcl_lexer::native_script_words_in_with_purpose(
            image,
            Span::new(0, local_end),
            self.config,
            tcl_lexer::SourceSegmentationPurpose::WholeInput,
        )
        .ok()?;
        let [command] = plan.commands.as_slice() else {
            return None;
        };
        let [word] = command.words.as_slice() else {
            return None;
        };
        if plan.fatal_tail.is_some()
            || word.span() != Span::new(0, local_end)
            || word.group().expand
        {
            return None;
        }
        if matches!(node, ExprNode::String { .. })
            && !matches!(
                word.group().kind,
                tcl_lexer::WordKind::Braced | tcl_lexer::WordKind::Quoted
            )
        {
            return None;
        }
        if matches!(node, ExprNode::Var { .. } | ExprNode::Command { .. }) {
            let arena = word.executable_parts();
            let [part] = arena.list(arena.root()) else {
                return None;
            };
            match (node, &part.part) {
                (ExprNode::Var { .. }, tcl_lexer::ExecutablePart::Variable { .. })
                | (ExprNode::Command { .. }, tcl_lexer::ExecutablePart::Command { .. }) => {}
                _ => return None,
            }
        }
        Some(source.to_owned())
    }
}

#[cfg(test)]
mod proposed_expression_argument_source_tests {
    use super::*;

    #[test]
    fn leaf_argument_proposals_keep_original_source_delimiters_and_variable_geometry() {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let context = ExprParseContext::for_profile(profile);
        let config = LexerConfig::from_grammar(context.lexer_grammar);
        for (text, expected) in [
            ("${name} lt $other", "${name}"),
            ("\"two words\" lt $other", "\"two words\""),
            ("[list value] lt $other", "[list value]"),
        ] {
            let original = OriginalSourceExpression::capture(text, &context, config).unwrap();
            let ExprNode::Binary { left, .. } = &original.tree else {
                panic!("binary source");
            };
            assert_eq!(
                original.proposed_tcl_argument_source(left),
                Some(expected.to_owned())
            );
            let foreign =
                OriginalSourceExpression::capture("$foreign lt 3", &context, config).unwrap();
            let ExprNode::Binary { left, .. } = &foreign.tree else {
                unreachable!()
            };
            assert!(original.proposed_tcl_argument_source(left).is_none());
        }
    }
}
