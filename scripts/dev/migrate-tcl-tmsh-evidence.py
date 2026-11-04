#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Recover executed tmsh scripts and their recorded command streams."""

from __future__ import annotations

import json
import re
import runpy
from pathlib import Path

from tcl_evidence_format import markdown_block, validate_document

ROOT = Path(__file__).resolve().parents[2]
helpers = runpy.run_path(str(ROOT / "scripts/dev/migrate-tcl-stock-evidence.py"))
QUESTIONS = helpers["QUESTIONS"]
decoded_rows = helpers["decoded_rows"]
REQUEST = ROOT / "experiments/name-handling/f5/REQUEST.md"
BIGIP_QUESTIONS = dict(
    re.findall(r"(?m)^\| (F5-N\d\d) \| (.*?) \|", REQUEST.read_text())
)


def question(record: dict) -> str | None:
    identity = record.get("case_id", "")
    if identity in QUESTIONS:
        return QUESTIONS[identity]
    if identity == "version":
        return "Which Tcl version and execution namespace are available in this tmsh context, so that observations from this context can be scoped correctly?"
    match = re.match(r"(F5-N\d\d)", str(record.get("question_id") or ""))
    return BIGIP_QUESTIONS.get(match.group(1)) if match else None


def main() -> None:
    output = ROOT / "evidence/tcl"
    written = skipped = 0
    for script_path in sorted(
        (ROOT / "experiments/name-handling").glob("**/tmsh-cli-script-*/input.tcl")
    ):
        directory = script_path.parent
        stdout_path = directory / "invoke.stdout"
        stderr_path = directory / "invoke.stderr"
        exit_path = directory / "invoke.exit"
        records_path = directory.parent / "records.json"
        if not all(
            path.is_file()
            for path in (stdout_path, stderr_path, exit_path, records_path)
        ):
            skipped += 1
            continue
        stdout, stderr = stdout_path.read_bytes(), stderr_path.read_bytes()
        lines = stdout.splitlines()
        identities = {
            fields[2].decode("ascii")
            for line in lines
            if (fields := line.split(b"|"))[0] == b"NHO" and len(fields) == 13
        }
        if not identities:
            # A failed load or command invocation does not prove Tcl ran.
            skipped += 1
            continue
        records = [
            row
            for row in json.loads(records_path.read_bytes())
            if (row.get("actual_input") or {}).get("path")
            == str(script_path.relative_to(ROOT))
            and (row.get("provider") or {}).get("context") == "tmsh-cli-script"
        ]
        by_case = {row["case_id"]: row for row in records}
        if not identities <= by_case.keys() or any(
            question(by_case[name]) is None for name in identities
        ):
            skipped += 1
            continue
        providers = [by_case[name]["provider"] for name in sorted(identities)]
        versions = {str(row.get("version") or "Not recorded") for row in providers}
        patches = {
            str(row.get("interpreter_patchlevel") or "Not recorded")
            for row in providers
        }
        if len(versions) != 1 or len(patches) != 1:
            skipped += 1
            continue
        exit_text = exit_path.read_text().strip()
        if not re.fullmatch(r"[0-9]+", exit_text):
            skipped += 1
            continue
        parts = [
            f"# BIG-IP tmsh Tcl: {directory.parent.name}, {directory.name}\n\n",
            ("## Question\n\nThis script contains the following cases, which investigate "
            "Tcl name handling relevant to the LSP, compiler and VM. Each observation "
            "is limited to its own case and this execution context.\n"),
        ]
        for identity in sorted(identities):
            parts.append(f"\n- **{identity}:** {question(by_case[identity])}\n")
        parts.append("\n## Assumptions\n\n")
        for identity in sorted(identities):
            record = by_case[identity]
            scope = str(record.get("scope") or "Not recorded").replace("\n", " ")
            reached = record.get("intended_lookup_reached")
            state = (
                "reached"
                if reached is True
                else "not reached"
                if reached is False
                else "not recorded"
            )
            parts.append(f"- **{identity}:** {scope} Intended operation: {state}.\n")
        parts.extend(
            [
                "\nOther assumptions were not separately recorded.\n\n## Script\n\n",
                markdown_block(script_path.read_bytes()),
                "\n## Results\n\n### BIG-IP tmsh CLI script\n\n",
                f"- Recorded version: BIG-IP {next(iter(versions))}; Tcl patchlevel {next(iter(patches))}.\n",
                "- Recorded input channel: tmsh run cli script.\n",
                f"- Exit status: {exit_text}\n",
            ]
        )
        for title, data in (("Standard output", stdout), ("Standard error", stderr)):
            if data:
                parts.extend(
                    [f"- {title}:\n\n", markdown_block(data, "text-visible"), "\n"]
                )
            else:
                parts.append(f"- {title}: empty.\n")
        parts.append(
            "\nThe exit status and streams are from the recorded tmsh invocation. "
            "The entries below decode complete NHO payloads from its standard output.\n"
        )
        for identity in sorted(identities):
            rows = b"\n".join(
                line
                for line in lines
                if line.startswith(b"NHO|") and line.split(b"|")[2] == identity.encode()
            )
            parts.append(f"\n#### {identity}\n")
            parts.append(decoded_rows(rows, identity, "tmsh-cli-script"))
        document = "".join(parts).rstrip() + "\n"
        errors = validate_document(document)
        if errors:
            raise ValueError(f"{script_path}: {errors}")
        path = output / f"bigip-{directory.parent.name}-{directory.name}.md"
        if path.exists() and path.read_text() != document:
            raise ValueError(f"refusing to overwrite {path}")
        path.write_text(document)
        written += 1
    print(
        f"Recovered {written} tmsh captures; skipped {skipped} ambiguous or unreached inputs"
    )


if __name__ == "__main__":
    main()
