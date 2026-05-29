"""TP/FP regression tests for the DS (dead-store / unused) family.

Each test pairs to an ``FP-DS-NN`` entry in
``docs/design/compiler/FP.md``.  The Tcl reproducer string is **copied verbatim**
from the doc — if either side drifts the test will visibly catch it.

DS family covers W220 (dead store) and W211 (unused variable) determinations
where a real read of the variable lives in an otherwise-opaque construct (cmd-
sub, expr cmd-sub, eval body, return terminator, trace callback, or an
ARRAY_ELEM Place that's distinct from the immediately-following write).

Ground truth: real C tclsh 9.0.3.
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from analyser import analyse
from server.features.diagnostics import get_diagnostics


def _codes(source: str, code: str):
    return [d for d in get_diagnostics(source) if d.code == code]


# FP-DS-01 — incr/append/lappend inside cmd-sub keeps the init live


FP_DS_01_REPRO = """\
proc f {} {
    # incr inside the cmd-sub reads `i` (the prior value) — so the
    # feeding `set i 0` is alive, not a dead store.
    set i 0
    foreach j {1 2 3} { lappend r [incr i $j] }
    return $r
}
"""


def test_FP_DS_01_init_kept_live_by_cmdsub_incr():
    """FP: the `set i 0` is read-modify-written by the nested `[incr i $j]`,
    so it must NOT fire W220 (dead store) or W211 (unused)."""
    assert _codes(FP_DS_01_REPRO, "W220") == []
    assert _codes(FP_DS_01_REPRO, "W211") == []


def test_FP_DS_01_genuine_dead_store_still_fires():
    """TP control: no read-modify-write of `i` — the first assignment is
    truly dead.  Catches a regression where the cmd-sub fix accidentally
    suppresses real dead stores."""
    src = "proc f {} { set i 0\n set i 5\n return $i }"
    assert _codes(src, "W220"), "real dead store must still fire"


# FP-DS-02 — reads inside [expr {...}] cmd-sub are real uses


FP_DS_02_REPRO = """\
proc f {} {
    # $w is read inside the [expr {...}] cmd-sub — `set w 5` is NOT
    # a dead store, and `w` is NOT unused.
    set w 5
    set i 0
    incr i [expr {$w}]
    return $i
}
"""


def test_FP_DS_02_expr_cmdsub_read_keeps_def_live():
    """FP: the `[expr {$w}]` argument to `incr` reads `w`; `set w 5` is alive."""
    assert _codes(FP_DS_02_REPRO, "W220") == []
    assert _codes(FP_DS_02_REPRO, "W211") == []


def test_FP_DS_02_no_expr_cmdsub_read_still_fires():
    """TP control: no expr-cmdsub use of `w` — `set w 5` IS truly dead."""
    src = "proc f {} { set w 5\n set w 6\n return $w }"
    assert _codes(src, "W220"), "redundant assignment must fire W220"


# FP-DS-03 — eval {literal-body} reads run in caller scope


FP_DS_03_REPRO = """\
proc f {} {
    # eval's braced body runs in the current scope; `$x` read here is
    # a real read of the local `x`.
    set x 1
    eval {puts $x}
}
"""


def test_FP_DS_03_eval_body_read_kept_live():
    """FP: the eval body reads `$x` so `set x 1` is alive, `x` is used."""
    assert _codes(FP_DS_03_REPRO, "W220") == []
    assert _codes(FP_DS_03_REPRO, "W211") == []


def test_FP_DS_03_eval_body_without_read_still_fires():
    """TP control: when eval's body doesn't read `x`, `x` IS truly unused."""
    src = "proc f {} { set x 1\n eval {puts hi} }"
    assert _codes(src, "W211"), "eval body that doesn't read x must fire W211"


# FP-DS-04 — traced variables excluded (soundness)


FP_DS_04_REPRO = """\
proc f {} {
    # The write is observable through the callback — must NOT fire
    # W220 (dead-store) or W211 (unused).
    trace add variable x write cb
    set x 1
}
"""


def test_FP_DS_04_traced_var_no_w220():
    """FP guard: a trace-write callback observes every `set x …`; the write
    is not dead even with no in-proc read."""
    assert _codes(FP_DS_04_REPRO, "W220") == []
    assert _codes(FP_DS_04_REPRO, "W211") == []


def test_FP_DS_04_84_form_also_excluded():
    """The Tcl 8.4 `trace variable x w cb` form must be exempted too."""
    src = "proc f {} { trace variable x w cb\n set x 1 }"
    assert _codes(src, "W220") == []
    assert _codes(src, "W211") == []


def test_FP_DS_04_untraced_unrelated_var_still_fires():
    """TP control: tracing `x` must not blanket-suppress an unrelated `y`."""
    src = "proc f {} { trace add variable x write cb\n set y 1 }"
    assert _codes(src, "W220") or _codes(src, "W211"), (
        "unrelated `y` must still fire W220/W211 — the trace exemption is name-scoped"
    )


# FP-DS-05 — return value read counts as a use


FP_DS_05_REPRO = """\
proc f {} {
    # return $x reads $x — `set x 1` is NOT a dead store, `x` is NOT unused.
    set x 1
    return $x
}
"""


def test_FP_DS_05_return_read_counts():
    """FP: `return $x` is a use of `x`; the feeding `set x 1` is alive."""
    assert _codes(FP_DS_05_REPRO, "W220") == []
    assert _codes(FP_DS_05_REPRO, "W211") == []


# FP-DS-06 — ARRAY_ELEM Place: distinct array elements are distinct stores


FP_DS_06_REPRO = """\
proc f {} {
    # k and j are distinct array element Places — set a(k) is NOT
    # killed by set a(j); the read of $a(k) makes the first write live.
    set a(k) 1
    set a(j) 2
    puts $a(k)
}
"""


def test_FP_DS_06_array_elem_writes_distinct():
    """FP: distinct array-element writes do not kill each other (Phase 8G).
    The read of $a(k) keeps the first write live, so no W220 on `set a(k) 1`."""
    assert _codes(FP_DS_06_REPRO, "W220") == [], (
        "set a(k) 1 must NOT be W220 — set a(j) 2 writes a different Place"
    )


@pytest.mark.xfail(
    reason="FP-DS-06 open: same-element ARRAY_ELEM dead-store detection deferred — "
    "the Place model's overlap relation is suppress-only after the 8E refinement, so "
    "even writes to a syntactically-identical key like `set a(k) 1; set a(k) 2` are "
    "not flagged (the second write doesn't kill the first in the memory-SSA sense). "
    "Lifting this requires the must-alias analysis the phase8-place-migration plan "
    "defers; flips when ARRAY_ELEM gets a precise kill model.",
    strict=True,
)
def test_FP_DS_06_same_element_overwrite_still_fires():
    """TP control / OPEN: writing the SAME element twice should be a dead store
    once the Place model gains precise must-alias kills.  Currently silent."""
    src = "proc f {} {\n    set a(k) 1\n    set a(k) 2\n    return $a(k)\n}"
    found = _codes(src, "W220") or _codes(src, "O109")
    assert found, (
        "two writes to the same array element without an intervening read "
        "should surface as W220 or O109 once must-alias kills land"
    )


# Sanity check the analyse() entry-point too — get_diagnostics is the full
# pipeline but analyse() is the analyser-only path tests/test_checks.py uses.
def test_FP_DS_smoke_analyse_pipeline():
    """The reproducers must analyse without exceptions (defensive smoke test)."""
    for src in (
        FP_DS_01_REPRO,
        FP_DS_02_REPRO,
        FP_DS_03_REPRO,
        FP_DS_04_REPRO,
        FP_DS_05_REPRO,
        FP_DS_06_REPRO,
    ):
        result = analyse(src)
        # Beyond "didn't raise": assert the analyse contract — every repro
        # yields a well-formed diagnostics list (not merely some non-None
        # object), so a pipeline that silently returned a stub would fail here.
        assert isinstance(result.diagnostics, list)
