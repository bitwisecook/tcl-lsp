"""Differential oracle: single-pass recovering tokeniser == two-pass recovery.

The recovering tokeniser (``compiler.parsing.recovering_lexer``) must produce a
stream identical to the established detect-then-reparse recovery
(``compute_virtual_insertions`` + ``tokenise(virtual_insertions=...)``) — token
for token (type, span, text, quote flag) **and** warning for warning.  The
warnings matter as much as the tokens: they are the ``missing "`` /
``missing close-brace`` signals the analyser turns into E2xx diagnostics, and the
two-pass keeps an over-running token + warning whenever a heuristic declines, so
the single pass must reproduce that too.

This file is a **deterministic** safety net: curated cases per recovery
heuristic, exact per-heuristic diagnostic characterization, and the minimal
repros the offline fuzzer has surfaced (``TestPromotedFuzzFindings``).  Live
random fuzzing is *not* run here — it runs offline via ``make
test-fuzz-recovery`` (``tests/fuzz_recovery_campaign.py``), and any divergence it
shrinks is promoted into a deterministic case here or in
``tests/test_recovery_edge_cases.py``.  That keeps CI fast and reproducible while
the campaign keeps exploring.
"""

from __future__ import annotations

import random
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from compiler.parsing.green_tree import tokenise
from compiler.parsing.recovering_lexer import tokenise_recovering
from compiler.parsing.recovery import compute_virtual_insertions, segment_with_recovery


def _two_pass(source: str):
    """The current, golden recovery: detect virtual insertions, then re-tokenise."""
    vi = compute_virtual_insertions(source) or None
    return tokenise(source, 0, 0, 0, virtual_insertions=vi)


def _tok_key(tokens) -> list[tuple]:
    return [(t.type.name, t.start.offset, t.end.offset, t.text, bool(t.in_quote)) for t in tokens]


def _warn_key(warnings) -> list[tuple]:
    return [(w[0].offset, w[1]) for w in (warnings or [])]


def _diag_key(diags) -> list[tuple]:
    """Compare diagnostics by everything the LSP renders, including quick-fixes."""
    out = []
    for d in diags or []:
        fixes = tuple(
            (
                f.range.start.offset,
                f.range.end.offset,
                f.new_text,
                f.description,
            )
            for f in (d.fixes or ())
        )
        out.append(
            (
                d.code,
                d.severity,
                d.range.start.offset,
                d.range.end.offset,
                d.message,
                fixes,
            )
        )
    return out


def _assert_equivalent(source: str) -> None:
    old_toks, old_warns = _two_pass(source)
    new_toks, new_warns = tokenise_recovering(source)
    assert _tok_key(new_toks) == _tok_key(old_toks), (
        f"token streams diverged for {source!r}\n"
        f"  two-pass:    {_tok_key(old_toks)}\n"
        f"  single-pass: {_tok_key(new_toks)}"
    )
    assert _warn_key(new_warns) == _warn_key(old_warns), (
        f"warnings diverged for {source!r}\n"
        f"  two-pass:    {_warn_key(old_warns)}\n"
        f"  single-pass: {_warn_key(new_warns)}"
    )


def _assert_no_duplicate_diagnostics(source: str) -> None:
    """The recovery diagnostic set must never contain exact duplicates.

    The first parse and the recovered re-parse can each emit the same lexer
    warning for an unchanged region; ``assemble_recovery_diagnostics`` dedupes
    them so the editor never shows two identical squiggles.  This fuzzes that
    invariant over the whole analyser recovery path.
    """
    _cmds, diags = segment_with_recovery(source)
    keys = _diag_key(diags)
    assert len(keys) == len(set(keys)), (
        f"duplicate recovery diagnostics for {source!r}\n  {keys}"
    )


class TestRecoveryHeuristicCases:
    """One curated case per recovery heuristic (and the no-recovery siblings),
    so each path the two-pass takes is pinned by name, not only by fuzzing."""

    @pytest.mark.parametrize(
        "source",
        [
            # E201 unterminated '[' — command-break heuristic (known cmd next line)
            "set x [foo bar\nset y 2\n",
            # E201 — comment-break heuristic ('#' line after incomplete '[')
            "set x [foo bar\n# comment\nset y 2\n",
            # E201 — brace-break heuristic (first '{' inside '[')
            "set x [foo {a b}\n",
            # E202 unterminated '"' — newline heuristic ('"' at EOL, known cmd next)
            'set x "\nset y 2\n',
            # E202 — quote with text on the line: heuristic declines, warning kept
            'set x "hello\nset y 2\n',
            # E203 unterminated '{' — command heuristic (de-indented known cmd)
            "proc p {x y} {\nputs hi\nset z 3\n",
            # E203 — no heuristic match: over-running STR + warning kept
            "proc p {x\nset y 2\n",
            # nested: unterminated '[' inside a braced body
            "proc p {} {\n  set x [foo\n}\n",
        ],
    )
    def test_heuristic_case(self, source):
        _assert_equivalent(source)
        _assert_no_duplicate_diagnostics(source)

    @pytest.mark.parametrize(
        "source",
        [
            "",
            "set x 1\n",
            "set x [foo bar]\nset y 2\n",  # well-formed bracket
            'set x "hello world"\n',  # well-formed quote
            "proc p {x y} {\n  return [expr {$x + $y}]\n}\n",  # well-formed braces
            "# just a comment\n",
            "set x 1; set y 2\n",
            "[]\n",  # empty command substitution
            'set x ""\n',  # empty quote
            "set x {}\n",  # empty brace
        ],
    )
    def test_well_formed_unchanged(self, source):
        _assert_equivalent(source)


class TestRecoveryDiagnosticCharacterization:
    """Pin the exact recovery diagnostic (code, span, message, quick-fix) each
    heuristic produces, so the suggestions the LSP surfaces can't silently drift.
    """

    @pytest.mark.parametrize(
        ("source", "expected"),
        [
            (
                "set x [foo bar\nset y 2\n",
                [("E201", 6, 13, "missing close-bracket", (("]", "Insert missing ']' before command"),))],
            ),
            (
                "set x [foo bar\n# comment\nset y 2\n",
                [("E201", 6, 13, "missing close-bracket", (("]", "Insert missing ']' before comment"),))],
            ),
            (
                "set x [foo {a b}\n",
                [("E201", 6, 9, "missing close-bracket", (("]", "Insert missing ']' before '{'"),))],
            ),
            (
                'set x "\nset y 2\n',
                [("E202", 6, 6, 'missing "', (('"', "Insert missing '\"' to close string"),))],
            ),
            (
                "proc p {x y} {\nputs hi\nset z 3\n",
                [("E203", 13, 13, "missing close-brace", ())],
            ),
        ],
    )
    def test_exact_diagnostics(self, source, expected):
        _cmds, diags = segment_with_recovery(source)
        got = [
            (
                d.code,
                d.range.start.offset,
                d.range.end.offset,
                d.message,
                tuple((f.new_text, f.description) for f in (d.fixes or ())),
            )
            for d in diags
        ]
        assert got == expected, f"diagnostics drifted for {source!r}\n  got: {got}"


# --- promoted fuzz findings --------------------------------------------------
#
# We do NOT run live random fuzz in CI (it re-checks the same fixed-seed cases
# every run for ~1 min and explores nothing new).  Instead the campaign runs
# offline — ``make test-fuzz-recovery`` (tests/fuzz_recovery_campaign.py) — and
# any divergence it shrinks is promoted to a deterministic case below (or to
# tests/test_recovery_edge_cases.py).  These are the minimal repros the campaign
# has surfaced; each must satisfy single-pass == two-pass and carry no duplicate
# diagnostics.  The generators are kept for the offline campaign and for
# tests/test_chunk_cache_soundness.py.

_KNOWN_CMDS = [
    "set a 1",
    "proc p {} {}",
    "puts hi",
    "if {1} {}",
    "return",
    "expr {1 + 2}",
    "namespace eval n {}",
    "foreach x $l {}",
    "while {1} {}",
]
_OPENERS = ["{", "[", '"', "set x {", "puts [", 'set y "', "if {", "proc q {"]
_NOISE = ["}", "]", '"', "# comment here", "x", "$v", ";", "  ", "a b c", "\t"]


def _rand_recovery_source(rng: random.Random) -> str:
    parts = []
    for _ in range(rng.randint(1, 14)):
        r = rng.random()
        parts.append(
            rng.choice(_OPENERS)
            if r < 0.4
            else rng.choice(_KNOWN_CMDS)
            if r < 0.8
            else rng.choice(_NOISE)
        )
    return "\n".join(parts) + ("\n" if rng.random() < 0.7 else "")


_SOUP = 'set proc if {} [] $x "q" ; \n# c\nputs expr { } [ ] " $ x 1 } ] '


def _rand_soup_source(rng: random.Random) -> str:
    return "".join(rng.choice(_SOUP) for _ in range(rng.randint(0, 120)))


class TestDistilledCorpus:
    """A small, readable corpus distilled from the offline fuzz exploration.

    One *sane* case per recovery behaviour the campaign surfaces — which closers
    get inserted and which E2xx diagnostics fire — plus the minimal bug repros it
    found.  The exhaustive random sweep stays offline (``make
    test-fuzz-recovery``); this is the deterministic standard set CI runs.
    """

    @pytest.mark.parametrize(
        "source",
        [
            # nested [ whose single inserted ] cannot balance -> command over-runs
            "set x [foo [bar baz\n",
            "set x [a [b [c\nputs hi\n",
            # multiple independent recoveries in one source (two inserted ])
            "set a [foo\nset b 2\nset c [bar\nset d 4\n",
            # fast-path soundness: a " that closes at a later stray ", and a "
            # whose tail is a VAR/CMD substitution (nothing reaches EOF in-quote)
            '"\nforeach"$x',
            'set y "\nproc q {\n$v',
            # unterminated braced expression recovers before a known command
            "if {$x > 5\nset\n",
            # unterminated data value: de-indented command still closes it
            "set v {\n  a b\nset y 2\n",
            # mixed closers in one source
            "set x [foo {a b\n",
            'puts [foo "bar\n',
            # lexer diagnostics for trailing/var-name braces (E204/E205/E206)
            "set x {a b}c\n",
            'set x "ab"c\n',
            "puts ${a b\n",
            # de-dup: first parse and recovered re-parse both flag one region
            '{\nif\n""[\nset',
            'set x "\nif {1} {\n  puts [foo\n}\nset\n',
            # empty delimiters
            "[]\n",
            'set x ""\n',
            "set x {}\n",
        ],
    )
    def test_case(self, source):
        _assert_equivalent(source)
        _assert_no_duplicate_diagnostics(source)
