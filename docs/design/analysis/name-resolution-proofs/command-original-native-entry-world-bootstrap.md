# naming.command.original-native-entry-world-bootstrap

Kind: `implementation-contract`

## Problem statement

A native source entry can reach a branch before any command-table operation. Initialising its immutable naming policy on the first operation makes the changed branch disagree with the untouched incoming baseline and withdraws the shared source ledger. Filling missing native rows from Registry metadata would hide that provenance error.

## Question

Does a closed retained native entry establish its exact namespace geometry and selected naming policy once before branch transfer, without filling native command absence or reviving a withdrawn ledger?

## Conclusion

After installing the genuine native entry rows, the source ledger initialises only from the same closed entry and matching independently selected command naming policy. Every namespace context retains its original interpreter/token and exact C components or Jim namespace-object bytes. Branches share that selected baseline. Missing native rows remain missing; incomplete, unknown-history, foreign or already-withdrawn entries cannot reseed the ledger.

## Scope

Current Rust source-analysis contract at the NativeCompilationEntry installation boundary. The full retained entry, independently closed command table, original namespace contexts and matching policy are required. This supplies immutable namespace geometry and selected policy only; it grants no missing command implementation, original source publication, observer absence, successful root completion, compiler hook, variable cell or actual guest execution. The named selector is a validation obligation without an attached passing execution receipt.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No guest execution of this Rust implementation question is claimed.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No guest execution of this Rust implementation question is claimed.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No guest execution of this Rust implementation question is claimed.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No guest execution of this Rust implementation question is claimed.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No guest execution of this Rust implementation question is claimed.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

No guest execution of this Rust implementation question is claimed.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No guest execution of this Rust implementation question is claimed.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/command_binding/source_command_world.rs](../../../../rust/tcl-compiler/src/command_binding/source_command_world.rs), `OriginalSourceCommandWorld::for_runtime_entry`: Once-only policy and exact retained namespace geometry bootstrap from the same closed native entry.
- [rust/tcl-compiler/src/command_binding/runtime_entry.rs](../../../../rust/tcl-compiler/src/command_binding/runtime_entry.rs), `ModuleCommandBindings::install_runtime_entry`: Install actual native rows before initialising their immutable source geometry; no Registry missing-row fallback.
- [rust/tcl-compiler/src/command_binding/runtime_entry.rs](../../../../rust/tcl-compiler/src/command_binding/runtime_entry.rs), `command_binding::runtime_entry::tests::original_native_world_selects_policy_before_any_branch_and_never_reseeds_withdrawal` (linked): Unchanged and selected branches retain identical policy/geometry; missing native rows stay missing, and withdrawn/incomplete entries cannot reseed.

A named test is a coverage binding, not a claim that it executed.

## Replay

No passing Rust or guest execution result is attached to this implementation contract. Existing branch and namespace-path guard tests remain independent obligations.
