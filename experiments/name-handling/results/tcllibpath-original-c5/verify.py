#!/usr/bin/env python3
"""Verify retained checksums only; never execute an oracle or implementation."""
from pathlib import Path
import hashlib

ROOT = Path(__file__).resolve().parent
ORIGINAL_SEAL = "788c7a1f03f844c621c114cba6484296dbe7da0c7305640eb197f317afa8dd81"


def verify(directory):
    listed = set()
    for row in (directory / "SHA256SUMS").read_text().splitlines():
        expected, relative = row.split("  ", 1)
        path = Path(relative)
        if path.is_absolute() or ".." in path.parts or relative in listed:
            raise SystemExit(f"invalid checksum path: {relative}")
        listed.add(relative)
        actual = hashlib.sha256((directory / path).read_bytes()).hexdigest()
        if actual != expected:
            raise SystemExit(f"checksum mismatch: {relative}")
    actual_files = {str(path.relative_to(directory)) for path in directory.rglob("*") if path.is_file()}
    if actual_files != listed | {"SHA256SUMS"}:
        raise SystemExit(f"checksum inventory mismatch: {actual_files ^ (listed | {'SHA256SUMS'})}")
    return len(listed)


if hashlib.sha256((ROOT / "original/SHA256SUMS").read_bytes()).hexdigest() != ORIGINAL_SEAL:
    raise SystemExit("original checksum seal mismatch")
original = verify(ROOT / "original")
outer = verify(ROOT)
print(f"Verified {original} original entries and {outer} retention entries; no execution performed.")
