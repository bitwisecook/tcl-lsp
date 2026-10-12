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

"""Verify and decode the counted-input boundary evidence."""

import argparse
import base64
import collections
import hashlib
import json
import re
import tkinter
from pathlib import Path


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--appliance", required=True, type=Path)
parser.add_argument("--traffic", required=True, type=Path)
parser.add_argument("--fixtures", required=True, type=Path)
parser.add_argument("--out", required=True, type=Path)
args = parser.parse_args()

interp = tkinter.Tcl()
nul_sentinel = "\ue000"


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def split_tcl_bytes(data: bytes) -> list[str]:
    text = data.decode("latin1").replace("\x00", nul_sentinel)
    return [word.replace(nul_sentinel, "\x00") for word in interp.splitlist(text)]


def decode_rows(data: bytes) -> dict[str, object]:
    rows = split_tcl_bytes(data)
    return {
        "result_hex": data.hex(),
        "result_sha256": digest(data),
        "result_size": len(data),
        "rows": [split_tcl_bytes(row.encode("latin1")) for row in rows],
    }


def decode_phase(path: Path) -> dict[str, object]:
    units = collections.Counter()
    statuses = collections.Counter()
    bodies = collections.Counter()
    body_units: dict[str, set[str]] = collections.defaultdict(set)
    backend_peers = collections.Counter()
    errors = []
    for line_number, line in enumerate(
        path.read_text(encoding="utf-8").splitlines(), 1
    ):
        record = json.loads(line)
        if "error" in record:
            errors.append({"line": line_number, "error": record["error"]})
            continue
        raw = base64.b64decode(record["response_base64"], validate=True)
        if digest(raw) != record["response_sha256"]:
            raise RuntimeError(f"response digest mismatch: {path}:{line_number}")
        statuses[record["status"]] += 1
        units.update(record["reported_units"])
        body = raw.partition(b"\r\n\r\n")[2]
        bodies[body] += 1
        for unit in record["reported_units"]:
            body_units[digest(body)].add(unit)
        if path.name == "traffic-baseline.jsonl":
            backend = json.loads(body)
            backend_peers[backend["client"][0]] += 1

    body_rows = []
    for body, count in bodies.items():
        item: dict[str, object] = {
            "count": count,
            "body_hex": body.hex(),
            "body_sha256": digest(body),
            "body_size": len(body),
            "observed_units": sorted(body_units[digest(body)]),
        }
        if path.name == "traffic-counted-capture.jsonl":
            item["decoded"] = decode_rows(bytes.fromhex(body.decode("ascii")))
        elif path.name in {
            "traffic-array-root.jsonl",
            "traffic-array-index.jsonl",
        }:
            outer = split_tcl_bytes(body)
            item["outer_rc"] = outer[0]
            item["decoded"] = decode_rows(bytes.fromhex(outer[1]))
        body_rows.append(item)

    return {
        "records": sum(statuses.values()) + len(errors),
        "statuses": dict(statuses),
        "units": dict(sorted(units.items())),
        "transport_errors": errors,
        "backend_peer_ips": dict(backend_peers),
        "unique_bodies": body_rows,
    }


def marker_units(path: Path) -> dict[str, int]:
    counts = collections.Counter()
    pattern = re.compile(r"RESOLUTION2286_COUNTED_INPUT_BOUNDARIES tmm (\d+) rows")
    for line in path.read_text(errors="replace").splitlines():
        match = pattern.search(line)
        if not match:
            raise RuntimeError(f"unparsed counted marker: {path}")
        counts[f"0:{match.group(1)}"] += 1
    return dict(sorted(counts.items()))


fixture_manifest = json.loads((args.fixtures / "manifest.json").read_text())
output = {
    "source_commit": fixture_manifest["source_commit"],
    "run": fixture_manifest["run"],
    "fixture_archive_sha256": (args.appliance / "fixture-archive-sha256.txt")
    .read_text()
    .split()[0],
    "fixture_manifest_sha256": digest((args.fixtures / "manifest.json").read_bytes()),
    "fixture_inventory_sha256": digest((args.fixtures / "SHA256SUMS").read_bytes()),
    "payloads": {
        item["case"]: item
        for item in fixture_manifest["payloads"]
        if item["case"]
        in {
            "counted_input_boundaries",
            "counted_input_capture",
            "array_root_boundaries",
            "array_index_boundaries",
        }
    },
    "wrappers": {
        Path(item["file"]).stem: item
        for item in fixture_manifest["http_wrappers"]
        if Path(item["file"]).stem
        in {
            "counted_input_boundaries",
            "counted_input_capture",
            "array_root_boundaries",
            "array_index_boundaries",
        }
    },
    "traffic": {
        path.stem.removeprefix("traffic-"): decode_phase(path)
        for path in sorted(args.traffic.glob("traffic-*.jsonl"))
    },
    "exact_marker_units": marker_units(args.appliance / "counted-exact-markers.txt"),
    "capture_marker_units": marker_units(
        args.appliance / "counted-capture-markers.txt"
    ),
    "loader_output": {
        path.name: path.read_text(errors="replace")
        for path in sorted(args.appliance.glob("*-load.txt"))
    },
    "cleanup_verification": (args.appliance / "cleanup-verification.txt").read_text(
        errors="replace"
    ),
}
args.out.write_text(json.dumps(output, indent=2, sort_keys=True) + "\n")
