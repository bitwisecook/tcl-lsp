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

"""Per-iteration interpreter instruction counts under callgrind.

Each benchmark in ``interp/`` runs at two iteration counts; the difference in
total instructions divided by the difference in iterations is the cost of one
iteration, free of startup and teardown. Instruction counts are deterministic
for one binary on one machine, so comparing a base and a head build measured
in the same job is a real gate where wall time is not.

    python3 interp_instructions.py measure --engine tclvm=PATH \\
        --engine runtime=PATH --out head.json
    python3 interp_instructions.py compare base.json head.json --tolerance 0.02
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

BENCHMARKS = Path(__file__).resolve().parent / "interp"
SMALL = 300
LARGE = 900
TOTALS = re.compile(r"^(?:summary|totals):\s+(\d+)", re.MULTILINE)


def engine_command(name: str, binary: str, script: Path) -> list[str]:
    """The VM CLI takes the script; the runtime example also wants --quiet."""
    if name.startswith("runtime"):
        return [binary, "--quiet", str(script)]
    return [binary, str(script)]


def total_instructions(callgrind_out: str) -> int:
    match = TOTALS.search(callgrind_out)
    if match is None:
        raise ValueError("callgrind output has no summary or totals line")
    return int(match.group(1))


def per_iteration(
    small_total: int, large_total: int, span: int = LARGE - SMALL
) -> float:
    return (large_total - small_total) / span


def render(source: str, iterations: int) -> str:
    if "@N@" not in source:
        raise ValueError("benchmark has no @N@ iteration placeholder")
    return source.replace("@N@", str(iterations))


def run_callgrind(command: list[str], workdir: Path) -> int:
    out = workdir / "callgrind.out"
    subprocess.run(
        ["valgrind", "--tool=callgrind", f"--callgrind-out-file={out}", *command],
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    return total_instructions(out.read_text())


def measure(
    engines: dict[str, str], benchmarks: list[Path]
) -> dict[str, dict[str, float]]:
    results: dict[str, dict[str, float]] = {}
    with tempfile.TemporaryDirectory() as tmp:
        workdir = Path(tmp)
        for name, binary in engines.items():
            results[name] = {}
            for bench in benchmarks:
                totals = []
                for iterations in (SMALL, LARGE):
                    script = workdir / f"{bench.stem}.{iterations}.tcl"
                    script.write_text(render(bench.read_text(), iterations))
                    totals.append(
                        run_callgrind(engine_command(name, binary, script), workdir)
                    )
                results[name][bench.stem] = per_iteration(*totals)
    return results


@dataclass(frozen=True)
class Row:
    engine: str
    benchmark: str
    base: float
    head: float

    @property
    def change(self) -> float:
        return (self.head - self.base) / self.base if self.base else 0.0


def compare(
    base: dict[str, dict[str, float]],
    head: dict[str, dict[str, float]],
    tolerance: float,
) -> tuple[list[Row], list[Row]]:
    rows = [
        Row(engine, bench, base[engine][bench], value)
        for engine, benches in sorted(head.items())
        if engine in base
        for bench, value in sorted(benches.items())
        if bench in base[engine]
    ]
    return rows, [row for row in rows if row.change > tolerance]


def table(rows: list[Row], tolerance: float) -> str:
    lines = [
        "| Engine | Benchmark | Base | Head | Change |",
        "| --- | --- | ---: | ---: | ---: |",
    ]
    for row in rows:
        flag = " ⚠" if row.change > tolerance else ""
        lines.append(
            f"| {row.engine} | {row.benchmark} | {row.base:,.0f} | {row.head:,.0f} "
            f"| {row.change:+.1%}{flag} |"
        )
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="command", required=True)
    run = sub.add_parser("measure")
    run.add_argument("--engine", action="append", required=True, metavar="NAME=PATH")
    run.add_argument("--benchmark", action="append", metavar="NAME")
    run.add_argument("--out", type=Path, required=True)
    diff = sub.add_parser("compare")
    diff.add_argument("base", type=Path)
    diff.add_argument("head", type=Path)
    diff.add_argument("--tolerance", type=float, default=0.02)
    args = parser.parse_args(argv)

    if args.command == "measure":
        engines = dict(spec.split("=", 1) for spec in args.engine)
        benchmarks = sorted(BENCHMARKS.glob("*.tcl"))
        if args.benchmark:
            benchmarks = [b for b in benchmarks if b.stem in set(args.benchmark)]
        results = measure(engines, benchmarks)
        args.out.write_text(json.dumps(results, indent=2, sort_keys=True) + "\n")
        print(json.dumps(results, indent=2, sort_keys=True))
        return 0

    rows, regressions = compare(
        json.loads(args.base.read_text()),
        json.loads(args.head.read_text()),
        args.tolerance,
    )
    print(table(rows, args.tolerance))
    if regressions:
        print(
            f"\n{len(regressions)} benchmark(s) regressed by more than {args.tolerance:.0%}."
        )
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
