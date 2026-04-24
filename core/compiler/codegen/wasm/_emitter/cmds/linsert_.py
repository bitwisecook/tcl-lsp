"""WASM emit hook for ``linsert`` — multi-value list insert.

``linsert list index v1 ?v2 ...?`` with multiple values — the Zig
``tcl_cmd_list_insert`` export is single-value, so we chain per-value
inserts at the same index.  Iteration order depends on how the index
resolves against the growing list:

* Numeric indices stay pinned to the same position through each
  insert, so inserting in *reverse* value order produces the
  correct forward layout ``{before} v1 v2 … vN {after}``.
* ``end`` / ``end-N`` indices re-resolve after every insert (``end``
  moves as the list grows).  Here *forward* value order is correct
  because each iteration's index lands at ``end-N + (i-1)`` — right
  after the previously inserted value.

The textual shape of the index tells us which strategy to use at
compile time; a ``$var`` index whose runtime value is an ``end-N``
string would be mis-ordered, but that is an uncommon shape we
accept for now.
"""

from __future__ import annotations

from ......commands.registry import REGISTRY, EmitContext
from .._ops import _is_end_relative_index


def _emit_linsert(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    # Single-value form falls through to the generic runtime dispatch.
    if len(args) <= 3:
        return False
    prep = emitter._runtime_prep("linsert", args)
    if prep is None:
        return False
    func_idx, spec = prep

    list_arg = args[0]
    index_arg = args[1]
    values = args[2:]
    emitter._emit_value(list_arg)
    if _is_end_relative_index(index_arg):
        iter_values = values
    else:
        iter_values = tuple(reversed(values))
    for v in iter_values:
        emitter._emit_value(index_arg)
        emitter._emit_value(v)
        emitter._emit_call(func_idx)
    emitter._runtime_call_end(spec, defs, context)
    return True


REGISTRY.register_wasm_emitter("linsert", _emit_linsert)
