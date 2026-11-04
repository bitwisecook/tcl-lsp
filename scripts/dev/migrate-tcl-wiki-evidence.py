#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Recover the retained multi-version Tcl Wiki naming experiment."""

from __future__ import annotations

import json
import re
import runpy
from pathlib import Path

from tcl_evidence_format import markdown_block, validate_document

ROOT = Path(__file__).resolve().parents[2]
QUESTIONS = runpy.run_path(str(ROOT / "scripts/dev/migrate-tcl-apl-evidence.py"))[
    "PATTERN_QUESTIONS"
]


def main() -> None:
    base = ROOT / "experiments/name-handling/patterns"
    script = (base / "wiki-probes.tcl").read_bytes()
    identities = re.findall(rb"(?m)^::tcltest::test (wiki-[a-z0-9-]+) ", script)
    names = [identity.decode("ascii") for identity in identities]
    if len(names) != len(set(names)) or any(name not in QUESTIONS for name in names):
        raise ValueError("the retained script contains an unknown or duplicate case")
    provenance = json.loads((base / "wiki-results/provenance.json").read_bytes())
    if provenance.get("probes") != "experiments/name-handling/patterns/wiki-probes.tcl":
        raise ValueError("the retained run does not identify this script")
    parts = [
        "# Tcl Wiki naming patterns across Tcl versions\n\n",
        (
            "## Question\n\nThis script examines the following Tcl naming and "
            "execution distinctions that affect LSP navigation and diagnostics, "
            "compiler transformations, and VM command and variable lookup. "
            "Each case answers only its stated question.\n"
        ),
    ]
    for name in names:
        parts.append(f"\n- **{name}:** {QUESTIONS[name]}\n")
    parts.extend(
        [
            (
                "\n## Assumptions\n\nEach test creates a fresh child interpreter. "
                "The real tcltest package supplies the test runner. A skipped "
                "constraint is not an observation of the skipped behavior. The "
                "Wiki examples motivate the reductions; they do not establish "
                "runtime behavior themselves. BIG-IP and Jim were not measured "
                "in this collection. Other assumptions were not separately recorded.\n\n"
            ),
            "## Script\n\n",
            markdown_block(script),
            "\n## Results\n",
        ]
    )
    for record in provenance["records"]:
        version = record["patchlevel"]
        invocation = record.get("invocation") or []
        if not invocation or not invocation[-1].endswith(
            "/experiments/name-handling/patterns/wiki-probes.tcl"
        ):
            raise ValueError(f"the {version} invocation does not identify the script")
        status = record.get("exit_code")
        status = str(status) if type(status) is int else "Not recorded"
        parts.extend(
            [
                f"\n### Tcl {version}\n\n",
                f"- Recorded version: {version}\n",
                "- Recorded input channel: script file passed to tclsh.\n",
                f"- Exit status: {status}\n",
            ]
        )
        for field, suffix in (
            ("Standard output", "stdout"),
            ("Standard error", "stderr"),
        ):
            data = (base / "wiki-results" / f"tcl{version}.{suffix}").read_bytes()
            if data:
                parts.extend(
                    [f"- {field}:\n\n", markdown_block(data, "text-visible"), "\n"]
                )
            else:
                parts.append(f"- {field}: empty.\n")
    document = "".join(parts).rstrip() + "\n"
    errors = validate_document(document)
    if errors:
        raise ValueError("; ".join(errors))
    path = ROOT / "evidence/tcl/wiki-naming-patterns.md"
    if path.exists() and path.read_text() != document:
        raise ValueError(f"refusing to overwrite {path}")
    path.write_text(document)
    print(
        f"Recovered {len(names)} questions across {len(provenance['records'])} Tcl versions"
    )


if __name__ == "__main__":
    main()
