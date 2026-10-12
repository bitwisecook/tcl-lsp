# naming.core.original-document-grammar-completion

Kind: `implementation-contract`

## Problem statement

Resolving a document grammar from a profile label can lose the current full lexer configuration, including BOM interpretation, or offer grammar words after the source changes.

## Question

Does closed document-grammar completion preserve the actual current source and every retained parser configuration axis?

## Conclusion

Document-grammar completion consumes the actual Analysis source owner and full lexer configuration before selecting the independently retained closed grammar vocabulary. BOM Skip and Content remain distinct source interpretations; missing or stale original source cannot borrow a nominal profile grammar. This source vocabulary supplies no selected command implementation, runtime lookup, evaluation, Normal or edit authority.

## Scope

Current Core closed document-grammar suggestions, with a genuine Sslic source vocabulary under independently retained BOM Skip/Content and stale-source controls. This is a Rust source-consumer contract; no native-provider or passing Rust receipt is attached.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

This is a Rust source-consumer implementation contract. No native provider observation or executed Rust result is attached; selected source advice supplies no physical native or runtime authority.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

This is a Rust source-consumer implementation contract. No native provider observation or executed Rust result is attached; selected source advice supplies no physical native or runtime authority.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

This is a Rust source-consumer implementation contract. No native provider observation or executed Rust result is attached; selected source advice supplies no physical native or runtime authority.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

This is a Rust source-consumer implementation contract. No native provider observation or executed Rust result is attached; selected source advice supplies no physical native or runtime authority.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

This is a Rust source-consumer implementation contract. No native provider observation or executed Rust result is attached; selected source advice supplies no physical native or runtime authority.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

This is a Rust source-consumer implementation contract. No native provider observation or executed Rust result is attached; selected source advice supplies no physical native or runtime authority.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

This is a Rust source-consumer implementation contract. No native provider observation or executed Rust result is attached; selected source advice supplies no physical native or runtime authority.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-core/src/completion.rs](../../../../rust/tcl-lsp-core/src/completion.rs), `completions`: Select closed document grammar suggestions only after complete current source/configuration correspondence; preserve independently selected parser axes.
- [rust/tcl-lsp-core/src/completion.rs](../../../../rust/tcl-lsp-core/src/completion.rs), `completion::original_document_grammar_tests::original_document_grammar_completion_preserves_full_source_configuration` (linked): Genuine closed grammar vocabulary distinguishes retained BOM Skip and Content and refuses stale full source without treating a profile label as the source owner.

A named test is a coverage binding, not a claim that it executed.

## Replay

The named Rust selectors bind fixed source-consumer assertions. No Rust execution receipt or native experiment is attached. Whole-image/configuration/Registry currency and independently required purpose permissions remain mandatory.
