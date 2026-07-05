# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

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
