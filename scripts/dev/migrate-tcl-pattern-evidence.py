#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Recover stock Tcl and Jim naming-pattern execution captures."""

from __future__ import annotations

import json
import re
import runpy
from pathlib import Path

from tcl_evidence_format import markdown_block, validate_document

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "experiments/name-handling/patterns"
QUESTIONS = runpy.run_path(str(ROOT / "scripts/dev/migrate-tcl-apl-evidence.py"))[
    "PATTERN_QUESTIONS"
]


def decoded_fields(data: bytes) -> str:
    parts: list[str] = []
    for line in data.splitlines():
        match = re.fullmatch(rb"FIELD ([a-z_]+) (\{\}|[0-9a-fA-F]*)", line)
        if not match:
            continue
        value = b"" if match[2] == b"{}" else bytes.fromhex(match[2].decode("ascii"))
        parts.extend(
            [
                f"\n- {match[1].decode('ascii')}:\n\n",
                markdown_block(value, "text-visible"),
            ]
        )
    return "".join(parts)


def write_capture(
    identity: str,
    script: bytes,
    captures: list[tuple[str, dict, bytes, bytes]],
    suffix: str,
) -> None:
    helper = (
        "The Jim wrapper also sources the test helper named in the script; "
        "the result depends on that helper. "
        if suffix == "jim"
        else ""
    )
    parts = [
        f"# Tcl naming pattern: {identity}\n\n",
        f"## Question\n\n{QUESTIONS[identity]}\n\n",
        (
            "This distinction informs command and variable resolution in the LSP, "
            "compiler and VM. The observations cover only this case and the stated environments.\n\n"
        ),
        "## Assumptions\n\nThe executed wrapper separates setup from the final operation. "
        "A setup failure leaves that operation unreached. Each capture uses a fresh "
        "process with the recorded interpreter; LC_ALL was C. "
        + helper
        + "Other assumptions were "
        "not separately recorded.\n\n## Script\n\n",
        markdown_block(script),
        "\n## Results\n",
    ]
    for version, record, stdout, stderr in captures:
        status = record.get("process_exit_code")
        status = str(status) if type(status) is int else "Not recorded"
        parts.extend(
            [
                f"\n### {version}\n\n",
                f"- Recorded version: {version}\n",
                "- Recorded input channel: script supplied to the interpreter.\n",
                f"- Exit status: {status}\n",
            ]
        )
        for title, data in (("Standard output", stdout), ("Standard error", stderr)):
            if data:
                parts.extend(
                    [f"- {title}:\n\n", markdown_block(data, "text-visible"), "\n"]
                )
            else:
                parts.append(f"- {title}: empty.\n")
        code = record.get("lookup_code", record.get("question_code"))
        if code == "not-run":
            parts.append("\nThe intended final operation was not reached.\n")
        fields = decoded_fields(stdout)
        if fields:
            parts.extend(
                [
                    "\nThe following entries decode the FIELD payloads in standard output.\n",
                    fields,
                ]
            )
    document = "".join(parts).rstrip() + "\n"
    errors = validate_document(document)
    if errors:
        raise ValueError(f"{identity}: {errors}")
    path = ROOT / "evidence/tcl" / f"naming-pattern-{identity}-{suffix}.md"
    if path.exists() and path.read_text() != document:
        raise ValueError(f"refusing to overwrite {path}")
    path.write_text(document)


def main() -> None:
    versions = ("8.4.20", "8.5.19", "8.6.18", "9.0.4", "9.1.0")
    directory = BASE / "results"
    identities = sorted(
        path.name
        for path in (directory / f"tcl{versions[0]}").iterdir()
        if path.is_dir()
    )
    for identity in identities:
        if identity not in QUESTIONS:
            raise ValueError(f"unknown question: {identity}")
        scripts = [
            (directory / f"tcl{version}" / identity / "input.tcl").read_bytes()
            for version in versions
        ]
        if any(script != scripts[0] for script in scripts[1:]):
            raise ValueError(f"different scripts recorded for {identity}")
        captures = []
        for version in versions:
            capture = directory / f"tcl{version}" / identity
            record = json.loads((capture / "record.json").read_bytes())
            if record.get("case_id") != identity or record.get("patchlevel") != version:
                raise ValueError(f"mismatched Tcl record for {identity}/{version}")
            captures.append(
                (
                    f"Tcl {version}",
                    record,
                    (capture / "stdout.txt").read_bytes(),
                    (capture / "stderr.txt").read_bytes(),
                )
            )
        write_capture(identity, scripts[0], captures, "tcl")
        capture = BASE / "jim-results-5bac7c9/corpus" / identity
        record = json.loads((capture / "record.json").read_bytes())
        if record.get("case_id") != identity:
            raise ValueError(f"mismatched Jim record for {identity}")
        write_capture(
            identity,
            (capture / "input.tcl").read_bytes(),
            [
                (
                    "Jim 0.84-9-g5bac7c9",
                    record,
                    (capture / "stdout.txt").read_bytes(),
                    (capture / "stderr.txt").read_bytes(),
                )
            ],
            "jim",
        )
    print(f"Recovered {len(identities) * 2} scripts across stock Tcl and Jim")


if __name__ == "__main__":
    main()
