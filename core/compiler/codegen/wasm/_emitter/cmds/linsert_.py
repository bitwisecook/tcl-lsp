"""WASM emit hook for ``linsert`` — multi-value list insert.

``linsert list index v1 ?v2 ...?`` with multiple values — the Zig
``tcl_cmd_list_insert`` export is single-value, so we chain per-value
inserts at the same index when the chain is provably safe.  Iteration
order depends on how the index resolves against the growing list:

* ``end`` / ``end-N`` indices re-resolve after every insert (``end``
  moves as the list grows).  Here *forward* value order is correct
  because each iteration's index lands at ``end-N + (i-1)`` — right
  after the previously inserted value.
* Numeric indices STRICTLY within the original list's bounds (i.e.
  ``index < len``) stay pinned, so inserting in *reverse* value
  order produces the correct forward layout.  But because we don't
  know ``len`` at compile time, only literals small enough to be
  safe for any non-empty list (``0`` for prepend, indices the
  programmer wrote knowing the data shape) get this fast path.
* Anything else — numeric out-of-range, ``$var`` indices, computed
  expressions — routes through the runtime dispatcher (``tcl_eval``
  → ``eval_linsert``) which resolves the index ONCE against the
  original list and inserts all values in forward order with the
  position incrementing.  Slower but correct (linsert-1.10:
  ``linsert {} 2 a b c`` must produce ``a b c`` not ``c b a``).
"""

from __future__ import annotations

from ......commands.registry import REGISTRY, EmitContext
from ..._ir import WasmOp
from .._ops import _is_end_relative_index


def _emit_linsert(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    # Single-value forms route through this hook so we can decide between
    # the chained-insert fast path and the eval fallback.
    # ``linsert list index`` (no values) and numeric-indexed multi-value
    # forms both need the runtime's whole-list canonicalisation logic.
    if len(args) == 2:
        # ``linsert list index`` — TIP 323 no-value form.  Always
        # canonicalises the input list (``"a\nb\nc"`` → ``a b c``).
        # The single-value runtime fast path would insert an empty
        # element instead, producing ``{} a b c``.
        emitter._emit_eval_fallback("linsert", args)
        if context is EmitContext.STATEMENT:
            emitter._emit(WasmOp.DROP)
        return True
    if len(args) <= 3:
        return False
    index_arg = args[1]
    # End-relative indices stay accurate through chained single-value
    # inserts because each subsequent ``end-N`` re-resolves against
    # the now-longer list.  Numeric indices are NOT safe — see the
    # module docstring — so they fall through to the eval-fallback
    # path in ``_emit_cmd_value`` / statement context.
    if not _is_end_relative_index(index_arg):
        return False
    prep = emitter._runtime_prep("linsert", args)
    if prep is None:
        return False
    func_idx, rimp = prep

    list_arg = args[0]
    values = args[2:]
    emitter._emit_value(list_arg)
    for v in values:
        emitter._emit_value(index_arg)
        emitter._emit_value(v)
        emitter._emit_call(func_idx)
    emitter._runtime_call_end(rimp, defs, context)
    return True


REGISTRY.register_wasm_emitter("linsert", _emit_linsert)
