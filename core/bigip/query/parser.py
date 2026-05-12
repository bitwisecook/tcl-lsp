"""Recursive-descent parser for the query DSL.

Grammar (informal — the canonical version lives in
``docs/design/f5-query-dsl.md``)::

    program     := statement (';' statement)* EOF
    statement   := assignment | pipeline
    assignment  := path ASSIGN_OP pipeline
    pipeline    := or_expr ('|' or_expr)*
    or_expr     := and_expr ('or' and_expr)*
    and_expr    := not_expr ('and' not_expr)*
    not_expr    := 'not' not_expr | cmp_expr
    cmp_expr    := add_expr (CMP_OP add_expr)?
    add_expr    := mul_expr (('+' | '-') mul_expr)*
    mul_expr    := unary    (('*' | '/') unary)*
    unary       := '-' unary | postfix
    postfix     := primary path_step*
    primary     := literal | call | path_start | '(' pipeline ')'
    path_start  := '.' | '.' field path_step*
    path_step   := '.' field | '[' subscript ']'
    field       := IDENT | STRING
    subscript   := /* empty */ | pipeline | REGEX | NUMBER
    call        := IDENT '(' (pipeline (',' pipeline)*)? ')'

A few divergences from jq:

- Function arguments are separated by ``,`` rather than ``;`` —
  ``select(.a == "x", .b)`` is unambiguous because we keep ``,`` out of
  the expression grammar entirely.  jq's stream-comma is not in v1.
- The bare ``.`` is the identity expression (matches jq), but
  ``.foo.bar`` is a single :class:`.ast.PathExpr` node rather than a
  pipe of two field accesses.  This makes assignment ergonomic: the
  whole LHS of an ``=`` is one path.
- Regex subscripts use ``["~pattern"]`` rather than jq's
  ``test("pattern")``, so they nest naturally with the rest of the
  subscript forms.
"""

from __future__ import annotations

from .ast import (
    Assignment,
    BinOp,
    Call,
    Expr,
    Field,
    Identity,
    Literal,
    PathExpr,
    Pipe,
    Program,
    Subscript,
    UnaryOp,
)
from .errors import ParseError
from .lexer import Token, TokenKind, tokenise

_ASSIGN_OPS = {
    TokenKind.EQ: "=",
    TokenKind.PIPE_EQ: "|=",
    TokenKind.PLUS_EQ: "+=",
    TokenKind.MINUS_EQ: "-=",
}

_CMP_OPS = {
    TokenKind.EQEQ: "==",
    TokenKind.NEQ: "!=",
    TokenKind.LT: "<",
    TokenKind.LE: "<=",
    TokenKind.GT: ">",
    TokenKind.GE: ">=",
}


class _Parser:
    def __init__(self, tokens: list[Token]):
        self._tokens = tokens
        self._pos = 0

    # --- token utilities ----------------------------------------------------

    def _peek(self, offset: int = 0) -> Token:
        idx = self._pos + offset
        if idx >= len(self._tokens):
            return self._tokens[-1]
        return self._tokens[idx]

    def _consume(self) -> Token:
        tok = self._tokens[self._pos]
        if tok.kind is not TokenKind.EOF:
            self._pos += 1
        return tok

    def _expect(self, kind: TokenKind, *, msg: str | None = None) -> Token:
        tok = self._peek()
        if tok.kind is not kind:
            raise ParseError(
                msg or f"expected {kind.name}, got {tok.kind.name} ({tok.text!r})",
                tok.offset,
            )
        return self._consume()

    def _match(self, *kinds: TokenKind) -> Token | None:
        if self._peek().kind in kinds:
            return self._consume()
        return None

    # --- top-level entry point ---------------------------------------------

    def parse_program(self) -> Program:
        stmts: list[Expr] = []
        stmts.append(self._parse_pipeline())
        while self._match(TokenKind.SEMICOLON):
            # Allow a trailing ';' before EOF.
            if self._peek().kind is TokenKind.EOF:
                break
            stmts.append(self._parse_pipeline())
        self._expect(TokenKind.EOF, msg="expected end of query")
        return Program(statements=tuple(stmts))

    # --- expressions --------------------------------------------------------

    def _parse_pipeline(self) -> Expr:
        expr = self._parse_pipe_stage()
        while True:
            tok = self._peek()
            if tok.kind is TokenKind.PIPE:
                self._consume()
                rhs = self._parse_pipe_stage()
                expr = Pipe(lhs=expr, rhs=rhs, offset=tok.offset)
                continue
            break
        return expr

    def _parse_pipe_stage(self) -> Expr:
        """A single stage of a pipeline.

        Each stage is an or-expression with an optional trailing
        assignment.  This makes ``a | b |= c`` parse as
        ``a | (b |= c)`` rather than ``(a | b) |= c`` — the assignment
        binds to its direct LHS, which is what users want when streaming
        edits across many objects (``.ltm.virtual[] | .destination |= …``).
        """
        lhs = self._parse_or()
        tok = self._peek()
        if tok.kind in _ASSIGN_OPS:
            target = _as_path(lhs, tok.offset)
            self._consume()
            rhs = self._parse_pipe_stage()
            return Assignment(
                target=target, op=_ASSIGN_OPS[tok.kind], rhs=rhs, offset=tok.offset
            )
        return lhs

    def _parse_or(self) -> Expr:
        expr = self._parse_and()
        while True:
            tok = self._peek()
            if tok.kind is TokenKind.OR:
                self._consume()
                rhs = self._parse_and()
                expr = BinOp(op="or", lhs=expr, rhs=rhs, offset=tok.offset)
                continue
            break
        return expr

    def _parse_and(self) -> Expr:
        expr = self._parse_not()
        while True:
            tok = self._peek()
            if tok.kind is TokenKind.AND:
                self._consume()
                rhs = self._parse_not()
                expr = BinOp(op="and", lhs=expr, rhs=rhs, offset=tok.offset)
                continue
            break
        return expr

    def _parse_not(self) -> Expr:
        tok = self._peek()
        if tok.kind is TokenKind.NOT:
            self._consume()
            operand = self._parse_not()
            return UnaryOp(op="not", operand=operand, offset=tok.offset)
        return self._parse_cmp()

    def _parse_cmp(self) -> Expr:
        expr = self._parse_add()
        tok = self._peek()
        if tok.kind in _CMP_OPS:
            self._consume()
            rhs = self._parse_add()
            return BinOp(op=_CMP_OPS[tok.kind], lhs=expr, rhs=rhs, offset=tok.offset)
        return expr

    def _parse_add(self) -> Expr:
        expr = self._parse_mul()
        while True:
            tok = self._peek()
            if tok.kind in (TokenKind.PLUS, TokenKind.MINUS):
                self._consume()
                rhs = self._parse_mul()
                op = "+" if tok.kind is TokenKind.PLUS else "-"
                expr = BinOp(op=op, lhs=expr, rhs=rhs, offset=tok.offset)
                continue
            break
        return expr

    def _parse_mul(self) -> Expr:
        expr = self._parse_unary()
        while True:
            tok = self._peek()
            if tok.kind in (TokenKind.STAR, TokenKind.SLASH):
                self._consume()
                rhs = self._parse_unary()
                op = "*" if tok.kind is TokenKind.STAR else "/"
                expr = BinOp(op=op, lhs=expr, rhs=rhs, offset=tok.offset)
                continue
            break
        return expr

    def _parse_unary(self) -> Expr:
        tok = self._peek()
        if tok.kind is TokenKind.MINUS:
            self._consume()
            operand = self._parse_unary()
            return UnaryOp(op="-", operand=operand, offset=tok.offset)
        return self._parse_primary_with_postfix()

    def _parse_primary_with_postfix(self) -> Expr:
        primary = self._parse_primary()
        # ``foo(...)[...]`` is not in the grammar; subscripts only chain
        # after a path expression.  This keeps the AST clean — the
        # special PathExpr handling only has to consider field/subscript
        # steps emitted by ``_parse_path_starting_with_dot``.
        return primary

    def _parse_primary(self) -> Expr:
        tok = self._peek()

        if tok.kind in (TokenKind.NUMBER, TokenKind.STRING, TokenKind.TRUE, TokenKind.FALSE, TokenKind.NULL):
            self._consume()
            return Literal(value=tok.value, offset=tok.offset)

        if tok.kind is TokenKind.LPAREN:
            self._consume()
            inner = self._parse_pipeline()
            self._expect(TokenKind.RPAREN, msg="expected ')'")
            return inner

        if tok.kind is TokenKind.IDENT:
            # Bare identifier is always a call — there are no
            # user-defined variables in v1.  A function with zero
            # arguments still uses the ``f()`` form.
            self._consume()
            args: list[Expr] = []
            if self._peek().kind is TokenKind.LPAREN:
                self._consume()
                if self._peek().kind is not TokenKind.RPAREN:
                    args.append(self._parse_pipeline())
                    while self._match(TokenKind.COMMA):
                        args.append(self._parse_pipeline())
                self._expect(TokenKind.RPAREN, msg="expected ')' to close call")
            return Call(name=tok.text, args=tuple(args), offset=tok.offset)

        if tok.kind is TokenKind.DOT:
            return self._parse_path_starting_with_dot()

        raise ParseError(f"unexpected token {tok.text!r}", tok.offset)

    # --- path expressions --------------------------------------------------

    def _parse_path_starting_with_dot(self) -> Expr:
        dot = self._consume()  # the leading '.'
        # Bare '.' (identity) when the next token cannot continue a path.
        nxt = self._peek()
        if nxt.kind not in (TokenKind.IDENT, TokenKind.STRING, TokenKind.LBRACKET):
            return Identity(offset=dot.offset)

        steps: list = []
        # First step.
        if nxt.kind in (TokenKind.IDENT, TokenKind.STRING):
            steps.append(self._parse_field_step())
        elif nxt.kind is TokenKind.LBRACKET:
            steps.append(self._parse_subscript_step())
        # Subsequent steps.
        while True:
            t = self._peek()
            if t.kind is TokenKind.DOT and self._peek(1).kind in (
                TokenKind.IDENT,
                TokenKind.STRING,
            ):
                self._consume()  # '.'
                steps.append(self._parse_field_step())
                continue
            if t.kind is TokenKind.LBRACKET:
                steps.append(self._parse_subscript_step())
                continue
            break
        return PathExpr(steps=tuple(steps), offset=dot.offset)

    def _parse_field_step(self) -> Field:
        tok = self._consume()
        if tok.kind is TokenKind.IDENT:
            name = tok.text
        elif tok.kind is TokenKind.STRING:
            name = str(tok.value)
        else:  # pragma: no cover - unreachable
            raise ParseError("expected field name", tok.offset)
        return Field(name=name, optional=False, offset=tok.offset)

    def _parse_subscript_step(self) -> Subscript:
        lb = self._expect(TokenKind.LBRACKET, msg="expected '['")
        nxt = self._peek()
        if nxt.kind is TokenKind.RBRACKET:
            self._consume()
            return Subscript(stream=True, index=None, regex=None, offset=lb.offset)
        if nxt.kind is TokenKind.REGEX:
            self._consume()
            self._expect(TokenKind.RBRACKET, msg="expected ']' after regex subscript")
            return Subscript(stream=False, index=None, regex=str(nxt.value), offset=lb.offset)
        inner = self._parse_pipeline()
        self._expect(TokenKind.RBRACKET, msg="expected ']'")
        return Subscript(stream=False, index=inner, regex=None, offset=lb.offset)


def _as_path(expr: Expr, op_offset: int) -> PathExpr:
    if isinstance(expr, PathExpr):
        return expr
    if isinstance(expr, Identity):
        return PathExpr(steps=(), offset=expr.offset)
    raise ParseError(
        "left-hand side of an assignment must be a path expression "
        "(starting with '.')",
        op_offset,
    )


def parse_query(source: str) -> Program:
    """Tokenise and parse *source* into a :class:`.ast.Program`."""
    tokens = tokenise(source)
    return _Parser(tokens).parse_program()
