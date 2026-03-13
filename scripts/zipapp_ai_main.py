"""AI analysis zipapp entry point.

Usage: python tcl-lsp-ai-VERSION.pyz <subcommand> <args...>

Standalone Tcl/iRules analysis tool for Claude Code AI skills.
Bundles the server analysis engine — no LSP server or pip install needed.
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
        print(f"tcl-lsp-ai {version}")
        print("Standalone Tcl/iRules analysis tool for Claude Code AI skills")
        print()
        print("Usage: python tcl-lsp-ai.pyz <subcommand> <args...>")
        print()
        print("Subcommands:")
        print("  context <file>          Build context pack (diagnostics + symbols + events)")
        print("  diagnostics <file>      Show diagnostics from the analyzer")
        print("  validate <file>         Categorized validation report")
        print("  review <file>           Security-focused diagnostic report")
        print("  convert <file>          Detect legacy patterns for modernisation")
        print("  symbols <file>          Show document symbol hierarchy")
        print("  diagram <file>          Extract control flow from compiler IR")
        print("  optimize <file>         Show optimization suggestions and rewritten source")
        print("  event-order <file>      Show events in canonical firing order")
        print("  event-info <EVENT>      Show iRules event registry metadata")
        print("  command-info <CMD>      Show iRules command registry metadata")
        return 0

    from ai.claude.tcl_ai import main as tcl_ai_main

    tcl_ai_main()
    return 0


sys.exit(main())
