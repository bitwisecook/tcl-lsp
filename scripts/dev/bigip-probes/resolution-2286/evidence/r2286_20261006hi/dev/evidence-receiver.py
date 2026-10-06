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

"""Bounded lab-only HTTP receiver for a single BIG-IP evidence archive."""

import argparse
import hashlib
import json
import re
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--bind", required=True)
parser.add_argument("--port", type=int, required=True)
parser.add_argument("--out-dir", required=True, type=Path)
parser.add_argument("--name", required=True)
args = parser.parse_args()
if not re.fullmatch(r"[A-Za-z0-9_.-]{1,96}", args.name):
    parser.error("invalid archive name")
args.out_dir.mkdir(parents=True, exist_ok=True)
destination = args.out_dir / args.name


class Handler(BaseHTTPRequestHandler):
    def do_PUT(self):
        if self.path != f"/{args.name}" or destination.exists():
            self.send_error(409)
            return
        try:
            length = int(self.headers["Content-Length"])
        except (KeyError, ValueError):
            self.send_error(411)
            return
        if not 1 <= length <= 256 * 1024 * 1024:
            self.send_error(413)
            return
        digest = hashlib.sha256()
        remaining = length
        with destination.open("xb") as output:
            while remaining:
                chunk = self.rfile.read(min(remaining, 1024 * 1024))
                if not chunk:
                    destination.unlink(missing_ok=True)
                    self.send_error(400)
                    return
                output.write(chunk)
                digest.update(chunk)
                remaining -= len(chunk)
        body = (
            json.dumps(
                {"name": args.name, "size": length, "sha256": digest.hexdigest()}
            )
            + "\n"
        ).encode()
        self.send_response(201)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, format, *values):
        print(format % values, flush=True)


HTTPServer((args.bind, args.port), Handler).serve_forever()
