#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Recover one-source native Tcl/Jim C API execution observations."""

from __future__ import annotations

import json
import re
from collections import defaultdict
from pathlib import Path

from tcl_evidence_format import markdown_block, script_from_markdown, validate_document

ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / "docs/design/analysis/name-resolution-proofs/manifest.json"


def quote(value: str) -> str:
    return "\n".join(
        "> " + line if line else ">" for line in value.strip().splitlines()
    )


def clean_scope(value: str) -> str:
    """Keep the experiment's limitations without legacy artifact bookkeeping."""
    for sentence in (
        "Complete original source/request/compiler/ELF/process/stdout/stderr/code/result/version and required pins are retained.",
        "No assertion outcome is attached; fixture selection cannot donate general worker, library or lookup authority.",
        "This source definition supplies no process replay or passing software outcome.",
        "Existing raw provider evidence is unchanged.",
    ):
        value = value.replace(sentence, "")
    return re.sub(r" {2,}", " ", value).strip()


def main() -> None:
    output = ROOT / "evidence/tcl"
    existing = {script_from_markdown(path.read_text()) for path in output.glob("*.md")}
    manifest = json.loads(MANIFEST.read_bytes())
    groups: dict[bytes, dict] = {}
    for record in manifest["proofs"]:
        if record.get("kind") != "native-observation" or record["id"].startswith(
            "naming.bigip-pattern."
        ):
            continue
        # Legacy bookkeeping in a scope needs a human edit before publication.
        if re.search(
            r"(?i)sha.?256|digest|\bhash(?:es)?\b(?![- ](?:order|iteration|table|key))",
            record.get("scope") or "",
        ):
            continue
        evidence = {item["id"]: item for item in record.get("evidence", [])}
        for provider in record.get("providers", []):
            if provider.get("status") != "observed":
                continue
            refs = [
                evidence[name]
                for name in provider.get("evidence", [])
                if name in evidence
            ]
            inputs: dict[str, set[str]] = defaultdict(set)
            for item in refs:
                path = item.get("path", "")
                if item.get("role") == "input" and path.endswith(".tcl"):
                    inputs[Path(path).name].add(path)
            for item in refs:
                if item.get("role") != "provider" or not item.get("path", "").endswith(
                    "/receipt.json"
                ):
                    continue
                receipt_path = ROOT / item["path"]
                if not receipt_path.is_file():
                    continue
                try:
                    receipt = json.loads(receipt_path.read_bytes())
                except (UnicodeError, ValueError):
                    continue
                if not isinstance(receipt, dict):
                    continue
                command = receipt.get("command")
                if (
                    not isinstance(command, list)
                    or len(command) != 4
                    or command[1] != "-"
                    or command[3] != "-"
                ):
                    continue
                source_name = (
                    Path(command[2]).name if isinstance(command[2], str) else ""
                )
                if not source_name.endswith(".tcl") or len(inputs[source_name]) != 1:
                    continue
                source_path = ROOT / next(iter(inputs[source_name]))
                stdout_path, stderr_path = (
                    receipt_path.parent / "stdout",
                    receipt_path.parent / "stderr",
                )
                if not all(
                    path.is_file() for path in (source_path, stdout_path, stderr_path)
                ):
                    continue
                script, stdout, stderr = (
                    source_path.read_bytes(),
                    stdout_path.read_bytes(),
                    stderr_path.read_bytes(),
                )
                rows = [
                    line.split(b"|")
                    for line in stdout.splitlines()
                    if line.startswith(b"ORIGINAL|")
                ]
                if (
                    len(rows) != 1
                    or len(rows[0]) != 3
                    or not re.fullmatch(rb"[0-9]+", rows[0][1])
                ):
                    continue
                try:
                    decoded = bytes.fromhex(rows[0][2].decode("ascii"))
                except (UnicodeError, ValueError):
                    continue
                if script in existing:
                    continue
                group = groups.setdefault(
                    script, {"records": {}, "captures": {}, "source": source_path}
                )
                group["records"][record["id"]] = record
                group["captures"][str(receipt_path)] = (
                    provider,
                    receipt,
                    stdout,
                    stderr,
                    rows[0][1].decode(),
                    decoded,
                )
    written = 0
    filenames: set[str] = set()
    for script, group in groups.items():
        records = list(group["records"].values())
        parts = ["# Tcl counted-source execution evidence\n\n## Question\n\n"]
        for record in records:
            parts.extend(
                [
                    quote(record["problem_statement"]),
                    "\n\n",
                    quote(record["question"]),
                    "\n\n",
                ]
            )
        parts.append(
            "This script is one recorded source in the stated question; it does not by itself establish results for other sources or environments.\n\n## Assumptions\n\n"
        )
        for record in records:
            parts.extend(
                [
                    "Recorded scope and limits:\n\n",
                    quote(clean_scope(record["scope"])),
                    "\n\n",
                ]
            )
        parts.extend(
            [
                (
                    "Other assumptions were not separately recorded. The retained driver "
                    "evaluated this single counted Tcl source without a prelude or query "
                    "source; its process exit is separate from Tcl's completion code.\n\n"
                ),
                "## Script\n\n",
                markdown_block(script),
                "\n## Results\n",
            ]
        )
        for index, (provider, receipt, stdout, stderr, completion, result) in enumerate(
            group["captures"].values(), 1
        ):
            name = str(provider.get("id") or "Not recorded").replace("\n", " ")
            version = str(provider.get("version") or "Not recorded").replace("\n", " ")
            status = receipt.get("exit")
            status = str(status) if type(status) is int else "Not recorded"
            parts.extend(
                [
                    f"\n### {name}, run {index}\n\n",
                    f"- Recorded version: {version}\n",
                    "- Recorded input channel: counted-source native C API driver.\n",
                    f"- Exit status: {status}\n",
                ]
            )
            for title, data in (
                ("Standard output", stdout),
                ("Standard error", stderr),
            ):
                if data:
                    parts.extend(
                        [f"- {title}:\n\n", markdown_block(data, "text-visible"), "\n"]
                    )
                else:
                    parts.append(f"- {title}: empty.\n")
            parts.extend(
                [
                    (
                        f"\nTcl completion code for the original source: {completion}. "
                        "The following block decodes the ORIGINAL result payload from standard output.\n\n"
                    ),
                    markdown_block(result, "text-visible"),
                ]
            )
        document = "".join(parts).rstrip() + "\n"
        errors = validate_document(document)
        if errors:
            raise ValueError(f"{records[0]['id']}: {errors}")
        slug = re.sub(
            r"[^a-z0-9-]+", "-", f"{records[0]['id']}-{group['source'].stem}".lower()
        ).strip("-")
        base, index = slug, 1
        while slug in filenames:
            index += 1
            slug = f"{base}-{index}"
        filenames.add(slug)
        path = output / f"counted-{slug}.md"
        if path.exists() and path.read_text() != document:
            raise ValueError(f"refusing to overwrite {path}")
        path.write_text(document)
        written += 1
    print(f"Recovered {written} additional one-source native Tcl/Jim scripts")


if __name__ == "__main__":
    main()
