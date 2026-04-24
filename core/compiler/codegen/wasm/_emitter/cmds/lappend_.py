"""WASM emit hook for ``lappend`` — variadic mutator onto a list variable.

``lappend var v1 ?v2 ...?`` reads the current value of ``var``, calls
``tcl_cmd_lappend(cur, vN)`` once per value, and writes the running
list back between iterations.  In VALUE context the final write
keeps the updated value on the stack for implicit return.
"""

from __future__ import annotations

from ......commands.registry import REGISTRY, EmitContext
from ..._parsing import _parse_array_ref


def _emit_lappend(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    if len(args) < 2:
        return False
    prep = emitter._runtime_prep("lappend", args)
    if prep is None:
        return False
    func_idx, _rimp = prep

    var_name = args[0]
    array_ref = _parse_array_ref(var_name)
    # ``arr(key)`` + alias + aliased-array-base must go through the var
    # subsystem so the write lands in the array hash table; otherwise
    # ``_intern_local`` creates a scalar slot named ``arr(key)``.
    use_var_path = (
        var_name in emitter._aliases
        or array_ref is not None
        or (array_ref is None and "(" in var_name and var_name.split("(")[0] in emitter._aliases)
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


REGISTRY.register_wasm_emitter("lappend", _emit_lappend)
