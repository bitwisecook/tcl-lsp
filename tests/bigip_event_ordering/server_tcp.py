#!/usr/bin/env python3
"""Pool member: plain TCP echo server.

Accepts a connection, reads data until the client closes, echoes it
back, then closes the server side.  Used as the backend for the
``plain_tcp`` event-ordering scenario.

Usage::

    python server_tcp.py --port 9080 [--bind 0.0.0.0]
"""

from __future__ import annotations

import argparse
import socketserver
import sys


class EchoHandler(socketserver.StreamRequestHandler):
    def handle(self) -> None:
        addr = f"{self.client_address[0]}:{self.client_address[1]}"
        print(f"[tcp] connection from {addr}", flush=True)
        while True:
            data = self.request.recv(4096)
            if not data:
                break
            self.request.sendall(data)
        print(f"[tcp] closed {addr}", flush=True)


def main() -> None:
    parser = argparse.ArgumentParser(description="TCP echo server (pool member)")
    parser.add_argument("--port", type=int, default=9080)
    parser.add_argument("--bind", default="0.0.0.0")
    args = parser.parse_args()

    with socketserver.TCPServer((args.bind, args.port), EchoHandler) as srv:
        print(f"[tcp] listening on {args.bind}:{args.port}", flush=True)
        try:
            srv.serve_forever()
        except KeyboardInterrupt:
            print("\n[tcp] shutting down", flush=True)
            sys.exit(0)


if __name__ == "__main__":
    main()
