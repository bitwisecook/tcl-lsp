# naming.refactor.original-extracted-procedure-publication

Kind: `implementation-contract`

## Problem statement

A generated procedure name can collide with an opaque procedure, class or alias even when reporting maps collapse or are cleared. Searching UI names or accepting an exhausted short-name search can introduce a different publication. Applicability requires the genuine completed byte world and independently retained declaration candidates.

## Question

Does extracted procedure naming test the exact proposed native publication against current occupied slots?

## Conclusion

Explicit API literal names are projected through the selected Document ingress and publication purpose. Name search checks the closed original completed world and typed declaration slots, including class and alias occupants, without rebuilding keys from reporting labels. Unknown occupancy and exhausted names decline. This is publication naming advice and supplies no procedure insertion or body relocation permission.

## Scope

Original extracted procedure name proposal and literal API matching contract; the overall Native extraction remains disabled when motion/insertion contracts are missing. Lexical compatibility uses its separate authoring domain.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This Rust consumer contract has no native execution receipt.

## Consumer bindings

- [rust/tcl-lsp-core/src/original_declaration.rs](../../../../rust/tcl-lsp-core/src/original_declaration.rs), `literal_name_matches_slot`: Project explicit literal API input to the selected C root or Jim flat slot without minting source ownership.
- [rust/tcl-lsp-core/src/refactor/extract_proc.rs](../../../../rust/tcl-lsp-core/src/refactor/extract_proc.rs), `unique_proc_name`: Choose a free proposed publication only through the current byte owner.
- [rust/tcl-lsp-core/src/refactor/extract_proc.rs](../../../../rust/tcl-lsp-core/src/refactor/extract_proc.rs), `refactor::extract_proc::original_extract_tests::original_extract_name_collision_uses_publications_without_reporting_maps` (linked): Cleared reporting maps do not hide occupied opaque publication slots.
- [rust/tcl-lsp-core/src/refactor/extract_proc.rs](../../../../rust/tcl-lsp-core/src/refactor/extract_proc.rs), `refactor::extract_proc::original_extract_tests::original_extract_name_search_refuses_unknown_or_fully_occupied_slots` (linked): Unknown occupancy and exhausted generated-name search remain terminal.

## Evidence

- [rust/tcl-lsp-core/src/original_declaration.rs](../../../../rust/tcl-lsp-core/src/original_declaration.rs)
- [rust/tcl-lsp-core/src/refactor/extract_proc.rs](../../../../rust/tcl-lsp-core/src/refactor/extract_proc.rs)
