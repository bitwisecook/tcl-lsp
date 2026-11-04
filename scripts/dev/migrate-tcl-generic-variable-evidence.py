#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Recover generic variable-operand and trace shell executions."""

import json
import runpy
import tkinter
from pathlib import Path

from tcl_evidence_format import markdown_block, validate_document

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "rust/tcl-syntax/tests/data/native_generic_variable_consumers"
lifecycle = runpy.run_path(str(ROOT / "scripts/dev/migrate-tcl-lifecycle-evidence.py"))
OPERATIONS = lifecycle["OPERATIONS"]
NAME_KINDS = lifecycle["NAME_KINDS"]
TRACE_QUESTIONS = {
    "quiet-read-callback-error": "How does zero-argument lappend handle a read-trace error and the existing errorCode? This constrains quiet reads and error propagation in the compiler and VM.",
    "lappend-recreate-receiver": "Does lappend continue with a receiver recreated by its read trace, and what value remains? This constrains variable-cell lifetime in the compiler and VM.",
    "lset-recreate-receiver": "Does lset continue with a receiver recreated by its read trace, and what value remains? This constrains variable-cell lifetime in the compiler and VM.",
    "sequential-output-link-rebind": "Can a trace on the first scan output rebind the second output variable while scan is assigning results? This constrains output-variable lookup and links in the compiler and VM.",
    "qualified-index-trace-report": "What variable and index names are reported to a trace on a qualified array element accessed by lappend? This constrains array-name resolution and trace dispatch in the LSP, compiler and VM.",
}


def question(case, trace):
    if trace:
        modern = case.startswith("modern-")
        key = case.removeprefix("modern-")
        return TRACE_QUESTIONS[key] + (
            " This script uses trace add/remove."
            if modern
            else " This script uses the legacy trace interface."
        )
    operation, kind = case.rsplit("-", 1)
    return (
        f"How does {OPERATIONS[operation]} resolve and update {NAME_KINDS[kind]} "
        "when the command is invoked through a computed command name? "
        "This constrains name resolution and output-variable semantics in the LSP, compiler and VM."
    )


def main():
    parser = tkinter.Tcl()
    existing = (
        ROOT / "evidence/tcl/naming-variable-generic-trace-quiet-missing-error-code.md"
    )
    existing_source = (DATA / "trace-quiet-missing-error-code.tcl").read_bytes()
    # This separately migrated script already contains the six recorded runs.
    from tcl_evidence_format import script_from_markdown

    if script_from_markdown(existing.read_text()) != existing_source:
        raise ValueError("previously migrated quiet-read source differs")
    written = 0
    for manifest_name, prefix, trace in (
        ("manifest.json", "", False),
        ("trace-manifest.json", "trace-", True),
    ):
        manifest = json.loads((DATA / manifest_name).read_bytes())
        engines = manifest["engines"]
        cases = [row["case"] for row in engines[0]["cases"]]
        if any([row["case"] for row in engine["cases"]] != cases for engine in engines):
            raise ValueError(f"case sets differ in {manifest_name}")
        for case in cases:
            if trace and case == "quiet-missing-error-code":
                continue
            name = prefix + case
            script = (DATA / f"{name}.tcl").read_bytes()
            parts = [
                f"# Tcl variable operand: {name.replace('_', ' ').replace('-', ' ')}\n\n## Question\n\n{question(case, trace)}\n\n",
                (
                    "## Assumptions\n\nThe recorded programs use a computed command name and "
                    "the name values constructed by the script. Binary-NUL and decomposed "
                    "names are produced at runtime, not literal non-ASCII source bytes. "
                    "The script catches some Tcl errors and encodes result bytes as hex; "
                    "those caught errors are distinct from the shell's exit status. "
                    "The capture records launcher arguments but does not separately identify "
                    "the script input channel. Other assumptions were not separately recorded.\n\n## Script\n\n"
                ),
                markdown_block(script),
                "\n## Results\n",
            ]
            for engine in engines:
                rows = [row for row in engine["cases"] if row["case"] == case]
                if len(rows) != 1 or type(rows[0].get("exit")) is not int:
                    raise ValueError(
                        f"ambiguous process status for {engine['engine']}/{name}"
                    )
                label = engine["engine"]
                version = (DATA / f"{label}-version.stdout").read_text().strip()
                stdout = (DATA / f"{label}-{name}.stdout").read_bytes()
                stderr = (DATA / f"{label}-{name}.stderr").read_bytes()
                argv = rows[0].get("argv", [])
                if len(argv) != 1:
                    raise ValueError(
                        f"unexpected launcher arguments for {label}/{name}"
                    )
                parts.extend(
                    [
                        f"\n### {label}\n\n- Recorded version: {version}.\n",
                        f"- Recorded input channel: Not separately recorded; launcher `{argv[0]}`.\n",
                        f"- Exit status: {rows[0]['exit']}.\n- Standard output:\n\n",
                        markdown_block(stdout, "text-visible"),
                        "\n- Standard error: "
                        + (
                            "empty.\n"
                            if not stderr
                            else "\n\n" + markdown_block(stderr, "text-visible")
                        ),
                    ]
                )
                fields = parser.splitlist(stdout.decode("ascii").strip())
                expected = 2 if trace else 5
                if len(fields) != expected:
                    raise ValueError(f"unexpected output shape for {label}/{name}")
                parts.extend(
                    [
                        "\nThe script's hex-encoded inner command result, decoded for reading:\n\n",
                        markdown_block(bytes.fromhex(fields[1]), "text-visible"),
                    ]
                )
                if not trace:
                    parts.extend(
                        [
                            "\nThe script's hex-encoded final variable-read result, decoded for reading:\n\n",
                            markdown_block(bytes.fromhex(fields[4]), "text-visible"),
                        ]
                    )
            document = "".join(parts).rstrip() + "\n"
            if errors := validate_document(document):
                raise ValueError(f"{name}: {errors}")
            output = (
                ROOT / "evidence/tcl" / f"native-generic-{name.replace('_', '-')}.md"
            )
            if output.exists() and output.read_text() != document:
                raise ValueError(f"refusing to overwrite {output}")
            output.write_text(document)
            written += 1
    print(f"Recovered {written} distinct generic variable and trace scripts")


if __name__ == "__main__":
    main()
