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

"""Public API checks for the two-backend codegen package split.

The parent ``compiler.codegen`` deliberately re-exports nothing — a
caller picks a backend explicitly so neither is privileged.  These
checks pin that contract: the bytecode and WASM subpackages each
expose their own surface, and the bytecode path still produces
disassembly text.
"""

from __future__ import annotations

from compiler.cfg import build_cfg
from compiler.codegen.bytecode import (
    __all__ as bytecode_all,
)
from compiler.codegen.bytecode import (
    codegen_module,
    format_module_asm,
)
from compiler.lowering import lower_to_ir


def test_bytecode_subpackage_exports_expected_symbols():
    expected = {
        "Op",
        "Instruction",
        "FunctionAsm",
        "ModuleAsm",
        "codegen_function",
        "codegen_module",
        "format_function_asm",
        "format_module_asm",
    }
    assert expected.issubset(set(bytecode_all))


def test_wasm_subpackage_exposes_codegen_entry_points():
    from compiler.codegen.wasm import (
        WasmFunction,
        WasmModule,
        wasm_codegen_function,
        wasm_codegen_module,
    )

    assert WasmModule is not None
    assert WasmFunction is not None
    assert callable(wasm_codegen_function)
    assert callable(wasm_codegen_module)


def test_parent_codegen_package_reexports_nothing():
    import compiler.codegen as codegen

    # The parent is a thin doc-only package: it must not privilege
    # either backend by re-exporting its symbols at the top level.
    for name in ("codegen_module", "format_module_asm", "wasm_codegen_module"):
        assert not hasattr(codegen, name), (
            f"compiler.codegen should not re-export {name!r}; "
            "import it from the bytecode/wasm subpackage instead"
        )


def test_bytecode_path_still_generates_disassembly_text():
    source = "set x 1\n"
    ir = lower_to_ir(source)
    cfg = build_cfg(ir)

    module = codegen_module(cfg, ir)
    text = format_module_asm(module)

    assert "ByteCode ::top" in text
    assert "done" in text
