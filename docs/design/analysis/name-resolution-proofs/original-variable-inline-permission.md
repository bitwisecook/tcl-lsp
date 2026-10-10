# naming.refactor.original-variable-inline-permission

Kind: `implementation-contract`

## Problem statement

A store/read rewrite can remove a read observer, select an earlier SSA definition, cross an alias mutation or move an evaluated value. UI variables and source spelling cannot supply these permissions.

## Question

Does original inline-variable selection retain exact store/read ownership and refuse observers, reassignment, aliases, stale source and value-effect movement?

## Conclusion

The refactor requires the current complete original image/configuration, a selected literal store with retained contents, one later same-block operand use of the exact SSA definition, and the original source read extent. The shared SSA read projection independently requires value production in every retained context, a quiet bounded scalar cell with a current generation, and matching typed cell/version. Observers, aliases, reassignment, unknown residuals, stale source and effectful producing expressions decline. Source rendering and store deletion remain separately checked by the refactor.

## Scope

Rust source rewrite contract over original source, selected Registry operation and independently retained SSA/cell/read facts. The fixed tests use pure braced Unicode source with reporting variables cleared and refuse reassignment, unset/recreation, aliases, observers and evaluated values. Native shell comparisons are separate finite observations and do not execute this Rust transformation. The actual inline source consumer carries complete AnalysisResult input through CompilationUnit and the independent Module/FunctionUnit metadata join into original definition/transfer selection. Missing supplied input is terminal. The existing exact read/version, observer, source extent and value-motion prerequisites remain separate from availability and source grammar.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This precise Rust source contract has no native execution receipt.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This precise Rust source contract has no native execution receipt.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This precise Rust source contract has no native execution receipt.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This precise Rust source contract has no native execution receipt.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This precise Rust source contract has no native execution receipt.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

This precise Rust source contract has no native execution receipt.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

This precise Rust source contract has no native execution receipt.

## Exact evidence

- `implementation-0` (implementation): [rust/tcl-lsp-core/src/refactor/inline_variable.rs](../../../../rust/tcl-lsp-core/src/refactor/inline_variable.rs). SHA-256 `6a94860b32c16a74e76bc29d9dffafffd935a009d632403b33adf7b2d3acdbaa`. Current refactor/shared SSA owner source and fixed assertions; no execution receipt.
- `implementation-1` (implementation): [rust/tcl-compiler/src/ssa.rs](../../../../rust/tcl-compiler/src/ssa.rs). SHA-256 `aed9342499c50ed49dff1679a088ff8d42031413402c42b61483d520208f6389`. Current refactor/shared SSA owner source and fixed assertions; no execution receipt.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-core/src/refactor/inline_variable.rs](../../../../rust/tcl-lsp-core/src/refactor/inline_variable.rs), `value_read_binding`: Select one authentic later operand read of the exact original stored SSA version and retain its source extent.
- [rust/tcl-compiler/src/ssa.rs](../../../../rust/tcl-compiler/src/ssa.rs), `SsaSourceView::read_produces_value_at`: Recheck the exact original substitution and all retained context alternatives for value production separately from address and source geometry.
- [rust/tcl-compiler/src/ssa.rs](../../../../rust/tcl-compiler/src/ssa.rs), `SsaSourceView::replaceable_read_at`: Require independently quiet current scalar cell/version correspondence before replacing the represented read; grant no store deletion or value movement.
- [rust/tcl-lsp-core/src/refactor/inline_variable.rs](../../../../rust/tcl-lsp-core/src/refactor/inline_variable.rs), `original_inline_binding`: Retain actual whole AnalysisResult input and Module/FunctionUnit metadata before joining independently proved original definition and later read; no profile fallback for missing input.
- [rust/tcl-lsp-core/src/refactor/inline_variable.rs](../../../../rust/tcl-lsp-core/src/refactor/inline_variable.rs), `refactor::inline_variable::tests::original_inline_variable_requires_exact_definition_and_unobserved_read_without_ui_maps` (linked): Exact braced Unicode original store/read remains selectable with reporting maps cleared under each independently selected source dialect. Stale complete source and missing actual input refuse. The conditional rewrite still requires original SSA/read/observer/value-motion permission; metadata alone supplies no edit equivalence.
- [rust/tcl-lsp-core/src/refactor/inline_variable.rs](../../../../rust/tcl-lsp-core/src/refactor/inline_variable.rs), `refactor::inline_variable::tests::original_inline_variable_rejects_reassigned_cells_observers_and_value_effect_movement` (linked): Refuse reassigned/unset cells, observed reads, upvar alias mutation and moving an evaluated value.

A named test is a coverage binding, not a claim that it executed.

## Replay

The named fixed Rust selectors bind source rewrite assertions; their execution outcome is recorded separately. Native shell controls are independently authored scripts, not Rust-generated edits.
