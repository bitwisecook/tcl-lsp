# naming.core.original-variable-outline-advice

Kind: `implementation-contract`

## Problem statement

A Registry-selected write can name a source declaration while unknown execution prevents selecting its variable cell. Reusing a reporting VarDef would conflate opaque names, whereas demanding a storage Symbol would hide source cards. Source outline advice must retain its original name and selected receiver form without becoming navigation or rename authority.

## Question

Can variable outline cards consume conditional original write-name advice independently of storage-symbol availability?

## Conclusion

OriginalVariableWriteAdvice retains the original Input, written span, selected Combined receiver form and optional authentic frame/namespace. The outline validates the complete source, uses the selected combined-root projection, excludes genuine procedure and receiver frames, and does not duplicate syntax already covered by a symbol declaration. The cards issue no storage identity, native store, navigation target or editable reference set.

## Scope

Conditional source outline cards under retained native naming policies. Missing source/configuration and displaced source refuse. Unknown frame supplies no root or namespace owner; the card stays unhomed. Procedure-local exclusion requires an authentic frame receipt. Logical-only profiles retain their separately bounded outline path.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This is a current Rust implementation contract. No native provider observation or test execution is claimed for this precise consumer question.

## Consumer bindings

- [rust/tcl-compiler/src/signature_scan/variable_symbol.rs](../../../../rust/tcl-compiler/src/signature_scan/variable_symbol.rs), `OriginalVariableWriteAdvice`: Retain conditional selected write-name syntax without a storage Symbol.
- [rust/tcl-lsp-core/src/document_symbols/original.rs](../../../../rust/tcl-lsp-core/src/document_symbols/original.rs), `source_outline`: Project current conditional source cards independently of navigation and rename receipts.
- [rust/tcl-lsp-core/src/document_symbols/original.rs](../../../../rust/tcl-lsp-core/src/document_symbols/original.rs), `document_symbols::original::tests::original_write_advice_cards_do_not_require_or_donate_storage_symbols` (linked): Require distinct opaque source cards with storage/reporting maps absent; reject displaced source and a removed advice inventory.

## Evidence

- [rust/tcl-lsp-core/src/document_symbols/original.rs](../../../../rust/tcl-lsp-core/src/document_symbols/original.rs)
