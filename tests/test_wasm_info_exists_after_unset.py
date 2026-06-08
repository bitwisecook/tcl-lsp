"""Compiled ``info exists`` must stay coherent across ``unset``.

``set x 1; unset x; info exists x`` returned 1 even though the variable
was genuinely gone — a read (``$x``) errored with ``can't read "x": no
such variable`` correctly, but introspection lied.  Two independent root
causes, both "``unset`` never invalidates the compiler's local-liveness
view":

  - Top-level (script-scope): the compiled ``info exists`` answered from
    the WASM-local mirror (seeded by ``set x 1``, never cleared by the
    eval-fallback ``unset``).  Fixed by routing script-scope existence
    through ``tcl_info_exists`` — the same authoritative state a
    top-level ``$x`` read consults.

  - Proc-local / parameter: ``unset`` nulls the frame bucket's value
    (``var_set(name, 0)``) but leaves the directory bucket, and
    ``var_exists`` / ``local_exists`` reported "exists" for any present
    bucket.  Fixed in the runtime: a null OFF_VALUE means "declared but
    unset" → does not exist.

These compile the snippet directly (not via ``eval``) so the COMPILED
``info exists`` path is exercised.  Reference: Tcl 9 ``Tcl_UnsetObjCmd`` /
``InfoExistsCmd`` (generic/tclVar.c, tclCmdIL.c).
"""

from __future__ import annotations

import pytest

pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402


def _run(source: str) -> str:
    _, stdout = _run_wasm(_compile_tcl(source), capture_stdout=True)
    return stdout.rstrip("\n")


def test_top_level_scalar_info_exists_after_unset() -> None:
    assert _run("set x 1; unset x; puts [info exists x]") == "0"


def test_proc_local_scalar_info_exists_after_unset() -> None:
    assert _run("proc f {} { set x 1; unset x; return [info exists x] }; puts [f]") == "0"


def test_proc_parameter_info_exists_after_unset() -> None:
    assert _run("proc f {a} { unset a; return [info exists a] }; puts [f 7]") == "0"


def test_top_level_whole_array_info_exists_after_unset() -> None:
    assert _run("set a(1) 1; unset a; puts [info exists a]") == "0"


def test_global_declared_info_exists_after_unset() -> None:
    # ``global g`` aliases the global table; ``unset g`` clears it, so
    # ``info exists g`` inside the proc AND at top level must read 0.
    src = "set g 1; proc f {} {global g; unset g; return [info exists g]}; puts [f]/[info exists g]"
    assert _run(src) == "0/0"


def test_global_declared_info_exists_still_true_when_set() -> None:
    # Guard the affirmative answer for global-declared aliases, incl.
    # a pure-array global (no scalar slot).
    assert _run("proc f {} {global g; set g 5; return [info exists g]}; puts [f]") == "1"
    assert _run("set arr(1) 9; proc f {} {global arr; return [info exists arr]}; puts [f]") == "1"


def test_info_exists_still_true_for_live_vars() -> None:
    # Guard: the fix must not break the affirmative answer for set vars.
    assert _run("set x 1; puts [info exists x]") == "1"
    assert _run("set a(1) 1; puts [info exists a]") == "1"
    assert _run("proc f {} { set x 5; return [info exists x] }; puts [f]") == "1"


def test_info_exists_true_again_after_reset() -> None:
    # unset then re-set: existence must flip back to 1 with the new value.
    assert _run("set x 1; unset x; set x 5; puts [info exists x]/$x") == "1/5"
    assert (
        _run("proc f {} { set x 1; unset x; set x 9; return [info exists x]/$x }; puts [f]")
        == "1/9"
    )
