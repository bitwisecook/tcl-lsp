"""WASM string-concat / append aliasing contract.

``tcl_cmd_append(acc, x)`` mutates ``acc`` in place on its refcount-1
fast path.  A borrowed ``$var`` read has refcount 1 (held only by the
variable), so any concat chain that uses a borrowed read as its
accumulator and then re-reads the same variable would corrupt the
variable mid-expression — the classic ``$x$x$x`` / ``append x $x $x``
doubling bug (1→2→4→8 chars).

The codegen seeds every multi-operand concat chain with a null
accumulator (``_emit_concat_chain``) so the first ``tcl_cmd_append(0,
op0)`` makes a fresh owned copy and no operand object is ever mutated.
These tests pin that contract across the three affected surfaces:
string interpolation, ``string cat``, and the ``append`` command.

Regression guard for the ``parseOld-11.10-*`` cluster (62 failing
tcltest IDs before the fix).
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
        # Interpolation self-concat — the value must come from an owned
        # buffer (``string index`` yields a cap>0 rc==1 string), which is
        # exactly the object the in-place fast path would mutate.
        (
            "set t [string index 0123456789 5]\nset t $t$t$t$t\nputs $t",
            "5555",
        ),
        # Interpolation into a *different* variable still re-reads the
        # source operand, so it doubles without the seed fix.
        (
            "set p [string index 0123456789 7]\nset q $p$p$p\nputs $q",
            "777",
        ),
        # ``string cat`` of a repeated owned operand.
        (
            "set p [string index abcdef 2]\nset r [string cat $p $p $p]\nputs $r",
            "ccc",
        ),
        # ``append`` command with multiple value args that alias the
        # target variable — re-reads the half-built accumulator.
        (
            "set x [string index 0123456789 5]\nappend x $x $x\nputs $x",
            "555",
        ),
        # Single-value append still works (in-place fast path retained).
        (
            "set s [string index 0123456789 9]\nappend s $s\nputs $s",
            "99",
        ),
        # The canonical append-accumulator loop (distinct piece var) is
        # unaffected — guards against a perf-motivated regression.
        (
            "set acc {}\nforeach c {a b c d} { append acc $c }\nputs $acc",
            "abcd",
        ),
    ],
)
def test_concat_no_operand_mutation(source: str, expected: str) -> None:
    assert _run(source) == expected
