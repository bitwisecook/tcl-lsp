"""WASM emit hook for ``format`` — Tcl printf-style string formatting.

The Zig ``tcl_cmd_format`` export takes a fixed 4-slot signature
``(fmt, a1, a2, a3)``; callers pass zero-valued TclObjs for unused
slots.  ``format`` with more than 3 substitutions falls through to
the runtime's variadic dispatch via the eval fallback.
"""

from __future__ import annotations

from ......commands.registry import REGISTRY, EmitContext


def _emit_format(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    prep = emitter._runtime_prep("format", args)
    if prep is None:
        return False
    func_idx, spec = prep

    if not args:
        emitter._emit_i32_const(0)
        emitter._emit_i32_const(0)
        emitter._emit_i32_const(0)
        emitter._emit_i32_const(0)
    else:
        emitter._emit_value(args[0])
        for slot in range(1, 4):
            if slot < len(args):
                emitter._emit_value(args[slot])
            else:
                emitter._emit_i32_const(0)
    emitter._emit_call(func_idx)
    emitter._runtime_call_end(spec, defs, context)
    return True


REGISTRY.register_wasm_emitter("format", _emit_format)
