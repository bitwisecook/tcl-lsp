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

"""Hold every original Rust source file to the full AGPL-3.0 notice.

CONTRIBUTING.md asks for the full notice at the top of our own source files.
A truncated or missing notice is copied forward whenever a new file is modelled
on an old one, so this gate fails on any tracked `.rs` file under `rust/` or
`runtime/rust/` whose leading `//` comment block is not the notice.

Exempt: vendored code (`/vendor/`), test data (`/tests/data/`), generated
files (`@generated` or `Generated from` in the first lines), files carrying a
non-AGPL SPDX identifier, and the paths in scripts/dev/license-header-exempt.txt.

`--fix` rewrites the leading notice in place, keeping the copyright year.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

EXEMPT_LIST = Path("scripts/dev/license-header-exempt.txt")
ROOTS = ("rust", "runtime/rust")
EXEMPT_PARTS = ("/vendor/", "/tests/data/")
GENERATED_MARKERS = ("@generated", "Generated from")
HEAD_LINES = 3
DEFAULT_YEAR = "2026"

NOTICE = """\
// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) {year} James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later
"""

LICENSE_MARKERS = ("tcl-lsp —", "Copyright (C)", "SPDX-License-Identifier", "GNU Affero")
YEAR = re.compile(r"Copyright \(C\) (\d{4}(?:[-–]\d{4})?)")
SPDX = re.compile(r"SPDX-License-Identifier:\s*(\S+)")


def is_plain_comment(line: str) -> bool:
    return line.startswith("//") and not line.startswith(("///", "//!"))


def leading_block(lines: list[str]) -> int:
    """Number of leading plain `//` lines (the candidate notice block)."""
    n = 0
    while n < len(lines) and is_plain_comment(lines[n]):
        n += 1
    return n


def exempt_by_content(text: str) -> bool:
    head = text.splitlines()[:HEAD_LINES]
    if any(marker in line for line in head for marker in GENERATED_MARKERS):
        return True
    m = SPDX.search(text[:4096])
    return bool(m and not m.group(1).startswith("AGPL"))


def expected_notice(text: str) -> str:
    lines = text.splitlines()
    block = "\n".join(lines[: leading_block(lines)])
    m = YEAR.search(block)
    return NOTICE.format(year=m.group(1) if m else DEFAULT_YEAR)


def check(text: str) -> bool:
    return text.startswith(expected_notice(text))


def fix(text: str) -> str:
    notice = expected_notice(text)
    if text.startswith(notice):
        return text
    lines = text.splitlines(keepends=True)
    n = leading_block(lines)
    if any(marker in "".join(lines[:n]) for marker in LICENSE_MARKERS):
        lines = lines[n:]
    rest = "".join(lines).lstrip("\n")
    return notice + "\n" + rest


def tracked_sources(root: Path) -> list[str]:
    out = subprocess.run(
        ["git", "ls-files", "-z", "--", *(f"{r}/*.rs" for r in ROOTS)],
        cwd=root,
        check=True,
        capture_output=True,
    ).stdout.decode()
    return [p for p in out.split("\0") if p]


def load_exempt(root: Path) -> set[str]:
    path = root / EXEMPT_LIST
    if not path.exists():
        return set()
    entries = (line.split("#", 1)[0].strip() for line in path.read_text().splitlines())
    return {e for e in entries if e}


def candidates(root: Path) -> tuple[list[str], list[str]]:
    exempt = load_exempt(root)
    files = [
        p
        for p in tracked_sources(root)
        if p not in exempt and not any(part in f"/{p}" for part in EXEMPT_PARTS)
    ]
    present = set(tracked_sources(root))
    stale = sorted(e for e in exempt if e not in present)
    return files, stale


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--fix", action="store_true", help="rewrite failing notices in place")
    parser.add_argument("--root", type=Path, default=Path.cwd())
    args = parser.parse_args(argv)

    files, stale = candidates(args.root)
    bad: list[str] = []
    for rel in files:
        path = args.root / rel
        text = path.read_text(encoding="utf-8")
        if exempt_by_content(text) or check(text):
            continue
        if args.fix:
            path.write_text(fix(text), encoding="utf-8")
        else:
            bad.append(rel)

    for rel in stale:
        print(f"{EXEMPT_LIST}: {rel} is not a tracked Rust source", file=sys.stderr)
    for rel in bad:
        print(f"{rel}: missing or truncated AGPL-3.0 notice", file=sys.stderr)
    if bad:
        print(
            f"{len(bad)} file(s) fail; run `python3 scripts/dev/check-license-headers.py --fix`",
            file=sys.stderr,
        )
    return 1 if bad or stale else 0


if __name__ == "__main__":
    sys.exit(main())
