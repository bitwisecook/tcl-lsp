#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Generate isolated BIG-IP loader controls for Unicode and physical lines."""

import argparse
import hashlib
import json
import re
from pathlib import Path


def main():
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

    def emit(case, body, purpose, line_endings="lf"):
        object_name = f"/Common/{prefix}_{case}"
        data = f"ltm rule {object_name} {{\n{body}\n}}\n".encode("utf-8")
        if line_endings == "crlf":
            data = data.replace(b"\n", b"\r\n")
        filename = f"{case}.conf"
        (args.out / filename).write_bytes(data)
        (args.out / f"{filename}.hex").write_text(data.hex() + "\n", encoding="ascii")
        fixtures.append(
            {
                "case": case,
                "object": object_name,
                "file": filename,
                "sha256": hashlib.sha256(data).hexdigest(),
                "size": len(data),
                "line_endings": line_endings,
                "purpose": purpose,
            }
        )
        rows.append(f"{filename}\t{object_name}")

    emit(
        "ascii_binary",
        f'''when HTTP_REQUEST {{
    set x "A|e"
    binary scan $x H* hx
    log local0. "R2286|{args.run}|ascii_binary|hex=$hx"
    HTTP::respond 200 content "$hx\\n" Connection close
}}''',
        "ASCII control for the binary scan form used by rejected supplied fixtures",
    )
    emit(
        "unicode_precomposed",
        f'''when HTTP_REQUEST {{
    set x "\u00e9"
    log local0. "R2286|{args.run}|unicode_precomposed|value=$x"
    HTTP::respond 200 content "$x\\n" Connection close
}}''',
        "literal precomposed U+00E9 without binary scan",
    )
    emit(
        "unicode_decomposed",
        f'''when HTTP_REQUEST {{
    set x "e\u0301"
    log local0. "R2286|{args.run}|unicode_decomposed|value=$x"
    HTTP::respond 200 content "$x\\n" Connection close
}}''',
        "literal U+0065 U+0301 without binary scan",
    )

    physical = f'''when HTTP_REQUEST {{
    set brace {{A\\
  B}}
    set quote "A\\
  B"
    set literal {{\\n}}
    set out [list $brace $quote $literal]
    log local0. "R2286|{args.run}|physical_ascii|value=$out"
    HTTP::respond 200 content "$out\\n" Connection close
}}'''
    emit(
        "physical_ascii_lf",
        physical,
        "physical backslash-LF and literal backslash-n without Unicode or binary scan",
    )
    emit(
        "physical_ascii_crlf",
        physical,
        "physical backslash-CRLF and literal backslash-n without Unicode or binary scan",
        line_endings="crlf",
    )

    assert "\u00e9".encode() == bytes.fromhex("c3a9")
    assert "e\u0301".encode() == bytes.fromhex("65cc81")
    manifest = {
        "run": args.run,
        "kind": "additional_loader_controls",
        "encoding": "UTF-8",
        "expected_appliance_results": "UNMEASURED",
        "fixtures": fixtures,
    }
    (args.out / "manifest.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    (args.out / "rules.tsv").write_text("\n".join(rows) + "\n", encoding="utf-8")
    print(f"Generated {len(fixtures)} loader controls in {args.out}")


if __name__ == "__main__":
    main()
