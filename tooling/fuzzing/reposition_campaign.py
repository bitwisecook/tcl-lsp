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

"""Adversarial campaign targeting the reposition fast-path's position tracking.

The reposition cache reuses a proc's *position-independent* dataflow analysis
(keyed by block name + statement index + SSA value key) while rebuilding only
the CFG/SSA to recover fresh ranges.  Its soundness rests on two assumptions:

  1. ``build_cfg_function(body)`` is a *pure function of body text* — identical
     block names AND statement indices every time — so the reused index-based
     analysis still points at the right statement.
  2. The freshly-built CFG carries correct absolute ranges *including byte
     offsets*, which flow into optimiser / code-action edit ``data``.

This campaign hunts for any violation by driving an incremental ``DocumentState``
through *reposition-biased* edit bursts (line churn above/around procs, blank
lines, comment toggles, fast multi-edits) and, after each burst, asserting its
**full compilation unit** — every proc's CFG block/index/line/char/**offset** —
is byte-identical to a from-scratch rebuild on the same text.  This is a far
stricter oracle than the diagnostics-only signature (which ignores byte
offsets), so a misaligned reused analysis or a stale offset cannot hide.

Reproducible: a finding is fully described by ``(seed, base, edit history)``.
"""

from __future__ import annotations

import random
from dataclasses import dataclass, field

from server.workspace.document_state import DocumentState

# --- full-CU structural signature -------------------------------------------


def _stmt_sig(s) -> tuple | None:
    r = getattr(s, "range", None)
    if r is None:
        return None
    # Include byte offsets — the reposition path's reason to exist is correct
    # offsets, and the diagnostics signature does NOT compare them.
    return (
        type(s).__name__,
        r.start.line,
        r.start.character,
        r.start.offset,
        r.end.line,
        r.end.character,
        r.end.offset,
    )


def _proc_cu_sig(fu) -> tuple:
    """Structural + positional signature of one FunctionUnit's CFG.

    Captures block names, per-block statement order, and every statement's
    absolute range (line/char/offset).  Index-based analysis facts are also
    folded in so a reused analysis pointing at the wrong statement is caught.
    """
    blocks = []
    for bn in sorted(fu.cfg.blocks):
        blk = fu.cfg.blocks[bn]
        stmts = tuple(_stmt_sig(s) for s in blk.statements)
        term = getattr(blk, "terminator", None)
        tr = getattr(term, "range", None) if term is not None else None
        term_sig = None if tr is None else (tr.start.offset, tr.end.offset)
        blocks.append((bn, stmts, term_sig))
    an = fu.analysis
    facts = (
        tuple(sorted((r.block, r.statement_index, r.variable) for r in an.read_before_set)),
        tuple(sorted((d.block, d.statement_index, d.variable) for d in an.dead_stores)),
        tuple(sorted((u.block, u.statement_index, u.variable) for u in an.unused_variables)),
    )
    return (tuple(blocks), facts, fu.complexity_guarded)


def cu_signature(state: DocumentState) -> tuple | None:
    """Full-CU signature: every proc's CFG structure + absolute positions."""
    cu = state.compilation_unit
    if cu is None:
        return None
    return tuple(sorted((q, _proc_cu_sig(fu)) for q, fu in cu.procedures.items()))


def _full_cu_sig(text: str, uri: str) -> tuple | None:
    s = DocumentState(uri=uri)
    s.update(text)
    return cu_signature(s)


# --- reposition-biased edit generation --------------------------------------

# Bases rich in procs (the reposition path only fires on procs) with varied
# bodies so the reused analysis has real facts to misalign.
_BASES = [
    (
        "proc add {a b} {\n"
        "    set unused 1\n"
        "    return [expr {$a + $b}]\n"
        "}\n"
        "proc greet {name} {\n"
        '    puts "hi $name"\n'
        "    return $missing\n"
        "}\n"
        "proc loop {n} {\n"
        "    for {set i 0} {$i < $n} {incr i} {\n"
        "        if {$i == 3} { set found 1 }\n"
        "    }\n"
        "    return $found\n"
        "}\n"
    ),
    (
        "namespace eval app {\n"
        "    proc init {} {\n"
        "        variable state\n"
        "        set state ready\n"
        "    }\n"
        "    proc run {cmd} {\n"
        "        switch $cmd {\n"
        "            go { return 1 }\n"
        "            stop { return 0 }\n"
        "        }\n"
        "        return $undefined\n"
        "    }\n"
        "}\n"
    ),
    (
        "proc validate {x} {\n"
        "    if {$x < 0} {\n"
        '        return -code error "negative"\n'
        "    } elseif {$x > 100} {\n"
        "        set capped 100\n"
        "    }\n"
        "    return $x\n"
        "}\n"
        "proc process {items} {\n"
        "    foreach item $items {\n"
        "        lappend result [validate $item]\n"
        "    }\n"
        "    return $result\n"
        "}\n"
    ),
]

# Strings inserted/removed *between* and *above* procs — pure repositioning that
# shifts proc offsets without touching proc body text (the fast-path's domain).
_FILLERS = [
    "\n",
    "\n\n",
    "\n\n\n",
    "# a comment\n",
    "# another comment line here\n",
    "set toplevel_var 1\n",
    'puts "top level"\n',
    "    \n",  # whitespace-only line
    "\t\n",
    ";# trailing\n",
]


def reposition_edit(rng: random.Random, text: str) -> str:
    """Apply one reposition-biased edit: insert/remove filler at a line boundary
    (above/between procs), or duplicate/delete a whole line.  Keeps proc *body*
    text intact far more often than the generic fuzzer, so the fast-path fires."""
    lines = text.splitlines(keepends=True)
    if not lines:
        return rng.choice(_FILLERS)
    kind = rng.random()
    if kind < 0.45:
        # Insert filler at a random line boundary.
        idx = rng.randrange(len(lines) + 1)
        lines.insert(idx, rng.choice(_FILLERS))
    elif kind < 0.7:
        # Delete a random line (may delete a blank/comment, repositioning rest).
        idx = rng.randrange(len(lines))
        del lines[idx]
    elif kind < 0.85:
        # Duplicate a line.
        idx = rng.randrange(len(lines))
        lines.insert(idx, lines[idx])
    else:
        # Swap two adjacent lines (reorders, shifting offsets both ways).
        if len(lines) >= 2:
            idx = rng.randrange(len(lines) - 1)
            lines[idx], lines[idx + 1] = lines[idx + 1], lines[idx]
    return "".join(lines)


@dataclass
class Divergence:
    seed: int
    step: int
    text: str
    history: list[str] = field(default_factory=list)
    detail: str = ""


def _first_block_diff(inc_sig, full_sig) -> str:
    """Human-readable first point of divergence between two CU signatures."""
    inc = dict(inc_sig or ())
    full = dict(full_sig or ())
    for q in sorted(set(inc) | set(full)):
        if q not in inc:
            return f"proc {q} missing from incremental"
        if q not in full:
            return f"proc {q} missing from full rebuild"
        if inc[q] != full[q]:
            ib, fb = inc[q][0], full[q][0]
            ibd = dict((b[0], b) for b in ib)
            fbd = dict((b[0], b) for b in fb)
            for bn in sorted(set(ibd) | set(fbd)):
                if ibd.get(bn) != fbd.get(bn):
                    return f"proc {q} block {bn}:\n   inc ={ibd.get(bn)}\n   full={fbd.get(bn)}"
            return f"proc {q} facts/guard differ:\n   inc ={inc[q][1:]}\n   full={full[q][1:]}"
    return "signatures differ but no per-proc diff isolated"


def run_burst_sequence(seed: int, base: str, n_bursts: int, burst_size: int) -> Divergence | None:
    """Drive *fast multi-edit bursts*: apply ``burst_size`` reposition edits
    back-to-back (no settling) before each comparison, ``n_bursts`` times.

    Compares the incremental state's FULL CU (offsets included) against a cold
    rebuild after every burst."""
    rng = random.Random(seed)
    inc = DocumentState(uri=f"repos-inc:{seed}")
    inc.update(base)
    text = base
    history = [base]

    # Sanity: base must match (full == full).
    si = cu_signature(inc)
    sf = _full_cu_sig(base, f"repos-full:{seed}:base")
    if si != sf:
        return Divergence(seed, -1, text, history, _first_block_diff(si, sf))

    for b in range(n_bursts):
        # Fast burst: several edits applied in immediate succession.
        for _ in range(burst_size):
            text = reposition_edit(rng, text)
            inc.update(text)  # every keystroke hits the incremental path
        history.append(text)
        si = cu_signature(inc)
        sf = _full_cu_sig(text, f"repos-full:{seed}:{b}")
        if si != sf:
            return Divergence(seed, b, text, history, _first_block_diff(si, sf))
    return None


def campaign(
    seeds: int = 200,
    n_bursts: int = 12,
    burst_size: int = 4,
    max_report: int = 8,
) -> list[Divergence]:
    """Run a full campaign; return up to *max_report* divergences."""
    found: list[Divergence] = []
    for i in range(seeds):
        base = _BASES[i % len(_BASES)]
        d = run_burst_sequence(i, base, n_bursts, burst_size)
        if d is not None:
            found.append(d)
            if len(found) >= max_report:
                break
    return found


if __name__ == "__main__":
    import argparse
    import time

    ap = argparse.ArgumentParser()
    ap.add_argument("-n", "--seeds", type=int, default=200)
    ap.add_argument("--bursts", type=int, default=12)
    ap.add_argument("--burst-size", type=int, default=4)
    args = ap.parse_args()

    t0 = time.perf_counter()
    divs = campaign(args.seeds, args.bursts, args.burst_size)
    secs = time.perf_counter() - t0
    total_edits = args.seeds * args.bursts * args.burst_size
    print(
        f"RESULT seeds={args.seeds} bursts={args.bursts} burst_size={args.burst_size} "
        f"edits~={total_edits} secs={secs:.0f} divergences={len(divs)}"
    )
    for d in divs:
        print(f"\nDIVERGENCE seed={d.seed} burst={d.step}")
        print(f"  {d.detail}")
