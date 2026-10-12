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

"""Drive fresh HTTP connections with exact request and phase identifiers."""

import argparse
import base64
import collections
import hashlib
import json
import socket
import time
from pathlib import Path


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--vip", required=True)
parser.add_argument("--port", required=True, type=int)
parser.add_argument("--run", required=True)
parser.add_argument("--phase", required=True)
parser.add_argument("--path", required=True)
parser.add_argument("--source-ip", required=True)
parser.add_argument("--source-port-start", required=True, type=int)
parser.add_argument("--requests", type=int, default=128)
parser.add_argument("--expect-units", required=True)
parser.add_argument("--out", required=True, type=Path)
args = parser.parse_args()
if args.requests < 1 or args.requests > 4096:
    parser.error("requests must be 1..4096")
if args.source_port_start < 1024 or args.source_port_start + args.requests > 65536:
    parser.error("source-port range invalid")
for value in (args.run, args.phase, args.path):
    if any(char in value for char in "\r\n"):
        parser.error("HTTP fields cannot contain newlines")
    value.encode("ascii")

counts = collections.Counter()
with args.out.open("x", encoding="utf-8") as output:
    for index in range(args.requests):
        request_id = f"{args.run}-{args.phase}-{index}"
        separator = "&" if "?" in args.path else "?"
        target = f"{args.path}{separator}probe_id={request_id}"
        request = (
            f"GET {target} HTTP/1.1\r\n"
            "Host: resolution-2286-followup.invalid\r\n"
            "Connection: close\r\n"
            f"X-R2286-Request: {request_id}\r\n"
            f"X-R2286-Phase: {args.phase}\r\n\r\n"
        ).encode("ascii")
        record = {
            "phase": args.phase,
            "request": request_id,
            "target": target,
            "started_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        }
        try:
            with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as connection:
                connection.settimeout(8)
                connection.bind((args.source_ip, args.source_port_start + index))
                connection.connect((args.vip, args.port))
                record["client"] = connection.getsockname()
                record["peer"] = connection.getpeername()
                connection.sendall(request)
                response = bytearray()
                while True:
                    chunk = connection.recv(65536)
                    if not chunk:
                        break
                    response.extend(chunk)
                    if len(response) > 1_048_576:
                        raise RuntimeError("response exceeds 1 MiB evidence cap")
            raw = bytes(response)
            record["response_sha256"] = hashlib.sha256(raw).hexdigest()
            record["response_base64"] = base64.b64encode(raw).decode("ascii")
            headers = raw.partition(b"\r\n\r\n")[0].split(b"\r\n")
            record["status"] = headers[0].decode("latin1") if headers else ""
            units = [
                header.partition(b":")[2].strip().decode("ascii")
                for header in headers
                if header.lower().startswith(b"x-r2286-tmm:")
            ]
            record["reported_units"] = units
            if len(units) == 1:
                counts[units[0]] += 1
        except (OSError, UnicodeDecodeError, OverflowError, RuntimeError) as error:
            record["error"] = str(error)
        output.write(json.dumps(record, sort_keys=True) + "\n")
        output.flush()

expected = set(filter(None, args.expect_units.split(",")))
summary = {
    "actual_response_unit_counts": dict(counts),
    "expected_active_roster": sorted(expected),
    "missing_units": sorted(expected - set(counts)),
    "coverage": (
        "COMPLETE_FOR_SUPPLIED_ROSTER"
        if expected and expected <= set(counts)
        else "INCOMPLETE_OR_ROSTER_UNSPECIFIED"
    ),
}
print(json.dumps(summary, indent=2, sort_keys=True))
