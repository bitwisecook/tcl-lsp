# naming.editor.original-variable-rename-frame-currency

Kind: `implementation-contract`

## Problem statement

Two procedures can declare local with the same spelling in independent frames. Equal spelling, stale analysis or an already occupied replacement could produce an edit that merges unrelated source symbols.

## Question

Does a local variable edit retain the selected authentic source frame and refuse changed source or a typed replacement collision?

## Conclusion

The edit contract requires complete current source/grammar and the selected typed frame symbol before planning geometry. The regression applies edits to p only while q retains local, then requires changed-source and occupied changed-name cases to refuse. The frame correspondence is source ownership, not an entered runtime frame or current cell.

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

- `plan` (implementation): [rust/tcl-lsp-core/src/variable_symbol/rename.rs](../../../../rust/tcl-lsp-core/src/variable_symbol/rename.rs). SHA-256 `6d33395aaf6db04061ef7918172271e256494e47526d84005eafde8dbedd605c`. Actual source plan and dedicated fixed-input regression; source digest is reissued only after owner formatting is stable.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-core/src/variable_symbol/rename.rs](../../../../rust/tcl-lsp-core/src/variable_symbol/rename.rs), `original_variable_rename_edits`: Requires complete source/grammar, typed root/frame/coverage/collision facts and independently editable original geometry before issuing all source replacements.
- [rust/tcl-compiler/src/signature_scan/variable_symbol.rs](../../../../rust/tcl-compiler/src/signature_scan/variable_symbol.rs), `renamed_input`: Retains each original receiver purpose and root/index geometry when deriving proposed native input bytes.
- [rust/tcl-compiler/src/analyser/types.rs](../../../../rust/tcl-compiler/src/analyser/types.rs), `original_variable_rename_is_complete`: Withdraws editing when original target/alias/read coverage for the selected source symbol is incomplete.
- [rust/tcl-lsp-core/src/original_name_edit.rs](../../../../rust/tcl-lsp-core/src/original_name_edit.rs), `original_variable_root_name_edit`: Checks original written lexical delimiters and root extent before issuing one immutable source edit.
- [rust/tcl-lsp-core/src/original_name_edit.rs](../../../../rust/tcl-lsp-core/src/original_name_edit.rs), `original_name_input_edits`: Retains genuine original list-container geometry and refuses overlapping or unrelated-child edits.
- [rust/tcl-lsp-core/src/original_name_edit.rs](../../../../rust/tcl-lsp-core/src/original_name_edit.rs), `original_input_matches_source`: Checks original input/source-span correspondence under the consumer complete source and grammar; shared by namespace and variable editors.
- [rust/tcl-lsp-core/src/variable_symbol/rename.rs](../../../../rust/tcl-lsp-core/src/variable_symbol/rename.rs), `variable_symbol::rename::tests::original_variable_rename_uses_authentic_local_frame_and_refuses_stale_or_colliding_sources` (linked): The edit contract requires complete current source/grammar and the selected typed frame symbol before planning geometry. The regression applies edits to p only while q retains local, then requires changed-source and occupied changed-name cases to refuse. The frame correspondence is source ownership, not an entered runtime frame or current cell.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "cargo",
  "test",
  "-p",
  "tcl-lsp-core",
  "variable_symbol::rename::tests::original_variable_rename_uses_authentic_local_frame_and_refuses_stale_or_colliding_sources"
]
```

The exact current source selector identifies the implementation assertion; naming it or matching its comment is not an execution claim. All native providers remain not-tested for this Rust edit question.
