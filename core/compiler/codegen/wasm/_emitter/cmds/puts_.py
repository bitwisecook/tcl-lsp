"""WASM emit hook for ``puts`` — channel write.

``puts ?-nonewline? ?channelId? string`` — the ``-nonewline``-only
form dispatches to a separate runtime helper that suppresses the
trailing newline.  When the call shape includes a channel argument
(``puts $fd "..."``) we fall back to the interpreter so the
runtime can route through ``tcl_cmd_puts_chan`` — the codegen fast
path only models the stdout case and would otherwise drop the
channel id silently.
"""

from __future__ import annotations

from ......commands.registry import REGISTRY, EmitContext


def _emit_puts(emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext) -> bool:
    nonewline = len(args) >= 1 and args[0] == "-nonewline"
    # Channel-id form: 2 args without -nonewline, or 3 args with it.
    # Routing those through the interpreter lets ``eval_puts`` pick the
    # right ``tcl_cmd_puts_chan`` / ``tcl_cmd_puts_nonewline`` shape.
    has_channel = (not nonewline and len(args) >= 2) or (nonewline and len(args) >= 3)
    if has_channel:
        return False

    prep = emitter._runtime_prep("puts", args)
    if prep is None:
        return False
    func_idx, rimp = prep

    if nonewline:
        no_nl_idx = emitter._shared_imports.get("tcl_puts_nonewline")
        if no_nl_idx is not None:
            emitter._emit_value(args[-1])
            emitter._emit_call(no_nl_idx)
            emitter._runtime_call_end(rimp, defs, context)
            return True
    if args:
        emitter._emit_value(args[-1])
    else:
        emitter._emit_i32_const(0)
    emitter._emit_call(func_idx)
    emitter._runtime_call_end(rimp, defs, context)
    return True


REGISTRY.register_wasm_emitter("puts", _emit_puts)
