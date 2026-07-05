#!/usr/bin/env python3
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

"""Dump our codegen disassembly for all bytecode snippets.

Usage:
    uv run python scripts/dev/dump_our_bytecode.py [output_dir]

Writes one .disasm file per snippet to output_dir (default:
tests/bytecode_reference/ours/).
"""

from __future__ import annotations

import sys
from pathlib import Path

# Ensure repo root is on sys.path
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

from compiler.cfg import build_cfg
from compiler.codegen.bytecode import codegen_module, format_module_asm
from compiler.lowering import lower_to_ir


def compile_and_format(source: str) -> str:
    ir = lower_to_ir(source)
    cfg = build_cfg(ir)
    module = codegen_module(cfg, ir)
    return format_module_asm(module)


def main() -> None:
    snippets_dir = Path(__file__).resolve().parents[2] / "tests" / "bytecode_snippets"
    output_dir = (
        Path(sys.argv[1])
        if len(sys.argv) > 1
        else (Path(__file__).resolve().parents[2] / "tests" / "bytecode_reference" / "ours")
    )
    output_dir.mkdir(parents=True, exist_ok=True)

    snippets = sorted(snippets_dir.glob("*.tcl"))
    for snippet in snippets:
        source = snippet.read_text()
        try:
            disasm = compile_and_format(source)
        except Exception as e:
            disasm = f"ERROR: {e}"

        outfile = output_dir / f"{snippet.stem}.disasm"
        outfile.write_text(disasm + "\n")
        print(f"  {snippet.stem} -> {outfile.name}")

    print(f"\n{len(snippets)} files written to {output_dir}")


if __name__ == "__main__":
    main()
