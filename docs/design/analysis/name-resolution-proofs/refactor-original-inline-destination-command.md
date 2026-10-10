# naming.refactor.original-inline-destination-command

Kind: `implementation-contract`

## Problem statement

A body command can be a stock builtin in its declaring namespace while the same proposed spelling resolves to a user procedure in the caller namespace. Moving the body without rechecking its destination changes the implementation. The body occurrence and caller proposal are different purposes and cannot share an invented original head.

## Question

Does inlining independently validate each proposed body builtin at the actual call destination?

## Conclusion

The bounded inline subset admits only an exact original body head matching the selected authored ASCII Registry identifier, then independently checks that implementation at the actual call point through the shared proposed Registry command facade. Shadowed, replaced, observed, qualified or opaque unsupported proposals decline. The facade supplies conditional command selection only, without insertion, frame relocation, store or Native preparation authority.

## Scope

Native single-command procedure inline destination check. General byte command proposals, alias body heads and independent expression-function relocation obligations are outside this facade.

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

- `implementation-0` (implementation): [rust/tcl-compiler/src/command_binding/declaration_lookup.rs](../../../../rust/tcl-compiler/src/command_binding/declaration_lookup.rs). SHA-256 `480e4982a5d651f92dcd67be9c9b74c888ed69cbd374ee33f856c5057d934296`. Current shared owner, consumer and fixed validation assertions. No executed Rust or native result is implied.
- `implementation-1` (implementation): [rust/tcl-lsp-core/src/refactor/inline_proc.rs](../../../../rust/tcl-lsp-core/src/refactor/inline_proc.rs). SHA-256 `e0ee37cbf0aa0f515f434021eb30aa31adf46a2396f05a62749d0884a949bbac`. Current shared owner, consumer and fixed validation assertions. No executed Rust or native result is implied.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/command_binding/declaration_lookup.rs](../../../../rust/tcl-compiler/src/command_binding/declaration_lookup.rs), `original_proposed_registry_commands`: Validate proposed authored Registry spellings against original point snapshots and observer/operand closure.
- [rust/tcl-lsp-core/src/refactor/inline_proc.rs](../../../../rust/tcl-lsp-core/src/refactor/inline_proc.rs), `original_inline_body`: Keep original body naming and caller proposal selection separate.
- [rust/tcl-lsp-core/src/refactor/inline_proc.rs](../../../../rust/tcl-lsp-core/src/refactor/inline_proc.rs), `refactor::inline_proc::original_inline_destination_tests::original_inline_refuses_a_body_builtin_shadowed_at_the_call_destination` (linked): Caller namespace shadowing refuses the rewrite with no edits.

A named test is a coverage binding, not a claim that it executed.

## Replay

Named fixed Rust selectors bind validation obligations. No Rust execution receipt is attached to this implementation contract. No native interpreter experiment or overall passing suite is claimed. Missing authority and unsupported source purposes remain explicit in the scope and conclusion.
