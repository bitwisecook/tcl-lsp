# naming.editor.original-variable-rename-alias-coverage

Kind: `implementation-contract`

## Problem statement

A global alias derives its local tail from the target, while an explicit upvar local alias has an independently written name. Renaming all equal displayed reads or editing only qualified targets would respectively damage the independent alias or omit derived reads.

## Question

Does the original edit plan account for both derived global reads and independently named upvar aliases when renaming one selected namespace variable?

## Conclusion

The edit contract requires complete original target/read coverage and preserves the actual alias naming purpose. The regression applies edits to the namespace declaration, global target and derived $v read, plus the upvar target, while alias and $alias remain exact. Unknown/computed targets without independently editable geometry decline; native alias execution and cell identity are separate questions.

## Scope

Current Rust source edit correspondence, original byte/policy symbol, selected receiver purpose, independently issued written root/list-container geometry, complete source/grammar currency and typed coverage. Native Tcl/Jim/BIG-IP language observations motivate separate questions and do not execute this invariant. Named tests are linked coverage obligations; no execution result is asserted. Edit authority is distinct from readonly selection, source occurrence counts and any physical runtime frame/cell/value capability.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`. Version: tcl8.4 not tested for this Rust edit contract; tcl8.5 not tested for this Rust edit contract; tcl8.6 not tested for this Rust edit contract; tcl9.0 not tested for this Rust edit contract; tcl9.1 not tested for this Rust edit contract; jim not tested for this Rust edit contract; bigip not tested for this Rust edit contract.

No native provider run establishes this Rust source-edit invariant. Native name/cell/alias observations and actual Rust test execution are separate evidence.

## Consumer bindings

- [rust/tcl-lsp-core/src/variable_symbol/rename.rs](../../../../rust/tcl-lsp-core/src/variable_symbol/rename.rs), `original_variable_rename_edits`: Requires complete source/grammar, typed root/frame/coverage/collision facts and independently editable original geometry before issuing all source replacements.
- [rust/tcl-compiler/src/signature_scan/variable_symbol.rs](../../../../rust/tcl-compiler/src/signature_scan/variable_symbol.rs), `renamed_input`: Retains each original receiver purpose and root/index geometry when deriving proposed native input bytes.
- [rust/tcl-compiler/src/analyser/types.rs](../../../../rust/tcl-compiler/src/analyser/types.rs), `original_variable_rename_is_complete`: Withdraws editing when original target/alias/read coverage for the selected source symbol is incomplete.
- [rust/tcl-lsp-core/src/original_name_edit.rs](../../../../rust/tcl-lsp-core/src/original_name_edit.rs), `original_variable_root_name_edit`: Checks original written lexical delimiters and root extent before issuing one immutable source edit.
- [rust/tcl-lsp-core/src/original_name_edit.rs](../../../../rust/tcl-lsp-core/src/original_name_edit.rs), `original_name_input_edits`: Retains genuine original list-container geometry and refuses overlapping or unrelated-child edits.
- [rust/tcl-lsp-core/src/original_name_edit.rs](../../../../rust/tcl-lsp-core/src/original_name_edit.rs), `original_input_matches_source`: Checks original input/source-span correspondence under the consumer complete source and grammar; shared by namespace and variable editors.
- [rust/tcl-lsp-core/src/variable_symbol/rename.rs](../../../../rust/tcl-lsp-core/src/variable_symbol/rename.rs), `variable_symbol::rename::tests::original_variable_rename_accounts_for_derived_and_independent_alias_reads` (linked): The edit contract requires complete original target/read coverage and preserves the actual alias naming purpose. The regression applies edits to the namespace declaration, global target and derived $v read, plus the upvar target, while alias and $alias remain exact. Unknown/computed targets without independently editable geometry decline; native alias execution and cell identity are separate questions.

## Evidence

- [rust/tcl-lsp-core/src/variable_symbol/rename.rs](../../../../rust/tcl-lsp-core/src/variable_symbol/rename.rs)
