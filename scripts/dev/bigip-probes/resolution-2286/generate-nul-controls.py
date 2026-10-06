#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Generate isolated literal and dynamically materialized embedded-NUL controls."""

import argparse
import hashlib
import json
import re
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run", required=True)
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()
    if not re.fullmatch(r"[A-Za-z][A-Za-z0-9_]{0,23}", args.run):
        parser.error("run must match [A-Za-z][A-Za-z0-9_]{0,23}")
    args.out.mkdir(parents=True, exist_ok=False)
    prefix = f"__tcl_lsp_probe_2286_{args.run}"
    fixtures = []
    rows = []

    def emit(case: str, data: bytes, purpose: str) -> None:
        filename = case + ".conf"
        object_name = f"/Common/{prefix}_{case}"
        (args.out / filename).write_bytes(data)
        (args.out / (filename + ".hex")).write_text(data.hex() + "\n", encoding="ascii")
        fixtures.append(
            {
                "case": case,
                "object": object_name,
                "file": filename,
                "sha256": hashlib.sha256(data).hexdigest(),
                "size": len(data),
                "line_endings": "lf",
                "purpose": purpose,
            }
        )
        rows.append(f"{filename}\t{object_name}")

    dynamic = f"""ltm rule /Common/{prefix}_dynamic_embedded_nul {{
when HTTP_REQUEST {{
    set u "[TMM::cmp_group]:[TMM::cmp_unit]"
    set name [binary format H* 410042]
    binary scan $name H* name_hex
    set set_rc [catch {{set $name VALUE}} set_result]
    set read_rc [catch {{set $name}} read_result]
    set exists [info exists $name]
    set out [list [string length $name] $name_hex $set_rc $set_result $read_rc $read_result $exists]
    binary scan $out H* out_hex
    HTTP::respond 200 content "$out_hex\\n" X-R2286-TMM $u Connection close
}}
}}
""".encode("ascii")
    emit(
        "dynamic_embedded_nul",
        dynamic,
        "runtime materialization and use of the byte sequence 41 00 42",
    )

    literal = (
        f"""ltm rule /Common/{prefix}_literal_embedded_nul {{
when HTTP_REQUEST {{
    set x "A""".encode("ascii")
        + b"\x00"
        + f"""B"
    HTTP::respond 200 content "$x\\n" Connection close
}}
}}
""".encode("ascii")
    )
    emit(
        "literal_embedded_nul",
        literal,
        "literal 00 byte between ASCII A and B in the config source",
    )

    manifest = {
        "run": args.run,
        "kind": "additional_embedded_nul_controls",
        "expected_appliance_results": "UNMEASURED",
        "fixtures": fixtures,
    }
    (args.out / "manifest.json").write_text(
        json.dumps(manifest, indent=2) + "\n", encoding="ascii"
    )
    (args.out / "rules.tsv").write_text("\n".join(rows) + "\n", encoding="ascii")


if __name__ == "__main__":
    main()
