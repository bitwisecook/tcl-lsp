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
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program. If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Record and verify every tracked path using reversible JSON names and SHA-256."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path
from typing import TypedDict, cast


class FileDigest(TypedDict):
    path: str
    sha256: str


class Snapshot(TypedDict):
    version: int
    source_commit: str
    file_count: int
    files: list[FileDigest]


def git_output(arguments: list[str], root: Path | None = None) -> bytes:
    return subprocess.check_output(["git", *arguments], cwd=root)


def repository_root() -> Path:
    return Path(
        os.fsdecode(git_output(["rev-parse", "--show-toplevel"]).removesuffix(b"\n"))
    )


def source_commit(root: Path) -> str:
    commit = git_output(["rev-parse", "--verify", "HEAD"], root).decode("ascii").strip()
    if re.fullmatch(r"[0-9a-f]{40}", commit) is None:
        raise ValueError("HEAD is not a complete commit identity")
    return commit


def validate_path(path: str) -> None:
    if "\0" in path or any(part in {"", ".", ".."} for part in path.split("/")):
        raise ValueError(f"non-canonical tracked path: {path!r}")
    if os.fsdecode(os.fsencode(path)) != path:
        raise ValueError(f"tracked path does not round-trip: {path!r}")


def tracked_paths(root: Path) -> list[str]:
    raw = git_output(["ls-files", "--cached", "--full-name", "-z"], root)
    if not raw or not raw.endswith(b"\0"):
        raise ValueError("tracked path list is empty or lacks its NUL terminator")
    paths = [os.fsdecode(path) for path in sorted(raw[:-1].split(b"\0"))]
    if len(paths) != len(set(paths)):
        raise ValueError("tracked path list contains duplicates")
    for path in paths:
        validate_path(path)
    return paths


def file_digest(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        while block := stream.read(1024 * 1024):
            digest.update(block)
    return digest.hexdigest()


def record(root: Path, output: Path) -> dict[str, object]:
    commit = source_commit(root)
    paths = tracked_paths(root)
    snapshot: Snapshot = {
        "version": 1,
        "source_commit": commit,
        "file_count": len(paths),
        "files": [{"path": path, "sha256": file_digest(root / path)} for path in paths],
    }
    if source_commit(root) != commit or tracked_paths(root) != paths:
        raise ValueError("HEAD or tracked membership changed during recording")
    with output.open("x", encoding="utf-8") as stream:
        json.dump(snapshot, stream, ensure_ascii=True, indent=2)
        stream.write("\n")
    return {"status": "RECORDED", "source_commit": commit, "file_count": len(paths)}


def read_snapshot(path: Path) -> tuple[Snapshot, str]:
    raw = path.read_bytes()
    manifest_digest = hashlib.sha256(raw).hexdigest()
    value: object = json.loads(raw)
    if not isinstance(value, dict):
        raise TypeError("snapshot must be a JSON object")
    header = cast(dict[str, object], value)
    if set(header) != {"version", "source_commit", "file_count", "files"}:
        raise ValueError("unexpected snapshot fields")
    if type(header["version"]) is not int or header["version"] != 1:
        raise ValueError("unsupported snapshot version")
    commit = header["source_commit"]
    if not isinstance(commit, str) or re.fullmatch(r"[0-9a-f]{40}", commit) is None:
        raise ValueError("invalid snapshot commit identity")
    count = header["file_count"]
    records = header["files"]
    if type(count) is not int or count <= 0 or not isinstance(records, list):
        raise ValueError("invalid snapshot file count or records")
    files: list[FileDigest] = []
    for item in cast(list[object], records):
        if not isinstance(item, dict):
            raise TypeError("invalid file record")
        row = cast(dict[str, object], item)
        path_value, digest = row.get("path"), row.get("sha256")
        if set(row) != {"path", "sha256"} or not isinstance(path_value, str):
            raise ValueError("invalid file record fields")
        validate_path(path_value)
        if not isinstance(digest, str) or re.fullmatch(r"[0-9a-f]{64}", digest) is None:
            raise ValueError("invalid file digest")
        files.append({"path": path_value, "sha256": digest})
    if len(files) != count or len({row["path"] for row in files}) != count:
        raise ValueError("file count mismatch or duplicate path records")
    return (
        {"version": 1, "source_commit": commit, "file_count": count, "files": files},
        manifest_digest,
    )


def verify(root: Path, manifest: Path) -> dict[str, object]:
    snapshot, manifest_digest = read_snapshot(manifest)
    commit = source_commit(root)
    current = tracked_paths(root)
    expected = {row["path"] for row in snapshot["files"]}
    missing = sorted(expected - set(current))
    extra = sorted(set(current) - expected)
    mismatches: list[dict[str, str]] = []
    read_errors: list[dict[str, str]] = []
    matched = 0
    for row in snapshot["files"]:
        try:
            actual = file_digest(root / row["path"])
        except OSError as error:
            read_errors.append({"path": row["path"], "error": str(error)})
            continue
        if actual != row["sha256"]:
            mismatches.append(
                {"path": row["path"], "expected": row["sha256"], "actual": actual}
            )
        else:
            matched += 1
    stable_membership = tracked_paths(root) == current and source_commit(root) == commit
    passed = (
        commit == snapshot["source_commit"]
        and stable_membership
        and not missing
        and not extra
        and not mismatches
        and not read_errors
        and matched == snapshot["file_count"] == len(current)
    )
    return {
        "status": "PASS" if passed else "FAIL",
        "manifest_sha256": manifest_digest,
        "source_commit": commit,
        "expected_source_commit": snapshot["source_commit"],
        "expected_file_count": snapshot["file_count"],
        "current_tracked_file_count": len(current),
        "checked_file_count": len(snapshot["files"]) - len(read_errors),
        "matched_file_count": matched,
        "stable_membership_during_verification": stable_membership,
        "missing_tracked_paths": missing,
        "extra_tracked_paths": extra,
        "checksum_mismatches": mismatches,
        "read_errors": read_errors,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=("record", "verify"))
    parser.add_argument("manifest", type=Path)
    args = parser.parse_args()
    result: dict[str, object]
    try:
        root = repository_root()
        result = (
            record(root, args.manifest)
            if args.operation == "record"
            else verify(root, args.manifest)
        )
    except (OSError, TypeError, ValueError, subprocess.CalledProcessError) as error:
        result = {"status": "FAIL", "error": str(error)}
    print(json.dumps(result, ensure_ascii=True, indent=2))
    return int(result["status"] not in {"RECORDED", "PASS"})


if __name__ == "__main__":
    sys.exit(main())
