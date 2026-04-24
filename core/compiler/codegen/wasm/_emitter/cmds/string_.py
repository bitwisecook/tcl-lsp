"""WASM emit hook for ``string``."""

from __future__ import annotations

from ......commands.registry import REGISTRY, EmitContext
from ..._imports import (
    _RUNTIME_IMPORTS,
    _STRING_IS_IMPORT,
    _STRING_SUBCMD_IMPORT,
)
from ..._ir import WasmOp


def _emit_string(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    """``string subcommand ...`` — dispatch to runtime import (i32 args)."""
    if context is EmitContext.VALUE:
        # Tail / implicit-return: the result stays on the operand stack.
        # Runtime imports that return ``i32`` already leave one there;
        # void-result imports get a ``i32.const 0`` pushed to fill in
        # an empty-string result.  Unknown subcommands / missing imports
        # produce a null TclObj.
        if args:
            subcmd = args[0]
            if subcmd == "is" and len(args) >= 3:
                is_key = _STRING_IS_IMPORT.get(args[1])
                if is_key is not None and is_key in emitter._shared_imports:
                    func_idx = emitter._shared_imports[is_key]
                    emitter._emit_value(args[-1])
                    emitter._emit_call(func_idx)
                    return True
            import_key = _STRING_SUBCMD_IMPORT.get(subcmd)
            if import_key is not None and import_key in emitter._shared_imports:
                func_idx = emitter._shared_imports[import_key]
                spec = _RUNTIME_IMPORTS[import_key]
                param_count = len(spec[2])
                sub_args = args[1:]
                for i in range(min(param_count, len(sub_args))):
                    emitter._emit_value(sub_args[i])
                for _ in range(param_count - len(sub_args)):
                    emitter._emit_i32_const(0)
                emitter._emit_call(func_idx)
                if not spec[3]:
                    emitter._emit_i32_const(0)
                return True
        emitter._emit_i32_const(0)
        return True
    if not args:
        emitter._emit_unsupported_trap("string (no subcommand)")
        return True
    subcmd = args[0]

    if subcmd == "cat":
        sub_args = args[1:]
        if not sub_args:
            emitter._emit_obj_literal("")
        elif all(
            not a.startswith("$")
            and not a.startswith("[")
            and not emitter._has_embedded_subst(a)
            and a not in emitter._aliases
            and a not in emitter._local_index
            for a in sub_args
        ):
            emitter._emit_obj_literal("".join(sub_args))
        else:
            append_idx = emitter._shared_imports.get("tcl_append")
            if append_idx is None:
                emitter._emit_eval_fallback("string", args)
                if defs:
                    def_idx = emitter._intern_local(defs[0])
                    emitter._emit_local_set(def_idx)
                else:
                    emitter._emit(WasmOp.DROP)
                return True
            emitter._emit_value(sub_args[0])
            for rest in sub_args[1:]:
                emitter._emit_value(rest)
                emitter._emit_call(append_idx)
        if defs:
            def_idx = emitter._intern_local(defs[0])
            emitter._emit_local_set(def_idx)
        else:
            emitter._emit(WasmOp.DROP)
        return True

    if subcmd == "is" and len(args) >= 3:
        class_name = args[1]
        is_key = _STRING_IS_IMPORT.get(class_name)
        if is_key is not None and is_key in emitter._shared_imports:
            func_idx = emitter._shared_imports[is_key]
            spec = _RUNTIME_IMPORTS[is_key]
            val_arg = args[-1]
            emitter._emit_value(val_arg)
            emitter._emit_call(func_idx)
            if defs:
                def_idx = emitter._intern_local(defs[0])
                emitter._emit_local_set(def_idx)
            elif spec[3]:
                emitter._emit(WasmOp.DROP)
            return True

    import_key = _STRING_SUBCMD_IMPORT.get(subcmd)
    if import_key is not None and import_key in emitter._shared_imports:
        func_idx = emitter._shared_imports[import_key]
        spec = _RUNTIME_IMPORTS[import_key]
        param_count = len(spec[2])
        sub_args = args[1:]
        for i in range(min(param_count, len(sub_args))):
            emitter._emit_value(sub_args[i])
        for _ in range(param_count - len(sub_args)):
            emitter._emit_i32_const(0)
        emitter._emit_call(func_idx)
        if defs:
            def_idx = emitter._intern_local(defs[0])
            emitter._emit_local_set(def_idx)
        elif spec[3]:
            emitter._emit(WasmOp.DROP)
    else:
        emitter._emit_unsupported_trap(f"string {subcmd}")
    return True


REGISTRY.register_wasm_emitter("string", _emit_string)
