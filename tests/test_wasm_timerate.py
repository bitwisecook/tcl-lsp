"""WASM ``timerate`` command.

Pins ``eval_timerate`` in ``runtime/zig/cmds/flow.zig`` against
``Tcl_TimeRateObjCmd``: argument / option validation, the 8-element
result list shape (``<µs/#> µs/# <count> # <#/sec> #/sec <net-ms>
net-ms``), max-count vs max-time, and break / continue / return / error
flow out of the measured command.  Regression guard for cmdMZ-6.*.
"""

from __future__ import annotations

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402


def _run(source: str) -> str:
    wasm = _compile_tcl(source)
    _, stdout = _run_wasm(wasm, capture_stdout=True)
    return stdout.rstrip("\n")


_USAGE = (
    'wrong # args: should be "timerate ?-direct? ?-calibrate? '
    '?-overhead double? command ?time ?max-count??"'
)


@pytest.mark.parametrize(
    "source,expected",
    [
        # Argument / option validation (cmdMZ-6.1 / 6.2.* / 6.3).
        ("puts [list [catch {timerate} m] $m]", f"1 {{{_USAGE}}}"),
        ("puts [list [catch {timerate a b c d} m] $m]", f"1 {{{_USAGE}}}"),
        ("puts [list [catch {timerate a b c} m] $m]", '1 {expected integer but got "b"}'),
        ("puts [list [catch {timerate a b} m] $m]", '1 {expected integer but got "b"}'),
        (
            "puts [list [catch {timerate -overhead b {} a b} m] $m]",
            '1 {expected floating-point number but got "b"}',
        ),
        # Result shape — one iteration (cmdMZ-6.5a) and zero iterations
        # via max-count 0 (cmdMZ-6.5b).
        (
            r"puts [regexp {^\d+(?:\.\d+)? \ws/# 1 # \d+(?:\.\d+)? #/sec \d+(?:\.\d+)? net-ms$} [timerate {} 0]]",
            "1",
        ),
        (r"puts [regexp {^0 \ws/# 0 # 0 #/sec 0 net-ms$} [timerate {} 0 0]]", "1"),
        # max-count wins when small (cmdMZ-6.9).
        ("puts [lindex [timerate {} 1000 5] 2]", "5"),
        # break ends the measurement after one iteration (cmdMZ-6.8).
        ("puts [lindex [timerate {break}] 2]", "1"),
        # continue keeps iterating to max-count (cmdMZ-6.8.1).
        ("puts [lindex [timerate {continue} 1000 7] 2]", "7"),
        # return propagates out of timerate (cmdMZ-6.7.1).
        (
            'set x 0; proc r1 {} {upvar x x; timerate {incr x; return "r1"; incr x} 1000 10}; puts [list [r1] $x]',
            "r1 1",
        ),
        # error propagates with the body's message (cmdMZ-6.7).
        ("puts [list [catch {timerate {error foo} 1} m] $m]", "1 foo"),
        # a malformed command is still parsed even with a negative time
        # (cmdMZ-6.4).
        (
            'puts [list [catch {timerate "foreach a {c d e} \\{" -12456} m] $m]',
            "1 {missing close-brace}",
        ),
    ],
)
def test_timerate(source: str, expected: str) -> None:
    assert _run(source) == expected
