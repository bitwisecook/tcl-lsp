#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Recover executed r2286i Unicode logging event bodies."""

import collections
import re
import tkinter
from pathlib import Path

from tcl_evidence_format import markdown_block, validate_document

ROOT = Path(__file__).resolve().parents[2]
DATA = (
    ROOT
    / "scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006hi/appliance"
)
CASES = {
    "unicode_dynamic": "When UTF-8 byte sequences for accented letters, symbols and emoji are constructed with binary format H*, how do BIG-IP Tcl string length, binary scan and the log display treat those bytes? This constrains Unicode and bytearray modeling in the LSP, compiler and VM.",
    "unicode_literal_emoji_grinning": "When a literal grinning-face emoji is accepted in an iRule, what bytes, displayed value and string length does BIG-IP Tcl report? This constrains literal Unicode and byte-length modeling in the LSP, compiler and VM.",
    "unicode_literal_emoji_skin_tone": "When a literal emoji with a skin-tone modifier is accepted in an iRule, what bytes, displayed value and string length does BIG-IP Tcl report? This constrains multi-code-point literal and byte-length modeling in the LSP, compiler and VM.",
    "unicode_literal_emoji_flag": "When a literal flag made from two regional indicators is accepted in an iRule, what bytes, displayed value and string length does BIG-IP Tcl report? This constrains multi-code-point literal and byte-length modeling in the LSP, compiler and VM.",
}


def main():
    parser = tkinter.Tcl()
    installed = (DATA / "config-before-cleanup.txt").read_bytes()
    logs = (DATA / "ltm-raw.log").read_bytes().splitlines()
    for case, question in CASES.items():
        config = (DATA / f"{case}.conf").read_bytes()
        rule = parser.splitlist(config.decode("utf-8"))
        events = parser.splitlist(rule[3])
        if len(events) != 6 or events[:2] != ("when", "HTTP_REQUEST"):
            raise ValueError(f"unexpected event structure in {case}")
        script = events[2].encode("utf-8")
        if script not in config or script not in installed:
            raise ValueError(f"installed source differs for {case}")
        marker = f"R2286|r2286i|{case}|".encode()
        groups = collections.defaultdict(collections.Counter)
        for line in logs:
            if marker not in line:
                continue
            value = line.split(marker, 1)[1]
            unit_field, request_field, observed = value.split(b"|", 2)
            match = re.fullmatch(rb"unit=(\d+)", unit_field)
            if not match or not request_field.startswith(b"request="):
                raise ValueError(f"unexpected marker for {case}")
            unit = match[1].decode()
            groups[unit][observed] += 1
        if set(groups) != {"0", "1", "2", "3"}:
            raise ValueError(f"incomplete TMM coverage for {case}")
        parts = [
            f"# BIG-IP Unicode logging: {case.replace('_', ' ')}\n\n## Question\n\n{question}\n\n",
            (
                "## Assumptions\n\nThis is the HTTP_REQUEST event body from the installed rule. "
                "Each line below is the observed log suffix after the unit and request fields; "
                "identical suffixes are grouped and their counts shown. Logged display bytes "
                "and the script's binary-scan hex are distinct observations. The results apply "
                "only to these inputs and this context.\n\n## Script\n\n"
            ),
            markdown_block(script),
            "\n## Results\n",
        ]
        for unit, suffixes in sorted(groups.items()):
            if case == "unicode_dynamic" and (
                len(suffixes) != 13 or set(suffixes.values()) != {64}
            ):
                raise ValueError(f"unexpected dynamic results on unit {unit}")
            if case != "unicode_dynamic" and (
                len(suffixes) != 1 or set(suffixes.values()) != {16}
            ):
                raise ValueError(f"unexpected literal results on unit {unit}")
            # Each line is an exact recorded suffix; LF separates distinct observations.
            projection = b"\n".join(sorted(suffixes)) + b"\n"
            count = next(iter(suffixes.values()))
            parts.extend(
                [
                    f"\n### TMM unit {unit}, HTTP_REQUEST\n\n",
                    "- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.\n",
                    "- Recorded input channel: HTTP_REQUEST.\n- Exit status: Not recorded.\n- Standard output: Not recorded.\n- Standard error: Not recorded.\n- Recorded result:\n\n",
                    markdown_block(projection, "text-visible"),
                    f"\nEach displayed suffix appeared in {count} log lines for this unit. The line breaks separate captured log suffixes.\n",
                ]
            )
        document = "".join(parts).rstrip() + "\n"
        if errors := validate_document(document):
            raise ValueError(f"{case}: {errors}")
        output = ROOT / "evidence/tcl" / f"bigip-r2286i-{case.replace('_', '-')}.md"
        if output.exists() and output.read_text() != document:
            raise ValueError(f"refusing to overwrite {output}")
        output.write_text(document)
    print(f"Recovered {len(CASES)} Unicode logging event bodies")


if __name__ == "__main__":
    main()
