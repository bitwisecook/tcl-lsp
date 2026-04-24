"""WASM emit hooks for ``set`` and ``incr``."""

from __future__ import annotations

from ......commands.registry import REGISTRY, EmitContext
from ..._ir import WasmOp


def _emit_set(emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext) -> bool:
    """``set varName ?value?`` — read or write a local/global/aliased variable."""
    if not (1 <= len(args) <= 2):
        return False
    var = args[0]
    if len(args) >= 2:
        emitter._emit_value(args[1])
        emitter._emit_var_write_obj(var)
    else:
        emitter._emit_var_read_obj(var)
        if defs:
            def_idx = emitter._intern_local(defs[0])
            emitter._emit_local_set(def_idx)
        else:
            emitter._emit(WasmOp.DROP)
    return True


def _emit_incr(emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext) -> bool:
    """``incr varName ?increment?`` — unbox, i64 add, rebox."""
    if not (1 <= len(args) <= 2):
        return False
    var = args[0]
    emitter._emit_var_read_obj(var)
    emitter._emit_unbox_int()
    amt = 1
    if len(args) >= 2:
        try:
            amt = int(args[1])
        except ValueError:
            emitter._emit_value(args[1])
            emitter._emit_unbox_int()
            emitter._emit(WasmOp.I64_ADD)
            emitter._emit_box_int()
            if defs:
                emitter._emit_var_write_obj_keep(var)
                def_idx = emitter._intern_local(defs[0])
                emitter._emit_local_set(def_idx)
            else:
                emitter._emit_var_write_obj(var)
            return True
    emitter._emit_i64_const(amt)
    emitter._emit(WasmOp.I64_ADD)
    emitter._emit_box_int()
    if defs:
        emitter._emit_var_write_obj_keep(var)
        def_idx = emitter._intern_local(defs[0])
        emitter._emit_local_set(def_idx)
    else:
        emitter._emit_var_write_obj(var)
    return True


REGISTRY.register_wasm_emitter("set", _emit_set)
REGISTRY.register_wasm_emitter("incr", _emit_incr)
