#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Recover individually executed callbacks from retained BIG-IP APL runs."""

from __future__ import annotations

import argparse
import json
import re
from collections import defaultdict
from pathlib import Path

from tcl_evidence_format import markdown_block, validate_document

ROOT = Path(__file__).resolve().parents[2]
RUN = ROOT / "experiments/name-handling/f5/answers/nh20261012aplbroad1"
PATTERN_RUNS = {
    name: ROOT / "experiments/name-handling/patterns/f5/answers" / name
    for name in ("nh20261012aplpat1", "nh20261012aplexact1")
}
PATTERN_QUESTIONS = {
    "alias-prefix-lifetime": "After an interpreter alias with a fixed multiword argument is renamed, what does querying the old alias name return, and how does the renamed command apply its retained prefix to a new argument?",
    "computed-binding-mutation": "When a command name is assembled into a variable and the command at that name is replaced, does executing the unchanged variable use the original or the replacement binding?",
    "computed-qualified": "Can a qualified command name assembled from a namespace variable be invoked as a computed command head, and which command does that spelling reach?",
    "ensemble-prefix-map": "Does an ensemble subcommand mapping preserve the elements of a list command prefix, including a fixed argument containing a space, when passing a new argument?",
    "factory-body-near-miss": "When a loop defines procedures using a braced body containing the loop variable, is that variable substituted during definition or looked up when the generated procedure executes?",
    "finite-factory": "When a loop constructs procedure bodies as lists containing the current loop value, do the resulting commands retain their respective values?",
    "frame-wrapper": "Does a forwarding procedure using uplevel 1 let a subsequent upvar 1 reach a spaced variable name in the original caller's frame?",
    "frame-wrapper-near-miss": "When the same forwarding operation uses eval rather than uplevel 1, which caller frame does upvar 1 reach, and is the original caller's spaced variable changed?",
    "keyed-variable-link": "Can upvar link to a namespace array whose constructed name contains a space, and does writing an element through the link update the intended namespace variable?",
    "list-prefix": "Does a list command prefix preserve a command name containing spaces and its fixed multiword argument when eval invokes it with an additional argument?",
    "namespace-captured-prefix": "Does a namespace code prefix retain the defining namespace when evaluated elsewhere, including after a same-named global command is defined, and preserve its argument elements?",
    "oo-prefix-deleted-near-miss": "What happens to a command prefix containing an object's self command after that object is destroyed: can it still invoke the method or does command lookup fail?",
    "oo-self-prefix": "Can a TclOO method return a list prefix containing its own object command and a multiword argument for later method invocation?",
    "string-prefix-near-miss": "What happens when a command name containing spaces is interpolated into an eval script string rather than preserved as a list command-prefix element?",
    "tcltest-shared-lifecycle": "Do tcltest setup, body and cleanup share the expected namespace variable state, and is the setup variable absent after cleanup?",
}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run", choices=["broad", *PATTERN_RUNS], default="broad")
    args = parser.parse_args()
    request = (RUN.parents[1] / "REQUEST.md").read_text()
    questions = dict(re.findall(r"(?m)^\| (F5-N\d\d) \| (.*?) \|", request))
    run = RUN if args.run == "broad" else PATTERN_RUNS[args.run]
    if args.run != "broad":
        questions.update(
            {f"f5-pattern-{key}": value for key, value in PATTERN_QUESTIONS.items()}
        )
    cases = json.loads((run / "iapp-apl-cases.json").read_bytes())
    records = json.loads((run / "records.json").read_bytes())
    providers = {row["case_id"]: row["provider"] for row in records}
    directory = run / "iapp-apl-0"
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
        provider = providers[identity]
        if provider.get("context") != "iapp-apl":
            raise ValueError(f"unexpected context for {identity}")
        version = provider.get("version") or "Not recorded"
        patchlevel = provider.get("interpreter_patchlevel") or "Not recorded"
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
            f"- Recorded version: BIG-IP {version}; Tcl patchlevel {patchlevel}.\n"
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
        prefix = "bigip-apl" if args.run == "broad" else f"bigip-apl-{args.run}"
        path = output / f"{prefix}-{identity}.md"
        if path.exists() and path.read_text() != text:
            raise ValueError(f"refusing to overwrite {path}")
        path.write_text(text)
        written += 1
    print(f"Recovered {written} individual APL callback captures")


if __name__ == "__main__":
    main()
