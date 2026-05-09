"""WASM ``for`` control-flow tests.

Pins the next-clause control-code semantics from
``tmp/tcl9.0.3/generic/tclCmdAH.c`` — specifically ``ForPostNextCallback``
(line ~2713): the next-clause's ``BREAK`` collapses to ``TCL_OK`` and
terminates the loop, while ``CONTINUE`` / ``ERROR`` / ``RETURN``
propagate unchanged.

Pre-fix, ``eval_for`` ignored the next-clause's return code, so
``for {} {1} {break} {}`` (the ``for-6.17`` upstream test) span-locked
the WASM bundle for the full 180s watchdog window — surfaced as a
Tier-3 timeout against the ``for`` stem in
``tests/baselines/tcl9-tcltest-wasm/``.
"""

from __future__ import annotations

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402


def _run(source: str) -> str:
    wasm = _compile_tcl(source)
    _, stdout = _run_wasm(wasm, capture_stdout=True)
    return stdout.strip()


def test_for_break_in_next_clause_terminates_loop() -> None:
    """``for {} {1} {break} {}`` must return TCL_OK without spinning.
    Pre-fix this hung the runtime — the BREAK signal raised by the
    next clause was never observed because the cond is a bare literal
    so ``eval_expr_str`` doesn't short-circuit on a pending signal."""
    out = _run(
        """
set rc [catch {for {} {1} {break} {}} err]
puts "$rc|$err"
"""
    )
    assert out == "0|"


def test_for_continue_in_next_clause_propagates() -> None:
    """``for {} {1} {continue} {}`` must propagate the CONTINUE code
    so ``catch`` returns 4 — same contract as upstream
    ``for-6.17`` / ``for-6.18``.  Pre-fix this also span-locked
    because the runtime didn't snapshot the next-clause result."""
    out = _run(
        """
set rc [catch {for {} {1} {continue} {}} err]
puts "$rc|$err"
"""
    )
    assert out == "4|"


def test_for_break_in_init_propagates() -> None:
    """``for {break} {1} {} {}`` — BREAK in the init clause aborts the
    loop before any iteration runs and surfaces as the for command's
    return code.  ``catch`` returns 3 (TCL_BREAK)."""
    out = _run(
        """
set rc [catch {for {break} {1} {} {}} err]
puts "$rc|$err"
"""
    )
    assert out == "3|"


def test_for_continue_in_init_propagates() -> None:
    """``for {continue} {1} {} {}`` — CONTINUE in the init clause
    propagates as the for command's return code.  ``catch`` returns 4
    (TCL_CONTINUE)."""
    out = _run(
        """
set rc [catch {for {continue} {1} {} {}} err]
puts "$rc|$err"
"""
    )
    assert out == "4|"


def test_for_break_in_cond_propagates() -> None:
    """``for {} {[break]} {} {}`` — the cond expression invokes
    ``break``, which sets the BREAK signal *during* expression
    evaluation.  The for terminates and the BREAK code propagates
    through ``catch`` (returns 3)."""
    out = _run(
        """
set rc [catch {for {} {[break]} {} {}} err]
puts "$rc|$err"
"""
    )
    assert out == "3|"


def test_for_continue_in_cond_propagates() -> None:
    """``for {} {[continue]} {} {}`` — same shape as the BREAK case
    but with CONTINUE.  ``catch`` returns 4."""
    out = _run(
        """
set rc [catch {for {} {[continue]} {} {}} err]
puts "$rc|$err"
"""
    )
    assert out == "4|"


def test_for_error_in_next_clause_propagates() -> None:
    """An ERROR raised inside the next clause must propagate up
    rather than being silently swallowed (which would deadlock the
    same way BREAK / CONTINUE did)."""
    out = _run(
        """
set rc [catch {for {set i 0} {$i < 5} {set} {set i 5}} err]
puts "$rc|$err"
"""
    )
    assert out == ' 1|wrong # args: should be "set varName ?newValue?"'.lstrip()


def test_for_normal_loop_still_works() -> None:
    """Regression: the BREAK / CONTINUE branches in the next-clause
    handler must not break the normal ``for`` loop iteration path."""
    out = _run(
        """
set sum 0
for {set i 1} {$i <= 10} {incr i} {
    set sum [expr {$sum + $i}]
}
puts $sum
"""
    )
    assert out == "55"


def test_for_break_in_body_still_terminates() -> None:
    """Regression: ``break`` raised by the *body* (not the next
    clause) must still terminate the loop with TCL_OK."""
    out = _run(
        """
set last 0
for {set i 0} {$i < 100} {incr i} {
    set last $i
    if {$i == 7} { break }
}
puts $last
"""
    )
    assert out == "7"


def test_for_continue_in_body_skips_to_next_clause() -> None:
    """Regression: ``continue`` in the body must run the next clause
    (not skip it like ``while`` does) and re-test the cond."""
    out = _run(
        """
set seen {}
for {set i 0} {$i < 5} {incr i} {
    if {$i == 2} { continue }
    lappend seen $i
}
puts $seen
"""
    )
    assert out == "0 1 3 4"
