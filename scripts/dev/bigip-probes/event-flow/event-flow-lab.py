#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Dependency-free client and backend for BIG-IP event-flow measurements."""

import argparse
import base64
import hashlib
import http.server
import json
import socket
import ssl
import struct
import threading
import time
from pathlib import Path


class Recorder:
    def __init__(self, path: Path):
        self.path = path
        self.lock = threading.Lock()

    def write(self, **values):
        values["utc_ns"] = time.time_ns()
        row = json.dumps(values, sort_keys=True, separators=(",", ":"))
        with self.lock, self.path.open("a", encoding="utf-8", newline="\n") as stream:
            stream.write(row + "\n")


def encoded(data: bytes) -> dict:
    return {
        "bytes": len(data),
        "sha256": hashlib.sha256(data).hexdigest(),
        "base64": base64.b64encode(data).decode("ascii"),
    }


class HttpHandler(http.server.BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
    recorder: Recorder

    def log_message(self, _format, *args):
        del args

    def do_GET(self):
        self.respond()

    def do_HEAD(self):
        self.respond()

    def do_POST(self):
        self.respond()

    def respond(self):
        length = int(self.headers.get("Content-Length", "0"))
        body_in = self.rfile.read(length) if length else b""
        request_id = self.headers.get("X-Evtflow-Request", "none")
        status = 200
        body = b"evtflow-ok\n"
        headers = {}
        if self.path.startswith("/missing"):
            status, body = 404, b"evtflow-missing\n"
        elif self.path.startswith("/auth"):
            status, body = 401, b"evtflow-auth\n"
            headers["WWW-Authenticate"] = 'Basic realm="evtflow"'
        elif self.path.startswith("/collect"):
            body = b"evtflow-collected-response\n"
            headers["X-Evtflow-Collect"] = "yes"
        elif self.path.startswith("/cache"):
            body = b"evtflow-cacheable\n"
            headers["Cache-Control"] = "public, max-age=300"
        elif self.path.startswith("/nocontent"):
            status, body = 204, b""
        self.recorder.write(
            role="http-backend",
            peer=[self.client_address[0], self.client_address[1]],
            local=[self.server.server_address[0], self.server.server_address[1]],
            request=request_id,
            method=self.command,
            path=self.path,
            version=self.request_version,
            headers=list(self.headers.items()),
            body=encoded(body_in),
        )
        self.send_response(status)
        for name, value in headers.items():
            self.send_header(name, value)
        self.send_header("Content-Length", str(len(body)))
        self.send_header("X-Evtflow-Backend", "dev.bitwisecook.org")
        self.end_headers()
        if self.command != "HEAD":
            self.wfile.write(body)


def serve_tcp(bind: str, port: int, recorder: Recorder):
    listener = socket.create_server((bind, port), reuse_port=False)
    while True:
        connection, peer = listener.accept()
        threading.Thread(
            target=handle_tcp,
            args=(connection, peer, listener.getsockname(), recorder),
            daemon=True,
        ).start()


def handle_tcp(connection, peer, local, recorder):
    with connection:
        data = connection.recv(65535)
        recorder.write(
            role="tcp-backend",
            peer=list(peer),
            local=list(local),
            payload=encoded(data),
        )
        if data.startswith((b"OPTIONS ", b"DESCRIBE ", b"SETUP ", b"PLAY ")):
            response = b"RTSP/1.0 200 OK\r\nCSeq: 1\r\nContent-Length: 0\r\n\r\n"
        elif data.startswith(b"\x10"):
            response = b"\x20\x02\x00\x00"
        else:
            response = b"TCP-ECHO:" + data
        connection.sendall(response)


def serve_udp(bind: str, port: int, recorder: Recorder, dns: bool):
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.bind((bind, port))
    while True:
        data, peer = sock.recvfrom(65535)
        recorder.write(
            role="dns-backend" if dns else "udp-backend",
            peer=list(peer),
            local=list(sock.getsockname()),
            payload=encoded(data),
        )
        if dns:
            response = dns_response(data)
        elif data.startswith((b"INVITE ", b"OPTIONS ", b"REGISTER ")):
            headers = {}
            for line in data.split(b"\r\n")[1:]:
                if b":" in line:
                    name, value = line.split(b":", 1)
                    headers[name.lower()] = value.strip()
            response = b"SIP/2.0 200 OK\r\n"
            for name in (b"via", b"from", b"to", b"call-id", b"cseq"):
                if name in headers:
                    response += name.title() + b": " + headers[name] + b"\r\n"
            response += b"Content-Length: 0\r\n\r\n"
        else:
            response = b"UDP-ECHO:" + data
        sock.sendto(response, peer)


def dns_response(query: bytes) -> bytes:
    if len(query) < 12:
        return query
    question_count = struct.unpack("!H", query[4:6])[0]
    if question_count != 1:
        return (
            query[:2]
            + b"\x81\x81"
            + query[4:6]
            + b"\x00\x00\x00\x00\x00\x00"
            + query[12:]
        )
    offset = 12
    while offset < len(query) and query[offset] != 0:
        offset += query[offset] + 1
    question_end = offset + 5
    question = query[12:question_end]
    answer = b"\xc0\x0c\x00\x01\x00\x01\x00\x00\x00\x3c\x00\x04\xcb\x00\x71\x07"
    return query[:2] + b"\x81\x80\x00\x01\x00\x01\x00\x00\x00\x00" + question + answer


def serve(args):
    recorder = Recorder(args.log)
    threads = [
        threading.Thread(
            target=serve_tcp, args=(args.bind, args.base, recorder), daemon=True
        ),
        threading.Thread(
            target=serve_udp,
            args=(args.bind, args.base + 4, recorder, False),
            daemon=True,
        ),
        threading.Thread(
            target=serve_udp,
            args=(args.bind, args.base + 5, recorder, True),
            daemon=True,
        ),
    ]
    for offset in (1, 2, 3):
        server = http.server.ThreadingHTTPServer(
            (args.bind, args.base + offset), HttpHandler
        )
        server.RequestHandlerClass.recorder = recorder
        threads.append(threading.Thread(target=server.serve_forever, daemon=True))
    for thread in threads:
        thread.start()
    recorder.write(role="server-start", bind=args.bind, base=args.base)
    while True:
        time.sleep(3600)


def serve_tls(args):
    recorder = Recorder(args.log)
    server = http.server.ThreadingHTTPServer((args.bind, args.port), HttpHandler)
    server.RequestHandlerClass.recorder = recorder
    context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    context.load_cert_chain(args.cert, args.key)
    server.socket = context.wrap_socket(server.socket, server_side=True)
    recorder.write(role="tls-server-start", bind=args.bind, port=args.port)
    server.serve_forever()


def receive_all(sock) -> bytes:
    result = bytearray()
    sock.settimeout(2)
    while True:
        try:
            chunk = sock.recv(65535)
        except TimeoutError:
            break
        if not chunk:
            break
        result.extend(chunk)
    return bytes(result)


def tcp_exchange(vip, port, source_port, payload, use_tls=False):
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    sock.bind(("0.0.0.0", source_port))
    sock.settimeout(5)
    sock.connect((vip, port))
    if use_tls:
        context = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
        context.check_hostname = False
        context.verify_mode = ssl.CERT_NONE
        sock = context.wrap_socket(sock, server_hostname="evtflow.invalid")
    sock.sendall(payload)
    try:
        sock.shutdown(socket.SHUT_WR)
    except OSError:
        pass
    response = receive_all(sock)
    tls = None
    if use_tls:
        tls = {"version": sock.version(), "cipher": sock.cipher()}
    sock.close()
    return response, tls


def udp_exchange(vip, port, source_port, payload):
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.bind(("0.0.0.0", source_port))
    sock.settimeout(5)
    sock.sendto(payload, (vip, port))
    response, peer = sock.recvfrom(65535)
    sock.close()
    return response, peer


def http_request(request_id: str, path: str, method="GET", body=b"") -> bytes:
    return (
        f"{method} {path} HTTP/1.1\r\n"
        "Host: evtflow.invalid\r\n"
        f"X-Evtflow-Request: {request_id}\r\n"
        f"Content-Length: {len(body)}\r\n"
        "Connection: close\r\n\r\n"
    ).encode("ascii") + body


def dns_query(identifier: int) -> bytes:
    labels = b"\x07evtflow\x07invalid\x00"
    return (
        struct.pack("!HHHHHH", identifier, 0x0100, 1, 0, 0, 0)
        + labels
        + b"\x00\x01\x00\x01"
    )


def matrix(args):
    recorder = Recorder(args.log)
    cases = []
    source_port = args.source_port_base

    def record(case, protocol, request, response, **extra):
        recorder.write(
            role="client",
            case=case,
            protocol=protocol,
            request=encoded(request),
            response=encoded(response),
            source_port=extra.pop("source_port"),
            **extra,
        )

    for index in range(args.repetitions):
        request_id = f"tcp-{index:02d}"
        payload = (request_id + "\n").encode("ascii")
        response, _ = tcp_exchange(args.vip, args.base, source_port, payload)
        record(request_id, "tcp", payload, response, source_port=source_port)
        source_port += 1

    http_cases = [
        ("ok", "/ok", "GET", b""),
        ("missing", "/missing", "GET", b""),
        ("auth", "/auth", "GET", b""),
        ("collect", "/collect", "GET", b""),
        ("post", "/post", "POST", b"request-body"),
        ("head", "/ok", "HEAD", b""),
    ]
    for name, path, method, body in http_cases:
        request_id = f"http-{name}"
        request = http_request(request_id, path, method, body)
        response, _ = tcp_exchange(args.vip, args.base + 1, source_port, request)
        record(request_id, "http1", request, response, source_port=source_port)
        source_port += 1

    request = http_request("https-ok", "/ok")
    response, tls = tcp_exchange(
        args.vip, args.base + 2, source_port, request, use_tls=True
    )
    record(
        "https-ok", "https-http1", request, response, source_port=source_port, tls=tls
    )
    source_port += 1

    udp_request = b"udp-request"
    udp_response, udp_peer = udp_exchange(
        args.vip, args.base + 4, source_port, udp_request
    )
    record(
        "udp",
        "udp",
        udp_request,
        udp_response,
        source_port=source_port,
        peer=list(udp_peer),
    )
    source_port += 1

    query = dns_query(0xE701)
    dns_answer, dns_peer = udp_exchange(args.vip, args.base + 5, source_port, query)
    record(
        "dns-a",
        "dns-udp",
        query,
        dns_answer,
        source_port=source_port,
        peer=list(dns_peer),
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    serve_parser = subparsers.add_parser("serve")
    serve_parser.add_argument("--bind", required=True)
    serve_parser.add_argument("--base", required=True, type=int)
    serve_parser.add_argument("--log", required=True, type=Path)
    serve_parser.set_defaults(function=serve)
    tls_parser = subparsers.add_parser("serve-tls")
    tls_parser.add_argument("--bind", required=True)
    tls_parser.add_argument("--port", required=True, type=int)
    tls_parser.add_argument("--cert", required=True)
    tls_parser.add_argument("--key", required=True)
    tls_parser.add_argument("--log", required=True, type=Path)
    tls_parser.set_defaults(function=serve_tls)
    matrix_parser = subparsers.add_parser("matrix")
    matrix_parser.add_argument("--vip", required=True)
    matrix_parser.add_argument("--base", required=True, type=int)
    matrix_parser.add_argument("--source-port-base", required=True, type=int)
    matrix_parser.add_argument("--repetitions", type=int, default=16)
    matrix_parser.add_argument("--log", required=True, type=Path)
    matrix_parser.set_defaults(function=matrix)
    args = parser.parse_args()
    args.function(args)


if __name__ == "__main__":
    main()
