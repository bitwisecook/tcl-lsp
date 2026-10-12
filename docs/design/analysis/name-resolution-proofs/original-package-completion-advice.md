# naming.core.original-package-completion-advice

Kind: `implementation-contract`

## Problem statement

A completion caller can supply a profile label that differs from the retained document context, and reporting package/version fields cannot authenticate a package-purpose name or requirement floor.

## Question

Does package completion read the document’s complete retained availability context and genuine original package/requirement operands while keeping package loading independent?

## Conclusion

DocumentFloor borrows ResolvedAnalysisInput::availability_context rather than replacing it with a caller profile. Package advice matches the authentic package-purpose key and exact current complete-source/configuration owner of original requirements. Reporting names, versions and requirement strings cannot replace those operands. Alternative version requirements preserve their least admitted floor; independent unconditional requirements combine by their greatest floor, while conditional/control-flow requests cannot establish an unconditional package suggestion. Ambient authoring context remains independently retained. Primary completion shares these owners and refuses changed complete source.

## Scope

Readonly package catalogue and version-floor advice only. Genuine original package/requirement Inputs, erased or changed reporting names, caller-profile disagreement, foreign-source rows and conditional/version alternatives are linked Rust coverage. No package loader, installed provider, actual version, command publication, runtime lookup, evaluation, Normal or edit grant follows. An independently selected explicit lexical compatibility branch remains separate.

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

- [rust/tcl-compiler/src/analyser/input.rs](../../../../rust/tcl-compiler/src/analyser/input.rs), `ResolvedAnalysisInput::availability_context`: Borrow the complete actual retained authoring context without recreating it from a caller profile label.
- [rust/tcl-lsp-core/src/document_floor.rs](../../../../rust/tcl-lsp-core/src/document_floor.rs), `DocumentFloor`: Match current genuine package-purpose and requirement operands and preserve independently retained ambient/conditional/floor bounds.
- [rust/tcl-lsp-core/src/completion.rs](../../../../rust/tcl-lsp-core/src/completion.rs), `completions`: Consume package/floor advice from the current analysis and its retained context before presenting catalogue suggestions.
- [rust/tcl-lsp-core/src/document_floor.rs](../../../../rust/tcl-lsp-core/src/document_floor.rs), `document_floor::original_package_advice_tests::original_package_advice_keeps_actual_context_and_original_requirement_units` (linked): Actual original package names and version operands survive changed reporting fields; missing/foreign source rows cannot donate a require, and a caller profile cannot replace ambient context.
- [rust/tcl-lsp-core/src/document_floor.rs](../../../../rust/tcl-lsp-core/src/document_floor.rs), `document_floor::original_package_advice_tests::original_package_advice_preserves_alternative_floor_and_conditional_bounds` (linked): Alternative requirements retain the least admitted version floor; conditional requirements cannot establish unconditional package advice and retain the independent ambient floor.
- [rust/tcl-lsp-core/src/completion.rs](../../../../rust/tcl-lsp-core/src/completion.rs), `completion::original_package_completion_tests::original_package_completion_uses_retained_context_and_ignores_reported_names` (linked): Current original Tk advice survives altered reporting names and a foreign caller profile while changed whole source withdraws completion.

A named test is a coverage binding, not a claim that it executed.

## Replay

The linked Rust selectors exercise readonly package/context and floor premises. No execution receipt or native provider observation is attached to this implementation contract.
