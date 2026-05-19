"""Bytecoded codegen for list commands."""

from __future__ import annotations

from typing import TYPE_CHECKING

from .._helpers import _tcl_list_element
from ..format import _esc
from ..opcodes import (
    _INDEX_END,
    Op,
    _parse_tcl_index,
)

if TYPE_CHECKING:
    from .._emitter import _Emitter


def codegen_llength(emitter: _Emitter, args: tuple[str, ...]) -> bool:
    """Compile ``llength`` to specialised opcodes."""
    if len(args) != 1:
        return False
    emitter._emit_value(args[0])
    emitter._emit(Op.LIST_LENGTH)
    emitter._emit(Op.POP)
    return True


def codegen_lassign(emitter: _Emitter, args: tuple[str, ...]) -> bool:
    """Compile ``lassign`` to specialised opcodes."""
    if len(args) < 2:
        return False
    # lassign list var1 var2 ...
    # Pattern: load list, then for each var: push varname,
    # over 1, listIndexImm idx, storeStk, pop.
    # Finally: listRangeImm <num_vars> end
    emitter._emit_value(args[0])  # load the list
    var_names = args[1:]
    for i, var in enumerate(var_names):
        emitter._push_lit(var)
        emitter._emit(Op.OVER, 1)
        emitter._emit(Op.LIST_INDEX_IMM, i)
        emitter._emit(Op.STORE_STK)
        emitter._emit(Op.POP)
    emitter._emit(Op.LIST_RANGE_IMM, len(var_names), _INDEX_END)
    emitter._emit(Op.POP)
    return True


def codegen_lindex(emitter: _Emitter, args: tuple[str, ...]) -> bool:
    """Compile ``lindex`` to specialised opcodes."""
    if len(args) != 2:
        return False
    emitter._emit_value(args[0])
    # Use listIndexImm when the index is an integer constant
    try:
        idx_val = int(args[1])
        emitter._emit(Op.LIST_INDEX_IMM, idx_val)
    except (ValueError, TypeError):
        emitter._emit_value(args[1])
        emitter._emit(Op.LIST_INDEX)
    emitter._emit(Op.POP)
    return True


def codegen_lrange(emitter: _Emitter, args: tuple[str, ...]) -> bool:
    """Compile ``lrange`` to specialised opcodes."""
    if len(args) != 3:
        return False
    # Use listRangeImm when both indices are constant (int or end-relative)
    start_idx = _parse_tcl_index(str(args[1]))
    end_idx = _parse_tcl_index(str(args[2]))
    if start_idx is None or end_idx is None:
        return False
    emitter._emit_value(args[0])
    emitter._emit(Op.LIST_RANGE_IMM, start_idx, end_idx)
    emitter._emit(Op.POP)
    return True


def codegen_lreplace(emitter: _Emitter, args: tuple[str, ...]) -> bool:
    """Compile ``lreplace`` to specialised opcodes."""
    if len(args) < 3:
        return False
    # lreplace list first last ?element ...?
    # → load list, push first, push last, push elements,
    #   lreplace4 <stack_count> 1
    emitter._emit_value(args[0])
    for a in args[1:]:
        emitter._emit_value(a)
    emitter._emit(Op.LREPLACE4, len(args), 1)
    emitter._emit(Op.POP)
    return True


def codegen_linsert(emitter: _Emitter, args: tuple[str, ...]) -> bool:
    """Compile ``linsert`` to specialised opcodes."""
    if len(args) < 2:
        return False
    # linsert list index ?element ...?
    # → load list, push index, push elements,
    #   lreplace4 <stack_count> 2
    emitter._emit_value(args[0])
    for a in args[1:]:
        emitter._emit_value(a)
    emitter._emit(Op.LREPLACE4, len(args), 2)
    emitter._emit(Op.POP)
    return True


def codegen_lset(emitter: _Emitter, args: tuple[str, ...]) -> bool:
    """Compile ``lset`` to specialised opcodes."""
    if len(args) < 3:
        return False
    # lset varname ?index ...? newvalue
    var_name = args[0]
    indices = args[1:-1]
    value = args[-1]
    if emitter._is_proc and not emitter._is_qualified(var_name) and len(indices) >= 1:
        slot = emitter._lvt.intern(var_name)
        for idx in indices:
            emitter._emit_value(idx)
        emitter._emit_value(value)
        emitter._emit(Op.LOAD_SCALAR1, slot, comment=f'var "{var_name}"')
        if len(indices) >= 2:
            # Multiple flat indices: use lsetFlat.
            # Operand = indices + value + variable on stack.
            emitter._emit(Op.LSET_FLAT, len(indices) + 2)
        else:
            # Single index (possibly a list like {1 1}): use lsetList.
            emitter._emit(Op.LSET_LIST)
        op = Op.STORE_SCALAR1 if slot < 256 else Op.STORE_SCALAR4
        emitter._emit(op, slot, comment=f'var "{var_name}"')
    else:
        # Non-proc or qualified: use stack-based lsetList.
        # → push varname, push indices, push value,
        #   over <depth>, loadStk, lsetList, storeStk
        depth = len(indices) + 1  # indices + value
        emitter._push_lit(var_name)
        for idx in indices:
            emitter._emit_value(idx)
        emitter._emit_value(value)
        emitter._emit(Op.OVER, depth)
        emitter._emit(Op.LOAD_STK)
        emitter._emit(Op.LSET_LIST)
        emitter._emit(Op.STORE_STK)
    emitter._emit(Op.POP)
    return True


def codegen_list(emitter: _Emitter, args: tuple[str, ...]) -> bool:
    """Compile ``list`` to specialised opcodes."""
    has_vars = any("$" in a or "[" in a for a in args)
    if has_vars:
        # Non-constant args: emit each value, then list N.
        # For [cmd ...] command substitutions, compile inline
        # rather than pushing as literal (which the VM would
        # evaluate at runtime via subst_command, but the
        # analysis pipeline would not).
        for a in args:
            emitter._emit_list_arg(a)
        emitter._emit(Op.LIST, len(args))
        emitter._emit(Op.POP)
        return True
    # Constant-fold list with all-constant args.
    # Strip VM-compiler sentinel markers if present
    # (they wrap braced literals for substitution suppression).
    clean_args: list[str] = []
    for a in args:
        if a.startswith("\x00\x01{") and a.endswith("}\x01\x00"):
            clean_args.append(a[3:-3])
        else:
            clean_args.append(a)
    parts = [_tcl_list_element(a) for a in clean_args]
    result = " ".join(parts) if parts else ""
    # List fold produces a computed Tcl_Obj — use a fresh
    # literal slot so later identical strings get their own.
    idx = emitter._lit.register(result)
    op = Op.PUSH1 if idx < 256 else Op.PUSH4
    # Use a list-specific comment so _fold_const_push_pop_nops
    # does not exclude the empty-string case (it skips '""').
    emitter._emit(
        op,
        idx,
        comment=f'"{_esc(result)}" # list{emitter._NO_DEDUP_TAG}',
    )
    emitter._emit(Op.POP)
    return True


def codegen_concat(emitter: _Emitter, args: tuple[str, ...]) -> bool:
    """Compile ``concat`` to specialised opcodes."""
    if not args:
        return False
    for a in args:
        emitter._emit_value(a)
    emitter._emit(Op.CONCAT_STK, len(args))
    emitter._emit(Op.POP)
    return True


def register() -> None:
    from core.commands.registry import REGISTRY

    REGISTRY.register_codegen("llength", codegen_llength)
    REGISTRY.register_codegen("lassign", codegen_lassign)
    REGISTRY.register_codegen("lindex", codegen_lindex)
    REGISTRY.register_codegen("lrange", codegen_lrange)
    REGISTRY.register_codegen("lreplace", codegen_lreplace)
    REGISTRY.register_codegen("linsert", codegen_linsert)
    REGISTRY.register_codegen("lset", codegen_lset)
    REGISTRY.register_codegen("list", codegen_list)
    REGISTRY.register_codegen("concat", codegen_concat)
