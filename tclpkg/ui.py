"""CLI output helpers — progress, colour, JSON mode.

These helpers centralise the conventions shared by all ``tcl pkg`` and
``tcl venv`` subcommands: ANSI colour codes, ``--json`` mode, progress
lines on stderr, and the check/cross/warning symbols.
"""

from __future__ import annotations

import json
import os
import sys
from typing import Any

_COLOUR_RESET = "\033[0m"
_COLOUR_GREEN = "\033[32m"
_COLOUR_RED = "\033[31m"
_COLOUR_YELLOW = "\033[33m"
_COLOUR_CYAN = "\033[36m"
_COLOUR_DIM = "\033[2m"
_COLOUR_BOLD = "\033[1m"


def _is_stdout_tty() -> bool:
    try:
        return sys.stdout.isatty()
    except Exception:
        return False


def _is_stderr_tty() -> bool:
    try:
        return sys.stderr.isatty()
    except Exception:
        return False


def use_colour(*, force: bool | None = None) -> bool:
    """Decide whether to emit ANSI colour codes.

    *force* overrides auto-detection: ``True`` always colours,
    ``False`` never.  ``None`` (default) auto-detects from the
    terminal and ``NO_COLOR`` / ``FORCE_COLOR`` env vars.
    """
    if force is True:
        return True
    if force is False:
        return False
    if os.environ.get("NO_COLOR"):
        return False
    if os.environ.get("FORCE_COLOR"):
        return True
    return _is_stdout_tty()


def _c(code: str, text: str, *, colour: bool) -> str:
    if not colour:
        return text
    return f"{code}{text}{_COLOUR_RESET}"


def ok(text: str, *, colour: bool = True) -> str:
    return _c(_COLOUR_GREEN, f"  ✓ {text}", colour=colour)


def fail(text: str, *, colour: bool = True) -> str:
    return _c(_COLOUR_RED, f"  ✗ {text}", colour=colour)


def warn(text: str, *, colour: bool = True) -> str:
    return _c(_COLOUR_YELLOW, f"  ⚠ {text}", colour=colour)


def info(text: str, *, colour: bool = True) -> str:
    return _c(_COLOUR_CYAN, text, colour=colour)


def dim(text: str, *, colour: bool = True) -> str:
    return _c(_COLOUR_DIM, text, colour=colour)


def bold(text: str, *, colour: bool = True) -> str:
    return _c(_COLOUR_BOLD, text, colour=colour)


def progress(message: str) -> None:
    """Print a progress line to stderr (overwritten by the next call)."""
    if _is_stderr_tty():
        sys.stderr.write(f"\r  {message}\033[K")
        sys.stderr.flush()
    else:
        sys.stderr.write(f"  {message}\n")


def progress_done() -> None:
    """Clear the progress line."""
    if _is_stderr_tty():
        sys.stderr.write("\r\033[K")
        sys.stderr.flush()


def json_output(data: Any) -> None:
    """Print JSON to stdout (used when ``--json`` is active)."""
    print(json.dumps(data, indent=2, sort_keys=True, ensure_ascii=False))


__all__ = [
    "bold",
    "dim",
    "fail",
    "info",
    "json_output",
    "ok",
    "progress",
    "progress_done",
    "use_colour",
    "warn",
]
