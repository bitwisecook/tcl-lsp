"""WASM emit hook for ``return``."""

from __future__ import annotations

from ......commands.registry import REGISTRY, EmitContext


def _emit_return(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    """``return ?-code code? ?value?`` — delegates to _emit_cmd_return."""
    if context is EmitContext.VALUE:
        # Tail position (rare — the CFG usually collapses ``return``
        # into IRReturn).  Leave the declared value on the stack, or
        # a null TclObj if no argument was given.
        if args:
            emitter._emit_value(args[0])
        else:
            emitter._emit_i32_const(0)
        return True
    emitter._emit_cmd_return(args)
    return True


REGISTRY.register_wasm_emitter("return", _emit_return)
