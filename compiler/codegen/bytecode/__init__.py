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

"""Tcl VM bytecode assembly backend.

Takes a pre-SSA :class:`CFGModule` and emits assembly text matching the
format produced by ``tcl::unsupported::disassemble`` in Tcl 9.0.2.

Two-phase approach:
  1. Walk CFG blocks, emit :class:`Instruction` nodes with symbolic labels.
  2. Layout pass resolves labels → byte offsets, then format to text.

Public API::

    codegen_function(cfg, params=()) -> FunctionAsm
    codegen_module(cfg_module, ir_module) -> ModuleAsm
    format_function_asm(asm) -> str
    format_module_asm(module) -> str
"""

from __future__ import annotations

from ._emitter import codegen_function, codegen_module
from ._types import FunctionAsm, Instruction, LiteralTable, LocalVarTable, ModuleAsm
from .format import (
    _esc,
    format_function_asm,
    format_function_explorer,
    format_module_asm,
    format_module_explorer,
)
from .opcodes import Op

__all__ = [
    "FunctionAsm",
    "Instruction",
    "LiteralTable",
    "LocalVarTable",
    "ModuleAsm",
    "Op",
    "_esc",
    "codegen_function",
    "codegen_module",
    "format_function_asm",
    "format_function_explorer",
    "format_module_asm",
    "format_module_explorer",
]
