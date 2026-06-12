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
import io
import json
import subprocess
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
BASELINE_DIR = ROOT / "tests" / "baselines" / "registry"


def load_fixture(name: str) -> Any:
    """Load a golden fixture (e.g. ``"commands-tcl8.6.json"``)."""
    return json.loads((BASELINE_DIR / name).read_text(encoding="utf-8"))


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
