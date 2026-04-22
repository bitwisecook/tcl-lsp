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
    help="Compile and emit bytecode disassembly.",
)
def _configure_dis(
    p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str
) -> None:
    _add_input_arguments(p, include_output=True, default_dialect=default_dialect)
    p.add_argument(
        "--optimise",
        action="store_true",
        help="Enable optimiser path before disassembly.",
    )
    p.set_defaults(handler=_run_dis)


@verb(
    "compwasm",
    aliases=("wasm",),
    help="Compile source to WebAssembly binary.",
)
def _configure_compwasm(
    p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str
) -> None:
    _add_input_arguments(p, include_output=True, default_dialect=default_dialect)
    p.add_argument(
        "--optimise",
        "-O",
        action="store_true",
        help="Enable WebAssembly optimisation passes.",
    )
    p.add_argument(
        "--wat-output",
        default="",
        help="Optional path for WAT text output.",
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
