"""Intermediate representation (IR) for Tcl analysis.

This is a structured IR front-end that keeps source ranges on all nodes.
Later passes can lower this further to CFG + SSA.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import TYPE_CHECKING

from ..analysis.semantic_model import Range
from .expr_ast import ExprNode

if TYPE_CHECKING:
    from ..parsing.tokens import Token


@dataclass(frozen=True, slots=True)
class CommandTokens:
    """Original parsed tokens for a command invocation.

    Carried on ``IRCall`` and ``IRBarrier`` so downstream passes
    (optimiser, compiler checks) can inspect tokens without re-lexing.
    """

    argv: tuple["Token", ...]
    argv_texts: tuple[str, ...]
    single_token_word: tuple[bool, ...]
    all_tokens: tuple["Token", ...] = ()
    expand_word: tuple[bool, ...] | None = None  # {*} expansion markers


@dataclass(frozen=True, slots=True)
class IRAssignConst:
    range: Range
    name: str
    value: str


@dataclass(frozen=True, slots=True)
class IRAssignExpr:
    range: Range
    name: str
    expr: ExprNode


@dataclass(frozen=True, slots=True)
class IRAssignValue:
    range: Range
    name: str
    value: str
    value_needs_backsubst: bool = False


@dataclass(frozen=True, slots=True)
class IRIncr:
    range: Range
    name: str
    amount: str | None = None


@dataclass(frozen=True, slots=True)
class IRExprEval:
    """Standalone expression evaluation (``expr`` command).

    Evaluates the expression for its result but does not assign it to
    a variable.  The result becomes the command's return value.
    """

    range: Range
    expr: ExprNode


@dataclass(frozen=True, slots=True)
class IRCall:
    """A generic command invocation, optionally annotated with variables it defines.

    When ``reads_own_defs`` is true, the defined variables are also read
    (read-before-write semantics), like ``append`` which reads and then
    extends the variable's current value.

    ``reads`` lists bare variable names that the command reads without
    modification (e.g. ``info exists varName``, ``array get arrayName``).
    These are not ``$varName`` references in the args so the SSA scanner
    cannot detect them automatically.
    """

    range: Range
    command: str
    args: tuple[str, ...] = ()
    defs: tuple[str, ...] = ()
    reads: tuple[str, ...] = ()
    reads_own_defs: bool = False
    tokens: CommandTokens | None = None


@dataclass(frozen=True, slots=True)
class IRReturn:
    range: Range
    value: str | None = None


@dataclass(frozen=True, slots=True)
class IRBarrier:
    """A command whose side effects defeat static analysis.

    Commands like ``eval``, ``uplevel``, and ``upvar`` can modify
    arbitrary variables at runtime, so no constant propagation or
    dead-store reasoning can cross a barrier.  The ``reason`` field
    is a human-readable label for diagnostic messages; ``command``
    and ``args`` preserve the original call for passes that inspect
    specific barrier shapes (e.g. ``for`` inside a barrier block).
    """

    range: Range
    reason: str
    command: str = ""
    args: tuple[str, ...] = ()
    tokens: CommandTokens | None = None


@dataclass(frozen=True, slots=True)
class IRScript:
    statements: tuple["IRStatement", ...] = ()


@dataclass(frozen=True, slots=True)
class IRIfClause:
    condition: ExprNode
    condition_range: Range
    body: IRScript
    body_range: Range


@dataclass(frozen=True, slots=True)
class IRIf:
    range: Range
    clauses: tuple[IRIfClause, ...]
    else_body: IRScript | None = None
    else_range: Range | None = None


@dataclass(frozen=True, slots=True)
class IRFor:
    range: Range
    init: IRScript
    init_range: Range
    condition: ExprNode
    condition_range: Range
    next: IRScript
    next_range: Range
    body: IRScript
    body_range: Range
    raw_args: tuple[str, ...] = ()


@dataclass(frozen=True, slots=True)
class IRWhile:
    """A while loop: test an expression, execute body while true."""

    range: Range
    condition: ExprNode
    condition_range: Range
    body: IRScript
    body_range: Range
    raw_args: tuple[str, ...] = ()


@dataclass(frozen=True, slots=True)
class IRForeach:
    """Iterate over one or more lists, assigning elements to loop variables.

    Covers both ``foreach``, ``lmap``, and ``dict for``/``dict map``.
    Each ``(var_list, list_arg)`` pair in ``iterators`` corresponds to
    one varList/list argument group.
    """

    range: Range
    iterators: tuple[tuple[tuple[str, ...], str], ...]
    body: IRScript
    body_range: Range
    is_lmap: bool = False
    raw_args: tuple[str, ...] = ()
    is_dict_iteration: bool = False


@dataclass(frozen=True, slots=True)
class IRCatch:
    """``catch script ?resultVar? ?optionsVar?`` — evaluate body and trap exceptions."""

    range: Range
    body: IRScript
    body_range: Range
    result_var: str | None = None
    options_var: str | None = None
    raw_args: tuple[str, ...] = ()


@dataclass(frozen=True, slots=True)
class IRTryHandler:
    """One ``on``/``trap`` handler clause inside a ``try`` statement."""

    kind: str  # "on" or "trap"
    match_arg: str  # return code or error class pattern
    var_name: str | None  # variable bound to result
    options_var: str | None  # variable bound to options dict
    body: IRScript
    body_range: Range


@dataclass(frozen=True, slots=True)
class IRTry:
    """``try body ?on code varList body ...? ?finally body?`` — structured exception handling."""

    range: Range
    body: IRScript
    body_range: Range
    handlers: tuple[IRTryHandler, ...] = ()
    finally_body: IRScript | None = None
    finally_range: Range | None = None
    raw_args: tuple[str, ...] = ()


@dataclass(frozen=True, slots=True)
class IRSwitchArm:
    pattern: str
    pattern_range: Range
    body: IRScript | None
    body_range: Range | None
    fallthrough: bool = False


@dataclass(frozen=True, slots=True)
class IRSwitch:
    range: Range
    subject: str
    subject_range: Range
    arms: tuple[IRSwitchArm, ...] = ()
    default_body: IRScript | None = None
    default_range: Range | None = None
    mode: str = "exact"  # "exact", "glob", or "regexp"
    raw_args: tuple[str, ...] = ()  # original command args for generic fallback


@dataclass(frozen=True, slots=True)
class IRProcedure:
    name: str
    qualified_name: str
    params: tuple[str, ...]
    range: Range
    body: IRScript
    params_raw: str = ""
    body_source: str | None = None  # None for synthetic procs (``when``)
    namespace_scoped: bool = False  # True when defined inside namespace eval


@dataclass
class IRModule:
    top_level: IRScript = field(default_factory=IRScript)
    procedures: dict[str, IRProcedure] = field(default_factory=dict)
    redefined_procedures: set[str] = field(default_factory=set)


IRStatement = (
    IRAssignConst
    | IRAssignExpr
    | IRAssignValue
    | IRExprEval
    | IRIncr
    | IRCall
    | IRReturn
    | IRBarrier
    | IRIf
    | IRFor
    | IRWhile
    | IRForeach
    | IRCatch
    | IRTry
    | IRSwitch
)
