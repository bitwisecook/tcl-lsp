#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Recover measured BIG-IP scripts and results from the retained r2286t archives."""

from __future__ import annotations

import base64
import json
import re
import tarfile
from collections import Counter, defaultdict
from pathlib import Path

from tcl_evidence_format import markdown_block, validate_document

ROOT = Path(__file__).resolve().parents[2]
ARCHIVES = (
    ROOT / "scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006naming"
)
CASES = {
    "counted_input_boundaries": (
        "counted-exact",
        "Do event-local scalar names containing embedded NUL bytes remain distinct from their shorter prefixes across writes, reads and unsets? This determines which variable-name distinctions the LSP, compiler and VM may model in this BIG-IP context.",
        "This is the original event rule. The retained syslog lines stop before the full result list is complete; only the visible prefix of each line was recorded.",
    ),
    "counted_input_capture": (
        "counted-capture",
        "Do event-local scalar names containing embedded NUL bytes remain distinct from their shorter prefixes across writes, reads and unsets? This determines which variable-name distinctions the LSP, compiler and VM may model in this BIG-IP context.",
        "This is a separately executed capture adaptation of the counted-name control. Its HTTP response carries the result that could not be fully retained in the original syslog lines.",
    ),
    "array_root_boundaries": (
        "array-root",
        "Do array-root names with embedded NUL bytes remain distinct from shorter or relocated roots through writes, reads, upvar, unsets and trace callbacks? The result informs how the LSP, compiler and VM distinguish variable roots in this BIG-IP context.",
        "The recorded iRule wrapper constructs the retained payload bytes and evaluates them under catch; the HTTP body contains the wrapper's completion and result.",
    ),
    "array_index_boundaries": (
        "array-index",
        "Do array indices with embedded NUL bytes remain distinct through writes, reads, unsets and trace callbacks, including empty and grammar-sensitive indices? The result informs how the LSP, compiler and VM distinguish array elements in this BIG-IP context.",
        "The recorded iRule wrapper constructs the retained payload bytes and evaluates them under catch; the HTTP body contains the wrapper's completion and result.",
    ),
}


def member(archive: tarfile.TarFile, suffix: str) -> bytes:
    matches = [
        item
        for item in archive.getmembers()
        if item.isfile() and item.name.endswith("/" + suffix)
    ]
    if len(matches) != 1:
        raise ValueError(f"expected one archive member ending in {suffix}")
    stream = archive.extractfile(matches[0])
    if stream is None:
        raise ValueError(f"cannot read {suffix}")
    return stream.read()


def main() -> None:
    with (
        tarfile.open(ARCHIVES / "r2286t-fixtures.tgz") as fixtures,
        tarfile.open(ARCHIVES / "r2286t-appliance-evidence.tgz") as appliance,
        tarfile.open(ARCHIVES / "r2286t-dev-evidence.tgz") as traffic,
    ):
        version_text = member(appliance, "sys-version.txt").decode("ascii")
        version = re.search(r"(?m)^\s*Version\s+(\S+)", version_text)
        build = re.search(r"(?m)^\s*Build\s+(\S+)", version_text)
        if not version or not build:
            raise ValueError("appliance version or build was not retained")
        for source_name, (phase, semantic_question, limitation) in CASES.items():
            script = member(fixtures, f"additional/{source_name}.tcl")
            installed = member(appliance, f"{phase}-config.txt")
            if (
                script not in installed
                and script.hex().encode("ascii") not in installed
            ):
                raise ValueError(
                    f"the installed configuration does not contain {source_name}"
                )
            parts = [
                f"# BIG-IP counted names: {source_name.replace('_', ' ')}\n\n",
                f"## Question\n\n{semantic_question}\n\n",
                (
                    f"## Assumptions\n\n{limitation} The measured context is the "
                    "HTTP_REQUEST event on the recorded BIG-IP appliance. Results from "
                    "other contexts or versions are not implied. Other assumptions were "
                    "not separately recorded.\n\n## Script\n\n"
                ),
                markdown_block(script),
                "\n## Results\n",
            ]
            if phase == "counted-exact":
                rows: dict[str, list[bytes]] = defaultdict(list)
                for line in member(appliance, "counted-exact-markers.txt").splitlines():
                    match = re.search(
                        rb"RESOLUTION2286_COUNTED_INPUT_BOUNDARIES tmm ([0-9]+) rows (.*)$",
                        line,
                    )
                    if not match:
                        raise ValueError("unexpected counted-input log line")
                    rows[match[1].decode("ascii")].append(match[2])
                for unit, values in sorted(rows.items()):
                    counts = Counter(values)
                    if len(counts) != 1:
                        raise ValueError(
                            f"multiple counted-input outputs on TMM {unit}"
                        )
                    value, count = next(iter(counts.items()))
                    parts.extend(
                        [
                            f"\n### TMM unit {unit}, HTTP_REQUEST\n\n",
                            f"- Recorded version: BIG-IP {version[1]} build {build[1]}; Tcl patchlevel Not recorded.\n",
                            "- Recorded input channel: iRule HTTP_REQUEST event.\n",
                            "- Exit status: Not recorded.\n- Standard output: Not recorded.\n- Standard error: Not recorded.\n",
                            "- Recorded result:\n\n",
                            markdown_block(value, "text-visible"),
                            (
                                f"\nThe retained log contains this same incomplete result prefix {count} times for this unit. "
                                "No result is inferred for the truncated remainder.\n"
                            ),
                        ]
                    )
            else:
                rows: dict[str, list[tuple[str, bytes]]] = defaultdict(list)
                for line in member(traffic, f"traffic-{phase}.jsonl").splitlines():
                    record = json.loads(line)
                    if "error" in record or len(record.get("reported_units", [])) != 1:
                        raise ValueError(
                            f"incomplete or ambiguous traffic record for {phase}"
                        )
                    raw = base64.b64decode(record["response_base64"], validate=True)
                    _header, separator, body = raw.partition(b"\r\n\r\n")
                    if not separator:
                        raise ValueError(f"missing HTTP response body for {phase}")
                    rows[record["reported_units"][0]].append(
                        (str(record["status"]), body)
                    )
                for unit, values in sorted(rows.items()):
                    counts = Counter(values)
                    if len(counts) != 1:
                        raise ValueError(
                            f"multiple HTTP results on TMM {unit} for {phase}"
                        )
                    (status, body), count = next(iter(counts.items()))
                    outer = None
                    encoded = body
                    if phase in {"array-root", "array-index"}:
                        outer, encoded = body.split(b" ", 1)
                    decoded = bytes.fromhex(encoded.decode("ascii"))
                    parts.extend(
                        [
                            f"\n### TMM {unit}, HTTP_REQUEST\n\n",
                            f"- Recorded version: BIG-IP {version[1]} build {build[1]}; Tcl patchlevel Not recorded.\n",
                            "- Recorded input channel: iRule HTTP_REQUEST event.\n",
                            "- Exit status: Not recorded.\n- Standard output: Not recorded.\n- Standard error: Not recorded.\n",
                            "- Recorded result:\n\n",
                            markdown_block(body, "text-visible"),
                            f"\nThe retained HTTP response body above was recorded {count} times on this unit with status {status}. "
                            + (
                                f"The wrapper's Tcl completion code was {outer.decode('ascii')}. "
                                if outer is not None
                                else ""
                            )
                            + "The following block decodes its hexadecimal payload.\n\n",
                            markdown_block(decoded, "text-visible"),
                        ]
                    )
            document = "".join(parts).rstrip() + "\n"
            errors = validate_document(document)
            if errors:
                raise ValueError(f"{source_name}: {errors}")
            path = (
                ROOT
                / "evidence/tcl"
                / f"bigip-r2286t-{source_name.replace('_', '-')}.md"
            )
            if path.exists() and path.read_text() != document:
                raise ValueError(f"refusing to overwrite {path}")
            path.write_text(document)
    print(f"Recovered {len(CASES)} measured scripts from retained BIG-IP archives")


if __name__ == "__main__":
    main()
