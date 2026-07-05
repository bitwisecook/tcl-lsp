# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Incremental-vs-full differential fuzzer for the green-tree LSP pipeline (#477).

Drives a :class:`DocumentState` through a sequence of random edits — the
*incremental* update path that runs on every keystroke — and after **every
edit** asserts its analysis output is identical to a from-scratch
``DocumentState`` built on the same text (the ground truth).  This is the
feature-level counterpart to the chunk-segmentation property test: it proves
the incremental reparse can never diverge from a full rebuild in the
diagnostics, severities, ranges, and quick-fixes the editor would surface.

Edit kinds mirror real editing: single insert/delete/replace, multi-cursor
(several simultaneous non-overlapping edits), and search-and-replace-all.
Edits are *not* kept valid — a sequence may produce unbalanced braces or
truncated commands; the incremental path must still match the full rebuild on
that exact (malformed) text, which is precisely the robustness under test.

Reproducible: a finding is fully described by ``(seed, base, edit history)``.
"""

from __future__ import annotations

import random
import re
from dataclasses import dataclass, field

from server.workspace.document_state import DocumentState

# --- output signatures -------------------------------------------------------


def _range_sig(r) -> tuple[int, int, int, int]:
    return (r.start.line, r.start.character, r.end.line, r.end.character)


def _fix_sig(f) -> tuple:
    return (_range_sig(f.range), f.new_text, f.description)


def _diag_sig(d) -> tuple:
    return (
        d.code,
        d.severity.name,
        _range_sig(d.range),
        d.message,
        tuple(_fix_sig(f) for f in d.fixes),
        tuple((_range_sig(rg), m) for rg, m in d.related_ranges),
    )


def diagnostics_signature(state: DocumentState) -> tuple | None:
    """A sorted, position-aware signature of a state's diagnostics + quick-fixes."""
    a = state.analysis
    if a is None:
        return None
    return tuple(sorted(_diag_sig(d) for d in a.diagnostics))


def _full_state(text: str, uri: str) -> DocumentState:
    s = DocumentState(uri=uri)
    s.update(text)
    return s


# --- edit generation ---------------------------------------------------------

# Snippets spliced in to grow/mutate the document and fire diverse diagnostics
# (unused vars, arity errors, undefined procs, nested control flow, expr, ...).
_SNIPPETS = [
    "set x 1\n",
    "set unused 99\n",
    "puts $undefined_var\n",
    "proc p {a b} {\n    return [expr {$a + $b}]\n}\n",
    "p 1 2 3\n",
    "if {$x > 0} {\n    incr x\n} else {\n    set x 0\n}\n",
    "foreach i {1 2 3} {\n    puts $i\n}\n",
    "while {$x < 10} { incr x }\n",
    "switch $x {\n    1 { puts one }\n    default { puts other }\n}\n",
    "namespace eval ns {\n    variable v 0\n}\n",
    "set d [dict create a 1 b 2]\n",
    "# comment line\n",
    "}\n",
    "{\n",
    '"\n',
    "[\n",
    "\n",
    ";",
    "set ",
    "$",
]

_IDENT_RE = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")


def _apply(text: str, edits: list[tuple[int, int, str]]) -> str:
    """Apply non-overlapping (start, end, replacement) edits to *text*."""
    for start, end, repl in sorted(edits, key=lambda e: e[0], reverse=True):
        text = text[:start] + repl + text[end:]
    return text


def random_edit(rng: random.Random, text: str) -> tuple[str, str]:
    """Return ``(kind, new_text)`` for one random edit applied to *text*."""
    n = len(text)
    kind = rng.choices(
        ["insert", "delete", "replace", "multi", "search"],
        weights=[30, 20, 20, 15, 15],
    )[0]

    if kind == "insert" or n == 0:
        at = rng.randint(0, n)
        ins = rng.choice(_SNIPPETS) if rng.random() < 0.5 else rng.choice('x \n${}[]"')
        return "insert", text[:at] + ins + text[at:]

    if kind == "delete":
        i = rng.randint(0, n - 1)
        j = min(n, i + rng.randint(1, 12))
        return "delete", text[:i] + text[j:]

    if kind == "replace":
        i = rng.randint(0, n - 1)
        j = min(n, i + rng.randint(1, 10))
        repl = rng.choice(_SNIPPETS) if rng.random() < 0.5 else rng.choice("xy \n{}")
        return "replace", text[:i] + repl + text[j:]

    if kind == "multi":
        # Multi-cursor: 2-4 non-overlapping single edits applied at once.
        k = rng.randint(2, 4)
        positions = sorted(rng.sample(range(n + 1), min(k, n + 1)))
        edits: list[tuple[int, int, str]] = []
        last = -1
        for p in positions:
            if p <= last:
                continue
            ins = rng.choice(["x", "}", "{", " ", "$y", "set z 1\n"])
            edits.append((p, p, ins))
            last = p
        return "multi", _apply(text, edits)

    # search-replace-all: replace every occurrence of an existing identifier.
    idents = _IDENT_RE.findall(text)
    if not idents:
        at = rng.randint(0, n)
        return "search", text[:at] + "x" + text[at:]
    tok = rng.choice(idents)
    repl = rng.choice(["renamed", "q", tok + "2", ""])
    return "search", text.replace(tok, repl)


# --- driver ------------------------------------------------------------------


@dataclass
class Divergence:
    seed: int
    step: int  # -1 = base, 0..n-1 = after that edit
    kind: str
    text: str
    history: list[tuple[str, str]]
    inc_only: list = field(default_factory=list)
    full_only: list = field(default_factory=list)

    def report(self) -> str:
        lines = [
            f"INCREMENTAL != FULL  seed={self.seed} step={self.step} edit={self.kind}",
            f"--- text ({len(self.text)} bytes) ---",
            self.text,
            "--- diagnostics only in INCREMENTAL state ---",
            *[f"  {s}" for s in self.inc_only],
            "--- diagnostics only in FULL rebuild ---",
            *[f"  {s}" for s in self.full_only],
        ]
        return "\n".join(lines)


def run_sequence(seed: int, base: str, n_edits: int) -> Divergence | None:
    """Drive one incremental edit sequence; return the first divergence or None."""
    rng = random.Random(seed)
    inc = DocumentState(uri=f"inc:{seed}")
    inc.update(base)
    text = base
    history: list[tuple[str, str]] = [("base", base)]

    # Compare the base too (full == full, sanity + determinism check).
    si = diagnostics_signature(inc)
    sf = diagnostics_signature(_full_state(base, f"full:{seed}:base"))
    if si != sf:
        return _divergence(seed, -1, "base", text, history, si, sf)

    for step in range(n_edits):
        kind, text = random_edit(rng, text)
        history.append((kind, text))
        inc.update(text)
        full = _full_state(text, f"full:{seed}:{step}")
        si = diagnostics_signature(inc)
        sf = diagnostics_signature(full)
        if si != sf:
            return _divergence(seed, step, kind, text, history, si, sf)
    return None


def _divergence(seed, step, kind, text, history, si, sf) -> Divergence:
    si_set = set(si or ())
    sf_set = set(sf or ())
    return Divergence(
        seed=seed,
        step=step,
        kind=kind,
        text=text,
        history=history,
        inc_only=sorted(str(x) for x in si_set - sf_set),
        full_only=sorted(str(x) for x in sf_set - si_set),
    )


# Hand-written bases rich in diagnostic-firing constructs.
_CORPUS = [
    "proc add {a b} {\n    set unused 1\n    return [expr {$a + $b}]\n}\nset z [add 1 2 3]\n",
    "set x 1\nputs $x\nputs $never_defined\nset y [expr {$x * 2}]\n",
    'foreach item {a b c} {\n    if {$item eq "b"} { continue }\n    puts $item\n}\n',
    "namespace eval app {\n    variable count 0\n    proc inc {} { variable count; incr count }\n}\n",
    "switch -exact $cmd {\n    start { puts go }\n    stop { puts halt }\n}\nset leftover 5\n",
    "while {true} {\n    set x [expr {$x + 1}]\n    if {$x > 100} break\n}\n",
]


def base_scripts(rng: random.Random, count: int) -> list[str]:
    """A mix of generator output and the hand corpus."""
    from tooling.fuzzing.tcl_gen import TclGenerator

    out = list(_CORPUS)
    for _ in range(count):
        out.append(TclGenerator(seed=rng.randint(0, 2**31)).generate())
    return out
