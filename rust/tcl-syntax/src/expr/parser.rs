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

//! Pratt parser for Tcl `[expr]` expressions.
//!
//! Builds a structured [`ExprNode`] tree from the flat token list
//! produced by [`tokenise_expr`](tcl_lexer::tokenise_expr). Falls
//! back to [`ExprNode::Raw`] on any parse error, ensuring the
//! compiler pipeline never crashes on malformed expressions.
//!
//! Tcl expression precedence (low → high), following the Tcl man page:
//!
//! 1. `? :`     (ternary, right-associative)
//! 2. `||`
//! 3. `&&`
//! 4. `|`
//! 5. `^`
//! 6. `&`
//! 7. `== != eq ne`
//! 8. `< > <= >= lt le gt ge in ni`
//! 9. `<< >>`
//! 10. `+ -`
//! 11. `* / %`
//! 12. `**`       (right-associative, below unary operators in Tcl)
//! 13. unary `+ - ~ ! not`
//! 14. atoms, function calls, parenthesised sub-expressions

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, OnceLock};

use tcl_dialect::{DialectProfile, NumberSyntax};
use tcl_lexer::{ExprToken, ExprTokenType, tokenise_expr_checked};

use crate::expr::ast::{BinOp, ExprNode, UnaryOp};
use crate::naming::normalise_var_name;

/// Binding powers for binary operators: `(left_bp, right_bp)`.
///
/// Left-associative:  `right_bp = left_bp + 1`
/// Right-associative: `right_bp = left_bp`
fn binary_bp(op_text: &str) -> Option<(u8, u8)> {
    Some(match op_text {
        // Logical
        "||" | "or" => (4, 5),
        "&&" | "and" => (6, 7),
        // Bitwise
        "|" => (8, 9),
        "^" => (10, 11),
        "&" => (12, 13),
        // Equality / string comparison / iRules comparisons
        "==" | "!=" | "eq" | "ne" | "contains" | "starts_with" | "ends_with" | "equals"
        | "matches_glob" | "matches_regex" => (14, 15),
        // Relational / list membership
        "<" | ">" | "<=" | ">=" | "in" | "ni" | "lt" | "le" | "gt" | "ge" => (16, 17),
        // Shift
        "<<" | ">>" => (18, 19),
        // Additive
        "+" | "-" => (20, 21),
        // Multiplicative
        "*" | "/" | "%" => (22, 23),
        // Exponentiation (right-associative, below unary)
        "**" => (23, 23),
        _ => return None,
    })
}

/// Map operator text to its [`BinOp`] variant.
fn binop_from_text(text: &str) -> Option<BinOp> {
    Some(match text {
        "+" => BinOp::Add,
        "-" => BinOp::Sub,
        "*" => BinOp::Mul,
        "/" => BinOp::Div,
        "%" => BinOp::Mod,
        "**" => BinOp::Pow,
        "<<" => BinOp::LShift,
        ">>" => BinOp::RShift,
        "&" => BinOp::BitAnd,
        "|" => BinOp::BitOr,
        "^" => BinOp::BitXor,
        "&&" => BinOp::And,
        "||" => BinOp::Or,
        "==" => BinOp::Eq,
        "!=" => BinOp::Ne,
        "<" => BinOp::Lt,
        "<=" => BinOp::Le,
        ">" => BinOp::Gt,
        ">=" => BinOp::Ge,
        "eq" => BinOp::StrEq,
        "ne" => BinOp::StrNe,
        "lt" => BinOp::StrLt,
        "le" => BinOp::StrLe,
        "gt" => BinOp::StrGt,
        "ge" => BinOp::StrGe,
        "in" => BinOp::In,
        "ni" => BinOp::Ni,
        "and" => BinOp::WordAnd,
        "or" => BinOp::WordOr,
        "contains" => BinOp::Contains,
        "starts_with" => BinOp::StartsWith,
        "ends_with" => BinOp::EndsWith,
        "equals" => BinOp::StrEquals,
        "matches_glob" => BinOp::MatchesGlob,
        "matches_regex" => BinOp::MatchesRegex,
        _ => return None,
    })
}

/// Map operator text to its [`UnaryOp`] variant.
fn unaryop_from_text(text: &str) -> Option<UnaryOp> {
    Some(match text {
        "-" => UnaryOp::Neg,
        "+" => UnaryOp::Pos,
        "~" => UnaryOp::BitNot,
        "!" => UnaryOp::Not,
        "not" => UnaryOp::WordNot,
        _ => return None,
    })
}

/// Whether `text` spells an operator this parser accepts in prefix position.
///
/// The [`syntax_error`](super::syntax_error) diagnosis pass needs the same
/// unary/binary split the parser applies, so it reads it from here rather than
/// keeping a second list that could drift.
pub(super) fn is_unary_operator(text: &str) -> bool {
    unaryop_from_text(text).is_some()
}

/// Whether `text` spells an operator this parser accepts in infix position.
/// Companion to [`is_unary_operator`]: `-` is both, `~` and `!` are prefix only.
pub(super) fn is_binary_operator(text: &str) -> bool {
    binop_from_text(text).is_some()
}

/// Binding power for prefix unary operators (higher than any binary).
const UNARY_BP: u8 = 24;

/// Internal parse error — caught by [`parse_expr`] and converted to
/// [`ExprNode::Raw`].
#[derive(Debug)]
struct ParseError;

/// Maximum nesting depth for the recursive-descent `expression`
/// parser.  Deeply-nested input (`((((…))))`, chained unary/ternary)
/// would otherwise recurse one Rust frame per level and overflow the
/// stack — an uncatchable SIGABRT.  At the cap we bail with a
/// [`ParseError`], which `parse_expr` converts to [`ExprNode::Raw`],
/// honouring the "never crashes on malformed expressions" contract.
/// No hand-written or generated expression nests anywhere near this.
const MAX_EXPR_DEPTH: usize = 256;

/// Pratt (top-down operator precedence) parser for Tcl expressions.
struct PrattParser<'a> {
    tokens: &'a [ExprToken],
    pos: usize,
    /// Current recursion depth, bounded by [`MAX_EXPR_DEPTH`].
    depth: usize,
    /// The release's numeric-literal grammar, used to reject a `Number` token
    /// the lexer only *delimited* (see [`Self::number_literal`]).
    numbers: NumberSyntax,
}

impl<'a> PrattParser<'a> {
    fn new(tokens: &'a [ExprToken], numbers: NumberSyntax) -> Self {
        Self {
            tokens,
            pos: 0,
            depth: 0,
            numbers,
        }
    }

    fn peek(&self) -> Option<&'a ExprToken> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> &'a ExprToken {
        let tok = &self.tokens[self.pos];
        self.pos += 1;
        tok
    }

    fn expect(&mut self, kind: ExprTokenType) -> Result<&'a ExprToken, ParseError> {
        let tok = self.peek().ok_or(ParseError)?;
        if tok.kind != kind {
            return Err(ParseError);
        }
        Ok(self.advance())
    }

    /// Parse an expression with minimum binding power `min_bp`.
    ///
    /// Depth-guarded: every recursive descent (prefix unary, parens,
    /// ternary arms, function args) re-enters here, so bounding this one
    /// entry point caps the whole recursion at [`MAX_EXPR_DEPTH`].
    fn expression(&mut self, min_bp: u8) -> Result<ExprNode, ParseError> {
        self.depth += 1;
        if self.depth > MAX_EXPR_DEPTH {
            self.depth -= 1;
            return Err(ParseError);
        }
        let result = self.expression_inner(min_bp);
        self.depth -= 1;
        result
    }

    fn expression_inner(&mut self, min_bp: u8) -> Result<ExprNode, ParseError> {
        let mut left = self.prefix()?;

        while let Some(tok) = self.peek() {
            // Ternary operator
            if tok.kind == ExprTokenType::TernaryQ {
                if 2 < min_bp {
                    break;
                }
                self.advance();
                let true_branch = self.expression(0)?;
                self.expect(ExprTokenType::TernaryC)?;
                let false_branch = self.expression(2)?;
                left = ExprNode::Ternary {
                    condition: Box::new(left),
                    true_branch: Box::new(true_branch),
                    false_branch: Box::new(false_branch),
                };
                continue;
            }

            // Binary operators
            if tok.kind == ExprTokenType::Operator {
                let Some(bp) = binary_bp(&tok.text) else {
                    break;
                };
                let (left_bp, right_bp) = bp;
                if left_bp < min_bp {
                    break;
                }
                let op_text = self.advance().text.clone();
                let right = self.expression(right_bp)?;
                let binop = binop_from_text(&op_text).ok_or(ParseError)?;
                left = ExprNode::Binary {
                    op: binop,
                    left: Box::new(left),
                    right: Box::new(right),
                };
                continue;
            }

            break;
        }

        Ok(left)
    }

    /// Parse a prefix expression (atoms, unary operators, parens, function calls).
    fn prefix(&mut self) -> Result<ExprNode, ParseError> {
        let tok = self.peek().ok_or(ParseError)?;

        // Unary operators
        if tok.kind == ExprTokenType::Operator
            && let Some(op) = unaryop_from_text(&tok.text)
        {
            self.advance();
            let operand = self.expression(UNARY_BP)?;
            return Ok(ExprNode::Unary {
                op,
                operand: Box::new(operand),
            });
        }

        // Parenthesised sub-expression
        if tok.kind == ExprTokenType::ParenOpen {
            self.advance();
            let inner = self.expression(0)?;
            self.expect(ExprTokenType::ParenClose)?;
            return Ok(inner);
        }

        // Number literal — but only if it really is a whole number in this
        // release. The lexer delimits a numeric-looking run without validating
        // it (it sits below the number parser), so `0o8`, `0x`, `0xg`, and
        // `0d99` before 9.0 arrive here as `Number` tokens. C rejects exactly
        // these in `ParseLexeme` — `TclParseNumber` fails to consume the whole
        // lexeme, so the text is a *bareword* — and reporting it that way is
        // what produces `invalid bareword "0o8"` instead of silently evaluating
        // the literal to its own text.
        if tok.kind == ExprTokenType::Number {
            if !crate::number::is_whole_number(&tok.text, self.numbers) {
                return Err(ParseError);
            }
            let tok = self.advance();
            return Ok(ExprNode::Literal {
                text: tok.text.clone(),
                start: tok.start,
                end: tok.end,
            });
        }

        // Boolean literal
        if tok.kind == ExprTokenType::Bool {
            let tok = self.advance();
            return Ok(ExprNode::Literal {
                text: tok.text.clone(),
                start: tok.start,
                end: tok.end,
            });
        }

        // String literal
        if tok.kind == ExprTokenType::String {
            let tok = self.advance();
            return Ok(ExprNode::String {
                text: tok.text.clone(),
                start: tok.start,
                end: tok.end,
            });
        }

        // Variable reference
        if tok.kind == ExprTokenType::Variable {
            let tok = self.advance();
            let name = normalise_var_name(&tok.text).to_owned();
            return Ok(ExprNode::Var {
                text: tok.text.clone(),
                name,
                start: tok.start,
                end: tok.end,
            });
        }

        // Command substitution
        if tok.kind == ExprTokenType::Command {
            let tok = self.advance();
            return Ok(ExprNode::Command {
                text: tok.text.clone(),
                start: tok.start,
                end: tok.end,
            });
        }

        // Function call: name ( args )
        if tok.kind == ExprTokenType::Function {
            return self.parse_function_call();
        }

        Err(ParseError)
    }

    fn parse_function_call(&mut self) -> Result<ExprNode, ParseError> {
        let func_tok = self.advance();
        let func_name = func_tok.text.clone();
        let func_start = func_tok.start;

        self.expect(ExprTokenType::ParenOpen)?;

        let mut args = Vec::new();

        // Check for empty argument list
        if let Some(peek) = self.peek()
            && peek.kind == ExprTokenType::ParenClose
        {
            let close_tok = self.advance();
            return Ok(ExprNode::Call {
                function: func_name,
                args,
                start: func_start,
                end: close_tok.end,
            });
        }

        // Parse first argument
        args.push(self.expression(0)?);

        // Parse remaining comma-separated arguments
        loop {
            let peek = self.peek().ok_or(ParseError)?;
            if peek.kind == ExprTokenType::ParenClose {
                let close_tok = self.advance();
                return Ok(ExprNode::Call {
                    function: func_name,
                    args,
                    start: func_start,
                    end: close_tok.end,
                });
            }
            if peek.kind == ExprTokenType::Comma {
                self.advance();
                args.push(self.expression(0)?);
            } else {
                return Err(ParseError);
            }
        }
    }
}

/// Parse a Tcl expression string into a structured AST.
///
/// Returns [`ExprNode::Raw`] on any error, so the compiler pipeline
/// never crashes on malformed expressions.
///
/// ```
/// use tcl_syntax::expr::parser::parse_expr;
/// use tcl_syntax::expr::ast::{ExprNode, BinOp};
///
/// let node = parse_expr("$a + 1", None);
/// assert!(matches!(node, ExprNode::Binary { op: BinOp::Add, .. }));
///
/// // Malformed → ExprRaw fallback
/// let raw = parse_expr("", None);
/// assert!(matches!(raw, ExprNode::Raw { .. }));
/// ```
/// The numeric grammar to parse with: a named dialect's own, else the grammar
/// the runtime was built for ([`crate::number::runtime_syntax`]).
///
/// The fallback matters. An unnamed dialect resolves to the permissive 9.x
/// profile, so without this a runtime built for 8.6 would *parse* `0o17` as a
/// number and then fail to *read* it as one, leaving the literal to evaluate to
/// its own text — the silent-wrong-answer shape this whole change removes.
pub(super) fn numbers_for(dialect: Option<&str>, profile: &DialectProfile) -> NumberSyntax {
    if dialect.is_some() {
        profile.grammar.numbers
    } else {
        crate::number::runtime_syntax()
    }
}

#[must_use]
pub fn parse_expr(source: &str, dialect: Option<&str>) -> ExprNode {
    // Resolve the dialect string to its interned profile once and thread the
    // canonical name down — so the grammar branch in the expr lexer and the
    // cache key below can never disagree about what a given spelling means.
    let profile = DialectProfile::by_opt_name(dialect);
    let (raw_tokens, has_unknown) = tokenise_expr_checked(source, Some(profile.name));

    if has_unknown {
        return ExprNode::Raw {
            text: source.to_owned(),
        };
    }

    let tokens: Vec<ExprToken> = raw_tokens
        .into_iter()
        .filter(|t| !t.kind.is_skipped())
        .collect();

    if tokens.is_empty() {
        return ExprNode::Raw {
            text: source.to_owned(),
        };
    }

    let mut parser = PrattParser::new(&tokens, numbers_for(dialect, profile));
    match parser.expression(0) {
        Ok(result) if parser.pos >= tokens.len() => result,
        _ => ExprNode::Raw {
            text: source.to_owned(),
        },
    }
}

// LRU-cached parse_expr
//
// The analyser callers are once-per-source, so the existing
// `parse_expr` stays uncached; this sibling is for the VM, which
// re-evaluates loop conditions on every iteration.
//
// Key shape: `(source, profile identity)` — the dialect string is
// resolved through `DialectProfile::by_opt_name` and the canonical
// profile name is the key, so alias spellings and unknown-dialect
// typos share one entry per behaviour instead of one per spelling.
// The cache is process-global (a `OnceLock<Mutex<…>>`) and capped at
// 4096 entries with simple LRU eviction (move-to-back on hit, evict
// front on capacity overflow). Entries return `Arc<ExprNode>` so
// multiple callers can share the parsed tree without cloning the AST.

/// Cache capacity.  4096 entries was
/// empirically large enough to hold every distinct expression a
/// 10k-iter loop encounters (typically <10 distinct expressions
/// per proc); larger workloads stress the LRU eviction path.
const EXPR_CACHE_CAPACITY: usize = 4096;

type ExprCacheKey = (String, &'static str);

struct ExprCache {
    map: HashMap<ExprCacheKey, Arc<ExprNode>>,
    order: VecDeque<ExprCacheKey>,
}

impl ExprCache {
    fn new() -> Self {
        Self {
            map: HashMap::with_capacity(EXPR_CACHE_CAPACITY),
            order: VecDeque::with_capacity(EXPR_CACHE_CAPACITY),
        }
    }

    fn get(&mut self, key: &ExprCacheKey) -> Option<Arc<ExprNode>> {
        let value = self.map.get(key)?.clone();
        // Move-to-back on hit (LRU recency update).  Linear scan
        // of the deque; per-entry O(N) but N is bounded at 4096
        // and the alternative (a doubly-linked-list-with-cursors
        // shape) needs `unsafe` or a third-party LRU crate.
        if let Some(pos) = self.order.iter().position(|k| k == key)
            && pos + 1 < self.order.len()
            && let Some(k) = self.order.remove(pos)
        {
            self.order.push_back(k);
        }
        Some(value)
    }

    fn insert(&mut self, key: ExprCacheKey, value: Arc<ExprNode>) {
        if self.map.contains_key(&key) {
            self.map.insert(key.clone(), value);
            // Refresh recency.
            if let Some(pos) = self.order.iter().position(|k| k == &key)
                && let Some(k) = self.order.remove(pos)
            {
                self.order.push_back(k);
            }
            return;
        }
        if self.map.len() >= EXPR_CACHE_CAPACITY
            && let Some(old) = self.order.pop_front()
        {
            self.map.remove(&old);
        }
        self.map.insert(key.clone(), value);
        self.order.push_back(key);
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.map.len()
    }
}

fn expr_cache() -> &'static Mutex<ExprCache> {
    static CACHE: OnceLock<Mutex<ExprCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(ExprCache::new()))
}

/// LRU-cached `parse_expr`.
///
/// Identical semantics to [`parse_expr`] — same `(source, dialect)`
/// inputs return the same `ExprNode` shape — but cached on the
/// process-global LRU, keyed by `(source, resolved profile)`.  Two
/// calls with the same key return `Arc::ptr_eq` results; eviction is
/// FIFO once 4096 entries are reached (oldest evicted first).
///
/// Use this from VM-loop hot paths (re-evaluating `expr {$i < N}`
/// on every iteration); use the un-cached [`parse_expr`] from
/// once-per-invocation analyser sites.
#[must_use]
pub fn parse_expr_cached(source: &str, dialect: Option<&str>) -> Arc<ExprNode> {
    let profile = DialectProfile::by_opt_name(dialect);
    let key: ExprCacheKey = (source.to_owned(), profile.name);
    {
        let mut cache = expr_cache().lock().expect("expr cache mutex poisoned");
        if let Some(hit) = cache.get(&key) {
            return hit;
        }
    }
    let parsed = Arc::new(parse_expr(source, dialect));
    {
        let mut cache = expr_cache().lock().expect("expr cache mutex poisoned");
        // A concurrent caller may have populated the same key
        // between our miss and re-lock.  `insert` overwrites,
        // which is fine — the parsed AST is deterministic.
        cache.insert(key, parsed.clone());
    }
    parsed
}

#[cfg(test)]
#[doc(hidden)]
pub fn expr_cache_reset_for_tests() {
    let mut cache = expr_cache().lock().expect("expr cache mutex poisoned");
    cache.map.clear();
    cache.order.clear();
}

#[cfg(test)]
#[doc(hidden)]
#[must_use]
pub fn expr_cache_len_for_tests() -> usize {
    let cache = expr_cache().lock().expect("expr cache mutex poisoned");
    cache.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::ast::*;

    fn parse(s: &str) -> ExprNode {
        parse_expr(s, None)
    }

    fn parse_irules(s: &str) -> ExprNode {
        parse_expr(s, Some("f5-irules"))
    }

    /// Drift guard for this module's own `binop_from_text`/`unaryop_from_text`
    /// — a same-crate duplicate of `operators.rs`'s `BinOp`/`UnaryOp` ↔
    /// spelling mapping (`as_str()`, `ALL_BIN_OPS`/`ALL_UNARY_OPS`), with no
    /// dependency-direction excuse for the duplication. Proves the parser's
    /// reverse lookup agrees with the canonical spelling for every variant,
    /// in both directions.
    #[test]
    fn binop_and_unaryop_from_text_agree_with_operators_rs_for_every_variant() {
        use crate::expr::operators::{ALL_BIN_OPS, ALL_UNARY_OPS};

        for &op in ALL_BIN_OPS {
            let spelling = op.as_str();
            assert_eq!(
                binop_from_text(spelling),
                Some(op),
                "binop_from_text({spelling:?}) should round-trip to {op:?}"
            );
        }
        for &op in ALL_UNARY_OPS {
            let spelling = op.as_str();
            assert_eq!(
                unaryop_from_text(spelling),
                Some(op),
                "unaryop_from_text({spelling:?}) should round-trip to {op:?}"
            );
        }
    }

    // Adversarial nesting — must not overflow the stack

    #[test]
    fn deeply_nested_parens_does_not_overflow() {
        // ~9000 parens reproduced a SIGABRT stack overflow before the
        // depth guard.  Now it must degrade gracefully to `Raw`, never abort.
        let depth = 9000;
        let src = format!("{}1{}", "(".repeat(depth), ")".repeat(depth));
        let node = parse(&src);
        assert!(matches!(node, ExprNode::Raw { .. }));
    }

    #[test]
    fn deeply_chained_unary_does_not_overflow() {
        let src = format!("{}1", "-".repeat(9000));
        let node = parse(&src);
        assert!(matches!(node, ExprNode::Raw { .. }));
    }

    #[test]
    fn modestly_nested_parens_still_parses() {
        // Well under MAX_EXPR_DEPTH — real code is unaffected by the cap.
        let src = format!("{}1 + 2{}", "(".repeat(32), ")".repeat(32));
        let node = parse(&src);
        assert!(matches!(node, ExprNode::Binary { op: BinOp::Add, .. }));
    }

    // Literals

    #[test]
    fn integer_literal() {
        let node = parse("42");
        assert!(matches!(node, ExprNode::Literal { ref text, .. } if text == "42"));
    }

    #[test]
    fn float_literal() {
        let node = parse("3.14");
        assert!(matches!(node, ExprNode::Literal { ref text, .. } if text == "3.14"));
    }

    #[test]
    fn hex_literal() {
        let node = parse("0xFF");
        assert!(matches!(node, ExprNode::Literal { ref text, .. } if text == "0xFF"));
    }

    #[test]
    fn boolean_literal() {
        let node = parse("true");
        assert!(matches!(node, ExprNode::Literal { ref text, .. } if text == "true"));
    }

    #[test]
    fn string_literal() {
        let node = parse(r#""hello""#);
        assert!(matches!(node, ExprNode::String { ref text, .. } if text == r#""hello""#));
    }

    // Variables

    #[test]
    fn simple_var() {
        let node = parse("$x");
        if let ExprNode::Var { text, name, .. } = &node {
            assert_eq!(text, "$x");
            assert_eq!(name, "x");
        } else {
            panic!("expected Var, got {node:?}");
        }
    }

    #[test]
    fn braced_var() {
        let node = parse("${foo}");
        if let ExprNode::Var { name, .. } = &node {
            assert_eq!(name, "foo");
        } else {
            panic!("expected Var, got {node:?}");
        }
    }

    #[test]
    fn array_var() {
        let node = parse("$arr(idx)");
        if let ExprNode::Var { name, .. } = &node {
            assert_eq!(name, "arr");
        } else {
            panic!("expected Var, got {node:?}");
        }
    }

    #[test]
    fn namespace_var() {
        let node = parse("$ns::var");
        if let ExprNode::Var { name, .. } = &node {
            assert_eq!(name, "ns::var");
        } else {
            panic!("expected Var, got {node:?}");
        }
    }

    // Command substitution

    #[test]
    fn command_substitution() {
        let node = parse("[llength $list]");
        assert!(matches!(node, ExprNode::Command { .. }));
    }

    // Binary operators

    #[test]
    fn add() {
        let node = parse("$a + $b");
        assert!(matches!(node, ExprNode::Binary { op: BinOp::Add, .. }));
    }

    #[test]
    fn sub() {
        let node = parse("$a - $b");
        assert!(matches!(node, ExprNode::Binary { op: BinOp::Sub, .. }));
    }

    #[test]
    fn mul() {
        let node = parse("$a * $b");
        assert!(matches!(node, ExprNode::Binary { op: BinOp::Mul, .. }));
    }

    #[test]
    fn div() {
        let node = parse("$a / $b");
        assert!(matches!(node, ExprNode::Binary { op: BinOp::Div, .. }));
    }

    #[test]
    fn modulo() {
        let node = parse("$a % $b");
        assert!(matches!(node, ExprNode::Binary { op: BinOp::Mod, .. }));
    }

    #[test]
    fn pow() {
        let node = parse("$a ** $b");
        assert!(matches!(node, ExprNode::Binary { op: BinOp::Pow, .. }));
    }

    #[test]
    fn equality() {
        let node = parse("$a == $b");
        assert!(matches!(node, ExprNode::Binary { op: BinOp::Eq, .. }));
    }

    #[test]
    fn str_eq() {
        let node = parse("$a eq $b");
        assert!(matches!(
            node,
            ExprNode::Binary {
                op: BinOp::StrEq,
                ..
            }
        ));
    }

    #[test]
    fn logical_and() {
        let node = parse("$a && $b");
        assert!(matches!(node, ExprNode::Binary { op: BinOp::And, .. }));
    }

    #[test]
    fn logical_or() {
        let node = parse("$a || $b");
        assert!(matches!(node, ExprNode::Binary { op: BinOp::Or, .. }));
    }

    #[test]
    fn in_operator() {
        let node = parse("$x in $list");
        assert!(matches!(node, ExprNode::Binary { op: BinOp::In, .. }));
    }

    #[test]
    fn shift_left() {
        let node = parse("$a << 3");
        assert!(matches!(
            node,
            ExprNode::Binary {
                op: BinOp::LShift,
                ..
            }
        ));
    }

    // Unary operators

    #[test]
    fn unary_neg() {
        let node = parse("-$x");
        assert!(matches!(
            node,
            ExprNode::Unary {
                op: UnaryOp::Neg,
                ..
            }
        ));
    }

    #[test]
    fn unary_not() {
        let node = parse("!$x");
        assert!(matches!(
            node,
            ExprNode::Unary {
                op: UnaryOp::Not,
                ..
            }
        ));
    }

    #[test]
    fn unary_bitnot() {
        let node = parse("~$x");
        assert!(matches!(
            node,
            ExprNode::Unary {
                op: UnaryOp::BitNot,
                ..
            }
        ));
    }

    // Precedence

    #[test]
    fn mul_before_add() {
        // $a + $b * $c → ADD(a, MUL(b, c))
        let node = parse("$a + $b * $c");
        if let ExprNode::Binary {
            op: BinOp::Add,
            right,
            ..
        } = &node
        {
            assert!(matches!(**right, ExprNode::Binary { op: BinOp::Mul, .. }));
        } else {
            panic!("expected Add at top level, got {node:?}");
        }
    }

    #[test]
    fn left_associative_add() {
        // $a + $b + $c → ADD(ADD(a, b), c)
        let node = parse("$a + $b + $c");
        if let ExprNode::Binary {
            op: BinOp::Add,
            left,
            ..
        } = &node
        {
            assert!(matches!(**left, ExprNode::Binary { op: BinOp::Add, .. }));
        } else {
            panic!("expected Add at top level, got {node:?}");
        }
    }

    #[test]
    fn right_associative_pow() {
        // $a ** $b ** $c → POW(a, POW(b, c))
        let node = parse("$a ** $b ** $c");
        if let ExprNode::Binary {
            op: BinOp::Pow,
            right,
            ..
        } = &node
        {
            assert!(matches!(**right, ExprNode::Binary { op: BinOp::Pow, .. }));
        } else {
            panic!("expected Pow at top level, got {node:?}");
        }
    }

    #[test]
    fn parens_override_precedence() {
        // ($a + $b) * $c → MUL(ADD(a, b), c)
        let node = parse("($a + $b) * $c");
        if let ExprNode::Binary {
            op: BinOp::Mul,
            left,
            ..
        } = &node
        {
            assert!(matches!(**left, ExprNode::Binary { op: BinOp::Add, .. }));
        } else {
            panic!("expected Mul at top level, got {node:?}");
        }
    }

    #[test]
    fn unary_before_binary() {
        // -$a + $b → ADD(NEG(a), b)
        let node = parse("-$a + $b");
        if let ExprNode::Binary {
            op: BinOp::Add,
            left,
            ..
        } = &node
        {
            assert!(matches!(
                **left,
                ExprNode::Unary {
                    op: UnaryOp::Neg,
                    ..
                }
            ));
        } else {
            panic!("expected Add at top level, got {node:?}");
        }
    }

    // Ternary

    #[test]
    fn ternary() {
        let node = parse("$x ? 1 : 0");
        assert!(matches!(node, ExprNode::Ternary { .. }));
    }

    #[test]
    fn nested_ternary() {
        // $a ? 1 : $b ? 2 : 3 → Ternary(a, 1, Ternary(b, 2, 3))
        let node = parse("$a ? 1 : $b ? 2 : 3");
        if let ExprNode::Ternary { false_branch, .. } = &node {
            assert!(matches!(**false_branch, ExprNode::Ternary { .. }));
        } else {
            panic!("expected Ternary, got {node:?}");
        }
    }

    // Function calls

    #[test]
    fn function_no_args() {
        let node = parse("rand()");
        if let ExprNode::Call { function, args, .. } = &node {
            assert_eq!(function, "rand");
            assert!(args.is_empty());
        } else {
            panic!("expected Call, got {node:?}");
        }
    }

    #[test]
    fn function_one_arg() {
        let node = parse("sin($x)");
        if let ExprNode::Call { function, args, .. } = &node {
            assert_eq!(function, "sin");
            assert_eq!(args.len(), 1);
        } else {
            panic!("expected Call, got {node:?}");
        }
    }

    #[test]
    fn function_two_args() {
        let node = parse("max($a, $b)");
        if let ExprNode::Call { function, args, .. } = &node {
            assert_eq!(function, "max");
            assert_eq!(args.len(), 2);
        } else {
            panic!("expected Call, got {node:?}");
        }
    }

    #[test]
    fn function_nested() {
        let node = parse("int(sin($x))");
        if let ExprNode::Call { function, args, .. } = &node {
            assert_eq!(function, "int");
            assert_eq!(args.len(), 1);
            assert!(matches!(args[0], ExprNode::Call { .. }));
        } else {
            panic!("expected Call, got {node:?}");
        }
    }

    #[test]
    fn function_with_expr_arg() {
        let node = parse("abs($a - $b)");
        if let ExprNode::Call { args, .. } = &node {
            assert_eq!(args.len(), 1);
            assert!(matches!(args[0], ExprNode::Binary { op: BinOp::Sub, .. }));
        } else {
            panic!("expected Call, got {node:?}");
        }
    }

    // Complex expressions

    #[test]
    fn command_plus_literal() {
        let node = parse("[llength $list] + 1");
        if let ExprNode::Binary {
            op: BinOp::Add,
            left,
            right,
            ..
        } = &node
        {
            assert!(matches!(**left, ExprNode::Command { .. }));
            assert!(matches!(**right, ExprNode::Literal { .. }));
        } else {
            panic!("expected Binary Add, got {node:?}");
        }
    }

    #[test]
    fn mixed_operators() {
        // $a > 0 && $b < 10
        let node = parse("$a > 0 && $b < 10");
        if let ExprNode::Binary { op: BinOp::And, .. } = &node {
            // correct
        } else {
            panic!("expected And at top level, got {node:?}");
        }
    }

    // iRules operators

    #[test]
    fn irules_contains() {
        let node = parse_irules("$uri contains \"admin\"");
        assert!(matches!(
            node,
            ExprNode::Binary {
                op: BinOp::Contains,
                ..
            }
        ));
    }

    #[test]
    fn irules_word_and() {
        let node = parse_irules("$a and $b");
        assert!(matches!(
            node,
            ExprNode::Binary {
                op: BinOp::WordAnd,
                ..
            }
        ));
    }

    #[test]
    fn irules_word_not() {
        let node = parse_irules("not $x");
        assert!(matches!(
            node,
            ExprNode::Unary {
                op: UnaryOp::WordNot,
                ..
            }
        ));
    }

    // Fallback

    #[test]
    fn empty_string_is_raw() {
        let node = parse("");
        assert!(matches!(node, ExprNode::Raw { .. }));
    }

    #[test]
    fn whitespace_only_is_raw() {
        let node = parse("   ");
        assert!(matches!(node, ExprNode::Raw { .. }));
    }

    #[test]
    fn unconsumed_tokens_is_raw() {
        // "1 + 2 garbage" → ExprRaw because "garbage" is unconsumed
        let node = parse("1 + 2 garbage");
        // "garbage" is treated as a function name by the lexer, which
        // means the parser sees a function token after a complete
        // expression — unconsumed tokens → Raw fallback.
        // But actually, the parser might not fail here. Let's verify.
        // The parser stops after "1 + 2" because "garbage" (Function)
        // doesn't match any infix operator. Then pos < len → Raw.
        assert!(matches!(node, ExprNode::Raw { .. }));
    }

    // Variable extraction

    #[test]
    fn vars_in_binary() {
        let node = parse("$a + $b");
        let vars = node.vars();
        assert_eq!(vars.len(), 2);
        assert!(vars.contains("a"));
        assert!(vars.contains("b"));
    }

    #[test]
    fn vars_in_ternary() {
        let node = parse("$x ? $y : $z");
        let vars = node.vars();
        assert_eq!(vars.len(), 3);
        assert!(vars.contains("x"));
        assert!(vars.contains("y"));
        assert!(vars.contains("z"));
    }

    #[test]
    fn vars_in_function() {
        let node = parse("max($a, $b)");
        let vars = node.vars();
        assert_eq!(vars.len(), 2);
        assert!(vars.contains("a"));
        assert!(vars.contains("b"));
    }

    // Round-trip rendering

    #[test]
    fn render_simple_binary() {
        let node = parse("$a + $b");
        let rendered = crate::expr::ast::render_expr(&node);
        assert_eq!(rendered, "$a + $b");
    }

    #[test]
    fn render_precedence() {
        // ($a + $b) * $c → "(1 + 2) * 3" style (preserves parens)
        let node = parse("($a + $b) * $c");
        let rendered = crate::expr::ast::render_expr(&node);
        assert_eq!(rendered, "($a + $b) * $c");
    }

    #[test]
    fn render_ternary() {
        let node = parse("$x ? 1 : 0");
        let rendered = crate::expr::ast::render_expr(&node);
        assert_eq!(rendered, "$x ? 1 : 0");
    }

    #[test]
    fn render_function() {
        let node = parse("sin($x)");
        let rendered = crate::expr::ast::render_expr(&node);
        assert_eq!(rendered, "sin($x)");
    }

    #[test]
    fn render_unary() {
        let node = parse("-$x");
        let rendered = crate::expr::ast::render_expr(&node);
        assert_eq!(rendered, "-$x");
    }

    // parse_expr_cached
    //
    // The global cache is shared across the whole test binary, so
    // tests that assert on cache state (length / eviction) run
    // against the [`ExprCache`] type directly to stay isolated.  The
    // one integration test exercising [`parse_expr_cached`]
    // serialises through a local mutex so concurrent tests don't
    // race on the global cache.

    fn test_serial_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
        LOCK.get_or_init(|| std::sync::Mutex::new(()))
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    #[test]
    fn parse_expr_cached_identity() {
        let _guard = test_serial_lock();
        super::expr_cache_reset_for_tests();
        let a = super::parse_expr_cached("$x + 1", None);
        let b = super::parse_expr_cached("$x + 1", None);
        assert!(
            std::sync::Arc::ptr_eq(&a, &b),
            "two cached calls with identical key should return ptr_eq Arcs",
        );
    }

    #[test]
    fn parse_expr_cached_dialect_distinct() {
        let _guard = test_serial_lock();
        super::expr_cache_reset_for_tests();
        let plain = super::parse_expr_cached("$x", None);
        let irules = super::parse_expr_cached("$x", Some("f5-irules"));
        assert!(
            !std::sync::Arc::ptr_eq(&plain, &irules),
            "different dialect should produce distinct cache entries",
        );
    }

    #[test]
    fn parse_expr_cached_whitespace_sensitive() {
        let _guard = test_serial_lock();
        super::expr_cache_reset_for_tests();
        let tight = super::parse_expr_cached("$x + 1", None);
        let loose = super::parse_expr_cached("$x  +  1", None);
        assert!(
            !std::sync::Arc::ptr_eq(&tight, &loose),
            "different source strings (even when AST-equivalent) should be distinct cache entries",
        );
    }

    /// Test eviction directly on `ExprCache` rather than through the
    /// global `parse_expr_cached` to avoid racing other tests on the
    /// process-wide cache.  The eviction logic under test is the
    /// same — `parse_expr_cached` is a thin wrapper.
    #[test]
    fn expr_cache_capacity_eviction_isolated() {
        let mut cache = super::ExprCache::new();
        let cap = super::EXPR_CACHE_CAPACITY;
        // Fill exactly to capacity.
        for i in 0..cap {
            let key = (format!("expr_seed_{i}"), "tcl");
            cache.insert(
                key,
                std::sync::Arc::new(crate::expr::ast::ExprNode::Raw {
                    text: format!("expr_seed_{i}"),
                }),
            );
        }
        assert_eq!(cache.len(), cap);
        let first_key = ("expr_seed_0".to_owned(), "tcl");
        assert!(cache.map.contains_key(&first_key));
        // One more insert evicts the front (the LRU entry).
        cache.insert(
            ("expr_seed_extra".to_owned(), "tcl"),
            std::sync::Arc::new(crate::expr::ast::ExprNode::Raw {
                text: "expr_seed_extra".to_owned(),
            }),
        );
        assert_eq!(cache.len(), cap);
        assert!(
            !cache.map.contains_key(&first_key),
            "front (LRU) entry should have been evicted",
        );
    }

    // =================================================================
    // TIP 582 `#` comments
    // =================================================================

    /// `source` with every comment overwritten by spaces of equal length.
    ///
    /// Keeps every other token at its original offset, so a parse of the result
    /// is comparable node-for-node with a parse of the original — `ExprNode`
    /// carries source offsets, so simply *deleting* the comment would shift
    /// every literal to its right and the trees could not be compared.
    fn blank_comments(source: &str) -> String {
        let mut out = source.as_bytes().to_vec();
        for t in tcl_lexer::tokenise_expr(source, None) {
            if t.kind == ExprTokenType::Comment {
                out[t.start as usize..=t.end as usize].fill(b' ');
            }
        }
        String::from_utf8(out).expect("overwriting a whole token with spaces keeps UTF-8 valid")
    }

    /// A comment is filtered out with the whitespace before the Pratt parse, so
    /// the grammar cannot see it: replacing the comment with *whitespace of the
    /// same length* yields an identical tree. That equivalence is exactly C's
    /// rule — `ParseExpr` skips a `COMMENT` lexeme the same way it skips a
    /// whitespace run (`tclCompExpr.c:701`).
    ///
    /// Before this, the expr lexer classified `#` as an unknown character, which
    /// tripped `parse_expr`'s `has_unknown` early return and degraded every one
    /// of these to `ExprNode::Raw`.
    #[test]
    fn comments_are_invisible_to_the_grammar() {
        for src in [
            // Trailing, mid-expression, and hugging an operator.
            "1 + 2 # note",
            "1 #c\n+ 2",
            "1#c\n+2",
            "$a > 1 # why\n&& $b",
            // Between a function name and its `(` — C's `:750` case, expr-62.10.
            "max# comment\n(1,2)",
            // Inside the argument list, either side of the comma (expr-62.7/8/9).
            "max(1,# comment\n2)",
            "max(1# comment\n,2)",
            "max(# comment\n1,2)",
            // A comment containing `#`, and one with no terminating newline.
            "1 + 2 # a # b",
            "[llength {a b}] # c",
            // expr-62.5: comments do not splice the tokens they separate.
            "$a#don't splice\nne#don't splice\nfalse",
        ] {
            let parsed = parse(src);
            assert!(
                !matches!(parsed, ExprNode::Raw { .. }),
                "{src:?} should parse, got Raw"
            );
            let blanked = blank_comments(src);
            assert_ne!(blanked, src, "{src:?} should contain a comment to blank");
            assert_eq!(parsed, parse(&blanked), "{src:?} vs blanked {blanked:?}");
        }
    }

    /// A comment swallows the rest of its line, operators included — so
    /// expr-62.1's `expr {1 # + 2}` parses as the bare literal `1`, not `1 + 2`.
    #[test]
    fn a_comment_swallows_the_rest_of_its_line() {
        assert_eq!(
            parse("1 # + 2"),
            ExprNode::Literal {
                text: "1".to_owned(),
                start: 0,
                end: 0
            }
        );
        // But only to the newline: the `+ 2` on the next line is still parsed,
        // which is expr-62.2's `expr "1 #\n+ 2"` == 3.
        assert!(matches!(
            parse("1 #\n+ 2"),
            ExprNode::Binary { op: BinOp::Add, .. }
        ));
    }

    /// A comment-only body has no operands at all, so it is C's *empty*
    /// expression and still falls back to `Raw` (the VM turns that into
    /// `empty expression`). The comment must not be mistaken for an operand.
    #[test]
    fn a_comment_only_expression_is_empty() {
        for src in ["# c", "  # c", "#", "# a\n# b"] {
            assert!(
                matches!(parse(src), ExprNode::Raw { .. }),
                "{src:?} should be Raw (empty expression)"
            );
        }
    }

    /// The gate: under an 8.x dialect `#` is still an invalid expression
    /// character, so the expression degrades to `Raw` exactly as before —
    /// TIP 582 is 9.0+.
    #[test]
    fn comments_stay_unparsable_before_tcl9() {
        assert!(matches!(
            parse_expr("1 + 2 # note", Some("tcl8.6")),
            ExprNode::Raw { .. }
        ));
        assert!(matches!(parse_irules("1 + 2 # note"), ExprNode::Raw { .. }));
        // 9.0 is where it starts working.
        assert!(!matches!(
            parse_expr("1 + 2 # note", Some("tcl9.0")),
            ExprNode::Raw { .. }
        ));
    }
}
