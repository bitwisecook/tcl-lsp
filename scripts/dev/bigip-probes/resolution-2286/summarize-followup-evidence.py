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

"""Summarize retained follow-up HTTP responses without discarding raw bytes."""

import argparse
import base64
import collections
import hashlib
import json
import tkinter
from pathlib import Path


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--traffic-dir", required=True, type=Path)
parser.add_argument("--out", required=True, type=Path)
args = parser.parse_args()

interp = tkinter.Tcl()
output = {"phases": []}
for path in sorted(args.traffic_dir.glob("*.jsonl")):
    bodies = collections.Counter()
    units = collections.Counter()
    statuses = collections.Counter()
    errors = []
    backend_client_ips = collections.Counter()
    request_count = 0
    for line_number, line in enumerate(
        path.read_text(encoding="utf-8").splitlines(), 1
    ):
        request_count += 1
        record = json.loads(line)
        if "error" in record:
            errors.append({"line": line_number, "error": record["error"]})
            continue
        raw = base64.b64decode(record["response_base64"], validate=True)
        if hashlib.sha256(raw).hexdigest() != record["response_sha256"]:
            raise RuntimeError(f"response digest mismatch: {path}:{line_number}")
        statuses[record["status"]] += 1
        for unit in record["reported_units"]:
            units[unit] += 1
        body = raw.partition(b"\r\n\r\n")[2]
        bodies[body] += 1
        if path.stem == "backend-initial":
            backend = json.loads(body)
            backend_client_ips[backend["client"][0]] += 1

    results = []
    for body, count in sorted(bodies.items(), key=lambda item: item[0]):
        result = {
            "count": count,
            "body_sha256": hashlib.sha256(body).hexdigest(),
            "body_hex": body.hex(),
        }
        if path.stem != "backend-initial":
            words = interp.splitlist(body.decode("ascii"))
            result["tcl_words"] = list(words)
            decoded_hex_words = []
            for word in words[1:]:
                data = bytes.fromhex(word)
                decoded = {
                    "hex": data.hex(),
                    "latin1": data.decode("latin1"),
                }
                try:
                    decoded["tcl_list"] = list(interp.splitlist(data.decode("latin1")))
                except (TypeError, tkinter.TclError) as error:
                    decoded["tcl_list_error"] = str(error)
                decoded_hex_words.append(decoded)
            result["decoded_hex_words"] = decoded_hex_words
        results.append(result)

    output["phases"].append(
        {
            "phase": path.stem,
            "source": path.name,
            "request_count": request_count,
            "transport_errors": errors,
            "status_counts": dict(statuses),
            "unit_counts": dict(sorted(units.items())),
            "unique_response_bodies": results,
            "backend_client_ip_counts": dict(sorted(backend_client_ips.items())),
        }
    )

args.out.write_text(
    json.dumps(output, indent=2, sort_keys=True) + "\n", encoding="utf-8"
)
