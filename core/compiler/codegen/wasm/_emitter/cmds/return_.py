"""WASM emit hook for ``return``."""

from __future__ import annotations

from ......commands.registry import REGISTRY, EmitContext


def _emit_return(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    """``return ?-code code? ?value?`` — delegates to _emit_cmd_return."""
    if context is EmitContext.VALUE:
        # Tail-context not yet migrated — handled inline in _statements.py.
        return False
    emitter._emit_cmd_return(args)
    return True


REGISTRY.register_wasm_emitter("return", _emit_return)
