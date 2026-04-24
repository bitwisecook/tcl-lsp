"""WASM emit hooks for ``set`` and ``incr``."""

from __future__ import annotations

from ......commands.registry import REGISTRY, EmitContext
from ..._ir import WasmOp
from ..._parsing import _parse_array_ref


def _emit_set(emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext) -> bool:
    """``set varName ?value?`` — read or write a local/global/aliased variable."""
    if not (1 <= len(args) <= 2):
        return False
    var = args[0]

    if context is EmitContext.VALUE:
        # At tail / value position the call's result must stay on the
        # stack.  Fast-path proc-locals via ``local.tee``; for aliases,
        # array refs, and unqualified names inside a ``namespace eval``
        # body, route through the full keep-on-stack write.
        in_ns_block = (
            not emitter._is_proc
            and emitter._block_namespace is not None
            and emitter._block_namespace != "::"
            and _parse_array_ref(var) is None
        )
        if var in emitter._aliases:
            if len(args) >= 2:
                emitter._emit_value(args[1])
                emitter._emit_var_write_obj_keep(var)
            else:
                emitter._emit_var_read_obj(var)
            return True
        if in_ns_block:
            if len(args) >= 2:
                emitter._emit_value(args[1])
                emitter._emit_var_write_obj_keep(var)
            else:
                emitter._emit_var_read_obj(var)
            return True
        idx = emitter._intern_local(var)
        if len(args) >= 2:
            emitter._emit_value(args[1])
            emitter._emit_local_tee(idx)
        else:
            emitter._emit_local_get(idx)
        return True

    # STATEMENT context — result is either stored into ``defs[0]`` or
    # dropped.  ``_emit_var_write_obj`` already resolves aliases,
    # array refs, namespace qualification, and frame-only vars.
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

    if context is EmitContext.VALUE:
        # Tail / value position: leave the new value on the stack.
        # Fast-path proc-local incr with ``local.tee``; otherwise read
        # via the var path, add, and write with keep.
        in_ns_block = (
            not emitter._is_proc
            and emitter._block_namespace is not None
            and emitter._block_namespace != "::"
            and _parse_array_ref(var) is None
        )
        array_ref = _parse_array_ref(var)
        base = array_ref[0] if array_ref else var
        if var in emitter._aliases or base in emitter._aliases or in_ns_block:
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
                    emitter._emit_var_write_obj_keep(var)
                    return True
            emitter._emit_i64_const(amt)
            emitter._emit(WasmOp.I64_ADD)
            emitter._emit_box_int()
            emitter._emit_var_write_obj_keep(var)
            return True
        idx = emitter._intern_local(var)
        emitter._emit_local_get(idx)
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
                emitter._emit_local_tee(idx)
                return True
        emitter._emit_i64_const(amt)
        emitter._emit(WasmOp.I64_ADD)
        emitter._emit_box_int()
        emitter._emit_local_tee(idx)
        return True

    # STATEMENT context — existing behaviour.
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
