#!/usr/bin/env python3
"""Pool member: HTTPS server with auto-generated self-signed certificate.

Used as the backend for scenarios with ServerSSL where the BIG-IP
re-encrypts traffic to the pool member.

On first run, generates a self-signed cert+key pair in the current
directory (``pool_cert.pem`` / ``pool_key.pem``).

Usage::

    python server_https.py --port 9443 [--bind 0.0.0.0] \
        [--cert pool_cert.pem] [--key pool_key.pem]
"""

from __future__ import annotations

import argparse
import json
import os
import ssl
import subprocess
import sys
from http.server import BaseHTTPRequestHandler, HTTPServer


class Handler(BaseHTTPRequestHandler):
    def do_GET(self) -> None:
        self._respond()

    def do_POST(self) -> None:
        length = int(self.headers.get("Content-Length", 0))
        if length:
            self.rfile.read(length)
        self._respond()

    def _respond(self) -> None:
        body = json.dumps({"status": "ok", "server": "event-ordering-pool-tls"}).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, format: str, *args: object) -> None:  # noqa: A002
        print(
            f"[https] {self.client_address[0]}:{self.client_address[1]} {format % args}", flush=True
        )


def _ensure_cert(cert_path: str, key_path: str) -> None:
    """Generate a self-signed cert if it doesn't already exist."""
    if os.path.exists(cert_path) and os.path.exists(key_path):
        return
    print(f"[https] generating self-signed cert: {cert_path}", flush=True)
    subprocess.run(
        [
            "openssl",
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-keyout",
            key_path,
            "-out",
            cert_path,
            "-days",
            "365",
            "-nodes",
            "-subj",
            "/CN=event-ordering-pool",
        ],
        check=True,
        capture_output=True,
    )


def main() -> None:
    parser = argparse.ArgumentParser(description="HTTPS server (pool member)")
    parser.add_argument("--port", type=int, default=9443)
    parser.add_argument("--bind", default="0.0.0.0")
    parser.add_argument("--cert", default="pool_cert.pem")
    parser.add_argument("--key", default="pool_key.pem")
    args = parser.parse_args()

    _ensure_cert(args.cert, args.key)

    ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    ctx.load_cert_chain(args.cert, args.key)

    srv = HTTPServer((args.bind, args.port), Handler)
    srv.socket = ctx.wrap_socket(srv.socket, server_side=True)

    print(f"[https] listening on {args.bind}:{args.port}", flush=True)
    try:
        srv.serve_forever()
    except KeyboardInterrupt:
        print("\n[https] shutting down", flush=True)
        srv.server_close()
        sys.exit(0)


if __name__ == "__main__":
    main()
