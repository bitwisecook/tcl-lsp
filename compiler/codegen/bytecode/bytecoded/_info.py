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

"""Bytecoded codegen for ``info`` subcommands."""

from __future__ import annotations

from typing import TYPE_CHECKING

from ..opcodes import Op

if TYPE_CHECKING:
    from .._emitter import _Emitter


def codegen_info(emitter: _Emitter, args: tuple[str, ...]) -> bool:
    """Compile ``info`` subcommands to specialised opcodes."""
    if len(args) < 2:
        return False
    sub = args[0]
    rest = args[1:]
    match sub:
        case "exists" if len(rest) == 1:
            var_name = rest[0]
            if emitter._is_proc and not emitter._is_qualified(var_name):
                slot = emitter._lvt.intern(var_name)
                emitter._emit(Op.EXIST_SCALAR, slot, comment=f'var "{var_name}"')
                emitter._emit(Op.NOP)
            else:
                emitter._push_lit(var_name)
                emitter._emit(Op.EXIST_STK)
            emitter._emit(Op.POP)
            return True
        case _:
            return False


def register() -> None:
    from compiler.registry import REGISTRY

    REGISTRY.register_codegen("info", codegen_info)
