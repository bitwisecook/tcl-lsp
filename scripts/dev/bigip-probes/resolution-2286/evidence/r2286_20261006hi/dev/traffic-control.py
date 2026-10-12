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

"""Bounded lab-only HTTP trigger for the resolution-2286 traffic driver."""

import argparse
import json
import re
import subprocess
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlsplit


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--bind", required=True)
parser.add_argument("--port", type=int, default=18082)
parser.add_argument("--traffic", required=True, type=Path)
parser.add_argument("--out-dir", required=True, type=Path)
parser.add_argument("--vip", required=True)
parser.add_argument("--vip-port", type=int, default=18086)
parser.add_argument("--source-ip", required=True)
parser.add_argument("--run", required=True)
args = parser.parse_args()
args.out_dir.mkdir(parents=True, exist_ok=True)


class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        query = parse_qs(urlsplit(self.path).query, keep_blank_values=True)
        try:
            label = query["label"][0]
            if not re.fullmatch(r"[A-Za-z0-9_.-]{1,96}", label):
                raise ValueError("invalid label")
            path = query.get("path", ["/?action=read"])[0]
            if not path.startswith("/") or any(c in path for c in "\r\n"):
                raise ValueError("invalid path")
            requests = int(query.get("requests", ["8"])[0])
            source_port = int(query["source_port"][0])
            if not 1 <= requests <= 4096 or not 1024 <= source_port <= 65535 - requests:
                raise ValueError("invalid bounds")
            expected = query.get("expect", [""])[0]
            if expected and not re.fullmatch(r"[0-9:,]+", expected):
                raise ValueError("invalid roster")
            raw = args.out_dir / f"traffic-{label}.jsonl"
            summary = args.out_dir / f"coverage-{label}.json"
            command = [
                "python3",
                str(args.traffic),
                "--vip",
                args.vip,
                "--port",
                str(args.vip_port),
                "--run",
                f"{args.run}-{label}",
                "--source-ip",
                args.source_ip,
                "--source-port-start",
                str(source_port),
                "--requests",
                str(requests),
                "--path",
                path,
                "--out",
                str(raw),
            ]
            if expected:
                command.extend(["--expect-units", expected])
            result = subprocess.run(
                command,
                text=True,
                capture_output=True,
                timeout=requests * 9 + 10,
                check=False,
            )
            summary.write_text(result.stdout, encoding="utf-8")
            record = {
                "label": label,
                "command": command,
                "returncode": result.returncode,
                "stdout": result.stdout,
                "stderr": result.stderr,
            }
            (args.out_dir / f"trigger-{label}.json").write_text(
                json.dumps(record, indent=2) + "\n", encoding="utf-8"
            )
            body = (json.dumps(record, sort_keys=True) + "\n").encode()
            self.send_response(200 if result.returncode == 0 else 500)
        except (KeyError, ValueError) as error:
            body = (json.dumps({"error": str(error)}) + "\n").encode()
            self.send_response(400)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, format, *values):
        print(format % values, flush=True)


ThreadingHTTPServer((args.bind, args.port), Handler).serve_forever()
