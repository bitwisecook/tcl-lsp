"""WASM emit hook for ``lreplace``.

Multi-value ``lreplace LIST FIRST LAST v1 v2 ...`` routes through
the dedicated ``tcl_cmd_lreplace_list`` runtime helper — the
inserts are packed into a Tcl list at the call site and the
helper splices them at absolute positions resolved against the
ORIGINAL list, so ``end-N`` does not drift slot-by-slot through
the chain (lreplace-4.7.1 / lreplace-4.11).  Single-value and
zero-value forms fall through to the dedicated
``tcl_cmd_list_replace`` runtime import.
"""

from __future__ import annotations

from ......commands.registry import REGISTRY, EmitContext


def _emit_lreplace(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    if len(args) <= 4:
        return False
    prep = emitter._runtime_prep("lreplace", args)
    if prep is None:
        return False
    func_idx, rimp = prep

    list_arg = args[0]
    first_arg = args[1]
    last_arg = args[2]
    values = args[3:]
    lr_list_idx = emitter._shared_imports.get("tcl_cmd_lreplace_list")
    list_create_idx = emitter._shared_imports.get("tcl_list_create")
    if lr_list_idx is None or list_create_idx is None or not values:
        emitter._emit_value(list_arg)
        emitter._emit_value(first_arg)
        emitter._emit_value(last_arg)
        emitter._emit_value(values[0] if values else "")
        emitter._emit_call(func_idx)
    else:
        emitter._emit_value(list_arg)
        emitter._emit_value(first_arg)
        emitter._emit_value(last_arg)
        emitter._emit_obj_literal("")
        for v in values:
            emitter._emit_value(v)
            emitter._emit_call(list_create_idx)
        emitter._emit_call(lr_list_idx)
    emitter._runtime_call_end(rimp, defs, context)
    return True


REGISTRY.register_wasm_emitter("lreplace", _emit_lreplace)
