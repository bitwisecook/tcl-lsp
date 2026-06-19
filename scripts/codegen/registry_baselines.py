#!/usr/bin/env python3
"""Regenerate the registry presence fixtures under ``tests/baselines/registry/``.

The committed golden is a small set of CSVs proving that every command,
event, profile, and object is present in the registry with basic data
(arities and counts for commands).  They are the *presence safety-net*;
the heavy coverage lives in the generated front-end behaviour tests under
``tests/registry_contract/``.

CSVs were chosen over verbose JSON dumps on purpose: a registry change
produces a tiny, line-oriented, reviewable diff (one row per command),
not a multi-megabyte blob.  There is no committed JSON dump; structural
detail beyond the CSVs is asserted by reading the registry directly in
the contract tests.

Usage::

    python scripts/codegen/registry_baselines.py            # rewrite CSVs
    python scripts/codegen/registry_baselines.py --check    # fail if stale
"""

from __future__ import annotations

import argparse
import csv
import io
import sys
from collections.abc import Callable
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

from tooling.registry_snapshot import (  # noqa: E402
    all_command_rows,
    event_rows,
    object_rows,
    profile_rows,
)

ROOT = Path(__file__).resolve().parents[2]
BASELINE_DIR = ROOT / "tests" / "baselines" / "registry"

# fixture filename -> (builder, ordered column names)
_FIXTURES: dict[str, tuple[Callable[[], list[dict[str, str]]], list[str]]] = {
    "commands.csv": (
        all_command_rows,
        ["dialect", "command", "arity_min", "arity_max", "subcommands", "switches"],
    ),
    "events.csv": (
        event_rows,
        ["event", "known", "deprecated", "side", "multiplicity", "valid_commands"],
    ),
    "profiles.csv": (profile_rows, ["profile", "layer", "side"]),
    "objects.csv": (object_rows, ["kind", "module", "object_types"]),
}


def _serialise(rows: list[dict[str, str]], columns: list[str]) -> str:
    buf = io.StringIO()
    writer = csv.DictWriter(buf, fieldnames=columns, lineterminator="\n")
    writer.writeheader()
    writer.writerows(rows)
    return buf.getvalue()


def write_all(out_dir: Path = BASELINE_DIR) -> list[Path]:
    out_dir.mkdir(parents=True, exist_ok=True)
    written: list[Path] = []
    for name, (build, columns) in _FIXTURES.items():
        path = out_dir / name
        path.write_text(_serialise(build(), columns), encoding="utf-8")
        written.append(path)
    return written


def check_all(out_dir: Path = BASELINE_DIR) -> list[str]:
    problems: list[str] = []
    for name, (build, columns) in _FIXTURES.items():
        path = out_dir / name
        expected = _serialise(build(), columns)
        if not path.exists():
            problems.append(f"missing fixture: {path.relative_to(ROOT)}")
        elif path.read_text(encoding="utf-8") != expected:
            problems.append(f"stale fixture: {path.relative_to(ROOT)}")
    return problems


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="Fail if any CSV is stale.")
    args = parser.parse_args(argv)

    if args.check:
        problems = check_all()
        if problems:
            print("registry fixtures are out of date:", file=sys.stderr)
            for problem in problems:
                print(f"  - {problem}", file=sys.stderr)
            print(
                "\nRun `make gen-registry-baselines` and commit the result.",
                file=sys.stderr,
            )
            return 1
        print("registry fixtures are up to date.")
        return 0

    written = write_all()
    print(f"wrote {len(written)} registry fixtures to {BASELINE_DIR.relative_to(ROOT)}/")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
