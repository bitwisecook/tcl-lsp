# naming.core.original-declaration-graphs

Kind: `implementation-contract`

## Problem statement

A graph keyed by reporting names collapses opaque names and repeated procedure declarations. A String-keyed interprocedural summary can also appear to describe a different source declaration. The graph must retain separate original declaration identities, match call edges by the retained implementation allocation, and attach summaries only after independent body and namespace correspondence.

## Question

Can graph consumers preserve original procedure declarations and canonical call edges without donating String IR summaries to opaque or replaced declarations?

## Conclusion

Original graph nodes use current OriginalDeclarationIdentity records and per-inventory IDs. Canonical call edges require the original Input, its own lookup and retained allocation through the shared predicate. Purity and effects require an independently retained matching implementation body, namespace, full source and configuration; otherwise they remain null. CLI text and MCP JSON consume the original source projection, while compiled IR is separate.

## Scope

Readonly procedure declaration graphs, canonical retained calls, source body grouping and optional independently joined compiled summaries. No command installation, entered body, native compilation or edit authority. Scope and variable cards remain reporting projections, and profiles without original naming inputs retain explicitly bounded lexical/compiled views.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This is a current Rust implementation contract. No native provider observation or test execution is claimed for this precise consumer question.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This is a current Rust implementation contract. No native provider observation or test execution is claimed for this precise consumer question.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This is a current Rust implementation contract. No native provider observation or test execution is claimed for this precise consumer question.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This is a current Rust implementation contract. No native provider observation or test execution is claimed for this precise consumer question.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This is a current Rust implementation contract. No native provider observation or test execution is claimed for this precise consumer question.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

This is a current Rust implementation contract. No native provider observation or test execution is claimed for this precise consumer question.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

This is a current Rust implementation contract. No native provider observation or test execution is claimed for this precise consumer question.

## Exact evidence

- `implementation-1` (implementation): [rust/tcl-lsp-core/src/graphs/original.rs](../../../../rust/tcl-lsp-core/src/graphs/original.rs). SHA-256 `2cee1d14eff671791fc6362b1f86a33e197c34e962864d5010b4e3e39253e109`. Current source graph consumer and fixed Rust assertions; no execution receipt.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-core/src/original_declaration.rs](../../../../rust/tcl-lsp-core/src/original_declaration.rs), `OriginalDeclarationIdentity`: Current readonly original inventory membership and full source/configuration currency.
- [rust/tcl-lsp-core/src/original_declaration.rs](../../../../rust/tcl-lsp-core/src/original_declaration.rs), `invocation_targets_declaration`: Canonical retained Input/lookup/reference/allocation matching.
- [rust/tcl-lsp-core/src/graphs/original.rs](../../../../rust/tcl-lsp-core/src/graphs/original.rs), `compiled_label`: Independent original implementation body and namespace mapping, with no QName identity reconstruction.
- [rust/tcl-lsp-core/src/graphs/original.rs](../../../../rust/tcl-lsp-core/src/graphs/original.rs), `graphs::original::tests::original_graph_nodes_preserve_opaque_redefinitions_and_canonical_edges_without_ui_maps` (linked): Preserve three distinct declaration IDs, including two repeated opaque names, actual edges, unknown summaries and stale/missing-lookup refusal.
- [rust/tcl-lsp-core/src/graphs/original.rs](../../../../rust/tcl-lsp-core/src/graphs/original.rs), `graphs::original::tests::original_graph_alias_edges_select_terminal_allocation_without_reporting_names` (linked): Select the terminal target declaration after all reporting names are counterfactual.
- [rust/tcl-lsp-core/src/graphs/original.rs](../../../../rust/tcl-lsp-core/src/graphs/original.rs), `graphs::original::tests::original_graph_summary_requires_body_allocation_even_when_ir_labels_agree` (linked): Equal IR labels and source text cannot donate summary facts when independent body allocation inventory is removed.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "cargo",
  "test",
  "-p",
  "tcl-lsp-core",
  "graphs::original::tests::"
]
```

The fixed Rust selectors bind source graph assertions. Execution receipts are separate; all seven native providers remain not-tested for this precise graph question.
