# naming.variable.selected-setter-expression-reads

Kind: `implementation-contract`

## Problem statement

A setter value can contain a selected expression alias or a same-spelled procedure. Parsing the written expression label again can introduce reads that the actual child selection does not perform.

## Question

How do setter SSA reads preserve selected child expression ownership without borrowing a written command label?

## Conclusion

uses_in_call collects original entered and conditional child expression reads through the shared command-word scan. For a queried source binding, set_value_reads retains ordinary written substitutions and returns before the unqueried direct-Registry expression adapter. Unknown or replaced child selection cannot acquire another expression evaluator from its written head. The independent actual setter definition remains separate from its value dependencies.

## Scope

One linked selector retains a genuine native entry owner across three fixed source shapes in each of five C release contexts: direct expression through an aliased setter, a same-spelled procedure returning literal VALUE, and aliased expression plus setter. Each retains Script::retained_source_tokens_for_statement from its actual source carrier, including valid AssignExpr specialization, and checks an independently selected output definition and the expected input-read presence. The existing unqueried scanner-element compatibility control remains separate. Native CLI result observations do not establish compiler admission, private frames/cells or these Rust outcomes.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

No native provider execution is attached to this source classification contract.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

No native provider execution is attached to this source classification contract.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

No native provider execution is attached to this source classification contract.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

No native provider execution is attached to this source classification contract.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

No native provider execution is attached to this source classification contract.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No native provider execution is attached to this source classification contract.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

No native provider execution is attached to this source classification contract.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/ssa.rs](../../../../rust/tcl-compiler/src/ssa.rs), `set_value_reads`: Keep queried child selection terminal before the unqueried compatibility expression parser; preserve ordinary source substitutions.
- [rust/tcl-compiler/src/ssa.rs](../../../../rust/tcl-compiler/src/ssa.rs), `uses_in_call`: Consume the genuine setter definition and shared selected child expression read inventory independently.
- [rust/tcl-compiler/src/ssa.rs](../../../../rust/tcl-compiler/src/ssa.rs), `ssa::tests::setter_source_reads_use_selected_child_expression_owners` (linked): Five C context labels retain actual native entry ownership and the genuine original Script statement carrier, accepting valid AssignExpr specialization. Direct/aliased expressions retain input reads, while a same-spelled user procedure does not invent a missing-variable expression read. Independent output definitions and expected uses remain required.

A named test is a coverage binding, not a claim that it executed.

## Replay

Named body-marked Rust selectors are authored coverage bindings. Executed outcomes require independent exact-source receipts; no native observation or Rust success is attached here.
