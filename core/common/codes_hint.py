"""Hint diagnostic codes (H-series)."""

from core.common.codes import diag

H300 = diag(
    "H300",
    "Possible paste error — repeated assignment to same variable with same value.",
    section="hint",
)
