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

"""Client: raw TCP connection driver.

Opens a TCP socket to the VIP, sends a short message, reads the echo
response, and closes.  Use ``--count`` to send multiple messages on
the same connection.

Usage::

    python client_tcp.py --host 10.1.1.10 --port 8080 [--count 1]
"""

from __future__ import annotations

import argparse
import socket


def main() -> None:
    parser = argparse.ArgumentParser(description="TCP client (event-ordering test)")
    parser.add_argument("--host", required=True, help="VIP address")
    parser.add_argument("--port", type=int, required=True, help="VIP port")
    parser.add_argument("--count", type=int, default=1, help="Number of messages to send")
    args = parser.parse_args()

    print(f"[tcp-client] connecting to {args.host}:{args.port}", flush=True)
    with socket.create_connection((args.host, args.port), timeout=10) as sock:
        for i in range(args.count):
            msg = f"HELLO {i}\r\n".encode()
            sock.sendall(msg)
            resp = sock.recv(4096)
            print(f"[tcp-client] sent {msg!r} -> got {resp!r}", flush=True)
    print("[tcp-client] done", flush=True)


if __name__ == "__main__":
    main()
