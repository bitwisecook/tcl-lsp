"""WASM emit hook for ``return``."""

from __future__ import annotations

from ......commands.registry import REGISTRY, EmitContext
from ..._ir import WasmOp


def _emit_return(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    """``return ?-code code? ?value?`` — delegates to _emit_cmd_return."""
    if context is EmitContext.VALUE:
        # Tail position (rare — the CFG usually collapses ``return``
        # into IRReturn).  Leave the declared value on the stack, or
        # a null TclObj if no argument was given.
        if args:
            emitter._emit_value(args[0])
        else:
            emitter._emit_i32_const(0)
        return True
    emitter._emit_cmd_return(args)
    return True


REGISTRY.register_wasm_emitter("return", _emit_return)


def _emit_cmd_return(emitter, args: tuple[str, ...]) -> None:
    """``return ?value?`` or ``return -code code ?value?``.

    The simple single-value form compiles to a WASM return.
    ``return -code error <msg>`` is special-cased inline: evaluate
    *msg* via :meth:`_emit_value` (so embedded ``$var`` /
    ``[cmd]`` substitutions work), then call ``tcl_cmd_error``
    — which sets the catch's ``error_flag``/``error_msg`` when
    inside a ``catch`` or traps otherwise.  Going through
    :meth:`_emit_eval_fallback` would brace-wrap the message to
    preserve list structure, blocking the substitutions the
    error text needs — a real hazard because tcltest's error
    messages embed ``$option`` / ``$values`` everywhere.

    Other ``-code`` forms (``return -code break``, ``return -code
    continue``, numeric codes, ``-level N``, ``-errorinfo``
    ``-errorcode``) are rarer and fall through to the eval
    fallback, whose argument quoting is safe for them because
    their payloads are typically literal keywords or numeric
    values without interpolation.
    """
    if args and len(args) >= 3 and args[0] == "-code" and args[1] == "error" and len(args) == 3:
        # return -code error <msg>
        emitter._emit_value(args[2])
        # ``_RUNTIME_IMPORTS`` keys the error import as
        # ``tcl_error`` (internal key) → WASM name
        # ``tcl_cmd_error``; use the internal key to look up
        # the shared import slot.
        err_idx = emitter._shared_imports.get("tcl_error")
        if err_idx is None:
            emitter._emit_eval_fallback("return", args)
            return
        emitter._emit_call(err_idx)
        # tcl_cmd_error returns nothing; emit a null TclObj for
        # the WASM return value.  When inside a catch, error_flag
        # is now set and the catch body's has_error check will
        # trip on the next statement.  When outside a catch, the
        # runtime's tcl_cmd_error already traps and this return
        # is unreachable.
        emitter._emit_i32_const(0)
        emitter._emit(WasmOp.RETURN)
        return
    if args and args[0].startswith("-"):
        emitter._emit_eval_fallback("return", args)
        return
    if args:
        emitter._emit_value(args[0])
    else:
        emitter._emit_i32_const(0)
    emitter._emit(WasmOp.RETURN)


class _CmdReturnMixin:
    """Expose the migrated helpers as methods for MRO composition."""

    _emit_cmd_return = _emit_cmd_return
