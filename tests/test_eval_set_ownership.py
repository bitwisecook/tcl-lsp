"""Stress test for eval_set read-form ownership fix.

Exercises the interpreted eval_set read form (via dynamic var names so
codegen can't fold into a direct frame read). After the fix, each
[set $varname] substitution must return a +1 owned handle and not
dangle the var slot's value.
"""
import wasmtime
import pytest

from tests.test_wasm_execution import (
    _compile_to_wasm,
    _get_engine,
    _link_and_instantiate,
)


def test_repeated_set_read_via_dynamic_name():
    """Stress: repeatedly read a global through a dynamic var name.

    `set $name` is the read form (no value) with a dynamic name —
    the compiler can't fold this and routes via the eval-fallback
    path through eval_set. Before the fix, each iteration's read
    would queue the slot's value for free, and the next iteration
    would see a freed handle.
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
    tcl_instance, rt_instance = _link_and_instantiate(store, wasm_bytes)
    top_func = tcl_instance.exports(store).get("::top")
    if top_func is not None:
        top_func(store)
    # Survival is the test — if the slot dangled we'd trap or read garbage.


def test_eval_set_via_eval_command():
    """eval triggers the interpreter path, not codegen.

    `eval "set x"` parses the body string and routes through
    eval_set's read form. After the fix, the result must be +1
    owned and not corrupt the slot.
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
    tcl_instance, rt_instance = _link_and_instantiate(store, wasm_bytes)
    top_func = tcl_instance.exports(store).get("::top")
    if top_func is not None:
        top_func(store)


if __name__ == "__main__":
    test_repeated_set_read_via_dynamic_name()
    print("repeated dynamic-name set: OK")
    test_eval_set_via_eval_command()
    print("eval set: OK")
