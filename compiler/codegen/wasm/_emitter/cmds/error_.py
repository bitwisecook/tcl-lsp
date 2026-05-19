"""WASM emit hook for ``error`` — 3-arg form populates ``::errorCode``.

The default 1-arg ``error msg`` form goes through the standard
``tcl_error`` runtime import (= ``tcl_cmd_error``).  The 3-arg
``error msg ?info? ?code?`` form is intercepted here and routed
through ``tcl_cmd_error_full`` so the optional info / code arguments
land in ``::errorInfo`` / ``::errorCode`` — without this the
errorCode reverts to the default ``NONE`` and ``catch ... opt; dict
get $opt -errorcode`` loses the explicit class.
"""

from __future__ import annotations

from compiler.registry import REGISTRY, EmitContext


def _emit_error(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    if not args or len(args) < 2:
        return False
    err_full_idx = emitter._shared_imports.get("tcl_error_full")
    if err_full_idx is None:
        return False
    emitter._emit_value(args[0])
    emitter._emit_value(args[1])
    if len(args) >= 3:
        emitter._emit_value(args[2])
    else:
        emitter._emit_i32_const(0)
    emitter._emit_call(err_full_idx)
    if context is EmitContext.VALUE:
        emitter._emit_i32_const(0)
    return True


REGISTRY.register_wasm_emitter("error", _emit_error)
