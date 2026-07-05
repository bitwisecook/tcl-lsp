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

"""Client: HTTP GET request driver.

Sends one or more HTTP GET requests to the VIP and prints the response.
Use ``--count`` for keep-alive testing (multiple requests on the same
connection to observe HTTP_REQUEST repeating).

Usage::

    python client_http.py --host 10.1.1.10 --port 8081 [--count 1]
"""

from __future__ import annotations

import argparse
import http.client


def main() -> None:
    parser = argparse.ArgumentParser(description="HTTP client (event-ordering test)")
    parser.add_argument("--host", required=True, help="VIP address")
    parser.add_argument("--port", type=int, required=True, help="VIP port")
    parser.add_argument("--count", type=int, default=1, help="Number of requests to send")
    args = parser.parse_args()

    print(f"[http-client] connecting to {args.host}:{args.port}", flush=True)
    conn = http.client.HTTPConnection(args.host, args.port, timeout=10)
    try:
        for i in range(args.count):
            conn.request("GET", f"/test?req={i}")
            resp = conn.getresponse()
            body = resp.read()
            print(
                f"[http-client] GET /test?req={i} -> {resp.status} ({len(body)} bytes)", flush=True
            )
    finally:
        conn.close()
    print("[http-client] done", flush=True)


if __name__ == "__main__":
    main()
