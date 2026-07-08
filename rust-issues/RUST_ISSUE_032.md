# RUST_ISSUE_032: `normalize_config_payload` panics on a client-supplied config payload whose flat-dotted key collides with a non-object value, killing the server

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | LSP server & document sync |
| **Location** | `rust/tcl-lsp-server/src/lib.rs:7851` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lsp-server/src/lib.rs:7851 — `normalize_config_payload` panics on a client-supplied config payload whose flat-dotted key collides with a non-object value, killing the server.
A `workspace/configuration` reply like `{"tclLsp.optimiser": true, "tclLsp.optimiser.enabled": false}` (or nested `{"tclLsp":{"style":"x"}}` plus flat `"tclLsp.style.nonAscii"`) makes the dotted-key fold hit an existing scalar: `cursor.entry(...).or_insert_with(...)` keeps the scalar and `.as_object_mut().expect("nested config segment is an object")` panics inside the `initialized`/`didChangeConfiguration` handler — an uncontained unwind through tower-lsp's `serve` join, i.e. process death from client data the function is explicitly designed to accept in mixed shapes. Confidence: high
