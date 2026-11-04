#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Recover r2286m payloads and their actual BIG-IP responses."""

import base64
import json
import runpy
import tarfile
import tkinter
from collections import Counter, defaultdict
from pathlib import Path

from tcl_evidence_format import markdown_block, validate_document

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006m"
helpers = runpy.run_path(
    str(ROOT / "scripts/dev/migrate-tcl-context-archive-evidence.py")
)
emit_result = helpers["emit_result"]
QUESTIONS = dict(helpers["QUESTIONS"])
QUESTIONS["expressions"] = (
    "How do BIG-IP Tcl contexts evaluate matches and starts_with, including their "
    "precedence relative to not and or for the tested operands? This constrains "
    "expression parsing and evaluation in the LSP, compiler and VM."
)


def main():
    tcl = tkinter.Tcl()
    installed = (DATA / "appliance/raw/config-before-cleanup.txt").read_bytes()
    with tarfile.open(DATA / "fixtures/r2286m-fixtures-v5.tgz") as fixtures:
        root = "r2286m-fixtures-v5"

        def read(name):
            return fixtures.extractfile(f"{root}/{name}").read()

        manifest = json.loads(read("manifest.json"))
        for row in manifest["payloads"]:
            case = row["case"]
            script = read(row["file"])
            config = read(case + ".conf")
            individual = DATA / f"appliance/raw/load/{case}-object.txt"
            observed_config = (
                individual.read_bytes() if individual.exists() else installed
            )
            if (
                script.hex().encode("ascii") not in config
                or script.hex().encode("ascii") not in observed_config
            ):
                raise ValueError(f"installed payload differs for {case}")
            parts = [
                f"# BIG-IP follow-up: {case.replace('_', ' ')}\n\n## Question\n\n{QUESTIONS[case]}\n\n",
                (
                    "## Assumptions\n\nThe retained payload ran under catch in the BIG-IP "
                    "wrappers for the contexts below. Its result is limited to the recorded inputs "
                    "and contexts; it does not establish behavior in another environment. "
                    "Other assumptions were not separately recorded.\n\n## Script\n\n"
                ),
                markdown_block(script),
                "\n## Results\n",
            ]
            groups = defaultdict(list)
            for line in (DATA / f"dev/traffic/{case}.jsonl").read_bytes().splitlines():
                record = json.loads(line)
                if (
                    record.get("phase") != case
                    or len(record.get("reported_units", [])) != 1
                ):
                    raise ValueError(f"ambiguous traffic record for {case}")
                raw = base64.b64decode(record["response_base64"], validate=True)
                _, separator, body = raw.partition(b"\r\n\r\n")
                if not separator:
                    raise ValueError(f"missing HTTP body for {case}")
                code, result_hex = tcl.splitlist(body.decode("ascii"))
                groups[record["reported_units"][0]].append(
                    (record["status"], code, bytes.fromhex(result_hex))
                )
            if set(groups) != {"0:0", "0:1", "0:2", "0:3"}:
                raise ValueError(f"missing TMM responses for {case}")
            for unit, values in sorted(groups.items()):
                counts = Counter(values)
                if len(counts) != 1:
                    raise ValueError(f"inconsistent responses for {case}/{unit}")
                (status, code, result), count = next(iter(counts.items()))
                emit_result(
                    parts,
                    f"TMM {unit}, HTTP_REQUEST",
                    "21.1.0.1 build 0.0.26",
                    code,
                    result,
                    f"The same result appeared in {count} responses with status {status}.",
                )
            if case == "expressions":
                with tarfile.open(DATA / "fixtures/r2286n-contexts.tgz") as contexts:
                    if (
                        contexts.extractfile("r2286n-contexts/payload.tcl").read()
                        != script
                    ):
                        raise ValueError("cross-context expression payload differs")
                    for name in ("cli.conf", "iapp.conf", "icall.conf"):
                        if (
                            script.hex().encode("ascii")
                            not in contexts.extractfile(
                                f"r2286n-contexts/{name}"
                            ).read()
                        ):
                            raise ValueError(f"expression payload missing from {name}")
                seen = set()
                for name in ("cli-run.txt", "scriptd-results.txt"):
                    for line in (
                        (DATA / "appliance/raw/contexts" / name)
                        .read_bytes()
                        .splitlines()
                    ):
                        pos = line.find(b"R2286FOLLOWUP|")
                        if pos < 0:
                            continue
                        fields = line[pos:].decode("ascii").split("|")
                        if fields[1] != "r2286n" or fields[2] in seen:
                            raise ValueError("ambiguous expression context")
                        seen.add(fields[2])
                        values = dict(
                            item.split("=", 1) for item in fields[3:] if "=" in item
                        )
                        emit_result(
                            parts,
                            fields[2],
                            "21.1.0.1 build 0.0.26",
                            values["rc"],
                            bytes.fromhex(values["result_hex"]),
                            "This result was decoded from the retained context marker.",
                        )
                if seen != {"TmshCliScript", "IAppImplementation", "ICallScript"}:
                    raise ValueError("incomplete expression contexts")
            document = "".join(parts).rstrip() + "\n"
            errors = validate_document(document)
            if errors:
                raise ValueError(f"{case}: {errors}")
            output = ROOT / "evidence/tcl" / f"bigip-r2286m-{case.replace('_', '-')}.md"
            if output.exists() and output.read_text() != document:
                raise ValueError(f"refusing to overwrite {output}")
            output.write_text(document)
    print(f"Recovered {len(manifest['payloads'])} BIG-IP follow-up payloads")


if __name__ == "__main__":
    main()
