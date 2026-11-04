#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Recover the r2286r and r2286s corrected BIG-IP naming observations."""

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
read_member = helpers["read_member"]
emit_result = helpers["emit_result"]
QUESTIONS = {
    "qualified_nul": helpers["QUESTIONS"]["qualified_nul"],
    "nul_array_root_portable": "Does an embedded NUL distinguish an array root from its shorter prefix through writes, reads, links, unsets and supported trace callbacks across BIG-IP Tcl contexts? This affects array-root modeling in the LSP, compiler and VM.",
    "nul_array_index_portable": "Does an embedded NUL distinguish an array index from its shorter prefix through writes, reads, links, unsets and supported trace callbacks across BIG-IP Tcl contexts? This affects array-element modeling in the LSP, compiler and VM.",
}
ARCHIVES = (
    ROOT / "scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006naming"
)
CASES = {
    "r": ("qualified_nul",),
    "s": ("nul_array_root_portable", "nul_array_index_portable"),
}


def main() -> None:
    tcl = tkinter.Tcl()
    written = 0
    for run, cases in CASES.items():
        with (
            tarfile.open(ARCHIVES / f"r2286{run}-fixtures.tgz") as fixtures,
            tarfile.open(ARCHIVES / f"r2286{run}-appliance-evidence.tgz") as appliance,
            tarfile.open(ARCHIVES / f"r2286{run}-dev-evidence.tgz") as traffic,
            tarfile.open(ARCHIVES / "r2286s-root-dev-evidence.tgz") as root_traffic,
            tarfile.open(
                ARCHIVES / "r2286s-root-appliance-evidence.tgz"
            ) as root_appliance,
        ):
            fixture_root = f"r2286{run}-fixtures"
            raw_root = f"r2286{run}-evidence/raw"
            traffic_root = f"r2286{run}-dev-evidence"
            manifest = json.loads(
                read_member(fixtures, f"{fixture_root}/manifest.json")
            )
            sources = {row["case"]: row["file"] for row in manifest["payloads"]}
            context_runs = {
                row["case"]: row["context_run"] for row in manifest["contexts"]
            }
            version_text = read_member(appliance, f"{raw_root}/version.txt").decode(
                "ascii"
            )
            version = re.search(r"(?m)^\s*Version\s+(\S+)", version_text)
            build = re.search(r"(?m)^\s*Build\s+(\S+)", version_text)
            if not version or not build:
                raise ValueError("appliance version or build was not recorded")
            label_version = f"{version[1]} build {build[1]}"
            for case in cases:
                script = read_member(fixtures, f"{fixture_root}/{sources[case]}")
                if script != read_member(
                    fixtures, f"{fixture_root}/contexts/{case}/payload.tcl"
                ):
                    raise ValueError(f"context payload differs for {case}")
                for wrapper in ("cli.conf", "iapp.conf", "icall.conf"):
                    data = read_member(
                        fixtures, f"{fixture_root}/contexts/{case}/{wrapper}"
                    )
                    if script.hex().encode("ascii") not in data:
                        raise ValueError(f"payload missing from {case}/{wrapper}")
                wrapper = read_member(
                    fixtures, f"{fixture_root}/{sources[case][:-4]}.conf"
                )
                if script.hex().encode("ascii") not in wrapper:
                    raise ValueError(f"payload missing from HTTP wrapper for {case}")
                parts = [
                    f"# BIG-IP corrected naming: {case.replace('_', ' ')}\n\n",
                    f"## Question\n\n{QUESTIONS[case]}\n\n",
                    (
                        "## Assumptions\n\nThe retained payload was evaluated under catch "
                        "inside context-specific wrappers. The result is scoped to "
                        "the named context and inputs; it does not establish behavior "
                        "in another context or version. Other assumptions were not "
                        "separately recorded.\n\n## Script\n\n"
                    ),
                    markdown_block(script),
                    "\n## Results\n",
                ]
                phase = case.replace("_", "-")
                runs = [
                    (
                        traffic,
                        traffic_root,
                        f"traffic-http-{phase}.jsonl",
                        "initial",
                        label_version,
                    )
                ]
                if case == "nul_array_index_portable":
                    runs.append(
                        (
                            traffic,
                            traffic_root,
                            f"traffic-http-{phase}-rerun.jsonl",
                            "rerun",
                            label_version,
                        )
                    )
                if case == "nul_array_root_portable":
                    load = read_member(
                        root_appliance, "r2286s-root-evidence/raw/root-load.txt"
                    )
                    if (
                        b"/r2286s-fixtures/additional/nul_array_root_portable.conf"
                        not in load
                    ):
                        raise ValueError(
                            "root rerun fixture was not identified in the load record"
                        )
                    parts.append(
                        "\nThe initial HTTP collection labeled for this root test returned "
                        "the array-index test's result, so it cannot establish a result for "
                        "this script. The separately loaded root rerun is recorded below.\n"
                    )
                    runs = [
                        (
                            root_traffic,
                            "r2286s-root-dev-evidence",
                            f"traffic-http-{phase}-rerun.jsonl",
                            "rerun",
                            "Not recorded",
                        )
                    ]
                for archive, prefix, filename, run_label, run_version in runs:
                    by_unit: dict[str, list[tuple[str, str, bytes]]] = defaultdict(list)
                    transport_errors: Counter[str] = Counter()
                    for line in read_member(
                        archive, f"{prefix}/{filename}"
                    ).splitlines():
                        record = json.loads(line)
                        if "error" in record:
                            transport_errors[str(record["error"])] += 1
                            continue
                        if len(record.get("reported_units", [])) != 1:
                            raise ValueError(
                                f"incomplete HTTP record for {case}/{run_label}"
                            )
                        raw = base64.b64decode(record["response_base64"], validate=True)
                        _, separator, body = raw.partition(b"\r\n\r\n")
                        if not separator:
                            raise ValueError(
                                f"missing HTTP body for {case}/{run_label}"
                            )
                        words = tcl.splitlist(body.decode("ascii"))
                        if len(words) != 2:
                            raise ValueError(
                                f"unexpected HTTP body for {case}/{run_label}"
                            )
                        result = bytes.fromhex(words[1])
                        expected_prefix = {
                            "nul_array_root_portable": b"{roots ",
                            "nul_array_index_portable": b"{indexes ",
                        }.get(case)
                        if expected_prefix and not result.startswith(expected_prefix):
                            raise ValueError(
                                f"unexpected script result for {case}/{run_label}"
                            )
                        by_unit[record["reported_units"][0]].append(
                            (str(record["status"]), words[0], result)
                        )
                    for error, count in sorted(transport_errors.items()):
                        parts.extend(
                            [
                                (
                                    f"\nThe {run_label} HTTP collection had {count} transport failures "
                                    "with no recorded Tcl result or TMM unit:\n\n"
                                ),
                                markdown_block(error.encode("utf-8"), "text-visible"),
                            ]
                        )
                    for unit, values in sorted(by_unit.items()):
                        counts = Counter(values)
                        if len(counts) != 1:
                            raise ValueError(
                                f"multiple HTTP results for {case}/{run_label}/{unit}"
                            )
                        (status, code, result), count = next(iter(counts.items()))
                        emit_result(
                            parts,
                            f"TMM {unit}, HTTP_REQUEST, {run_label}",
                            run_version,
                            code,
                            result,
                            f"The same result was recorded in {count} responses with status {status}.",
                        )
                contexts: dict[str, tuple[str, bytes]] = {}
                for name in ("cli-run.txt", "scriptd-results.txt"):
                    data = read_member(appliance, f"{raw_root}/contexts/{case}/{name}")
                    for line in data.splitlines():
                        offset = line.find(b"R2286FOLLOWUP|")
                        if offset < 0:
                            continue
                        fields = line[offset:].decode("ascii").split("|")
                        if len(fields) < 4 or fields[1] != context_runs[case]:
                            raise ValueError(f"ambiguous context marker for {case}")
                        values = dict(
                            item.split("=", 1) for item in fields[3:] if "=" in item
                        )
                        if fields[2] in contexts:
                            raise ValueError(f"duplicate {fields[2]} marker for {case}")
                        contexts[fields[2]] = (
                            values["rc"],
                            bytes.fromhex(values["result_hex"]),
                        )
                if set(contexts) != {
                    "TmshCliScript",
                    "IAppImplementation",
                    "ICallScript",
                }:
                    raise ValueError(f"missing context marker for {case}")
                for context, (code, result) in sorted(contexts.items()):
                    emit_result(
                        parts,
                        context,
                        label_version,
                        code,
                        result,
                        "This result was decoded from the retained context marker.",
                    )
                document = "".join(parts).rstrip() + "\n"
                errors = validate_document(document)
                if errors:
                    raise ValueError(f"{case}: {errors}")
                path = (
                    ROOT
                    / "evidence/tcl"
                    / f"bigip-r2286{run}-{case.replace('_', '-')}.md"
                )
                if path.exists() and path.read_text() != document:
                    raise ValueError(f"refusing to overwrite {path}")
                path.write_text(document)
                written += 1
    print(f"Recovered {written} corrected cross-context BIG-IP payloads")


if __name__ == "__main__":
    main()
