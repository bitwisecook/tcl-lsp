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

"""Generate an ASCII-only exact-name runtime cleanup iRule for a probe run."""

import argparse
import hashlib
import json
import re
from pathlib import Path


def quote(value: str) -> str:
    return (
        '"'
        + value.translate(
            str.maketrans(
                {
                    "\\": "\\\\",
                    '"': '\\"',
                    "$": "\\$",
                    "[": "\\[",
                    "]": "\\]",
                    "{": "\\{",
                    "}": "\\}",
                }
            )
        )
        + '"'
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run", required=True)
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()
    if not re.fullmatch(r"[A-Za-z][A-Za-z0-9_]{0,23}", args.run):
        parser.error("run must match [A-Za-z][A-Za-z0-9_]{0,23}")
    args.out.mkdir(parents=True, exist_ok=False)

    ns = f"__tcl_lsp_probe_2286_{args.run}"
    prefix = ns + "_"
    object_name = f"/Common/{ns}_runtime_cleanup_ascii"
    variables = ["::" + prefix + x for x in ["g", "events", "x", "arr"]]
    variables += ["static::" + prefix + x for x in ["events", "x"]]
    variables += [
        "static::" + ns + "_cell",
        "::" + ns + "_cell",
        "static::" + ns + "_collision",
    ]
    commands = [
        prefix + x
        for x in [
            "p",
            "read",
            "inner",
            "outer",
            "one",
            "alias",
            "moved",
            "trace",
            "failure",
            "test",
            "literal",
            "pick",
            "mark",
        ]
    ]
    namespaces = ["::" + ns, "::" + ns + "_consumer", "::" + ns + "_provider"]

    lines = ["set outcomes {}"]
    for name in variables:
        lines.append(
            f"lappend outcomes [list unset {name} [catch {{unset {name}}} e] $e]"
        )
    for name in commands:
        lines.append(
            f"lappend outcomes [list rename {name} [catch {{rename {name} {{}}}} e] $e]"
        )
    for name in namespaces:
        lines.append(
            f"lappend outcomes [list namespace {name} [catch {{namespace delete {name}}} e] $e]"
        )
    lines.append(
        f"lappend outcomes [list package [catch {{package forget {prefix}package}} e] $e]"
    )
    lines.append("set outcomes")
    payload = "\n".join(lines)
    body = f"""ltm rule {object_name} {{
when HTTP_REQUEST {{
    set u "[TMM::cmp_group]:[TMM::cmp_unit]"
    set script {quote(payload)}
    set rc [catch {{eval $script}} result]
    binary scan $result H* hx
    log local0. "R2286|{args.run}|runtime_cleanup_ascii|tmm=$u|rc=$rc|result_hex=$hx"
    HTTP::respond 200 content "$rc $hx\\n" X-R2286-TMM $u Connection close
}}
}}
""".encode("ascii")
    filename = "runtime_cleanup_ascii.conf"
    (args.out / filename).write_bytes(body)
    (args.out / f"{filename}.hex").write_text(body.hex() + "\n", encoding="ascii")
    digest = hashlib.sha256(body).hexdigest()
    manifest = {
        "run": args.run,
        "kind": "additional_ascii_runtime_cleanup",
        "fixtures": [
            {
                "case": "runtime_cleanup_ascii",
                "object": object_name,
                "file": filename,
                "sha256": digest,
                "size": len(body),
                "line_endings": "lf",
            }
        ],
        "excluded": "Unicode-name cleanup is excluded because those supplied outer iRules reject at load on the tested appliance.",
    }
    (args.out / "manifest.json").write_text(
        json.dumps(manifest, indent=2) + "\n", encoding="ascii"
    )
    (args.out / "rules.tsv").write_text(
        f"{filename}\t{object_name}\n", encoding="ascii"
    )


if __name__ == "__main__":
    main()
