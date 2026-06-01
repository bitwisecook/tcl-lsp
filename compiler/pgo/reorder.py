"""Profile-guided branch-reordering suggestions.

This is the first profile-guided optimisation: given a
:class:`~compiler.pgo.profile_data.ProfileData`, find ``if`` / ``elseif``
**equality-dispatch chains** whose branches are not tested
hottest-first, and suggest reordering them so the most-frequently-taken
branch is checked first (fewer comparisons on the common path).

Safety — a chain is only ever touched when reordering is provably
behaviour-preserving:

* Every clause condition is ``<subject> eq|==|equals <constant>`` with the
  **same** subject expression and **distinct** constants, so the clauses
  are mutually exclusive — exactly one can match regardless of order.
* The subject is **side-effect-free** (a variable read, or a command the
  side-effect registry classifies as non-writing), so evaluating it a
  different number of times across the reordered tests changes nothing
  observable.

Anything that does not fit (mixed operators, a side-effecting subject,
repeated constants, ``glob`` / range tests, fewer than two ``elseif``
clauses) is left untouched.

Suggestions are emitted as ``hint_only`` :class:`Optimisation` objects
(code ``P100``) carrying a materialised ``replacement`` so an opt-in
``--apply`` can rewrite the chain.  This module is **never** called by the
default optimiser pipeline — only by the explicit, off-by-default PGO
entry points.
"""

from __future__ import annotations

import logging
from collections.abc import Iterator

from shared.diagnostic import Range
from shared.naming import normalise_var_name
from shared.ranges import widen_range_for_closer

from ..compilation_unit import CompilationUnit, ensure_compilation_unit
from ..expr_ast import (
    BinOp,
    ExprBinary,
    ExprCommand,
    ExprLiteral,
    ExprNode,
    ExprString,
    ExprVar,
)
from ..ir import (
    IRAssignConst,
    IRAssignExpr,
    IRAssignValue,
    IRBlock,
    IRCall,
    IRCatch,
    IRFor,
    IRForeach,
    IRIf,
    IRIfClause,
    IRIncr,
    IRModule,
    IRScript,
    IRSwitch,
    IRTry,
    IRUpFrame,
    IRWhile,
)
from ..optimiser._types import Optimisation
from ..side_effects import classify_side_effects
from .profile_data import ProfileData

log = logging.getLogger(__name__)

#: The PGO suggestion code (registered in ``shared/codes.py`` as a PGO-kind
#: code so it stays out of the optimisation profiles).
REORDER_CODE = "P100"

#: Equality operators that form a mutually-exclusive dispatch on distinct
#: constants: ``==`` (numeric), ``eq`` (string), ``equals`` (iRules).
_EQ_OPS = frozenset({BinOp.EQ, BinOp.STR_EQ, BinOp.STR_EQUALS})

#: Dialect used for subject side-effect classification.  iRule getters
#: (``IP::remote_addr`` …) resolve here; the registry falls back to
#: ``get_any`` for plain Tcl commands, so this is safe for both.
_DIALECT = "f5-irules"


# ---------------------------------------------------------------------------
# IR traversal
# ---------------------------------------------------------------------------


def _iter_ifs(script: IRScript | None) -> Iterator[IRIf]:
    """Yield every :class:`IRIf` in *script*, descending into nested bodies."""
    if script is None:
        return
    for stmt in script.statements:
        if isinstance(stmt, IRIf):
            yield stmt
            for clause in stmt.clauses:
                yield from _iter_ifs(clause.body)
            yield from _iter_ifs(stmt.else_body)
        elif isinstance(stmt, (IRWhile, IRForeach, IRCatch, IRBlock, IRUpFrame)):
            yield from _iter_ifs(stmt.body)
        elif isinstance(stmt, IRFor):
            yield from _iter_ifs(stmt.init)
            yield from _iter_ifs(stmt.next)
            yield from _iter_ifs(stmt.body)
        elif isinstance(stmt, IRTry):
            yield from _iter_ifs(stmt.body)
            for handler in stmt.handlers:
                yield from _iter_ifs(handler.body)
            yield from _iter_ifs(stmt.finally_body)
        elif isinstance(stmt, IRSwitch):
            for arm in stmt.arms:
                yield from _iter_ifs(arm.body)
            yield from _iter_ifs(stmt.default_body)


def _iter_module_ifs(module: IRModule) -> Iterator[IRIf]:
    """Yield every :class:`IRIf` across the whole module."""
    yield from _iter_ifs(module.top_level)
    for proc in module.procedures.values():
        yield from _iter_ifs(proc.body)
    for method in module.methods.values():
        yield from _iter_ifs(method.body)


# ---------------------------------------------------------------------------
# Equality-dispatch recognition
# ---------------------------------------------------------------------------


_Subject = ExprVar | ExprCommand
_Const = ExprString | ExprLiteral


def _split_eq(cond: ExprNode) -> tuple[_Subject, _Const] | None:
    """Return ``(subject, const)`` if *cond* is ``subject <eq> const``."""
    if not isinstance(cond, ExprBinary) or cond.op not in _EQ_OPS:
        return None
    left, right = cond.left, cond.right
    if isinstance(right, (ExprString, ExprLiteral)) and isinstance(left, (ExprVar, ExprCommand)):
        return left, right
    if isinstance(left, (ExprString, ExprLiteral)) and isinstance(right, (ExprVar, ExprCommand)):
        return right, left
    return None


def _subject_key(node: _Subject) -> str:
    """Normalised identity of a subject, for the same-subject check."""
    if isinstance(node, ExprVar):
        return "var:" + normalise_var_name(node.name)
    # ExprCommand — collapse internal whitespace so spacing differences
    # between clauses don't defeat the equality test.
    return "cmd:" + " ".join(node.text.split())


def _const_value(node: _Const) -> str:
    """The constant's value text, stripped of one layer of quotes/braces."""
    text = node.text
    if len(text) >= 2 and ((text[0] == '"' and text[-1] == '"') or (text[0] == "{" and text[-1] == "}")):
        return text[1:-1]
    return text


def _subject_is_safe(node: _Subject) -> bool:
    """True when evaluating *node* repeatedly has no observable effect."""
    if isinstance(node, ExprVar):
        return True  # reading a variable is side-effect-free
    inner = node.text.strip()
    if inner.startswith("[") and inner.endswith("]"):
        inner = inner[1:-1].strip()
    # A nested command substitution could hide a side effect the registry
    # cannot see — stay conservative.
    if "[" in inner:
        return False
    from compiler.parsing.command_segmenter import segment_commands

    try:
        cmds = segment_commands(inner)
    except Exception:
        return False
    if len(cmds) != 1 or not cmds[0].texts:
        return False
    command = cmds[0].texts[0]
    if "$" in command:  # dynamic command name
        return False
    args = tuple(cmds[0].texts[1:])
    effects = classify_side_effects(command, args, dialect=_DIALECT)
    return not effects.writes_any and not effects.dynamic_barrier


# ---------------------------------------------------------------------------
# Profile attribution
# ---------------------------------------------------------------------------


def _first_line(clause: IRIfClause) -> int | None:
    """1-based source line of the clause body's first statement."""
    for stmt in clause.body.statements:
        rng = getattr(stmt, "range", None)
        if rng is not None:
            return rng.start.line + 1  # internal lines are 0-based
    if clause.body_range is not None:
        return clause.body_range.start.line + 1
    return None


def _signal_key(clause: IRIfClause) -> tuple[str, str] | None:
    """Coarse attribution key — the first observable command / variable write."""
    for stmt in clause.body.statements:
        if isinstance(stmt, IRCall):
            return ("cmd", stmt.command)
        if isinstance(stmt, (IRAssignConst, IRAssignValue, IRAssignExpr, IRIncr)):
            return ("var", normalise_var_name(stmt.name))
    return None


def _clause_weight(clause: IRIfClause, profile: ProfileData) -> int | None:
    """Execution weight for *clause*, preferring precise line counts."""
    if profile.has_line_data:
        line = _first_line(clause)
        if line is not None:
            return profile.line_count(line)
    key = _signal_key(clause)
    if key is None:
        return None
    kind, name = key
    if kind == "cmd":
        return profile.command_count(name)
    return profile.var_mod_counts.get(name, 0)


# ---------------------------------------------------------------------------
# Rewrite construction
# ---------------------------------------------------------------------------


def _slice(source: str, rng: Range | None) -> str | None:
    """Verbatim source for *rng*, including a trailing brace/bracket closer."""
    if rng is None:
        return None
    widened = widen_range_for_closer(source, rng)
    start = widened.start.offset
    end = widened.end.offset + 1  # Range end offsets are inclusive
    if 0 <= start <= end <= len(source):
        return source[start:end]
    return None


def _rebuild_if(if_node: IRIf, order: list[int], source: str) -> str | None:
    """Reassemble the ``if`` chain with clauses in *order* (else stays last)."""
    parts: list[str] = []
    for position, idx in enumerate(order):
        clause = if_node.clauses[idx]
        cond_txt = _slice(source, clause.condition_range)
        body_txt = _slice(source, clause.body_range)
        if cond_txt is None or body_txt is None:
            return None
        keyword = "if " if position == 0 else " elseif "
        parts.append(f"{keyword}{cond_txt} {body_txt}")
    if if_node.else_body is not None:
        else_txt = _slice(source, if_node.else_range)
        if else_txt is None:
            return None
        parts.append(f" else {else_txt}")
    return "".join(parts)


def _replace_range(if_node: IRIf, source: str) -> Range:
    """Range covering the whole ``if`` statement (through the final closer)."""
    last = (
        if_node.else_range
        if if_node.else_body is not None and if_node.else_range is not None
        else if_node.clauses[-1].body_range
    )
    if last is not None:
        end = widen_range_for_closer(source, last).end
    else:
        end = if_node.range.end
    return Range(start=if_node.range.start, end=end)


# ---------------------------------------------------------------------------
# Analysis
# ---------------------------------------------------------------------------


def _analyse_if(if_node: IRIf, source: str, profile: ProfileData) -> Optimisation | None:
    clauses = if_node.clauses
    # Need at least two conditions to have any reordering to suggest.
    if len(clauses) < 2:
        return None

    subjects: list[str] = []
    consts: list[str] = []
    for clause in clauses:
        split = _split_eq(clause.condition)
        if split is None:
            return None
        subject, const = split
        if not _subject_is_safe(subject):
            return None
        subjects.append(_subject_key(subject))
        consts.append(_const_value(const))

    # Same subject across all clauses, and distinct constants → mutually
    # exclusive → free to reorder.
    if len(set(subjects)) != 1:
        return None
    if len(set(consts)) != len(consts):
        return None

    weights: list[int] = []
    for clause in clauses:
        w = _clause_weight(clause, profile)
        if w is None:
            return None
        weights.append(w)

    # Need at least two clauses with a positive signal to rank meaningfully.
    if sum(1 for w in weights if w > 0) < 2:
        return None

    order = sorted(range(len(clauses)), key=lambda i: (-weights[i], i))
    if order == list(range(len(clauses))):
        return None  # already hottest-first

    replacement = _rebuild_if(if_node, order, source)
    if replacement is None:
        return None

    hot_idx = order[0]
    message = (
        f"Reorder branches by profile frequency: the branch matching "
        f"{consts[hot_idx]!r} is taken most often ({weights[hot_idx]}×) but is "
        f"tested in position {hot_idx + 1}; test the hottest branch first."
    )
    return Optimisation(
        code=REORDER_CODE,
        message=message,
        range=_replace_range(if_node, source),
        replacement=replacement,
        hint_only=True,
    )


def find_pgo_suggestions(
    source: str,
    profile: ProfileData | None,
    cu: CompilationUnit | None = None,
) -> list[Optimisation]:
    """Return profile-guided reordering suggestions for *source*.

    Off-by-default by construction: returns ``[]`` when *profile* is empty,
    and is never invoked by the standard optimiser pipeline.
    """
    if profile is None or profile.is_empty:
        return []
    cu = ensure_compilation_unit(source, cu, logger=log, context="pgo")
    if cu is None:
        return []
    suggestions: list[Optimisation] = []
    for if_node in _iter_module_ifs(cu.ir_module):
        opt = _analyse_if(if_node, source, profile)
        if opt is not None:
            suggestions.append(opt)
    suggestions.sort(key=lambda o: o.range.start.offset)
    return suggestions
