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

"""WASM emit hook for ``lsearch`` — list search with optional switches.

``lsearch ?options? list pattern`` — the Zig ``tcl_cmd_list_search``
export is the no-option form (list, pattern).  When options are
present, fall through to the full ``eval_lsearch`` dispatch handler
via ``tcl_eval`` so that ``-exact``, ``-all``, ``-stride``,
``-nocase``, ``-index PATH``, ``-subindices``, etc. are handled
correctly.
"""

from __future__ import annotations

from compiler.registry import REGISTRY, EmitContext

from .lsort_ import _smart_quote_arg


def _looks_like_option(arg: str) -> bool:
    if not arg or len(arg) < 2 or arg[0] != "-":
        return False
    if arg in ("-", "--"):
        return False
    second = arg[1]
    return not (second.isdigit() or second == ".")


def _emit_lsearch(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    has_options = len(args) > 2 or (len(args) >= 1 and _looks_like_option(args[0]))
    if has_options:
        # Build an eval script with proper single-brace quoting so
        # ``_emit_eval_fallback``'s default list-quote path doesn't
        # double-wrap literal list args into ``{{a b c}}``.  Mirrors
        # the lsort hook's strategy.
        script = "lsearch " + " ".join(_smart_quote_arg(a) for a in args)
        emitter._emit_eval_fallback("lsearch", args, script_override=script)
        if context is EmitContext.STATEMENT:
            from ..._ir import WasmOp

            emitter._emit(WasmOp.DROP)
        return True

    prep = emitter._runtime_prep("lsearch", args)
    if prep is None:
        return False
    func_idx, rimp = prep

    param_count = len(rimp.params)
    for i in range(min(param_count, len(args))):
        emitter._emit_value(args[i])
    for _ in range(param_count - len(args)):
        emitter._emit_i32_const(0)

    emitter._emit_call(func_idx)
    emitter._runtime_call_end(rimp, defs, context)
    return True


REGISTRY.register_wasm_emitter("lsearch", _emit_lsearch)
