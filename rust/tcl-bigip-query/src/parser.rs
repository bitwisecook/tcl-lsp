//! Recursive-descent parser for the query DSL.
//!
//! Grammar (informal — the canonical version lives in
//! `docs/references/f5_query/dsl.md`):
//!
//! ```text
//! program     := pipeline (';' pipeline)* EOF
//! pipeline    := pipe_stage ('|' pipe_stage)*
//! pipe_stage  := or_expr (ASSIGN_OP pipe_stage)?
//! ASSIGN_OP   := '=' | '|=' | '+=' | '-='
//! or_expr     := and_expr ('or' and_expr)*
//! and_expr    := not_expr ('and' not_expr)*
//! not_expr    := 'not' not_expr | cmp_expr
//! cmp_expr    := add_expr (CMP_OP add_expr)?
//! add_expr    := mul_expr (('+' | '-') mul_expr)*
//! mul_expr    := unary    (('*' | '/') unary)*
//! unary       := '-' unary | primary
//! primary     := literal | call | path_start | '(' pipeline ')'
//! ```
//!
//! The divergences from jq are documented in the parser overview (comma-
//! separated call args, assignment as a trailing pipe-stage operator,
//! single `PathExpr` nodes, and `["~pattern"]` regex subscripts).

use crate::ast::{Expr, LitValue, PathStep, Program};
use crate::errors::QueryError;
use crate::lexer::{Token, TokenKind, py_repr_str, tokenise};

fn assign_op(kind: TokenKind) -> Option<&'static str> {
    match kind {
        TokenKind::Eq => Some("="),
        TokenKind::PipeEq => Some("|="),
        TokenKind::PlusEq => Some("+="),
        TokenKind::MinusEq => Some("-="),
        _ => None,
    }
}

fn cmp_op(kind: TokenKind) -> Option<&'static str> {
    match kind {
        TokenKind::EqEq => Some("=="),
        TokenKind::Neq => Some("!="),
        TokenKind::Lt => Some("<"),
        TokenKind::Le => Some("<="),
        TokenKind::Gt => Some(">"),
        TokenKind::Ge => Some(">="),
        _ => None,
    }
}

/// String coercion of a value for the few token-value reads the parser
/// performs (always a `Str` in practice — `IDENT` / `$IDENT` / `STRING`).
fn lit_str(v: &LitValue) -> String {
    match v {
        LitValue::Str(s) => s.clone(),
        LitValue::Null => "None".to_string(),
        LitValue::Bool(true) => "True".to_string(),
        LitValue::Bool(false) => "False".to_string(),
        LitValue::Int(i) => i.to_string(),
        LitValue::Float(f) => f.to_string(),
    }
}

/// Recursion-depth ceiling for the parser. Each nested grouping (`(`, `[`,
/// `{`, `if`) and each chained unary/`not` prefix descends one native stack
/// frame; an adversarial query string like `((((…))))` or `- - - …` nests
/// without bound.
///
/// The recursive-descent chain spends a dozen-odd frames per nesting level,
/// so this is sized empirically, not optimistically: a debug build overflows
/// a small 2 MiB worker stack near ~100 levels of grouping. 64 sits safely
/// below that on the tightest stack we run on yet is several times deeper
/// than any hand-written or generated query, so the budget rejects the
/// attack without constraining real input. (The evaluator's
/// [`MAX_EVAL_DEPTH`](crate::eval) is set above this so every tree the parser
/// accepts still evaluates.)
const MAX_PARSE_DEPTH: usize = 64;

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    /// Current nesting depth, bumped at each recursive grouping / prefix
    /// entry and checked against [`MAX_PARSE_DEPTH`] to bound stack use.
    depth: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            pos: 0,
            depth: 0,
        }
    }

    /// Enter one nesting level, failing with a parse error at the cap.
    ///
    /// Callers pair this with [`Self::leave`] so a deeply-nested query
    /// returns [`QueryError::Parse`] instead of overflowing the stack.
    fn enter(&mut self, offset: usize) -> Result<(), QueryError> {
        self.depth += 1;
        if self.depth > MAX_PARSE_DEPTH {
            return Err(QueryError::Parse {
                message: "query nests too deeply".to_string(),
                offset,
            });
        }
        Ok(())
    }

    fn leave(&mut self) {
        self.depth -= 1;
    }

    // --- token utilities ---------------------------------------------------

    fn peek(&self, offset: usize) -> Token {
        let idx = self.pos + offset;
        if idx >= self.tokens.len() {
            return self.tokens[self.tokens.len() - 1].clone();
        }
        self.tokens[idx].clone()
    }

    fn consume(&mut self) -> Token {
        let tok = self.tokens[self.pos].clone();
        if tok.kind != TokenKind::Eof {
            self.pos += 1;
        }
        tok
    }

    fn expect(&mut self, kind: TokenKind, msg: &str) -> Result<Token, QueryError> {
        let tok = self.peek(0);
        if tok.kind != kind {
            return Err(QueryError::Parse {
                message: msg.to_string(),
                offset: tok.offset,
            });
        }
        Ok(self.consume())
    }

    fn match_kind(&mut self, kind: TokenKind) -> Option<Token> {
        if self.peek(0).kind == kind {
            Some(self.consume())
        } else {
            None
        }
    }

    // --- top-level entry point ---------------------------------------------

    fn parse_program(&mut self) -> Result<Program, QueryError> {
        let mut stmts: Vec<Expr> = vec![self.parse_pipeline()?];
        while self.match_kind(TokenKind::Semicolon).is_some() {
            // Allow a trailing ';' before EOF.
            if self.peek(0).kind == TokenKind::Eof {
                break;
            }
            stmts.push(self.parse_pipeline()?);
        }
        self.expect(TokenKind::Eof, "expected end of query")?;
        Ok(Program { statements: stmts })
    }

    // --- expressions -------------------------------------------------------

    fn parse_pipeline(&mut self) -> Result<Expr, QueryError> {
        let mut expr = self.parse_comma_chain()?;
        loop {
            let tok = self.peek(0);
            if tok.kind == TokenKind::Pipe {
                self.consume();
                let rhs = self.parse_comma_chain()?;
                expr = Expr::Pipe {
                    lhs: Box::new(expr),
                    rhs: Box::new(rhs),
                    offset: tok.offset,
                };
                continue;
            }
            break;
        }
        Ok(expr)
    }

    fn parse_comma_chain(&mut self) -> Result<Expr, QueryError> {
        let first = self.parse_pipe_stage()?;
        if self.peek(0).kind != TokenKind::Comma {
            return Ok(first);
        }
        let offset = first.offset();
        let mut parts: Vec<Expr> = vec![first];
        while self.peek(0).kind == TokenKind::Comma {
            self.consume();
            parts.push(self.parse_pipe_stage()?);
        }
        Ok(Expr::CommaStream { parts, offset })
    }

    fn parse_pipeline_no_comma(&mut self) -> Result<Expr, QueryError> {
        let mut expr = self.parse_pipe_stage()?;
        loop {
            let tok = self.peek(0);
            if tok.kind == TokenKind::Pipe {
                self.consume();
                let rhs = self.parse_pipe_stage()?;
                expr = Expr::Pipe {
                    lhs: Box::new(expr),
                    rhs: Box::new(rhs),
                    offset: tok.offset,
                };
                continue;
            }
            break;
        }
        Ok(expr)
    }

    fn parse_pipe_stage(&mut self) -> Result<Expr, QueryError> {
        let lhs = self.parse_or()?;
        let tok = self.peek(0);
        if let Some(op) = assign_op(tok.kind) {
            let (target, source) = as_path(lhs, tok.offset)?;
            self.consume();
            let rhs = self.parse_pipe_stage()?;
            return Ok(Expr::Assignment {
                target: Box::new(target),
                op: op.to_string(),
                rhs: Box::new(rhs),
                offset: tok.offset,
                source: source.map(Box::new),
            });
        }
        if tok.kind == TokenKind::As {
            // ``expr as $name | body`` — let-binding (right-associative).
            self.consume();
            let name_tok = self.peek(0);
            if name_tok.kind != TokenKind::DollarIdent {
                return Err(QueryError::Parse {
                    message: format!(
                        "'as' must be followed by a $-prefixed identifier \
                         (e.g. ``as $vs``), got {}",
                        py_repr_str(&name_tok.text)
                    ),
                    offset: name_tok.offset,
                });
            }
            self.consume();
            let pipe_tok = self.peek(0);
            if pipe_tok.kind != TokenKind::Pipe {
                return Err(QueryError::Parse {
                    message: format!(
                        "'as $name' must be immediately followed by '|' and \
                         the body expression, got {}",
                        py_repr_str(&pipe_tok.text)
                    ),
                    offset: pipe_tok.offset,
                });
            }
            self.consume();
            let body = self.parse_pipeline()?;
            return Ok(Expr::LetBinding {
                source: Box::new(lhs),
                name: lit_str(&name_tok.value),
                body: Box::new(body),
                offset: tok.offset,
            });
        }
        Ok(lhs)
    }

    fn parse_or(&mut self) -> Result<Expr, QueryError> {
        let mut expr = self.parse_and()?;
        loop {
            let tok = self.peek(0);
            if tok.kind == TokenKind::Or {
                self.consume();
                let rhs = self.parse_and()?;
                expr = Expr::BinOp {
                    op: "or".to_string(),
                    lhs: Box::new(expr),
                    rhs: Box::new(rhs),
                    offset: tok.offset,
                };
                continue;
            }
            break;
        }
        Ok(expr)
    }

    fn parse_and(&mut self) -> Result<Expr, QueryError> {
        let mut expr = self.parse_not()?;
        loop {
            let tok = self.peek(0);
            if tok.kind == TokenKind::And {
                self.consume();
                let rhs = self.parse_not()?;
                expr = Expr::BinOp {
                    op: "and".to_string(),
                    lhs: Box::new(expr),
                    rhs: Box::new(rhs),
                    offset: tok.offset,
                };
                continue;
            }
            break;
        }
        Ok(expr)
    }

    fn parse_not(&mut self) -> Result<Expr, QueryError> {
        let tok = self.peek(0);
        if tok.kind == TokenKind::Not {
            // jq exposes ``not`` as a unary prefix and a postfix filter.
            // When the token after ``not`` can't start an operand, fall
            // through to the primary-level zero-arg ``Call("not")`` so
            // ``x | not`` works and trailing arithmetic splices on top.
            if !token_starts_operand(self.peek(1).kind) {
                return self.parse_cmp();
            }
            self.consume();
            // ``not not not …`` self-recurses one frame per prefix; charge
            // the depth budget so a long run returns an error, not a crash.
            self.enter(tok.offset)?;
            let operand = self.parse_not();
            self.leave();
            return Ok(Expr::UnaryOp {
                op: "not".to_string(),
                operand: Box::new(operand?),
                offset: tok.offset,
            });
        }
        self.parse_cmp()
    }

    fn parse_cmp(&mut self) -> Result<Expr, QueryError> {
        let expr = self.parse_add()?;
        let tok = self.peek(0);
        if let Some(op) = cmp_op(tok.kind) {
            self.consume();
            let rhs = self.parse_add()?;
            return Ok(Expr::BinOp {
                op: op.to_string(),
                lhs: Box::new(expr),
                rhs: Box::new(rhs),
                offset: tok.offset,
            });
        }
        Ok(expr)
    }

    fn parse_add(&mut self) -> Result<Expr, QueryError> {
        let mut expr = self.parse_mul()?;
        loop {
            let tok = self.peek(0);
            if tok.kind == TokenKind::Plus || tok.kind == TokenKind::Minus {
                self.consume();
                let rhs = self.parse_mul()?;
                let op = if tok.kind == TokenKind::Plus {
                    "+"
                } else {
                    "-"
                };
                expr = Expr::BinOp {
                    op: op.to_string(),
                    lhs: Box::new(expr),
                    rhs: Box::new(rhs),
                    offset: tok.offset,
                };
                continue;
            }
            break;
        }
        Ok(expr)
    }

    fn parse_mul(&mut self) -> Result<Expr, QueryError> {
        let mut expr = self.parse_unary()?;
        loop {
            let tok = self.peek(0);
            if tok.kind == TokenKind::Star || tok.kind == TokenKind::Slash {
                self.consume();
                let rhs = self.parse_unary()?;
                let op = if tok.kind == TokenKind::Star {
                    "*"
                } else {
                    "/"
                };
                expr = Expr::BinOp {
                    op: op.to_string(),
                    lhs: Box::new(expr),
                    rhs: Box::new(rhs),
                    offset: tok.offset,
                };
                continue;
            }
            break;
        }
        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr, QueryError> {
        let tok = self.peek(0);
        if tok.kind == TokenKind::Minus {
            self.consume();
            // ``- - - …`` self-recurses one frame per prefix; charge the
            // depth budget so a long run returns an error, not a crash.
            self.enter(tok.offset)?;
            let operand = self.parse_unary();
            self.leave();
            return Ok(Expr::UnaryOp {
                op: "-".to_string(),
                operand: Box::new(operand?),
                offset: tok.offset,
            });
        }
        self.parse_primary_with_postfix()
    }

    fn parse_primary_with_postfix(&mut self) -> Result<Expr, QueryError> {
        let primary = self.parse_primary()?;
        let mut steps: Vec<PathStep> = Vec::new();
        loop {
            let tok = self.peek(0);
            if tok.kind == TokenKind::Dot
                && matches!(self.peek(1).kind, TokenKind::Ident | TokenKind::String)
            {
                self.consume(); // '.'
                steps.push(self.parse_field_step()?);
                continue;
            }
            if tok.kind == TokenKind::LBracket {
                steps.push(self.parse_subscript_step()?);
                continue;
            }
            break;
        }
        if steps.is_empty() {
            return Ok(primary);
        }
        let offset = primary.offset();
        // Identity primary: the steps walk from ``.`` directly. Every
        // other primary becomes the ``head`` of the path so the evaluator
        // can preserve the outer current for subscript expressions.
        if matches!(primary, Expr::Identity { .. }) {
            return Ok(Expr::PathExpr {
                steps,
                offset,
                head: None,
            });
        }
        Ok(Expr::PathExpr {
            steps,
            offset,
            head: Some(Box::new(primary)),
        })
    }

    fn parse_primary(&mut self) -> Result<Expr, QueryError> {
        // Every nesting level reaches its operand through exactly one
        // `parse_primary` call, so charging the depth budget here bounds the
        // whole recursive descent (grouping primaries below re-enter
        // `parse_pipeline`) with a single guard.
        let tok = self.peek(0);
        self.enter(tok.offset)?;
        let result = self.parse_primary_inner(&tok);
        self.leave();
        result
    }

    fn parse_primary_inner(&mut self, tok: &Token) -> Result<Expr, QueryError> {
        match tok.kind {
            TokenKind::Not => {
                // Postfix ``not`` (jq's ``x | not``) reaches us via
                // ``parse_not``'s no-operand-follows branch. Consume it
                // here and emit a zero-arg Call.
                self.consume();
                Ok(Expr::Call {
                    name: "not".to_string(),
                    args: Vec::new(),
                    offset: tok.offset,
                })
            }
            TokenKind::Number
            | TokenKind::String
            | TokenKind::True
            | TokenKind::False
            | TokenKind::Null => {
                self.consume();
                Ok(Expr::Literal {
                    value: tok.value.clone(),
                    offset: tok.offset,
                })
            }
            TokenKind::LParen => {
                self.consume();
                let inner = self.parse_pipeline()?;
                self.expect(TokenKind::RParen, "expected ')'")?;
                Ok(inner)
            }
            TokenKind::LBracket => {
                // ``[ expr ]`` collects the stream of values produced by
                // *expr* into a single list; ``[]`` is the empty list.
                self.consume();
                if self.peek(0).kind == TokenKind::RBracket {
                    self.consume();
                    return Ok(Expr::ListLiteral {
                        inner: None,
                        offset: tok.offset,
                    });
                }
                let inner_expr = self.parse_pipeline()?;
                self.expect(TokenKind::RBracket, "expected ']' to close list literal")?;
                Ok(Expr::ListLiteral {
                    inner: Some(Box::new(inner_expr)),
                    offset: tok.offset,
                })
            }
            TokenKind::Ident => {
                // Bare identifier is a call. ``f()`` (empty args) is
                // accepted; a bare ``length`` is ``length(.)`` at eval.
                self.consume();
                let mut args: Vec<Expr> = Vec::new();
                if self.peek(0).kind == TokenKind::LParen {
                    self.consume();
                    if self.peek(0).kind != TokenKind::RParen {
                        // Inside call args the comma is a separator, not the
                        // stream-concat operator — use the no-comma form.
                        args.push(self.parse_pipeline_no_comma()?);
                        while self.match_kind(TokenKind::Comma).is_some() {
                            args.push(self.parse_pipeline_no_comma()?);
                        }
                    }
                    self.expect(TokenKind::RParen, "expected ')' to close call")?;
                }
                Ok(Expr::Call {
                    name: tok.text.clone(),
                    args,
                    offset: tok.offset,
                })
            }
            TokenKind::Dot => self.parse_path_starting_with_dot(),
            TokenKind::LBrace => self.parse_object_literal(),
            TokenKind::DollarIdent => {
                self.consume();
                Ok(Expr::Variable {
                    name: lit_str(&tok.value),
                    offset: tok.offset,
                })
            }
            TokenKind::If => self.parse_if_then_else(),
            _ => Err(QueryError::Parse {
                message: format!("unexpected token {}", py_repr_str(&tok.text)),
                offset: tok.offset,
            }),
        }
    }

    fn parse_if_then_else(&mut self) -> Result<Expr, QueryError> {
        let if_tok = self.expect(TokenKind::If, "expected 'if'")?;
        let cond = self.parse_pipeline()?;
        self.expect(TokenKind::Then, "expected 'then' after if-condition")?;
        let then_body = self.parse_pipeline()?;
        let mut elifs: Vec<(Expr, Expr)> = Vec::new();
        while self.peek(0).kind == TokenKind::Elif {
            self.consume();
            let elif_cond = self.parse_pipeline()?;
            self.expect(TokenKind::Then, "expected 'then' after elif-condition")?;
            let elif_body = self.parse_pipeline()?;
            elifs.push((elif_cond, elif_body));
        }
        let mut else_body: Option<Box<Expr>> = None;
        if self.peek(0).kind == TokenKind::Else {
            self.consume();
            else_body = Some(Box::new(self.parse_pipeline()?));
        }
        self.expect(TokenKind::End, "expected 'end' to close if-expression")?;
        Ok(Expr::IfThenElse {
            cond: Box::new(cond),
            then_body: Box::new(then_body),
            elifs,
            else_body,
            offset: if_tok.offset,
        })
    }

    // --- path expressions --------------------------------------------------

    fn parse_path_starting_with_dot(&mut self) -> Result<Expr, QueryError> {
        let dot = self.consume(); // the leading '.'
        let nxt = self.peek(0);
        // Bare '.' (identity) when the next token cannot continue a path.
        if !matches!(
            nxt.kind,
            TokenKind::Ident | TokenKind::String | TokenKind::LBracket
        ) {
            return Ok(Expr::Identity { offset: dot.offset });
        }

        let mut steps: Vec<PathStep> = Vec::new();
        // First step.
        if matches!(nxt.kind, TokenKind::Ident | TokenKind::String) {
            steps.push(self.parse_field_step()?);
        } else if nxt.kind == TokenKind::LBracket {
            steps.push(self.parse_subscript_step()?);
        }
        // Subsequent steps.
        loop {
            let t = self.peek(0);
            if t.kind == TokenKind::Dot
                && matches!(self.peek(1).kind, TokenKind::Ident | TokenKind::String)
            {
                self.consume(); // '.'
                steps.push(self.parse_field_step()?);
                continue;
            }
            if t.kind == TokenKind::LBracket {
                steps.push(self.parse_subscript_step()?);
                continue;
            }
            break;
        }
        Ok(Expr::PathExpr {
            steps,
            offset: dot.offset,
            head: None,
        })
    }

    fn parse_field_step(&mut self) -> Result<PathStep, QueryError> {
        let tok = self.consume();
        let name = match tok.kind {
            TokenKind::Ident => tok.text.clone(),
            TokenKind::String => lit_str(&tok.value),
            _ => {
                return Err(QueryError::Parse {
                    message: "expected field name".to_string(),
                    offset: tok.offset,
                });
            }
        };
        let optional = self.consume_optional_marker();
        Ok(PathStep::Field {
            name,
            optional,
            offset: tok.offset,
        })
    }

    fn parse_subscript_step(&mut self) -> Result<PathStep, QueryError> {
        let lb = self.expect(TokenKind::LBracket, "expected '['")?;
        let nxt = self.peek(0);
        if nxt.kind == TokenKind::RBracket {
            self.consume();
            let optional = self.consume_optional_marker();
            return Ok(PathStep::Subscript {
                stream: true,
                index: None,
                regex: None,
                offset: lb.offset,
                optional,
            });
        }
        // A bare string subscript whose contents start with ``~`` is a
        // regex subscript. Recognised here (not in the lexer) so a string
        // starting with ``~`` in any other position stays just a string.
        if nxt.kind == TokenKind::String
            && let LitValue::Str(s) = &nxt.value
            && s.starts_with('~')
            && self.peek(1).kind == TokenKind::RBracket
        {
            self.consume();
            self.expect(TokenKind::RBracket, "expected ']' after regex subscript")?;
            let optional = self.consume_optional_marker();
            return Ok(PathStep::Subscript {
                stream: false,
                index: None,
                regex: Some(s.chars().skip(1).collect()),
                offset: lb.offset,
                optional,
            });
        }
        let inner = self.parse_pipeline()?;
        self.expect(TokenKind::RBracket, "expected ']'")?;
        let optional = self.consume_optional_marker();
        Ok(PathStep::Subscript {
            stream: false,
            index: Some(Box::new(inner)),
            regex: None,
            offset: lb.offset,
            optional,
        })
    }

    fn consume_optional_marker(&mut self) -> bool {
        if self.peek(0).kind == TokenKind::Question {
            self.consume();
            true
        } else {
            false
        }
    }

    fn parse_object_literal(&mut self) -> Result<Expr, QueryError> {
        let lb = self.consume(); // consume '{'
        let mut entries: Vec<(String, Expr)> = Vec::new();
        if self.peek(0).kind == TokenKind::RBrace {
            self.consume();
            return Ok(Expr::ObjectLiteral {
                entries,
                offset: lb.offset,
            });
        }
        loop {
            let key_tok = self.peek(0);
            let key = match key_tok.kind {
                TokenKind::Ident => {
                    self.consume();
                    key_tok.text.clone()
                }
                TokenKind::String => {
                    self.consume();
                    lit_str(&key_tok.value)
                }
                _ => {
                    return Err(QueryError::Parse {
                        message: format!(
                            "expected an object-literal key (identifier or string) but got {}",
                            py_repr_str(&key_tok.text)
                        ),
                        offset: key_tok.offset,
                    });
                }
            };
            // Optional ``: expression`` — when omitted, desugar to a
            // ``.<key>`` path so ``{name, destination}`` works.
            let value: Expr = if self.peek(0).kind == TokenKind::Colon {
                self.consume();
                self.parse_pipe_stage()?
            } else {
                Expr::PathExpr {
                    steps: vec![PathStep::Field {
                        name: key.clone(),
                        optional: false,
                        offset: key_tok.offset,
                    }],
                    offset: key_tok.offset,
                    head: None,
                }
            };
            entries.push((key, value));
            if self.peek(0).kind == TokenKind::Comma {
                self.consume();
                continue;
            }
            break;
        }
        self.expect(TokenKind::RBrace, "expected '}' to close object literal")?;
        Ok(Expr::ObjectLiteral {
            entries,
            offset: lb.offset,
        })
    }
}

/// Return `(path, source)` for an assignment LHS.
///
/// `source` is set when the LHS was `$name.path` — the parser produces that
/// as `Pipe(Variable, PathExpr)` via the primary-with-postfix machinery,
/// and the evaluator needs to know which named root to evaluate the path
/// against. The common case (`.path`) returns `(path, None)`.
fn as_path(expr: Expr, op_offset: usize) -> Result<(Expr, Option<Expr>), QueryError> {
    match expr {
        path @ Expr::PathExpr { .. } => Ok((path, None)),
        Expr::Identity { offset } => Ok((
            Expr::PathExpr {
                steps: Vec::new(),
                offset,
                head: None,
            },
            None,
        )),
        Expr::Pipe { lhs, rhs, .. } if matches!(*lhs, Expr::Variable { .. }) => match *rhs {
            path @ Expr::PathExpr { .. } => Ok((path, Some(*lhs))),
            Expr::Identity { offset } => Ok((
                Expr::PathExpr {
                    steps: Vec::new(),
                    offset,
                    head: None,
                },
                Some(*lhs),
            )),
            _ => Err(as_path_error(op_offset)),
        },
        _ => Err(as_path_error(op_offset)),
    }
}

fn as_path_error(op_offset: usize) -> QueryError {
    QueryError::Parse {
        message: "left-hand side of an assignment must be a path expression \
                  (starting with '.' or '$name')"
            .to_string(),
        offset: op_offset,
    }
}

/// Conservatively guess whether a token of *kind* could begin an operand.
///
/// Used to disambiguate jq's postfix `not` from this DSL's unary prefix
/// `not`. Arithmetic operators that *could* be unary (`+`/`-`) are treated
/// as non-operand-starting so `1 | not - 1` parses as `(1 | not) - 1`.
fn token_starts_operand(kind: TokenKind) -> bool {
    if matches!(
        kind,
        TokenKind::Pipe
            | TokenKind::Comma
            | TokenKind::RParen
            | TokenKind::RBracket
            | TokenKind::RBrace
            | TokenKind::Semicolon
            | TokenKind::Eof
            | TokenKind::And
            | TokenKind::Or
            | TokenKind::Then
            | TokenKind::Elif
            | TokenKind::Else
            | TokenKind::End
            | TokenKind::As
    ) {
        return false;
    }
    if cmp_op(kind).is_some() || assign_op(kind).is_some() {
        return false;
    }
    if matches!(
        kind,
        TokenKind::Plus | TokenKind::Minus | TokenKind::Star | TokenKind::Slash
    ) {
        return false;
    }
    true
}

/// Tokenise and parse *source* into a [`Program`].
///
/// # Errors
///
/// Returns [`QueryError::Lex`] from the tokeniser or [`QueryError::Parse`]
/// when the token stream does not match the grammar.
pub fn parse_query(source: &str) -> Result<Program, QueryError> {
    let tokens = tokenise(source)?;
    Parser::new(tokens).parse_program()
}
