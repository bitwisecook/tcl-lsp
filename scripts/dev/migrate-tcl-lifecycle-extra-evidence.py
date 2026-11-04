#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Recover the standalone lifecycle and callback-argument event payloads."""

import re
import runpy
import tarfile
import tkinter
from pathlib import Path

from tcl_evidence_format import markdown_block, validate_document

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261007vllex"
helpers = runpy.run_path(
    str(ROOT / "scripts/dev/migrate-tcl-context-archive-evidence.py")
)
emit_result = helpers["emit_result"]


def payload(version, name):
    root = f"r2286vl1-fixtures-{version}"
    with tarfile.open(DATA / f"appliance/{root}.tgz") as archive:
        wrapper = archive.extractfile(f"{root}/sources/{name}.conf").read()
        match = re.search(rb"set payload \[binary format H\* ([0-9a-f]+)\]", wrapper)
        if not match:
            raise ValueError(f"payload absent from {name}")
        source = bytes.fromhex(match[1].decode("ascii"))
        if name == "argument-bytes":
            adapted = archive.extractfile(
                f"{root}/adapted/active-element-argument-bytes.tcl"
            ).read()
            installed = (DATA / "appliance/vl-v4-argument-installed.conf").read_bytes()
            if source != adapted or match[1] not in installed:
                raise ValueError("argument payload differs from installed source")
        return source


def write(name, parts):
    document = "".join(parts).rstrip() + "\n"
    errors = validate_document(document)
    if errors:
        raise ValueError(f"{name}: {errors}")
    path = ROOT / "evidence/tcl" / f"bigip-r2286vl1-{name}.md"
    if path.exists() and path.read_text() != document:
        raise ValueError(f"refusing to overwrite {path}")
    path.write_text(document)


def main():
    parser = tkinter.Tcl()
    source = payload("v2", "main-supported")
    if (
        DATA / "appliance/vl1-v2-load-main-supported.status"
    ).read_text().strip() != "0":
        raise ValueError("main supported wrapper was not loaded")
    responses = [
        (DATA / f"client-server/vl-main-u{i}.body").read_bytes() for i in range(4)
    ]
    if len(set(responses)) != 1 or not responses[0].startswith(b"vl1 "):
        raise ValueError("main responses differ or are incomplete")
    parts = [
        "# BIG-IP active variable lifecycle, modern and legacy traces\n\n## Question\n\n",
        "When a read trace unsets and recreates a scalar during lappend, what values and callbacks result with the modern and legacy trace APIs? This constrains trace dispatch and variable-cell lifetime in the compiler and VM.\n\n",
        "## Assumptions\n\nThis payload ran under catch in an HTTP_REQUEST wrapper after the literal rule was rejected. It constructs the comparison within one request; the results do not establish behavior in other contexts or versions. The response bodies do not themselves record TMM identity or an outer completion code.\n\n## Script\n\n",
        markdown_block(source),
        "\n## Results\n\n### BIG-IP HTTP_REQUEST responses\n\n",
        "- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel not separately recorded by this run.\n",
        "- Recorded input channel: HTTP_REQUEST, evaluated by the supported wrapper.\n",
        "- Exit status: Not recorded.\n- Standard output: Not recorded.\n- Standard error: Not recorded.\n- Recorded result:\n\n",
        markdown_block(responses[0], "text-visible"),
        "\nFour retained HTTP response bodies were byte-identical.\n",
    ]
    write("main-modern-legacy", parts)

    source = payload("v4", "argument-bytes")
    parts = [
        "# BIG-IP array-element trace callback arguments\n\n## Question\n\n",
        "When a read trace unsets and recreates an array element during lappend, what root and index bytes are passed to the read and unset callbacks, and which value does lappend use? This constrains array-element identity and trace dispatch in the compiler and VM.\n\n",
        "## Assumptions\n\nThe payload ran under catch in an HTTP_REQUEST wrapper. Callback names were encoded by the script using binary scan; the result below retains those encoded fields. The observation is limited to the recorded context and inputs.\n\n## Script\n\n",
        markdown_block(source),
        "\n## Results\n",
    ]
    seen = set()
    for port in (42100, 42101, 42102, 42103):
        body = (DATA / f"client-server/vl-argument-bytes-{port}.body").read_text()
        run, group, unit, code, scan_code, encoded, scan_result = parser.splitlist(body)
        if (
            run != "vl1"
            or scan_code != "0"
            or scan_result != "1"
            or (group, unit) in seen
        ):
            raise ValueError(f"invalid argument response on port {port}")
        seen.add((group, unit))
        emit_result(
            parts,
            f"TMM {group}:{unit}, HTTP_REQUEST",
            "21.1.0.1 build 0.0.26",
            code,
            bytes.fromhex(encoded),
            f"The response was collected from source port {port}.",
        )
    if seen != {("0", str(i)) for i in range(4)}:
        raise ValueError("argument response did not cover all four units")
    write("argument-bytes", parts)
    print("Recovered two lifecycle event payloads")


if __name__ == "__main__":
    main()
