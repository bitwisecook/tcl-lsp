"""Regression test for the compiled-codegen switch over-release.

The CFG builder for ``-exact`` ``switch`` lowers each arm into a
``CFGBranch`` whose condition is
``ExprBinary(STR_EQ, ExprRaw(subject), ExprLiteral(pattern))``.  The
codegen helper :meth:`_emit_expr_obj_cmp_binary` (in
``_emitter/_expressions.py``) stashed each operand in a scratch local
and blanket-released both after the cmp call.  It only retained
*borrowed* operands first, and recognised borrowed-ness via
:meth:`_expr_node_is_borrowed`, which returned True solely for
``ExprVar`` — not for ``ExprRaw`` even when the raw text resolved to
a bare ``$var`` via ``_emit_value``.

The result: every non-matching arm released the subject's TclObj
without a matching retain.  N arms before the match ⇒ N over-releases
per call ⇒ ``double_free_pending`` grows linearly with iteration count
and arm position.  The fix propagates the ownership tag from
``_emit_str_value`` so the helper retains the borrowed handle before
stashing it.

Requires the leak-check runtime (``make build-runtime-leakcheck``);
skips otherwise.
"""

from __future__ import annotations

import os
import sys
import tempfile
from pathlib import Path

import pytest
import wasmtime

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO))

from shared.runtime_wasm import DEFAULT_PATH as _RUNTIME_WASM  # noqa: E402

os.environ.setdefault("TCL_LSP_RUNTIME_WASM", str(_RUNTIME_WASM))

from tests.test_wasm_real_tcl import (  # noqa: E402
    _compile_tcl,
    _define_call_compiled_proc,
    _get_engine_with_timeout,
)


def _has_leakcheck_exports() -> bool:
    """True when the runtime binary carries the leak-check counters."""
    if not _RUNTIME_WASM.exists():
        return False
    engine = wasmtime.Engine()
    module = wasmtime.Module.from_file(engine, str(_RUNTIME_WASM))
    names = {imp.name for imp in module.exports}
    return "tcl_test_double_free_pending_count" in names


pytestmark = pytest.mark.skipif(
    not _has_leakcheck_exports(),
    reason="leak-check runtime exports missing — run 'make build-runtime-leakcheck'",
)


def _call_counter(exports, name: str, store: wasmtime.Store) -> int:
    """Look up a counter-export by *name*, assert it's a Func, call it,
    and coerce the result to ``int``.

    The leak-check counter exports all have signature ``() -> i32``,
    so the call result rounds-trips via ``int()`` cleanly.  This
    tiny wrapper exists to satisfy ty's union-type narrowing — the
    subscript on ``exports`` returns
    ``Func | Table | Memory | SharedMemory | Global`` and only
    ``Func`` is callable.
    """
    fn = exports[name]
    assert isinstance(fn, wasmtime.Func), f"{name!r} is not a Func: {type(fn)}"
    result = fn(store)
    assert result is not None, f"{name!r} returned None"
    return int(result)  # type: ignore[call-overload]


def _call_void(exports, name: str, store: wasmtime.Store) -> None:
    """Call a void-returning export by *name*.  See :func:`_call_counter`
    for the narrowing rationale.
    """
    fn = exports[name]
    assert isinstance(fn, wasmtime.Func), f"{name!r} is not a Func: {type(fn)}"
    fn(store)


def _run_under_leakcheck(tcl_source: str) -> dict[str, int]:
    """Compile *tcl_source*, run ``::top``, return the leak-check counters."""
    wasm_bytes = _compile_tcl(tcl_source, frame_elision=True)
    engine = _get_engine_with_timeout()
    store = wasmtime.Store(engine)
    store.set_epoch_deadline(1_000_000_000)
    wasi_config = wasmtime.WasiConfig()
    with tempfile.TemporaryDirectory(prefix="leakrep-") as host_tmp:
        wasi_config.preopen_dir(host_tmp, "/")
        store.set_wasi(wasi_config)
        rt_module = wasmtime.Module.from_file(engine, str(_RUNTIME_WASM))
        linker = wasmtime.Linker(engine)
        linker.define_wasi()
        tcl_box, mem_box, rt_box = _define_call_compiled_proc(linker, store)
        rt_inst = linker.instantiate(store, rt_module)
        rt_box[0] = rt_inst
        init = rt_inst.exports(store).get("_initialize")
        if init is not None and isinstance(init, wasmtime.Func):
            init(store)
        for export in rt_module.exports:
            name = export.name
            if name.startswith("__"):
                continue
            val = rt_inst.exports(store)[name]
            if isinstance(val, wasmtime.Func):
                linker.define(store, "tcl", name, val)
            elif name == "memory":
                linker.define(store, "tcl", name, val)
        exports = rt_inst.exports(store)
        _call_void(exports, "tcl_test_reset_counters", store)
        user_module = wasmtime.Module(engine, wasm_bytes)
        user_inst = linker.instantiate(store, user_module)
        tcl_box[0] = user_inst
        mem_box[0] = exports["memory"]
        _call_void(user_inst.exports(store), "::top", store)
        residual = _call_counter(exports, "tcl_test_finalize", store)
        return {
            "residual": residual,
            "double_free": _call_counter(exports, "tcl_test_double_free_count", store),
            "pending": _call_counter(exports, "tcl_test_double_free_pending_count", store),
            "poison": _call_counter(exports, "tcl_test_double_free_poison_count", store),
            "bad_rc": _call_counter(exports, "tcl_test_double_free_bad_rc_count", store),
        }


def test_switch_exact_no_pending_overrelease():
    """A ``switch -exact`` whose subject is a variable must not over-release
    the subject's TclObj as arms are tested.

    Before the fix: ``pending`` scaled as ``iters * (matching_arm_index)``.
    After the fix: ``pending == 0`` regardless of arm position.
    """
    src = """
proc test {x} {
    switch $x { a {return 1} b {return 2} c {return 3} d {return 4} e {return 5} }
}
for {set i 0} {$i < 50} {incr i} { test e }
"""
    counters = _run_under_leakcheck(src)
    assert counters["pending"] == 0, (
        f"switch over-released subject TclObj: {counters}.  "
        f"Each non-matching arm before the match released the subject "
        f"without a matching retain — fix in _emit_expr_obj_cmp_binary."
    )
    assert counters["double_free"] == 0, counters


def test_switch_exact_stress_10k():
    """10k-iteration stress: ``pending`` and ``double_free`` stay at 0
    across a much larger workload.  Catches regressions that the
    smaller 50-iter test might miss (e.g. counter overflow, slab
    recycling races).
    """
    src = """
proc test {x} {
    switch $x { a {return 1} b {return 2} c {return 3} d {return 4} e {return 5} }
}
for {set i 0} {$i < 10000} {incr i} { test e }
"""
    counters = _run_under_leakcheck(src)
    assert counters["pending"] == 0, counters
    assert counters["double_free"] == 0, counters


@pytest.mark.parametrize(
    "subject,arm_count_before_match",
    [
        ("a", 0),
        ("b", 1),
        ("c", 2),
        ("d", 3),
        ("e", 4),
    ],
)
def test_switch_no_overrelease_at_each_position(subject, arm_count_before_match):
    """Each match position in the arm list must hold rc discipline.

    The bug's signature was ``pending == iters * arm_count_before_match``;
    by sweeping each position we'd see a positive ``pending`` for any
    position except ``a`` (the first arm) if the regression returned.
    """
    src = f"""
proc test {{x}} {{
    switch $x {{ a {{return 1}} b {{return 2}} c {{return 3}} d {{return 4}} e {{return 5}} }}
}}
for {{set i 0}} {{$i < 100}} {{incr i}} {{ test {subject} }}
"""
    counters = _run_under_leakcheck(src)
    assert counters["pending"] == 0, (
        f"match position {subject!r} (arm index {arm_count_before_match}) over-released: {counters}"
    )
    assert counters["double_free"] == 0, counters


def test_switch_glob_no_pending_overrelease():
    """``-glob`` switches use the inline ``_emit_switch`` path rather
    than CFG lowering.  Verify they also stay clean — even though the
    bug fix targets cmp_binary, future refactors might unify the two
    paths.  (The glob path has unrelated alloc-residual that is not
    asserted on here.)
    """
    src = """
proc test {x} {
    switch -glob $x { a {return 1} b {return 2} c {return 3} d {return 4} e {return 5} }
}
for {set i 0} {$i < 50} {incr i} { test e }
"""
    counters = _run_under_leakcheck(src)
    assert counters["pending"] == 0, counters
    assert counters["double_free"] == 0, counters


def test_switch_fallthrough_arms_no_pending_overrelease():
    """Fallthrough arms route through ``_emit_switch`` (not CFG) but
    use the same ``string_equal`` import.  Verify rc discipline is
    held.
    """
    src = """
proc test {x} {
    switch $x { a - b - c {return 1} d - e {return 2} }
}
for {set i 0} {$i < 50} {incr i} { test e }
"""
    counters = _run_under_leakcheck(src)
    assert counters["pending"] == 0, counters
    assert counters["double_free"] == 0, counters


def test_if_elseif_chain_no_pending_overrelease():
    """``if`` / ``elseif`` chains exercise ``_emit_expr_obj_cmp_binary``
    too, but with ``ExprVar`` operands (not ``ExprRaw``).  The pre-fix
    code path handled this correctly via the ``_expr_node_is_borrowed``
    check — kept here to lock in the equivalence between switch and
    if/elseif now that switch is also clean.
    """
    src = """
proc test {x} {
    if {$x eq "a"} {return 1} \\
    elseif {$x eq "b"} {return 2} \\
    elseif {$x eq "c"} {return 3} \\
    elseif {$x eq "d"} {return 4} \\
    elseif {$x eq "e"} {return 5}
}
for {set i 0} {$i < 50} {incr i} { test e }
"""
    counters = _run_under_leakcheck(src)
    assert counters["pending"] == 0, counters
    assert counters["double_free"] == 0, counters
