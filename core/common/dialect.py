"""Dialect enum and detection from the active signature profile."""

from __future__ import annotations

import re
from collections.abc import Iterable, Iterator
from contextlib import contextmanager
from enum import Enum

from ..commands.registry.runtime import (
    _canonical_dialect,
    _dialect_var,
    _extra_commands_var,
    active_signature_profile,
)


class Dialect(Enum):
    """Known Tcl dialect variants supported by the language server."""

    TCL_8_4 = "tcl8.4"
    TCL_8_5 = "tcl8.5"
    TCL_8_6 = "tcl8.6"
    TCL_9_0 = "tcl9.0"
    F5_IRULES = "f5-irules"
    F5_IAPPS = "f5-iapps"
    F5_BIGIP = "f5-bigip"
    SYNOPSYS_EDA = "synopsys-eda-tcl"
    CADENCE_EDA = "cadence-eda-tcl"
    XILINX_EDA = "xilinx-eda-tcl"
    INTEL_QUARTUS_EDA = "intel-quartus-eda-tcl"
    MENTOR_EDA = "mentor-eda-tcl"
    EXPECT = "expect"


def active_dialect() -> str:
    """Return the dialect string for the active signature profile."""
    profile = active_signature_profile()
    dialect = profile.get("dialect")
    if isinstance(dialect, str) and dialect:
        return dialect
    return Dialect.TCL_8_6.value


@contextmanager
def dialect_scope(
    dialect: str | None = None,
    extra_commands: Iterable[str] | None = None,
) -> Iterator[str]:
    """Scope the active dialect (and optional extra_commands) for a request.

    LSP request handlers should wrap their work in this context manager so
    that ``active_dialect()`` / ``SIGNATURES`` / lexer flags reflect the
    folder-resolved dialect of the document being processed rather than a
    process-wide singleton.  See issue #407.

    ``dialect=None`` leaves the active dialect untouched.  Invalid dialect
    strings are silently ignored (the current value is preserved).

    Yields the effective dialect string for callers that want to log or
    branch on it.
    """
    dialect_token = None
    extras_token = None

    if dialect is not None:
        canonical = _canonical_dialect(dialect)
        if canonical is not None:
            dialect_token = _dialect_var.set(canonical)

    if extra_commands is not None:
        normalised = tuple(
            sorted({name.strip() for name in extra_commands if name and name.strip()})
        )
        extras_token = _extra_commands_var.set(normalised)

    try:
        yield _dialect_var.get()
    finally:
        if extras_token is not None:
            _extra_commands_var.reset(extras_token)
        if dialect_token is not None:
            _dialect_var.reset(dialect_token)


_DIALECT_DIRECTIVE_RE = re.compile(r"^#\s*tcl-dialect:\s*(\S+)", re.IGNORECASE)
_SHEBANG_EXPECT_RE = re.compile(r"^#!.*\bexpect\b", re.IGNORECASE)
_SHEBANG_TCLSH_RE = re.compile(r"^#!.*\btclsh(\d+\.\d+)\b", re.IGNORECASE)
_PKG_REQUIRE_TCL_RE = re.compile(r"^\s*package\s+require\s+(?:-exact\s+)?Tcl\s*(\d+\.\d+)")

_TCL_VERSION_MAP: dict[str, str] = {
    "8.4": "tcl8.4",
    "8.5": "tcl8.5",
    "8.6": "tcl8.6",
    "9.0": "tcl9.0",
}

DIALECT_DIRECTIVE_SCAN_LINES = 5

# Scan more lines for package require — it often appears after comments.
_PKG_REQUIRE_SCAN_LINES = 30


def detect_dialect_from_source(source: str) -> str | None:
    """Detect a dialect from source content.

    Checks (in priority order):
    1. ``# tcl-dialect: <dialect>`` comment directive (first 5 lines)
    2. Shebang (first line)
    3. ``package require Tcl <version>`` (first 30 lines)

    Returns ``None`` if no dialect hint is found.
    """
    from ..commands.registry.dialects import KNOWN_DIALECTS

    lines = source.split("\n", _PKG_REQUIRE_SCAN_LINES)

    # Comment directive (``# tcl-dialect: <dialect>``)
    for line in lines[:DIALECT_DIRECTIVE_SCAN_LINES]:
        m = _DIALECT_DIRECTIVE_RE.match(line)
        if m:
            candidate = m.group(1).strip().lower()
            if candidate in KNOWN_DIALECTS:
                return candidate

    # Shebang detection (first line only)
    if lines:
        first = lines[0]
        if _SHEBANG_EXPECT_RE.match(first):
            return "expect"
        m = _SHEBANG_TCLSH_RE.match(first)
        if m:
            return _TCL_VERSION_MAP.get(m.group(1))

    # package require Tcl <version>
    for line in lines[:_PKG_REQUIRE_SCAN_LINES]:
        m = _PKG_REQUIRE_TCL_RE.match(line)
        if m:
            return _TCL_VERSION_MAP.get(m.group(1))

    # Conf-wrapped iRules: ``ltm rule /path { ... }`` or ``gtm rule /path { ... }``
    from ..bigip.rule_extract import is_conf_wrapped_irules

    if is_conf_wrapped_irules(source):
        return "f5-irules"

    return None
