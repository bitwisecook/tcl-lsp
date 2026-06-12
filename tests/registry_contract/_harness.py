"""Helpers for driving the front-end CLIs and loading the golden fixtures.

The contract tests treat the CLIs as black boxes: they invoke a verb and
parse its JSON, exactly as an external consumer (or a Rust reimplementation
driven over a subprocess) would.  Two execution modes are offered:

- :func:`run_tcl` / :func:`run_f5` call the CLI ``main(argv)`` in-process
  and capture stdout.  This exercises the full argparse -> handler -> JSON
  path and is fast enough to run over every command/event.
- :func:`run_tcl_subprocess` shells out to the installed console script
  (or ``python -m``) so a handful of smoke tests prove the real wire.
"""

from __future__ import annotations

import contextlib
import csv
import io
import json
import os
import subprocess
import sys
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


def run_tcl(argv: list[str]) -> tuple[int, str]:
    """Run the ``tcl`` CLI in-process; return ``(returncode, stdout)``."""
    from tooling.tcl.main import main

    return _run_main(main, argv)


def run_f5(argv: list[str]) -> tuple[int, str]:
    """Run the ``f5`` CLI in-process; return ``(returncode, stdout)``."""
    from tooling.f5.main import main

    return _run_main(main, argv)


def run_tcl_json(argv: list[str]) -> Any:
    rc, out = run_tcl(argv)
    assert rc == 0, f"tcl {argv} exited {rc}: {out!r}"
    return json.loads(out)


def run_f5_json(argv: list[str]) -> Any:
    rc, out = run_f5(argv)
    assert rc == 0, f"f5 {argv} exited {rc}: {out!r}"
    return json.loads(out)


def run_tcl_subprocess(argv: list[str]) -> Any:
    """Run the ``tcl`` CLI as a real subprocess and parse its JSON stdout."""
    proc = subprocess.run(
        [sys.executable, "-m", "tooling.tcl.main", *argv],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=True,
    )
    return json.loads(proc.stdout)


def run_f5_subprocess(argv: list[str]) -> Any:
    proc = subprocess.run(
        [sys.executable, "-m", "tooling.f5.main", *argv],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=True,
    )
    return json.loads(proc.stdout)


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
