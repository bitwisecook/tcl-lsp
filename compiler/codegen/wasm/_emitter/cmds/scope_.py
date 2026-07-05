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

"""WASM emit hooks for ``global``, ``upvar``, and ``variable``."""

from __future__ import annotations

from compiler.registry import REGISTRY, EmitContext


def _emit_global(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    """``global varName ?varName ...?`` — register variable as global-scoped.

    Pre-loads the global value into a local slot so downstream reads can
    fast-path through the local without a ``tcl_global_get`` call.  Returns
    the empty string (null TclObj in value context).
    """
    for var_name in args:
        emitter._globals.add(var_name)
        # Register the same-name global alias in the runtime frame too
        # (mirrors what ``upvar`` / ``variable`` already do via
        # ``frame_alias_*``).  The compiled read/write fast paths use the
        # WASM-local mirror, so this is mainly for the interpreter-side
        # dispatch that consults the frame: e.g. ``trace add variable x``
        # must see ``x`` as a global so the trace lands in the global
        # directory and survives the proc's return (trace-17.2 / 17.3).
        # No-op at top level (``frame_alias_global`` bails when no frame
        # is active), so only worth emitting inside a proc body.
        if emitter._is_proc:
            alias_idx = emitter._shared_imports.get("tcl_frame_alias_global")
            if alias_idx is not None:
                emitter._emit_obj_literal(var_name)
                emitter._emit_call(alias_idx)
        gget_idx = emitter._shared_imports.get("tcl_global_get")
        if gget_idx is not None:
            local_idx = emitter._intern_local(var_name)
            emitter._emit_obj_literal(var_name)
            emitter._emit_call(gget_idx)
            emitter._emit_local_set(local_idx)
    if context is EmitContext.VALUE:
        emitter._emit_i32_const(0)
    return True


def _emit_upvar(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    """``upvar ?level? otherVar myVar ?otherVar myVar ...?``."""
    emitter._emit_cmd_upvar(args)
    if context is EmitContext.VALUE:
        emitter._emit_i32_const(0)
    return True


def _emit_variable(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    """``variable name ?value? ?name value ...?``."""
    emitter._emit_cmd_variable(args)
    if context is EmitContext.VALUE:
        emitter._emit_i32_const(0)
    return True


REGISTRY.register_wasm_emitter("global", _emit_global)
REGISTRY.register_wasm_emitter("upvar", _emit_upvar)
REGISTRY.register_wasm_emitter("variable", _emit_variable)
