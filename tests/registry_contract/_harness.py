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

"""Helpers for driving the front-end CLIs and loading the golden fixtures.

The contract tests treat the CLIs as black boxes: they invoke a verb and
parse its output in-process via the CLI ``main(argv)``, exactly as an
external consumer (or a Rust reimplementation) would.  Structural and
presence checks read the registry / CSVs directly instead.
"""

from __future__ import annotations

import contextlib
import csv
import io
import json
import os
import tempfile
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
BASELINE_DIR = ROOT / "tests" / "baselines" / "registry"


def load_csv(name: str) -> list[dict[str, str]]:
    """Load a golden CSV fixture (e.g. ``"commands.csv"``) as a list of rows."""
    with (BASELINE_DIR / name).open(encoding="utf-8", newline="") as fh:
        return list(csv.DictReader(fh))


def _run_main(main: Any, argv: list[str]) -> tuple[int, str]:
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        rc = main(argv)
    return rc, buf.getvalue()


def run_f5(argv: list[str]) -> tuple[int, str]:
    """Run the ``f5`` CLI in-process; return ``(returncode, stdout)``."""
    from tooling.f5.main import main

    return _run_main(main, argv)


def run_f5_json(argv: list[str]) -> Any:
    rc, out = run_f5(argv)
    assert rc == 0, f"f5 {argv} exited {rc}: {out!r}"
    return json.loads(out)


@dataclass(frozen=True)
class Diagnostic:
    code: str
    message: str
    line: int
    column: int
    severity: str


def _parse_diag_array(stdout: str) -> list[Diagnostic]:
    # ``tcl diag --json`` prints a JSON array of {file, diagnostics:[...]}
    # followed by a one-line human summary; slice to the closing bracket.
    end = stdout.rfind("]")
    arr = json.loads(stdout[: end + 1])
    out: list[Diagnostic] = []
    for f in arr:
        for d in f.get("diagnostics", []):
            out.append(
                Diagnostic(
                    code=d.get("code", ""),
                    message=d.get("message", ""),
                    line=d.get("line", 0),
                    column=d.get("column", 0),
                    severity=d.get("severity", ""),
                )
            )
    return out


def tcl_diagnostics(source: str, *, dialect: str) -> list[Diagnostic]:
    """Run ``tcl diag --json`` over *source* and return parsed diagnostics."""
    from tooling.tcl.main import main

    suffix = ".irule" if dialect.startswith("f5") else ".tcl"
    fd, path = tempfile.mkstemp(suffix=suffix)
    try:
        os.write(fd, source.encode("utf-8"))
        os.close(fd)
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            main(["diag", path, "--json", "--dialect", dialect])
        return _parse_diag_array(buf.getvalue())
    finally:
        if os.path.exists(path):
            os.unlink(path)


def tcl_diag_codes(source: str, *, dialect: str) -> set[str]:
    """The set of diagnostic codes ``tcl diag`` reports for *source*."""
    return {d.code for d in tcl_diagnostics(source, dialect=dialect)}


def f5_event_order(source: str) -> list[str]:
    """Run ``f5 irule event-order --json`` and return the ordered event names."""
    fd, path = tempfile.mkstemp(suffix=".irule")
    try:
        os.write(fd, source.encode("utf-8"))
        os.close(fd)
        data = run_f5_json(["irule", "event-order", path, "--json"])
        return [e["name"] for e in data["events"]]
    finally:
        if os.path.exists(path):
            os.unlink(path)


@dataclass(frozen=True)
class DiagCase:
    """A generated behavioural case: source plus the codes it must (not) raise."""

    cid: str
    source: str
    dialect: str
    must_have: frozenset[str] = field(default_factory=frozenset)
    must_not: frozenset[str] = field(default_factory=frozenset)

    def check(self) -> str | None:
        """Run the case; return a failure description or ``None`` when it passes."""
        codes = tcl_diag_codes(self.source, dialect=self.dialect)
        missing = self.must_have - codes
        present = self.must_not & codes
        if missing or present:
            parts = []
            if missing:
                parts.append(f"missing {sorted(missing)}")
            if present:
                parts.append(f"unexpected {sorted(present)}")
            return f"{self.cid}: {', '.join(parts)} (source={self.source!r})"
        return None
