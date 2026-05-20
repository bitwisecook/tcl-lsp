"""Regression test for eval_set read-form ownership fix.

Exercises the interpreted eval_set read form (via dynamic var names so
codegen can't fold into a direct frame read).  After the fix, every
command handler hands the dispatcher a true +1 owned share regardless
of whether the underlying value came from a var slot, a fresh
allocation, or a parser-allocated word.

Before the fix the read form returned the var slot's stored handle
without bumping its refcount; the eval_script loop's release of the
previous statement's result would decrement the slot's hold to zero
and queue the obj for free.  In practice the deferred-free queue
masked the bug (drains fire only at the outermost ``tcl_eval``
boundary), but the rc accounting was lying — every borrow-return
sequence was one resize or hash-table rebuild away from a use-after-
free.
"""

from __future__ import annotations

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_execution import (  # noqa: E402
    _compile_to_wasm,
    _get_engine,
    _link_and_instantiate,
)


def test_repeated_set_read_via_dynamic_name():
    """Stress: repeatedly read a global through a dynamic var name.

    ``set $name`` is the read form (no value) with a dynamic name —
    the compiler can't fold this and routes via the eval-fallback
    path through ``eval_set``.  Before the fix each iteration's read
    queued the slot's value for free, and the next iteration could
    observe a recycled slab if the allocator reissued it before the
    drain fired.
    """
    src = """\
set persistent [string repeat "alphabet" 50]
proc readit {n} { set $n }
set tmp ""
for {set i 0} {$i < 100} {incr i} {
    set tmp [readit persistent]
}
"""
    _, wasm_bytes = _compile_to_wasm(src)

    engine = _get_engine()
    store = wasmtime.Store(engine)
    wasi_config = wasmtime.WasiConfig()
    store.set_wasi(wasi_config)
    tcl_instance, _ = _link_and_instantiate(store, wasm_bytes)
    top_func = tcl_instance.exports(store).get("::top")
    if top_func is not None:
        top_func(store)
    # Survival is the test — a dangling slot would trap or read garbage
    # somewhere in the 100-iteration loop.


def test_eval_set_via_eval_command():
    """``eval`` triggers the interpreter path, not codegen.

    ``eval "set x"`` parses the body string and routes through
    ``eval_set``'s read form.  After the fix the result is a true +1
    owned handle and the outer ``set y`` write doesn't trip on a
    freed handle.
    """
    src = """\
set x "hello world"
set y [eval "set x"]
"""
    _, wasm_bytes = _compile_to_wasm(src)
    engine = _get_engine()
    store = wasmtime.Store(engine)
    wasi_config = wasmtime.WasiConfig()
    store.set_wasi(wasi_config)
    tcl_instance, _ = _link_and_instantiate(store, wasm_bytes)
    top_func = tcl_instance.exports(store).get("::top")
    if top_func is not None:
        top_func(store)
