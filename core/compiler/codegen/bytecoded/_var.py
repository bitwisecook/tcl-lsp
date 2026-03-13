"""Bytecoded codegen for ``append`` and ``lappend``."""

from __future__ import annotations

from typing import TYPE_CHECKING

from ..opcodes import Op

if TYPE_CHECKING:
    from .._emitter import _Emitter


def codegen_append(emitter: _Emitter, args: tuple[str, ...]) -> bool:
    """Compile ``append`` to specialised opcodes."""
    if len(args) == 1:
        # ``append x`` with no value — just returns current value.
        # tclsh 9.0 compiles this to a simple variable load.
        if emitter._is_proc and not emitter._is_qualified(args[0]):
            emitter._load_var(args[0])
        else:
            return False
        return True

    if len(args) >= 2:
        # tclsh 9.0: multi-value append (3+ args) at top level
        # falls back to generic invokeStk1.
        if not emitter._is_proc and len(args) > 2:
            return False
        if emitter._is_proc:
            slot = emitter._lvt.intern(args[0])
            for a in args[1:]:
                emitter._emit_value(a)
                emitter._emit(Op.APPEND_SCALAR1, slot, comment=f'var "{args[0]}"')
        else:
            for a in args[1:]:
                emitter._push_lit(args[0])
                emitter._emit_value(a)
                emitter._emit(Op.APPEND_STK)
        emitter._emit(Op.POP)
        return True

    return False


def codegen_lappend(emitter: _Emitter, args: tuple[str, ...]) -> bool:
    """Compile ``lappend`` to specialised opcodes."""
    if len(args) < 2:
        return False

    # tclsh 9.0: multi-value lappend (3+ args) at top level
    # uses ``list N; lappendListStk`` (or lappendListArrayStk
    # for array variables).
    if not emitter._is_proc and len(args) > 2:
        arr = emitter._split_array_ref(args[0])
        if arr is not None:
            emitter._push_lit(arr[0])
            emitter._push_lit(arr[1])
        else:
            emitter._push_lit(args[0])
        for a in args[1:]:
            emitter._emit_value(a)
        emitter._emit(Op.LIST, len(args) - 1)
        if arr is not None:
            emitter._emit(Op.LAPPEND_LIST_ARRAY_STK)
        else:
            emitter._emit(Op.LAPPEND_LIST_STK)
        emitter._emit(Op.POP)
        return True
    if emitter._is_proc:
        arr = emitter._split_array_ref(args[0])
        if arr is not None and not emitter._is_qualified(args[0]):
            # Proc-context array lappend: push key, values, list, lappendListArray
            base = arr[0]
            slot = emitter._lvt.intern(base)
            emitter._push_array_key(arr[1])
            for a in args[1:]:
                emitter._emit_value(a)
            emitter._emit(Op.LIST, len(args) - 1)
            emitter._emit(Op.LAPPEND_LIST_ARRAY, slot, comment=f'var "{base}"')
        else:
            var_name = args[0]
            slot = emitter._lvt.intern(var_name)
            if len(args) > 2:
                # tclsh 9.0: multi-value lappend in proc context
                # uses ``list N; lappendList %vN``.
                for a in args[1:]:
                    emitter._emit_value(a)
                emitter._emit(Op.LIST, len(args) - 1)
                emitter._emit(Op.LAPPEND_LIST, slot, comment=f'var "{var_name}"')
            else:
                emitter._emit_value(args[1])
                emitter._emit(Op.LAPPEND_SCALAR1, slot, comment=f'var "{var_name}"')
    else:
        for a in args[1:]:
            emitter._push_lit(args[0])
            emitter._emit_value(a)
            emitter._emit(Op.LAPPEND_STK)
    emitter._emit(Op.POP)
    return True


def register() -> None:
    from core.commands.registry import REGISTRY

    REGISTRY.register_codegen("append", codegen_append)
    REGISTRY.register_codegen("lappend", codegen_lappend)
