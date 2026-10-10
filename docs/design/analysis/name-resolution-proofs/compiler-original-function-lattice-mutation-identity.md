# naming.compiler.original-function-lattice-mutation-identity

Kind: `implementation-contract`

## Problem statement

A function lattice built under untouched command bindings cannot be replayed under a namespace shadow or unknown mutation obligation. Excluding every such request also prevents identical original event source from sharing the memo.

## Question

Does each memo request retain the complete module mutation snapshot alongside the sealed original event, source, body, grammar, Registry and CFG context?

## Conclusion

The request transports the existing canonical CommandTrustSnapshot into the interned function entry and reconstructs the exact mutation obligations for the lattice fold owner. A changed mutation axis rekeys an otherwise identical event and body. Memo admission retains the actual mutation snapshot; independent original event correspondence, body/config/Registry identity, seed and module context requirements, and exact current CFG equality after rebasing remain required.

## Scope

Original source function lattice memo identity for ordinary procedures and Registry-selected conditional iRules event bodies. CommandTrustSnapshot retains its full sorted named, rebound, dynamic, unnameable-subject, runtime-frame, resolution and opaque-namespace axes. Event descriptors retain their complete original source, declaration/body, configuration and Registry identity. This cache transport does not establish a worker, executing event, backend frame, current native binding, body completion or Normal result, and does not erase any proof-restoration or current-CFG refusal.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No native execution of this Rust implementation question is claimed.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No native execution of this Rust implementation question is claimed.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No native execution of this Rust implementation question is claimed.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No native execution of this Rust implementation question is claimed.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

No native execution of this Rust implementation question is claimed.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

No native execution of this Rust implementation question is claimed.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No native execution of this Rust implementation question is claimed.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/command_binding.rs](../../../../rust/tcl-compiler/src/command_binding.rs), `ModuleCommandMutations::snapshot`: Existing complete canonical sorted mutation snapshot and exact reconstruction, without a new trust classification.
- [rust/tcl-compiler/src/compilation_unit.rs](../../../../rust/tcl-compiler/src/compilation_unit.rs), `build_procedure_units`: Pass exact mutation obligations in each original memo request and retain independent current CFG equality.
- [rust/tcl-compiler/src/compilation_unit.rs](../../../../rust/tcl-compiler/src/compilation_unit.rs), `FunctionUnit::build_for_lattice`: Consume actual keyed mutation obligations for the fold owner rather than substituting an untouched module.
- [rust/tcl-lsp-db/src/lib.rs](../../../../rust/tcl-lsp-db/src/lib.rs), `FnLatticeEntry`: Structurally retain full original event and module mutation identity in the interned function key.
- [rust/tcl-lsp-db/src/lib.rs](../../../../rust/tcl-lsp-db/src/lib.rs), `function_lattice`: Reconstruct the exact canonical mutation snapshot while retaining all original source/configuration/Registry/context axes.
- [rust/tcl-lsp-db/src/lib.rs](../../../../rust/tcl-lsp-db/src/lib.rs), `tests::original_event_lattice_cache_retains_conditional_entry_and_reuses_same_source` (linked): Two identical source files share the def-use memo while retaining conditional local/TMM event entry and no executing worker.
- [rust/tcl-lsp-db/src/lib.rs](../../../../rust/tcl-lsp-db/src/lib.rs), `tests::original_event_lattice_key_rejects_foreign_source_body_config_and_registry` (linked): Full source/body/configuration/Registry/event correspondence remains required and a changed complete mutation snapshot mints a different key.

A named test is a coverage binding, not a claim that it executed.

## Replay

The exact selectors bind the current implementation contract; no execution receipt is attached. Independent CFG equality and complete producer identity remain acceptance obligations. All seven native providers are not-tested for this Rust memo question.
