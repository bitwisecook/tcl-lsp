#!/usr/bin/env python3
"""Dump our codegen disassembly for all bytecode snippets.

Usage:
    uv run python scripts/dump_our_bytecode.py [output_dir]

Writes one .disasm file per snippet to output_dir (default:
tests/bytecode_reference/ours/).
"""

from __future__ import annotations

import sys
from pathlib import Path

# Ensure repo root is on sys.path
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.compiler.cfg import build_cfg
from core.compiler.codegen import codegen_module, format_module_asm
from core.compiler.lowering import lower_to_ir


def compile_and_format(source: str) -> str:
    ir = lower_to_ir(source)
    cfg = build_cfg(ir)
    module = codegen_module(cfg, ir)
    return format_module_asm(module)


def main() -> None:
    snippets_dir = Path(__file__).resolve().parent.parent / "tests" / "bytecode_snippets"
    output_dir = (
        Path(sys.argv[1])
        if len(sys.argv) > 1
        else (Path(__file__).resolve().parent.parent / "tests" / "bytecode_reference" / "ours")
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
