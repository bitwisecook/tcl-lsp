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

"""Decode retained CONTEXT_NAMING_CHECKS.md evidence without rewriting it."""

import argparse
import base64
import collections
import hashlib
import json
import re
import tkinter
from pathlib import Path


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--primary-appliance", required=True, type=Path)
parser.add_argument("--primary-dev", required=True, type=Path)
parser.add_argument("--event-appliance", required=True, type=Path)
parser.add_argument("--event-dev", required=True, type=Path)
parser.add_argument("--event-http-dev", required=True, type=Path)
parser.add_argument("--qualified-appliance", required=True, type=Path)
parser.add_argument("--qualified-dev", required=True, type=Path)
parser.add_argument("--portable-appliance", required=True, type=Path)
parser.add_argument("--portable-dev", required=True, type=Path)
parser.add_argument("--portable-root-dev", required=True, type=Path)
parser.add_argument("--out", required=True, type=Path)
args = parser.parse_args()

interp = tkinter.Tcl()
nul_sentinel = "\ue000"


def split_tcl_bytes(data: bytes) -> list[str]:
    """Split a Tcl list while preserving embedded NUL as U+0000 in JSON."""
    text = data.decode("latin1").replace("\x00", nul_sentinel)
    return [word.replace(nul_sentinel, "\x00") for word in interp.splitlist(text)]


def decoded_result(result_hex: str) -> dict[str, object]:
    data = bytes.fromhex(result_hex)
    return {
        "result_hex": result_hex,
        "result_sha256": hashlib.sha256(data).hexdigest(),
        "result_size": len(data),
        "rows_latin1": split_tcl_bytes(data),
    }


def marker_result(line: str) -> dict[str, object]:
    fields = {}
    for item in line.strip().split("|"):
        if "=" in item:
            key, value = item.split("=", 1)
            fields[key] = value
    result = {
        "rc": fields["rc"],
        "payload_sha256": fields.get("payload_sha256"),
    }
    result.update(decoded_result(fields["result_hex"]))
    return result


def traffic_summary(directory: Path) -> dict[str, object]:
    phases = {}
    for path in sorted(directory.glob("traffic-*.jsonl")):
        units = collections.Counter()
        statuses = collections.Counter()
        bodies = collections.Counter()
        backend_peers = collections.Counter()
        errors = []
        for line_number, line in enumerate(path.read_text().splitlines(), 1):
            record = json.loads(line)
            if "error" in record:
                errors.append({"line": line_number, "error": record["error"]})
                continue
            raw = base64.b64decode(record["response_base64"], validate=True)
            if hashlib.sha256(raw).hexdigest() != record["response_sha256"]:
                raise RuntimeError(f"response digest mismatch: {path}:{line_number}")
            statuses[record["status"]] += 1
            units.update(record["reported_units"])
            body = raw.partition(b"\r\n\r\n")[2]
            bodies[body] += 1
            if path.name == "traffic-baseline.jsonl" or "client-accepted" in path.name:
                backend = json.loads(body)
                backend_peers[backend["client"][0]] += 1
        unique_bodies = []
        for body, count in bodies.items():
            item = {
                "count": count,
                "body_hex": body.hex(),
                "body_sha256": hashlib.sha256(body).hexdigest(),
            }
            if path.name.startswith("traffic-http-"):
                words = split_tcl_bytes(body)
                item["outer_rc"] = words[0]
                item.update(decoded_result(words[1]))
            unique_bodies.append(item)
        phases[path.stem] = {
            "requests": sum(statuses.values()) + len(errors),
            "statuses": dict(statuses),
            "units": dict(sorted(units.items())),
            "transport_errors": errors,
            "backend_peer_ips": dict(backend_peers),
            "unique_bodies": unique_bodies,
        }
    return phases


def non_tmm_results(root: Path) -> dict[str, object]:
    output = {}
    for directory in sorted((root / "contexts").iterdir()):
        contexts = {}
        cli_line = (directory / "cli-run.txt").read_text().strip()
        if cli_line:
            contexts["TmshCliScript"] = marker_result(cli_line)
        for line in (directory / "scriptd-results.txt").read_text().splitlines():
            match = re.search(r"R2286FOLLOWUP\|[^|]+\|([^|]+)\|", line)
            if match:
                contexts[match.group(1)] = marker_result(line)
        groups = collections.defaultdict(list)
        for context, result in contexts.items():
            groups[result["result_sha256"]].append(context)
        output[directory.name] = {
            "contexts": contexts,
            "result_equality_groups": list(groups.values()),
        }
    return output


chunk_pattern = re.compile(
    r"R2286NAMESCHUNK\|[^|]+\|([^|]+)\|([^|]+)\|tmm=([^|]+)"
    r"\|rc=([^|]+)\|part=(\d+)/(\d+)\|result_hex_chunk=([0-9a-f]*)"
)


def chunked_event_results(path: Path) -> dict[str, object]:
    chunks = collections.defaultdict(lambda: collections.defaultdict(collections.Counter))
    part_counts = {}
    for line in path.read_text().splitlines():
        match = chunk_pattern.search(line)
        if not match:
            raise RuntimeError(f"unparsed result chunk: {line}")
        event, case, tmm, rc, part, total, value = match.groups()
        key = (event, case, tmm, rc)
        chunks[key][int(part)][value] += 1
        part_counts.setdefault(key, set()).add(int(total))

    output = {}
    for (event, case, tmm, rc), parts in sorted(chunks.items()):
        totals = part_counts[(event, case, tmm, rc)]
        if len(totals) != 1:
            raise RuntimeError(f"inconsistent chunk totals: {event}/{case}/{tmm}")
        total = next(iter(totals))
        if set(parts) != set(range(total)) or any(len(values) != 1 for values in parts.values()):
            raise RuntimeError(f"incomplete or divergent chunks: {event}/{case}/{tmm}")
        result_hex = "".join(next(iter(parts[index])) for index in range(total))
        observations = sorted(
            {next(iter(parts[index].values())) for index in range(total)}
        )
        item = {"rc": rc, "observations_per_part": observations, "parts": total}
        item.update(decoded_result(result_hex))
        output.setdefault(event, {}).setdefault(case, {})[tmm] = item
    return output


def loads(root: Path) -> dict[str, object]:
    output = {}
    for path in sorted(root.rglob("*load.txt")):
        text = path.read_text(errors="replace")
        rejected = "Syntax Error" in text or "error:" in text.lower()
        output[str(path.relative_to(root))] = {
            "accepted": not rejected,
            "output": text,
        }
    return output


trace_pattern = re.compile(
    r"R2286TRACE\|[^|]+\|([^|]+)\|tmm=([^|]+)\|root_hex=([^|]*)"
    r"\|index_hex=([^|]*)\|op=([^| ]+)"
)


def trace_results(path: Path) -> dict[str, object]:
    counts = collections.Counter()
    for line in path.read_text(errors="replace").splitlines():
        match = trace_pattern.search(line)
        if match:
            counts[match.groups()] += 1
    output = {}
    for (case, tmm, root_hex, index_hex, operation), count in sorted(counts.items()):
        output.setdefault(case, {}).setdefault(tmm, []).append(
            {
                "root_hex": root_hex,
                "index_hex": index_hex,
                "operation": operation,
                "count": count,
            }
        )
    return output


primary_traffic = traffic_summary(args.primary_dev)
event_traffic = traffic_summary(args.event_dev)
event_http_traffic = traffic_summary(args.event_http_dev)
qualified_traffic = traffic_summary(args.qualified_dev)
event_results = chunked_event_results(args.event_appliance / "ltm-result-chunks.txt")

same_run_comparison = {}
for case in ("nul_counted_last", "nul_array_root", "nul_array_index", "lexical_clean"):
    http = event_http_traffic[f"traffic-http-{case}"]["unique_bodies"][0]
    matches = {}
    for event, cases in event_results.items():
        matches[event] = {
            tmm: result["result_hex"] == http["result_hex"]
            for tmm, result in cases[case].items()
        }
    same_run_comparison[case] = {
        "http_result_sha256": http["result_sha256"],
        "event_tmm_exact_result_matches": matches,
    }

lifecycle_root = args.primary_appliance / "icall-lifecycles"
lifecycle = {
    "literal_periodic_load": (lifecycle_root / "periodic-load.txt").read_text(),
    "corrected_periodic": [
        marker_result(line)
        for line in (lifecycle_root / "periodic-corrected-results.txt")
        .read_text()
        .splitlines()
    ],
    "perpetual": [
        marker_result(line)
        for line in (lifecycle_root / "perpetual-results.txt").read_text().splitlines()
    ],
}

output = {
    "version": (args.primary_appliance / "version-final.txt").read_text(),
    "failover": (args.primary_appliance / "failover-final.txt").read_text(),
    "tmm_processes": (args.primary_appliance / "tmm-processes-final.txt").read_text(),
    "primary": {
        "traffic": primary_traffic,
        "non_tmm": non_tmm_results(args.primary_appliance),
        "trace_callbacks": trace_results(args.primary_appliance / "ltm-raw.log"),
        "loads": loads(args.primary_appliance),
        "lifecycle": lifecycle,
        "apl_callback_results": (
            args.primary_appliance / "apl" / "callback-results.txt"
        ).read_text(),
    },
    "chunked_events": event_results,
    "same_run_event_http_comparison": same_run_comparison,
    "event_traffic": event_traffic,
    "qualified_corrected": {
        "traffic": qualified_traffic,
        "non_tmm": non_tmm_results(args.qualified_appliance),
        "loads": loads(args.qualified_appliance),
    },
    "portable_array_controls": {
        "traffic": traffic_summary(args.portable_dev),
        "isolated_root_traffic": traffic_summary(args.portable_root_dev),
        "non_tmm": non_tmm_results(args.portable_appliance),
        "loads": loads(args.portable_appliance),
    },
    "cleanup": {
        "primary": (args.primary_appliance / "cleanup-verification-final.txt").read_text(),
        "events": (args.event_appliance / "cleanup-verification-final.txt").read_text(),
        "qualified": (
            args.qualified_appliance / "cleanup-verification-final.txt"
        ).read_text(),
    },
}
args.out.write_text(json.dumps(output, indent=2, sort_keys=True) + "\n", encoding="utf-8")
