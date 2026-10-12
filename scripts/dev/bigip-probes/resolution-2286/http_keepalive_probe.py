#!/usr/bin/env python3
"""Send one or more HTTP requests on one explicitly bound TCP connection."""

import argparse
import socket
from pathlib import Path


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--host", required=True)
parser.add_argument("--port", required=True, type=int)
parser.add_argument("--local-port", required=True, type=int)
parser.add_argument("--output-prefix", required=True, type=Path)
parser.add_argument("paths", nargs="+")
args = parser.parse_args()


def receive_response(stream):
    status = stream.readline()
    if not status:
        raise EOFError("connection closed before status line")
    headers = []
    lengths = []
    while True:
        line = stream.readline()
        if line in (b"\r\n", b"\n", b""):
            break
        headers.append(line)
        name, _, value = line.partition(b":")
        if name.lower() == b"content-length":
            lengths.append(int(value.strip()))
    if len(lengths) != 1:
        raise ValueError(f"expected one Content-Length, got {lengths}")
    body = stream.read(lengths[0])
    if len(body) != lengths[0]:
        raise EOFError(f"short body: {len(body)} of {lengths[0]}")
    return status + b"".join(headers) + b"\r\n", body


sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
sock.settimeout(120)
sock.bind(("0.0.0.0", args.local_port))
sock.connect((args.host, args.port))
stream = sock.makefile("rb")
for index, path in enumerate(args.paths):
    request = (
        f"GET {path} HTTP/1.1\r\n"
        f"Host: {args.host}:{args.port}\r\n"
        f"X-R2286-Request: lexical-{args.local_port}-{index}\r\n"
        "Connection: keep-alive\r\n\r\n"
    ).encode("ascii")
    sock.sendall(request)
    headers, body = receive_response(stream)
    Path(f"{args.output_prefix}-{index}.request").write_bytes(request)
    Path(f"{args.output_prefix}-{index}.headers").write_bytes(headers)
    Path(f"{args.output_prefix}-{index}.body").write_bytes(body)
sock.close()
