"""CLI tool for compiling Tcl scripts to WebAssembly.

Usage:
    python -m explorer.wasm_cli [OPTIONS] [FILE]

Options:
    --source TEXT       Tcl source code (instead of file)
    --optimise / -O     Enable WASM optimisations
    --format FMT        Output format: wasm (binary), wat (text), both
    --output / -o PATH  Output file (default: stdout for WAT, out.wasm for binary)

Examples:
    # Compile to WAT (human-readable), non-optimised
    python -m explorer.wasm_cli script.tcl --format wat

    # Compile to WAT, optimised
    python -m explorer.wasm_cli script.tcl --format wat --optimise

    # Compile to WASM binary
    python -m explorer.wasm_cli script.tcl --format wasm -o output.wasm

    # Compare optimised vs non-optimised
    python -m explorer.wasm_cli --source 'set x [expr {1 + 2}]' --format both
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

try:
    from lsp._build_info import FULL_VERSION
except ImportError:
    FULL_VERSION = "dev"

from core.compiler.cfg import build_cfg
from core.compiler.codegen.wasm import WasmModule, wasm_codegen_module
from core.compiler.lowering import lower_to_ir


def compile_tcl_to_wasm(
    source: str,
    *,
    optimise: bool = False,
) -> WasmModule:
    """Compile Tcl source to a WASM module.

    Args:
        source: Tcl source code.
        optimise: Enable optimisation passes (constant folding,
            peephole optimisation, dead-code elimination).

    Returns:
        A ``WasmModule`` ready for serialisation.
    """
    ir_module = lower_to_ir(source)
    cfg_module = build_cfg(ir_module)
    return wasm_codegen_module(cfg_module, ir_module, optimise=optimise)


def main() -> int:
    """CLI entry point for the Tcl-to-WASM compiler."""
    parser = argparse.ArgumentParser(
        description="Compile Tcl scripts to WebAssembly",
        prog="tcl-wasm",
    )
    parser.add_argument(
        "--version",
        action="version",
        version=f"tcl-wasm {FULL_VERSION}",
    )
    parser.add_argument(
        "file",
        nargs="?",
        help="Tcl source file (reads from stdin if omitted and no --source)",
    )
    parser.add_argument(
        "--source",
        "-s",
        help="Tcl source code (instead of file)",
    )
    parser.add_argument(
        "--optimise",
        "-O",
        action="store_true",
        default=False,
        help="Enable WASM optimisations (constant folding, peephole, DCE)",
    )
    parser.add_argument(
        "--format",
        "-f",
        choices=["wasm", "wat", "both"],
        default="wat",
        help="Output format: wasm (binary), wat (text), both (default: wat)",
    )
    parser.add_argument(
        "--output",
        "-o",
        help="Output file path (default: stdout for WAT, out.wasm for binary)",
    )

    args = parser.parse_args()

    # Read source
    if args.source:
        source = args.source
    elif args.file:
        source = Path(args.file).read_text()
    elif not sys.stdin.isatty():
        source = sys.stdin.read()
    else:
        parser.error("No input: provide a file, --source, or pipe via stdin")
        return 1

    # Compile
    module = compile_tcl_to_wasm(source, optimise=args.optimise)

    opt_label = "optimised" if args.optimise else "non-optimised"

    if args.format in ("wat", "both"):
        wat = module.to_wat()
        if args.format == "both":
            print(f";; === WAT output ({opt_label}) ===")
        if args.output and args.format == "wat":
            out_path = args.output
            Path(out_path).write_text(wat)
            print(f"Wrote WAT to {out_path}", file=sys.stderr)
        else:
            print(wat)

    if args.format in ("wasm", "both"):
        wasm_bytes = module.to_bytes()
        if args.output:
            out_path = args.output
        elif args.format == "both":
            out_path = "out.wasm"
        else:
            out_path = args.output or "out.wasm"

        Path(out_path).write_bytes(wasm_bytes)
        size = len(wasm_bytes)
        print(
            f"Wrote WASM binary to {out_path} ({size} bytes, {opt_label})",
            file=sys.stderr,
        )

    if args.format == "both":
        # Also show comparison info
        module_no_opt = compile_tcl_to_wasm(source, optimise=False)
        module_opt = compile_tcl_to_wasm(source, optimise=True)

        no_opt_size = len(module_no_opt.to_bytes())
        opt_size = len(module_opt.to_bytes())

        no_opt_instrs = sum(len(f.body) for f in module_no_opt.functions)
        opt_instrs = sum(len(f.body) for f in module_opt.functions)

        print(file=sys.stderr)
        print(";; === Comparison ===", file=sys.stderr)
        print(
            f";;   Non-optimised: {no_opt_size} bytes, {no_opt_instrs} instructions",
            file=sys.stderr,
        )
        print(
            f";;   Optimised:     {opt_size} bytes, {opt_instrs} instructions",
            file=sys.stderr,
        )
        if no_opt_size > 0:
            pct = (1 - opt_size / no_opt_size) * 100
            print(
                f";;   Reduction:     {pct:.1f}% smaller",
                file=sys.stderr,
            )

    return 0


if __name__ == "__main__":
    sys.exit(main())
