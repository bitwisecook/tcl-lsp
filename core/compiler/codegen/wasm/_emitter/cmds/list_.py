"""WASM emit hooks for ``list``, ``lset``, and ``lassign``."""

from __future__ import annotations

from ......commands.registry import REGISTRY, EmitContext
from ..._ir import WasmOp


def _emit_list(emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext) -> bool:
    """``list ?arg ...?`` — variadic list builder; delegates to _emit_list_value."""
    emitter._emit_list_value(args)
    if defs:
        def_idx = emitter._intern_local(defs[0])
        emitter._emit_local_set(def_idx)
    else:
        emitter._emit(WasmOp.DROP)
    return True


def _emit_lset(emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext) -> bool:
    """``lset varName ?index ...? newValue`` — replace a list element."""
    if len(args) < 2:
        emitter._emit_unsupported_trap("lset (too few args)")
        return True

    var_name = args[0]
    new_value = args[-1]
    index_args = args[1:-1]

    list_set_idx = emitter._shared_imports.get("tcl_list_set")
    if list_set_idx is None:
        emitter._emit_unsupported_trap("lset (missing tcl_list_set)")
        return True

    emitter._emit_var_read_obj(var_name)

    if not index_args:
        emitter._emit_obj_literal("")
    elif len(index_args) == 1:
        emitter._emit_value(index_args[0])
    else:
        tcl_list_idx = emitter._shared_imports.get("tcl_list_create")
        if tcl_list_idx is None:
            raise RuntimeError(
                "internal error: lset multi-index requires "
                "tcl_list_create import; scan phase should have "
                "registered it.  Check _scan.py's IRCall path."
            )
        emitter._emit_value(index_args[0])
        for arg in index_args[1:]:
            emitter._emit_value(arg)
            emitter._emit_call(tcl_list_idx)

    emitter._emit_value(new_value)
    emitter._emit_call(list_set_idx)

    if defs:
        emitter._emit_var_write_obj_keep(var_name)
        def_idx = emitter._intern_local(defs[0])
        emitter._emit_local_set(def_idx)
    else:
        emitter._emit_var_write_obj(var_name)
    if emitter._optimise:
        emitter._const_map.pop(var_name, None)
    return True


def _emit_lassign(emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext) -> bool:
    """``lassign list ?varName ...?`` — destructure a list; delegates to _emit_cmd_lassign."""
    if not args:
        return False
    emitter._emit_cmd_lassign(args, defs, keep_on_stack=False)
    return True


REGISTRY.register_wasm_emitter("list", _emit_list)
REGISTRY.register_wasm_emitter("lset", _emit_lset)
REGISTRY.register_wasm_emitter("lassign", _emit_lassign)
