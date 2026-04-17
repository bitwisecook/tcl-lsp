"""Tk package detection utility."""

from __future__ import annotations

from enum import Enum

from ..analysis.semantic_model import AnalysisResult


class TkMode(Enum):
    """Whether Tk commands should be available in a given file."""

    ENABLED = "enabled"  # ``package require Tk`` or wish shebang
    DISABLED = "disabled"  # tclsh shebang, no Tk require
    UNKNOWN = "unknown"  # no clear signal


def has_tk_require(analysis: AnalysisResult) -> bool:
    """Return True if the analysis detected ``package require Tk``."""
    return any(pr.name == "Tk" for pr in analysis.package_requires)


def infer_tk_mode(
    source: str,
    analysis: AnalysisResult,
    interpreter_override: str | None = None,
) -> TkMode:
    """Infer whether Tk commands should be available for a file.

    Inference rules (in priority order):

    1. LSP config override (``interpreter_override``) — ``"wish"`` → ENABLED,
       ``"tclsh"`` → DISABLED.
    2. ``package require Tk`` in source → ENABLED.
    3. Shebang ``#!/usr/bin/wish`` or ``#!/usr/bin/env wish`` → ENABLED.
    4. Shebang ``#!/usr/bin/tclsh`` or ``#!/usr/bin/env tclsh`` → DISABLED
       (unless rule 2 already set ENABLED).
    5. No signal → UNKNOWN.
    """
    # Rule 1: explicit interpreter override from LSP config.
    if interpreter_override:
        interp = interpreter_override.lower()
        if "wish" in interp:
            return TkMode.ENABLED
        if "tclsh" in interp:
            return TkMode.DISABLED

    # Rule 2: explicit package require Tk.
    if has_tk_require(analysis):
        return TkMode.ENABLED

    # Rule 3-4: shebang detection.
    shebang = _parse_shebang(source)
    if shebang is not None:
        if "wish" in shebang:
            return TkMode.ENABLED
        if "tclsh" in shebang:
            return TkMode.DISABLED

    return TkMode.UNKNOWN


def _parse_shebang(source: str) -> str | None:
    """Extract the interpreter name from a ``#!`` shebang line, if any."""
    if not source.startswith("#!"):
        return None
    # Find end of first line.
    end = source.find("\n")
    if end < 0:
        end = len(source)
    line = source[2:end].strip()
    if not line:
        return None
    # Handle ``/usr/bin/env wish`` form.
    parts = line.split()
    if parts and parts[0].endswith("/env") and len(parts) > 1:
        return parts[1]
    # Direct path form: ``/usr/bin/wish8.6`` → ``wish8.6``.
    return parts[0].rsplit("/", 1)[-1] if parts else None
