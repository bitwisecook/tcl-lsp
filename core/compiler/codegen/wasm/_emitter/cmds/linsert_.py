"""WASM emit hook for ``linsert`` — multi-value list insert.

``linsert list index v1 ?v2 ...?`` with multiple values — the Zig
``tcl_cmd_list_insert`` export is single-value.  This hook decides
between two paths:

* ``end`` / ``end-N`` indices use the chained single-value fast
  path.  Each subsequent insert re-resolves ``end-N`` against the
  now-longer list, so iterating values in forward order produces
  the correct ``end-N+(i-1)`` layout.
* Every other index shape — bare numeric literal, ``$var``, or a
  computed expression — routes through the eval fallback
  (``tcl_eval`` → ``eval_linsert``).  The runtime helper resolves
  the index ONCE against the original list and then inserts every
  value in forward order with the position incrementing, so
  ``linsert {} 2 a b c`` lands ``a b c`` instead of ``c b a``
  (linsert-1.10).
* The no-value form ``linsert list index`` always routes through
  the eval fallback so the runtime canonicalises the input list
  (TIP 323 — ``linsert "a\\nb\\nc" 0`` → ``a b c``).  The
  single-value runtime fast path would otherwise insert an empty
  element and produce ``{} a b c`` (linsert-2.5 / 2.6).

A numeric-index fast path is intentionally NOT implemented even
for "looks small" literal indices: we can't tell at compile time
whether the literal exceeds the runtime list length, and the
reverse-order chained-insert trick only works when the index is
strictly within bounds.  The eval-fallback cost is acceptable
because multi-value linsert is uncommon on the WASM hot path.
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
