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

"""WASM emit hook for ``error`` — 3-arg form populates ``::errorCode``.

The default 1-arg ``error msg`` form goes through the standard
``tcl_error`` runtime import (= ``tcl_cmd_error``).  The 3-arg
``error msg ?info? ?code?`` form is intercepted here and routed
through ``tcl_cmd_error_full`` so the optional info / code arguments
land in ``::errorInfo`` / ``::errorCode`` — without this the
errorCode reverts to the default ``NONE`` and ``catch ... opt; dict
get $opt -errorcode`` loses the explicit class.
"""

from __future__ import annotations

from compiler.registry import REGISTRY, EmitContext


def _emit_error(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    if not args or len(args) < 2:
        return False
    err_full_idx = emitter._shared_imports.get("tcl_error_full")
    if err_full_idx is None:
        return False
    emitter._emit_value(args[0])
    emitter._emit_value(args[1])
    if len(args) >= 3:
        emitter._emit_value(args[2])
    else:
        emitter._emit_i32_const(0)
    emitter._emit_call(err_full_idx)
    if context is EmitContext.VALUE:
        emitter._emit_i32_const(0)
    return True


REGISTRY.register_wasm_emitter("error", _emit_error)
