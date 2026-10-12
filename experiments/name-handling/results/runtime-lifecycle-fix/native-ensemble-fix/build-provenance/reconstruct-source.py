# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Recover and verify the immutable source snapshot without changing git state."""

import argparse
import hashlib
import json
import subprocess
import tempfile
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    manifest_path = Path(__file__).with_name("manifest.json")
    manifest = json.loads(manifest_path.read_text())
    repository = next(
        path for path in manifest_path.parents if (path / ".git").exists()
    )
    output = args.output.resolve()
    if output.is_relative_to(repository):
        parser.error("write the recovered snapshot outside the repository")
    if output.exists():
        parser.error("output already exists; choose a fresh path")
    digest = hashlib.sha256()
    size = 0
    with tempfile.TemporaryFile() as recovered:
        for part in manifest["parts"]:
            archived_path = f"{manifest['archive_directory']}/{part['path']}"
            data = subprocess.check_output(
                ["git", "show", f"{manifest['archive_commit']}:{archived_path}"],
                cwd=repository,
            )
            if (
                len(data) != part["bytes"]
                or hashlib.sha256(data).hexdigest() != part["sha256"]
            ):
                raise ValueError(f"archived chunk failed verification: {part['path']}")
            recovered.write(data)
            digest.update(data)
            size += len(data)
        if (
            size != manifest["original_bytes"]
            or digest.hexdigest() != manifest["original_sha256"]
        ):
            raise ValueError("reconstructed snapshot failed complete verification")
        recovered.seek(0)
        with output.open("xb") as destination:
            while block := recovered.read(1024 * 1024):
                destination.write(block)
    print(f"{digest.hexdigest()}  {output}")


if __name__ == "__main__":
    main()
