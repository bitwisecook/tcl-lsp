"""WASM emit hooks for ``global``, ``upvar``, and ``variable``."""

from __future__ import annotations

from compiler.registry import REGISTRY, EmitContext


def _emit_global(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    """``global varName ?varName ...?`` — register variable as global-scoped.

    Pre-loads the global value into a local slot so downstream reads can
    fast-path through the local without a ``tcl_global_get`` call.  Returns
    the empty string (null TclObj in value context).
    """
    for var_name in args:
        emitter._globals.add(var_name)
        gget_idx = emitter._shared_imports.get("tcl_global_get")
        if gget_idx is not None:
            local_idx = emitter._intern_local(var_name)
            emitter._emit_obj_literal(var_name)
            emitter._emit_call(gget_idx)
            emitter._emit_local_set(local_idx)
    if context is EmitContext.VALUE:
        emitter._emit_i32_const(0)
    return True


def _emit_upvar(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    """``upvar ?level? otherVar myVar ?otherVar myVar ...?``."""
    emitter._emit_cmd_upvar(args)
    if context is EmitContext.VALUE:
        emitter._emit_i32_const(0)
    return True


def _emit_variable(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    """``variable name ?value? ?name value ...?``."""
    emitter._emit_cmd_variable(args)
    if context is EmitContext.VALUE:
        emitter._emit_i32_const(0)
    return True


REGISTRY.register_wasm_emitter("global", _emit_global)
REGISTRY.register_wasm_emitter("upvar", _emit_upvar)
REGISTRY.register_wasm_emitter("variable", _emit_variable)
