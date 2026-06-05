"""WASM list/dict build scaling — the Phase 1 list internal rep.

``tcl_cmd_lappend`` used to re-validate the *entire* accumulator with
``check_list_syntax`` (an O(N) scan) on every append, so building an
N-element list — ``lappend`` in a loop, ``lsearch -all``, ``dict map``
collecting a result — was O(N^2).  At 100k elements this blew past the
180 s watchdog (dict-24.24 / dict-24.25 timed out).

Phase 1 gives a lappend-built list a ``TYPE_LIST`` marker meaning "this
buffer is a canonical list by construction", so subsequent appends skip
the re-validation and the build is back to amortised O(N).  These tests
exercise sizes that finish in well under a second when linear but would
trip the wasmtime watchdog if the quadratic regressed.

The runtime ``timeout_s`` is the real assertion: a regression to O(N^2)
makes ``_run_wasm`` raise (watchdog interrupt) and the test fails.
"""

from __future__ import annotations

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402


def _run(body: str, timeout_s: float = 30.0) -> str:
    _, stdout = _run_wasm(
        _compile_tcl(body), capture_stdout=True, timeout_s=timeout_s
    )
    return stdout.rstrip("\n")


def test_lappend_loop_is_linear() -> None:
    # Build a 50k-element list one lappend at a time; O(N^2) would not
    # finish under the watchdog.  Verify length + a sampled element.
    body = (
        "set x {}\n"
        "for {set i 0} {$i < 50000} {incr i} { lappend x $i }\n"
        "puts [llength $x]\n"
        "puts [lindex $x 49999]\n"
    )
    assert _run(body) == "50000\n49999"


def test_lsearch_all_is_linear() -> None:
    # lsearch -all accumulates every match via tcl_cmd_lappend — the
    # exact path that made dict-24.24's input build quadratic.
    body = "puts [llength [lsearch -all [lrepeat 100000 x] x]]\n"
    assert _run(body) == "100000"


def test_dict_map_huge_dict_terminates() -> None:
    # dict-24.24 verbatim: dict map over a 100k-element dict, summing
    # k*v.  Hung (>120 s) before the fix; ~1.7 s after.
    body = (
        "puts [tcl::mathop::+ {*}"
        "[dict map {k v} [lsearch -all [lrepeat 100000 x] x] "
        "{ expr { $k * $v } }]]\n"
    )
    assert _run(body) == "166666666600000"


def test_lappend_still_validates_malformed_list() -> None:
    # The marker must not weaken correctness: appending to a value whose
    # string rep is NOT a canonical list still raises (listobj-4.4).
    body = (
        'set x "a \\{b"\n'
        "puts [catch {lappend x c} m]\n"
        "puts $m\n"
    )
    assert _run(body) == "1\nunmatched open brace in list"


@pytest.mark.parametrize(
    "body,expected",
    [
        # A lappend-built list whose single element is numeric must still
        # parse as a number (the TYPE_LIST marker is scalar-transparent).
        ("set x {}\nlappend x 5\nputs [expr {$x + 1}]", "6"),
        ("set x {}\nlappend x 5.5\nputs [expr {$x + 1}]", "6.5"),
        ("set x {}\nlappend x a b c\nputs [lindex $x 1]", "b"),
    ],
)
def test_lappend_marker_is_scalar_transparent(body: str, expected: str) -> None:
    assert _run(body) == expected
