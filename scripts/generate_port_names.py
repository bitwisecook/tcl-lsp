#!/usr/bin/env python3
"""Generate ``core/bigip/_port_names_table.py`` from the SCF port-name CSV.

mcpd's port-name table is checked in as ``core/bigip/data/scf_port_names.csv``
for human inspection and diffs.  Shipping the CSV in a wheel/zipapp would
force every cold start to crack open the archive and run a csv reader, so
we precompute a ``dict`` literal into a Python module instead — that gets
marshalled once into ``__pycache__`` (or the zipapp's bytecode entry) and
unmarshals essentially for free on subsequent imports.

Run via ``make generate``; ``make check-generated`` confirms the checked-in
table matches the CSV.
"""

from __future__ import annotations

import argparse
import csv
import subprocess
import sys
from pathlib import Path

_REPO_ROOT = Path(__file__).resolve().parent.parent
_DEFAULT_CSV = _REPO_ROOT / "core" / "bigip" / "data" / "scf_port_names.csv"
_DEFAULT_OUT = _REPO_ROOT / "core" / "bigip" / "_port_names_table.py"

_HEADER = '''\
"""Generated from ``core/bigip/data/scf_port_names.csv`` — do not edit by hand.

Regenerate with ``make generate`` (which runs ``scripts/generate_port_names.py``).
The dict literal below is the canonical mcpd service-name → port-number
table.  Keys are pre-lowercased so callers don't need to ``.lower()`` at
lookup time.
"""

from __future__ import annotations

NAME_TO_PORT: dict[str, int] = {
'''

_FOOTER = "}\n"


def _load(csv_path: Path) -> list[tuple[str, int]]:
    rows: list[tuple[str, int]] = []
    seen: set[str] = set()
    with csv_path.open(encoding="utf-8", newline="") as fh:
        for lineno, row in enumerate(csv.reader(fh), start=1):
            if len(row) != 2:
                raise SystemExit(f"{csv_path}:{lineno}: expected 2 columns, got {row!r}")
            name, num = row[0].strip(), row[1].strip()
            if not name:
                raise SystemExit(f"{csv_path}:{lineno}: empty name")
            if not num.isdigit():
                raise SystemExit(f"{csv_path}:{lineno}: non-numeric port {num!r}")
            port = int(num)
            if not 0 <= port <= 65535:
                raise SystemExit(f"{csv_path}:{lineno}: port {port} out of range")
            key = name.lower()
            if key in seen:
                raise SystemExit(f"{csv_path}:{lineno}: duplicate name {name!r}")
            seen.add(key)
            rows.append((key, port))
    rows.sort(key=lambda item: (item[1], item[0]))
    return rows


def _render(rows: list[tuple[str, int]]) -> str:
    parts = [_HEADER]
    for name, port in rows:
        # One row per line for readable diffs; the final string is piped
        # through ``ruff format`` below so the on-disk file is exactly
        # what the lint gate expects no matter how ruff's defaults shift.
        parts.append(f'    "{name}": {port},\n')
    parts.append(_FOOTER)
    return _ruff_format("".join(parts))


def _ruff_format(source: str) -> str:
    """Normalise *source* through ``ruff format -`` so the result always
    matches what ``ruff format --check`` expects.  Falls back to the raw
    text if ruff isn't installed — the check job will still catch any
    formatting drift, just with a less helpful error message.
    """
    try:
        result = subprocess.run(
            ["ruff", "format", "-"],
            input=source,
            capture_output=True,
            text=True,
            check=True,
        )
    except (FileNotFoundError, subprocess.CalledProcessError):
        return source
    return result.stdout


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--csv", type=Path, default=_DEFAULT_CSV)
    parser.add_argument("--output", type=Path, default=_DEFAULT_OUT)
    parser.add_argument(
        "--check",
        action="store_true",
        help="Exit non-zero if the generated table is out of date instead of writing.",
    )
    args = parser.parse_args(argv)

    rendered = _render(_load(args.csv))
    if args.check:
        existing = args.output.read_text(encoding="utf-8") if args.output.is_file() else ""
        if existing != rendered:
            print(
                f"{args.output} is out of date — run `make generate`.",
                file=sys.stderr,
            )
            return 1
        return 0

    args.output.write_text(rendered, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
