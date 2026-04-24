"""WASM emit hook for ``lreplace`` — multi-value list replace.

``lreplace list first last v1 ?v2 ...?`` with multiple values —
emit one ``lreplace`` for the first value (consumes the range
``[first..last]`` and drops it in that slot), then chain inserts
for the remaining values.  Same index-type-dependent ordering as
``linsert`` (see ``cmds/linsert_.py``): numeric ``first`` uses
reverse value order after the base replace; ``end``-family uses
forward.
"""

from __future__ import annotations

from ......commands.registry import REGISTRY, EmitContext
from .._ops import _is_end_relative_index


def _emit_lreplace(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    # The Zig ``tcl_cmd_lreplace`` export takes 4 slots
    # (list, first, last, value); we handle the multi-value case
    # here by chaining ``tcl_list_insert`` after the base call.
    # Single-value and zero-value forms fall through to the
    # generic runtime dispatch.
    if len(args) <= 4:
        return False
    prep = emitter._runtime_prep("lreplace", args)
    if prep is None:
        return False
    func_idx, rimp = prep

    list_arg = args[0]
    first_arg = args[1]
    last_arg = args[2]
    values = args[3:]
    list_insert_idx = emitter._shared_imports.get("tcl_list_insert")
    if list_insert_idx is None or not values:
        emitter._emit_value(list_arg)
        emitter._emit_value(first_arg)
        emitter._emit_value(last_arg)
        emitter._emit_value(values[0] if values else "")
        emitter._emit_call(func_idx)
    elif _is_end_relative_index(first_arg):
        # ``end-N`` — replace with the *first* value, then
        # forward-insert the rest at ``first``.  ``end-N`` grows
        # with the list so each insert lands immediately after
        # the previous.
        emitter._emit_value(list_arg)
        emitter._emit_value(first_arg)
        emitter._emit_value(last_arg)
        emitter._emit_value(values[0])
        emitter._emit_call(func_idx)
        for v in values[1:]:
            emitter._emit_value(first_arg)
            emitter._emit_value(v)
            emitter._emit_call(list_insert_idx)
    else:
        # Numeric index — replace with the *last* value, then
        # insert the earlier values in reverse at ``first``.
        emitter._emit_value(list_arg)
        emitter._emit_value(first_arg)
        emitter._emit_value(last_arg)
        emitter._emit_value(values[-1])
        emitter._emit_call(func_idx)
        for v in reversed(values[:-1]):
            emitter._emit_value(first_arg)
            emitter._emit_value(v)
            emitter._emit_call(list_insert_idx)
    emitter._runtime_call_end(rimp, defs, context)
    return True


REGISTRY.register_wasm_emitter("lreplace", _emit_lreplace)
