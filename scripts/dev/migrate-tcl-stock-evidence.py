#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Recover individual stock-Tcl captures from recorded BIG-IP SSH runs."""

from __future__ import annotations

import json
import runpy
from collections import defaultdict
from pathlib import Path

from tcl_evidence_format import markdown_block, validate_document

ROOT = Path(__file__).resolve().parents[2]
QUESTIONS = runpy.run_path(str(ROOT / "scripts/dev/migrate-tcl-apl-evidence.py"))[
    "PATTERN_QUESTIONS"
]
RUNS = ("nh20261012stock2", "nh20261012pat1", "nh20261012stock3", "nh20261012pat3")


def decoded_rows(stdout: bytes, identity: str, context: str) -> str:
    """Keep full stdout and additionally decode complete NHO result payloads."""
    groups: dict[tuple[bytes, ...], list[tuple[int, int, bytes]]] = defaultdict(list)
    for line in stdout.splitlines():
        if not line.startswith(b"NHO|"):
            continue
        fields = line.split(b"|")
        if len(fields) != 13 or fields[2:4] != [identity.encode(), context.encode()]:
            raise ValueError(f"unexpected result identity in {identity}")
        groups[tuple(fields[3:10])].append(
            (int(fields[10]), int(fields[11]), bytes.fromhex(fields[12].decode()))
        )
    out: list[str] = []
    for key, parts in groups.items():
        parts.sort()
        if [part[0] for part in parts] != list(range(len(parts))) or any(
            part[1] != len(parts) for part in parts
        ):
            raise ValueError(f"incomplete result chunks in {identity}")
        _, phase, completion, unit, invocation, field, encoding = (
            item.decode("ascii") for item in key
        )
        out.append(
            f"\n- {phase}, completion {completion}, {field} "
            f"({encoding}; unit {unit}, invocation {invocation}):\n\n"
            + markdown_block(b"".join(part[2] for part in parts), "text-visible")
        )
    return "".join(out)


def main() -> None:
    output = ROOT / "evidence/tcl"
    output.mkdir(parents=True, exist_ok=True)
    written = 0
    for name in RUNS:
        run = ROOT / "experiments/name-handling/patterns/f5/answers" / name
        records = json.loads((run / "records.json").read_bytes())
        for version in ("8.4", "8.5"):
            context = f"stock-{version}"
            by_case = {
                row["case_id"]: row
                for row in records
                if row["provider"]["context"] == context
            }
            cases = json.loads((run / f"{context}-cases.json").read_bytes())
            for index, case in enumerate(cases):
                identity = case["id"]
                if identity not in QUESTIONS:
                    continue
                record = by_case[identity]
                directory = run / f"{context}-{index}"
                script_path = directory / "input.tcl"
                invocation_path = directory / "invocation.json"
                if record.get("actual_input", {}).get("path") != str(
                    script_path.relative_to(ROOT)
                ) or not any(
                    item.get("path") == str(invocation_path.relative_to(ROOT))
                    for item in record.get("invocation_evidence", [])
                ):
                    raise ValueError(
                        f"script or invocation does not match {name}/{context}/{identity}"
                    )
                invocation = json.loads(invocation_path.read_bytes())
                argv = invocation.get("argv", [])
                provider = record["provider"]
                if (
                    not argv
                    or argv[0] != "ssh"
                    or argv[-1] != provider.get("interpreter_path")
                ):
                    raise ValueError(
                        f"unexpected transport for {name}/{context}/{identity}"
                    )
                status = invocation.get("exit_code")
                status = str(status) if type(status) is int else "Not recorded"
                script = script_path.read_bytes()
                stdout = (directory / "runtime.stdout").read_bytes()
                stderr = (directory / "runtime.stderr").read_bytes()
                patchlevel = provider.get("interpreter_patchlevel") or "Not recorded"
                appliance = provider.get("version") or "Not recorded"
                scope = str(case.get("scope") or "Not recorded").replace("\n", " ")
                reached = record.get("intended_lookup_reached")
                note = (
                    "The intended operation was not reached; this capture records the earlier outcome.\n\n"
                    if reached is False
                    else ""
                )
                result = [
                    f"# BIG-IP stock Tcl: {identity}\n\n",
                    f"## Question\n\n{QUESTIONS[identity]}\n\n",
                    ("This case informs name resolution and diagnostics in the LSP, compiler and VM; "
                    "it does not establish behavior in other environments.\n\n"),
                    f"## Assumptions\n\nRecorded case scope: {scope}\n\n",
                    note,
                    ("Other assumptions were not separately recorded. Connection identifiers were "
                    "redacted in the retained invocation.\n\n"),
                    "## Script\n\n",
                    markdown_block(script),
                    "\n## Results\n\n",
                    f"### BIG-IP {context}\n\n",
                    f"- Recorded version: BIG-IP {appliance}; Tcl patchlevel {patchlevel}.\n",
                    f"- Recorded input channel: SSH to {provider['interpreter_path']}, script on standard input.\n",
                    f"- Exit status: {status}\n",
                ]
                for field, data in (
                    ("Standard output", stdout),
                    ("Standard error", stderr),
                ):
                    if data:
                        result.extend(
                            [
                                f"- {field}:\n\n",
                                markdown_block(data, "text-visible"),
                                "\n",
                            ]
                        )
                    else:
                        result.append(f"- {field}: empty.\n")
                result.extend(
                    [
                        ("\nThe exit status and streams are from the SSH invocation; standard error "
                        "can include SSH transport messages as well as remote errors. The entries "
                        "below decode complete NHO result payloads from standard output.\n"),
                        decoded_rows(stdout, identity, context),
                    ]
                )
                document = "".join(result).rstrip() + "\n"
                errors = validate_document(document)
                if errors:
                    raise ValueError(f"{identity}: {errors}")
                path = output / f"bigip-{name}-{context}-{identity}.md"
                if path.exists() and path.read_text() != document:
                    raise ValueError(f"refusing to overwrite {path}")
                path.write_text(document)
                written += 1
    print(f"Recovered {written} individual stock Tcl captures")


if __name__ == "__main__":
    main()
