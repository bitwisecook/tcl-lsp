"""Exception hierarchy for the query DSL.

Every error message is shaped so the CLI verb can prefix it with
``error:`` and present it directly to the user.  The optional
*position* on parse / lex errors lets us underline the offending span
in the source.
"""

from __future__ import annotations

from dataclasses import dataclass


class QueryError(Exception):
    """Base class for every error raised by the query DSL."""


@dataclass
class LexError(QueryError):
    """The lexer hit a character it does not understand."""

    message: str
    offset: int

    def __str__(self) -> str:  # pragma: no cover - exercised via the CLI
        return f"{self.message} at offset {self.offset}"


@dataclass
class ParseError(QueryError):
    """The parser saw a token it did not expect."""

    message: str
    offset: int

    def __str__(self) -> str:  # pragma: no cover - exercised via the CLI
        return f"{self.message} at offset {self.offset}"


class EvalError(QueryError):
    """Evaluation hit an undefined name, a type mismatch, or similar."""


class EditError(QueryError):
    """An assignment cannot be applied (conflict, non-writable path, ...)."""


class BuiltinError(QueryError):
    """A builtin function rejected its arguments."""
