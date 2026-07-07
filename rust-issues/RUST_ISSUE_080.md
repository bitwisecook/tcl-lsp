# RUST_ISSUE_080: registry-side taint queries use exact `spec.subcommand(...)` lookup, so legal unique-prefix abbreviations dodge security classification (false negatives)

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Command registry |
| **Location** | `rust/tcl-registry/src/taint.rs:157` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-registry/src/taint.rs:157 — registry-side taint queries use exact `spec.subcommand(...)` lookup, so legal unique-prefix abbreviations dodge security classification (false negatives).
`chan g stdin` / `encoding convertf utf-8 $x` dispatch to `chan gets`/`encoding convertfrom` in real Tcl but `is_taint_source` misses the subcommand-level `TAINT_SOURCE`; same exact-match in `is_sanitiser` (:246), `taint_transform` (:337), `taint_double_encode_colour` (:357), `classify_taint_sinks` (:313 — `HTTP::cookie ins` skips the IRULE3002 sink). Live via tcl-compiler/src/taint.rs:329/397; the F5 minifier deliberately emits prefix-abbreviated subcommands. Confidence: high
