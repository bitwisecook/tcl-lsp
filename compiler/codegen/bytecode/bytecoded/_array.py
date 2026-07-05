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

"""Bytecoded codegen for ``array`` subcommands."""

from __future__ import annotations

from typing import TYPE_CHECKING

from ..opcodes import Op

if TYPE_CHECKING:
    from .._emitter import _Emitter


def codegen_array(emitter: _Emitter, args: tuple[str, ...]) -> bool:
    """Compile ``array`` subcommands using resolved FQ names."""
    if not (len(args) >= 2 and not emitter._is_proc):
        return False
    sub = args[0]
    rest = args[1:]
    match sub:
        case "exists" if (
            len(rest) == 1
            and emitter._is_proc
            and not emitter._is_qualified(rest[0])
            and not emitter._is_array_ref(rest[0])
        ):
            slot = emitter._lvt.intern(rest[0])
            emitter._emit(Op.ARRAY_EXISTS_IMM, slot, comment=f'var "{rest[0]}"')
            emitter._emit(Op.POP)
            return True
        case "names" | "size" if len(rest) >= 1:
            fq_name = f"::tcl::array::{sub}"
            emitter._push_lit(fq_name)
            for a in rest:
                emitter._emit_value(a)
            emitter._emit(Op.INVOKE_STK1, 1 + len(rest))
            emitter._emit(Op.POP)
            emitter._seen_generic_invoke = True
            return True
        case _:
            return False


def register() -> None:
    from compiler.registry import REGISTRY

    REGISTRY.register_codegen("array", codegen_array)
