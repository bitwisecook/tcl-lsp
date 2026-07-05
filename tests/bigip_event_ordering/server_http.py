#!/usr/bin/env python3
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

"""Pool member: plain HTTP server.

Returns a 200 response with a small JSON body for every request.
Used as the backend for ``tcp_http`` and ``tcp_clientssl_http``
event-ordering scenarios (where the BIG-IP connects to the pool
member over plain HTTP).

Usage::

    python server_http.py --port 9081 [--bind 0.0.0.0]
"""

from __future__ import annotations

import argparse
import json
import sys
from http.server import BaseHTTPRequestHandler, HTTPServer


class Handler(BaseHTTPRequestHandler):
    """Responds to any method with a 200 JSON body."""

    def do_GET(self) -> None:
        self._respond()

    def do_POST(self) -> None:
        # Read and discard the request body
        length = int(self.headers.get("Content-Length", 0))
        if length:
            self.rfile.read(length)
        self._respond()

    def _respond(self) -> None:
        body = json.dumps({"status": "ok", "server": "event-ordering-pool"}).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, format: str, *args: object) -> None:  # noqa: A002
        print(
            f"[http] {self.client_address[0]}:{self.client_address[1]} {format % args}", flush=True
        )


def main() -> None:
    parser = argparse.ArgumentParser(description="HTTP server (pool member)")
    parser.add_argument("--port", type=int, default=9081)
    parser.add_argument("--bind", default="0.0.0.0")
    args = parser.parse_args()

    srv = HTTPServer((args.bind, args.port), Handler)
    print(f"[http] listening on {args.bind}:{args.port}", flush=True)
    try:
        srv.serve_forever()
    except KeyboardInterrupt:
        print("\n[http] shutting down", flush=True)
        srv.server_close()
        sys.exit(0)


if __name__ == "__main__":
    main()
