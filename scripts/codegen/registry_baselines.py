#!/usr/bin/env python3
"""Regenerate the registry golden fixtures under ``tests/baselines/registry/``.

The fixtures are the language-agnostic shape contract for the command
registry and the iRules event / profile / object graphs.  They are
produced by the same builders the ``tcl registry-dump`` and
``f5 registry-dump`` front-end verbs use, so the committed fixture is
exactly what the front-ends emit.

Usage::

    python scripts/codegen/registry_baselines.py            # rewrite fixtures
    python scripts/codegen/registry_baselines.py --check    # fail if stale

``--check`` is the CI-style staleness gate: it rebuilds every fixture in
memory and compares against the committed file, exiting non-zero (and
listing the drifted fixtures) when the registry has changed without the
fixtures being regenerated.
"""

from __future__ import annotations

import argparse
import json
import sys
from collections.abc import Callable
from pathlib import Path
from typing import Any

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

from tooling.registry_snapshot import (  # noqa: E402
    ALL_DIALECTS,
    command_registry_snapshot,
    event_graph_snapshot,
    object_graph_snapshot,
    profile_graph_snapshot,
)

ROOT = Path(__file__).resolve().parents[2]
BASELINE_DIR = ROOT / "tests" / "baselines" / "registry"


def _fixtures() -> dict[str, Callable[[], Any]]:
    """Map fixture filename -> builder.  One entry per golden file."""
    fixtures: dict[str, Callable[[], Any]] = {}
    for dialect in ALL_DIALECTS:
        fixtures[f"commands-{dialect}.json"] = lambda d=dialect: command_registry_snapshot(d)
    fixtures["events.json"] = event_graph_snapshot
    fixtures["profiles.json"] = profile_graph_snapshot
    fixtures["objects.json"] = object_graph_snapshot
    return fixtures


def _serialise(payload: Any) -> str:
    return json.dumps(payload, indent=2, sort_keys=True) + "\n"


def write_all(out_dir: Path = BASELINE_DIR) -> list[Path]:
    out_dir.mkdir(parents=True, exist_ok=True)
    written: list[Path] = []
    for name, build in _fixtures().items():
        path = out_dir / name
        path.write_text(_serialise(build()), encoding="utf-8")
        written.append(path)
    return written


def check_all(out_dir: Path = BASELINE_DIR) -> list[str]:
    """Return a list of human-readable drift descriptions (empty when clean)."""
    problems: list[str] = []
    for name, build in _fixtures().items():
        path = out_dir / name
        expected = _serialise(build())
        if not path.exists():
            problems.append(f"missing fixture: {path.relative_to(ROOT)}")
            continue
        if path.read_text(encoding="utf-8") != expected:
            problems.append(f"stale fixture: {path.relative_to(ROOT)}")
    return problems


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="Fail if any committed fixture is missing or out of date.",
    )
    args = parser.parse_args(argv)

    if args.check:
        problems = check_all()
        if problems:
            print("registry fixtures are out of date:", file=sys.stderr)
            for problem in problems:
                print(f"  - {problem}", file=sys.stderr)
            print(
                "\nRun `make gen-registry-baselines` (or "
                "`python scripts/codegen/registry_baselines.py`) and commit the result.",
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
