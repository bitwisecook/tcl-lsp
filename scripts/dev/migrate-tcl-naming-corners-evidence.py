#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Recover original Tcl naming-corner scripts and their process captures."""

import json
import tkinter
from collections import defaultdict
from pathlib import Path

from tcl_evidence_format import markdown_block, validate_document

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "rust/tcl-syntax/tests/data/native_naming_corners"


def main():
    parser = tkinter.Tcl()
    manifest = json.loads((DATA / "manifest.json").read_bytes())
    legacy = json.loads(
        (
            ROOT / "docs/design/analysis/name-resolution-proofs/manifest.json"
        ).read_bytes()
    )
    questions = {
        r["id"]: r["question"]
        for r in legacy["proofs"]
        if r["id"].startswith("naming.corner.")
    }
    versions = {}
    for run in manifest["engines"]:
        fields = parser.splitlist((DATA / run["stdout"]).read_text().strip())
        if len(fields) != 2:
            raise ValueError(f"unexpected version capture for {run['engine']}")
        versions[run["engine"]] = fields[1]
    grouped = defaultdict(list)
    for row in manifest["rows"]:
        if row.get("timeout") or type(row.get("exit_status")) is not int:
            raise ValueError(f"incomplete process capture: {row['case']}")
        source = (DATA / row["source"]).read_bytes()
        stdout = (DATA / row["stdout"]).read_bytes()
        stderr = (DATA / row["stderr"]).read_bytes()
        words = parser.splitlist(stdout.decode("ascii").strip())
        if (
            len(words) != 2
            or words[0] != str(row["guest_completion_code"])
            or words[1] != row["guest_result_hex"]
        ):
            raise ValueError(
                f"guest result does not match stdout: {row['case']}/{row['engine']}"
            )
        grouped[source].append((row, stdout, stderr, bytes.fromhex(words[1])))
    if len(grouped) != 40:
        raise ValueError("unexpected number of distinct sources")
    for source, records in grouped.items():
        case_names = {row["case"] for row, *_ in records}
        if len(case_names) != 1 or len(records) != 6:
            raise ValueError("ambiguous case or environment association")
        case = next(iter(case_names))
        question = questions["naming.corner." + case]
        parts = [
            (
                f"# Tcl naming: {case.replace('-', ' ')}\n\n## Question\n\n{question} "
                "This constrains name resolution and binding behavior in the LSP, compiler and VM for the tested inputs.\n\n"
            ),
            (
                "## Assumptions\n\nEach recorded case was executed as a file in a separate shell process. "
                "The script catches the guest operation and prints its completion code and result bytes as hex; "
                "that Tcl completion is distinct from the shell process exit status. "
                "Results apply to the listed environments and inputs; BIG-IP was not part of this capture. "
                "Other assumptions were not separately recorded.\n\n## Script\n\n"
            ),
            markdown_block(source),
            "\n## Results\n",
        ]
        for row, stdout, stderr, result in sorted(
            records, key=lambda x: x[0]["engine"]
        ):
            engine = row["engine"]
            parts.extend(
                [
                    f"\n### {engine}\n\n- Recorded version: {versions[engine]}.\n",
                    f"- Recorded input channel: file argument to `{row['launcher']}`.\n",
                    f"- Exit status: {row['exit_status']}.\n- Standard output:\n\n",
                    markdown_block(stdout, "text-visible"),
                    "\n- Standard error: "
                    + (
                        "empty.\n"
                        if not stderr
                        else "\n\n" + markdown_block(stderr, "text-visible")
                    ),
                    (
                        f"\nThe script recorded Tcl completion code {row['guest_completion_code']}. "
                        "Its hex-encoded guest result, decoded for reading:\n\n"
                    ),
                    markdown_block(result, "text-visible"),
                ]
            )
        document = "".join(parts).rstrip() + "\n"
        if errors := validate_document(document):
            raise ValueError(f"{case}: {errors}")
        output = ROOT / "evidence/tcl" / f"native-naming-corner-{case}.md"
        if output.exists() and output.read_text() != document:
            raise ValueError(f"refusing to overwrite {output}")
        output.write_text(document)
    print(f"Recovered {len(grouped)} naming-corner scripts across six environments")


if __name__ == "__main__":
    main()
