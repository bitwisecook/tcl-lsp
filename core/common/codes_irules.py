"""iRules diagnostic codes (IRULE-series)."""

from core.common.codes import diag

# --- iRules general (section="irules") ---
IRULE1001 = diag(
    "IRULE1001", "Command invalid or ineffective in this iRules event.", section="irules"
)
IRULE1002 = diag("IRULE1002", "Unknown iRules event name.", section="irules")
IRULE1003 = diag("IRULE1003", "Deprecated iRules event.", section="irules")
IRULE1004 = diag("IRULE1004", "`when` block missing explicit `priority`.", section="irules")
IRULE1005 = diag("IRULE1005", "Data event without a matching `*::collect` call.", section="irules")
IRULE1006 = diag(
    "IRULE1006", "`*::payload` without a matching `*::collect` call.", section="irules"
)
IRULE1007 = diag(
    "IRULE1007",
    "`*::collect` without a matching `*::release` on the same connection side.",
    section="irules",
)
IRULE1008 = diag(
    "IRULE1008",
    "`*::release` without a matching `*::collect` on the same connection side.",
    section="irules",
)
IRULE1201 = diag(
    "IRULE1201", "HTTP command used after `HTTP::respond`/`HTTP::redirect`.", section="irules"
)
IRULE1202 = diag(
    "IRULE1202",
    "Multiple `HTTP::respond`/`HTTP::redirect` on different branches.",
    section="irules",
)
IRULE2001 = diag(
    "IRULE2001", "Deprecated `matchclass` — use `class match` instead.", section="irules"
)
IRULE2002 = diag("IRULE2002", "Deprecated iRules command.", section="irules")
IRULE2003 = diag("IRULE2003", "Unsafe iRules command.", section="irules")
IRULE2101 = diag(
    "IRULE2101",
    "Heavy `regexp` in a high-frequency event — consider `string match` or data-group.",
    section="irules",
)
IRULE5001 = diag("IRULE5001", "Ungated `log` in a high-frequency event.", section="irules")
IRULE5002 = diag(
    "IRULE5002",
    "`drop`/`reject`/`discard` without `event disable all` or `return`.",
    section="irules",
)
IRULE5004 = diag("IRULE5004", "`DNS::return` without `return`.", section="irules")
IRULE5005 = diag(
    "IRULE5005", "Direct proc invocation without `call` — use `call proc_name`.", section="irules"
)

# --- iRules security (section="irules_security") ---
IRULE3001 = diag("IRULE3001", "Tainted data in HTTP response body.", section="irules_security")
IRULE3002 = diag(
    "IRULE3002", "Tainted data in HTTP header or cookie value.", section="irules_security"
)
IRULE3003 = diag(
    "IRULE3003", "Tainted data in `log` command — log injection risk.", section="irules_security"
)
IRULE3101 = diag(
    "IRULE3101",
    "`HTTP::uri`/`HTTP::path` set to value not provably starting with `/`.",
    section="irules_security",
)
IRULE3102 = diag(
    "IRULE3102",
    "`HTTP::path`/`HTTP::uri`/`HTTP::query` getter used without `-normalized`.",
    section="irules_security",
)

# --- iRules variable (section="irules_variable") ---
IRULE4001 = diag(
    "IRULE4001", "Write to `static::` variable outside `RULE_INIT`.", section="irules_variable"
)
IRULE4002 = diag(
    "IRULE4002",
    "Generic `static::` variable name — collision likely across iRules.",
    section="irules_variable",
)
IRULE4003 = diag("IRULE4003", "Variable scoping concern across events.", section="irules_variable")
IRULE4004 = diag(
    "IRULE4004",
    "Constant `set` in per-request event could be hoisted to an earlier once-per-connection event.",
    section="irules_variable",
)
IRULE4005 = diag(
    "IRULE4005",
    "Potential race — `static::` variable written outside `RULE_INIT` and read in another event.",
    section="irules_variable",
)

# --- Internal ---
IRULE3103 = diag(
    "IRULE3103",
    "iRules taint flow — URI split injection.",
    section="irules_security",
    internal=True,
)
IRULE5003 = diag("IRULE5003", "iRules internal control flow.", section="irules", internal=True)
IRULE6001 = diag("IRULE6001", "iRules global variable usage.", section="irules", internal=True)
