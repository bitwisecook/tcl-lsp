"""WASM emit hook for ``puts`` — channel write.

``puts ?-nonewline? ?channelId? string`` — three call shapes the
hook handles directly so quoted-string arguments survive
substitution intact:

* ``puts msg``                     → :func:`tcl_cmd_puts`
* ``puts -nonewline msg``          → :func:`tcl_cmd_puts_nonewline`
* ``puts $chan msg``               → :func:`tcl_cmd_puts_chan`
* ``puts -nonewline $chan msg``    → :func:`tcl_cmd_puts_chan` with
                                     the no-newline flag

The channel-aware paths used to fall back to the interpreter, which
round-tripped any quoted-string arg through ``_tcl_list_quote``.
That helper braces words containing spaces / ``$`` / ``[``, and
braced words suppress substitution — so ``puts $chan "==== $name
FAILED"`` reached :func:`tcl_eval` as ``puts $chan {==== $name
FAILED}`` and emitted the literal ``$name``.  Emitting the runtime
import directly skips the round-trip and preserves substitution.
"""

from __future__ import annotations

from compiler.registry import REGISTRY, EmitContext


def _emit_puts(emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext) -> bool:
    prep = emitter._runtime_prep("puts", args)
    if prep is None:
        return False
    func_idx, rimp = prep

    nonewline = len(args) >= 1 and args[0] == "-nonewline"
    chan_form = (not nonewline and len(args) >= 2) or (nonewline and len(args) >= 3)

    if chan_form:
        chan_idx = emitter._shared_imports.get("tcl_puts_chan")
        if chan_idx is not None:
            chan_arg_idx = 1 if nonewline else 0
            msg_arg_idx = len(args) - 1
            emitter._emit_value(args[chan_arg_idx])
            emitter._emit_value(args[msg_arg_idx])
            emitter._emit_i32_const(1 if nonewline else 0)
            emitter._emit_call(chan_idx)
            emitter._runtime_call_end(rimp, defs, context)
            return True

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
