"""WASM ``return -code error`` with no message word.

``return -code error`` (no trailing message) raises an error whose result
is the *empty string*, not a null object — Tcl's ``catch`` then writes
``""`` into its message variable.  The runtime previously forwarded a
null ``result_obj`` to ``tcl_cmd_error_full``, leaving ``state.error_msg``
at 0 so ``catch {return -code error} m`` never set ``m`` and the next
``$m`` read raised "can't read".  Regression guard for proc-old-7.10 and
the cmdMZ-return-3.9..3.12.1 round-trip cluster (which compares
``catch $script`` against ``catch {return -options $bar $foo}``).

See ``eval_return`` in ``runtime/zig/cmds/flow.zig``.
"""

from __future__ import annotations

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402


def _run(source: str) -> str:
    wasm = _compile_tcl(source)
    _, stdout = _run_wasm(wasm, capture_stdout=True)
    return stdout.rstrip("\n")


@pytest.mark.parametrize(
    "source,expected",
    [
        # The message variable is set to "" (not left unset).
        (r'catch {return -code error} m; puts "m=<$m>"', "m=<>"),
        # proc-old-7.10 verbatim: catch a proc that returns -code error.
        (
            "proc tproc2 {} {return -code error}; puts [list [catch tproc2 msg] $msg]",
            "1 {}",
        ),
        # An explicit message still flows through unchanged.
        (r'catch {return -code error hi} m; puts "m=<$m>"', "m=<hi>"),
        # The options dict reports -code 1 either way.
        (r"catch {return -code error} m o; puts [dict get $o -code]", "1"),
        # Empty ``error ""`` (the path we mirror) keeps working.
        (r'catch {error ""} m; puts "m=<$m>"', "m=<>"),
        # cmdMZ-return-3.9 shape: a caught ``return -code error`` is
        # indistinguishable from re-raising it via ``return -options``.
        (
            "set one [list [catch {return -code error} foo bar] $foo]; "
            "set two [list [catch {return -options $bar $foo} f2 b2] $f2]; "
            "puts [string equal $one $two]",
            "1",
        ),
    ],
)
def test_return_code_error_empty_message(source: str, expected: str) -> None:
    assert _run(source) == expected
