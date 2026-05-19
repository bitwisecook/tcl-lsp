"""WASM emit hook for ``lappend`` — variadic mutator onto a list variable.

``lappend var v1 ?v2 ...?`` reads the current value of ``var``, calls
``tcl_cmd_lappend(cur, vN)`` once per value, and writes the running
list back between iterations.  In VALUE context the final write
keeps the updated value on the stack for implicit return.
"""

from __future__ import annotations

from core.commands.registry import REGISTRY, EmitContext

from ..._ownership import Ownership
from ..._parsing import _parse_array_ref
from .._variables import _is_dynamic_var_name


def _emit_lappend(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    if len(args) < 2:
        return False
    prep = emitter._runtime_prep("lappend", args)
    if prep is None:
        return False
    func_idx, _rimp = prep

    var_name = args[0]
    array_ref = _parse_array_ref(var_name)
    # ``arr(key)`` + alias + aliased-array-base must go through the var
    # subsystem so the write lands in the array hash table; otherwise
    # ``_intern_local`` creates a scalar slot named ``arr(key)``.
    #
    # Top-level vars must also use the var path: ``_emit_var_read_obj``
    # at top level routes through ``tcl_global_get`` (the WASM-local
    # mirror is bypassed so eval-fallback writes stay visible — see
    # Phase 4.5 finalisation comment in _variables.py).  If we wrote
    # ``lappend`` results via plain ``local_set`` here, the global
    # table would still hold the pre-lappend value and ``puts $l``
    # would print the stale value.  ``_emit_var_write_obj`` does the
    # global mirror itself, so routing through it keeps writes and
    # reads consistent.
    # ``global``-declared names must also use the var path so the
    # global table stays the single source of truth — a callee proc
    # that also declares ``global`` and mutates the same name would
    # leave this proc's WASM-local mirror stale.  Mirrors the
    # equivalent fix in append_.py / set_.py.
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
    last_index = len(args) - 1

    if use_var_path:
        for i, value_arg in enumerate(args[1:], start=1):
            # ``lappend`` auto-creates an unset variable with the
            # appended values (``lappend missing a b`` ⇒ ``a b``), so
            # the read must be lenient: a missing alias / namespace
            # global must return null TclObj rather than raise
            # ``can't read "<var>": no such variable``.  ``variable
            # OptionControlledVariables; lappend
            # OptionControlledVariables x`` inside ``::tcltest::Option``
            # is the canonical case — the ``variable`` declaration
            # registers the alias but does not initialise the slot,
            # so the first ``lappend`` must initialise it itself.
            emitter._emit_var_read_obj_lenient(var_name)
            emitter._emit_value(value_arg)
            emitter._emit_call(func_idx)
            # ``tcl_cmd_lappend`` returns either a freshly allocated
            # canonical-rebuild result (slow path, rc=1) or the input
            # handle mutated in place (fast path, rc unchanged).  Both
            # arrive on the stack at "+1 if alloc, +0 if borrow" — but
            # ``tcl_global_set`` already retains on store, so treating
            # the return as OWNED at the top-level write balances
            # correctly: the post-gset release in
            # ``_emit_var_write_obj_impl`` drops the +1 the caller
            # owed.  Without OWNED, every iteration leaks one rc on
            # the list value and the rc==1 fast path can never engage.
            if keep_last and i == last_index:
                emitter._emit_var_write_obj_keep(var_name, source=Ownership.OWNED)
            else:
                emitter._emit_var_write_obj(var_name, source=Ownership.OWNED)
    else:
        var_idx = emitter._intern_local(var_name)
        for i, value_arg in enumerate(args[1:], start=1):
            emitter._emit_local_get(var_idx)
            emitter._emit_value(value_arg)
            emitter._emit_call(func_idx)
            if keep_last and i == last_index:
                emitter._emit_local_tee(var_idx)
            else:
                emitter._emit_local_set(var_idx)

    return True


REGISTRY.register_wasm_emitter("lappend", _emit_lappend)
