# naming.server.original-namespace-rename-plans

Kind: `implementation-contract`

## Problem statement

Two opaque namespace siblings can share reporting spellings across URI-owned document copies. A cross-document rename could edit the wrong sibling or accept a proposed namespace already occupied only as an implicit parent of another document's declaration.

## Question

Does Server original namespace rename produce typed per-document source plans after reporting names are cleared and reject a proposed implicit-parent collision?

## Conclusion

The Server consumer derives the proposed namespace through the selected original symbol and complete source grammar, checks explicit and descendant-created occupancy through typed workspace rows, and invokes the shared original namespace edit planner separately for every inspected document. Missing document coverage is terminal. The fixed assertion clears reporting names, applies the exact D800 plans to both URI-owned copies while preserving D801, and refuses renamed when renamed::child occupies its implicit parent. This is an implementation assertion, not an executed Rust test or native namespace mutation proof.

## Scope

Source-only workspace editor contract for two documents under the C8.6 analysis profile plus one collision document. The test exercises actual emitted edit text and independent URI ownership, with cleared original/qualified reporting names. It does not execute the edited source, grant a native namespace instance or prove an arbitrary dynamic namespace dispatch. The shared Core edit planner also requires the supplied caller profile to match the authentic retained analysis input. Equal global command and variable simple tails remain independent from namespace identity; a foreign Tcl or hosted profile cannot supply edit advice.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This typed workspace rename integration is a Rust implementation invariant; no native provider observation answers this question.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This typed workspace rename integration is a Rust implementation invariant; no native provider observation answers this question.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This typed workspace rename integration is a Rust implementation invariant; no native provider observation answers this question.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This typed workspace rename integration is a Rust implementation invariant; no native provider observation answers this question.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This typed workspace rename integration is a Rust implementation invariant; no native provider observation answers this question.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

This typed workspace rename integration is a Rust implementation invariant; no native provider observation answers this question.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

This typed workspace rename integration is a Rust implementation invariant; no native provider observation answers this question.

## Exact evidence

- `consumer-source` (implementation): [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs). SHA-256 `93afa314052ead19f04f6116e17d8bd3ed14caac5d1ab997efd599176e7c6a3d`. Current actual typed Server consumer and fixed emitted-edit/collision assertion; not an execution receipt.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `Backend::cross_document_original_namespace_rename`: Select typed proposed occupancy and require independently inspected source edit plans for every indexed URI.
- [rust/tcl-lsp-core/src/namespace_rename/original.rs](../../../../rust/tcl-lsp-core/src/namespace_rename/original.rs), `original_namespace_rename_edits`: Manufacture actual written namespace edits only under retained source/grammar and complete original correspondence.
- [rust/tcl-lsp-core/src/workspace_index.rs](../../../../rust/tcl-lsp-core/src/workspace_index.rs), `WorkspaceIndex::original_namespace_descendant_declarations`: Treat typed descendant declarations as occupied implicit namespace parents without reporting-string joins.
- [rust/tcl-lsp-core/src/namespace_symbol.rs](../../../../rust/tcl-lsp-core/src/namespace_symbol.rs), `OriginalNamespaceSymbol::renamed`: Render and validate the proposed original address using the selected source channel and grammar; the proposed address supplies no namespace existence.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `tests::original_namespace_rename_uses_typed_workspace_collisions_and_source_plans` (linked): Clears namespace reporting names, applies exactly two edits per URI to the D800 namespace and its reference, preserves the D801 sibling, and rejects another document's implicit renamed parent.
- [rust/tcl-lsp-core/src/namespace_rename/original.rs](../../../../rust/tcl-lsp-core/src/namespace_rename/original.rs), `namespace_rename::original::namespace_tail_purpose_tests::original_namespace_rename_keeps_equal_global_command_and_variable_tails` (linked): The actual C8.6 source keeps namespace ::old distinct from global command/variable ::old tails: only the namespace declaration becomes ::new. Foreign tcl9.1 and f5-irules caller profiles refuse against the retained original analysis; no new native or workspace execution observation is supplied.

A named test is a coverage binding, not a claim that it executed.

## Replay

The named fixed Rust selector is a coverage binding. No Rust result or native interpreter capture is attached to this source edit invariant.
