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
            from shared._build_info import BUILD_TIMESTAMP, FULL_VERSION
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

    from server.server import server

    server.start_io()
    return 0


sys.exit(main())
