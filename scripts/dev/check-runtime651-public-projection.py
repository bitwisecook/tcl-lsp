#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Reconcile maintained Runtime651 expected values to retained native streams.

This checks a derived software representation. It does not execute a native
provider or Rust test, recover historical projection bytes, or certify a release.
"""

from __future__ import annotations

import argparse
import ast
import hashlib
import json
import re
import sys
from pathlib import Path

DIRECTORY = "runtime/rust/tests/data/native_original_list_group_command651"
PROOF = "naming.list.original-grouped-stride-and-command-result-getters"
PROVIDERS = {
    "tcl8.4": "8.4.20",
    "tcl8.5": "8.5.19",
    "tcl8.6": "8.6.18",
    "tcl9.0": "9.0.4",
    "tcl9.1": "9.1.0",
    "jim": "Jim",
}
FIELDS = {
    "dialect",
    "case",
    "command",
    "error",
    "real",
    "index",
    "count",
    "input",
    "returned",
    "effect",
    "return_effect",
    "code",
    "result",
    "stream_sha256",
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def digest(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def literal(text: str) -> object:
    """Read the small Rust literal subset used by these maintained rows."""

    def convert(node: ast.AST) -> object:
        if isinstance(node, ast.Constant) and type(node.value) in {
            str,
            int,
            type(None),
        }:
            return node.value
        if isinstance(node, ast.Name) and node.id in {"true", "false"}:
            return node.id == "true"
        if isinstance(node, ast.Tuple):
            return tuple(convert(child) for child in node.elts)
        if isinstance(node, ast.List):
            return [convert(child) for child in node.elts]
        if (
            isinstance(node, ast.Call)
            and isinstance(node.func, ast.Name)
            and node.func.id == "Some"
            and len(node.args) == 1
            and not node.keywords
        ):
            return convert(node.args[0])
        raise ValueError("unsupported Runtime651 row literal")

    return convert(ast.parse(text, mode="eval").body)


def fields(text: str, indentation: int) -> dict[str, object]:
    result = {}
    for line in text.strip("\n").splitlines():
        match = re.fullmatch(" " * indentation + r"(\w+): (.*),", line)
        require(match is not None, "unsupported Runtime651 field layout")
        name, value = match.groups()
        require(name not in result, "duplicate Runtime651 row field")
        result[name] = literal(value)
    return result


def projection_rows(source: str) -> list[dict]:
    array = re.search(r"(?ms)^const ROWS: &\[Row\] = &\[\n(.*?)^[ \t]*\];", source)
    require(array is not None, "Runtime651 ROWS declaration is absent")
    result = []
    previous = 0
    for match in re.finditer(r"(?ms)^    Row \{\n(.*?)^    \},\n", array[1]):
        require(
            not array[1][previous : match.start()].strip(), "unparsed Runtime651 row"
        )
        body = match[1]
        options = re.search(
            r"(?ms)^        options: ListGroupOptions \{\n(.*?)^        \},\n", body
        )
        require(options is not None, "Runtime651 options declaration is absent")
        row = fields(body[: options.start()] + body[options.end() :], 8)
        require(set(row) == FIELDS, "Runtime651 row field set changed")
        row["options"] = fields(options[1], 12)
        require(
            set(row["options"]) == {"unique", "stride"}, "Runtime651 options changed"
        )
        result.append(row)
        previous = match.end()
    require(not array[1][previous:].strip(), "unparsed Runtime651 array tail")
    require(len(result) == 179, "Runtime651 must retain exactly179 projected rows")
    return result


def same(actual: object, expected: object, label: str) -> None:
    require(type(actual) is type(expected), f"{label}: projection type differs")
    if isinstance(expected, (tuple, list)):
        require(len(actual) == len(expected), f"{label}: projection length differs")
        for left, right in zip(actual, expected, strict=True):
            same(left, right, label)
    else:
        require(
            actual == expected,
            f"{label}: projection differs from retained input/output",
        )


def input_value(stream: dict[str, list[str]], label: str) -> tuple[str, str]:
    value = stream[label]
    return value[0], value[1] if len(value) > 1 else ""


def reached_value(stream: dict[str, list[str]], label: str) -> tuple[str, bool] | None:
    value = stream[label]
    require(
        value[0] == "absent" or len(value) >= 2 and value[1] in {"0", "1"},
        "invalid Runtime651 reached value",
    )
    return None if value[0] == "absent" else (value[0], value[1] == "1")


def validate(root: Path) -> dict:
    root = root.resolve()
    consumed: dict[str, str] = {}

    def read(name: str) -> bytes:
        path = (root / name).resolve()
        require(path.is_relative_to(root), "projection input escapes repository")
        raw = path.read_bytes()
        sha = digest(raw)
        require(
            name not in consumed or consumed[name] == sha,
            "projection input changed during read",
        )
        consumed[name] = sha
        return raw

    manifest_path = "docs/design/analysis/name-resolution-proofs/manifest.json"
    manifest_raw = read(manifest_path)
    records = [row for row in json.loads(manifest_raw)["proofs"] if row["id"] == PROOF]
    require(len(records) == 1, "Runtime651 proof ID is absent or duplicated")
    record = records[0]
    evidence = {row["id"]: row for row in record["evidence"]}
    require(
        len(evidence) == len(record["evidence"]),
        "Runtime651 evidence IDs are duplicated",
    )
    pinned = {}
    for name, identifier in [
        ("probe.c", "probe"),
        ("retention.json", "lossless-retention"),
        ("interpretation.json", "interpretation"),
    ]:
        path = f"{DIRECTORY}/{name}"
        require(
            evidence[identifier]["path"] == path,
            "Runtime651 input evidence path changed",
        )
        pinned[name] = read(path)
        require(
            digest(pinned[name]) == evidence[identifier]["sha256"],
            "Runtime651 original input pin changed",
        )
    retained_rows = json.loads(pinned["retention.json"])["retained"]
    retained = {row["canonical"]: row for row in retained_rows}
    require(
        len(retained) == len(retained_rows), "Runtime651 retention paths are duplicated"
    )
    declarations = re.findall(
        r'^ \{"([^"]+)",(\d+),(\d+),(\d+),(\d+),(NULL|"[^"]*"),(\d+),(\d+),',
        pinned["probe.c"].decode(),
        re.MULTILINE,
    )
    require(
        len(declarations) == 32, "Runtime651 probe must retain all32 case declarations"
    )
    source = read(f"{DIRECTORY}/software_controls.rs")
    rows = projection_rows(source.decode())
    outcomes = []
    roster = set()
    for row in rows:
        dialect, case = row["dialect"], row["case"]
        require(
            type(dialect) is str and dialect in PROVIDERS,
            "unknown Runtime651 projection provider",
        )
        require(type(case) is int and 0 <= case < 32, "invalid Runtime651 case number")
        require((dialect, case) not in roster, "duplicate Runtime651 projection case")
        roster.add((dialect, case))
        path = f"{DIRECTORY}/{PROVIDERS[dialect]}/case-{case:02d}/execute.stdout"
        raw = read(path)
        sha = digest(raw)
        require(
            sha == retained[path]["original_sha256"] == retained[path]["stored_sha256"],
            "Runtime651 stream differs from lossless retention",
        )
        same(row["stream_sha256"], sha, "stream digest")
        stream = {}
        for line in raw.decode().splitlines():
            parts = line.split("\t")
            require(parts[0] not in stream, "duplicate Runtime651 stream label")
            stream[parts[0]] = parts[1:]
        declaration = declarations[case]
        require(
            stream["case"] == [str(case), declaration[0]],
            "Runtime651 probe/stream case identity differs",
        )
        require(int(declaration[2]) == 0, "search output cannot enter sort projection")
        for name, expected in [
            ("command", int(declaration[1]) != 0),
            ("error", bool(int(declaration[7]))),
            ("count", int(declaration[4])),
            ("index", None if declaration[5] == "NULL" else json.loads(declaration[5])),
            ("real", stream["mode"][0]),
            ("code", int(stream["code"][0])),
            ("result", stream["result-bytes"][0]),
        ]:
            same(row[name], expected, name)
        same(row["options"]["unique"], bool(int(declaration[6])), "unique")
        same(row["options"]["stride"], int(declaration[1]) != 1, "stride")
        same(
            row["input"],
            [input_value(stream, f"input-key{index}") for index in range(4)],
            "input",
        )
        same(row["returned"], input_value(stream, "input-callback-return"), "returned")
        same(
            row["effect"],
            [reached_value(stream, f"reached-key{index}") for index in range(4)],
            "effect",
        )
        same(
            row["return_effect"],
            reached_value(stream, "reached-callback-return"),
            "return_effect",
        )
        outcomes.append(
            {"dialect": dialect, "case": case, "stream": path, "sha256": sha}
        )
    expected_roster = {
        (dialect, case)
        for dialect in PROVIDERS
        for case in range(32)
        if case not in {14, 15} and (dialect, case) != ("jim", 12)
    }
    require(
        roster == expected_roster,
        "Runtime651 projection missing-output/search boundary changed",
    )
    for name, expected_sha in consumed.items():
        require(
            digest((root / name).read_bytes()) == expected_sha,
            "Runtime651 input changed during validation",
        )
    return {
        "schema_version": 1,
        "scope": "Field-by-field derivation of179 maintained Rust public projection rows from original pinned probe declarations and exact retained native streams; no native/Rust execution, historical projection recovery, private payload, alias/refcount or callback chronology claim.",
        "proof_id": PROOF,
        "input_manifest_sha256": digest(manifest_raw),
        "projection_sha256": digest(source),
        "verifier_sha256": digest(Path(__file__).read_bytes()),
        "inputs": [
            {"path": name, "sha256": sha} for name, sha in sorted(consumed.items())
        ],
        "rows": outcomes,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root", type=Path, default=Path(__file__).resolve().parents[2]
    )
    parser.add_argument(
        "--receipt", type=Path, help="Explicit destination for a derivation receipt."
    )
    args = parser.parse_args()
    try:
        result = validate(args.root)
        if args.receipt:
            args.receipt.write_text(json.dumps(result, indent=2) + "\n")
    except (ValueError, OSError, KeyError, IndexError, TypeError, SyntaxError) as exc:
        print(f"Runtime651 projection derivation failed: {exc}", file=sys.stderr)
        return 1
    print(
        json.dumps(
            {
                "verified_rows": len(result["rows"]),
                "projection_sha256": result["projection_sha256"],
                "execution": "not run",
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
