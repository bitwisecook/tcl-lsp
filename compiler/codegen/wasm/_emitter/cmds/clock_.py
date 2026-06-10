"""Command helpers for ``clock`` — subcommand dispatch via the registry."""

from __future__ import annotations

from ..._imports import subcommand_runtime_import_for


def _emit_clock_value(emitter, args: tuple[str, ...]) -> None:
    """Emit a ``clock <subcmd>`` expression; leaves i32 TclObj on stack.

    ``seconds``/``clicks``/``milliseconds`` call the WASI-backed
    runtime helpers directly.  Anything else falls through to the
    interpreter (which will likely trap for ``format``/``scan`` in
    the sandbox — that's fine as a clear diagnostic until we ship a
    timezone-aware formatter).
    """
    if not args:
        emitter._emit_i32_const(0)
        return
    subcmd = args[0]
    sri = subcommand_runtime_import_for("clock", subcmd)
    if sri is not None:
        import_key = sri.import_key
        func_idx = emitter._shared_imports.get(import_key)
        if func_idx is not None:
            emitter._emit_call(func_idx)
            return
    # Fall back to the interpreter for unsupported subcommands.
    emitter._emit_eval_fallback("clock", args)


class _CmdClockMixin:
    """Expose the migrated helpers as methods for MRO composition."""

    _emit_clock_value = _emit_clock_value
