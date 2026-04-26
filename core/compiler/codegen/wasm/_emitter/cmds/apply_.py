"""WASM emit hook for ``apply``.

The 2-arg ``tcl_cmd_apply(lambda, args)`` runtime export expects its
``args`` argument to be a *Tcl list* of every positional argument
the user passed — it list-parses ``args`` to recover individual
``words[2..]`` for the lambda.

The generic ``_emit_cmd_runtime`` path passes the call-site arg
verbatim, which is fine for a single bare-word arg
(``apply $f hello``) but mis-fires for any whitespace-bearing arg:

  apply {d {return $d}} {a 1 c 2}

is emitted as ``tcl_cmd_apply(lambda, "a 1 c 2")``.  The runtime
list-parses ``"a 1 c 2"`` into four elements and binds only the
first to ``d``.  Hence the lambda saw ``d == "a"``.

Fix: build a 1-or-more-element Tcl list TclObj from the call-site
args (re-using ``_emit_args_list``, which already handles the
literal/dynamic split) and pass that.
"""

from __future__ import annotations

from ......commands.registry import REGISTRY, EmitContext
from ..._ir import WasmOp


def _emit_apply(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    """``apply lambda ?arg ...?`` — pack args into a Tcl list."""
    if not args:
        return False  # let the eval fallback raise the ``wrong # args`` error
    func_idx = emitter._shared_imports.get("tcl_apply")
    if func_idx is None:
        return False
    # First arg is the lambda; remaining are positional args.
    emitter._emit_value(args[0])
    emitter._emit_args_list(tuple(args[1:]))
    emitter._emit_call(func_idx)
    if context is EmitContext.VALUE:
        return True
    if defs:
        def_idx = emitter._intern_local(defs[0])
        emitter._emit_local_set(def_idx)
    else:
        emitter._emit(WasmOp.DROP)
    return True


REGISTRY.register_wasm_emitter("apply", _emit_apply)
