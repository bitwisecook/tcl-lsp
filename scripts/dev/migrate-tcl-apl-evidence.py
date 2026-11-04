#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Recover individually executed callbacks from the retained broad APL run."""

from __future__ import annotations

import json
import re
from collections import defaultdict
from pathlib import Path

from tcl_evidence_format import markdown_block, validate_document

ROOT = Path(__file__).resolve().parents[2]
RUN = ROOT / "experiments/name-handling/f5/answers/nh20261012aplbroad1"


def main() -> None:
    request = (RUN.parents[1] / "REQUEST.md").read_text()
    questions = dict(re.findall(r"(?m)^\| (F5-N\d\d) \| (.*?) \|", request))
    cases = json.loads((RUN / "iapp-apl-cases.json").read_bytes())
    directory = RUN / "iapp-apl-0"
    source = (directory / "input.tcl").read_bytes()
    wrapper = (directory / "applied.conf").read_bytes()
    receipt = json.loads((directory / "invocation.json").read_bytes())
    if receipt.get("projection_validation") != "passed":
        raise ValueError("browser projection was not validated")
    scripts = re.findall(
        rb'(?ms)^set nh_apl_rows \{\}\n.*?^return \[join \$nh_apl_rows "\\n"\]\n',
        source,
    )
    if len(scripts) != len(cases) or b"\n".join(scripts) != source:
        raise ValueError("cannot uniquely split the retained callback source")
    lines: dict[str, list[bytes]] = defaultdict(list)
    for line in (directory / "invoke.stdout").read_bytes().splitlines(keepends=True):
        fields = line.rstrip(b"\n").split(b"|")
        if len(fields) != 13 or fields[0] != b"NHO":
            raise ValueError("unexpected browser result row")
        lines[fields[2].decode("ascii")].append(line)
    if set(lines) != {case["id"] for case in cases}:
        raise ValueError("browser rows do not match the callback cases")
    output = ROOT / "evidence/tcl"
    output.mkdir(parents=True, exist_ok=True)
    written = 0
    for index, (case, script) in enumerate(zip(cases, scripts, strict=True)):
        question_id = case.get("question_id")
        if question_id not in questions:
            continue
        identity = case["id"]
        header = f"choice result{index} tcl {{\n".encode()
        if header + script + b"\n}" not in wrapper:
            raise ValueError(f"callback {identity} is not in its recorded APL choice")
        if f"|{identity}|iapp-apl|".encode() not in script:
            raise ValueError(f"callback {identity} has a different embedded identity")
        # The raw option values are retained verbatim. Hex payloads can split
        # over several rows; group them before presenting a readable decoding.
        chunks: dict[tuple[bytes, ...], list[tuple[int, int, bytes]]] = defaultdict(
            list
        )
        for line in lines[identity]:
            fields = line.rstrip(b"\n").split(b"|")
            chunks[tuple(fields[3:10])].append(
                (int(fields[10]), int(fields[11]), bytes.fromhex(fields[12].decode()))
            )
        decoded: list[str] = []
        for key, fragments in chunks.items():
            fragments.sort()
            if [part[0] for part in fragments] != list(range(len(fragments))) or any(
                part[1] != len(fragments) for part in fragments
            ):
                raise ValueError(f"incomplete result chunks for {identity}")
            _, phase, completion, unit, invocation, field, encoding = (
                item.decode("ascii") for item in key
            )
            data = b"".join(part[2] for part in fragments)
            decoded.append(
                f"\n- {phase}, completion {completion}, {field} "
                f"({encoding}; unit {unit}, invocation {invocation}):\n\n"
                + markdown_block(data, "text-visible")
            )
        scope = str(case.get("scope") or "Not recorded").replace("\n", " ")
        text = (
            f"# BIG-IP APL callback: {identity}\n\n"
            f"## Question\n\n{questions[question_id]}\n\n"
            f"This callback addresses only the `{identity}` case in that broader "
            "question. Its result can inform name resolution and diagnostics in "
            "the LSP, compiler and VM, but does not establish the behavior of "
            "other cases or environments.\n\n"
            f"## Assumptions\n\nRecorded case scope: {scope}\n\n"
            "Other assumptions were not separately recorded. The APL callback "
            "was selected through the browser; this is not an iApp "
            "implementation, an iRule, or a stock Tcl measurement. A successful "
            "caught cleanup does not prove that every cleanup operation succeeded.\n\n"
            "## Script\n\n" + markdown_block(script) + "\n"
            "## Results\n\n### BIG-IP iApp APL\n\n"
            "- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel 8.4.6.\n"
            "- Recorded input channel: browser Application Services template choice callback.\n"
            "- Exit status: Not recorded.\n"
            "- Standard output: Not recorded.\n"
            "- Standard error: Not recorded.\n"
            "- Recorded result:\n\n"
            + markdown_block(b"".join(lines[identity]), "text-visible")
            + "\nThese are the retained DOM option values projected one row per "
            "value, not process stdout. The fields below decode their hexadecimal "
            "payloads; completion codes are the callback's caught Tcl codes.\n"
            + "".join(decoded)
        )
        errors = validate_document(text)
        if errors:
            raise ValueError(f"{identity}: {errors}")
        path = output / f"bigip-apl-{identity}.md"
        if path.exists() and path.read_text() != text:
            raise ValueError(f"refusing to overwrite {path}")
        path.write_text(text)
        written += 1
    print(f"Recovered {written} individual APL callback captures")


if __name__ == "__main__":
    main()
