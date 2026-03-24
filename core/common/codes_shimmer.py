"""Shimmer diagnostic codes (S-series)."""

from core.common.codes import diag

S100 = diag(
    "S100",
    "Single shimmer outside a loop — object internal representation changed.",
    section="shimmer",
)
S101 = diag(
    "S101",
    "Shimmer inside a loop body — per-iteration representation conversion cost.",
    section="shimmer",
)
S102 = diag(
    "S102", "Variable oscillates between two types across loop iterations.", section="shimmer"
)
