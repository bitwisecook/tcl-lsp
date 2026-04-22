"""WASM emit hooks for ``uplevel``, ``clock``, ``array``, and ``unset``."""

from __future__ import annotations

from ......commands.registry import REGISTRY
from ..._ir import WasmOp


def _emit_uplevel(emitter, args: tuple[str, ...], defs: tuple[str, ...]) -> bool:
    """``uplevel ?level? body`` — delegates to _emit_cmd_uplevel."""
    if not args:
        return False
    emitter._emit_cmd_uplevel(args)
    if defs:
        def_idx = emitter._intern_local(defs[0])
        emitter._emit_local_set(def_idx)
    else:
        emitter._emit(WasmOp.DROP)
    return True


def _emit_clock(emitter, args: tuple[str, ...], defs: tuple[str, ...]) -> bool:
    """``clock subcommand ...`` — delegates to _emit_clock_value."""
    if not args:
        return False
    emitter._emit_clock_value(args)
    if defs:
        def_idx = emitter._intern_local(defs[0])
        emitter._emit_local_set(def_idx)
    else:
        emitter._emit(WasmOp.DROP)
    return True


def _emit_array(emitter, args: tuple[str, ...], defs: tuple[str, ...]) -> bool:
    """``array subcommand ...`` — delegates to _emit_array_subcmd_value."""
    if not args:
        return False
    emitter._emit_array_subcmd_value(args)
    if defs:
        def_idx = emitter._intern_local(defs[0])
        emitter._emit_local_set(def_idx)
    else:
        emitter._emit(WasmOp.DROP)
    return True


def _emit_unset(emitter, args: tuple[str, ...], defs: tuple[str, ...]) -> bool:
    """``unset ?-nocomplain? ?--? varName ...`` — array-element unset."""
    if not args:
        return False
    return bool(emitter._emit_unset_array_elems(args))


REGISTRY.register_wasm_emitter("uplevel", _emit_uplevel)
REGISTRY.register_wasm_emitter("clock", _emit_clock)
REGISTRY.register_wasm_emitter("array", _emit_array)
REGISTRY.register_wasm_emitter("unset", _emit_unset)
