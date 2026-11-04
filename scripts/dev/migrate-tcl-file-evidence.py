#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Recover directly invoked native Tcl/Jim source-file observations."""

from __future__ import annotations

import json
import re
from collections import defaultdict
from pathlib import Path

from tcl_evidence_format import markdown_block, script_from_markdown, validate_document

ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / "docs/design/analysis/name-resolution-proofs/manifest.json"
SCOPES = {
    "naming.property.original-configure-retained-table-and-read-snapshot": "These source-file observations measure script-visible completion, result, event and property-roster values. They do not observe private pointer identity, reference-count chronology or internal error-formatting state. Two separately recorded setup refusals are distinct from the configurable-class observations. BIG-IP and Rust implementation equivalence were not tested. The inherited full environment and build replay were not recorded.",
    "naming.source.original-surrogate-versus-replacement-escape-values": r"The source is ASCII LF text with no raw NUL or direct non-ASCII input. Quoted \u escapes are evaluated by the CLI; braced words retain literal escape spelling. Public commands emit boolean and length rows, not private string bytes or object identity. Compiler version and unlisted inherited environment were not recorded. BIG-IP was not tested.",
    "naming.tcloo.original-abstract-manufacturer-public-visibility": "These are independent source-file runs. Tcl 9 runs reach the visibility operations; older Tcl and Jim runs can instead report that the producer is unavailable. Private method, frame and object identities, BIG-IP/TMM behavior and Rust implementation equivalence were not measured. The full inherited environment and build replay were not recorded.",
    "naming.tcltest.original-alias-caller-frame-lifecycle": "The ASCII LF CLI script uses real package loading, interpreter aliases and public catch/list results. It substitutes benign variable names and does not run the original tclopt constructors. Compiler version and unlisted inherited environment were not recorded. Private frame or object identity, general Tcltest behavior, BIG-IP behavior and Rust assertions are outside this observation.",
    "naming.variable.dictionary-update-envelope-controls": "Four separate ASCII LF scripts compare guarded versus bare bodies and eval versus catch of a script value. Each runs in a fresh process. The processes do not print their versions; the recorded version is associated with a separate version capture for the executable. Raw NUL, non-ASCII, continuation and control escapes, private native identities and general compiler or editor equivalence are not tested.",
    "naming.variable.original-eval-container-storage-columns": "Ten ASCII LF programs use puts with an eval script and run in separate fresh processes. The process does not print its own version; the recorded version is associated with a separate version capture for the executable. Raw NUL, non-ASCII, continuation and control escapes are not exercised. Any dictionary absence applies only to the captured guarded stock Tcl 8.4 branch. Private frame/object identity, general storage equivalence and Rust assertion outcomes are not established.",
}


def quote(value: str) -> str:
    return "\n".join(
        "> " + line if line else ">" for line in value.strip().splitlines()
    )


def main() -> None:
    output = ROOT / "evidence/tcl"
    existing = {script_from_markdown(path.read_text()) for path in output.glob("*.md")}
    groups: dict[bytes, dict] = {}
    for proof in json.loads(MANIFEST.read_bytes())["proofs"]:
        if proof.get("kind") != "native-observation" or proof["id"] not in SCOPES:
            continue
        evidence = {item["id"]: item for item in proof.get("evidence", [])}
        for provider in proof.get("providers", []):
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
                command = receipt.get("command") if isinstance(receipt, dict) else None
                if (
                    not isinstance(command, list)
                    or len(command) != 2
                    or not isinstance(command[-1], str)
                    or not command[-1].endswith(".tcl")
                ):
                    continue
                name = Path(command[-1]).name
                if len(inputs[name]) != 1:
                    continue
                source = ROOT / next(iter(inputs[name]))
                stdout, stderr = (
                    receipt_path.parent / "stdout",
                    receipt_path.parent / "stderr",
                )
                if not all(path.is_file() for path in (source, stdout, stderr)):
                    continue
                script = source.read_bytes()
                if script in existing:
                    continue
                group = groups.setdefault(
                    script, {"proofs": {}, "captures": {}, "source": source}
                )
                group["proofs"][proof["id"]] = proof
                group["captures"][str(receipt_path)] = (
                    provider,
                    receipt,
                    stdout.read_bytes(),
                    stderr.read_bytes(),
                )
    used: set[str] = set()
    for script, group in groups.items():
        proofs = list(group["proofs"].values())
        parts = ["# Tcl source-file execution evidence\n\n## Question\n\n"]
        for proof in proofs:
            parts.extend(
                [
                    quote(proof["problem_statement"]),
                    "\n\n",
                    quote(proof["question"]),
                    "\n\n",
                ]
            )
        parts.append(
            "This is one source in the stated question; it does not by itself establish results for other sources or environments.\n\n## Assumptions\n\n"
        )
        for proof in proofs:
            parts.extend([quote(SCOPES[proof["id"]]), "\n\n"])
        parts.extend(
            [
                "Other assumptions were not separately recorded.\n\n## Script\n\n",
                markdown_block(script),
                "\n## Results\n",
            ]
        )
        for index, (provider, receipt, stdout, stderr) in enumerate(
            group["captures"].values(), 1
        ):
            name = str(provider.get("id") or "Not recorded").replace("\n", " ")
            version = str(
                receipt.get("reported_version")
                or provider.get("version")
                or "Not recorded"
            ).replace("\n", " ")
            status = receipt.get("process_exit", receipt.get("exit"))
            status = str(status) if type(status) is int else "Not recorded"
            parts.extend(
                [
                    f"\n### {name}, run {index}\n\n",
                    f"- Recorded version: {version}\n",
                    "- Recorded input channel: source file passed to the interpreter.\n",
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
        document = "".join(parts).rstrip() + "\n"
        errors = validate_document(document)
        if errors:
            raise ValueError(f"{proofs[0]['id']}: {errors}")
        base = re.sub(
            r"[^a-z0-9-]+", "-", f"{proofs[0]['id']}-{group['source'].stem}".lower()
        ).strip("-")
        slug, index = base, 1
        while slug in used:
            index += 1
            slug = f"{base}-{index}"
        used.add(slug)
        path = output / f"source-file-{slug}.md"
        if path.exists() and path.read_text() != document:
            raise ValueError(f"refusing to overwrite {path}")
        path.write_text(document)
    print(f"Recovered {len(groups)} native source-file scripts")


if __name__ == "__main__":
    main()
