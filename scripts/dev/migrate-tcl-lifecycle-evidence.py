#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Recover the actual sequential variable-lifecycle evaluations on BIG-IP."""

import json
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
QUESTIONS = {
    "active-scalar": "When a scalar read trace unsets and recreates its variable during lappend, which value does the active operation use and which trace callbacks survive? This constrains variable-cell lifetime in the compiler and VM.",
    "active-element": "When an array-element read trace unsets and recreates its element during lappend, which value does the active operation use and which callbacks survive? This constrains array-element lifetime in the compiler and VM.",
    "active-pending-read-chain": "When a read trace destroys and recreates its variable, do later pending read traces still run? This constrains trace dispatch and variable-cell lifetime in the compiler and VM.",
    "active-recursive-read-write": "What happens when a read trace recursively writes the same variable while lappend is active? This constrains trace dispatch and variable-cell lifetime in the compiler and VM.",
    "trace-lset-recreate-receiver": "Does lset continue with a receiver recreated by its read trace, and what receiver value remains? This constrains variable-cell lifetime in the compiler and VM.",
    "trace-lappend-recreate-receiver": "Does lappend continue with a receiver recreated by its read trace, and what receiver value remains? This constrains variable-cell lifetime in the compiler and VM.",
    "trace-quiet-missing-error-code": "How does zero-argument lappend interact with a potentially missing variable and the existing errorCode? This constrains quiet variable reads in the compiler and VM; the shared event frame may leave the variable present.",
    "trace-quiet-read-callback-error": "How does zero-argument lappend handle an error raised by a variable read trace and the existing errorCode? This constrains quiet reads and error propagation in the compiler and VM.",
    "trace-sequential-output-link-rebind": "Can a trace on the first scan output rebind the second output variable while scan is assigning results? This constrains output-variable lookup and link lifetime in the compiler and VM.",
    "lappend-zero-recreate": "Does zero-argument lappend read a receiver recreated by its read trace, and what value remains? This constrains quiet reads and variable-cell lifetime in the compiler and VM.",
    "scan-rebind-frame-zero": "Can a trace on the first scan output use upvar 0 to rebind the second output in the same event frame? This constrains output-variable lookup and link lifetime in the compiler and VM.",
}
NAME_KINDS = {
    "plain": "an unqualified scalar name",
    "qualified": "a namespace-qualified scalar name",
    "index": "an array element with punctuation in its index",
    "binary_nul": "a dynamically constructed name containing NUL",
    "decomposed": "a dynamically constructed decomposed Unicode name",
}
OPERATIONS = {
    "append": "append",
    "lappend": "lappend",
    "lset": "lset",
    "scan": "scan output assignment",
    "binary_scan": "binary scan output assignment",
    "catch_output": "catch result assignment",
    "empty_lappend": "zero-argument lappend",
    "lassign": "lassign output assignment",
    "failindex": "string is -failindex output assignment",
    "info_default": "info default output assignment",
}


def question(case):
    if case in QUESTIONS:
        return QUESTIONS[case]
    if not case.startswith("generic-"):
        raise ValueError(f"unknown case {case}")
    operation, kind = case.removeprefix("generic-").rsplit("-", 1)
    return (
        f"How does {OPERATIONS[operation]} resolve and update {NAME_KINDS[kind]} "
        "when the command is invoked through a computed command name? "
        "This constrains name resolution and output-variable semantics in the LSP, compiler and VM."
    )


def main():
    parser = tkinter.Tcl()
    installed = (DATA / "appliance/vl1-controls-installed.txt").read_bytes()
    with tarfile.open(DATA / "appliance/r2286vl1-fixtures-v4.tgz") as archive:

        def member(name):
            return archive.extractfile("r2286vl1-fixtures-v4/" + name).read()

        manifest = json.loads(member("manifest.json"))
        wrapper = member("sources/controls.conf")
        scripts = {}
        for row in manifest["cases"]:
            case = row["case"]
            payload = member(f"adapted/{case}.tcl")
            if any(
                payload.hex().encode("ascii") not in source
                for source in (wrapper, installed)
            ):
                raise ValueError(
                    f"payload not present in generated and installed wrapper: {case}"
                )
            scripts[case] = payload
    if len(scripts) != 61 or len(set(scripts.values())) != 61:
        raise ValueError("unexpected lifecycle case set")

    results = {case: {} for case in scripts}
    for port in (42100, 42101, 42102, 42103):
        path = DATA / f"client-server/vl-controls-{port}.body"
        run, group, unit, rows = parser.splitlist(path.read_text())
        if run != "vl1":
            raise ValueError(f"unexpected run in {path}")
        seen = []
        for encoded in parser.splitlist(rows):
            case, code, scan_code, result_hex, scan_result = parser.splitlist(encoded)
            if (
                case not in scripts
                or case in seen
                or scan_code != "0"
                or scan_result != "1"
            ):
                raise ValueError(f"ambiguous result for {case} in {path}")
            seen.append(case)
            results[case][f"{group}:{unit}"] = (code, bytes.fromhex(result_hex), port)
        if seen != list(scripts):
            raise ValueError(f"case ordering differs in {path}")
    for position, (case, script) in enumerate(scripts.items()):
        if set(results[case]) != {"0:0", "0:1", "0:2", "0:3"}:
            raise ValueError(f"incomplete unit coverage: {case}")
        parts = [
            f"# BIG-IP variable lifecycle: {case}\n\n## Question\n\n{question(case)}\n\n",
            (
                "## Assumptions\n\nThe payload was evaluated under catch in an HTTP_REQUEST "
                "event, sequentially with 60 other cases in the same event frame. Earlier "
                "cases can affect later cases, so the results do not assume a fresh interpreter "
                "or unset variables. The script's own nested catch and hex encoding, where "
                "present, remain part of the result. The observations are limited to the "
                "recorded context and inputs.\n\n## Script\n\n"
            ),
            markdown_block(script),
            "\n## Results\n",
        ]
        if position:
            previous = list(scripts)[position - 1]
            parts[1] = parts[1].replace(
                "\n\n## Script\n\n",
                f" The immediately preceding case was [{previous}](bigip-r2286vl1-{previous.replace('_', '-')}.md).\n\n## Script\n\n",
            )
        if case == "trace-quiet-missing-error-code":
            parts[1] = parts[1].replace(
                "The observations are limited",
                "The captured result shows that v was already present; this is not a missing-variable observation. The observations are limited",
            )
        for unit, (code, result, port) in sorted(results[case].items()):
            emit_result(
                parts,
                f"TMM {unit}, HTTP_REQUEST",
                "21.1.0.1 build 0.0.26",
                code,
                result,
                f"The response from source port {port} included this case in the sequential control matrix.",
            )
            if case.startswith("generic-"):
                values = parser.splitlist(result.decode("ascii"))
                if len(values) != 5:
                    raise ValueError(f"unexpected generic result: {case}/{unit}")
                parts.extend(
                    [
                        "\nThe script encoded its inner command result as hex; decoded bytes:\n\n",
                        markdown_block(bytes.fromhex(values[1]), "text-visible"),
                        "\nThe script encoded its final variable-read result as hex; decoded bytes:\n\n",
                        markdown_block(bytes.fromhex(values[4]), "text-visible"),
                    ]
                )
        document = "".join(parts).rstrip() + "\n"
        errors = validate_document(document)
        if errors:
            raise ValueError(f"{case}: {errors}")
        output = ROOT / "evidence/tcl" / f"bigip-r2286vl1-{case.replace('_', '-')}.md"
        if output.exists() and output.read_text() != document:
            raise ValueError(f"refusing to overwrite {output}")
        output.write_text(document)
    print(f"Recovered {len(scripts)} lifecycle scripts across four TMM units")


if __name__ == "__main__":
    main()
