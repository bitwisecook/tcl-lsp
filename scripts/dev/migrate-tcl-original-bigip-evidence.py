#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Recover executed runtime payloads from the original r2286c appliance run."""

import collections
import re
import tarfile
import tkinter
from pathlib import Path

from tcl_evidence_format import markdown_block, validate_document

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006c/dev"
QUESTIONS = {
    "dynamic_word_bytes": "How are backslash-newline continuations and a literal backslash-n represented in braced and quoted words? This constrains word parsing and literal-value preservation in the LSP and compiler.",
    "dynamic_expr": "How does the runtime evaluate floating-point literals, shifts, math functions and an unavailable function? This constrains expression evaluation and error modeling in the compiler and VM.",
    "dynamic_exists": "How do info exists and array exists report a missing local name? This constrains variable existence analysis in the LSP, compiler and VM.",
    "dynamic_read_failure": "Does reading a missing variable abort evaluation before a later assignment, and what error is caught? This constrains control flow and error modeling in the LSP, compiler and VM.",
    "dynamic_colon_names": "Do qualified variable references with two versus three consecutive colons resolve to the same value in this namespace? This constrains qualified-name parsing in the LSP, compiler and VM.",
    "dynamic_package_resolution": "Does package ifneeded register a script that package require executes to provide the requested version? This constrains package discovery and evaluation modeling in the LSP and runtime.",
    "dynamic_package": "What do package provide Tcl and package require Tcl report in this context? This scopes package modeling in the LSP and runtime.",
    "dynamic_trace_static": "What callbacks occur when a static variable is read, unset and recreated after a legacy trace is installed? This constrains variable lifetime and trace dispatch in the compiler and VM.",
    "dynamic_trace_upvar_array": "What trace callbacks and values result when an array element is read through upvar #0 across repeated requests? This constrains array identity, links and trace dispatch in the compiler and VM.",
    "dynamic_trace_scalar": "What callbacks occur when a global scalar is read, written, unset and recreated after a legacy trace is installed? This constrains variable lifetime and trace dispatch in the compiler and VM.",
    "dynamic_namespace_delete": "Does a computed qualified command name resolve to the newly created procedure after its namespace is deleted and recreated? This constrains command identity and invalidation in the LSP, compiler and VM.",
    "dynamic_namespace_import": "Can a consumer import and call an exported namespace command, and what happens on repeated requests? This constrains imported command bindings in the LSP, compiler and VM.",
    "dynamic_rename_epoch": "Does a computed command name resolve to a replacement after the original command is renamed, and can the moved command still be invoked? This constrains binding invalidation in the LSP, compiler and VM.",
    "dynamic_alias_rename": "Does an interpreter alias continue to invoke its original target after that target is renamed? This constrains alias target resolution in the LSP, compiler and VM.",
    "dynamic_alias": "Does an interpreter alias supply its fixed argument to the target procedure? This constrains alias prefix and call modeling in the LSP, compiler and VM.",
    "dynamic_static_global_link": "Can a procedure access a static namespace variable through global using its unqualified local name? This constrains variable links and BIG-IP static storage in the LSP, compiler and VM.",
    "dynamic_uplevel_absolute": "Does uplevel #0 evaluate an assignment in the global frame? This constrains frame-sensitive evaluation in the LSP, compiler and VM.",
    "dynamic_upvar_zero": "Does upvar 0 link two local names to the same variable in the current frame? This constrains frame-sensitive variable links in the LSP, compiler and VM.",
    "dynamic_upvar_absolute": "Does upvar #0 link a procedure-local name to the selected global variable? This constrains variable links and frame resolution in the LSP, compiler and VM.",
    "dynamic_uplevel": "Does uplevel 1 modify the caller's local variable? This constrains caller-frame evaluation in the LSP, compiler and VM.",
    "dynamic_upvar_unset": "After upvar 1 links to a caller variable, does unsetting and recreating the link update the caller's variable? This constrains link and variable-cell lifetime in the LSP, compiler and VM.",
    "dynamic_upvar": "Does upvar 1 link a callee's local name to its caller's variable? This constrains caller-frame variable links in the LSP, compiler and VM.",
    "dynamic_global_alias": "Does global link a procedure-local name to the selected global variable? This constrains variable binding in the LSP, compiler and VM.",
    "dynamic_path_provider": "Does namespace path select the provider's command before the same-named global command? This constrains command lookup order in the LSP, compiler and VM.",
    "dynamic_path_shadow": "Does a namespace-local command take precedence over the same name in its configured namespace path? This constrains command lookup order in the LSP, compiler and VM.",
    "dynamic_namespace": "Does a namespace procedure resolve its namespace variable through variable? This constrains namespace-variable binding in the LSP, compiler and VM.",
}


def source_from_config(tcl, data):
    line = next(
        line for line in data.splitlines() if line.lstrip().startswith(b"set script ")
    )
    words = tcl.splitlist(line.decode("utf-8"))
    if len(words) != 3:
        raise ValueError("unexpected payload assignment")
    return words[2].encode("utf-8")


def main():
    tcl = tkinter.Tcl()
    with (
        tarfile.open(DATA / "artifacts/r2286_20261006c-upload.tgz") as fixture,
        tarfile.open(DATA / "r2286_20261006c-appliance.tgz") as capture,
    ):
        statuses = dict(
            line.split("\t")
            for line in capture.extractfile("load-driver-summary.tsv")
            .read()
            .decode()
            .splitlines()
        )
        lines = capture.extractfile("ltm-raw.log").read().splitlines()
        for case, question in QUESTIONS.items():
            source = fixture.extractfile(
                f"r2286_20261006c-fixtures/{case}.runtime.tcl"
            ).read()
            config = fixture.extractfile(f"r2286_20261006c-fixtures/{case}.conf").read()
            installed = capture.extractfile(
                f"journals/{case}/created-{case}.conf"
            ).read()
            if statuses.get(case) != "0" or any(
                source_from_config(tcl, x) != source for x in (config, installed)
            ):
                raise ValueError(f"unverified installed source for {case}")
            marker = f"R2286|r2286_20261006c|{case}|".encode()
            pattern = re.compile(
                re.escape(marker)
                + rb"unit=(\d+)\|rc=(\d+)\|result_hex=([0-9a-f]*)\|errorcode_hex=([0-9a-f]*)\|"
            )
            complete = collections.Counter()
            incomplete = collections.Counter()
            for line in lines:
                if marker not in line:
                    continue
                match = pattern.search(line)
                if match:
                    complete[match.groups()] += 1
                else:
                    incomplete[line[line.index(marker) :]] += 1
            if not complete or (incomplete and case != "dynamic_trace_upvar_array"):
                raise ValueError(f"missing or unexpected incomplete result for {case}")
            parts = [
                f"# BIG-IP original runtime: {case.replace('_', ' ')}\n\n## Question\n\n{question}\n\n",
                (
                    "## Assumptions\n\nThe installed HTTP_REQUEST wrapper evaluated this exact script under catch. "
                    "Repeated requests can share interpreter state, so later results need not match earlier results; "
                    "counts are captured log lines, not proof of isolated invocations. The wrapper's errorCode "
                    "field can reflect existing interpreter state. Results are limited to this context and inputs.\n\n## Script\n\n"
                ),
                markdown_block(source),
                "\n## Results\n",
            ]
            for index, ((unit, code, result_hex, error_hex), count) in enumerate(
                sorted(complete.items()), 1
            ):
                result, error = (
                    bytes.fromhex(result_hex.decode()),
                    bytes.fromhex(error_hex.decode()),
                )
                parts.extend(
                    [
                        f"\n### TMM unit {unit.decode()}, captured result {index}\n\n",
                        "- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.\n",
                        "- Recorded input channel: HTTP_REQUEST.\n- Exit status: Not recorded.\n- Standard output: Not recorded.\n- Standard error: Not recorded.\n- Recorded result:\n\n",
                        markdown_block(result, "text-visible"),
                        f"\nThe wrapper recorded Tcl completion code {code.decode()} in {count} log lines. Its recorded errorCode field was:\n\n",
                        markdown_block(error, "text-visible"),
                    ]
                )
            for index, (fragment, count) in enumerate(sorted(incomplete.items()), 1):
                parts.extend(
                    [
                        f"\n### Truncated HTTP_REQUEST log fragment {index}\n\n",
                        "- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.\n",
                        "- Recorded input channel: HTTP_REQUEST.\n- Exit status: Not recorded.\n- Standard output: Not recorded.\n- Standard error: Not recorded.\n- Recorded result:\n\n",
                        markdown_block(fragment, "text-visible"),
                        f"\nThis incomplete marker appeared in {count} log lines. The result and errorCode are truncated; the marker is shown verbatim and does not establish the missing bytes.\n",
                    ]
                )
            document = "".join(parts).rstrip() + "\n"
            if errors := validate_document(document):
                raise ValueError(f"{case}: {errors}")
            output = ROOT / "evidence/tcl" / f"bigip-r2286c-{case.replace('_', '-')}.md"
            if output.exists() and output.read_text() != document:
                raise ValueError(f"refusing to overwrite {output}")
            output.write_text(document)
    print(f"Recovered {len(QUESTIONS)} original BIG-IP runtime payloads")


if __name__ == "__main__":
    main()
