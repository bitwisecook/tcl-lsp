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

## Original source worker results

The private source callback loops seal the actual full request and Module,
prepared caller/parameter/outer-write/binding/summary context, complete input and
lexer configuration, optional profile and dialect spelling, seeds, class/trace
facts and value-transfer context. OriginalFunctionLatticeInputs is a data view,
not a constructor of authority. BodyLatticeInputs checks it against that retained
recipe before invoking the existing single CFG/FU/semantic worker pipeline.
Its private result snapshot separately checks actual semantic-value owner
availability alongside every original function field. Public field mutation or
a merely equal CFG cannot retain the original callback admission.

The actual native-template and neutral body correspondence owners retain their
original projections and exact proof restoration. Rebinding preserves ordinary
reporting results; unsupported worker receipts remain unavailable. The cache
purpose separates legacy and original-worker inputs without adding full source
or allocation salts to same-purpose canonical body/certificate equality.
The private source-CU producer independently requires every accepted callback to
have its genuine result receipt. Body-lowering provenance, completed whole raw
optimiser inventory and final current-report/action checks remain separate.
No execution, Native cell, entered body, Normal, equivalence or edit permission
is issued by this lattice receipt alone.

- [rust/tcl-compiler/src/compilation_unit/original_lattice_worker.rs](../../../../rust/tcl-compiler/src/compilation_unit/original_lattice_worker.rs), `BodyLatticeInputs::original_function_lattice`: Rejoin every actual worker input before the existing single analysis pipeline.
- [rust/tcl-compiler/src/compilation_unit/original_lattice_worker.rs](../../../../rust/tcl-compiler/src/compilation_unit/original_lattice_worker.rs), `OriginalFunctionLatticeTemplate::rebind_cached`: Restore the actual original native template and retain current request correspondence only for an unchanged genuine worker result.
- [rust/tcl-compiler/src/compilation_unit/original_lattice_worker.rs](../../../../rust/tcl-compiler/src/compilation_unit/original_lattice_worker.rs), `compilation_unit::original_lattice_worker::tests::original_lattice_worker_keeps_actual_recipe_and_refuses_changed_key_or_request` (linked): Genuine callback/worker transport retains exact key fields and separates a legacy-first cache purpose; changed request, profile or literal seeds refuse. Source control only, unexecuted.
- [rust/tcl-compiler/src/compilation_unit/original_lattice_worker.rs](../../../../rust/tcl-compiler/src/compilation_unit/original_lattice_worker.rs), `compilation_unit::original_lattice_worker::tests::original_lattice_worker_refuses_public_result_changes_without_changing_report_acceptance` (linked): A CFG-equal public analysis mutation remains reportable but cannot retain original worker admission. Source control only, unexecuted.
