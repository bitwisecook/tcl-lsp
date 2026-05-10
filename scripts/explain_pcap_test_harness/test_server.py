#!/usr/bin/env python3
"""test_server.py — multi-port HTTP/HTTPS origin for the explain-pcap lab.

Designed to be either:

* SCP'd to a real backend host (10.255.42.10 / 10.255.42.20 in the SCF)
  and run there as the pool member, or
* Executed *on the test client* when the BIG-IP virtual servers all use
  ``automap`` SNAT — the SNATted connection comes back to the BIG-IP self-IP
  which routes it to the client's local stack.  Either way, the server's
  job is the same: respond predictably so the captured pcap shows
  recognisable HTTP traffic plus the F5 trailer's reset-cause when the
  iRule / pool monitor decides to RST.

Endpoints:

  GET  /healthz                  -> 200 "ok\\n"
  GET  /api/v2/health            -> 200 JSON {"status": "ok"}
  GET  /slow                     -> 200 after a sleep, useful for
                                    monitor-timeout testing
  GET  /closeme                  -> server closes socket without
                                    finishing the response (triggers
                                    BIG-IP-side RST cause)
  GET  /500                      -> 500 with a body
  GET  /                         -> 200 with the request line echoed,
                                    so curl traffic is recognisable in
                                    the captured pcap

The server logs every request to stdout so the captured pcaps can be
correlated with what the server thought it was serving.

Usage:
  test_server.py --http-port 8080 [--https-port 8443] [--cert ...]
                 [--key ...] [--bind 0.0.0.0]
"""

from __future__ import annotations

import argparse
import http.server
import json
import os
import signal
import socket
import socketserver
import ssl
import sys
import threading
import time
from urllib.parse import urlparse


class _Handler(http.server.BaseHTTPRequestHandler):
    server_version = "explain-pcap-lab/1.0"

    def log_message(self, fmt: str, *args) -> None:  # noqa: A003
        sys.stdout.write(f"[{self.address_string()}] {self.log_date_time_string()} {fmt % args}\n")
        sys.stdout.flush()

    def _respond(self, status: int, body: bytes, content_type: str = "text/plain") -> None:
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Connection", "close")
        self.end_headers()
        try:
            self.wfile.write(body)
        except (BrokenPipeError, ConnectionResetError):
            pass

    def do_GET(self) -> None:
        path = urlparse(self.path).path

        if path == "/healthz":
            self._respond(200, b"ok\n")
            return

        if path == "/api/v2/health":
            self._respond(
                200,
                json.dumps({"status": "ok", "service": "lab-api"}).encode() + b"\n",
                "application/json",
            )
            return

        if path == "/slow":
            time.sleep(int(os.environ.get("LAB_SLOW_SECS", "20")))
            self._respond(200, b"slow ok\n")
            return

        if path == "/closeme":
            # Send headers but no body, then close abruptly mid-response so
            # the BIG-IP sees an unexpected close on the server side.
            self.send_response(200)
            self.send_header("Content-Type", "text/plain")
            self.send_header("Content-Length", "10")
            self.end_headers()
            try:
                self.wfile.write(b"par")
                self.wfile.flush()
            except OSError:
                pass
            try:
                self.connection.shutdown(socket.SHUT_RDWR)
                self.connection.close()
            except OSError:
                pass
            return

        if path == "/500":
            self._respond(500, b"upstream error\n")
            return

        # Default: echo with the path and host header.
        echo = (
            f"explain-pcap lab\nrequest: {self.command} {self.path}\n"
            f"host: {self.headers.get('Host', '')}\n"
            f"client: {self.client_address[0]}:{self.client_address[1]}\n"
        ).encode()
        self._respond(200, echo)

    def do_POST(self) -> None:
        length = int(self.headers.get("Content-Length", "0") or "0")
        body = self.rfile.read(length) if length else b""
        echoed = f"POST {self.path}\nlength: {length}\n".encode() + body + b"\n"
        self._respond(200, echoed)


class _ThreadingHTTPServer(socketserver.ThreadingMixIn, http.server.HTTPServer):
    daemon_threads = True
    allow_reuse_address = True


def _serve_plain(bind: str, port: int) -> threading.Thread:
    srv = _ThreadingHTTPServer((bind, port), _Handler)
    t = threading.Thread(target=srv.serve_forever, name=f"http-{port}", daemon=True)
    t.start()
    print(f"[server] listening on http://{bind}:{port}")
    return t


def _serve_tls(bind: str, port: int, cert: str, key: str, ca: str = "") -> threading.Thread:
    srv = _ThreadingHTTPServer((bind, port), _Handler)
    ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    ctx.load_cert_chain(certfile=cert, keyfile=key)
    if ca:
        ctx.load_verify_locations(cafile=ca)
        ctx.verify_mode = ssl.CERT_OPTIONAL
    srv.socket = ctx.wrap_socket(srv.socket, server_side=True)
    t = threading.Thread(target=srv.serve_forever, name=f"https-{port}", daemon=True)
    t.start()
    print(f"[server] listening on https://{bind}:{port}")
    return t


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    p.add_argument(
        "--bind",
        default="0.0.0.0",
        help="Listen address (default 0.0.0.0).",
    )
    p.add_argument(
        "--http-port",
        type=int,
        default=8080,
        help="Plain HTTP port; 0 to disable.",
    )
    p.add_argument(
        "--https-port",
        type=int,
        default=8443,
        help="HTTPS port; 0 to disable.",
    )
    p.add_argument(
        "--cert",
        default="",
        help="TLS certificate (PEM).  Required if --https-port>0.",
    )
    p.add_argument(
        "--key",
        default="",
        help="TLS private key (PEM).  Required if --https-port>0.",
    )
    p.add_argument(
        "--ca",
        default="",
        help="Optional CA bundle for client-cert verification (mTLS).",
    )
    args = p.parse_args(argv)

    threads: list[threading.Thread] = []
    if args.http_port:
        threads.append(_serve_plain(args.bind, args.http_port))
    if args.https_port:
        if not args.cert or not args.key:
            print(
                "test_server.py: --https-port requires --cert and --key",
                file=sys.stderr,
            )
            return 2
        threads.append(_serve_tls(args.bind, args.https_port, args.cert, args.key, args.ca))

    if not threads:
        print("test_server.py: nothing to serve (both --http-port and --https-port disabled)")
        return 2

    stopping = threading.Event()
    signal.signal(signal.SIGINT, lambda *_a: stopping.set())
    signal.signal(signal.SIGTERM, lambda *_a: stopping.set())
    try:
        while not stopping.is_set():
            stopping.wait(timeout=1)
    finally:
        print("[server] shutting down")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
