"""Error diagnostic codes (E-series)."""

from core.common.codes import diag

# --- Public (user-configurable) ---
E001 = diag(
    "E001", "Missing subcommand — e.g. bare `string` without a subcommand.", section="error"
)
E002 = diag("E002", "Too few arguments for command.", section="error")
E003 = diag("E003", "Too many arguments for command.", section="error")
E200 = diag(
    "E200", "Shimmer parse error — internal representation cannot be determined.", section="error"
)

# --- Internal (always active, not toggleable) ---
E004 = diag("E004", "Invalid argument count.", section="error", internal=True)
E100 = diag("E100", "Syntax error — unclosed brace.", section="error", internal=True)
E101 = diag("E101", "Syntax error — unclosed bracket.", section="error", internal=True)
E102 = diag("E102", "Syntax error — unclosed quote.", section="error", internal=True)
E103 = diag("E103", "Syntax error — unexpected token.", section="error", internal=True)
E201 = diag("E201", "Parser recovery — unclosed brace.", section="error", internal=True)
E202 = diag("E202", "Parser recovery — unclosed bracket.", section="error", internal=True)
E203 = diag("E203", "Parser recovery — unclosed quote.", section="error", internal=True)
