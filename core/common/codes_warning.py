"""Warning diagnostic codes (W-series)."""

from core.common.codes import diag

# --- Style & Best Practice (section="warning") ---
W001 = diag("W001", "Unknown subcommand.", section="warning")
W002 = diag("W002", "Command is disabled in active dialect profile.", section="warning")
W100 = diag(
    "W100",
    "Unbraced expression argument — prevents byte-compilation and risks double substitution.",
    section="warning",
)
W104 = diag(
    "W104", "String concatenation for list building — use `lappend` instead.", section="warning"
)
W105 = diag(
    "W105",
    "Unbraced code block or missing `variable` declaration in `namespace eval`.",
    section="warning",
)
W106 = diag(
    "W106", "Dangerous unbraced `switch` body — risks double substitution.", section="warning"
)
W108 = diag("W108", "Non-ASCII characters in token content.", section="warning")
W110 = diag("W110", "Use `eq`/`ne` instead of `==`/`!=` for string comparison.", section="warning")
W111 = diag(
    "W111", "Line exceeds maximum length (see `tclLsp.style.lineLength`).", section="warning"
)
W112 = diag("W112", "Trailing whitespace.", section="warning")
W113 = diag("W113", "Procedure shadows built-in command.", section="warning")
W114 = diag(
    "W114", "Redundant nested `[expr {...}]` — already in expression context.", section="warning"
)
W115 = diag(
    "W115", "Backslash-newline in comment silently swallows the next line.", section="warning"
)
W120 = diag("W120", "Command used without a corresponding `package require`.", section="warning")
W121 = diag("W121", "Subnet mask has non-contiguous bits.", section="warning")
W122 = diag("W122", "Mistyped IPv4 address (octet > 255 or leading zero).", section="warning")
W200 = diag(
    "W200",
    "`exec` result not captured or binary format modifier requires newer Tcl.",
    section="warning",
)
W201 = diag("W201", "Manual path concatenation — use `file join` instead.", section="warning")

# --- Variables (section="variable") ---
W210 = diag("W210", "Variable read before set.", section="variable")
W211 = diag("W211", "Variable set but never used.", section="variable")
W212 = diag(
    "W212",
    "Variable substitution where name expected (`set $x`, `incr $x`, `info exists $x`, etc.).",
    section="variable",
)
W213 = diag(
    "W213",
    "Variable may not exist — use `unset -nocomplain` to suppress the error.",
    section="variable",
)
W214 = diag(
    "W214",
    "Unused proc parameter — argument is declared but never read in the procedure body.",
    section="variable",
)
W220 = diag("W220", "Dead store — variable set but overwritten before use.", section="variable")

# --- Security (section="security") ---
W101 = diag("W101", "`eval` with string concatenation — code injection risk.", section="security")
W102 = diag("W102", "`subst` on variable input — code injection risk.", section="security")
W103 = diag("W103", "`open` with pipeline `|` — command injection risk.", section="security")
W300 = diag("W300", "`source` with variable argument — code execution risk.", section="security")
W301 = diag("W301", "`uplevel` with string-built script — injection risk.", section="security")
W302 = diag(
    "W302", "`catch` without result variable — errors are silently swallowed.", section="security"
)
W303 = diag("W303", "Regexp vulnerable to catastrophic backtracking (ReDoS).", section="security")
W304 = diag(
    "W304", "Missing option terminator `--` on option-bearing commands.", section="security"
)
W306 = diag("W306", "Substitution in literal-expected argument position.", section="security")
W307 = diag(
    "W307",
    "Non-literal command name — variable or command substitution as command.",
    section="security",
)
W308 = diag(
    "W308",
    "`subst` without `-nocommands` — risk of unintended command execution.",
    section="security",
)
W309 = diag("W309", "`eval`/`uplevel` with `subst` — double substitution risk.", section="security")

# --- Internal (not yet surfaced to editors) ---
W310 = diag("W310", "Hardcoded credentials in source.", section="security", internal=True)
W311 = diag("W311", "Destructive file operation.", section="security", internal=True)
W312 = diag("W312", "`interp eval` injection risk.", section="security", internal=True)
W313 = diag("W313", "Encoding mismatch.", section="security", internal=True)
