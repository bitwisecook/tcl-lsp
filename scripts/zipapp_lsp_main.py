"""LSP server zipapp entry point.

Usage: python tcl-lsp-server-VERSION.pyz [--help]

When invoked, starts the Tcl language server on stdin/stdout (JSON-RPC).
Pass --help to see the version info instead of starting the server.
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
        print(f"tcl-lsp {version}")
        print("Tcl Language Server (LSP over stdio)")
        print()
        print("Usage: python __main__.py  (or python tcl-lsp-server.pyz)")
        print("  Starts the language server on stdin/stdout (JSON-RPC).")
        return 0

    from lsp.server import server

    server.start_io()
    return 0


sys.exit(main())
