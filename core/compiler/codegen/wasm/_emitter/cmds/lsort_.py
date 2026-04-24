"""WASM emit hook for ``lsort`` — trailing-positional list sort.

``lsort ?-switches ...? list`` — the Zig ``tcl_cmd_lsort`` export is
the no-switch form, so when extra args are present pick the trailing
positional as the list rather than the leading ``-option`` token;
otherwise ``lsort -integer {3 1 2}`` would dispatch with the list
``-integer`` and produce the single-element result ``-integer``.
Any leading ``-switches`` are currently ignored — a fully-switched
form falls back to the eval path.
"""

from __future__ import annotations

from ......commands.registry import REGISTRY, EmitContext


def _emit_lsort(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    prep = emitter._runtime_prep("lsort", args)
    if prep is None:
        return False
    func_idx, spec = prep
    param_count = len(spec[2])

    if len(args) > param_count:
        emitter._emit_value(args[-1])
        for _ in range(param_count - 1):
            emitter._emit_i32_const(0)
    else:
        for i in range(min(param_count, len(args))):
            emitter._emit_value(args[i])
        for _ in range(param_count - len(args)):
            emitter._emit_i32_const(0)

    emitter._emit_call(func_idx)
    emitter._runtime_call_end(spec, defs, context)
    return True


REGISTRY.register_wasm_emitter("lsort", _emit_lsort)
