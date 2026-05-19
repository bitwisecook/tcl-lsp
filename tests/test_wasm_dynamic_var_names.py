"""WASM execution tests for variable commands with dynamic var names.

Also includes a focused unit test for :func:`_is_dynamic_var_name`,
the predicate the emitter uses to decide between the static-name
fast path and the runtime-resolve path.

Covers ``set``, ``append``, ``incr``, ``lappend`` invoked with a name
token that contains a substitution (``$x`` / ``${x}`` / ``[cmd]``) and
must therefore be resolved at runtime — never collapsed to a literal
WASM-local slot keyed on the canonicalised dynamic-name string.

Pinned regressions:

* The trace.test ``traceproc`` shape (``proc p {n args} { append ::$n
  * }``) used to trip the ``var-write contract`` assertion in
  ``core/compiler/codegen/wasm/_emitter/_core.py`` because the
  fast-path in ``_emit_append`` interned a WASM-local named
  ``::${n}`` and wrote to it via ``local.tee`` — bypassing
  ``tcl_var_set`` and ignoring the runtime-substituted name entirely.

* ``set ::$name v`` / ``append ::$name v`` / ``incr ::$name`` /
  ``lappend ::$name x`` from inside a proc must store into the
  global named *by the substituted value of* ``$name``, not into a
  global literally named ``::${name}``.

* ``set $name v`` for a proc-local dynamic name resolves through the
  current frame.

These tests fail against the pre-fix runtime (compile assertion or
wrong-target write) and must pass once the codegen routes dynamic var
names through ``tcl_var_set`` / ``tcl_var_resolve``.
"""

from __future__ import annotations

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from compiler.codegen.wasm._emitter._variables import _is_dynamic_var_name  # noqa: E402
from tests.test_wasm_execution import (  # noqa: E402
    _compile_and_run_proc,
    _compile_to_wasm,
    _get_engine,
    _link_and_instantiate,
    _read_global_string,
)


class TestIsDynamicVarName:
    """``_is_dynamic_var_name`` separates names that can fast-path
    via static interning from names that must round-trip through
    ``tcl_var_set`` / ``tcl_var_resolve``."""

    def test_plain_scalar_is_static(self):
        assert not _is_dynamic_var_name("x")
        assert not _is_dynamic_var_name("my_var")
        assert not _is_dynamic_var_name("::ns::name")

    def test_empty_name_is_dynamic(self):
        # An empty IR name comes from a malformed write target — be
        # conservative and route through the runtime path.
        assert _is_dynamic_var_name("")

    def test_substituted_name_is_dynamic(self):
        assert _is_dynamic_var_name("$x")
        assert _is_dynamic_var_name("${x}")
        assert _is_dynamic_var_name("::$x")
        assert _is_dynamic_var_name("::${x}")
        assert _is_dynamic_var_name("[gen_name]")
        assert _is_dynamic_var_name("prefix-${x}-suffix")

    def test_array_element_with_dynamic_key_is_static(self):
        # ``a($i)`` — array name is literal, only the key is dynamic.
        # The array write/read path already evaluates the key via
        # ``_emit_value`` and dispatches through ``tcl_array_*``.
        # Routing it through the dynamic var path would skip the
        # array hash table and write a scalar named e.g. ``a(3)``.
        assert not _is_dynamic_var_name("a($i)")
        assert not _is_dynamic_var_name("arr(${k})")
        assert not _is_dynamic_var_name("::ns::arr([f x])")

    def test_array_element_with_dynamic_array_name_is_dynamic(self):
        # ``${arrname}(key)`` — array identity is dynamic.
        assert _is_dynamic_var_name("$arr(key)")
        assert _is_dynamic_var_name("${arr}(key)")


def _run_and_read_global(source: str, name: str) -> bytes:
    """Compile, run ``::top``, and return the named global's string rep."""
    _, wasm_bytes = _compile_to_wasm(source)
    engine = _get_engine()
    store = wasmtime.Store(engine)
    store.set_wasi(wasmtime.WasiConfig())
    tcl_instance, rt_instance = _link_and_instantiate(store, wasm_bytes)
    top = tcl_instance.exports(store).get("::top")
    if top is not None:
        top(store)
    return _read_global_string(rt_instance, store, name)


class TestSetDynamicGlobalName:
    """``set ::$name value`` from inside a proc writes to the resolved global."""

    def test_set_resolves_substituted_name(self):
        """``proc p {n v} { set ::$n $v }`` writes to ``::<n>``."""
        result = _run_and_read_global(
            """\
proc setdyn {n v} { set ::$n $v }
setdyn target 42
""",
            "::target",
        )
        assert result == b"42"

    def test_set_does_not_create_literal_dollar_brace_global(self):
        """The literal name ``::${n}`` must NOT be created as a global —
        only the substituted target should exist."""
        result = _run_and_read_global(
            """\
proc setdyn {n v} { set ::$n $v }
setdyn realtarget 99
""",
            "::${n}",
        )
        assert result == b""


class TestAppendDynamicGlobalName:
    """``append ::$name suffix`` from inside a proc appends to the resolved global.

    This is the trace.test ``traceproc`` shape — the immediate
    blocker for the trace stem in the WASM core slice.
    """

    def test_append_resolves_substituted_name(self):
        """The traceproc-from-trace.test minimal shape compiles and runs."""
        result = _run_and_read_global(
            """\
set ::tracevar ""
proc traceproc {tracevar args} {
    append ::$tracevar *
}
traceproc tracevar one two three
traceproc tracevar four
""",
            "::tracevar",
        )
        assert result == b"**"

    def test_append_writes_to_resolved_target(self):
        """``proc p {n s} { append ::$n $s }`` appends to ``::<n>``."""
        result = _run_and_read_global(
            """\
set ::greeting "hello"
proc appdyn {n s} { append ::$n $s }
appdyn greeting " world"
""",
            "::greeting",
        )
        assert result == b"hello world"

    def test_append_does_not_create_literal_dollar_brace_global(self):
        """The literal name ``::${n}`` must NOT be created — the append
        must land on the resolved target only, not on a global keyed
        by the unsubstituted brace canonicalisation of the name."""
        result = _run_and_read_global(
            """\
set ::realtarget ""
proc appdyn {n s} { append ::$n $s }
appdyn realtarget data
""",
            "::${n}",
        )
        assert result == b""


class TestIncrDynamicGlobalName:
    """``incr ::$name`` from inside a proc increments the resolved global."""

    def test_incr_resolves_substituted_name(self):
        result = _run_and_read_global(
            """\
set ::counter 0
proc bumpdyn {n} { incr ::$n }
bumpdyn counter
bumpdyn counter
bumpdyn counter
""",
            "::counter",
        )
        assert result == b"3"


class TestLappendDynamicGlobalName:
    """``lappend ::$name elt`` from inside a proc appends to the resolved global list."""

    def test_lappend_resolves_substituted_name(self):
        result = _run_and_read_global(
            """\
set ::items {}
proc adddyn {n e} { lappend ::$n $e }
adddyn items a
adddyn items b
adddyn items c
""",
            "::items",
        )
        assert result == b"a b c"


class TestProcLocalDynamicName:
    """``set $name`` (no ``::``) writes to a proc-local resolved by the current frame."""

    def test_set_local_dynamic_name_round_trip(self):
        """``proc p {n v} { set $n $v ; return [set $n] }`` — round-trip
        the substituted local name."""
        result = _compile_and_run_proc(
            """\
proc roundtrip {} {
    set name "x"
    set $name 42
    return [set $name]
}
""",
            "roundtrip",
            (),
        )
        assert result == 42
