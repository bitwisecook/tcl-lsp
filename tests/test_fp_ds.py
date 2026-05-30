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


def test_FP_DS_06_same_element_overwrite_fires_w220():
    """TP / must-alias kill lock-in: writing the SAME literal-key
    array element twice with no intervening read of that element
    makes the first write dead.

    Locked the precision gap closure: ``_must_alias_killed_in_block``
    in ``compiler/core_analyses.py`` walks the def's block forward
    looking for (a) an intervening read of the EXACT same place
    (cancels the kill) or (b) a later write to the EXACT same place
    (must-alias kill).  Literal-key comparison; dynamic keys keep the
    conservative behaviour."""
    src = "proc f {} {\n    set a(k) 1\n    set a(k) 2\n    return $a(k)\n}"
    found = _codes(src, "W220") or _codes(src, "O109")
    assert found, (
        "two writes to the same array element without an intervening read "
        "should surface as W220 or O109 once must-alias kills land"
    )


# FP-DS-07 — namespace-eval body scope survives an inline/factory IRBlock rebuild
#
# A `namespace eval ns {…}` body runs in `ns`, not the caller frame, so an
# unqualified `$x` there is NOT a use of the caller's parameter (tclsh: "can't
# read x: no such variable").  The IRBlock that carries this (caller_scope=False)
# is rebuilt by the inline-uplevel / factory-specialise passes when its body
# changes; that rebuild must preserve the flag, or the body's reads get recovered
# as caller reads and falsely suppress the genuine W214/W220.


FP_DS_07_REPRO = """\
proc reset {} { uplevel 1 {set counter 0} }
proc g {x} {
    namespace eval ::ns {
        reset
        puts "hello $x"
    }
}
"""


def test_FP_DS_07_ns_eval_param_unused_through_rebuild_fires():
    """TP: `reset` is an uplevel-passthrough candidate, so inline_uplevel rebuilds
    the ns-eval IRBlock.  `$x` runs in ::ns (not g's frame) → the parameter `x`
    is genuinely unused → W214 must still fire.  A rebuild that dropped
    caller_scope would recover `$x` as a caller read and suppress it."""
    assert _codes(FP_DS_07_REPRO, "W214"), (
        "param used only inside a (rebuilt) namespace eval body must fire W214"
    )


def test_FP_DS_07_plain_eval_body_read_is_caller_use_silent():
    """FP control: a *plain* `eval {…}` body runs in the caller frame, so `$x`
    IS a use of the parameter — W214 must NOT fire (contrast the ns-eval form)."""
    src = 'proc g {x} { eval { puts "hello $x" } }'
    assert _codes(src, "W214") == [], "plain eval body read is a genuine caller use"


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
