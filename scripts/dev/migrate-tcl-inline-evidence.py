#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Recover the recorded Tcl variable-inlining comparison scripts."""

from __future__ import annotations

import json
import re
from collections import defaultdict
from pathlib import Path

from tcl_evidence_format import markdown_block, validate_document

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "rust/tcl-lsp-core/tests/data/native_inline_variable_semantics"
MANIFEST = ROOT / "docs/design/analysis/name-resolution-proofs/manifest.json"


def quote(value: str) -> str:
    return "\n".join("> " + line if line else ">" for line in value.strip().splitlines())


def main() -> None:
    manifest = json.loads(MANIFEST.read_bytes())
    associated: dict[tuple[str, int], dict[str, dict]] = defaultdict(dict)
    for proof in manifest["proofs"]:
        if proof.get("kind") != "native-observation":
            continue
        for item in proof.get("evidence", []):
            path = item.get("path", "")
            pointer = item.get("json_pointer", "")
            match = re.fullmatch(r"/records/([0-9]+)", pointer)
            if not match:
                continue
            for mode in ("file", "stdin"):
                if path == str((BASE / mode / "receipts.json").relative_to(ROOT)):
                    associated[(mode, int(match[1]))][proof["id"]] = proof
    groups: dict[bytes, dict] = {}
    for mode in ("file", "stdin"):
        directory = BASE / mode
        receipt = json.loads((directory / "receipts.json").read_bytes())
        for index, row in enumerate(receipt["records"]):
            proofs = associated[(mode, index)]
            if not proofs:
                raise ValueError(f"missing semantic question for {mode} row {index}")
            script = bytes.fromhex(row["source_hex"])
            stdout = bytes.fromhex(row["stdout_hex"])
            stderr = bytes.fromhex(row["stderr_hex"])
            for field, expected in (("source", script), ("stdout", stdout), ("stderr", stderr)):
                if (directory / row[field]).read_bytes() != expected:
                    raise ValueError(f"retained {field} differs from the captured bytes in {mode} row {index}")
            version = bytes.fromhex(row["version_stdout_hex"]).decode("utf-8").strip() or "Not recorded"
            group = groups.setdefault(script, {"proofs": {}, "runs": [], "name": row["source"]})
            group["proofs"].update(proofs)
            group["runs"].append((mode, version, row, stdout, stderr))
    output = ROOT / "evidence/tcl"
    names: set[str] = set()
    for script, group in groups.items():
        parts = [
            "# Tcl variable inlining execution\n\n## Question\n\n",
            ("How do original variable parsing, reaching definitions, callee upvar "
            "mutation and read observers affect literal single-use variable inlining "
            "in the LSP? The recorded comparisons ask:\n\n"),
        ]
        for proof in group["proofs"].values():
            parts.extend([quote(proof["problem_statement"]), "\n\n", quote(proof["question"]), "\n\n"])
        parts.append(
            "This file records one side of the comparison; its result alone does not establish the result of the other script.\n\n"
            "## Assumptions\n\n"
        )
        for scope in dict.fromkeys(proof["scope"] for proof in group["proofs"].values()):
            parts.extend([quote(scope), "\n\n"])
        parts.extend(
            [
                ("The captures use fresh native processes. Handwritten replacement "
                "scripts are controls, not output from the Rust refactor. BIG-IP "
                "was not tested. Other assumptions were not separately recorded.\n\n"),
                "## Script\n\n", markdown_block(script), "\n## Results\n",
            ]
        )
        for mode, version, row, stdout, stderr in group["runs"]:
            status = row.get("exit")
            status = str(status) if type(status) is int else "Not recorded"
            provider = str(row.get("provider") or "Not recorded")
            channel = "script file" if mode == "file" else "standard input"
            parts.extend(
                [
                    f"\n### {provider} {version}, {channel}\n\n",
                    f"- Recorded version: {version}\n",
                    f"- Recorded input channel: {channel}\n",
                    f"- Exit status: {status}\n",
                ]
            )
            for field, data in (("Standard output", stdout), ("Standard error", stderr)):
                if data:
                    parts.extend([f"- {field}:\n\n", markdown_block(data, "text-visible"), "\n"])
                else:
                    parts.append(f"- {field}: empty.\n")
        document = "".join(parts).rstrip() + "\n"
        errors = validate_document(document)
        if errors:
            raise ValueError(f"{group['name']}: {errors}")
        name = re.sub(r"^[^-]+-", "", Path(group["name"]).stem)
        slug = re.sub(r"[^a-z0-9-]+", "-", name.lower()).strip("-")
        if slug in names:
            raise ValueError(f"ambiguous output name for {group['name']}")
        names.add(slug)
        path = output / f"variable-inline-{slug}.md"
        if path.exists() and path.read_text() != document:
            raise ValueError(f"refusing to overwrite {path}")
        path.write_text(document)
    print(f"Recovered {len(groups)} distinct scripts and 120 recorded executions")


if __name__ == "__main__":
    main()
