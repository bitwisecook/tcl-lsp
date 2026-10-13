# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Read byte-exact oracle artefacts from the indexed, deduplicated F5 store."""

from __future__ import annotations

import gzip
import hashlib
import json
from functools import lru_cache
from pathlib import Path
from types import SimpleNamespace
from zipfile import ZipFile

STORE = Path("experiments/oracles/f5/captures")


@lru_cache(maxsize=4)
def catalogue(root: Path) -> dict:
    path = root / STORE / "catalogue.json.gz"
    if not path.is_file():
        return {"files": {}}
    return json.loads(gzip.decompress(path.read_bytes()))


@lru_cache(maxsize=4)
def archive(root: Path) -> ZipFile:
    return ZipFile(root / STORE / "artefacts.zip")


@lru_cache(maxsize=8)
def body(root: Path, sha256: str) -> bytes:
    data = archive(root).read(sha256)
    if hashlib.sha256(data).hexdigest() != sha256:
        raise ValueError(f"oracle artefact digest changed: {sha256}")
    return data


class PackedFile:
    """A read-only logical file; never silently extracts into the checkout."""

    def __init__(self, root: Path, name: str):
        self.root = root.resolve()
        self.logical = Path(name)
        self.descriptor = catalogue(self.root)["files"][name]

    @property
    def suffix(self):
        return self.logical.suffix

    @property
    def parts(self):
        return self.logical.parts

    @property
    def name(self):
        return self.logical.name

    def relative_to(self, root):
        if Path(root).resolve() != self.root:
            raise ValueError("oracle file belongs to another repository")
        return self.logical

    def read_bytes(self):
        data = body(self.root, self.descriptor["sha256"])
        if len(data) != self.descriptor["bytes"]:
            raise ValueError("oracle artefact length differs from the catalogue")
        return data

    def read_text(self, encoding="utf-8", errors="strict"):
        return self.read_bytes().decode(encoding, errors)

    def stat(self):
        return SimpleNamespace(st_size=self.descriptor["bytes"])

    def is_file(self):
        return True


def retained_file(root: Path, name: str):
    """Resolve a safe current path or a retained historical logical path."""
    root = root.resolve()
    logical = Path(name)
    if logical.is_absolute() or ".." in logical.parts or not logical.parts:
        raise ValueError(f"expected a repository-relative path: {name!r}")
    physical = (root / logical).resolve()
    if not physical.is_relative_to(root):
        raise ValueError(f"path escapes repository: {name!r}")
    if physical.is_file():
        return physical
    if name in catalogue(root)["files"]:
        return PackedFile(root, name)
    raise ValueError(f"missing file: {name}")


def retained_files(root: Path, prefix: str):
    return [
        PackedFile(root, name)
        for name in catalogue(root.resolve())["files"]
        if name.startswith(prefix.rstrip("/") + "/")
    ]
