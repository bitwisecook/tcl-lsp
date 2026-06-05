"""PROTOTYPE: does an offset-based bracket-level delta index predict recovery?

Captures bracket-nesting deltas during the *first* lex (``TclLexer`` with
``track_levels=True``) and asks the one question the re-lex currently exists to
answer: when the ``[`` recovery heuristic wants to insert ``]`` at offset off,
does that closer actually balance the outermost open ``[`` (so the command
closes at off) or not (so it over-runs to EOF — the nested case)?

Prediction (no re-lex):   level_at(off) == 1     [prefix sum of deltas before off]
Ground truth (two-pass):  the recovered CMD anchored at the outer ``[`` ends
                          before EOF (closed) vs reaches EOF (over-ran).

If the prediction matches the two-pass outcome across the fuzz corpus, the
delta index is a faithful, re-lex-free substitute for the balance decision.

    uv run python tests/proto_level_index.py
"""

from __future__ import annotations

import random
import sys
from bisect import bisect_left
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from compiler.parsing.green_tree import tokenise
from compiler.parsing.lexer import TclLexer
from compiler.parsing.recovery import compute_virtual_insertions

sys.path.insert(0, str(Path(__file__).resolve().parent))
from test_recovering_lexer_differential import (  # noqa: E402
    _rand_recovery_source,
    _rand_soup_source,
)


def state_index(source: str):
    """Return (snapshots, inert) — structural snapshots + inert (escaped/${}) spans."""
    lx = TclLexer(source, track_levels=True)
    lx.tokenise_all()
    return sorted(lx._levels), sorted(lx._inert)


def _in_inert(inert, offset: int) -> bool:
    for start, end in inert:
        if start <= offset < end:
            return True
        if start > offset:
            break
    return False


def state_at(snapshots, offset: int):
    """Structural state in effect at *offset*: the last snapshot before it."""
    i = bisect_left(snapshots, (offset,))  # snapshots strictly before offset
    if i == 0:
        return (0, 0, 0, 0, 0)  # top level
    return snapshots[i - 1]


def closes_outer_bracket(snapshots, inert, offset: int) -> bool:
    """Does inserting ``]`` at *offset* eventually close the outermost ``[``?

    From the structural state at off alone, then a forward walk of the (sparse)
    delta stream — NO character re-lex:

    * The inserted ``]`` is inert if off is inside a quote/brace (matching the
      lexer's ``blevel == 0 and not in_quotes`` guard), or inside an escaped /
      ``${...}`` span -> never closes.
    * Otherwise it drops the running bracket level by one.  If that reaches 0 the
      command closes at off; else a *later* real ``]`` can close it once the
      insertion has rebalanced — the first ``]`` transition whose (shifted) level
      returns to 0 outside any quote/brace.  If none, the command over-runs.
    """
    if _in_inert(inert, offset):
        return False
    _pos, level, blevel, in_quote, _bd = state_at(snapshots, offset)
    if blevel != 0 or in_quote:
        return False
    if level - 1 == 0:
        return True
    # Walk forward: the persistent -1 from the insertion means the outer closes
    # at the first ``]`` (bracket delta -1) whose recorded level is 1, outside a
    # quote/brace.
    i = bisect_left(snapshots, (offset + 1,))
    for _p, lvl, blv, inq, bd in snapshots[i:]:
        if bd == -1 and blv == 0 and not inq and lvl == 1:
            return True
    return False


def main() -> int:
    seeds = [1, 7, 42, 1234, 0xC0FFEE, 3, 99]
    checked = match = 0
    mismatches = []
    for seed in seeds:
        rng = random.Random(seed)
        for _ in range(20000):
            src = _rand_recovery_source(rng) if rng.random() < 0.6 else _rand_soup_source(rng)
            vi = compute_virtual_insertions(src)
            # Bracket-only, single-insertion cases — the prototype's scope.
            if len(vi) != 1:
                continue
            (off, ch), = vi.items()
            if ch != "]":
                continue

            # The outer '[' of the unterminated command, from the bare parse.
            bare, _ = tokenise(src, 0, 0, 0)
            unterm = [
                t for t in bare
                if t.type.name == "CMD" and t.end.offset >= len(src) - 1
            ]
            if not unterm:
                continue
            outer_b = unterm[-1].start.offset  # the '[' position

            snapshots, inert = state_index(src)
            predicted_close = closes_outer_bracket(snapshots, inert, off)

            # Ground truth: two-pass with the insertion applied.
            recovered, _ = tokenise(src, 0, 0, 0, virtual_insertions=vi)
            anchored = [t for t in recovered if t.type.name == "CMD" and t.start.offset == outer_b]
            # Closed iff that command no longer reaches EOF.
            actual_close = bool(anchored) and anchored[0].end.offset < len(src) - 1

            checked += 1
            if predicted_close == actual_close:
                match += 1
            elif len(mismatches) < 8:
                mismatches.append(
                    (src, off, state_at(snapshots, off), predicted_close, actual_close)
                )

    print(f"checked={checked}  match={match}  mismatch={checked - match}")
    for src, off, st, pred, act in mismatches:
        print(f"\n  src={src!r}\n  off={off} state(pos,lvl,blvl,inq)={st} pred={pred} actual={act}")
    return 0 if match == checked else 1


if __name__ == "__main__":
    raise SystemExit(main())
