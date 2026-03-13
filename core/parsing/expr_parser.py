"""Pratt parser for Tcl [expr] expressions.

Builds a structured :class:`~core.compiler.expr_ast.ExprNode` tree from
the flat token list produced by :func:`tokenise_expr`.  Falls back to
:class:`~core.compiler.expr_ast.ExprRaw` on any parse error, ensuring
the compiler pipeline never crashes on malformed expressions.

Tcl expression precedence (low → high), following the Tcl man page:

  1.  ``? :``     (ternary, right-associative)
  2.  ``||``
  3.  ``&&``
  4.  ``|``
  5.  ``^``
  6.  ``&``
  7.  ``== != eq ne``
  8.  ``< > <= >= lt le gt ge in ni``
  9.  ``<< >>``
  10. ``+ -``
  11. ``* / %``
  12. ``**``       (right-associative)
  13. unary ``+ - ~ !``
  14. atoms, function calls, parenthesised sub-expressions
"""

from __future__ import annotations

from ..common.naming import normalise_var_name as _normalise_var_name
from ..compiler.expr_ast import (
    BinOp,
    ExprBinary,
    ExprCall,
    ExprCommand,
    ExprLiteral,
    ExprNode,
    ExprRaw,
    ExprString,
    ExprTernary,
    ExprUnary,
    ExprVar,
    UnaryOp,
)
from .expr_lexer import ExprToken, ExprTokenType, tokenise_expr_checked


class _ParseError(Exception):
    """Internal signal — caught by :func:`parse_expr`."""


# Binding powers  (left_bp, right_bp)
#
# For left-associative ops:  right_bp = left_bp + 1
# For right-associative ops: right_bp = left_bp

_BINARY_BP: dict[str, tuple[int, int]] = {
    # Ternary handled specially — bp 2 so it binds very loosely
    # "?"  is prefix of ternary, parsed in _infix
    # Logical
    "||": (4, 5),
    "&&": (6, 7),
    # Bitwise
    "|": (8, 9),
    "^": (10, 11),
    "&": (12, 13),
    # Equality / string comparison
    "==": (14, 15),
    "!=": (14, 15),
    "eq": (14, 15),
    "ne": (14, 15),
    # Relational / list membership / string comparison
    "<": (16, 17),
    ">": (16, 17),
    "<=": (16, 17),
    ">=": (16, 17),
    "in": (16, 17),
    "ni": (16, 17),
    "lt": (16, 17),
    "le": (16, 17),
    "gt": (16, 17),
    "ge": (16, 17),
    # Shift
    "<<": (18, 19),
    ">>": (18, 19),
    # Additive
    "+": (20, 21),
    "-": (20, 21),
    # Multiplicative
    "*": (22, 23),
    "/": (22, 23),
    "%": (22, 23),
    # Exponentiation (right-associative, BELOW unary operators in Tcl)
    "**": (23, 23),
    # iRules word logical operators (same precedence as symbolic counterparts)
    "or": (4, 5),
    "and": (6, 7),
    # iRules string comparison operators (same precedence as eq/ne)
    "contains": (14, 15),
    "starts_with": (14, 15),
    "ends_with": (14, 15),
    "equals": (14, 15),
    "matches_glob": (14, 15),
    "matches_regex": (14, 15),
}

_BINOP_MAP: dict[str, BinOp] = {op.value: op for op in BinOp}

_UNARY_MAP: dict[str, UnaryOp] = {
    "-": UnaryOp.NEG,
    "+": UnaryOp.POS,
    "~": UnaryOp.BIT_NOT,
    "!": UnaryOp.NOT,
    "not": UnaryOp.WORD_NOT,
}

# Binding power for prefix unary operators (higher than any binary op).
_UNARY_BP = 24


class _PrattParser:
    """Pratt (top-down operator precedence) parser for Tcl expressions."""

    def __init__(self, tokens: list[ExprToken], source: str) -> None:
        self.tokens = tokens
        self.source = source
        self.pos = 0

    def _peek(self) -> ExprToken | None:
        if self.pos < len(self.tokens):
            return self.tokens[self.pos]
        return None

    def _advance(self) -> ExprToken:
        tok = self.tokens[self.pos]
        self.pos += 1
        return tok

    def _expect(self, tok_type: ExprTokenType, text: str | None = None) -> ExprToken:
        tok = self._peek()
        if tok is None:
            raise _ParseError(f"expected {tok_type} but got EOF")
        if tok.type is not tok_type:
            raise _ParseError(f"expected {tok_type} but got {tok.type}")
        if text is not None and tok.text != text:
            raise _ParseError(f"expected {text!r} but got {tok.text!r}")
        return self._advance()

    def expression(self, min_bp: int = 0) -> ExprNode:
        """Parse an expression with minimum binding power *min_bp*."""
        left = self._prefix()

        while True:
            tok = self._peek()
            if tok is None:
                break

            # Ternary operator
            if tok.type is ExprTokenType.TERNARY_Q:
                if 2 < min_bp:  # ternary bp = 2, right-assoc so use <
                    break
                self._advance()  # consume '?'
                true_branch = self.expression(0)
                self._expect(ExprTokenType.TERNARY_C)
                false_branch = self.expression(2)  # right-assoc
                left = ExprTernary(
                    condition=left,
                    true_branch=true_branch,
                    false_branch=false_branch,
                )
                continue

            # Binary operators
            if tok.type is ExprTokenType.OPERATOR:
                bp = _BINARY_BP.get(tok.text)
                if bp is None:
                    break  # unknown operator — stop
                left_bp, right_bp = bp
                if left_bp < min_bp:
                    break
                self._advance()
                right = self.expression(right_bp)
                binop = _BINOP_MAP.get(tok.text)
                if binop is None:
                    raise _ParseError(f"unknown binary operator {tok.text!r}")
                left = ExprBinary(op=binop, left=left, right=right)
                continue

            # Nothing else can extend the expression
            break

        return left

    def _prefix(self) -> ExprNode:
        tok = self._peek()
        if tok is None:
            raise _ParseError("unexpected end of expression")

        # Unary operators
        if tok.type is ExprTokenType.OPERATOR and tok.text in _UNARY_MAP:
            self._advance()
            operand = self.expression(_UNARY_BP)
            return ExprUnary(op=_UNARY_MAP[tok.text], operand=operand)

        # Parenthesised sub-expression
        if tok.type is ExprTokenType.PAREN_OPEN:
            self._advance()
            inner = self.expression(0)
            self._expect(ExprTokenType.PAREN_CLOSE)
            return inner

        # Number literal
        if tok.type is ExprTokenType.NUMBER:
            self._advance()
            return ExprLiteral(text=tok.text, start=tok.start, end=tok.end)

        # Boolean literal
        if tok.type is ExprTokenType.BOOL:
            self._advance()
            return ExprLiteral(text=tok.text, start=tok.start, end=tok.end)

        # String literal
        if tok.type is ExprTokenType.STRING:
            self._advance()
            return ExprString(text=tok.text, start=tok.start, end=tok.end)

        # Variable reference
        if tok.type is ExprTokenType.VARIABLE:
            self._advance()
            name = _normalise_var_name(tok.text)
            return ExprVar(text=tok.text, name=name, start=tok.start, end=tok.end)

        # Command substitution
        if tok.type is ExprTokenType.COMMAND:
            self._advance()
            return ExprCommand(text=tok.text, start=tok.start, end=tok.end)

        # Function call: name ( args )
        if tok.type is ExprTokenType.FUNCTION:
            return self._parse_function_call()

        raise _ParseError(f"unexpected token {tok.type}: {tok.text!r}")

    def _parse_function_call(self) -> ExprNode:
        func_tok = self._advance()  # FUNCTION token
        self._expect(ExprTokenType.PAREN_OPEN)
        args: list[ExprNode] = []
        # Check for empty argument list
        peek = self._peek()
        if peek is not None and peek.type is ExprTokenType.PAREN_CLOSE:
            self._advance()
            return ExprCall(
                function=func_tok.text,
                args=tuple(args),
                start=func_tok.start,
                end=peek.end,
            )
        # Parse first argument
        args.append(self.expression(0))
        # Parse remaining comma-separated arguments
        while True:
            peek = self._peek()
            if peek is None:
                raise _ParseError("unterminated function call")
            if peek.type is ExprTokenType.PAREN_CLOSE:
                close_tok = self._advance()
                return ExprCall(
                    function=func_tok.text,
                    args=tuple(args),
                    start=func_tok.start,
                    end=close_tok.end,
                )
            if peek.type is ExprTokenType.COMMA:
                self._advance()
                args.append(self.expression(0))
            else:
                raise _ParseError(f"expected ',' or ')' in function call, got {peek.text!r}")


def parse_expr(source: str, *, dialect: str | None = None) -> ExprNode:
    """Parse a Tcl expression string into a structured AST.

    Returns :class:`ExprRaw` on any error, so the compiler pipeline
    never crashes on malformed expressions.
    """
    raw_tokens, has_unknown = tokenise_expr_checked(source, dialect=dialect)
    if has_unknown:
        # Source contained characters the lexer could not classify
        # (e.g. backtick) — treat as unparseable so eval_expr raises
        # a syntax error.
        return ExprRaw(text=source)
    tokens = [t for t in raw_tokens if t.type is not ExprTokenType.WHITESPACE]
    if not tokens:
        return ExprRaw(text=source)
    try:
        parser = _PrattParser(tokens, source)
        result = parser.expression(0)
        if parser.pos < len(parser.tokens):
            # Unconsumed tokens — treat as unparseable.
            return ExprRaw(text=source)
        return result
    except _ParseError:
        return ExprRaw(text=source)
