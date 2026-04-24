"""WASM emit hook for ``dict``."""

from __future__ import annotations

from ......commands.registry import REGISTRY, EmitContext
from ..._imports import (
    _RUNTIME_IMPORTS,
    subcommand_runtime_import_for,
)
from ..._ir import WasmOp


def _emit_dict(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    """``dict subcommand ...`` — dispatch to runtime import (i32 args)."""
    if context is EmitContext.VALUE:
        # Tail / implicit-return: leave the result on the operand stack.
        # ``dict set`` has bespoke arity handling (first sub-arg is a
        # local var name, the call returns the updated dict which gets
        # tee'd back into that local).  Every other subcommand goes
        # through the generic import-dispatch; void-result imports get
        # a ``i32.const 0`` pushed in place of a missing return value.
        if args:
            subcmd = args[0]
            sri = subcommand_runtime_import_for("dict", subcmd)
            if sri is not None and sri.import_key in emitter._shared_imports:
                import_key = sri.import_key
                func_idx = emitter._shared_imports[import_key]
                spec = _RUNTIME_IMPORTS[import_key]
                param_count = len(spec[2])
                sub_args = args[1:]
                if subcmd == "set" and len(sub_args) >= 3:
                    var_idx = emitter._intern_local(sub_args[0])
                    emitter._emit_local_get(var_idx)
                    emitter._emit_value(sub_args[1])
                    emitter._emit_value(sub_args[2])
                    emitter._emit_call(func_idx)
                    emitter._emit_local_tee(var_idx)
                else:
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
        emitter._emit_unsupported_trap("dict (no subcommand)")
        return True
    subcmd = args[0]

    if subcmd == "merge":
        sub_args = args[1:]
        merge_idx = emitter._shared_imports.get("tcl_dict_merge_pair")
        if not sub_args:
            emitter._emit_obj_literal("")
        elif merge_idx is None:
            emitter._emit_value(sub_args[0])
        else:
            emitter._emit_value(sub_args[0])
            for rest in sub_args[1:]:
                emitter._emit_value(rest)
                emitter._emit_call(merge_idx)
        if defs:
            def_idx = emitter._intern_local(defs[0])
            emitter._emit_local_set(def_idx)
        else:
            emitter._emit(WasmOp.DROP)
        return True

    if subcmd == "create":
        kv = args[1:]
        if not kv:
            emitter._emit_obj_literal("")
            if defs:
                def_idx = emitter._intern_local(defs[0])
                emitter._emit_local_set(def_idx)
            else:
                emitter._emit(WasmOp.DROP)
            return True
        if all(
            not a.startswith("$")
            and not a.startswith("[")
            and not emitter._has_embedded_subst(a)
            and a not in emitter._aliases
            and a not in emitter._local_index
            for a in kv
        ):
            emitter._emit_obj_literal(" ".join(kv))
            if defs:
                def_idx = emitter._intern_local(defs[0])
                emitter._emit_local_set(def_idx)
            else:
                emitter._emit(WasmOp.DROP)
            return True
        lappend_idx = emitter._shared_imports.get("tcl_lappend")
        if lappend_idx is None:
            emitter._emit_eval_fallback("dict", args)
            if defs:
                def_idx = emitter._intern_local(defs[0])
                emitter._emit_local_set(def_idx)
            else:
                emitter._emit(WasmOp.DROP)
            return True
        emitter._emit_obj_literal("")
        for elem in kv:
            emitter._emit_value(elem)
            emitter._emit_call(lappend_idx)
        if defs:
            def_idx = emitter._intern_local(defs[0])
            emitter._emit_local_set(def_idx)
        else:
            emitter._emit(WasmOp.DROP)
        return True

    sri = subcommand_runtime_import_for("dict", subcmd)
    if sri is not None and sri.import_key in emitter._shared_imports:
        import_key = sri.import_key
        func_idx = emitter._shared_imports[import_key]
        spec = _RUNTIME_IMPORTS[import_key]
        param_count = len(spec[2])
        sub_args = args[1:]
        if subcmd == "set" and len(sub_args) >= 3:
            emitter._emit_var_read_obj(sub_args[0])
            emitter._emit_value(sub_args[1])
            emitter._emit_value(sub_args[2])
            emitter._emit_call(func_idx)
            if defs:
                emitter._emit_var_write_obj_keep(sub_args[0])
                def_idx = emitter._intern_local(defs[0])
                emitter._emit_local_set(def_idx)
            else:
                emitter._emit_var_write_obj(sub_args[0])
            return True
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
        emitter._emit_unsupported_trap(f"dict {subcmd}")
    return True


REGISTRY.register_wasm_emitter("dict", _emit_dict)
