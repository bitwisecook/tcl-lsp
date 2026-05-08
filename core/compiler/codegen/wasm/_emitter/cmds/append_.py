"""WASM emit hook for ``append`` — variadic mutator onto a string variable.

``append var v1 ?v2 ...?`` reads the current value of ``var``, calls
``tcl_cmd_append(cur, vN)`` once per value, and writes the running
result back between iterations.  In VALUE context the final write
keeps the updated value on the stack for implicit return.
"""

from __future__ import annotations

from ......commands.registry import REGISTRY, EmitContext
from ..._parsing import _parse_array_ref
from .._variables import _is_dynamic_var_name


def _emit_append(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    if len(args) < 2:
        return False
    prep = emitter._runtime_prep("append", args)
    if prep is None:
        return False
    func_idx, _rimp = prep

    var_name = args[0]
    array_ref = _parse_array_ref(var_name)
    # Route array-element writes (``arr(key)``) and alias writes through
    # the variable subsystem — ``_intern_local`` would otherwise make a
    # scalar slot named ``arr(key)`` and miss the array hash table
    # entirely.  ``var in _aliases`` covers direct aliases
    # (``upvar 0 target var``); the array-ref branch covers the
    # unaliased array-element case and the aliased-base case
    # (``upvar 0 srcArr a; append a(key) …``).
    #
    # Top-level vars also use the var path: ``_emit_var_read_obj`` at
    # top level routes through ``tcl_global_get`` (the WASM-local
    # mirror is bypassed so eval-fallback writes stay visible — see
    # _variables.py Phase 4.5 finalisation).  A bare ``local_set`` at
    # top level would update the WASM local but leave the global
    # stale, so subsequent ``$var`` reads see the pre-append value.
    #
    # ``global``-declared names inside a proc also need the var path
    # so the append's writeback reaches the global table.  Without
    # this branch, ``proc f {} { global a ; append a x }`` only
    # updated the WASM-local mirror; the global stayed at its
    # pre-append value (the bug ``hello_world`` exposed).
    at_top_level = not emitter._is_proc
    is_global = var_name in emitter._globals
    use_var_path = (
        at_top_level
        or is_global
        or var_name in emitter._aliases
        or array_ref is not None
        or (array_ref is None and "(" in var_name and var_name.split("(")[0] in emitter._aliases)
        or _is_dynamic_var_name(var_name)
    )
    keep_last = context is EmitContext.VALUE
    last_index = len(args) - 1

    if use_var_path:
        for i, value_arg in enumerate(args[1:], start=1):
            emitter._emit_var_read_obj(var_name)
            emitter._emit_value(value_arg)
            emitter._emit_call(func_idx)
            if keep_last and i == last_index:
                emitter._emit_var_write_obj_keep(var_name)
            else:
                emitter._emit_var_write_obj(var_name)
    else:
        var_idx = emitter._intern_local(var_name)
        for i, value_arg in enumerate(args[1:], start=1):
            emitter._emit_local_get(var_idx)
            emitter._emit_value(value_arg)
            emitter._emit_call(func_idx)
            if keep_last and i == last_index:
                emitter._emit_local_tee(var_idx)
            else:
                emitter._emit_local_set(var_idx)

    return True


REGISTRY.register_wasm_emitter("append", _emit_append)
