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

//! W003 and W002 inside `expr`: operators and math functions the active dialect does not have, and the portable rewrite of a gated operator.

use super::super::helpers::is_ident_continue;
use crate::analyser::state::Analyser;
use crate::analyser::types::Severity;
use crate::expr_ast::ExprNode;
use tcl_core_types::DiagCode;
use tcl_dialect::model::SpecSurface;

impl Analyser {
    /// **W003.** Emit "Expression operator not available in active
    /// dialect" warnings for an EXPR-role argument that uses a Tcl
    /// 9.0 string-comparison operator (`lt` / `le` / `gt` / `ge`,
    /// TIP 461) in a pre-9.0 dialect, or `in` / `ni` (TIP 201, Tcl
    /// 8.5+) in an earlier one. One diagnostic is emitted per
    /// offending operator *occurrence*, anchored at that operator's
    /// own token span rather than the whole argument, so the
    /// squiggle stays tight around the actual problem even inside a
    /// large compound expression.
    ///
    /// `content_span` is the argument's inner content: the caller has
    /// already stripped any wrapping `{}` / `""` / `$` via the
    /// token's `content_offset`, so this slices `self.source`
    /// directly instead of trusting a possibly-reconstructed text —
    /// a quoted `"$x lt $y"` argument's word text is canonicalised to
    /// `"${x} lt ${y}"` by the segmenter, which would misalign any
    /// offset computed from it.
    pub(in crate::analyser) fn emit_w003_dialect_invalid_expr_operator(
        &mut self,
        content_span: tcl_lexer::Span,
    ) {
        let start = content_span.start() as usize;
        let end = content_span.end() as usize;
        let Some(expr_text) = Analyser::source_slice(&self.source, start, end) else {
            return;
        };
        // Quick lexical bail-out — the gated operators are short
        // word-shaped keywords; if none appear as a whole word we
        // can skip the parse.  Boundary check uses ASCII identifier
        // continuation so `tab`-, `newline`-, and start/end-of-text
        // boundaries all count (mirrors Tcl expr's whitespace
        // tolerance — `$x\tlt\t$y` and a wrapped `in` expression
        // both qualify).
        if !contains_gated_word(expr_text) {
            return;
        }
        let Some(original) = self.original_expression_parser_context() else {
            return;
        };
        let Some(base) = original.expr_grammar_base else {
            return;
        };
        let f5_words = original.f5_word_grammar.is_some();

        // Operator assistance recognises the full shipped vocabulary while
        // preserving this document's lexical rules. Availability remains the
        // actual profile's gate above; this tree carries no native proof.
        let assistance = gated_operator_assistance_context(original);
        let parsed =
            tcl_syntax::expr::parser::parse_expr_with_syntax_context(expr_text, &assistance);
        if matches!(parsed, ExprNode::Raw { .. }) {
            return;
        }

        // Re-tokenise the same expression at the flat lexical
        // level: every gated-keyword `Operator` token here
        // corresponds 1:1 to a `Binary` node the parse above just
        // confirmed is real (a successful parse consumes the whole
        // token stream, and `in`/`ni`/`lt`/`le`/`gt`/`ge`/the iRules
        // words are not valid prefix/atom tokens — the only way one is
        // consumed is as a genuine infix application). This gives an
        // exact per-occurrence span directly from `ExprToken::start/end`
        // without adding source-position fields to `ExprNode::Binary`
        // (which recursive/optimiser/codegen consumers across the
        // compiler pattern-match on by name, not span).
        let (tokens, _) = tcl_lexer::tokenise_expr_checked_with_expression_grammar(
            expr_text,
            &assistance.lexer_grammar,
            assistance.expr_grammar_base,
            assistance.f5_word_grammar,
        );
        let gated: Vec<(&tcl_lexer::ExprToken, &'static str)> = tokens
            .iter()
            .filter(|t| t.kind == tcl_lexer::ExprTokenType::Operator)
            .filter_map(|t| gated_operator_name(&t.text, base, f5_words).map(|name| (t, name)))
            .collect();
        if gated.is_empty() {
            return;
        }

        // A mechanical rewrite is only safe to offer when the whole
        // argument IS exactly the one gated application (`2 in {1 2
        // 3}`, `$x lt $y`) with plain operands: the operator nested
        // inside a larger expression, more than one gated occurrence,
        // or an operand that isn't a single bare-word-safe atom
        // (`Binary`/`Unary`/`Ternary`/`Call` — e.g. `max($a, $b)`
        // contains an unprotected space) can't be reduced to one
        // local text replacement.
        let fix_text = (gated.len() == 1)
            .then_some(&parsed)
            .and_then(|node| match node {
                ExprNode::Binary { op, left, right }
                    if is_simple_operand(left) && is_simple_operand(right) =>
                {
                    self.gated_operator_source_proposal(
                        content_span,
                        expr_text,
                        &assistance,
                        &parsed,
                        (op.as_str(), left, right),
                    )
                }
                _ => None,
            });

        for (tok, op_name) in gated {
            let op_span = tcl_lexer::Span::new(
                content_span.start() + tok.start,
                // `ExprToken::end` is inclusive; `Span` is exclusive.
                content_span.start() + tok.end + 1,
            );
            let mut fixes = Vec::new();
            if let Some(new_text) = fix_text.clone() {
                fixes.push(super::super::types::CodeFix {
                    span: tcl_lexer::Span::new(
                        content_span.start(),
                        content_span.start() + u32::try_from(expr_text.len()).unwrap_or(0),
                    ),
                    new_text,
                    description: format!(
                        "Rewrite to a form supported by dialect '{}'",
                        self.dialect()
                    ),
                    // W003: the older-dialect spelling of an operator is rarely exact —
                    // `lt` compares as strings where `<` coerces numerically.
                    safety: crate::irules_checks::FixSafety::RequiresReview,
                });
            }
            self.result.diagnostics.push(crate::analyser::types::Diagnostic::new(
    DiagCode::W003,
    op_span,
    format!(
                    "Expression operator '{op_name}' is not available in dialect '{}'; requires {}.",
                    self.dialect(),
                    w003_tip_citation(op_name)
                ),
    Severity::Warning,
).with_fixes(fixes));
        }
    }

    fn gated_operator_source_proposal(
        &self,
        content_span: tcl_lexer::Span,
        expr_text: &str,
        assistance: &tcl_syntax::expr::parser::ExprParseContext,
        parsed: &ExprNode,
        (op, left, right): (&str, &ExprNode, &ExprNode),
    ) -> Option<String> {
        let config = self.file_lexer_config();
        let image = tcl_lexer::SourceImage::document(&self.source);
        let (word, offset) = self
            .head_identities
            .original_written_word_at_span_in_source(&image, content_span, config)?;
        if word.group().kind != tcl_lexer::WordKind::Braced
            || word.content_span().ok()? != content_span
            || word.image().bytes().get(content_span.as_range())?
                != self.source.as_bytes().get(content_span.as_range())?
        {
            return None;
        }
        let (command, _) = self
            .head_identities
            .invocation_at_source("", offset)
            .original_recorded_command()?;
        let original = self.original_diagnostic_source_for_segment(&command)?;
        let registry = original.registry()?;
        if !registry.matches_analysis(&self.result, &self.source) {
            return None;
        }
        let roles = registry
            .with_schema(crate::analyser::diagnostic_registry::source_expression_arguments)??;
        let mut matching = roles.arguments.into_iter().filter(|&argument| {
            original.written_index(argument).is_some() && original.word(argument) == Some(&word)
        });
        matching.next()?;
        if matching.next().is_some() {
            return None;
        }
        let source = tcl_syntax::expr::substitution::OriginalSourceExpression::capture(
            expr_text, assistance, config,
        )?;
        if !source.matches_tree(parsed) {
            return None;
        }
        let left = source.proposed_tcl_argument_source(left)?;
        let right = source.proposed_tcl_argument_source(right)?;
        let proposal = rewrite_gated_operator(op, &left, &right)?;
        // The shared expression AST owns the generated bracket's real script
        // region. No prefix stripping or parsed name donates original identity.
        let proposed = tcl_syntax::expr::substitution::OriginalSourceExpression::capture(
            &proposal, assistance, config,
        )?;
        let regions = proposed.command_regions()?;
        let [region] = regions.as_slice() else {
            return None;
        };
        let body = proposal.get(region.body.as_range())?;
        let plan = tcl_lexer::native_script_words_in_with_purpose(
            tcl_lexer::SourceImage::document(body),
            tcl_lexer::Span::new(0, u32::try_from(body.len()).ok()?),
            config,
            tcl_lexer::SourceSegmentationPurpose::WholeInput,
        )
        .ok()?;
        let [command] = plan.commands.as_slice() else {
            return None;
        };
        registry.words().with_stock_source_command_proposal(
            registry.context(),
            &command.words,
            |_| (),
        )?;
        Some(proposal)
    }

    /// **W002** for a math function used before its introducing release —
    /// `min(…)` under 8.4, an `is*(…)` classification under 8.6.  The function
    /// dispatches to `::tcl::mathfunc::<name>`, a command that simply does not
    /// exist in the older core, so the same disabled-in-dialect diagnostic the
    /// command path emits applies to the function token in the expression.
    pub(in crate::analyser) fn emit_expr_function_dialect_diagnostics(
        &mut self,
        expr_tok: tcl_lexer::Token,
    ) {
        let Some(ceiling) = crate::tcl_expr_eval::math_func_ceiling_for_dialect(self.profile)
        else {
            return;
        };
        for (name, span, _argc) in self.expr_function_calls(expr_tok) {
            let Some(since) = tcl_syntax::expr::mathfunc::added_in(&name) else {
                continue;
            };
            if since > ceiling {
                self.result
                    .diagnostics
                    .push(crate::analyser::types::Diagnostic::new(
                        DiagCode::W002,
                        span,
                        format!(
                            "'{}' is disabled in the active dialect profile ('{}')",
                            tcl_registry::mathfunc::qualified_name(&name),
                            self.dialect()
                        ),
                        Severity::Warning,
                    ));
            }
        }
    }

    /// **W003** for the multi-word `expr a b c …` form. Tcl's `expr`
    /// is the only EXPR-role command whose expression can be spread
    /// across several unbraced words (`if`/`while`/`for` require
    /// their condition to be a single Tcl word — an unbraced
    /// multi-word condition is a Tcl arity error, not valid syntax).
    /// Written this way, a gated operator keyword is necessarily its
    /// own standalone Tcl word, so its `arg_tokens` entry is already
    /// an exact, tight span — no text-offset remapping needed, unlike
    /// the single-argument path above.
    pub(in crate::analyser) fn emit_w003_dialect_invalid_expr_words(
        &mut self,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
        joined_text: &str,
    ) {
        if !contains_gated_word(joined_text) {
            return;
        }
        let Some(original) = self.original_expression_parser_context() else {
            return;
        };
        let Some(base) = original.expr_grammar_base else {
            return;
        };
        let f5_words = original.f5_word_grammar.is_some();
        let assistance = gated_operator_assistance_context(original);
        let parsed = tcl_syntax::expr::parser::parse_expr_with_syntax_context(
            joined_text.trim(),
            &assistance,
        );
        if matches!(parsed, ExprNode::Raw { .. }) {
            return;
        }
        for (word, tok) in args.iter().zip(arg_tokens.iter()) {
            let Some(op_name) = gated_operator_name(word, base, f5_words) else {
                continue;
            };
            self.result.diagnostics.push(crate::analyser::types::Diagnostic::new(
    DiagCode::W003,
    tok.span,
    format!(
                    "Expression operator '{op_name}' is not available in dialect '{}'; requires {}.",
                    self.dialect(),
                    w003_tip_citation(op_name)
                ),
    Severity::Warning,
).with_fixes(// The unbraced multi-word form has no single local
                // span to rewrite in place without also re-bracing
                // the whole expression (W100's concern, not this
                // one) — left unfixed.
Vec::new()));
        }
    }
}

/// Recognition context for operator availability assistance, never native
/// syntax acceptance. The warning compares each recognised operator with the
/// actual profile separately; lexical and numeral rules remain source-selected.
fn gated_operator_assistance_context(
    mut context: tcl_syntax::expr::parser::ExprParseContext,
) -> tcl_syntax::expr::parser::ExprParseContext {
    context.expr_grammar_base = Some(tcl_dialect::TclVersion::V9_1);
    context.f5_word_grammar = tcl_dialect::DialectProfile::irules().f5_core_expr_grammar();
    context.native_syntax = tcl_syntax::expr::parser::NativeExprSyntax::Unknown;
    context
}

/// A version-gated `expr` operator: its text, whether it is word-shaped, the
/// minimum Tcl version it needs, and the TIP citation for the W003 message.
///
/// Single source for the three W003 steps — the prefilter
/// ([`contains_gated_word`]), the per-token gate check ([`gated_operator_name`]),
/// and the message ([`w003_tip_citation`]). `op`/`word_shaped`/`min_version`
/// are derived from [`tcl_syntax::expr::operators`] — only the TIP citation
/// number is local prose (two
/// operators can share a minimum version but not a TIP: `**` is TIP 123,
/// `in`/`ni` is TIP 201, both Tcl 8.5). Three separate hardcoded matches
/// drift from each other instead — with the symbolic `**` in none of them,
/// `expr {2 ** 3}` under tcl8.4 is a false negative.
struct GatedExprOp {
    /// The operator text as the expr lexer emits it (`in`, `**`).
    op: &'static str,
    /// Word-shaped (`in`/`lt`) operators need identifier-boundary matching in
    /// the prefilter; symbolic ones (`**`) match on any occurrence.
    word_shaped: bool,
    /// What must hold for this operator to be valid: a minimum `expr`-grammar
    /// version, or iRules dialect identity.
    gate: ExprOpGate,
    /// The TIP citation surfaced in the W003 message.
    tip: &'static str,
}

/// What a [`GatedExprOp`] requires of the active dialect.
#[derive(Clone, Copy)]
enum ExprOpGate {
    /// The dialect's `expr`-grammar base version must be at least this.
    MinVersion(tcl_dialect::TclVersion),
    /// The dialect must *be* iRules (identity, not a version threshold) —
    /// these 9 word operators (`contains`, `and`, …) have no
    /// `::tcl::mathop` form and exist in no other dialect at any version.
    IrulesOnly,
}

/// The TIP citation string for `spelling`'s W003 message — the one fact a
/// gated operator's `tcl_syntax::expr::operators::OperatorSpec` doesn't
/// carry (see [`GatedExprOp`]'s docs for why).
fn w003_tip_string(spelling: &str) -> &'static str {
    match spelling {
        "**" => "Tcl 8.5+ (TIP 123)",
        "in" | "ni" => "Tcl 8.5+ (TIP 201)",
        "and" | "or" | "not" | "contains" | "starts_with" | "ends_with" | "equals"
        | "matches_glob" | "matches_regex" => "the iRules/BIG-IP Tcl dialect",
        // "lt" | "le" | "gt" | "ge", and the fallback `gated_expr_ops()`
        // itself guarantees no other spelling ever reaches this function.
        _ => "Tcl 9.0+ (TIP 461)",
    }
}

/// The dialect-gated `expr` operators: version-gated (`in`/`ni`/`**` from
/// 8.5; the string comparison words `lt`/`le`/`gt`/`ge` from 9.0) — every
/// `BinOp` whose
/// [`tcl_syntax::expr::operators::OperatorSpec::expr_grammar_min_version`]
/// is `Some(_)` — plus the 9 iRules-only word operators, every
/// `BinOp`/`UnaryOp` whose `OperatorSpec::dialects` is exactly
/// `Some(SpecSurface::IRULES)`. Computed once (not `const`: `OperatorSpec`
/// isn't cheaply iterable in a const context) and cached for the process
/// lifetime — W003 only calls into this after its own text prefilter narrows
/// to expressions that already contain a gated keyword, so this never runs
/// on a hot path.
fn gated_expr_ops() -> &'static [GatedExprOp] {
    static TABLE: std::sync::OnceLock<Vec<GatedExprOp>> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        let version_gated = tcl_syntax::expr::operators::ALL_BIN_OPS
            .iter()
            .filter_map(|op| {
                let spec = op.spec();
                let min_version = spec.expr_grammar_min_version?;
                Some(GatedExprOp {
                    op: spec.spelling,
                    word_shaped: spec.spelling.as_bytes()[0].is_ascii_alphabetic(),
                    gate: ExprOpGate::MinVersion(min_version),
                    tip: w003_tip_string(spec.spelling),
                })
            });
        let irules_bin = tcl_syntax::expr::operators::ALL_BIN_OPS
            .iter()
            .filter_map(|op| {
                let spec = op.spec();
                (spec.surface == Some(SpecSurface::IRULES)).then(|| GatedExprOp {
                    op: spec.spelling,
                    word_shaped: true,
                    gate: ExprOpGate::IrulesOnly,
                    tip: w003_tip_string(spec.spelling),
                })
            });
        let irules_un = tcl_syntax::expr::operators::ALL_UNARY_OPS
            .iter()
            .filter_map(|op| {
                let spec = op.spec();
                (spec.surface == Some(SpecSurface::IRULES)).then(|| GatedExprOp {
                    op: spec.spelling,
                    word_shaped: true,
                    gate: ExprOpGate::IrulesOnly,
                    tip: w003_tip_string(spec.spelling),
                })
            });
        version_gated.chain(irules_bin).chain(irules_un).collect()
    })
}

/// Return `true` if `text` contains any dialect-gated expression operator (a
/// word-shaped one as a whole word, or a symbolic one such as `**` anywhere).
/// Used as a fast prefilter to skip the expression parse for expressions that
/// obviously can't trigger W003. Word-boundary matching is whitespace-aware:
/// any non-identifier byte (parentheses, operators, …) counts as a boundary,
/// matching Tcl expr's tolerance for arbitrary whitespace between tokens.
pub(in super::super) fn contains_gated_word(text: &str) -> bool {
    let bytes = text.as_bytes();
    for g in gated_expr_ops() {
        let needle = g.op.as_bytes();
        let n = needle.len();
        let mut i = 0;
        while i + n <= bytes.len() {
            if &bytes[i..i + n] == needle {
                let word_ok = !g.word_shaped
                    || ((i == 0 || !is_ident_continue(bytes[i - 1]))
                        && (i + n == bytes.len() || !is_ident_continue(bytes[i + n])));
                if word_ok {
                    return true;
                }
            }
            i += 1;
        }
    }
    false
}

/// The gated operator name for `word` under a dialect whose `expr`-grammar
/// base version is `base` and whose F5-family word-operator acceptance is
/// `f5_words` — `None` when `word` isn't one of the dialect-gated keywords,
/// or the active dialect already satisfies its gate (version threshold met,
/// or the dialect is F5Tcl-cored for a word-form operator — a trunk fact,
/// measured valid in tmsh/iApp expr too, measurements §4a).
fn gated_operator_name(
    word: &str,
    base: tcl_dialect::TclVersion,
    f5_words: bool,
) -> Option<&'static str> {
    gated_expr_ops()
        .iter()
        .find(|g| {
            g.op == word
                && match g.gate {
                    ExprOpGate::MinVersion(min) => base < min,
                    ExprOpGate::IrulesOnly => !f5_words,
                }
        })
        .map(|g| g.op)
}

/// The TIP citation to surface in the W003 message for `op_name`.
fn w003_tip_citation(op_name: &str) -> &'static str {
    gated_expr_ops()
        .iter()
        .find(|g| g.op == op_name)
        .map_or("Tcl 9.0+ (TIP 461)", |g| g.tip)
}

/// Whether `node` is safe to splice as a bare Tcl word into a
/// rewritten command (`lsearch` / `string compare`) with no extra
/// quoting: it never contains an unprotected top-level space. Plain
/// atoms are always safe (`Literal` is bare digits, `String` already
/// carries its own `{}`/`""` delimiters, `Var` is `$name`/`${name}`,
/// `Command` is bracket-delimited); `Binary`/`Unary`/`Ternary` would
/// need re-parenthesising and `Call` (`max($a, $b)`) contains an
/// unprotected space after the comma, so none of those are rewritten.
fn is_simple_operand(node: &ExprNode) -> bool {
    matches!(
        node,
        ExprNode::Literal { .. }
            | ExprNode::String { .. }
            | ExprNode::Var { .. }
            | ExprNode::Command { .. }
    )
}

/// Build the portable rewrite for a dialect-gated `expr` operator
/// application, or `None` if `op_name` isn't one of the six gated
/// keywords. `left`/`right` must already be safe bare-word text (see
/// [`is_simple_operand`]) — the caller is responsible for checking
/// that before rendering them.
fn rewrite_gated_operator(op_name: &str, left: &str, right: &str) -> Option<String> {
    Some(match op_name {
        "in" => format!("([lsearch -exact {right} {left}] >= 0)"),
        "ni" => format!("([lsearch -exact {right} {left}] < 0)"),
        "lt" => format!("([string compare {left} {right}] < 0)"),
        "le" => format!("([string compare {left} {right}] <= 0)"),
        "gt" => format!("([string compare {left} {right}] > 0)"),
        "ge" => format!("([string compare {left} {right}] >= 0)"),
        _ => return None,
    })
}
