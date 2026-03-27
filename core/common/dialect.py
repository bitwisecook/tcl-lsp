"""Dialect enum and detection from the active signature profile."""

from __future__ import annotations

import re
from enum import Enum

from ..commands.registry.runtime import active_signature_profile


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


_DIALECT_DIRECTIVE_RE = re.compile(r"^#\s*tcl-dialect:\s*(\S+)", re.IGNORECASE)
_SHEBANG_EXPECT_RE = re.compile(r"^#!.*\bexpect\b", re.IGNORECASE)
_SHEBANG_TCLSH_RE = re.compile(r"^#!.*\btclsh(\d+\.\d+)\b", re.IGNORECASE)

_SHEBANG_VERSION_MAP: dict[str, str] = {
    "8.4": "tcl8.4",
    "8.5": "tcl8.5",
    "8.6": "tcl8.6",
    "9.0": "tcl9.0",
}

DIALECT_DIRECTIVE_SCAN_LINES = 5


def detect_dialect_from_source(source: str) -> str | None:
    """Detect a dialect from source content via comment directive or shebang.

    Checks the first few lines for a ``# tcl-dialect: <dialect>`` comment
    directive, then falls back to shebang detection.  Returns ``None`` if
    no dialect hint is found.
    """
    from ..commands.registry.dialects import KNOWN_DIALECTS

    lines = source.split("\n", DIALECT_DIRECTIVE_SCAN_LINES)

    # Comment directive (``# tcl-dialect: <dialect>``)
    for line in lines[:DIALECT_DIRECTIVE_SCAN_LINES]:
        m = _DIALECT_DIRECTIVE_RE.match(line)
        if m:
            candidate = m.group(1).lower()
            if candidate in KNOWN_DIALECTS:
                return candidate

    # Shebang detection (first line only)
    if lines:
        first = lines[0]
        if _SHEBANG_EXPECT_RE.match(first):
            return "expect"
        m = _SHEBANG_TCLSH_RE.match(first)
        if m:
            return _SHEBANG_VERSION_MAP.get(m.group(1))

    return None
