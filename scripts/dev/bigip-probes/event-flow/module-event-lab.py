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

"""Persistent protocol backends and clients for module event-flow probes."""

import argparse
import base64
import hashlib
import http.server
import json
import socket
import ssl
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

    def log_message(self, _format, *args):
        del args

    def do_GET(self):
        self.respond()

    def do_POST(self):
        self.respond()

    def respond(self):
        length = int(self.headers.get("Content-Length", "0"))
        request_body = self.rfile.read(length) if length else b""
        request_id = self.headers.get("X-Evtflow-Request", "none")
        if self.path.startswith("/missing"):
            status, body = 404, b"evtflow-missing\n"
        elif self.path.startswith("/vulnerable"):
            status = 200
            body = ("synthetic-reflection:" + self.path + "\n").encode(
                "utf-8", errors="surrogateescape"
            )
        else:
            status, body = 200, b"evtflow-module-ok\n"
        self.server.recorder.write(
            role="http-backend",
            peer=list(self.client_address),
            local=list(self.server.server_address),
            request=request_id,
            method=self.command,
            path=self.path,
            headers=list(self.headers.items()),
            body=encoded(request_body),
        )
        self.send_response(status)
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Content-Type", "text/plain; charset=utf-8")
        self.send_header("X-Evtflow-Backend", "dev.bitwisecook.org")
        self.end_headers()
        self.wfile.write(body)


def serve_http(bind: str, port: int, recorder: Recorder):
    server = http.server.ThreadingHTTPServer((bind, port), HttpHandler)
    server.recorder = recorder
    server.serve_forever()


def serve_lines(bind: str, port: int, recorder: Recorder, role: str):
    listener = socket.create_server((bind, port))
    while True:
        connection, peer = listener.accept()
        threading.Thread(
            target=handle_lines,
            args=(connection, peer, listener.getsockname(), recorder, role),
            daemon=True,
        ).start()


def handle_lines(connection, peer, local, recorder, role):
    buffer = bytearray()
    with connection:
        while True:
            data = connection.recv(65535)
            if not data:
                break
            buffer.extend(data)
            while b"\n" in buffer:
                line, _, remainder = buffer.partition(b"\n")
                message = line + b"\n"
                buffer[:] = remainder
                recorder.write(
                    role=role,
                    peer=list(peer),
                    local=list(local),
                    payload=encoded(message),
                )
                connection.sendall(b"ECHO:" + message)
        if buffer:
            recorder.write(
                role=role,
                peer=list(peer),
                local=list(local),
                payload=encoded(bytes(buffer)),
            )
            connection.sendall(b"ECHO:" + bytes(buffer))


def serve_tls_lines(
    bind: str, port: int, recorder: Recorder, cert: Path, key: Path
):
    listener = socket.create_server((bind, port))
    context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    context.load_cert_chain(cert, key)
    while True:
        connection, peer = listener.accept()
        try:
            tls_connection = context.wrap_socket(connection, server_side=True)
        except ssl.SSLError as error:
            recorder.write(role="tls-backend-error", peer=list(peer), error=str(error))
            connection.close()
            continue
        threading.Thread(
            target=handle_lines,
            args=(
                tls_connection,
                peer,
                listener.getsockname(),
                recorder,
                "tls-backend",
            ),
            daemon=True,
        ).start()


def read_http_headers(connection) -> bytes:
    data = bytearray()
    while b"\r\n\r\n" not in data and len(data) < 65536:
        chunk = connection.recv(4096)
        if not chunk:
            break
        data.extend(chunk)
    return bytes(data)


def receive_exact(connection, size: int) -> bytes:
    data = bytearray()
    while len(data) < size:
        chunk = connection.recv(size - len(data))
        if not chunk:
            raise EOFError("short WebSocket frame")
        data.extend(chunk)
    return bytes(data)


def receive_websocket_frame(connection) -> tuple[int, bytes]:
    first, second = receive_exact(connection, 2)
    opcode = first & 0x0F
    size = second & 0x7F
    if size == 126:
        size = int.from_bytes(receive_exact(connection, 2), "big")
    elif size == 127:
        size = int.from_bytes(receive_exact(connection, 8), "big")
    mask = receive_exact(connection, 4) if second & 0x80 else b""
    payload = receive_exact(connection, size)
    if mask:
        payload = bytes(value ^ mask[index % 4] for index, value in enumerate(payload))
    return opcode, payload


def websocket_frame(payload: bytes, *, masked: bool) -> bytes:
    first = b"\x81"
    mask_bit = 0x80 if masked else 0
    if len(payload) < 126:
        header = first + bytes([mask_bit | len(payload)])
    elif len(payload) < 65536:
        header = first + bytes([mask_bit | 126]) + len(payload).to_bytes(2, "big")
    else:
        header = first + bytes([mask_bit | 127]) + len(payload).to_bytes(8, "big")
    if not masked:
        return header + payload
    mask = b"\x12\x34\x56\x78"
    masked_payload = bytes(
        value ^ mask[index % 4] for index, value in enumerate(payload)
    )
    return header + mask + masked_payload


def serve_websocket(bind: str, port: int, recorder: Recorder):
    listener = socket.create_server((bind, port))
    while True:
        connection, peer = listener.accept()
        threading.Thread(
            target=handle_websocket,
            args=(connection, peer, listener.getsockname(), recorder),
            daemon=True,
        ).start()


def handle_websocket(connection, peer, local, recorder):
    with connection:
        request = read_http_headers(connection)
        headers = {}
        for line in request.split(b"\r\n")[1:]:
            if b":" in line:
                name, value = line.split(b":", 1)
                headers[name.strip().lower()] = value.strip()
        key = headers.get(b"sec-websocket-key", b"")
        accept = base64.b64encode(
            hashlib.sha1(
                key + b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11",
                usedforsecurity=False,
            ).digest()
        )
        response = (
            b"HTTP/1.1 101 Switching Protocols\r\n"
            b"Upgrade: websocket\r\nConnection: Upgrade\r\n"
            b"Sec-WebSocket-Accept: "
            + accept
            + b"\r\n\r\n"
        )
        connection.sendall(response)
        opcode, payload = receive_websocket_frame(connection)
        recorder.write(
            role="websocket-backend",
            peer=list(peer),
            local=list(local),
            handshake=encoded(request),
            opcode=opcode,
            payload=encoded(payload),
        )
        connection.sendall(websocket_frame(b"WS-ECHO:" + payload, masked=False))


def serve(args):
    recorder = Recorder(args.log)
    threads = [
        threading.Thread(
            target=serve_lines,
            args=(args.bind, args.base, recorder, "tcp-backend"),
            daemon=True,
        ),
        threading.Thread(
            target=serve_tls_lines,
            args=(args.bind, args.base + 1, recorder, args.cert, args.key),
            daemon=True,
        ),
        threading.Thread(
            target=serve_websocket,
            args=(args.bind, args.base + 2, recorder),
            daemon=True,
        ),
        threading.Thread(
            target=serve_lines,
            args=(args.bind, args.base + 7, recorder, "mrf-backend"),
            daemon=True,
        ),
    ]
    for offset in (3, 4, 5, 6, 9, 10, 11):
        threads.append(
            threading.Thread(
                target=serve_http,
                args=(args.bind, args.base + offset, recorder),
                daemon=True,
            )
        )
    for thread in threads:
        thread.start()
    recorder.write(role="server-start", bind=args.bind, base=args.base)
    while True:
        time.sleep(3600)


def receive_all(connection) -> bytes:
    result = bytearray()
    connection.settimeout(3)
    while True:
        try:
            chunk = connection.recv(65535)
        except (ConnectionResetError, TimeoutError, ssl.SSLError):
            break
        if not chunk:
            break
        result.extend(chunk)
    return bytes(result)


def http_request(request_id: str, path: str) -> bytes:
    return (
        f"GET {path} HTTP/1.1\r\n"
        "Host: bigip.bitwisecook.org\r\n"
        f"X-Evtflow-Request: {request_id}\r\n"
        "Connection: close\r\n\r\n"
    ).encode("ascii")


def tls_http_exchange(
    vip: str,
    port: int,
    source_port: int,
    request: bytes,
    ca: Path,
    cert: Path | None,
    key: Path | None,
) -> tuple[bytes, dict]:
    context = ssl.create_default_context(cafile=str(ca))
    context.minimum_version = ssl.TLSVersion.TLSv1_2
    context.maximum_version = ssl.TLSVersion.TLSv1_2
    if cert and key:
        context.load_cert_chain(cert, key)
    connection = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    connection.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    connection.bind(("0.0.0.0", source_port))
    connection.settimeout(8)
    connection.connect((vip, port))
    with context.wrap_socket(
        connection, server_hostname="bigip.bitwisecook.org"
    ) as tls:
        details = {"version": tls.version(), "cipher": tls.cipher()}
        tls.sendall(request)
        response = receive_all(tls)
    return response, details


def tcp_exchange(
    vip: str, port: int, source_port: int, payload: bytes
) -> tuple[bytes, tuple[str, int]]:
    connection = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    connection.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    connection.bind(("0.0.0.0", source_port))
    connection.settimeout(8)
    connection.connect((vip, port))
    connection.sendall(payload)
    response = receive_all(connection)
    peer = connection.getpeername()
    connection.close()
    return response, peer


def tls_line_exchange(
    vip: str, port: int, source_port: int, payload: bytes, ca: Path
) -> tuple[bytes, dict]:
    context = ssl.create_default_context(cafile=str(ca))
    connection = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    connection.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    connection.bind(("0.0.0.0", source_port))
    connection.settimeout(8)
    connection.connect((vip, port))
    with context.wrap_socket(
        connection, server_hostname="bigip.bitwisecook.org"
    ) as tls:
        details = {"version": tls.version(), "cipher": tls.cipher()}
        tls.sendall(payload)
        response = receive_all(tls)
    return response, details


def websocket_exchange(
    vip: str, port: int, source_port: int, payload: bytes
) -> tuple[bytes, bytes]:
    connection = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    connection.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    connection.bind(("0.0.0.0", source_port))
    connection.settimeout(8)
    connection.connect((vip, port))
    key = base64.b64encode(b"evtflow-websocket")
    request = (
        b"GET /websocket HTTP/1.1\r\n"
        b"Host: bigip.bitwisecook.org\r\n"
        b"Upgrade: websocket\r\nConnection: Upgrade\r\n"
        b"Sec-WebSocket-Version: 13\r\nSec-WebSocket-Key: "
        + key
        + b"\r\nX-Evtflow-Request: websocket\r\n\r\n"
    )
    connection.sendall(request)
    handshake = read_http_headers(connection)
    connection.sendall(websocket_frame(payload, masked=True))
    _, response = receive_websocket_frame(connection)
    connection.close()
    return handshake, response


def protocol_matrix(args):
    recorder = Recorder(args.log)
    cases = [
        ("tcp-data-user", args.base, b"tcp-data-user\n"),
        ("mrf-flow-two-messages", args.base + 7, b"mrf-flow-1\nmrf-flow-2\n"),
        (
            "mrf-message-two-messages",
            args.base + 8,
            b"mrf-message-1\nmrf-message-2\n",
        ),
    ]
    for index, (name, port, payload) in enumerate(cases):
        source_port = args.source_port_base + index
        try:
            response, peer = tcp_exchange(
                args.vip, port, source_port, payload
            )
            error = None
        except (OSError, TimeoutError) as caught:
            response, peer, error = b"", None, repr(caught)
        recorder.write(
            role="protocol-client",
            case=name,
            protocol="tcp" if port == args.base else "mrf-generic",
            vip=[args.vip, port],
            peer=list(peer) if peer else None,
            source_port=source_port,
            request=encoded(payload),
            response=encoded(response),
            error=error,
        )

    source_port = args.source_port_base + len(cases)
    tls_payload = b"ssl-client-data\n"
    try:
        response, tls = tls_line_exchange(
            args.vip, args.base + 1, source_port, tls_payload, args.ca
        )
        error = None
    except (OSError, ssl.SSLError, TimeoutError) as caught:
        response, tls, error = b"", None, repr(caught)
    recorder.write(
        role="protocol-client",
        case="ssl-data",
        protocol="tls",
        vip=[args.vip, args.base + 1],
        source_port=source_port,
        request=encoded(tls_payload),
        response=encoded(response),
        tls=tls,
        error=error,
    )

    source_port += 1
    websocket_payload = b"websocket-client-data"
    try:
        handshake, response = websocket_exchange(
            args.vip, args.base + 2, source_port, websocket_payload
        )
        error = None
    except (EOFError, OSError, TimeoutError) as caught:
        handshake, response, error = b"", b"", repr(caught)
    recorder.write(
        role="protocol-client",
        case="websocket-data",
        protocol="websocket",
        vip=[args.vip, args.base + 2],
        source_port=source_port,
        handshake=encoded(handshake),
        request=encoded(websocket_payload),
        response=encoded(response),
        error=error,
    )


def client_cert_matrix(args):
    recorder = Recorder(args.log)
    identities = {
        "none": (None, None),
        "trusted": (args.trusted_cert, args.trusted_key),
        "untrusted": (args.untrusted_cert, args.untrusted_key),
    }
    cases = [
        ("dynamic-public-none", 9, "/public", "none"),
        ("dynamic-request-none", 9, "/cert/request", "none"),
        ("dynamic-request-trusted", 9, "/cert/request", "trusted"),
        ("dynamic-require-trusted", 9, "/cert/require", "trusted"),
        ("dynamic-require-untrusted", 9, "/cert/require", "untrusted"),
        ("dynamic-ignore-trusted", 9, "/cert/ignore", "trusted"),
        ("static-request-none", 10, "/cert/static-request", "none"),
        ("static-request-trusted", 10, "/cert/static-request", "trusted"),
        ("static-request-untrusted", 10, "/cert/static-request", "untrusted"),
        ("static-require-none", 11, "/cert/static-require", "none"),
        ("static-require-trusted", 11, "/cert/static-require", "trusted"),
        ("static-require-untrusted", 11, "/cert/static-require", "untrusted"),
    ]
    for index, (name, offset, path, identity) in enumerate(cases):
        source_port = args.source_port_base + index
        request = http_request(name, path)
        cert, key = identities[identity]
        try:
            response, tls = tls_http_exchange(
                args.vip,
                args.base + offset,
                source_port,
                request,
                args.ca,
                cert,
                key,
            )
            error = None
        except (OSError, ssl.SSLError, TimeoutError) as caught:
            response, tls, error = b"", None, repr(caught)
        recorder.write(
            role="client-cert-client",
            case=name,
            identity=identity,
            path=path,
            vip=[args.vip, args.base + offset],
            source_port=source_port,
            request=encoded(request),
            response=encoded(response),
            tls=tls,
            error=error,
        )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    serve_parser = subparsers.add_parser("serve")
    serve_parser.add_argument("--bind", required=True)
    serve_parser.add_argument("--base", required=True, type=int)
    serve_parser.add_argument("--cert", required=True, type=Path)
    serve_parser.add_argument("--key", required=True, type=Path)
    serve_parser.add_argument("--log", required=True, type=Path)
    serve_parser.set_defaults(function=serve)
    cert_parser = subparsers.add_parser("client-cert-matrix")
    cert_parser.add_argument("--vip", required=True)
    cert_parser.add_argument("--base", required=True, type=int)
    cert_parser.add_argument("--source-port-base", required=True, type=int)
    cert_parser.add_argument("--ca", required=True, type=Path)
    cert_parser.add_argument("--trusted-cert", required=True, type=Path)
    cert_parser.add_argument("--trusted-key", required=True, type=Path)
    cert_parser.add_argument("--untrusted-cert", required=True, type=Path)
    cert_parser.add_argument("--untrusted-key", required=True, type=Path)
    cert_parser.add_argument("--log", required=True, type=Path)
    cert_parser.set_defaults(function=client_cert_matrix)
    protocol_parser = subparsers.add_parser("protocol-matrix")
    protocol_parser.add_argument("--vip", required=True)
    protocol_parser.add_argument("--base", required=True, type=int)
    protocol_parser.add_argument("--source-port-base", required=True, type=int)
    protocol_parser.add_argument("--ca", required=True, type=Path)
    protocol_parser.add_argument("--log", required=True, type=Path)
    protocol_parser.set_defaults(function=protocol_matrix)
    args = parser.parse_args()
    args.function(args)


if __name__ == "__main__":
    main()
