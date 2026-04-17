"""MCP server zipapp entry point.

Usage: python tcl-lsp-mcp-server-VERSION.pyz [--help]

Starts the Tcl/iRules MCP server on stdin/stdout (Model Context Protocol).
Bundles the analysis engine — no pip install needed.  Pure Python, no C extensions.
"""

from __future__ import annotations

import sys


def main() -> int:
    if "--help" in sys.argv or "-h" in sys.argv:
        try:
            from lsp._build_info import BUILD_TIMESTAMP, FULL_VERSION
        except ImportError:
            FULL_VERSION = "dev"
            BUILD_TIMESTAMP = ""
        version = FULL_VERSION
        if BUILD_TIMESTAMP:
            version += f" ({BUILD_TIMESTAMP})"
        print(f"tcl-lsp-mcp-server {version}")
        print("Tcl/iRules MCP Server (Model Context Protocol over stdio)")
        print()
        print("Usage: python tcl-lsp-mcp-server.pyz")
        print("  Starts the MCP server on stdin/stdout.")
        print()
        print("Available tools:")
        print("  analyze            Full analysis (diagnostics + symbols + events)")
        print("  validate           Categorized validation report")
        print("  review             Security-focused diagnostics")
        print("  convert            Legacy pattern detection")
        print("  optimize           Optimization suggestions + rewritten source")
        print("  hover              Hover info at position")
        print("  complete           Completions at position")
        print("  goto_definition    Find definition of symbol")
        print("  find_references    Find all references to symbol")
        print("  symbols            Document symbol hierarchy")
        print("  code_actions       Quick fixes for a source range")
        print("  format_source      Format source code")
        print("  rename             Rename a symbol")
        print("  event_info         iRules event metadata")
        print("  command_info       iRules command metadata")
        print("  event_order        Events in firing order")
        print("  diagram            Control flow diagram data")
        print("  set_dialect        Set active language dialect")
        return 0

    from ai.mcp.tcl_mcp_server import run_stdio

    run_stdio()
    return 0


sys.exit(main())
