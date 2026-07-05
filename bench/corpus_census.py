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

"""Corpus-wide diagnostic census + quick-fix verification.

--count : tally every diagnostic code over the WHOLE corpus (tcllib, Tcl 9.0
          stdlib, tklib, tdom), highest first — the worklist for precision audits.
--verify-fixes : for every diagnostic carrying a quick-fix, apply the fix and
          re-analyse, flagging misfires: (a) the fix doesn't clear the original
          diagnostic, (b) it introduces a NEW syntax/error diagnostic
          (E-series) — i.e. an alignment/range error or a semantically broken
          rewrite.  Reports per-code misfire counts + examples.
"""

from __future__ import annotations

import argparse
import collections
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from server.features.diagnostics import get_diagnostics  # noqa: E402

CORPUS_ROOTS = [
    "tmp/tcllib-2.0/modules",
    "tmp/tcl9.0.3/library",
    "tmp/tklib-0.9/modules",
    "tmp/tdom-master",
]


def _files(limit: int | None) -> list[Path]:
    out: list[Path] = []
    for r in CORPUS_ROOTS:
        b = Path(r)
        if b.exists():
            out += [p for p in sorted(b.rglob("*.tcl")) if 0 < p.stat().st_size <= 600_000]
    out.sort()
    if limit:
        out = out[:limit]
    return out


def _is_error(code: str) -> bool:
    return code.startswith("E")


def _write_tally(path: str, tally, done: int, total_files: int, errs: int) -> None:
    lines = [f"# {done}/{total_files} files, {sum(tally.values())} diagnostics, {errs} errors"]
    lines += [f"{code:8s} {n:7d}" for code, n in tally.most_common()]
    Path(path).write_text("\n".join(lines) + "\n")


def count(limit: int | None) -> None:
    tally: collections.Counter[str] = collections.Counter()
    files = _files(limit)
    out = "tmp/census_counts.txt"
    errs = 0
    for i, p in enumerate(files, 1):
        try:
            for d in get_diagnostics(p.read_text(errors="replace")):
                tally[d.code] += 1
        except Exception:
            errs += 1
        if i % 150 == 0:
            _write_tally(out, tally, i, len(files), errs)  # checkpoint
    _write_tally(out, tally, len(files), len(files), errs)
    print(Path(out).read_text())


def _apply(src: str, fix) -> str | None:
    """Apply a single-edit CodeFix; return None if it isn't a clean single edit."""
    r = getattr(fix, "range", None)
    if r is None:
        return None
    so = r.start.offset
    eo = r.end.offset
    if not (0 <= so <= eo <= len(src)):
        return None
    return src[:so] + fix.new_text + src[eo:]


def verify_fixes(limit: int | None) -> None:
    misfire_clear: collections.Counter[str] = collections.Counter()
    misfire_newerr: collections.Counter[str] = collections.Counter()
    applied: collections.Counter[str] = collections.Counter()
    ex_clear: dict[str, list] = collections.defaultdict(list)
    ex_newerr: dict[str, list] = collections.defaultdict(list)
    for p in _files(limit):
        try:
            src = p.read_text(errors="replace")
            diags = get_diagnostics(src)
        except Exception:
            continue
        before_errors = sum(1 for d in diags if _is_error(d.code))
        for d in diags:
            for fix in getattr(d, "fixes", None) or ():
                fixed = _apply(src, fix)
                if fixed is None or fixed == src:
                    continue
                applied[d.code] += 1
                try:
                    after = get_diagnostics(fixed)
                except Exception:
                    misfire_newerr[d.code] += 1
                    if len(ex_newerr[d.code]) < 3:
                        ex_newerr[d.code].append((p.name, d.range.start.line, "analyse crash"))
                    continue
                # (a) original diagnostic at that location cleared?
                key = (d.code, d.range.start.line, d.range.start.character)
                still = any(
                    (x.code, x.range.start.line, x.range.start.character) == key for x in after
                )
                if still:
                    misfire_clear[d.code] += 1
                    if len(ex_clear[d.code]) < 3:
                        ex_clear[d.code].append((p.name, d.range.start.line))
                # (b) introduced a new E-series error?
                after_errors = sum(1 for x in after if _is_error(x.code))
                if after_errors > before_errors:
                    misfire_newerr[d.code] += 1
                    if len(ex_newerr[d.code]) < 3:
                        ex_newerr[d.code].append((p.name, d.range.start.line, fix.new_text[:40]))
    print("# quick-fix verification (apply -> re-analyse)")
    print(f"{'code':8s} {'applied':>8} {'notCleared':>11} {'newError':>9}")
    for code in sorted(applied, key=lambda c: -applied[c]):
        print(f"{code:8s} {applied[code]:8d} {misfire_clear[code]:11d} {misfire_newerr[code]:9d}")
    for code in sorted(set(ex_clear) | set(ex_newerr)):
        if ex_clear[code]:
            print(f"  {code} NOT-CLEARED e.g. {ex_clear[code]}")
        if ex_newerr[code]:
            print(f"  {code} NEW-ERROR    e.g. {ex_newerr[code]}")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--count", action="store_true")
    ap.add_argument("--verify-fixes", action="store_true")
    ap.add_argument("--limit", type=int, default=None)
    args = ap.parse_args()
    if args.count:
        count(args.limit)
    elif args.verify_fixes:
        verify_fixes(args.limit)
    else:
        ap.print_help()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
