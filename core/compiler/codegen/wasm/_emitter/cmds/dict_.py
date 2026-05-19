"""WASM emit hook for ``dict``."""

from __future__ import annotations

from ......commands.registry import REGISTRY, EmitContext
from ..._imports import (
    subcommand_runtime_import_for,
)
from ..._ir import WasmOp
from ..._ownership import Ownership


def _emit_dict(emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext) -> bool:
    """``dict subcommand ...`` — dispatch to runtime import (i32 args)."""
    if context is EmitContext.VALUE:
        # Tail / implicit-return: leave the result on the operand stack.
        # ``dict set`` has bespoke arity handling (first sub-arg is a
        # local var name, the call returns the updated dict which gets
        # tee'd back into that local).  Every other subcommand goes
        # through the generic import-dispatch; void-result imports get
        # a ``i32.const 0`` pushed in place of a missing return value.
        # Unknown sub-commands / missing imports fall through to the
        # eval-fallback path so the caller sees a real ``bad subcommand``
        # error rather than a silent null.
        if args:
            subcmd = args[0]
            sri = subcommand_runtime_import_for("dict", subcmd)
            if sri is not None and sri.import_key in emitter._shared_imports:
                func_idx = emitter._shared_imports[sri.import_key]
                param_count = len(sri.params)
                sub_args = args[1:]
                if subcmd == "get" and len(sub_args) > 2:
                    # ``dict get DICT KEY ?KEY...?`` — Tcl 9 supports
                    # chained-key descent into nested dicts.  The
                    # 2-param runtime import only handles a single
                    # key; route the multi-key form through eval so
                    # the runtime ``dict get`` handler walks the
                    # chain (error-18.10's ``dict get $opts -during
                    # -during -errorcode`` exercises this).
                    emitter._emit_eval_fallback("dict", args)
                    return True
                if subcmd == "set" and len(sub_args) >= 3:
                    # Top-level vars must use the var path to keep the
                    # global table mirror in sync (see lappend_.py).
                    if not emitter._is_proc:
                        emitter._emit_var_read_obj(sub_args[0])
                        emitter._emit_value(sub_args[1])
                        emitter._emit_value(sub_args[2])
                        emitter._emit_call(func_idx)
                        emitter._emit_var_write_obj_keep(
                            sub_args[0],
                            source=Ownership.OWNED,
                        )
                    else:
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
                    if not sri.results:
                        emitter._emit_i32_const(0)
                return True
        # Unknown subcommand (or no subcommand at all) — delegate to
        # the interpreter so it raises Tcl's native ``bad subcommand``
        # error; ``tcl_eval``'s i32 result satisfies the tail
        # dispatcher's value-stack expectation.
        emitter._emit_eval_fallback("dict", args)
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
        # Odd arity is a ``wrong # args`` error in Tcl 9 — fall through
        # to the eval-fallback path so the runtime dispatcher in
        # ``cmds/dict.zig::eval_dict_create`` raises the canonical
        # message rather than the compile-time fast path silently
        # dropping the trailing key (Copilot review on PR #443).
        if len(kv) % 2 == 0 and all(
            not a.startswith("$")
            and not a.startswith("[")
            and not emitter._has_embedded_subst(a)
            and a not in emitter._aliases
            and a not in emitter._local_index
            for a in kv
        ):
            # Canonicalise duplicate keys at compile time so the literal
            # string rep matches Tcl 9's ``Tcl_DictObjPut`` semantics —
            # later occurrences of the same key REPLACE the value, while
            # the key's insertion position is preserved (tclDictObj.c).
            # Without this, ``string map [dict create a X b Y a Z] aaa``
            # walks the literal ``a X b Y a Z`` list and uses the first
            # ``a→X`` mapping; Tcl 9 expects last-wins (``a→Z``) because
            # the canonical dict has only one ``a`` entry.
            canon: list[str] = []
            key_pos: dict[str, int] = {}
            for wi in range(0, len(kv), 2):
                k, v = kv[wi], kv[wi + 1]
                if k in key_pos:
                    canon[key_pos[k] + 1] = v
                else:
                    key_pos[k] = len(canon)
                    canon.append(k)
                    canon.append(v)
            emitter._emit_obj_literal(" ".join(canon))
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
        func_idx = emitter._shared_imports[sri.import_key]
        param_count = len(sri.params)
        sub_args = args[1:]
        if subcmd == "set" and len(sub_args) >= 3:
            emitter._emit_var_read_obj(sub_args[0])
            emitter._emit_value(sub_args[1])
            emitter._emit_value(sub_args[2])
            emitter._emit_call(func_idx)
            if defs:
                emitter._emit_var_write_obj_keep(
                    sub_args[0],
                    source=Ownership.OWNED,
                )
                def_idx = emitter._intern_local(defs[0])
                emitter._emit_local_set(def_idx)
            else:
                emitter._emit_var_write_obj(
                    sub_args[0],
                    source=Ownership.OWNED,
                )
            return True
        for i in range(min(param_count, len(sub_args))):
            emitter._emit_value(sub_args[i])
        for _ in range(param_count - len(sub_args)):
            emitter._emit_i32_const(0)
        emitter._emit_call(func_idx)
        if defs:
            def_idx = emitter._intern_local(defs[0])
            emitter._emit_local_set(def_idx)
        elif sri.results:
            emitter._emit(WasmOp.DROP)
    else:
        # Unknown / not-yet-imported subcommand — delegate to the
        # interpreter.  ``cmds/dict.zig::eval`` knows how to evaluate
        # ``append``, ``lappend``, ``incr``, ``info``, ``for``, ``map``,
        # ``merge``, ``remove``, ``replace``, ``with``, ``filter``;
        # genuinely unknown subcommands raise ``bad subcommand`` from
        # the dispatcher, matching the reference Tcl behaviour and
        # remaining catchable.  This avoids the hard ``UNREACHABLE``
        # trap that previously aborted the whole bundle even when the
        # call sat inside a ``catch`` at runtime (the compile-time
        # ``_catch_depth`` cannot see runtime catch frames).
        emitter._emit_eval_fallback("dict", args)
        if defs:
            def_idx = emitter._intern_local(defs[0])
            emitter._emit_local_set(def_idx)
        else:
            emitter._emit(WasmOp.DROP)
    return True


REGISTRY.register_wasm_emitter("dict", _emit_dict)
