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

"""WASM emit hook for ``concat`` — variadic whitespace-trimming concat.

``concat ?arg arg ...?`` trims leading + trailing whitespace from
each argument and joins the non-empty results with a single space.
When every argument is a simple literal the entire result is folded
at compile time; mixed literals + runtime refs chain per-argument
``tcl_cmd_concat`` calls.
"""

from __future__ import annotations

from compiler.registry import REGISTRY, EmitContext


def _emit_concat(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    prep = emitter._runtime_prep("concat", args)
    if prep is None:
        return False
    func_idx, rimp = prep

    def _concat_is_lit(a: str) -> bool:
        return (
            not a.startswith("$")
            and not a.startswith("[")
            and not emitter._has_embedded_subst(a)
            and a not in emitter._aliases
            and a not in emitter._local_index
        )

    all_lits = all(_concat_is_lit(a) for a in args)
    if not args:
        emitter._emit_obj_literal("")
    elif all_lits:
        parts = [a.strip() for a in args]
        emitter._emit_obj_literal(" ".join(p for p in parts if p))
    else:
        emitter._emit_value(args[0])
        for a in args[1:]:
            emitter._emit_value(a)
            emitter._emit_call(func_idx)
    emitter._runtime_call_end(rimp, defs, context)
    return True


REGISTRY.register_wasm_emitter("concat", _emit_concat)
