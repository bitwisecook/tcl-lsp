# RUST_ISSUE_040: a namespace-qualified self-call is linked to the declaration despite covering different text, so the linked-edit set is not all-identical-content

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | LSP display features |
| **Location** | `rust/tcl-lsp-core/src/linked_editing_range.rs:95` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lsp-core/src/linked_editing_range.rs:95 — a namespace-qualified self-call is linked to the declaration despite covering different text, so the linked-edit set is not all-identical-content.
`proc greet {} { ::greet }`, cursor on declaration name: `matches_self_call` true via `proc.qualified_name == "::greet"`, pushed range is full `::greet` (7 cols) while range[0] is the `greet` name span (5 cols). Rename-as-you-type mirrors one range's text into the other, dropping the `::` and corrupting the call. Confidence: high
