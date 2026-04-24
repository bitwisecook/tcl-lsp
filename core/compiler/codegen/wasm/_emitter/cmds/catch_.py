"""WASM emit hook for ``catch``."""

from __future__ import annotations

from ......commands.registry import REGISTRY, EmitContext


def _emit_catch(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    """``catch {body} ?resultVar?`` — re-parse body and emit with error flag.

    Statement context drops the 0/1 return code; tail position keeps it on
    the operand stack for implicit return.
    """
    if not args:
        return False
    keep = context is EmitContext.VALUE
    emitter._emit_catch_from_args(args, defs, keep_on_stack=keep)
    return True


REGISTRY.register_wasm_emitter("catch", _emit_catch)
