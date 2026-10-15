#!/usr/bin/env python3
"""Extract measured bytes and verify their exact original source declaration slices."""
import argparse
import ast
import hashlib
import json
from pathlib import Path
import re


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--provider", required=True, type=Path)
    args = parser.parse_args()
    output = Path(__file__).resolve().parent
    provider = args.provider.resolve()
    manifest = json.loads((output / "manifest.json").read_text())
    for name in ["_nshelper.c", "nshelper.tcl"]:
        assert sha((provider / name).read_bytes()) == manifest["provider_before_sha256"][name]
    generated = (provider / "_nshelper.c").read_text()
    call = generated.split('return Jim_EvalSource(interp, "nshelper.tcl", 1, ', 1)[1].split(');', 1)[0]
    # Decode original C literal storage; do not recreate the original comment stripper.
    static = "".join(ast.literal_eval(value) for value in re.findall(r'"(?:[^"\\]|\\.)*"', call)).encode()
    raw = (provider / "nshelper.tcl").read_bytes()
    (output / "static-evaluated-source.tcl").write_bytes(static)
    (output / "raw-nshelper-source.tcl").write_bytes(raw)
    rows = [line.split("|") for line in (output / "probe.stdout").read_text().splitlines()]
    index = {}
    for purpose in ["STATIC", "RAW_AFTER_STATIC", "RAW_ONLY"]:
        directory = output / purpose.lower()
        directory.mkdir(exist_ok=True)
        source = static if purpose == "STATIC" else raw
        index[purpose] = []
        for ordinal in range(9):
            key = [purpose, str(ordinal)]
            observed = next(row for row in rows if row[:3] == ["HELPER", *key])
            name = bytes.fromhex(observed[3])
            declaration = {"ordinal": ordinal, "name": name.decode(), "present": observed[4] == "1", "command_flags": int(observed[5])}
            for role in ["FORMALS", "BODY"]:
                value = bytes.fromhex(next(row[4] for row in rows if row[:4] == ["BYTES", *key, role]))
                path = directory / f"{ordinal}-{role.lower()}.bin"
                path.write_bytes(value)
                location = next(row for row in rows if row[:4] == ["SOURCE", *key, role])
                header = next(row for row in rows if row[:4] == ["OBJECT", *key, role])
                declaration[role.lower()] = {"path": str(path.relative_to(output)), "bytes": len(value), "sha256": sha(value),
                    "filename_hex": location[5], "first_line": int(location[4]),
                    "header": {"resident": header[4] == "1", "type": header[5], "reference_count": int(header[6])}}
            first = declaration["body"]["first_line"]
            line_start = sum(len(line) for line in source.splitlines(keepends=True)[:first - 1])
            formals = (output / declaration["formals"]["path"]).read_bytes()
            prefix = b"proc {" + name + b"} {" + formals + b"} {"
            assert source.startswith(prefix, line_start)
            body_start = line_start + len(prefix)
            body = (output / declaration["body"]["path"]).read_bytes()
            assert source[body_start:body_start + len(body)] == body
            assert source[body_start + len(body):body_start + len(body) + 1] == b"}"
            declaration["derived_source_slice"] = {"source": "static-evaluated-source.tcl" if purpose == "STATIC" else "raw-nshelper-source.tcl",
                "source_sha256": sha(source), "declaration_start": line_start,
                "body_start": body_start, "body_end_exclusive": body_start + len(body),
                "limitation": "Exact retained source-byte slice verified against observed body/line; not a native byte-offset or live procedure identity receipt"}
            index[purpose].append(declaration)
    (output / "observations.json").write_text(json.dumps(index, indent=2) + "\n")


if __name__ == "__main__":
    main()
