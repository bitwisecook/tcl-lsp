#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Parse exact-context NHO captures without discarding repeats or missing chunks."""

import argparse
import hashlib
import json
import re
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]


def parse_capture(paths, context, run_id=None, root=ROOT):
    """Return retained rows and strictly complete fields; v1 has no invocation ID."""
    root = Path(root).resolve()
    rows = []
    rejected = []
    sources = []
    for supplied in paths:
        path = Path(supplied)
        path = (path if path.is_absolute() else root / path).resolve()
        name = path.relative_to(root).as_posix()
        data = path.read_bytes()
        sources.append({"path": name, "sha256": hashlib.sha256(data).hexdigest()})
        for line_number, raw_line in enumerate(data.splitlines(), 1):
            marker = raw_line.find(b"NHO|")
            if marker < 0:
                continue
            raw = raw_line[marker:]
            parts = raw.split(b"|")
            if len(parts) < 4 or parts[3] != context.encode("ascii"):
                continue
            if run_id is not None and parts[1] != run_id.encode("ascii"):
                continue
            location = {"path": name, "line": line_number, "line_hex": raw_line.hex()}
            try:
                values = [part.decode("ascii") for part in parts]
                if len(values) == 13:
                    invocation = values[7]
                    tail = values[8:]
                elif len(values) == 12:
                    invocation = None
                    tail = values[7:]
                else:
                    raise ValueError("expected 12 or 13 fields")
                field, encoding, index, count, payload = tail
                if not re.fullmatch(r"[0-9]+", values[5]):
                    raise ValueError("invalid completion code")
                if not re.fullmatch(r"[0-9]+", index) or not re.fullmatch(
                    r"[1-9][0-9]*", count
                ):
                    raise ValueError("invalid chunk coordinates")
                if int(index) >= int(count):
                    raise ValueError("chunk index outside extent")
                if not re.fullmatch(r"(?:[0-9a-f]{2})*", payload):
                    raise ValueError("invalid chunk hex")
                if encoding not in {"utf-8", "binary-scan-tcl-value"}:
                    raise ValueError("unrecognised value encoding")
                rows.append(
                    {
                        **location,
                        "format_version": 2 if invocation is not None else 1,
                        "run_id": values[1],
                        "case_id": values[2],
                        "context": values[3],
                        "stage": values[4],
                        "completion_code": int(values[5]),
                        "tmm_unit": values[6],
                        "invocation_id": invocation,
                        "field": field,
                        "encoding": encoding,
                        "chunk_index": int(index),
                        "chunk_count": int(count),
                        "chunk_hex": payload,
                    }
                )
            except (UnicodeError, ValueError) as exc:
                rejected.append({**location, "reason": str(exc)})
    groups = defaultdict(list)
    keys = (
        "run_id",
        "case_id",
        "context",
        "stage",
        "tmm_unit",
        "invocation_id",
        "field",
    )
    for index, row in enumerate(rows):
        groups[tuple(row[key] for key in keys)].append(index)
    fields = []
    for identity, indices in groups.items():
        selected = [rows[index] for index in indices]
        errors = []
        counts = {row["chunk_count"] for row in selected}
        encodings = {row["encoding"] for row in selected}
        codes = {row["completion_code"] for row in selected}
        by_index = defaultdict(list)
        for row in selected:
            by_index[row["chunk_index"]].append(row)
        duplicates = sorted(
            index for index, items in by_index.items() if len(items) > 1
        )
        if duplicates:
            errors.append("duplicate chunks; repeated observations remain unjoined")
        if len(counts) != 1:
            errors.append("inconsistent chunk counts")
        if len(encodings) != 1:
            errors.append("inconsistent value encodings")
        if len(codes) != 1:
            errors.append("inconsistent completion codes")
        count = next(iter(counts)) if len(counts) == 1 else None
        missing = (
            sorted(set(range(count)) - by_index.keys()) if count is not None else []
        )
        if missing:
            errors.append("missing chunks")
        if identity[5] == "":
            errors.append("empty v2 invocation identifier")
        value_hex = None
        text = None
        decoding_error = None
        if not errors and count is not None:
            value_hex = "".join(
                by_index[index][0]["chunk_hex"] for index in range(count)
            )
            if next(iter(encodings)) == "utf-8":
                try:
                    text = bytes.fromhex(value_hex).decode("utf-8")
                except UnicodeError as exc:
                    decoding_error = str(exc)
        fields.append(
            {
                **dict(zip(keys, identity)),
                "row_indices": indices,
                "complete": not errors,
                "errors": errors,
                "missing_chunks": missing,
                "duplicate_chunks": duplicates,
                "completion_code": next(iter(codes)) if len(codes) == 1 else None,
                "encoding": next(iter(encodings)) if len(encodings) == 1 else None,
                "value_hex": value_hex,
                "text": text,
                "decoding_error": decoding_error,
                "invocation_identified": identity[5] not in {None, ""},
            }
        )
    return {
        "schema_version": 1,
        "context": context,
        "run_id": run_id,
        "sources": sources,
        "rows": rows,
        "rejected_rows": rejected,
        "fields": fields,
        "limitations": (
            "Field completeness supplies no execution, setup, cleanup or provider authority. "
            "V1 rows have no invocation ID. Duplicate chunks are retained, never deduplicated. "
            "Only values explicitly marked utf-8 receive a decoded text projection."
        ),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("paths", type=Path, nargs="+")
    parser.add_argument("--context", required=True)
    parser.add_argument("--run")
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = parse_capture(args.paths, args.context, args.run, args.root)
    with args.output.open("x") as stream:
        stream.write(json.dumps(result, indent=2, ensure_ascii=False) + "\n")


if __name__ == "__main__":
    main()
