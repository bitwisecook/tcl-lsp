#!/usr/bin/env python3
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
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Hold the tracked tree to a size budget.

Every CI job checks the repository out, and every clone carries its history,
so bulk evidence (archives, captures, VM images) committed to the tree costs
each job and each developer from then on. A `work` checkout once reached
10.7 GB in 127k files, and its CI checkouts took up to 45 minutes.

The budget: no tracked file over FILE_LIMIT unless listed in
scripts/dev/tracked-size-allowlist.txt (one repo-relative path per line, `#`
comments), and the whole tracked tree under TREE_LIMIT. Sizes are of the
checked-out files, which is what a checkout writes.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

MIB = 1024 * 1024
FILE_LIMIT = 10 * MIB
TREE_LIMIT = 1024 * MIB
ALLOWLIST = Path("scripts/dev/tracked-size-allowlist.txt")


@dataclass(frozen=True)
class Report:
    oversized: list[tuple[str, int]]
    total: int
    stale_allowlist: list[str]

    def problems(self, tree_limit: int) -> list[str]:
        out = [
            f"{path}: {size / MIB:.1f} MiB exceeds the {FILE_LIMIT // MIB} MiB per-file budget"
            for path, size in self.oversized
        ]
        if self.total > tree_limit:
            out.append(
                f"tracked tree is {self.total / MIB:.0f} MiB, over the {tree_limit // MIB} MiB budget"
            )
        out.extend(
            f"{ALLOWLIST}: {path} is no longer an oversized tracked file; remove it"
            for path in self.stale_allowlist
        )
        return out


def tracked_files(root: Path) -> list[str]:
    raw = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-z"],
        check=True,
        capture_output=True,
    ).stdout
    return [p.decode() for p in raw.split(b"\0") if p]


def read_allowlist(path: Path) -> set[str]:
    if not path.is_file():
        return set()
    entries = set()
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.split("#", 1)[0].strip()
        if line:
            entries.add(line)
    return entries


def measure(
    root: Path, files: list[str], allowed: set[str], file_limit: int = FILE_LIMIT
) -> Report:
    oversized = []
    total = 0
    big = set()
    for rel in files:
        path = root / rel
        if path.is_symlink() or not path.is_file():
            continue
        size = path.stat().st_size
        total += size
        if size > file_limit:
            big.add(rel)
            if rel not in allowed:
                oversized.append((rel, size))
    return Report(sorted(oversized), total, sorted(allowed - big))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--root", type=Path, default=Path(__file__).resolve().parents[2]
    )
    parser.add_argument("--tree-limit-mib", type=int, default=TREE_LIMIT // MIB)
    args = parser.parse_args(argv)
    root = args.root
    report = measure(root, tracked_files(root), read_allowlist(root / ALLOWLIST))
    problems = report.problems(args.tree_limit_mib * MIB)
    for problem in problems:
        print(f"tracked-size: {problem}", file=sys.stderr)
    if problems:
        print(
            "tracked-size: keep bulk evidence out of git (release assets or an artefact store); "
            f"allowlist a file in {ALLOWLIST} only when a build or test reads it",
            file=sys.stderr,
        )
        return 1
    print(
        f"tracked-size: {report.total / MIB:.0f} MiB tracked, every file within {FILE_LIMIT // MIB} MiB"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
