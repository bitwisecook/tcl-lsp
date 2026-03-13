"""Unified Tcl tools zipapp entry point.

Usage: <tcl|irule> <verb> [args...]

Verbs:
  opt         optimise source text
  diag        run diagnostics across inputs
  lint        run lint diagnostics across inputs
  validate    validate syntax/error diagnostics
  format      format source text
  symbols     emit symbol definitions
  diagram     extract control-flow diagram data
  callgraph   build procedure call graph data
  symbolgraph build symbol relationship graph data
  dataflow    build taint/effect data-flow graph data
  event-order show iRules events in canonical firing order
  event-info  lookup iRules event metadata
  command-info lookup command registry metadata
  convert     detect legacy modernisation patterns
  dis         emit bytecode disassembly
  compwasm    compile to WASM binary
  highlight   emit syntax-highlighted output
  diff        diff two sources via AST/IR/CFG
  explore     compiler-explorer views (IR/CFG/SSA/optimiser/etc.)
  help        search KCS docs
"""

from __future__ import annotations

import sys
from pathlib import Path


def _infer_prog_name(argv0: str) -> str:
    raw_name = Path(argv0).name.strip()
    if not raw_name:
        return "tcl"

    stem = Path(raw_name).stem
    if not stem:
        return "tcl"

    lowered = stem.lower()
    if lowered.startswith("python"):
        return "tcl"
    if lowered.startswith("tcl-"):
        return "tcl"
    if lowered.startswith("irule-"):
        return "irule"
    return stem


def main() -> int:
    prog_name = _infer_prog_name(sys.argv[0])

    if len(sys.argv) <= 1 or sys.argv[1] in {"-h", "--help"}:
        try:
            from lsp._build_info import BUILD_TIMESTAMP, FULL_VERSION
        except ImportError:
            FULL_VERSION = "dev"
            BUILD_TIMESTAMP = ""
        version = FULL_VERSION
        if BUILD_TIMESTAMP:
            version += f" ({BUILD_TIMESTAMP})"
        print(f"{prog_name} {version}")
        print("Unified Tcl toolchain CLI")
        print()
        print(f"Usage: {prog_name} <verb> [args...]")
        print()
        print("Verbs:")
        print("  opt         optimise source text")
        print("  diag        run diagnostics across inputs")
        print("  lint        run lint diagnostics across inputs")
        print("  validate    validate syntax/error diagnostics")
        print("  format      format source text")
        print("  symbols     emit symbol definitions")
        print("  diagram     extract control-flow diagram data")
        print("  callgraph   build procedure call graph data")
        print("  symbolgraph build symbol relationship graph data")
        print("  dataflow    build taint/effect data-flow graph data")
        print("  event-order show iRules events in canonical firing order")
        print("  event-info  lookup iRules event metadata")
        print("  command-info lookup command registry metadata")
        print("  convert     detect legacy modernisation patterns")
        print("  dis         emit bytecode disassembly")
        print("  compwasm    compile to WASM binary")
        print("  highlight   emit syntax-highlighted output")
        print("  diff        diff two sources via AST/IR/CFG")
        print("  explore     compiler-explorer views")
        print("  help        search KCS docs")
        print()
        print(f"Run `{prog_name} <verb> --help` for verb-specific options.")
        return 0

    from explorer.tcl_cli import main as tcl_main

    return tcl_main(sys.argv[1:], prog_name=prog_name)


sys.exit(main())
