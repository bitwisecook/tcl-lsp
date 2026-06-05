"""PROTOTYPE: a parallel structural level-index for the expr sublanguage.

The third leg.  An ``ArgRole.EXPR`` brace (``if {$x > (1 + 2``) holds a braced
*expression*, whose recoverable delimiter is the paren — a level the script-side
index doesn't model.  The expr lexer already emits ``PAREN_OPEN`` / ``PAREN_CLOSE``
as discrete tokens and reads ``[...]`` / ``"..."`` / ``${...}`` as opaque whole
tokens, so the structural index falls straight out of the token stream — no
instrumentation, and opaque spans (where an inserted ``)`` is inert) are free.

Same "does it predict?" shape as proto_level_index: from the token-derived paren
index alone (binary search + a forward walk of the sparse paren deltas, NO
re-lex) predict whether inserting ``)`` at offset O closes the outermost open
``(``.  Ground truth: physically insert ``)`` at O, re-lex, and stack-match the
parens to see if the original outermost ``(`` is now closed.

    uv run python tests/proto_expr_level_index.py
"""

from __future__ import annotations

import random
import sys
from bisect import bisect_left
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from compiler.parsing.expr_lexer import ExprTokenType, tokenise_expr

_OPAQUE = (ExprTokenType.COMMAND, ExprTokenType.STRING, ExprTokenType.VARIABLE)


_CLOSER = {
    ExprTokenType.COMMAND: "]",
    ExprTokenType.STRING: '"',
    ExprTokenType.VARIABLE: ")",  # only relevant for $arr(idx)
}


def _opaque_unterminated(t, n: int) -> bool:
    """An opaque token that ran to EOF without its closer — still 'open'."""
    if t.end < n - 1:
        return False
    text = t.text
    if t.type is ExprTokenType.VARIABLE:
        return "(" in text and not text.endswith(")")
    return not (len(text) >= 2 and text.endswith(_CLOSER[t.type]))


def paren_index(source: str):
    """(deltas, opaque) — sorted paren snapshots (pos, depth_after, ±1) + opaque spans."""
    toks = tokenise_expr(source)
    n = len(source)
    deltas = []
    opaque = []
    depth = 0
    for t in toks:
        if t.type is ExprTokenType.PAREN_OPEN:
            depth += 1
            deltas.append((t.start, depth, 1))
        elif t.type is ExprTokenType.PAREN_CLOSE:
            # An extra ``)`` with nothing open closes nothing (matches the stack
            # semantics) — don't let the running depth go negative.
            if depth > 0:
                depth -= 1
                deltas.append((t.start, depth, -1))
        elif t.type in _OPAQUE:
            # An unterminated opaque token swallows everything to EOF: an inserted
            # closer there is absorbed by it, not by the expr paren.
            end = n if _opaque_unterminated(t, n) else t.end
            opaque.append((t.start, end))
    return deltas, sorted(opaque)


def _in_opaque(opaque, off: int) -> bool:
    for start, end in opaque:
        if start <= off <= end:
            return True
        if start > off:
            break
    return False


def depth_at(deltas, off: int) -> int:
    i = bisect_left(deltas, (off, -(1 << 30), -(1 << 30)))
    return deltas[i - 1][1] if i > 0 else 0


def closes_outer_paren(deltas, opaque, off: int) -> bool:
    """Does inserting ``)`` at *off* eventually close the outermost ``(``?"""
    if _in_opaque(opaque, off):
        return False
    level = depth_at(deltas, off)
    if level < 1:
        return False
    if level - 1 == 0:
        return True
    # Forward walk: with the persistent -1 from the insertion, the outer closes
    # at the first ``)`` whose recorded depth is 1, outside any opaque span.
    i = bisect_left(deltas, (off + 1, -(1 << 30), -(1 << 30)))
    for pos, d, delta in deltas[i:]:
        if delta == -1 and d == 1 and not _in_opaque(opaque, pos):
            return True
    return False


def unmatched_opens(source: str) -> list[int]:
    """Positions of unmatched ``(`` in *source* (stack matching over paren tokens)."""
    stack: list[int] = []
    for t in tokenise_expr(source):
        if t.type is ExprTokenType.PAREN_OPEN:
            stack.append(t.start)
        elif t.type is ExprTokenType.PAREN_CLOSE and stack:
            stack.pop()
    return stack


# --- corpus ------------------------------------------------------------------

_OPERAND = ["$x", "1", "2.5", "$arr(k)", "[llength $l]", '"str"', "sin", "${y}", "true"]
_OP = ["+", "-", "*", "==", "<", "&&", "||", "<=", "ne", "eq"]
_ATOM = ["(", ")", *_OPERAND, *_OP, " ", ",", "?", ":"]


def _rand_expr(rng):
    n = rng.randint(1, 22)
    return "".join(rng.choice(_ATOM) for _ in range(n))


def _rand_unbalanced(rng):
    # Bias toward open parens so we get unterminated exprs to recover.
    parts = []
    for _ in range(rng.randint(2, 14)):
        r = rng.random()
        parts.append(
            "(" if r < 0.32 else rng.choice(_OPERAND) if r < 0.7 else rng.choice(_OP)
            if r < 0.88 else ")"
        )
    return " ".join(parts)


def main() -> int:
    rng = random.Random(0xE)
    checked = match = 0
    mismatches = []
    for _ in range(60000):
        src = _rand_unbalanced(rng) if rng.random() < 0.6 else _rand_expr(rng)
        opens = unmatched_opens(src)
        if not opens:
            continue
        outer = opens[0]
        deltas, opaque = paren_index(src)
        # Candidate close offsets: each whitespace gap after the outer open, plus EOF.
        candidates = [i for i, c in enumerate(src) if c == " " and i > outer]
        candidates.append(len(src))
        for off in candidates:
            predicted = closes_outer_paren(deltas, opaque, off)
            # Ground truth: physically insert ) at off, does the original outer
            # open (still at `outer`, since outer < off) become matched?
            recovered = src[:off] + ")" + src[off:]
            actual = outer not in unmatched_opens(recovered)
            checked += 1
            if predicted == actual:
                match += 1
            elif len(mismatches) < 8:
                mismatches.append((src, off, depth_at(deltas, off), predicted, actual))

    print(f"checked={checked}  match={match}  mismatch={checked - match}")
    for src, off, lvl, pred, act in mismatches:
        print(f"\n  src={src!r}\n  off={off} depth={lvl} predicted={pred} actual={act}")
    return 0 if match == checked else 1


if __name__ == "__main__":
    raise SystemExit(main())
