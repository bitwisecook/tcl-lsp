"""WASM emit hooks for ``list``, ``lset``, and ``lassign``."""

from __future__ import annotations

from ......commands.registry import REGISTRY, EmitContext
from ..._encoding import _tcl_list_quote, _tcl_token_value
from ..._ir import ValType, WasmOp


def _emit_list(emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext) -> bool:
    """``list ?arg ...?`` — variadic list builder; delegates to _emit_list_value."""
    if context is EmitContext.VALUE:
        # Tail-context not yet migrated — handled inline in _statements.py.
        return False
    emitter._emit_list_value(args)
    if defs:
        def_idx = emitter._intern_local(defs[0])
        emitter._emit_local_set(def_idx)
    else:
        emitter._emit(WasmOp.DROP)
    return True


def _emit_lset(emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext) -> bool:
    """``lset varName ?index ...? newValue`` — replace a list element."""
    if context is EmitContext.VALUE:
        # Tail-context not yet migrated — handled inline in _statements.py.
        return False
    if len(args) < 2:
        emitter._emit_unsupported_trap("lset (too few args)")
        return True

    var_name = args[0]
    new_value = args[-1]
    index_args = args[1:-1]

    list_set_idx = emitter._shared_imports.get("tcl_list_set")
    if list_set_idx is None:
        emitter._emit_unsupported_trap("lset (missing tcl_list_set)")
        return True

    emitter._emit_var_read_obj(var_name)

    if not index_args:
        emitter._emit_obj_literal("")
    elif len(index_args) == 1:
        emitter._emit_value(index_args[0])
    else:
        tcl_list_idx = emitter._shared_imports.get("tcl_list_create")
        if tcl_list_idx is None:
            raise RuntimeError(
                "internal error: lset multi-index requires "
                "tcl_list_create import; scan phase should have "
                "registered it.  Check _scan.py's IRCall path."
            )
        emitter._emit_value(index_args[0])
        for arg in index_args[1:]:
            emitter._emit_value(arg)
            emitter._emit_call(tcl_list_idx)

    emitter._emit_value(new_value)
    emitter._emit_call(list_set_idx)

    if defs:
        emitter._emit_var_write_obj_keep(var_name)
        def_idx = emitter._intern_local(defs[0])
        emitter._emit_local_set(def_idx)
    else:
        emitter._emit_var_write_obj(var_name)
    if emitter._optimise:
        emitter._const_map.pop(var_name, None)
    return True


def _emit_lassign(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    """``lassign list ?varName ...?`` — destructure a list; delegates to _emit_cmd_lassign."""
    if not args:
        return False
    keep_on_stack = context is EmitContext.VALUE
    emitter._emit_cmd_lassign(args, defs, keep_on_stack=keep_on_stack)
    return True


REGISTRY.register_wasm_emitter("list", _emit_list)
REGISTRY.register_wasm_emitter("lset", _emit_lset)
REGISTRY.register_wasm_emitter("lassign", _emit_lassign)


def _emit_list_value(emitter, args: tuple[str, ...]) -> None:
    """Build a Tcl list from *args* and leave it on the stack as a TclObj.

    Each arg is emitted with proper Tcl list-element encoding —
    empty strings become ``{}``, words containing whitespace /
    braces / brackets / ``"`` / ``\\`` / ``$`` / ``;`` get wrapped
    in braces so re-parsers see one element per arg.  This is
    critical: ``[list "a b" c]`` must produce a two-element list
    ``{a b} c``, not the three-word string ``a b c``.

    When every arg is a literal the result is folded at compile
    time into a single ``obj_new_string``; otherwise we emit a
    ``tcl_concat`` chain with single-space separators.  Variable
    and command-substitution args skip the element-quoting step
    — their runtime value is used as-is, matching the pre-
    existing ``tcl_list`` behaviour for interpolated values
    (quoting them at compile time would brace-wrap whatever
    runtime value they hold, changing semantics).
    """
    if not args:
        emitter._emit_i32_const(0)
        return
    # Fast path: all literals → compile-time string.
    # _tcl_token_value expands each source token to its VALUE (strips
    # braces, applies backslash subst), then _tcl_list_quote encodes
    # the value as a list element.
    if all(not a.startswith("$") and not a.startswith("[") for a in args):
        emitter._emit_obj_literal(
            " ".join(
                _tcl_list_quote(_tcl_token_value(a), first=(i == 0)) for i, a in enumerate(args)
            )
        )
        return
    # Mixed: start with an empty list, lappend each arg so that
    # runtime values containing spaces are properly quoted as
    # single list elements (tcl_cmd_lappend wraps in {} if needed).
    lappend_idx = emitter._shared_imports.get("tcl_lappend")
    if lappend_idx is None:
        # No lappend helper — fall back to first arg only.
        emitter._emit_value(args[0])
        return

    def _emit_elem(a: str) -> None:
        # Braced literal — strip outer braces and emit the raw content;
        # tcl_cmd_lappend will re-quote it correctly if needed.
        if a.startswith("{") and a.endswith("}"):
            emitter._emit_obj_literal(a[1:-1])
            return
        if a.startswith("$") or a.startswith("["):
            emitter._emit_value(a)
        else:
            emitter._emit_obj_literal(a)

    emitter._emit_obj_literal("")  # empty list seed
    for a in args:
        _emit_elem(a)
        emitter._emit_call(lappend_idx)


def _emit_cmd_lassign(
    emitter,
    args: tuple[str, ...],
    defs: tuple[str, ...],
    *,
    keep_on_stack: bool,
) -> None:
    """``lassign list ?varName ...?`` — destructure a list into vars.

    For each ``varName`` at position *i*, assigns ``list_index(list, i)``
    to that variable (empty string if out of range).  Returns the
    leftover list (elements beyond the supplied variables) — computed
    at runtime via ``tcl_list_tail``.

    If *keep_on_stack* is True (value/expression context), the
    leftover-list i32 TclObj is left on the stack; otherwise the
    result is stored in ``defs[0]`` if given, or dropped.
    """
    if not args:
        emitter._emit_unsupported_trap("lassign (no list arg)")
        return
    list_arg = args[0]
    var_names = args[1:]

    # Stash the list value once so we can index into it repeatedly
    # without re-evaluating its expression (which may have side effects
    # via command substitution).
    list_local = emitter._add_extra_local(prefix="_lassign_list", val_type=ValType.I32)
    emitter._emit_value(list_arg)
    emitter._emit_local_set(list_local)

    # Per-variable: write list_index(list, i) into the named variable.
    lindex_idx = emitter._shared_imports.get("tcl_list_index")
    if lindex_idx is None:
        # Can't emit without the runtime helper — fall back to trap.
        emitter._emit_unsupported_trap("lassign (missing tcl_list_index)")
        return

    for i, var_name in enumerate(var_names):
        emitter._emit_local_get(list_local)
        # Index argument is a TclObj holding the integer i.
        emitter._emit_obj_literal(str(i))
        emitter._emit_call(lindex_idx)
        emitter._emit_var_write_obj(var_name)

    # Produce the leftover list (elements from index len(var_names) on).
    ltail_idx = emitter._shared_imports.get("tcl_list_tail")
    if ltail_idx is None:
        # No tail helper — emit empty list.
        emitter._emit_i32_const(0)
    else:
        emitter._emit_local_get(list_local)
        emitter._emit_obj_literal(str(len(var_names)))
        emitter._emit_call(ltail_idx)

    if keep_on_stack:
        return
    # Statement context: the leftover list is discarded.  ``defs`` for
    # lassign lists the VAR_WRITE targets (var_names), NOT a place to
    # store the command's return value — ignore it here to avoid
    # overwriting one of the just-assigned locals.
    emitter._emit(WasmOp.DROP)


class _CmdListMixin:
    """Expose the migrated helpers as methods for MRO composition."""

    _emit_list_value = _emit_list_value
    _emit_cmd_lassign = _emit_cmd_lassign
