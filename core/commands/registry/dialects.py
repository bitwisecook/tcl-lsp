"""Canonical dialect constants shared across the registry package."""

from __future__ import annotations

KNOWN_DIALECTS: frozenset[str] = frozenset(
    (
        "tcl8.4",
        "tcl8.5",
        "tcl8.6",
        "tcl9.0",
        "f5-irules",
        "f5-iapps",
        "synopsys-eda-tcl",
        "cadence-eda-tcl",
        "xilinx-eda-tcl",
        "intel-quartus-eda-tcl",
        "mentor-eda-tcl",
        "expect",
    )
)

# Positive dialect set for commands available everywhere except iRules.
DIALECTS_EXCEPT_IRULES: frozenset[str] = KNOWN_DIALECTS - frozenset({"f5-irules"})
