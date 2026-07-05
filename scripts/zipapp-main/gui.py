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

"""GUI zipapp entry point: serve the compiler explorer from within the zip archive.

Usage: python tcl-lsp-explorer-gui-VERSION.pyz [--port PORT] [--bind ADDR]
"""

from __future__ import annotations

import http.server
import os
import signal
import sys
import zipfile

# The .pyz is a zip file with a shebang prefix.  When Python executes it,
# __file__ is not reliably set, but sys.argv[0] is always the .pyz path.
_PYZ_PATH = os.path.abspath(sys.argv[0])
_PREFIX = "static/"

# Preload the zip central directory once at import time.
_ZIP = zipfile.ZipFile(_PYZ_PATH, "r")
_NAMELIST = set(_ZIP.namelist())

# Common extension -> MIME type mappings (supplement mimetypes module).
_EXTRA_MIME: dict[str, str] = {
    ".wasm": "application/wasm",
    ".mjs": "text/javascript",
}


class _ZipHandler(http.server.BaseHTTPRequestHandler):
    """Serve static files directly from the zip archive -- no extraction needed."""

    def do_GET(self) -> None:
        # Normalise: strip leading /, drop query/fragment, default to index.html.
        path = self.path.lstrip("/").split("?")[0].split("#")[0]
        if not path or path.endswith("/"):
            path += "index.html"
        zip_path = _PREFIX + path

        if zip_path not in _NAMELIST:
            self.send_error(404, f"Not found: {self.path}")
            return

        data = _ZIP.read(zip_path)
        self.send_response(200)
        self.send_header("Content-Type", self._guess_type(path))
        self.send_header("Content-Length", str(len(data)))
        # Allow SharedArrayBuffer (needed by some Pyodide builds).
        self.send_header("Cross-Origin-Opener-Policy", "same-origin")
        self.send_header("Cross-Origin-Embedder-Policy", "require-corp")
        self.end_headers()
        self.wfile.write(data)

    def do_HEAD(self) -> None:
        self.do_GET()

    def log_message(self, format: str, *args: object) -> None:
        sys.stderr.write(f"[explorer] {format % args}\n")

    @staticmethod
    def _guess_type(path: str) -> str:
        ext = os.path.splitext(path)[1].lower()
        if ext in _EXTRA_MIME:
            return _EXTRA_MIME[ext]
        import mimetypes

        ctype, _ = mimetypes.guess_type(path)
        return ctype or "application/octet-stream"


def main() -> int:
    import argparse

    parser = argparse.ArgumentParser(description="Tcl Compiler Explorer (GUI)")
    parser.add_argument("--port", type=int, default=8080, help="port (default: 8080)")
    parser.add_argument("--bind", default="127.0.0.1", help="bind address (default: 127.0.0.1)")
    args = parser.parse_args()

    with http.server.HTTPServer((args.bind, args.port), _ZipHandler) as httpd:
        url = f"http://{args.bind}:{args.port}"
        print(f"Tcl Compiler Explorer serving at {url}")
        print("Press Ctrl+C to stop.")
        signal.signal(signal.SIGINT, lambda *_: sys.exit(0))
        httpd.serve_forever()

    return 0


sys.exit(main())
