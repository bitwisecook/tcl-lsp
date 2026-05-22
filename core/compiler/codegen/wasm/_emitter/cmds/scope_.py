"""WASM emit hooks for ``global``, ``upvar``, and ``variable``."""

from __future__ import annotations

from ......commands.registry import REGISTRY, EmitContext


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
        # Register the same-name global alias in the runtime frame too
        # (mirrors what ``upvar`` / ``variable`` already do via
        # ``frame_alias_*``).  The compiled read/write fast paths use the
        # WASM-local mirror, so this is mainly for the interpreter-side
        # dispatch that consults the frame: e.g. ``trace add variable x``
        # must see ``x`` as a global so the trace lands in the global
        # directory and survives the proc's return (trace-17.2 / 17.3).
        # No-op at top level (``frame_alias_global`` bails when no frame
        # is active), so only worth emitting inside a proc body.
        if emitter._is_proc:
            alias_idx = emitter._shared_imports.get("tcl_frame_alias_global")
            if alias_idx is not None:
                emitter._emit_obj_literal(var_name)
                emitter._emit_call(alias_idx)
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
