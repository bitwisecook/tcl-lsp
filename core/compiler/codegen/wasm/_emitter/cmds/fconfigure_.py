"""WASM emit hook for ``fconfigure`` — channel option get/set.

The Zig ``tcl_cmd_fconfigure`` export takes a 2-arg signature
``(fd, opts_obj)``; this hook packs a variadic ``-option value``
sequence into one opts TclObj.  Fast path: all-literal options are
pre-joined at compile time; mixed literal + ``$var`` / ``[cmd]``
references fall back to a chained ``tcl_concat`` build.
"""

from __future__ import annotations

from ......commands.registry import REGISTRY, EmitContext


def _emit_fconfigure(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    prep = emitter._runtime_prep("fconfigure", args)
    if prep is None:
        return False
    func_idx, rimp = prep

    if not args:
        emitter._emit_i32_const(0)
        emitter._emit_i32_const(0)
    else:
        emitter._emit_value(args[0])
        rest = args[1:]
        if not rest:
            emitter._emit_i32_const(0)
        elif all(not a.startswith("$") and not a.startswith("[") for a in rest):
            emitter._emit_obj_literal(" ".join(rest))
        else:
            concat_idx = emitter._shared_imports.get("tcl_concat")
            if concat_idx is None:
                emitter._emit_obj_literal(" ".join(rest))
            else:
                emitter._emit_obj_literal(rest[0])
                for word in rest[1:]:
                    emitter._emit_obj_literal(" ")
                    emitter._emit_call(concat_idx)
                    emitter._emit_value(word)
                    emitter._emit_call(concat_idx)
    emitter._emit_call(func_idx)
    emitter._runtime_call_end(rimp, defs, context)
    return True


REGISTRY.register_wasm_emitter("fconfigure", _emit_fconfigure)
