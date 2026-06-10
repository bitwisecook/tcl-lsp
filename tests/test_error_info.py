"""Runtime tests for ``::errorInfo`` / ``::errorCode`` propagation.

Before this wave the runtime only propagated the error *message*
(available via ``catch { … } msg``); scripts that inspected
``$::errorInfo`` or ``$::errorCode`` after a caught error got an
empty string.  The runtime now stamps both globals when
``tcl_cmd_error`` fires, with a real Tcl-compatible traceback
appended (``while executing`` / ``invoked from within`` frames) via
``Tcl_LogCommandInfo``-equivalent walks over the active frame
stack — see :file:`runtime/zig/interp/tcl_catch.zig` and
:func:`tcl_interp.log_command_info` for the implementation.

    catch { error boom } msg
    puts $::errorInfo
        boom
            while executing
        "error boom"
    puts $::errorCode       ;# NONE
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from tests.test_wasm_real_tcl import _compile_tcl_with_diag, _run_wasm


def _run(src: str) -> str:
    wasm, _ = _compile_tcl_with_diag(src)
    _, stdout = _run_wasm(wasm, capture_stdout=True)
    return stdout


class TestErrorInfoBasic:
    def test_caught_error_sets_error_info(self) -> None:
        # ``::errorInfo`` carries the error message + a
        # ``Tcl_LogCommandInfo``-style ``while executing\n"<cmd>"``
        # traceback frame.  Matches reference Tcl 9 verbatim.
        out = _run("catch { error boom } msg\nputs $::errorInfo\n")
        assert out == 'boom\n    while executing\n"error boom"\n'

    def test_caught_error_sets_error_code_to_none(self) -> None:
        out = _run("catch { error boom } msg\nputs $::errorCode\n")
        assert out == "NONE\n"

    def test_error_msg_available_as_catch_result(self) -> None:
        # The ``msg`` catch-variable still receives the error text —
        # the new globals are in addition, not replacing.
        out = _run("catch { error widget } msg\nputs $msg\n")
        assert out == "widget\n"


class TestErrorInfoMultipleErrors:
    def test_later_error_overwrites_error_info(self) -> None:
        # Each error stamps fresh; only the most recent sticks.  The
        # traceback frame for the second error replaces the first
        # entirely (``last_log_script`` is reset on each fresh error
        # event in ``tcl_cmd_error``).
        out = _run("catch { error first } msg\ncatch { error second } msg\nputs $::errorInfo\n")
        assert out == 'second\n    while executing\n"error second"\n'


class TestErrorInfoGlobalReadback:
    def test_error_info_is_a_real_global(self) -> None:
        # Reading through ``set`` (no subcommand) should work just
        # like any other global — regression against a specialised
        # handler that only tracks errorInfo internally.
        out = _run("catch { error widget } msg\nputs [set ::errorInfo]\n")
        assert out == 'widget\n    while executing\n"error widget"\n'
