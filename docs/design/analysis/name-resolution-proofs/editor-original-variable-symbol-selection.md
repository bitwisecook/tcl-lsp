# naming.editor.original-variable-symbol-selection

Kind: `implementation-contract`

## Problem statement

Opaque sibling variable names can share a reporting spelling. A readonly editor provider could select the wrong declaration/reference or accept coordinates from a source that changed after analysis.

## Question

Does the shared variable editor selector retain opaque slot identity and complete original-source currency after reporting qualified names are removed?

## Conclusion

The shared selector checks the complete source image and retained grammar before issuing an original occurrence. Exact typed symbol equality selects declaration/reference spans and readonly hover counts; stale source and retained-but-unavailable operands are terminal. The fixed source assertion distinguishes surrogate D800/D801 siblings after clearing reporting names and rejects a same-width whole-source change. This is an implementation binding, not an executed test or a native cell/value/type proof.

## Scope

Core readonly source navigation/occurrence-count-summary contract for one C8.6 document containing escaped surrogate sibling names. The test asserts one declaration, one reference, optional declaration inclusion and source-only hover counts. No variable value, normal completion, runtime alias, frame lifetime or editable rename plan is supplied.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This readonly editor integration is a Rust implementation invariant; no native provider observation answers this question.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This readonly editor integration is a Rust implementation invariant; no native provider observation answers this question.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This readonly editor integration is a Rust implementation invariant; no native provider observation answers this question.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This readonly editor integration is a Rust implementation invariant; no native provider observation answers this question.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This readonly editor integration is a Rust implementation invariant; no native provider observation answers this question.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

This readonly editor integration is a Rust implementation invariant; no native provider observation answers this question.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

This readonly editor integration is a Rust implementation invariant; no native provider observation answers this question.

## Exact evidence

- `actual-consumer-source` (implementation): [rust/tcl-lsp-core/src/variable_symbol.rs](../../../../rust/tcl-lsp-core/src/variable_symbol.rs). SHA-256 `a1cbe154d4ce877607ee8cc0d29d0f0722936fa07711d84776366aba71ee18d8`. Exact current consumer and fixed test source; not an execution receipt or native observation.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-core/src/variable_symbol.rs](../../../../rust/tcl-lsp-core/src/variable_symbol.rs), `select`: Selects only authentic current-source occurrences and makes retained unknowns terminal.
- [rust/tcl-lsp-core/src/variable_symbol.rs](../../../../rust/tcl-lsp-core/src/variable_symbol.rs), `occurrences`: Compares actual typed variable symbols independently of optional reporting keys.
- [rust/tcl-lsp-core/src/variable_symbol.rs](../../../../rust/tcl-lsp-core/src/variable_symbol.rs), `declaration_spans`: Projects exact declaration-name source extents.
- [rust/tcl-lsp-core/src/variable_symbol.rs](../../../../rust/tcl-lsp-core/src/variable_symbol.rs), `reference_spans`: Projects source occurrence extents with explicit declaration inclusion.
- [rust/tcl-lsp-core/src/variable_symbol.rs](../../../../rust/tcl-lsp-core/src/variable_symbol.rs), `hover_text`: Renders the original name through retained policy/grammar and readonly source counts, without value/type inference.
- [rust/tcl-lsp-core/src/variable_symbol.rs](../../../../rust/tcl-lsp-core/src/variable_symbol.rs), `variable_symbol::tests::original_variable_editor_selection_uses_bytes_and_complete_source_currency` (linked): Clears qualified-name reports, distinguishes two opaque siblings, checks declaration/reference/hover counts, and refuses stale whole source.

A named test is a coverage binding, not a claim that it executed.

## Replay

The named fixed Rust test is a source coverage binding. No Rust test result or native provider capture is attached to this contract; C Tcl, Jim and BIG-IP have not tested this Rust invariant.
