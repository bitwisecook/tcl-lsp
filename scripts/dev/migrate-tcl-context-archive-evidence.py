#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Recover the r2286p cross-context BIG-IP Tcl payload observations."""

from __future__ import annotations

import base64
import json
import re
import tarfile
import tkinter
from collections import Counter, defaultdict
from pathlib import Path

from tcl_evidence_format import markdown_block, validate_document

ROOT = Path(__file__).resolve().parents[2]
ARCHIVES = (
    ROOT / "scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006naming"
)
QUESTIONS = {
    "nul_counted_last": "When a variable name containing an embedded NUL is written after its shorter prefix, do the two names refer to distinct cells through reads, upvar and unset? This affects variable-name resolution in the LSP, compiler and VM.",
    "nul_plain_last": "When a shorter variable name is written after the same prefix followed by an embedded NUL, do the two names refer to distinct cells through reads, upvar and unset? This affects variable-name resolution in the LSP, compiler and VM.",
    "nul_plain_unset_first": "When the shorter variable name is unset before the same prefix followed by an embedded NUL, does the counted name remain a separate cell? This affects variable-name resolution in the LSP, compiler and VM.",
    "nul_array_root": "Does an embedded NUL distinguish an array root from its shorter prefix through writes, reads, links and unsets? This affects array-root resolution in the LSP, compiler and VM.",
    "nul_array_index": "Does an embedded NUL distinguish an array index from its shorter prefix through writes, reads, links and unsets? This affects array-element resolution in the LSP, compiler and VM.",
    "unicode_format_forward": "Do precomposed and decomposed variable names produced by format %c remain distinct when written in this order? This affects Unicode name resolution in the LSP, compiler and VM.",
    "unicode_format_reverse": "Do precomposed and decomposed variable names produced by format %c remain distinct when written in the reverse order? This affects Unicode name resolution in the LSP, compiler and VM.",
    "unicode_bytes_forward": "Do precomposed and decomposed variable names produced from binary bytes remain distinct when written in this order? This affects Unicode name resolution in the LSP, compiler and VM.",
    "unicode_bytes_reverse": "Do precomposed and decomposed variable names produced from binary bytes remain distinct when written in the reverse order? This affects Unicode name resolution in the LSP, compiler and VM.",
    "unicode_cross_producer": "Do variable names produced by format %c and by binary format reach the same or different cells for the tested character values? This affects the LSP, compiler and VM's treatment of name producers.",
    "commands_format": "How are precomposed and decomposed command names produced by format %c created and resolved? This affects command-name modeling in the LSP, compiler and VM.",
    "commands_bytes": "How are precomposed and decomposed command names produced from binary bytes created and resolved? This affects command-name modeling in the LSP, compiler and VM.",
    "commands_cross_producer": "Do command names produced by format %c and binary format resolve to the same or different commands for the tested values? This affects command-name modeling in the LSP, compiler and VM.",
    "lexical_format_unbraced": "How does Tcl parse an unbraced variable token containing name characters produced by format %c? This affects lexical name recognition in the LSP and compiler.",
    "lexical_format_braced": "How does Tcl parse a braced variable token containing name characters produced by format %c? This affects lexical name recognition in the LSP and compiler.",
    "lexical_bytes_unbraced": "How does Tcl parse an unbraced variable token containing name bytes produced by binary format? This affects lexical name recognition in the LSP and compiler.",
    "lexical_bytes_braced": "How does Tcl parse a braced variable token containing name bytes produced by binary format? This affects lexical name recognition in the LSP and compiler.",
    "lexical_clean": "How do the tested format and binary byte producers affect braced and unbraced variable-token parsing when the source is constructed dynamically? This affects lexical name recognition in the LSP and compiler.",
    "qualified_nul": "Do qualified scalar and array names containing embedded NUL bytes reach distinct variables through reads, writes and traces? This affects qualified-name resolution in the LSP, compiler and VM.",
    "environment": "Which Tcl version, namespace and command capabilities are reported in the measured BIG-IP context, so that other naming observations can be scoped correctly?",
}


def read_member(archive: tarfile.TarFile, name: str) -> bytes:
    item = archive.getmember(name)
    if not item.isfile():
        raise ValueError(f"archive member is not a file: {name}")
    stream = archive.extractfile(item)
    if stream is None:
        raise ValueError(f"cannot read {name}")
    return stream.read()


def emit_result(
    parts: list[str], label: str, version: str, code: str, result: bytes, note: str
) -> None:
    version_label = f"BIG-IP {version}" if version != "Not recorded" else version
    parts.extend(
        [
            f"\n### {label}\n\n",
            f"- Recorded version: {version_label}; Tcl patchlevel Not recorded in this run's result.\n",
            f"- Recorded input channel: {label}.\n",
            "- Exit status: Not recorded.\n- Standard output: Not recorded.\n- Standard error: Not recorded.\n",
            "- Recorded result:\n\n",
            markdown_block(result, "text-visible"),
            f"\nThe wrapper recorded Tcl completion code {code}. {note}\n",
        ]
    )


def main() -> None:
    with (
        tarfile.open(ARCHIVES / "r2286p-fixtures-v2.tgz") as fixtures,
        tarfile.open(ARCHIVES / "r2286p-appliance-evidence.tgz") as appliance,
        tarfile.open(ARCHIVES / "r2286p-dev-evidence.tgz") as traffic,
    ):
        fixture_root = "r2286p-fixtures-v2"
        raw_root = "r2286p-evidence/appliance/raw"
        traffic_root = "r2286p-dev-evidence"
        manifest = json.loads(read_member(fixtures, f"{fixture_root}/manifest.json"))
        context_runs = {row["case"]: row["context_run"] for row in manifest["contexts"]}
        version_text = read_member(appliance, f"{raw_root}/version-final.txt").decode(
            "ascii"
        )
        version = re.search(r"(?m)^\s*Version\s+(\S+)", version_text)
        build = re.search(r"(?m)^\s*Build\s+(\S+)", version_text)
        if not version or not build:
            raise ValueError("appliance version or build was not recorded")
        label_version = f"{version[1]} build {build[1]}"
        tcl = tkinter.Tcl()
        for row in manifest["payloads"]:
            identity = row["case"]
            if identity not in QUESTIONS:
                raise ValueError(f"missing semantic question for {identity}")
            script = read_member(fixtures, f"{fixture_root}/{row['file']}")
            payload = read_member(
                fixtures, f"{fixture_root}/contexts/{identity}/payload.tcl"
            )
            if script != payload:
                raise ValueError(f"context payload differs for {identity}")
            for wrapper in ("cli.conf", "iapp.conf", "icall.conf"):
                data = read_member(
                    fixtures, f"{fixture_root}/contexts/{identity}/{wrapper}"
                )
                if script.hex().encode("ascii") not in data:
                    raise ValueError(f"payload missing from {identity}/{wrapper}")
            http_wrapper = read_member(
                fixtures, f"{fixture_root}/{row['file'][:-4]}.conf"
            )
            if script.hex().encode("ascii") not in http_wrapper:
                raise ValueError(f"payload missing from {identity} HTTP wrapper")
            parts = [
                f"# BIG-IP cross-context naming: {identity.replace('_', ' ')}\n\n",
                f"## Question\n\n{QUESTIONS[identity]}\n\n",
                (
                    "## Assumptions\n\nThe retained payload was evaluated under catch "
                    "inside context-specific wrappers. Result bytes below are the "
                    "wrapper's recorded Tcl result, rather than standalone process "
                    "stdout. A result in one context does not establish another "
                    "context or version. Other assumptions were not separately recorded.\n\n"
                ),
                "## Script\n\n",
                markdown_block(script),
                "\n## Results\n",
            ]
            by_unit: dict[str, list[tuple[str, str, bytes]]] = defaultdict(list)
            traffic_data = read_member(
                traffic, f"{traffic_root}/traffic-http-{identity}.jsonl"
            )
            for line in traffic_data.splitlines():
                record = json.loads(line)
                if "error" in record or len(record.get("reported_units", [])) != 1:
                    raise ValueError(f"incomplete HTTP result for {identity}")
                raw = base64.b64decode(record["response_base64"], validate=True)
                _, separator, body = raw.partition(b"\r\n\r\n")
                if not separator:
                    raise ValueError(f"missing HTTP body for {identity}")
                words = tcl.splitlist(body.decode("ascii"))
                if len(words) != 2:
                    raise ValueError(f"unexpected HTTP result for {identity}")
                by_unit[record["reported_units"][0]].append(
                    (str(record["status"]), words[0], bytes.fromhex(words[1]))
                )
            for unit, values in sorted(by_unit.items()):
                counts = Counter(values)
                if len(counts) != 1:
                    raise ValueError(f"multiple HTTP results for {identity} on {unit}")
                (status, code, result), count = next(iter(counts.items()))
                emit_result(
                    parts,
                    f"TMM {unit}, HTTP_REQUEST",
                    label_version,
                    code,
                    result,
                    f"The same result was recorded in {count} responses with status {status}.",
                )
            contexts: dict[str, tuple[str, bytes]] = {}
            for name in ("cli-run.txt", "scriptd-results.txt"):
                data = read_member(appliance, f"{raw_root}/contexts/{identity}/{name}")
                for line in data.splitlines():
                    offset = line.find(b"R2286FOLLOWUP|")
                    if offset < 0:
                        continue
                    fields = line[offset:].decode("ascii").split("|")
                    if len(fields) < 4 or fields[1] != context_runs[identity]:
                        raise ValueError(f"ambiguous context marker for {identity}")
                    values = dict(
                        item.split("=", 1) for item in fields[3:] if "=" in item
                    )
                    result = (values["rc"], bytes.fromhex(values["result_hex"]))
                    if fields[2] in contexts:
                        raise ValueError(f"duplicate {fields[2]} marker for {identity}")
                    contexts[fields[2]] = result
            if set(contexts) != {"TmshCliScript", "IAppImplementation", "ICallScript"}:
                raise ValueError(f"missing context marker for {identity}")
            for context, (code, result) in sorted(contexts.items()):
                emit_result(
                    parts,
                    context,
                    label_version,
                    code,
                    result,
                    "This result was decoded from the retained context marker.",
                )
            document = "".join(parts).rstrip() + "\n"
            errors = validate_document(document)
            if errors:
                raise ValueError(f"{identity}: {errors}")
            path = (
                ROOT / "evidence/tcl" / f"bigip-r2286p-{identity.replace('_', '-')}.md"
            )
            if path.exists() and path.read_text() != document:
                raise ValueError(f"refusing to overwrite {path}")
            path.write_text(document)
    print(f"Recovered {len(manifest['payloads'])} cross-context BIG-IP payloads")


if __name__ == "__main__":
    main()
