"""Taint diagnostic codes (T-series)."""

from core.common.codes import diag

# --- Public ---
T100 = diag(
    "T100",
    "Tainted data flows into a dangerous code-execution sink (`eval`, `expr`, `exec`, `uplevel`, `subst`).",
    section="taint",
)
T101 = diag("T101", "Tainted data flows into an output command (`puts`).", section="taint")
T102 = diag(
    "T102",
    "Tainted data in option position without `--` terminator — option injection risk.",
    section="taint",
)

# --- Internal ---
T103 = diag("T103", "Taint propagation through variable.", section="taint", internal=True)
T106 = diag("T106", "Taint propagation through command return.", section="taint", internal=True)
