# naming.server.original-variable-rename-plans

Kind: `implementation-contract`

## Problem statement

A namespace variable rename can cross equal-source documents and encounter a replacement declared in another URI. A local plan or display-based collision check can miss a document owner or merge original byte slots.

## Question

Does Server variable rename require independently inspected source plans for every URI and reject a typed replacement occupied elsewhere in the workspace?

## Conclusion

The Server contract constructs the proposed original symbol, checks typed workspace occupancy, independently reads/analyzes each participating URI and delegates its exact source plan to the shared Core editor. The regression applies both emitted document plans for D800 while preserving D801, then requires a separately indexed changed declaration to refuse. Missing document/coverage or invalid URI is terminal; no native execution or workspace runtime cell is granted.

## Scope

Current Rust source edit correspondence, original byte/policy symbol, selected receiver purpose, independently issued written root/list-container geometry, complete source/grammar currency and typed coverage. Native Tcl/Jim/BIG-IP language observations motivate separate questions and do not execute this invariant. Named tests are linked coverage obligations; no execution result is asserted. Edit authority is distinct from readonly selection, source occurrence counts and any physical runtime frame/cell/value capability.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested for this Rust edit contract. Build: not tested. Channel: not tested. Dialect: C Tcl.

No native provider run establishes this Rust source-edit invariant. Native name/cell/alias observations and actual Rust test execution are separate evidence.

### tcl8.5

Status: `not-tested`. Version: not tested for this Rust edit contract. Build: not tested. Channel: not tested. Dialect: C Tcl.

No native provider run establishes this Rust source-edit invariant. Native name/cell/alias observations and actual Rust test execution are separate evidence.

### tcl8.6

Status: `not-tested`. Version: not tested for this Rust edit contract. Build: not tested. Channel: not tested. Dialect: C Tcl.

No native provider run establishes this Rust source-edit invariant. Native name/cell/alias observations and actual Rust test execution are separate evidence.

### tcl9.0

Status: `not-tested`. Version: not tested for this Rust edit contract. Build: not tested. Channel: not tested. Dialect: C Tcl.

No native provider run establishes this Rust source-edit invariant. Native name/cell/alias observations and actual Rust test execution are separate evidence.

### tcl9.1

Status: `not-tested`. Version: not tested for this Rust edit contract. Build: not tested. Channel: not tested. Dialect: C Tcl.

No native provider run establishes this Rust source-edit invariant. Native name/cell/alias observations and actual Rust test execution are separate evidence.

### jim

Status: `not-tested`. Version: not tested for this Rust edit contract. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

No native provider run establishes this Rust source-edit invariant. Native name/cell/alias observations and actual Rust test execution are separate evidence.

### bigip

Status: `not-tested`. Version: not tested for this Rust edit contract. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No native provider run establishes this Rust source-edit invariant. Native name/cell/alias observations and actual Rust test execution are separate evidence.

## Exact evidence

- `plan` (implementation): [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs). SHA-256 `8837a17c9c7c7636b882f5c2f8414a42a4faad407d707f588c7402b8e122ff44`. Actual source plan and dedicated fixed-input regression; source digest is reissued only after owner formatting is stable.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-core/src/variable_symbol/rename.rs](../../../../rust/tcl-lsp-core/src/variable_symbol/rename.rs), `original_variable_rename_edits`: Requires complete source/grammar, typed root/frame/coverage/collision facts and independently editable original geometry before issuing all source replacements.
- [rust/tcl-compiler/src/signature_scan/variable_symbol.rs](../../../../rust/tcl-compiler/src/signature_scan/variable_symbol.rs), `renamed_input`: Retains each original receiver purpose and root/index geometry when deriving proposed native input bytes.
- [rust/tcl-compiler/src/analyser/types.rs](../../../../rust/tcl-compiler/src/analyser/types.rs), `original_variable_rename_is_complete`: Withdraws editing when original target/alias/read coverage for the selected source symbol is incomplete.
- [rust/tcl-lsp-core/src/original_name_edit.rs](../../../../rust/tcl-lsp-core/src/original_name_edit.rs), `original_variable_root_name_edit`: Checks original written lexical delimiters and root extent before issuing one immutable source edit.
- [rust/tcl-lsp-core/src/original_name_edit.rs](../../../../rust/tcl-lsp-core/src/original_name_edit.rs), `original_name_input_edits`: Retains genuine original list-container geometry and refuses overlapping or unrelated-child edits.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `cross_document_original_variable_rename`: Keeps independent URI owners, typed workspace collisions and all document coverage before returning the combined edit set.
- [rust/tcl-lsp-core/src/workspace_index.rs](../../../../rust/tcl-lsp-core/src/workspace_index.rs), `original_variable_occurrences`: Reports typed source occupancy without reconstructing optional presentation strings.
- [rust/tcl-lsp-core/src/original_name_edit.rs](../../../../rust/tcl-lsp-core/src/original_name_edit.rs), `original_input_matches_source`: Checks original input/source-span correspondence under the consumer complete source and grammar; shared by namespace and variable editors.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `tests::original_variable_rename_shares_source_plans_and_typed_workspace_collisions` (linked): The Server contract constructs the proposed original symbol, checks typed workspace occupancy, independently reads/analyzes each participating URI and delegates its exact source plan to the shared Core editor. The regression applies both emitted document plans for D800 while preserving D801, then requires a separately indexed changed declaration to refuse. Missing document/coverage or invalid URI is terminal; no native execution or workspace runtime cell is granted.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "cargo",
  "test",
  "-p",
  "tcl-lsp-server",
  "tests::original_variable_rename_shares_source_plans_and_typed_workspace_collisions"
]
```

The exact current source selector identifies the implementation assertion; naming it or matching its comment is not an execution claim. All native providers remain not-tested for this Rust edit question.
