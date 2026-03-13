"""Sub-lexer for Tcl [expr] bodies.

Tokenises the infix expression sub-language: operators, numbers, math
functions, string literals, variable references ($var), and command
substitutions ([cmd]).
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum, auto


class ExprTokenType(Enum):
    """Token types specific to Tcl expressions."""

    NUMBER = auto()  # integer or float literal
    STRING = auto()  # "quoted string"
    VARIABLE = auto()  # $var, $ns::var, $arr(idx)
    COMMAND = auto()  # [cmd ...]
    OPERATOR = auto()  # + - * / % ** == != < > <= >= && || ! & | ^ ~ << >>
    PAREN_OPEN = auto()  # (
    PAREN_CLOSE = auto()  # )
    COMMA = auto()  # , (function argument separator)
    FUNCTION = auto()  # math function name: sin, cos, int, double, etc.
    BOOL = auto()  # true, false
    TERNARY_Q = auto()  # ?
    TERNARY_C = auto()  # :  (ternary colon)
    WHITESPACE = auto()  # spaces/tabs
    EOF = auto()


@dataclass(frozen=True, slots=True)
class ExprToken:
    """A token in a Tcl expression."""

    type: ExprTokenType
    text: str
    start: int  # offset within the expression string
    end: int  # offset of last character


# Known Tcl math functions
_MATH_FUNCTIONS = frozenset(
    {
        "abs",
        "acos",
        "asin",
        "atan",
        "atan2",
        "bool",
        "ceil",
        "cos",
        "cosh",
        "double",
        "entier",
        "exp",
        "floor",
        "fmod",
        "hypot",
        "int",
        "isinf",
        "isnan",
        "isqrt",
        "log",
        "log10",
        "max",
        "min",
        "pow",
        "rand",
        "round",
        "sin",
        "sinh",
        "sqrt",
        "srand",
        "tan",
        "tanh",
        "wide",
    }
)

# Multi-character operators (longest first for greedy matching)
_MULTI_OPS = [
    "**",
    "<<",
    ">>",
    "<=",
    ">=",
    "==",
    "!=",
    "&&",
    "||",
    "ne",
    "eq",
    "in",
    "ni",
    "lt",
    "le",
    "gt",
    "ge",
]

# Single-character operators
_SINGLE_OPS = frozenset("+-*/%<>&|^~!")

# iRules-specific word operators (only recognised when dialect == "f5-irules")
_IRULES_OPS = frozenset(
    {
        "and",
        "or",
        "not",
        "contains",
        "starts_with",
        "ends_with",
        "equals",
        "matches_glob",
        "matches_regex",
    }
)


class ExprLexer:
    """Tokeniser for Tcl [expr] bodies."""

    def __init__(self, source: str, *, dialect: str | None = None) -> None:
        self._src = source
        self._pos = 0
        self._dialect = dialect
        self._has_unknown = False

    def tokenise(self) -> list[ExprToken]:
        """Tokenise the entire expression string."""
        tokens: list[ExprToken] = []
        while self._pos < len(self._src):
            ch = self._src[self._pos]

            # Whitespace
            if ch in " \t\n\r":
                start = self._pos
                while self._pos < len(self._src) and self._src[self._pos] in " \t\n\r":
                    self._pos += 1
                tokens.append(
                    ExprToken(
                        ExprTokenType.WHITESPACE, self._src[start : self._pos], start, self._pos - 1
                    )
                )
                continue

            # Numbers
            if ch.isdigit() or (
                ch == "." and self._pos + 1 < len(self._src) and self._src[self._pos + 1].isdigit()
            ):
                tokens.append(self._read_number())
                continue

            # Variable reference
            if ch == "$":
                tokens.append(self._read_variable())
                continue

            # Command substitution
            if ch == "[":
                tokens.append(self._read_command())
                continue

            # Quoted string
            if ch == '"':
                tokens.append(self._read_string())
                continue

            # Parentheses
            if ch == "(":
                # Check if previous token was a function name
                if tokens and tokens[-1].type == ExprTokenType.FUNCTION:
                    pass  # keep as paren
                tokens.append(ExprToken(ExprTokenType.PAREN_OPEN, "(", self._pos, self._pos))
                self._pos += 1
                continue
            if ch == ")":
                tokens.append(ExprToken(ExprTokenType.PAREN_CLOSE, ")", self._pos, self._pos))
                self._pos += 1
                continue

            # Comma
            if ch == ",":
                tokens.append(ExprToken(ExprTokenType.COMMA, ",", self._pos, self._pos))
                self._pos += 1
                continue

            # Ternary
            if ch == "?":
                tokens.append(ExprToken(ExprTokenType.TERNARY_Q, "?", self._pos, self._pos))
                self._pos += 1
                continue
            if ch == ":":
                # Could be ternary colon or namespace separator (in variable)
                tokens.append(ExprToken(ExprTokenType.TERNARY_C, ":", self._pos, self._pos))
                self._pos += 1
                continue

            # Multi-character operators
            matched = False
            for op in _MULTI_OPS:
                if self._src[self._pos : self._pos + len(op)] == op:
                    # Word operators require word boundaries: they must
                    # not be part of a longer identifier.  Preceded by a
                    # letter/underscore means we're mid-word (e.g. "equal"),
                    # so skip.  But digits before the op are fine (e.g.
                    # "1eq4" parses as "1 eq 4" in Tcl).
                    if op in ("eq", "ne", "in", "ni", "lt", "le", "gt", "ge"):
                        # Check preceding character — if alpha/underscore,
                        # this is mid-identifier, not an operator.
                        if self._pos > 0:
                            prev = self._src[self._pos - 1]
                            if prev.isalpha() or prev == "_":
                                continue
                        after = self._pos + len(op)
                        if after < len(self._src) and (
                            self._src[after].isalpha() or self._src[after] == "_"
                        ):
                            continue
                    tokens.append(
                        ExprToken(ExprTokenType.OPERATOR, op, self._pos, self._pos + len(op) - 1)
                    )
                    self._pos += len(op)
                    matched = True
                    break
            if matched:
                continue

            # Single-character operators
            if ch in _SINGLE_OPS:
                tokens.append(ExprToken(ExprTokenType.OPERATOR, ch, self._pos, self._pos))
                self._pos += 1
                continue

            # Identifier (function name or boolean literal)
            if ch.isalpha() or ch == "_":
                tokens.append(self._read_identifier())
                continue

            # Braces (Tcl braced expression)
            if ch == "{":
                tokens.append(self._read_braced())
                continue

            # Unknown character — skip but record the gap.
            self._has_unknown = True
            self._pos += 1

        return tokens

    def _read_number(self) -> ExprToken:
        start = self._pos
        # Handle 0x, 0o, 0b prefixes
        if (
            self._src[self._pos] == "0"
            and self._pos + 1 < len(self._src)
            and self._src[self._pos + 1] in "xXoObB"
        ):
            self._pos += 2
            while self._pos < len(self._src) and (
                self._src[self._pos].isalnum() or self._src[self._pos] == "_"
            ):
                self._pos += 1
            return ExprToken(
                ExprTokenType.NUMBER, self._src[start : self._pos], start, self._pos - 1
            )

        # Regular decimal integer/float
        while self._pos < len(self._src) and self._src[self._pos].isdigit():
            self._pos += 1
        if self._pos < len(self._src) and self._src[self._pos] == ".":
            self._pos += 1
            while self._pos < len(self._src) and self._src[self._pos].isdigit():
                self._pos += 1
        # Scientific notation — only consume 'e'/'E' if followed by a
        # digit (or sign then digit), so that "1eq4" is not misread as
        # number "1e" + identifier "q4".
        if self._pos < len(self._src) and self._src[self._pos] in "eE":
            save = self._pos
            self._pos += 1
            if self._pos < len(self._src) and self._src[self._pos] in "+-":
                self._pos += 1
            if self._pos < len(self._src) and self._src[self._pos].isdigit():
                while self._pos < len(self._src) and self._src[self._pos].isdigit():
                    self._pos += 1
            else:
                # Not valid scientific notation — back up.
                self._pos = save

        return ExprToken(ExprTokenType.NUMBER, self._src[start : self._pos], start, self._pos - 1)

    def _read_variable(self) -> ExprToken:
        start = self._pos
        self._pos += 1  # skip $

        if self._pos < len(self._src) and self._src[self._pos] == "{":
            # ${name}
            self._pos += 1
            while self._pos < len(self._src) and self._src[self._pos] != "}":
                self._pos += 1
            if self._pos < len(self._src):
                self._pos += 1  # skip }
        else:
            # $name, $ns::var, $arr(idx)
            while self._pos < len(self._src) and (
                self._src[self._pos].isalnum() or self._src[self._pos] in "_:"
            ):
                self._pos += 1
            if self._pos < len(self._src) and self._src[self._pos] == "(":
                self._pos += 1
                level = 1
                while self._pos < len(self._src) and level > 0:
                    if self._src[self._pos] == "(":
                        level += 1
                    elif self._src[self._pos] == ")":
                        level -= 1
                    self._pos += 1

        return ExprToken(ExprTokenType.VARIABLE, self._src[start : self._pos], start, self._pos - 1)

    def _read_command(self) -> ExprToken:
        start = self._pos
        self._pos += 1  # skip [
        level = 1
        while self._pos < len(self._src) and level > 0:
            if self._src[self._pos] == "[":
                level += 1
            elif self._src[self._pos] == "]":
                level -= 1
            elif self._src[self._pos] == "\\":
                self._pos += 1  # skip escaped char
            self._pos += 1

        return ExprToken(ExprTokenType.COMMAND, self._src[start : self._pos], start, self._pos - 1)

    def _read_string(self) -> ExprToken:
        start = self._pos
        self._pos += 1  # skip opening quote
        while self._pos < len(self._src) and self._src[self._pos] != '"':
            if self._src[self._pos] == "\\":
                self._pos += 1  # skip escaped char
            self._pos += 1
        if self._pos < len(self._src):
            self._pos += 1  # skip closing quote

        return ExprToken(ExprTokenType.STRING, self._src[start : self._pos], start, self._pos - 1)

    def _read_identifier(self) -> ExprToken:
        start = self._pos
        while self._pos < len(self._src) and (
            self._src[self._pos].isalnum() or self._src[self._pos] == "_"
        ):
            self._pos += 1
        text = self._src[start : self._pos]

        if text in ("true", "false", "yes", "no", "on", "off"):
            return ExprToken(ExprTokenType.BOOL, text, start, self._pos - 1)
        if self._dialect == "f5-irules" and text in _IRULES_OPS:
            return ExprToken(ExprTokenType.OPERATOR, text, start, self._pos - 1)
        if text in _MATH_FUNCTIONS:
            return ExprToken(ExprTokenType.FUNCTION, text, start, self._pos - 1)

        # Unknown identifier -- treat as a function name (could be user-defined)
        return ExprToken(ExprTokenType.FUNCTION, text, start, self._pos - 1)

    def _read_braced(self) -> ExprToken:
        """Read a braced expression (treated as string).

        If the closing ``}`` is never found, only the opening ``{`` is
        emitted so subsequent characters get tokenised normally and the
        parser rejects the expression (two adjacent atoms → ExprRaw →
        syntax error).
        """
        start = self._pos
        self._pos += 1  # skip {
        level = 1
        saved = self._pos
        while self._pos < len(self._src) and level > 0:
            if self._src[self._pos] == "{":
                level += 1
            elif self._src[self._pos] == "}":
                level -= 1
            self._pos += 1

        if level != 0:
            # Unmatched brace — rewind and emit just the bare '{'.
            self._pos = saved
            return ExprToken(ExprTokenType.STRING, "{", start, start)

        return ExprToken(ExprTokenType.STRING, self._src[start : self._pos], start, self._pos - 1)


def tokenise_expr(source: str, *, dialect: str | None = None) -> list[ExprToken]:
    """Convenience: tokenise a Tcl expression string."""
    return ExprLexer(source, dialect=dialect).tokenise()


def tokenise_expr_checked(
    source: str, *, dialect: str | None = None
) -> tuple[list[ExprToken], bool]:
    """Tokenise and report whether unknown characters were skipped.

    Returns ``(tokens, has_unknown)`` where *has_unknown* is ``True``
    when the source contained characters the lexer could not classify.
    """
    lexer = ExprLexer(source, dialect=dialect)
    tokens = lexer.tokenise()
    return tokens, lexer._has_unknown
