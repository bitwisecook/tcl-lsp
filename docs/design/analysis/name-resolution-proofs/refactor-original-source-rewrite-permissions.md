# naming.refactor.original-source-rewrite-permissions

Kind: `implementation-contract`

## Problem statement

An original command spelling or selected handler does not prove equivalent operand evaluation, branch dispatch or a safe fresh temporary store. String-only helpers can rewrite a source procedure named expr/if/switch as a builtin or insert into an occupied or observed cell.

## Question

Do source rewrite helpers separate actual current handler/geometry advice from independently required expression, control-flow and fresh-store/insertion permissions?

## Conclusion

The common rewrite selector consumes actual Analysis, validates the whole source and full configuration, and uses the retained Registry/source traversal to select the corresponding conditional handler. Missing or shadowed targets refuse. Native structure advice does not grant transformation equivalence: expression bracing, branch conversion and dictionary conversion return a no-edit obligation reason until their independent evaluation/control-flow or temporary-cell/observer/store/insertion permissions are supplied. Explicitly selected Logical source advice retains its separate bounded transformation branch; hosted/native missing policy cannot reopen that branch from display text. Source correspondence supplies no Normal, runtime handler or inserted-cell capability. The separate exact-literal-expression bracing issuer can permit one static pure expression operand only after unchanged original argv, selected handler, compiler name effects, observer closure, checked native expression and independent braced-source decode correspondence all agree; structural handler advice alone still cannot issue that permission.

## Scope

Current if-to-switch, switch-to-dictionary and expression-bracing source adapters and their editor/MCP consumers. Fixed controls distinguish real selected Native structure from shadowed targets, stale source and occupied/observed temporary-cell scenarios. No Native transformed-program or passing Rust result is attached. The supported single-operand literal-bracing subset is documented in naming.refactor.original-literal-expression-bracing; broader expression substitution, functions, branch conversion and dictionary insertion retain separate missing permissions.

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

- [rust/tcl-lsp-core/src/refactor/source_rewrite.rs](../../../../rust/tcl-lsp-core/src/refactor/source_rewrite.rs), `select`: Validate the actual source/configuration and selected conditional handler while retaining its distinct missing rewrite obligation.
- [rust/tcl-lsp-core/src/refactor/source_rewrite.rs](../../../../rust/tcl-lsp-core/src/refactor/source_rewrite.rs), `RewriteObligation`: Keep expression evaluation, control-flow equivalence and fresh store/insertion as separate source transformation permissions.
- [rust/tcl-lsp-core/src/refactor/source_rewrite.rs](../../../../rust/tcl-lsp-core/src/refactor/source_rewrite.rs), `refactor::source_rewrite::tests::original_source_rewrites_require_current_handler_and_distinct_permissions` (linked): Actual Native selected handlers yield disabled no-edit obligations; stale/shadowed targets do not transform while separately selected Logical authoring remains supported.
- [rust/tcl-lsp-core/src/refactor/source_rewrite.rs](../../../../rust/tcl-lsp-core/src/refactor/source_rewrite.rs), `refactor::source_rewrite::tests::original_dictionary_conversion_does_not_insert_into_an_existing_or_observed_cell` (linked): Dictionary conversion cannot use structural advice to insert an arbitrary temporary into an occupied or observed cell.

A named test is a coverage binding, not a claim that it executed.

## Replay

The named Rust selectors bind fixed source-consumer assertions. No Rust execution receipt or native experiment is attached. Whole-image/configuration/Registry currency and independently required purpose permissions remain mandatory.
