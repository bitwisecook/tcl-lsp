"""Dialect enum and detection from the active signature profile."""

from __future__ import annotations

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
