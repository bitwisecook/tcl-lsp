#!/usr/bin/env python3
"""Regenerate ``.expected`` files for every sample under
``samples/tcl9_smoke/`` by running the matching ``.tcl`` through
``tclsh9.0``.

The refgen is idempotent and safe to re-run.  Typical use:

    python3 scripts/tcl9_samples_refgen.py

Each sample's stdout becomes the new ``.expected``.  Samples whose
``tcl`` file is missing or whose ``tclsh9.0`` run fails are
reported but do not abort the batch.

Requires ``tclsh9.0`` on ``$PATH``.  When the binary is not
available, falls back to printing a diagnostic without overwriting
anything so the pre-existing committed ``.expected`` survives.

Cite: samples/tcl9_smoke/README.md — authoring workflow.
"""

from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path

_REPO_ROOT = Path(__file__).resolve().parents[2]
_SAMPLES_DIR = _REPO_ROOT / "samples" / "tcl9_smoke"


def _find_tclsh() -> str | None:
    """Return the first tclsh binary found on PATH (preferring 9.0)."""
    for name in ("tclsh9.0", "tclsh9", "tclsh"):
        resolved = shutil.which(name)
        if resolved is not None:
            return resolved
    return None


def _run(tclsh: str, tcl_path: Path) -> str:
    res = subprocess.run(
        [tclsh, str(tcl_path)],
        check=True,
        capture_output=True,
        text=True,
        timeout=30,
    )
    return res.stdout


def main() -> int:
    tclsh = _find_tclsh()
    if tclsh is None:
        print(
            "No tclsh9.0 / tclsh9 / tclsh on PATH — leaving existing .expected files untouched.",
            file=sys.stderr,
        )
        return 0

    if not _SAMPLES_DIR.is_dir():
        print(f"No samples dir at {_SAMPLES_DIR}", file=sys.stderr)
        return 1

    errors = 0
    written = 0
    for tcl in sorted(_SAMPLES_DIR.rglob("*.tcl")):
        expected_path = tcl.with_suffix(".expected")
        try:
            out = _run(tclsh, tcl)
        except subprocess.CalledProcessError as exc:
            print(
                f"FAIL {tcl.relative_to(_SAMPLES_DIR)}: {exc.stderr.strip()}",
                file=sys.stderr,
            )
            errors += 1
            continue
        except subprocess.TimeoutExpired:
            print(
                f"FAIL {tcl.relative_to(_SAMPLES_DIR)}: timed out",
                file=sys.stderr,
            )
            errors += 1
            continue
        expected_path.write_text(out, encoding="utf-8")
        written += 1

    print(f"Regenerated {written} .expected file(s); {errors} failures.")
    return 1 if errors else 0


if __name__ == "__main__":
    raise SystemExit(main())
