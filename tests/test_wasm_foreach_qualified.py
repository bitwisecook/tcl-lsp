"""WASM ``foreach`` / ``lmap`` with namespace-qualified loop variables.

tclsh 9.0 falls back to a generic invoke (rather than the inlined
``INST_FOREACH`` bytecode) when a ``foreach`` / ``lmap`` loop variable is
namespace-qualified (``foreach ::v list body``).  Our CFG mirrors that by
lowering such a loop to an opaque command instead of the inlined
``_emit_foreach_loop`` pattern.

Pre-fix that opaque command was an ``IRCall(command="foreach")``.  Because
``foreach`` carries ``wasm_emits_nothing=True`` in the registry (the
*inlined* loop's synthetic header def-marker is a no-op since the body is
emitted structurally), ``_emit_call_stmt`` hit its ``command_emits_nothing``
short-circuit and emitted **nothing** — the loop ran zero iterations and the
loop variable was never set.  In ``ioCmd.test`` this surfaced as a hard WASM
trap: ``foreach ::writeErr {0 1} {...}; unset ::writeErr`` left ``::writeErr``
unset, so the trailing ``unset`` raised ``can't unset "::writeErr": no such
variable`` and aborted the whole bundle before tcltest could print its
summary.

The fix routes the fallback through an ``IRBarrier`` (bypassing the
``emits_nothing`` fast-path, like the sibling ``dict for`` / ``dict map``
case) and threads the original command tokens through ``IRForeach.tokens`` so
the eval-fallback reconstructs the braced varLists / lists / body verbatim.
"""

from __future__ import annotations

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402


def _run(source: str) -> str:
    wasm = _compile_tcl(source)
    _, stdout = _run_wasm(wasm, capture_stdout=True)
    return stdout.strip()


def test_foreach_qualified_var_iterates() -> None:
    """``foreach ::x list body`` must run once per element — pre-fix it ran
    zero times because the opaque ``foreach`` call was swallowed by
    ``command_emits_nothing``."""
    out = _run(
        """
set n 0
foreach ::x {0 1 2} { incr n }
puts $n
"""
    )
    assert out == "3"


def test_foreach_qualified_var_left_set_after_loop() -> None:
    """The qualified loop var survives the loop set to the last element, so a
    trailing ``unset`` on it succeeds — the exact ``ioCmd.test`` shape that
    trapped the bundle (``foreach ::writeErr {0 1} {}; unset ::writeErr``)."""
    out = _run(
        """
foreach ::writeErr {0 1} { }
puts "exists=[info exists ::writeErr] value=$::writeErr"
unset ::writeErr
puts "exists=[info exists ::writeErr]"
"""
    )
    assert out == "exists=1 value=1\nexists=0"


def test_foreach_qualified_var_body_reads_var() -> None:
    """The body sees the per-iteration binding through the qualified name."""
    out = _run(
        """
set acc {}
foreach ::x {a b c} { append acc $::x }
puts $acc
"""
    )
    assert out == "abc"


def test_foreach_qualified_var_braced_body_preserved() -> None:
    """A braced body with substitutions / backslashes re-parses verbatim —
    the tokens threaded through ``IRForeach`` keep the braces intact instead
    of pre-substituting ``\\t`` / mis-parsing a leading ``[``."""
    out = _run(
        r"""
foreach ::x {a b} { puts "x=$::x\tsep" }
"""
    )
    assert out == "x=a\tsep\nx=b\tsep"


def test_foreach_qualified_var_list_substitution() -> None:
    """The list word may be a ``$var`` substitution — it must resolve at
    eval time, not be brace-wrapped into a literal."""
    out = _run(
        """
set L {0 1 2 3}
set n 0
foreach ::x $L { incr n }
puts $n
"""
    )
    assert out == "4"


def test_foreach_qualified_multi_var_group() -> None:
    """``foreach {::a ::b} list`` consumes elements in pairs."""
    out = _run(
        """
set p {}
foreach {::a ::b} {1 2 3 4} { append p "$::a-$::b " }
puts $p
"""
    )
    assert out == "1-2 3-4"


def test_lmap_qualified_var_collects_results() -> None:
    """``lmap`` with a qualified loop var must still collect a result list."""
    out = _run(
        """
set r [lmap ::x {1 2 3} { expr {$::x * $::x} }]
puts $r
"""
    )
    assert out == "1 4 9"


def test_foreach_plain_var_unaffected() -> None:
    """The inlined (non-qualified) fast path is unchanged."""
    out = _run(
        """
set n 0
foreach x {0 1 2 3 4} { incr n }
puts $n
"""
    )
    assert out == "5"
