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

"""WASM emit hook for ``format`` — Tcl printf-style string formatting.

The Zig ``tcl_cmd_format`` export takes a fixed 4-slot signature
``(fmt, a1, a2, a3)``; for callsites with more than three
substitution arguments (or where ``%-*s`` style consumes two args
per spec — opt.test's ``OptTree``), the codegen routes through the
variadic ``tcl_cmd_format_list`` export instead, packing args into
a Tcl list at runtime.
"""

from __future__ import annotations

from compiler.registry import REGISTRY, EmitContext

from ..._ir import WasmOp


def _emit_format(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    if len(args) > 4:
        format_list_idx = emitter._shared_imports.get("tcl_cmd_format_list")
        list_create_idx = emitter._shared_imports.get("tcl_list_create")
        if format_list_idx is None or list_create_idx is None:
            return False
        # Stack:
        #   <fmt> <empty list seed>
        #   <arg1> -> tcl_list seed,arg1 -> [arg1]
        #   <arg2> -> tcl_list [arg1],arg2 -> [arg1,arg2]
        #   ...
        # Then call tcl_cmd_format_list(fmt, args_list).
        emitter._emit_value(args[0])  # fmt
        emitter._emit_obj_literal("")  # list seed
        for a in args[1:]:
            emitter._emit_value(a)
            emitter._emit_call(list_create_idx)
        emitter._emit_call(format_list_idx)
        # tcl_cmd_format_list returns an i32 TclObj — settle per
        # context the same way ``_runtime_call_end`` would.
        if context is EmitContext.VALUE:
            return True
        if defs:
            def_idx = emitter._intern_local(defs[0])
            emitter._emit_local_set(def_idx)
        else:
            emitter._emit(WasmOp.DROP)
        return True

    prep = emitter._runtime_prep("format", args)
    if prep is None:
        return False
    func_idx, rimp = prep

    if not args:
        emitter._emit_i32_const(0)
        emitter._emit_i32_const(0)
        emitter._emit_i32_const(0)
        emitter._emit_i32_const(0)
    else:
        emitter._emit_value(args[0])
        for slot in range(1, 4):
            if slot < len(args):
                emitter._emit_value(args[slot])
            else:
                emitter._emit_i32_const(0)
    emitter._emit_call(func_idx)
    emitter._runtime_call_end(rimp, defs, context)
    return True


REGISTRY.register_wasm_emitter("format", _emit_format)
