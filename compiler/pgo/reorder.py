"""Profile-guided branch-reordering suggestions.

Given a :class:`~compiler.pgo.profile_data.ProfileData`, find branch
constructs that are not tested hottest-first and suggest reordering them so
the most-frequently-taken branch is checked first (fewer comparisons on the
common path).  Two constructs are handled, each only when reordering is
provably behaviour-preserving:

**if / elseif equality-dispatch chains.**  Every clause condition must be
``<subject> eq|equals <constant>`` (string equality) with the **same**
subject expression and **distinct** constant values (so the clauses are
mutually exclusive — exactly one matches regardless of order), and the
subject must be **side-effect-free** (a variable read, or a command the
side-effect registry classifies as non-writing), since reordering changes
how many times it is evaluated.  Numeric ``==`` is intentionally excluded
(textually distinct numbers can be numerically equal — ``1`` == ``01``),
as are constants containing backslash escapes (whose runtime value is
ambiguous), so distinct source text always implies distinct values.

**exact-match switch arms.**  Reordering arms is *safer* than an if/elseif
chain: the subject is evaluated exactly once, so no purity requirement
applies.  Only ``-exact`` switches with **distinct, non-fallthrough**
patterns qualify (``-glob`` / ``-regexp`` arms are first-match-wins and may
overlap); a trailing ``default`` arm stays last.

Anything that does not fit is left untouched.  Suggestions are emitted as
``hint_only`` :class:`Optimisation` objects (code ``P100``) carrying a
materialised ``replacement`` so an opt-in ``--apply`` can rewrite the
source.  This module is **never** called by the default optimiser pipeline
— only by the explicit, off-by-default PGO entry points.
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
#: String-equality operators whose distinct *constant text* guarantees the
#: arms are mutually exclusive: ``eq`` (Tcl string ==) and ``equals``
#: (iRules).  Numeric ``==`` (``BinOp.EQ``) is **excluded**: textually
#: distinct numeric literals can be numerically equal (``1`` == ``01`` ==
#: ``1.0`` == ``0x1``), so reordering such arms could change which body
#: runs.  String comparison has no such collision (``1 eq 01`` is false).
_EQ_OPS = frozenset({BinOp.STR_EQ, BinOp.STR_EQUALS})

#: Dialect used for subject side-effect classification.  iRule getters
#: (``IP::remote_addr`` …) resolve here; the registry falls back to
#: ``get_any`` for plain Tcl commands, so this is safe for both.
_DIALECT = "f5-irules"


# ---------------------------------------------------------------------------
# IR traversal
# ---------------------------------------------------------------------------


def _iter_control(script: IRScript | None) -> Iterator[IRIf | IRSwitch]:
    """Yield every :class:`IRIf` / :class:`IRSwitch`, descending into bodies."""
    if script is None:
        return
    for stmt in script.statements:
        if isinstance(stmt, IRIf):
            yield stmt
            for clause in stmt.clauses:
                yield from _iter_control(clause.body)
            yield from _iter_control(stmt.else_body)
        elif isinstance(stmt, IRSwitch):
            yield stmt
            for arm in stmt.arms:
                yield from _iter_control(arm.body)
            yield from _iter_control(stmt.default_body)
        elif isinstance(stmt, (IRWhile, IRForeach, IRCatch, IRBlock, IRUpFrame)):
            yield from _iter_control(stmt.body)
        elif isinstance(stmt, IRFor):
            yield from _iter_control(stmt.init)
            yield from _iter_control(stmt.next)
            yield from _iter_control(stmt.body)
        elif isinstance(stmt, IRTry):
            yield from _iter_control(stmt.body)
            for handler in stmt.handlers:
                yield from _iter_control(handler.body)
            yield from _iter_control(stmt.finally_body)


def _iter_module_control(module: IRModule) -> Iterator[IRIf | IRSwitch]:
    """Yield every :class:`IRIf` / :class:`IRSwitch` across the whole module."""
    yield from _iter_control(module.top_level)
    for proc in module.procedures.values():
        yield from _iter_control(proc.body)
    for method in module.methods.values():
        yield from _iter_control(method.body)


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
    if len(text) >= 2 and (
        (text[0] == '"' and text[-1] == '"') or (text[0] == "{" and text[-1] == "}")
    ):
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


def _body_first_line(body: IRScript, body_range: Range | None) -> int | None:
    """1-based source line of *body*'s first statement."""
    for stmt in body.statements:
        rng = getattr(stmt, "range", None)
        if rng is not None:
            return rng.start.line + 1  # internal lines are 0-based
    if body_range is not None:
        return body_range.start.line + 1
    return None


def _body_signal_key(body: IRScript) -> tuple[str, str] | None:
    """Coarse attribution key — the first observable command / variable write."""
    for stmt in body.statements:
        if isinstance(stmt, IRCall):
            return ("cmd", stmt.command)
        if isinstance(stmt, (IRAssignConst, IRAssignValue, IRAssignExpr, IRIncr)):
            return ("var", normalise_var_name(stmt.name))
    return None


def _body_weight(body: IRScript, body_range: Range | None, profile: ProfileData) -> int | None:
    """Execution weight for *body*, preferring precise line counts."""
    if profile.has_line_data:
        line = _body_first_line(body, body_range)
        if line is not None:
            return profile.line_count(line)
    key = _body_signal_key(body)
    if key is None:
        return None
    kind, name = key
    if kind == "cmd":
        return profile.command_count(name)
    return profile.var_mod_counts.get(name, 0)


# ---------------------------------------------------------------------------
# Rewrite construction
# ---------------------------------------------------------------------------


def _is_word_char(ch: str) -> bool:
    return ch.isalnum() or ch == "_"


def _find_keyword(source: str, word: str, lo: int, hi: int) -> int:
    """Offset of the last whole-word *word* in ``source[lo:hi]``, or -1.

    "Whole word" means not flanked by identifier characters, so ``default``
    is matched but ``mydefault`` / ``defaults`` are not.
    """
    idx = source.rfind(word, lo, hi)
    while idx >= 0:
        before = source[idx - 1] if idx > 0 else ""
        after = source[idx + len(word)] if idx + len(word) < len(source) else ""
        if not _is_word_char(before) and not _is_word_char(after):
            return idx
        idx = source.rfind(word, lo, idx)
    return -1


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


def _rebuild_switch(switch: IRSwitch, order: list[int], source: str) -> str | None:
    """Reassemble a ``switch`` with arms in *order*, preserving formatting.

    Works in slots: the inter-element separators (whitespace/newlines) keep
    their positions and only the arm *texts* are permuted, so original
    indentation is preserved exactly.  ``default`` keeps the final slot.
    """
    switch_start = switch.range.start.offset
    switch_end = widen_range_for_closer(source, switch.range).end.offset

    # Source-order element spans: arms first, then default (if present).
    spans: list[tuple[int, int]] = []
    for arm in switch.arms:
        if arm.body_range is None:
            return None
        spans.append(
            (
                arm.pattern_range.start.offset,
                widen_range_for_closer(source, arm.body_range).end.offset,
            )
        )
    has_default = switch.default_body is not None and switch.default_range is not None
    if has_default:
        assert switch.default_range is not None
        # Locate the ``default`` keyword as a whole word, searching only the
        # gap between the previous element's body and the default body — so a
        # stray "default" substring inside an earlier arm (or unrelated text)
        # cannot be matched.
        window_lo = spans[-1][1] + 1 if spans else switch_start
        kw = _find_keyword(source, "default", window_lo, switch.default_range.start.offset)
        if kw < 0:
            return None
        spans.append((kw, widen_range_for_closer(source, switch.default_range).end.offset))

    # Spans must be within the switch and strictly increasing (no overlap).
    cursor = switch_start
    for start, end in spans:
        if not (switch_start <= start <= end <= switch_end) or start < cursor:
            return None
        cursor = end + 1

    texts = [source[start : end + 1] for start, end in spans]
    prefix = source[switch_start : spans[0][0]]
    suffix = source[spans[-1][1] + 1 : switch_end + 1]
    seps = [source[spans[i][1] + 1 : spans[i + 1][0]] for i in range(len(spans) - 1)]

    # Permute arm texts into the leading slots; default text stays last.
    new_texts = [texts[i] for i in order] + (texts[len(switch.arms) :] if has_default else [])

    out = [prefix]
    for i, text in enumerate(new_texts):
        out.append(text)
        if i < len(seps):
            out.append(seps[i])
    out.append(suffix)
    return "".join(out)


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
        # A backslash escape (e.g. "\x41") makes the runtime string value
        # ambiguous against another arm's plain text ("A"), so distinct
        # source text no longer proves distinct values — refuse to reorder.
        if "\\" in const.text:
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
        w = _body_weight(clause.body, clause.body_range, profile)
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


def _analyse_switch(switch: IRSwitch, source: str, profile: ProfileData) -> Optimisation | None:
    """Suggest reordering exact-match ``switch`` arms by profile frequency.

    Safer than an if/elseif chain: the subject is evaluated exactly once, so
    no purity requirement applies.  Only ``-exact`` switches with distinct,
    non-fallthrough patterns qualify (glob/regexp arms are first-match-wins
    and may overlap).  A ``default`` arm is, by construction, already last
    (the lowerer only treats a trailing ``default`` as the catch-all) and
    stays there.
    """
    # Only exact matching is order-independent for distinct patterns.
    if switch.mode != "exact":
        return None
    arms = switch.arms
    if len(arms) < 2:
        return None
    # Fallthrough (``pat -``) chains share a body — reordering breaks them.
    if any(arm.fallthrough or arm.body is None for arm in arms):
        return None
    # A backslash escape in a pattern makes its runtime value ambiguous (it
    # depends on whether the arm list was braced), so distinct text no
    # longer proves distinct match values — refuse to reorder.
    if any("\\" in arm.pattern for arm in arms):
        return None
    # Distinct patterns (case-normalised under -nocase) ⇒ mutually exclusive.
    patterns = [arm.pattern.lower() if switch.nocase else arm.pattern for arm in arms]
    if len(set(patterns)) != len(patterns):
        return None

    weights: list[int] = []
    for arm in arms:
        assert arm.body is not None  # guaranteed by the fallthrough guard
        w = _body_weight(arm.body, arm.body_range, profile)
        if w is None:
            return None
        weights.append(w)

    if sum(1 for w in weights if w > 0) < 2:
        return None

    order = sorted(range(len(arms)), key=lambda i: (-weights[i], i))
    if order == list(range(len(arms))):
        return None  # already hottest-first

    replacement = _rebuild_switch(switch, order, source)
    if replacement is None:
        return None

    hot_idx = order[0]
    message = (
        f"Reorder switch arms by profile frequency: the arm matching "
        f"{arms[hot_idx].pattern!r} is taken most often ({weights[hot_idx]}×) but "
        f"is in position {hot_idx + 1}; place the hottest arm first."
    )
    return Optimisation(
        code=REORDER_CODE,
        message=message,
        range=Range(
            start=switch.range.start,
            end=widen_range_for_closer(source, switch.range).end,
        ),
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
    for node in _iter_module_control(cu.ir_module):
        if isinstance(node, IRIf):
            opt = _analyse_if(node, source, profile)
        else:
            opt = _analyse_switch(node, source, profile)
        if opt is not None:
            suggestions.append(opt)
    suggestions.sort(key=lambda o: o.range.start.offset)
    return suggestions
