#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Recover the r2286q multi-event BIG-IP Tcl payload observations."""

from __future__ import annotations

import base64
import json
import re
import runpy
import tarfile
import tkinter
from collections import Counter, defaultdict
from pathlib import Path

from tcl_evidence_format import markdown_block, validate_document

ROOT = Path(__file__).resolve().parents[2]
helpers = runpy.run_path(
    str(ROOT / "scripts/dev/migrate-tcl-context-archive-evidence.py")
)
QUESTIONS = helpers["QUESTIONS"]
read_member = helpers["read_member"]
ARCHIVES = (
    ROOT / "scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006naming"
)
CASES = ("nul_counted_last", "nul_array_root", "nul_array_index", "lexical_clean")
CHUNK = re.compile(
    rb"R2286NAMESCHUNK\|[^|]+\|([^|]+)\|([^|]+)\|tmm=([^|]+)"
    rb"\|rc=([^|]+)\|part=([0-9]+)/([0-9]+)\|result_hex_chunk=([0-9a-f]*)"
)


def main() -> None:
    with (
        tarfile.open(ARCHIVES / "r2286q-fixtures.tgz") as fixtures,
        tarfile.open(ARCHIVES / "r2286q-appliance-evidence.tgz") as appliance,
        tarfile.open(ARCHIVES / "r2286q-http-dev-evidence.tgz") as traffic,
    ):
        fixture_root = "r2286q-fixtures"
        raw_root = "r2286q-evidence/raw"
        traffic_root = "r2286q-http-dev-evidence"
        manifest = json.loads(read_member(fixtures, f"{fixture_root}/manifest.json"))
        sources = {row["case"]: row["file"] for row in manifest["payloads"]}
        version_text = read_member(appliance, f"{raw_root}/version.txt").decode("ascii")
        version = re.search(r"(?m)^\s*Version\s+(\S+)", version_text)
        build = re.search(r"(?m)^\s*Build\s+(\S+)", version_text)
        if not version or not build:
            raise ValueError("appliance version or build was not recorded")
        chunks: dict[tuple[str, str, str, str], dict[int, Counter[bytes]]] = (
            defaultdict(lambda: defaultdict(Counter))
        )
        totals: dict[tuple[str, str, str, str], set[int]] = defaultdict(set)
        for line in read_member(
            appliance, f"{raw_root}/ltm-result-chunks.txt"
        ).splitlines():
            match = CHUNK.search(line)
            if not match:
                raise ValueError("unparsed event result chunk")
            event, case, unit, code = (match[i].decode("ascii") for i in range(1, 5))
            part, total = int(match[5]), int(match[6])
            key = (event, case, unit, code)
            chunks[key][part][match[7]] += 1
            totals[key].add(total)
        tcl = tkinter.Tcl()
        for case in CASES:
            script = read_member(fixtures, f"{fixture_root}/{sources[case]}")
            for event in ("client_accepted", "rule_init"):
                wrapper = read_member(
                    fixtures, f"{fixture_root}/events/{event}-{case}.conf"
                )
                if script.hex().encode("ascii") not in wrapper:
                    raise ValueError(f"payload missing from {event}/{case} wrapper")
            http_wrapper = read_member(
                fixtures, f"{fixture_root}/{sources[case][:-4]}.conf"
            )
            if script.hex().encode("ascii") not in http_wrapper:
                raise ValueError(f"payload missing from HTTP wrapper for {case}")
            parts = [
                f"# BIG-IP multi-event naming: {case.replace('_', ' ')}\n\n",
                f"## Question\n\n{QUESTIONS[case]}\n\n",
                (
                    "## Assumptions\n\nThe payload was evaluated under catch inside "
                    "event-specific wrappers. Event results were reassembled only "
                    "when all parts were present and unambiguous. Results from one "
                    "event, unit or version do not establish another. Other "
                    "assumptions were not separately recorded.\n\n## Script\n\n"
                ),
                markdown_block(script),
                "\n## Results\n",
            ]
            for (event, captured_case, unit, code), values in sorted(chunks.items()):
                if captured_case != case:
                    continue
                declared = totals[(event, captured_case, unit, code)]
                if len(declared) != 1:
                    raise ValueError(
                        f"inconsistent result chunk totals for {event}/{case}/{unit}"
                    )
                total = next(iter(declared))
                if set(values) != set(range(total)) or any(
                    len(choices) != 1 for choices in values.values()
                ):
                    raise ValueError(
                        f"incomplete or conflicting chunks for {event}/{case}/{unit}"
                    )
                encoded = b"".join(next(iter(values[index])) for index in range(total))
                result = bytes.fromhex(encoded.decode("ascii"))
                counts = [sum(values[index].values()) for index in range(total)]
                parts.extend(
                    [
                        f"\n### TMM {unit}, {event}\n\n",
                        f"- Recorded version: BIG-IP {version[1]} build {build[1]}; Tcl patchlevel Not recorded.\n",
                        f"- Recorded input channel: iRule {event} event.\n",
                        "- Exit status: Not recorded.\n- Standard output: Not recorded.\n- Standard error: Not recorded.\n",
                        "- Recorded result:\n\n",
                        markdown_block(result, "text-visible"),
                        (
                            f"\nTcl completion code: {code}. This result was reconstructed from {total} log parts; "
                            f"the retained log contained these numbers of copies per part: {counts}.\n"
                        ),
                    ]
                )
            by_unit: dict[str, list[tuple[str, str, bytes]]] = defaultdict(list)
            for line in read_member(
                traffic, f"{traffic_root}/traffic-http-{case}.jsonl"
            ).splitlines():
                record = json.loads(line)
                if "error" in record or len(record.get("reported_units", [])) != 1:
                    raise ValueError(f"incomplete HTTP record for {case}")
                raw = base64.b64decode(record["response_base64"], validate=True)
                _, separator, body = raw.partition(b"\r\n\r\n")
                if not separator:
                    raise ValueError(f"missing HTTP body for {case}")
                words = tcl.splitlist(body.decode("ascii"))
                if len(words) != 2:
                    raise ValueError(f"unexpected HTTP body for {case}")
                by_unit[record["reported_units"][0]].append(
                    (str(record["status"]), words[0], bytes.fromhex(words[1]))
                )
            for unit, values in sorted(by_unit.items()):
                counts = Counter(values)
                if len(counts) != 1:
                    raise ValueError(f"multiple HTTP results for {case}/{unit}")
                (status, code, result), count = next(iter(counts.items()))
                parts.extend(
                    [
                        f"\n### TMM {unit}, HTTP_REQUEST\n\n",
                        "- Recorded version: Not separately recorded in this HTTP capture.\n",
                        "- Recorded input channel: iRule HTTP_REQUEST event.\n",
                        "- Exit status: Not recorded.\n- Standard output: Not recorded.\n- Standard error: Not recorded.\n",
                        "- Recorded result:\n\n",
                        markdown_block(result, "text-visible"),
                        (
                            f"\nThe wrapper recorded Tcl completion code {code}; the same result was recorded "
                            f"in {count} HTTP responses with status {status}.\n"
                        ),
                    ]
                )
            document = "".join(parts).rstrip() + "\n"
            errors = validate_document(document)
            if errors:
                raise ValueError(f"{case}: {errors}")
            path = ROOT / "evidence/tcl" / f"bigip-r2286q-{case.replace('_', '-')}.md"
            if path.exists() and path.read_text() != document:
                raise ValueError(f"refusing to overwrite {path}")
            path.write_text(document)
    print(f"Recovered {len(CASES)} multi-event BIG-IP payloads")


if __name__ == "__main__":
    main()
