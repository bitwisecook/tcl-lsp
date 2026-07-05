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

"""Bytecoded codegen for ``namespace`` subcommands."""

from __future__ import annotations

from typing import TYPE_CHECKING

from ..opcodes import Op

if TYPE_CHECKING:
    from .._emitter import _Emitter


def codegen_namespace(emitter: _Emitter, args: tuple[str, ...]) -> bool:
    """Compile ``namespace`` subcommands."""
    if not (len(args) >= 2 and not emitter._is_proc):
        return False
    sub = args[0]
    rest = args[1:]
    match sub:
        case "eval" if len(rest) >= 1:
            # invokeReplace pattern: push original words, args, FQ name.
            emitter._push_lit("namespace")
            emitter._push_lit("eval")
            for a in rest:
                emitter._emit_value(a)
            emitter._push_lit("::tcl::namespace::eval")
            emitter._emit(Op.INVOKE_REPLACE, 2 + len(rest), 2)
            emitter._emit(Op.POP)
            emitter._seen_generic_invoke = True
            return True
        case _:
            return False


def register() -> None:
    from compiler.registry import REGISTRY

    REGISTRY.register_codegen("namespace", codegen_namespace)
