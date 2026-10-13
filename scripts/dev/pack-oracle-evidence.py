#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Build a new verified store from reviewed captures and an optional old store."""

import argparse
import gzip
import hashlib
import json
import stat
from pathlib import Path
from zipfile import ZIP_DEFLATED, ZipFile, ZipInfo


def pack(source, prefixes, output, previous=None):
    if output.exists():
        raise ValueError(
            "output must be a new directory; existing stores are never overwritten"
        )
    rows = {}
    old = None
    if previous:
        rows = json.loads(
            gzip.decompress((previous / "catalogue.json.gz").read_bytes())
        )["files"]
        old = ZipFile(previous / "artefacts.zip")
    updates = {}
    for prefix in prefixes:
        logical = Path(prefix)
        if logical.is_absolute() or ".." in logical.parts or not logical.parts:
            raise ValueError("capture prefixes must be repository-relative")
        directory = source / logical
        if not directory.is_dir() or not directory.resolve().is_relative_to(
            source.resolve()
        ):
            raise ValueError("capture prefix is absent or escapes the source")
        for path in sorted(directory.rglob("*")):
            if path.is_symlink():
                raise ValueError("capture symlinks are not retained")
            if not path.is_file():
                continue
            data = path.read_bytes()
            sha = hashlib.sha256(data).hexdigest()
            name = path.relative_to(source).as_posix()
            rows[name] = {
                "sha256": sha,
                "bytes": len(data),
                "mode": stat.S_IMODE(path.stat().st_mode),
            }
            updates[sha] = path
    wanted = sorted({row["sha256"] for row in rows.values()})
    if not wanted:
        raise ValueError("no capture files selected")
    output.mkdir(parents=True)
    try:
        with ZipFile(
            output / "artefacts.zip", "w", compression=ZIP_DEFLATED, compresslevel=9
        ) as archive:
            for sha in wanted:
                data = updates[sha].read_bytes() if sha in updates else old.read(sha)
                if hashlib.sha256(data).hexdigest() != sha:
                    raise ValueError(
                        "capture changed while packing, or an old body fails its digest"
                    )
                entry = ZipInfo(sha, date_time=(1980, 1, 1, 0, 0, 0))
                entry.compress_type = ZIP_DEFLATED
                entry.external_attr = 0o644 << 16
                archive.writestr(entry, data, compresslevel=9)
        value = {
            "schema_version": 1,
            "storage": "sha256-addressed ZIP_DEFLATED members; exact retained bytes",
            "files": rows,
        }
        (output / "catalogue.json.gz").write_bytes(
            gzip.compress(
                (
                    json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n"
                ).encode(),
                mtime=0,
            )
        )
        with ZipFile(output / "artefacts.zip") as archive:
            sizes = {}
            for sha in wanted:
                data = archive.read(sha)
                if hashlib.sha256(data).hexdigest() != sha:
                    raise ValueError("packed body failed verification")
                sizes[sha] = len(data)
            if any(sizes[row["sha256"]] != row["bytes"] for row in rows.values()):
                raise ValueError("packed catalogue lengths differ")
        return {
            "logical_files": len(rows),
            "distinct_bodies": len(wanted),
            "bytes": sum(p.stat().st_size for p in output.iterdir()),
            "verified": True,
            "sanitisation_performed": False,
        }
    finally:
        if old:
            old.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--source",
        type=Path,
        required=True,
        help="root containing reviewed repository-relative captures",
    )
    parser.add_argument("--prefix", action="append", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--previous", type=Path)
    args = parser.parse_args()
    print(
        json.dumps(pack(args.source, args.prefix, args.output, args.previous), indent=2)
    )


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, KeyError) as error:
        raise SystemExit(str(error)) from error
