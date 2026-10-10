# naming.refactor.original-variable-extraction-permission

Kind: `implementation-contract`

## Problem statement

An inserted scalar assignment needs a fresh destination, the actual selected setter, unobserved store and a valid evaluation/placement change. An available name and scalar spelling prove only part of this: they cannot authorise a new store, move effects or introduce future captures. The actual whole-source analysis and frame must remain independent of reporting labels.

## Question

Does variable extraction stop at the independent Native store, motion and insertion boundary after exact naming proposals?

## Conclusion

The five-argument API consumes the actual analysis, complete source/configuration and resolved Registry. Native candidates require an exact static original value word at a genuine SSA point, the independent fresh assignment proposal and scalar source spelling, with complete typed function cells guarding future names. The proposal supplies no store, Normal, motion or edit grant, so Native extraction remains disabled with no edits until those contracts exist. Core and MCP preserve the disabled reason.

## Scope

Current Native variable extraction interface with stale source, reporting-map removal, prospective collision, shadowed setter and effectful selections. No enabled Native extraction is claimed; lower name/setter proposals remain independent query-only facts.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This Rust consumer contract has no native execution receipt.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This Rust consumer contract has no native execution receipt.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This Rust consumer contract has no native execution receipt.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This Rust consumer contract has no native execution receipt.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This Rust consumer contract has no native execution receipt.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

This Rust consumer contract has no native execution receipt.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

This Rust consumer contract has no native execution receipt.

## Exact evidence

- `implementation-0` (implementation): [rust/tcl-lsp-core/src/refactor/extract_variable.rs](../../../../rust/tcl-lsp-core/src/refactor/extract_variable.rs). SHA-256 `e20a036e18d1e32eba6c203704facbe9224c40c349cfbfe8f256d8a55eec29b2`. Current shared owner, consumer and fixed validation assertions. No executed Rust or native result is implied.
- `implementation-1` (implementation): [rust/tcl-mcp/src/tools.rs](../../../../rust/tcl-mcp/src/tools.rs). SHA-256 `cd34f62f9ff99cab1662769d392b2560dd43e3ff8cbf37909c00cbb7c7663ae8`. Current shared owner, consumer and fixed validation assertions. No executed Rust or native result is implied.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-core/src/refactor/extract_variable.rs](../../../../rust/tcl-lsp-core/src/refactor/extract_variable.rs), `original_assignment_candidate`: Select genuine original word/SSA point and independently retained name/setter proposal.
- [rust/tcl-lsp-core/src/refactor/extract_variable.rs](../../../../rust/tcl-lsp-core/src/refactor/extract_variable.rs), `extract_variable`: Retain full actual analysis and stop at missing insertion/store/movement authority.
- [rust/tcl-mcp/src/tools.rs](../../../../rust/tcl-mcp/src/tools.rs), `extract_variable`: Transport the actual analysis and disabled reason to the tool response.
- [rust/tcl-lsp-core/src/refactor/extract_variable.rs](../../../../rust/tcl-lsp-core/src/refactor/extract_variable.rs), `refactor::extract_variable::tests::original_variable_extraction_never_promotes_naming_proposals_to_motion_permission` (linked): Missing Native permission cannot be replaced by reporting maps, a changed dialect, future use, shadowed setter or effectful selection.

A named test is a coverage binding, not a claim that it executed.

## Replay

Named fixed Rust selectors bind validation obligations. No Rust execution receipt is attached to this implementation contract. No native interpreter experiment or overall passing suite is claimed. Missing authority and unsupported source purposes remain explicit in the scope and conclusion.
