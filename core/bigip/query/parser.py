"""Recursive-descent parser for the query DSL.

Grammar (informal — the canonical version lives in
``docs/design/f5-query-dsl.md``)::

    program     := pipeline (';' pipeline)* EOF
    pipeline    := pipe_stage ('|' pipe_stage)*
    pipe_stage  := or_expr (ASSIGN_OP pipe_stage)?
    ASSIGN_OP   := '=' | '|=' | '+=' | '-='
    or_expr     := and_expr ('or' and_expr)*
    and_expr    := not_expr ('and' not_expr)*
    not_expr    := 'not' not_expr | cmp_expr
    cmp_expr    := add_expr (CMP_OP add_expr)?
    add_expr    := mul_expr (('+' | '-') mul_expr)*
    mul_expr    := unary    (('*' | '/') unary)*
    unary       := '-' unary | primary
    primary     := literal | call | path_start | '(' pipeline ')'
    path_start  := '.' | '.' field path_step* | '.' subscript path_step*
    path_step   := '.' field | subscript
    field       := IDENT | STRING
    subscript   := '[' (']' | STRING-with-leading-'~' ']' | pipeline ']')
    call        := IDENT '(' (pipeline (',' pipeline)*)? ')'

A few divergences from jq:

- Function arguments are separated by ``,`` rather than ``;`` —
  ``select(.a == "x", .b)`` is unambiguous because we keep ``,`` out of
  the expression grammar entirely.  jq's stream-comma is not in v1.
- Assignment is parsed as a trailing operator on a pipe-stage rather
  than as a top-level statement, so ``a | b |= c`` parses as
  ``a | (b |= c)``.  That binds ``|=`` to its direct LHS, which is
  what users want when streaming edits across many objects
  (``.ltm.virtual[] | .destination |= ...``).
- The bare ``.`` is the identity expression (matches jq), but
  ``.foo.bar`` is a single :class:`.ast.PathExpr` node rather than a
  pipe of two field accesses.  This makes assignment ergonomic: the
  whole LHS of an ``=`` is one path.
- Regex subscripts use ``["~pattern"]`` (a STRING literal whose
  contents start with ``~``) rather than jq's ``test("pattern")``,
  so they nest naturally with the rest of the subscript forms.  The
  ``~``-prefix is only treated specially inside ``[ ... ]``; in any
  other position the string is just a string.
"""

from __future__ import annotations

from .ast import (
    Assignment,
    BinOp,
    Call,
    CommaStream,
    Expr,
    Field,
    Identity,
    IfThenElse,
    LetBinding,
    ListLiteral,
    Literal,
    ObjectLiteral,
    PathExpr,
    Pipe,
    Program,
    Subscript,
    UnaryOp,
    Variable,
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
        """Parse a full pipeline, including the ``,`` stream-comma operator.

        Precedence: ``|`` is the lowest precedence, ``,`` is the
        second-lowest — matching jq.  So ``a, b | c`` parses as
        ``(a, b) | c``: the comma chain on the LHS feeds two values
        through the pipe to ``c``, and each value's transform is
        concatenated into the output stream.

        Comma is *not* allowed inside contexts where the comma is a
        separator (call arguments, object-literal entries) — those
        callers use :meth:`_parse_pipeline_no_comma` so the existing
        spelling ``f(a, b)`` and ``{a: 1, b: 2}`` keep working.
        """
        expr = self._parse_comma_chain()
        while True:
            tok = self._peek()
            if tok.kind is TokenKind.PIPE:
                self._consume()
                rhs = self._parse_comma_chain()
                expr = Pipe(lhs=expr, rhs=rhs, offset=tok.offset)
                continue
            break
        return expr

    def _parse_comma_chain(self) -> Expr:
        """``pipe_stage (',' pipe_stage)*`` — jq's stream concatenation.

        Comma combines streams, so the AST keeps it as a dedicated
        node and lets the evaluator concatenate each part's output
        against the same input.
        """
        first = self._parse_pipe_stage()
        parts: list[Expr] = []
        while self._peek().kind is TokenKind.COMMA:
            if not parts:
                parts.append(first)
            self._consume()
            parts.append(self._parse_pipe_stage())
        if not parts:
            return first
        return CommaStream(
            parts=tuple(parts), offset=first.offset if hasattr(first, "offset") else 0
        )

    def _parse_pipeline_no_comma(self) -> Expr:
        """Pipeline form that excludes the stream-comma operator.

        Used in contexts where ``,`` is a structural separator: call
        arguments (``f(a, b)``), object-literal entries (``{x: 1, y:
        2}``).  Allows pipes (``f(.x | basename(.))``) but never
        commas.
        """
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
            target, source = _as_path(lhs, tok.offset)
            self._consume()
            rhs = self._parse_pipe_stage()
            return Assignment(
                target=target,
                op=_ASSIGN_OPS[tok.kind],
                rhs=rhs,
                offset=tok.offset,
                source=source,
            )
        if tok.kind is TokenKind.AS:
            # ``expr as $name | body`` — let-binding.  The body
            # consumes every subsequent ``|`` stage so the binding
            # stays in scope across nested streams, matching jq's
            # right-associative ``as`` semantics:
            #
            #   .ltm.virtual[] as $vs | $vs.pool.members[] | $vs.name + ...
            #
            # Reads as: bind $vs to each virtual, then evaluate
            # ``$vs.pool.members[] | $vs.name + ...`` with that
            # binding in scope.  The right-hand side absorbs through
            # to the next statement boundary (``;`` / EOF / ``)`` etc.)
            # so the body sees the entire downstream pipeline.
            self._consume()
            name_tok = self._peek()
            if name_tok.kind is not TokenKind.DOLLAR_IDENT:
                raise ParseError(
                    "'as' must be followed by a $-prefixed identifier "
                    f"(e.g. ``as $vs``), got {name_tok.text!r}",
                    name_tok.offset,
                )
            self._consume()
            pipe_tok = self._peek()
            if pipe_tok.kind is not TokenKind.PIPE:
                raise ParseError(
                    "'as $name' must be immediately followed by '|' and "
                    f"the body expression, got {pipe_tok.text!r}",
                    pipe_tok.offset,
                )
            self._consume()
            body = self._parse_pipeline()
            return LetBinding(
                source=lhs,
                name=str(name_tok.value),
                body=body,
                offset=tok.offset,
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
        """Parse a primary followed by any chain of postfix path-steps.

        Trailing ``.field`` and ``[...]`` are accepted after any
        primary, not just after a leading ``.`` — that matches jq's
        ``f[]`` / ``f.field`` postfix and lets idioms like
        ``refs(.)[]`` (iterate the result of a call) work.

        Non-Identity primaries (``$x.foo``, ``f(.).y[0]``,
        ``"literal"[0]``) emit a :class:`PathExpr` with ``head`` set
        to the primary.  The evaluator walks ``head`` to obtain the
        starting value but keeps the original pipe input as ``.``
        for subscript index expressions — matching jq's
        ``$x[.field]`` semantics where ``.field`` resolves against
        the surrounding current, not against ``$x``.
        """
        primary = self._parse_primary()
        steps: list = []
        while True:
            tok = self._peek()
            if tok.kind is TokenKind.DOT and self._peek(1).kind in (
                TokenKind.IDENT,
                TokenKind.STRING,
            ):
                self._consume()  # '.'
                steps.append(self._parse_field_step())
                continue
            if tok.kind is TokenKind.LBRACKET:
                steps.append(self._parse_subscript_step())
                continue
            break
        if not steps:
            return primary
        offset = getattr(primary, "offset", 0)
        # Identity primary: the steps walk from ``.`` directly —
        # no need for a ``head`` wrapper.  Every other primary
        # becomes the ``head`` of the path so the evaluator can
        # preserve the outer current for subscript expressions.
        if isinstance(primary, Identity):
            return PathExpr(steps=tuple(steps), offset=offset)
        return PathExpr(steps=tuple(steps), offset=offset, head=primary)

    def _parse_primary(self) -> Expr:
        tok = self._peek()

        if tok.kind in (
            TokenKind.NUMBER,
            TokenKind.STRING,
            TokenKind.TRUE,
            TokenKind.FALSE,
            TokenKind.NULL,
        ):
            self._consume()
            return Literal(value=tok.value, offset=tok.offset)

        if tok.kind is TokenKind.LPAREN:
            self._consume()
            inner = self._parse_pipeline()
            self._expect(TokenKind.RPAREN, msg="expected ')'")
            return inner

        if tok.kind is TokenKind.LBRACKET:
            # ``[ expr ]`` is a list literal that collects the stream
            # of values produced by *expr* into a single list — jq's
            # standard idiom for folding a generator before piping it
            # into an aggregator (``[.X[].name] | sort | first``).
            # The empty form ``[]`` is the empty list.
            self._consume()
            if self._peek().kind is TokenKind.RBRACKET:
                self._consume()
                return ListLiteral(inner=None, offset=tok.offset)
            # List-literal contents are a full pipeline (commas included)
            # — jq's ``[a, b, c]`` is exactly the comma-stream collected.
            inner_expr = self._parse_pipeline()
            self._expect(TokenKind.RBRACKET, msg="expected ']' to close list literal")
            return ListLiteral(inner=inner_expr, offset=tok.offset)

        if tok.kind is TokenKind.IDENT:
            # Bare identifier is a call.  When invoked with no
            # parentheses ``length`` is treated by the evaluator as
            # ``length(.)`` for single-argument non-special-form
            # builtins (matches jq's bare-filter convention).  An
            # empty arg list ``f()`` is also accepted.
            self._consume()
            args: list[Expr] = []
            if self._peek().kind is TokenKind.LPAREN:
                self._consume()
                if self._peek().kind is not TokenKind.RPAREN:
                    # Inside call arguments the comma is a separator,
                    # not the stream-concat operator — use the
                    # no-comma pipeline form.  Callers that want to
                    # pass a comma stream as a single argument wrap
                    # it in parens: ``f((a, b))``.
                    args.append(self._parse_pipeline_no_comma())
                    while self._match(TokenKind.COMMA):
                        args.append(self._parse_pipeline_no_comma())
                self._expect(TokenKind.RPAREN, msg="expected ')' to close call")
            return Call(name=tok.text, args=tuple(args), offset=tok.offset)

        if tok.kind is TokenKind.DOT:
            return self._parse_path_starting_with_dot()

        if tok.kind is TokenKind.LBRACE:
            return self._parse_object_literal()

        if tok.kind is TokenKind.DOLLAR_IDENT:
            # ``$name`` resolves to the root container of the named
            # source.  Postfix path steps land afterwards via
            # :meth:`_parse_primary_with_postfix`, so
            # ``$gtm.gtm.wideip[]`` parses as the variable followed by
            # the usual ``.gtm.wideip[]`` walk.
            self._consume()
            return Variable(name=str(tok.value), offset=tok.offset)

        if tok.kind is TokenKind.IF:
            return self._parse_if_then_else()

        raise ParseError(f"unexpected token {tok.text!r}", tok.offset)

    def _parse_if_then_else(self) -> "IfThenElse":
        """Parse ``if COND then BODY [elif COND then BODY]* [else BODY] end``.

        Matches jq's conditional form.  ``else`` is optional; without
        it the evaluator passes the input through on falsy conds.
        Each ``COND`` / ``BODY`` is a full pipeline so let-bindings,
        pipes, and assignments compose naturally.
        """
        if_tok = self._expect(TokenKind.IF, msg="expected 'if'")
        cond = self._parse_pipeline()
        self._expect(TokenKind.THEN, msg="expected 'then' after if-condition")
        then_body = self._parse_pipeline()
        elifs: list[tuple[Expr, Expr]] = []
        while self._peek().kind is TokenKind.ELIF:
            self._consume()
            elif_cond = self._parse_pipeline()
            self._expect(TokenKind.THEN, msg="expected 'then' after elif-condition")
            elif_body = self._parse_pipeline()
            elifs.append((elif_cond, elif_body))
        else_body: Expr | None = None
        if self._peek().kind is TokenKind.ELSE:
            self._consume()
            else_body = self._parse_pipeline()
        self._expect(TokenKind.END, msg="expected 'end' to close if-expression")
        return IfThenElse(
            cond=cond,
            then_body=then_body,
            elifs=tuple(elifs),
            else_body=else_body,
            offset=if_tok.offset,
        )

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
        optional = self._consume_optional_marker()
        return Field(name=name, optional=optional, offset=tok.offset)

    def _parse_subscript_step(self) -> Subscript:
        lb = self._expect(TokenKind.LBRACKET, msg="expected '['")
        nxt = self._peek()
        if nxt.kind is TokenKind.RBRACKET:
            self._consume()
            optional = self._consume_optional_marker()
            return Subscript(
                stream=True, index=None, regex=None, offset=lb.offset, optional=optional
            )
        # A bare string subscript whose contents start with ``~`` is a
        # regex subscript.  We recognise it here rather than in the
        # lexer so a string literal starting with ``~`` in any other
        # position (e.g. ``sub(.x, "~lit", "y")``) is still just a
        # string.  ``["~"]`` matches everything (empty pattern).
        if (
            nxt.kind is TokenKind.STRING
            and isinstance(nxt.value, str)
            and nxt.value.startswith("~")
            and self._peek(1).kind is TokenKind.RBRACKET
        ):
            self._consume()
            self._expect(TokenKind.RBRACKET, msg="expected ']' after regex subscript")
            optional = self._consume_optional_marker()
            return Subscript(
                stream=False, index=None, regex=nxt.value[1:], offset=lb.offset, optional=optional
            )
        inner = self._parse_pipeline()
        self._expect(TokenKind.RBRACKET, msg="expected ']'")
        optional = self._consume_optional_marker()
        return Subscript(stream=False, index=inner, regex=None, offset=lb.offset, optional=optional)

    def _consume_optional_marker(self) -> bool:
        """Consume an optional ``?`` token after a path step.

        Matches jq's optional-access form: ``.foo?`` and ``.[expr]?``
        swallow a missing key, out-of-range index, or wrong-type
        subscript error and produce no value instead of raising.
        """
        if self._peek().kind is TokenKind.QUESTION:
            self._consume()
            return True
        return False

    def _parse_object_literal(self) -> Expr:
        """``{ key: expr, key2, key3: expr3, ... }`` — jq's object constructor.

        Bareword keys may use the same identifier rules as ``.field`` (so
        ``data-group`` lexes as one token), or they can be quoted
        strings for keys with unusual characters.  When the value is
        omitted (``{name}``) it desugars to ``{name: .name}`` which is
        the common case when projecting an object into a row.
        """
        lb = self._consume()  # consume '{'
        entries: list[tuple[str, Expr]] = []
        if self._peek().kind is TokenKind.RBRACE:
            self._consume()
            return ObjectLiteral(entries=(), offset=lb.offset)
        while True:
            key_tok = self._peek()
            if key_tok.kind is TokenKind.IDENT:
                self._consume()
                key = key_tok.text
            elif key_tok.kind is TokenKind.STRING:
                self._consume()
                key = str(key_tok.value)
            else:
                raise ParseError(
                    "expected an object-literal key (identifier or string) "
                    f"but got {key_tok.text!r}",
                    key_tok.offset,
                )
            # Optional ``: expression`` — when omitted, desugar to a
            # ``.<key>`` path so ``{name, destination}`` works as
            # jq users expect.
            if self._peek().kind is TokenKind.COLON:
                self._consume()
                value: Expr = self._parse_pipe_stage()
            else:
                value = PathExpr(
                    steps=(Field(name=key, optional=False, offset=key_tok.offset),),
                    offset=key_tok.offset,
                )
            entries.append((key, value))
            if self._peek().kind is TokenKind.COMMA:
                self._consume()
                continue
            break
        self._expect(TokenKind.RBRACE, msg="expected '}' to close object literal")
        return ObjectLiteral(entries=tuple(entries), offset=lb.offset)


def _as_path(expr: Expr, op_offset: int) -> tuple[PathExpr, Variable | None]:
    """Return (path, source) for an assignment LHS.

    ``source`` is set when the LHS was ``$name.path`` — the parser
    produces that as ``Pipe(Variable, PathExpr)`` via the
    primary-with-postfix machinery, and the evaluator needs to know
    which named root to evaluate the path against.  The common case
    (``.path``) returns ``(path, None)``.
    """
    if isinstance(expr, PathExpr):
        return expr, None
    if isinstance(expr, Identity):
        return PathExpr(steps=(), offset=expr.offset), None
    if isinstance(expr, Pipe) and isinstance(expr.lhs, Variable):
        rhs = expr.rhs
        if isinstance(rhs, PathExpr):
            return rhs, expr.lhs
        if isinstance(rhs, Identity):
            return PathExpr(steps=(), offset=rhs.offset), expr.lhs
    raise ParseError(
        "left-hand side of an assignment must be a path expression (starting with '.' or '$name')",
        op_offset,
    )


def parse_query(source: str) -> Program:
    """Tokenise and parse *source* into a :class:`.ast.Program`."""
    tokens = tokenise(source)
    return _Parser(tokens).parse_program()
