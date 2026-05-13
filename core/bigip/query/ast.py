"""AST node types for the query DSL.

The tree is deliberately simple: one node class per syntactic form,
all frozen dataclasses, and every node carries the byte offset where
it began so error messages can underline the right span.

The evaluator (:mod:`.evaluator`) walks this tree top-down, producing
either a plain Python value or a :class:`.values.Stream`.  Path nodes
also produce a *location trail* used by the edit planner when the user
assigns to them.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Union


@dataclass(frozen=True, slots=True)
class Literal:
    value: object
    offset: int


@dataclass(frozen=True, slots=True)
class Identity:
    """The bare ``.`` — yields the current input."""

    offset: int


@dataclass(frozen=True, slots=True)
class Variable:
    """``$name`` — yields the root container of the named source.

    Used to address one specific config when the query verb was given
    more than one input.  Without ``--merge`` this is the only way to
    reach a non-primary source from inside an expression; with
    ``--merge`` it is still useful for narrowing back to one file.
    """

    name: str
    offset: int


@dataclass(frozen=True, slots=True)
class ListLiteral:
    """``[ inner ]`` — collect *inner*'s output into a list.

    Matches jq's array constructor: ``[.X[].name]`` collects the
    stream of names produced inside the brackets into a single list
    value, so subsequent pipe stages see one list rather than the
    stream iterating per item.  An empty ``[]`` is the empty list.
    """

    inner: "Expr | None"
    offset: int


@dataclass(frozen=True, slots=True)
class Field:
    """``.foo`` or ``."foo-bar"`` — a single named step."""

    name: str
    optional: bool  # `?` suffix — not used in v1, reserved.
    offset: int


@dataclass(frozen=True, slots=True)
class Subscript:
    """``[expr]`` / ``["~regex"]`` / ``[number]`` / ``[]``."""

    # When `stream` is True this is the bare ``[]`` (iterate all).
    stream: bool
    index: "Expr | None"
    regex: str | None  # set when the subscript was ``["~..."]``
    offset: int


@dataclass(frozen=True, slots=True)
class PathExpr:
    """A chain of ``Field`` / ``Subscript`` steps starting from input.

    Path expressions are special: they are both readable (the evaluator
    follows the chain to fetch a value) and writable (an assignment
    writes back through them).  Other expression kinds cannot appear on
    the LHS of ``=`` / ``|=``.
    """

    steps: tuple["PathStep", ...]
    offset: int


PathStep = Union[Field, Subscript]


@dataclass(frozen=True, slots=True)
class Call:
    name: str
    args: tuple["Expr", ...]
    offset: int


@dataclass(frozen=True, slots=True)
class BinOp:
    op: str  # "==", "!=", "<", "<=", ">", ">=", "+", "-", "*", "/", "and", "or"
    lhs: "Expr"
    rhs: "Expr"
    offset: int


@dataclass(frozen=True, slots=True)
class UnaryOp:
    op: str  # "not", "-"
    operand: "Expr"
    offset: int


@dataclass(frozen=True, slots=True)
class Pipe:
    """``lhs | rhs`` — evaluate lhs, feed each value through rhs."""

    lhs: "Expr"
    rhs: "Expr"
    offset: int


@dataclass(frozen=True, slots=True)
class Assignment:
    """``path = expr`` / ``path |= expr`` / ``path += expr`` / ``-=``.

    ``op`` is the source-spelt operator.  Semantics:

    - ``=``  → set path to value of rhs (rhs is evaluated against the
      outer input, not against the path's current value)
    - ``|=`` → set path to ``path | rhs`` (rhs evaluated with ``.``
      bound to the path's current value)
    - ``+=`` / ``-=`` → ``path = path <op> rhs`` for numeric or string
      values

    ``source`` is set when the LHS was ``$name.path``: the variable is
    evaluated once and its root container is used as the input for the
    path walk instead of the outer ``.``.  ``None`` is the common
    case (``.path = ...``).
    """

    target: PathExpr
    op: str
    rhs: "Expr"
    offset: int
    source: "Variable | None" = None


@dataclass(frozen=True, slots=True)
class Program:
    """One or more semicolon-separated statements."""

    statements: tuple["Expr", ...]


Expr = Union[
    Literal,
    Identity,
    Variable,
    ListLiteral,
    PathExpr,
    Call,
    BinOp,
    UnaryOp,
    Pipe,
    Assignment,
]
