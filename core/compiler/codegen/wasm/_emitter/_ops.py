"""Module-level constants and helpers shared across all emitter mixins."""

from __future__ import annotations

from ....expr_ast import BinOp, UnaryOp
from .._ir import WasmOp

# Maps Tcl binary expression operators to WASM i64 opcodes
_BINOP_WASM: dict[BinOp, int] = {
    BinOp.ADD: WasmOp.I64_ADD,
    BinOp.SUB: WasmOp.I64_SUB,
    BinOp.MUL: WasmOp.I64_MUL,
    BinOp.DIV: WasmOp.I64_DIV_S,
    BinOp.MOD: WasmOp.I64_REM_S,
    BinOp.LSHIFT: WasmOp.I64_SHL,
    BinOp.RSHIFT: WasmOp.I64_SHR_S,
    BinOp.BIT_AND: WasmOp.I64_AND,
    BinOp.BIT_OR: WasmOp.I64_OR,
    BinOp.BIT_XOR: WasmOp.I64_XOR,
    BinOp.EQ: WasmOp.I64_EQ,
    BinOp.NE: WasmOp.I64_NE,
    BinOp.LT: WasmOp.I64_LT_S,
    BinOp.GT: WasmOp.I64_GT_S,
    BinOp.LE: WasmOp.I64_LE_S,
    BinOp.GE: WasmOp.I64_GE_S,
}

# Maps Tcl unary operators to WASM equivalents
_UNARYOP_WASM: dict[UnaryOp, int | None] = {
    UnaryOp.NEG: None,  # implemented as 0 - x
    UnaryOp.POS: None,  # no-op
    UnaryOp.BIT_NOT: None,  # implemented as x ^ -1
    UnaryOp.NOT: WasmOp.I64_EQZ,
}


def _is_end_relative_index(idx: str) -> bool:
    """True if *idx* is an ``end`` / ``end-N`` / ``end+N`` index literal.

    Used by the multi-value linsert/lreplace chain to choose between
    forward-order iteration (for ``end``-family indices, whose position
    re-resolves after each insert) and reverse-order (for numeric
    indices, whose position stays pinned).  The check is textual and
    conservative: only plain ``end``-prefixed words are treated as
    end-relative; variable refs and command substitutions fall through
    to the reverse-order path.
    """
    if not idx:
        return False
    # Direct ``end`` / ``end-N`` / ``end+N`` — the three shapes
    # ``resolve_list_index`` handles in the Zig runtime.
    if idx == "end":
        return True
    if idx.startswith("end-") or idx.startswith("end+"):
        return True
    return False
