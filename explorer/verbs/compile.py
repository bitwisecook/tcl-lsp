"""Low-level compilation verbs: dis, compwasm."""

from __future__ import annotations

import argparse
import sys

from core.commands.registry.runtime import configure_signatures
from core.compiler.cfg import build_cfg
from core.compiler.codegen import format_module_asm
from core.compiler.codegen.wasm import wasm_codegen_module
from core.compiler.lowering import lower_to_ir
from vm.compiler import compile_script

from ._registry import verb
from ._utils import (
    _add_input_arguments,
    _combine_sources,
    _read_input_documents,
    _write_binary_output,
    _write_text_output,
)


@verb(
    "dis",
    aliases=("asm", "disassemble"),
    help="Compile source and emit human-readable bytecode disassembly.",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure_dis(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:
    p.description = (
        "Compile each input through the bytecode VM and print the resulting\n"
        "instruction listing (the same `ByteCode ::top` / `done` shape that\n"
        "tclsh's `tcl::unsupported::disassemble` produces).  Useful for\n"
        "verifying codegen, debugging compiler regressions, and comparing\n"
        "our output against reference tclsh bytecode.\n"
    )
    p.epilog = (
        "Examples:\n"
        f"  {prog_name} dis script.tcl\n"
        f"  {prog_name} dis script.tcl --optimise\n"
        f"  {prog_name} dis script.tcl -o asm.txt\n"
    )
    _add_input_arguments(p, include_output=True, default_dialect=default_dialect)
    p.add_argument(
        "--optimise",
        action="store_true",
        help="Run the optimiser before disassembly (shows the post-opt bytecode).",
    )
    p.set_defaults(handler=_run_dis)


@verb(
    "compwasm",
    aliases=("wasm",),
    help="Compile source to a WebAssembly binary.",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure_compwasm(
    p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str
) -> None:
    p.description = (
        "Lower source through the IR/CFG/WASM codegen pipeline and write a\n"
        "stand-alone `.wasm` module that the Zig-based runtime can\n"
        "interpret under `wasmtime`.  Pair with --wat-output to also emit\n"
        "the textual WAT representation for inspection.\n"
    )
    p.epilog = (
        "Examples:\n"
        f"  {prog_name} compwasm script.tcl -o out.wasm\n"
        f"  {prog_name} compwasm script.tcl --wat-output out.wat\n"
        f"  {prog_name} compwasm script.tcl -O -o out.wasm\n"
    )
    _add_input_arguments(p, include_output=True, default_dialect=default_dialect)
    p.add_argument(
        "--optimise",
        "-O",
        action="store_true",
        help="Enable WebAssembly-side optimisation passes.",
    )
    p.add_argument(
        "--wat-output",
        default="",
        help="Optional path for the WAT (WebAssembly text) sidecar output.",
    )
    p.set_defaults(handler=_run_compwasm, output="out.wasm")


# ---------------------------------------------------------------------------
# Handlers
# ---------------------------------------------------------------------------


def _run_dis(args: argparse.Namespace) -> int:
    documents = _read_input_documents(
        args.inputs,
        inline_sources=args.source,
        package_paths=args.package_path,
        recursive=not args.no_recursive,
    )
    configure_signatures(dialect=args.dialect)

    source = _combine_sources(documents)
    module_asm, _ = compile_script(source, optimise=args.optimise)
    disassembly = format_module_asm(module_asm)
    _write_text_output(args.output, disassembly)
    return 0


def _run_compwasm(args: argparse.Namespace) -> int:
    documents = _read_input_documents(
        args.inputs,
        inline_sources=args.source,
        package_paths=args.package_path,
        recursive=not args.no_recursive,
    )
    configure_signatures(dialect=args.dialect)

    source = _combine_sources(documents)
    ir_module = lower_to_ir(source)
    cfg_module = build_cfg(ir_module)
    wasm_module = wasm_codegen_module(cfg_module, ir_module, optimise=args.optimise)
    wasm_bytes = wasm_module.to_bytes()

    _write_binary_output(args.output, wasm_bytes)
    if args.wat_output:
        _write_text_output(args.wat_output, wasm_module.to_wat())

    output_target = "stdout" if args.output == "-" else args.output
    print(
        f"wrote wasm binary ({len(wasm_bytes)} bytes) to {output_target}",
        file=sys.stderr,
    )
    return 0
