"""WASM proc call-time arity — too-many-arguments rejection.

A proc whose last parameter is not the variadic ``args`` collector
rejects extra arguments with ``wrong # args: should be "<name> ...>"``
(proc-old-30.3).  ``eval_proc_call_bucket`` in
``runtime/zig/interp/tcl_interp.zig`` previously only checked the
too-FEW case (a required parameter with no supplied value), so extra
arguments were silently dropped.  Mirrors ``ProcWrongNumArgs`` in
tclProc.c.

The slice runs test bodies INTERPRETED (``eval_script``), so the procs
below are defined and called through that path via
``set s {…}; eval $s``.
"""

from __future__ import annotations

import pytest

pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402


def _run_interp(source: str) -> str:
    wrapped = "set __s {" + source + "}\neval $__s\n"
    _, stdout = _run_wasm(_compile_tcl(wrapped), capture_stdout=True)
    return stdout.rstrip("\n")


@pytest.mark.parametrize(
    "source,expected",
    [
        # Exact arg count — fine.
        ("proc t {x y z} {list $x $y $z}\nputs [t 1 2 3]", "1 2 3"),
        # Too many — error with the full parameter signature (proc-old-30.3).
        (
            'proc t {x y z} {list $x $y $z}\nputs "[catch {t 1 2 3 4} m]:$m"',
            '1:wrong # args: should be "t x y z"',
        ),
        # ``args`` collector accepts any number of trailing arguments.
        ("proc t {a args} {list $a $args}\nputs [t 1 2 3 4]", "1 {2 3 4}"),
        # Zero-parameter proc rejects any argument.
        ("proc t {} {return ok}\nputs [t]", "ok"),
        (
            'proc t {} {return ok}\nputs "[catch {t x} m]:$m"',
            '1:wrong # args: should be "t"',
        ),
        # Defaulted trailing parameter still rejects a true excess and
        # renders as ``?y?`` in the signature.
        ("proc t {x {y 9}} {list $x $y}\nputs [t 1 2]", "1 2"),
        (
            'proc t {x {y 9}} {list $x $y}\nputs "[catch {t 1 2 3} m]:$m"',
            '1:wrong # args: should be "t x ?y?"',
        ),
        # Too FEW still errors (unchanged regression guard).
        (
            'proc t {x y z} {list $x $y $z}\nputs "[catch {t 1 2} m]:$m"',
            '1:wrong # args: should be "t x y z"',
        ),
    ],
)
def test_proc_call_arity(source: str, expected: str) -> None:
    assert _run_interp(source) == expected
