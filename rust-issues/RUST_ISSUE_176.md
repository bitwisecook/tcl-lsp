# RUST_ISSUE_176: continuation handling runs before the comment check, so an indented `#`/`;` comment line inside a multi-line value is absorbed into the value, diverging from the configparser semantics the module claims to port

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | LSP server & document sync |
| **Location** | `rust/tcl-lsp-server/src/config_ini.rs:87-97` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lsp-server/src/config_ini.rs:87-97 — continuation handling runs before the comment check, so an indented `#`/`;` comment line inside a multi-line value is absorbed into the value, diverging from the configparser semantics the module claims to port.
In `.tcl-lsp.ini`, `extraCommands = a\n    # b\n    c` yields the value `"a\n# b\nc"`; `parse_comma_list` then emits tokens `#` and `b` — a commented-out entry becomes a live extra command (suppressing W123 for `b`). Confidence: high
