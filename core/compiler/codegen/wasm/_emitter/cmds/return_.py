"""WASM emit hook for ``return``."""

from __future__ import annotations

from ......commands.registry import REGISTRY


def _emit_return(emitter, args: tuple[str, ...], defs: tuple[str, ...]) -> bool:
    """``return ?-code code? ?value?`` — delegates to _emit_cmd_return."""
    emitter._emit_cmd_return(args)
    return True


REGISTRY.register_wasm_emitter("return", _emit_return)
