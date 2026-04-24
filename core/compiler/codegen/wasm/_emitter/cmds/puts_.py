"""WASM emit hook for ``puts`` — channel write.

``puts ?-nonewline? ?channelId? string`` — the ``-nonewline`` form
dispatches to a separate runtime helper that suppresses the trailing
newline.  Channel-id forms (e.g. ``puts stdout foo``) still fall
through to the default ``tcl_cmd_puts`` which writes to the current
stdout channel.  Both paths return the empty string.
"""

from __future__ import annotations

from ......commands.registry import REGISTRY, EmitContext


def _emit_puts(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    prep = emitter._runtime_prep("puts", args)
    if prep is None:
        return False
    func_idx, spec = prep

    nonewline = len(args) >= 2 and args[0] == "-nonewline"
    if nonewline:
        no_nl_idx = emitter._shared_imports.get("tcl_puts_nonewline")
        if no_nl_idx is not None:
            emitter._emit_value(args[-1])
            emitter._emit_call(no_nl_idx)
            emitter._runtime_call_end(spec, defs, context)
            return True
    if args:
        emitter._emit_value(args[-1])
    else:
        emitter._emit_i32_const(0)
    emitter._emit_call(func_idx)
    emitter._runtime_call_end(spec, defs, context)
    return True


REGISTRY.register_wasm_emitter("puts", _emit_puts)
