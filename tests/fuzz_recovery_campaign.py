"""Standalone fuzzing campaign for the single recovering-detection pass.

Run directly (not under pytest) for a large, long-running campaign:

    uv run python tests/fuzz_recovery_campaign.py --cases 200000
    uv run python tests/fuzz_recovery_campaign.py --cases 1000000 --seed 0

It hammers the recovery path with several generators and asserts, for every
input:

  1. tokens  — tokenise_recovering(src) == tokenise(src, vi=compute_virtual_insertions(src))
  2. warnings — same, position + message
  3. diagnostics — segment_with_recovery(src) never raises and never returns two
                   exact-duplicate diagnostics (the dedup invariant)

Any divergence is shrunk (delta-debugging on lines, then characters) to a
minimal reproducer and printed.  Exit code is non-zero on the first failure so
the campaign can gate CI or a nightly run.
"""

from __future__ import annotations

import argparse
import random
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from compiler.parsing.green_tree import tokenise
from compiler.parsing.recovering_lexer import tokenise_recovering
from compiler.parsing.recovery import compute_virtual_insertions, segment_with_recovery


def _tok_key(tokens):
    return [(t.type.name, t.start.offset, t.end.offset, t.text, bool(t.in_quote)) for t in tokens]


def _warn_key(warnings):
    return [(w[0].offset, w[1]) for w in (warnings or [])]


def _diag_key(diags):
    out = []
    for d in diags or []:
        fixes = tuple(
            (f.range.start.offset, f.range.end.offset, f.new_text, f.description)
            for f in (d.fixes or ())
        )
        out.append((d.code, d.severity, d.range.start.offset, d.range.end.offset, d.message, fixes))
    return out


def check(src: str) -> str | None:
    """Return a description of the first divergence for *src*, or None if OK."""
    vi = compute_virtual_insertions(src) or None
    old_toks, old_warns = tokenise(src, 0, 0, 0, virtual_insertions=vi)
    new_toks, new_warns = tokenise_recovering(src)
    if _tok_key(new_toks) != _tok_key(old_toks):
        return f"TOKENS\n  old={_tok_key(old_toks)}\n  new={_tok_key(new_toks)}"
    if _warn_key(new_warns) != _warn_key(old_warns):
        return f"WARNINGS\n  old={_warn_key(old_warns)}\n  new={_warn_key(new_warns)}"
    try:
        _c, seg_diags = segment_with_recovery(src)
    except Exception as exc:  # noqa: BLE001 — campaign reports, never swallows silently
        return f"DIAGNOSTICS raised {type(exc).__name__}: {exc}"
    keys = _diag_key(seg_diags)
    if len(keys) != len(set(keys)):
        dups = [k for k in keys if keys.count(k) > 1]
        return f"DUPLICATE DIAGNOSTICS\n  {dups}"
    return None


# --- generators --------------------------------------------------------------

_KNOWN = [
    "set a 1",
    "proc p {} {}",
    "puts hi",
    "if {1} {}",
    "return",
    "expr {1 + 2}",
    "namespace eval n {}",
    "foreach x $l {}",
    "while {1} {}",
    "catch {cmd} e",
    "for {set i 0} {$i<3} {incr i} {}",
]
_OPENERS = ["{", "[", '"', "set x {", "puts [", 'set y "', "if {", "proc q {", "[a [b", '"$x']
_NOISE = ["}", "]", '"', "# comment", "x", "$v", "${a b}", ";", "  ", "a b c", "\t", "\\", "\\\n"]
_DELIMS = '{}[]"$;\\#'
_SOUP = 'set proc if {} [] $x "q" ; \n# c\nputs expr { } [ ] " $ x 1 } ] \\\t'


def gen_recovery(rng):
    parts = []
    for _ in range(rng.randint(1, 18)):
        r = rng.random()
        parts.append(
            rng.choice(_OPENERS)
            if r < 0.4
            else rng.choice(_KNOWN)
            if r < 0.8
            else rng.choice(_NOISE)
        )
    return "\n".join(parts) + ("\n" if rng.random() < 0.7 else "")


def gen_soup(rng):
    return "".join(rng.choice(_SOUP) for _ in range(rng.randint(0, 160)))


def gen_delim_storm(rng):
    return "".join(rng.choice(_DELIMS + "abc \n") for _ in range(rng.randint(0, 80)))


def gen_nested(rng):
    # Deeply nested openers with sporadic closers — stresses level tracking.
    depth = rng.randint(1, 6)
    s = "".join(rng.choice(["[", "{", '"', "[a ", "{b "]) for _ in range(depth))
    s += rng.choice(["", "\n" + rng.choice(_KNOWN), "\n#c", "\nputs x"])
    s += "".join(rng.choice(["]", "}", '"', ""]) for _ in range(rng.randint(0, depth)))
    return s + "\n"


def gen_mutate(rng):
    # Take a clean snippet and delete a delimiter to make it broken.
    base = "\n".join(rng.sample(_KNOWN, rng.randint(1, len(_KNOWN))))
    if not base:
        return base
    i = rng.randrange(len(base))
    return base[:i] + base[i + 1 :] + "\n"


GENERATORS = [gen_recovery, gen_soup, gen_delim_storm, gen_nested, gen_mutate]


# --- shrinking ---------------------------------------------------------------


def shrink(src: str) -> str:
    """Delta-debug *src* to a smaller string that still diverges."""
    changed = True
    while changed:
        changed = False
        # Drop a line.
        lines = src.split("\n")
        for i in range(len(lines)):
            cand = "\n".join(lines[:i] + lines[i + 1 :])
            if cand != src and check(cand):
                src, changed = cand, True
                break
        if changed:
            continue
        # Drop a character.
        for i in range(len(src)):
            cand = src[:i] + src[i + 1 :]
            if cand != src and check(cand):
                src, changed = cand, True
                break
    return src


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--cases", type=int, default=200_000)
    ap.add_argument("--seed", type=int, default=0xC0FFEE)
    args = ap.parse_args()

    rng = random.Random(args.seed)
    start = time.time()
    recovered = 0
    diag_cases = 0
    for n in range(1, args.cases + 1):
        src = rng.choice(GENERATORS)(rng)
        if compute_virtual_insertions(src):
            recovered += 1
        _c, seg_diags = segment_with_recovery(src)
        if seg_diags:
            diag_cases += 1
        why = check(src)
        if why is not None:
            minimal = shrink(src)
            print(f"\n!!! DIVERGENCE after {n} cases (seed {args.seed})")
            print(f"input:   {src!r}")
            print(f"minimal: {minimal!r}")
            print(check(minimal) or why)
            return 1
        if n % 20_000 == 0:
            rate = n / (time.time() - start)
            print(
                f"  {n:>9,} ok  | recovered={recovered:,} diag={diag_cases:,} | {rate:,.0f}/s",
                flush=True,
            )

    dt = time.time() - start
    print(
        f"\nPASS: {args.cases:,} cases in {dt:.1f}s "
        f"({args.cases / dt:,.0f}/s) | recovery-triggering={recovered:,} "
        f"diagnostic-bearing={diag_cases:,}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
