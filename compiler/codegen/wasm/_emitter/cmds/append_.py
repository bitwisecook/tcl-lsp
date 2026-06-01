"""WASM emit hook for ``append`` — variadic mutator onto a string variable.

``append var v1 ?v2 ...?`` reads the current value of ``var``, calls
``tcl_cmd_append(cur, vN)`` once per value, and writes the running
result back between iterations.  In VALUE context the final write
keeps the updated value on the stack for implicit return.
"""

from __future__ import annotations

from collections.abc import Callable

from compiler.registry import REGISTRY, EmitContext

from ..._ownership import Ownership
from ..._parsing import _parse_array_ref
from .._variables import _is_dynamic_var_name


def _emit_append(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    if len(args) < 2:
        return False
    prep = emitter._runtime_prep("append", args)
    if prep is None:
        return False
    func_idx, _rimp = prep

    var_name = args[0]
    array_ref = _parse_array_ref(var_name)
    # Route array-element writes (``arr(key)``) and alias writes through
    # the variable subsystem — ``_intern_local`` would otherwise make a
    # scalar slot named ``arr(key)`` and miss the array hash table
    # entirely.  ``var in _aliases`` covers direct aliases
    # (``upvar 0 target var``); the array-ref branch covers the
    # unaliased array-element case and the aliased-base case
    # (``upvar 0 srcArr a; append a(key) …``).
    #
    # Top-level vars also use the var path: ``_emit_var_read_obj`` at
    # top level routes through ``tcl_global_get`` (the WASM-local
    # mirror is bypassed so eval-fallback writes stay visible — see
    # _variables.py Phase 4.5 finalisation).  A bare ``local_set`` at
    # top level would update the WASM local but leave the global
    # stale, so subsequent ``$var`` reads see the pre-append value.
    #
    # ``global``-declared names inside a proc also need the var path
    # so the append's writeback reaches the global table.  Without
    # this branch, ``proc f {} { global a ; append a x }`` only
    # updated the WASM-local mirror; the global stayed at its
    # pre-append value (the bug ``hello_world`` exposed).
    at_top_level = not emitter._is_proc
    is_global = var_name in emitter._globals
    use_var_path = (
        at_top_level
        or is_global
        or var_name in emitter._aliases
        or array_ref is not None
        or (array_ref is None and "(" in var_name and var_name.split("(")[0] in emitter._aliases)
        or _is_dynamic_var_name(var_name)
    )
    keep_last = context is EmitContext.VALUE
    values = args[1:]
    # ``append`` auto-creates an unset variable (``append missing x`` ⇒
    # ``x``; ``append arr(new) x`` creates the element), so the read must
    # be lenient — a missing scalar / array element / alias must read as
    # the null TclObj (empty), not raise ``can't read …``.
    # ``tcl_cmd_append`` treats null as the empty starting value.
    #
    # A single value keeps the in-place read-modify-write (the canonical
    # ``append acc $piece`` loop idiom — O(1) amortised in the runtime's
    # cap-doubling fast path).  Two or more values route through a
    # fresh-seeded concat chain: appending the running result back into
    # the variable between values would let the in-place fast path mutate
    # the variable's object mid-expression, so a value arg that aliases
    # the target (``append x $x $x``) re-reads the half-built accumulator
    # and the result doubles.  ``_emit_concat_chain`` seeds with a null
    # accumulator so the operands (including the variable's own object)
    # are never mutated, then we write the fresh result back once.

    ops: list[Callable[[], object]]
    if use_var_path:
        ops = [lambda: emitter._emit_var_read_obj_lenient(var_name)]
    else:
        var_idx = emitter._intern_local(var_name)
        ops = [lambda vi=var_idx: emitter._emit_local_get(vi)]
    ops += [lambda a=v: emitter._emit_value(a) for v in values]
    if len(values) == 1:
        # Single value — in-place read-modify-write (no re-read hazard).
        ops[0]()
        ops[1]()
        emitter._emit_call(func_idx)
    else:
        emitter._emit_concat_chain(func_idx, ops)
    # ``tcl_cmd_append`` / the concat chain leave an OWNED handle on the
    # stack (a fresh result, or the input mutated in place on the
    # single-value fast path).  Declaring the write source OWNED balances
    # the store's retain against the caller's +1 — without it the fresh
    # element on an empty array is dropped before the store lands
    # (var-24.7 append).
    if use_var_path:
        if keep_last:
            emitter._emit_var_write_obj_keep(var_name, source=Ownership.OWNED)
        else:
            emitter._emit_var_write_obj(var_name, source=Ownership.OWNED)
    else:
        var_idx = emitter._intern_local(var_name)
        if keep_last:
            emitter._emit_local_tee(var_idx)
        else:
            emitter._emit_local_set(var_idx)

    return True


REGISTRY.register_wasm_emitter("append", _emit_append)
