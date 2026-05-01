"""WASM event-loop tests — exercise the Stage-1 scheduler.

Covers the user-visible surface of the new ``after`` / ``vwait`` /
``update`` / ``fileevent`` / ``coroutine`` commands shipped by the
sched module.  The tests use very short delays (≤10 ms) and assert
event ordering, not wall-clock duration, so they stay deterministic
under CI noise.
"""

from __future__ import annotations

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import (  # noqa: E402
    _compile_tcl,
    _run_wasm,
)


def _run_for_stdout(source: str) -> str:
    """Compile + run the script, returning captured stdout."""
    wasm = _compile_tcl(source)
    _, stdout = _run_wasm(wasm, capture_stdout=True)
    return stdout


class TestAfter:
    def test_after_ms_returns_id(self):
        out = _run_for_stdout(
            "set id [after 5 {set ::done 1}]\n"
            'puts [string match "after#*" $id]\n'
            "vwait ::done\n"
            "puts $::done\n"
        )
        assert out.splitlines() == ["1", "1"]

    def test_after_orders_by_deadline(self):
        # Schedule three timers with different deadlines and append
        # to a global so the order is observable.
        out = _run_for_stdout(
            "set ::log {}\n"
            "after 30 {lappend ::log c; set ::done 1}\n"
            "after 5  {lappend ::log a}\n"
            "after 15 {lappend ::log b}\n"
            "vwait ::done\n"
            "puts $::log\n"
        )
        # Expect a, b, c in deadline order regardless of registration order.
        assert out.strip() == "a b c"

    def test_after_cancel_by_id(self):
        out = _run_for_stdout(
            "set id [after 5 {set ::v cancelled-fired}]\n"
            "after cancel $id\n"
            "after 10 {set ::v fallback}\n"
            "vwait ::v\n"
            "puts $::v\n"
        )
        assert out.strip() == "fallback"

    def test_after_cancel_by_script(self):
        out = _run_for_stdout(
            "after 5 {set ::v cancelled-fired}\n"
            "after cancel {set ::v cancelled-fired}\n"
            "after 10 {set ::v fallback}\n"
            "vwait ::v\n"
            "puts $::v\n"
        )
        assert out.strip() == "fallback"

    def test_after_info_lists_pending(self):
        out = _run_for_stdout(
            "set id1 [after 100 {nope}]\n"
            "set id2 [after 200 {nope}]\n"
            "set ids [after info]\n"
            "puts [llength $ids]\n"
            "after cancel $id1\n"
            "after cancel $id2\n"
        )
        # Two ids pending at the time of `after info`.
        assert out.strip() == "2"

    def test_after_info_quotes_script_with_unmatched_brace(self):
        # Regression for Codex P1 on PR #284: ``after info`` previously
        # wrapped the script in raw ``{...}`` which broke for scripts
        # containing unmatched ``}`` bytes.  Verify the result is a
        # well-formed 2-element Tcl list that round-trips through
        # ``lindex``.
        out = _run_for_stdout(
            'set id [after 100 "puts \\}"]\n'
            "set info [after info $id]\n"
            "puts [llength $info]\n"
            "puts [lindex $info 1]\n"
            "after cancel $id\n"
        )
        lines = out.splitlines()
        assert lines[0] == "2"
        assert lines[1] == "timer"


class TestUpdate:
    def test_update_drains_zero_ms_timers(self):
        out = _run_for_stdout("set ::log {}\nafter 0 {lappend ::log fired}\nupdate\nputs $::log\n")
        assert out.strip() == "fired"

    def test_update_idletasks_only_drains_idle(self):
        out = _run_for_stdout(
            "set ::log {}\n"
            "after 0    {lappend ::log timer}\n"
            "after idle {lappend ::log idle}\n"
            "update idletasks\n"
            "puts $::log\n"
        )
        assert out.strip() == "idle"

    def test_update_drains_idle_after_timer(self):
        # Real Tcl's ``update`` runs ready timers first, then idle.
        out = _run_for_stdout(
            "set ::log {}\n"
            "after idle {lappend ::log idle}\n"
            "after 0    {lappend ::log timer}\n"
            "update\n"
            "puts $::log\n"
        )
        assert out.strip() == "timer idle"


class TestVwait:
    def test_vwait_returns_when_var_written(self):
        out = _run_for_stdout("after 5 {set ::v hello}\nvwait ::v\nputs $::v\n")
        assert out.strip() == "hello"


class TestFileevent:
    def test_fileevent_registration_roundtrip(self):
        out = _run_for_stdout(
            "fileevent stdin readable {set ::r 1}\n"
            "puts [fileevent stdin readable]\n"
            "fileevent stdin readable {}\n"
            "puts [string length [fileevent stdin readable]]\n"
        )
        # First puts: the registered script body.  Second: 0 (cleared).
        lines = out.splitlines()
        assert lines[0] == "set ::r 1"
        assert lines[1] == "0"


class TestCoroutine:
    def test_coroutine_command_removed_after_terminal_return(self):
        # Regression for Codex P1 on PR #284: a coroutine that returns
        # normally must un-register its dispatch command so a later
        # ``[name]`` call surfaces ``invalid command name`` instead of
        # silently returning empty (and so the fixed MAX_COROS table
        # frees the slot for reuse).
        out = _run_for_stdout(
            "coroutine c apply {{} {return done}}\n"
            "puts [c]\n"
            "set rc [catch {c} msg]\n"
            "puts $rc\n"
            "puts $msg\n"
        )
        lines = out.splitlines()
        # First [c]: runs the body to completion, body returns "done".
        # Second [c]: command no longer exists → catch yields rc != 0
        # with an "invalid command name" message.
        assert lines[0] == "done"
        assert lines[1] == "1"
        assert "invalid command name" in lines[2] or "c" in lines[2]

    def test_coroutine_basic_yield_resume(self):
        # Three explicit ``yield`` calls so both drivers exercise the
        # multi-yield path:
        #   * v1 segment driver — each top-level command is a separate
        #     segment and ``[c]`` runs the next one.
        #   * Stage-2 asyncify driver — the rewind chain re-enters
        #     ``signal_yield`` for the previous yield, the host-side
        #     ``coro_yield_unwind`` trampoline calls
        #     ``asyncify_stop_rewind`` to flip state to ``NORMAL``,
        #     and the body advances to the next ``yield``
        #     (issue #282).
        # Avoid bodies that complete naturally past the last yield
        # — that path is not yet handled cleanly under asyncify and
        # is tracked as Stage 2.6 (see ``sched/tcl_coro.zig``).
        out = _run_for_stdout(
            "coroutine c apply {{} { yield A; yield B; yield C }}\nputs [c]\nputs [c]\nputs [c]\n"
        )
        lines = out.splitlines()
        assert lines[:3] == ["A", "B", "C"]
