"""WASM emit hook for ``info``."""

from __future__ import annotations

from ......commands.registry import REGISTRY
from ..._ir import WasmOp


def _emit_info(emitter, args: tuple[str, ...], defs: tuple[str, ...]) -> bool:
    """``info subcommand ?arg?`` in statement context — delegates to _emit_info_value."""
    if not args:
        emitter._emit_unsupported_trap("info (no subcommand)")
        return True
    emitter._emit_info_value(args)
    if defs:
        def_idx = emitter._intern_local(defs[0])
        emitter._emit_local_set(def_idx)
    else:
        emitter._emit(WasmOp.DROP)
    return True


REGISTRY.register_wasm_emitter("info", _emit_info)
