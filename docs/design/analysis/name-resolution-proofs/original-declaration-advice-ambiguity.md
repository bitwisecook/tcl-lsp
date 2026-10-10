# naming.consumer.original-declaration-advice-ambiguity

Kind: `implementation-contract`

## Problem statement

A source declaration query can find multiple authentic definitions of the same original name. Choosing the final reporting entry would turn ambiguous declaration advice into a claimed dispatch target.

## Question

Does original procedure declaration advice decline multiple authentic candidates instead of choosing a redefinition by reporting order?

## Conclusion

The procedure advice helper uses the actual original call operand and its retained namespace scope. It returns a declaration only when the original candidate vector contains exactly one entry; two genuine redefinitions yield no advice, independently of a counterfactual reporting label. Missing original input in Native mode does not enter lexical fallback. Positioned implementation receipts remain the separate owner of actual call targets.

## Scope

Rust readonly local procedure declaration advice. The fixed test has one actual call and two original definitions under the same source name and namespace. It does not assert native redefinition execution, installed command identity, reference edit permission or cross-document provider completeness.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This precise Rust source contract has no native execution receipt.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This precise Rust source contract has no native execution receipt.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This precise Rust source contract has no native execution receipt.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This precise Rust source contract has no native execution receipt.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This precise Rust source contract has no native execution receipt.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

This precise Rust source contract has no native execution receipt.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

This precise Rust source contract has no native execution receipt.

## Exact evidence

- `implementation-0` (implementation): [rust/tcl-lsp-core/src/definition.rs](../../../../rust/tcl-lsp-core/src/definition.rs). SHA-256 `b4ae9871cee3ccc73410891afee0b9639b5b04695ff9bf6e83ea56ac4142c553`. Current Rust source and fixed assertion bindings; no execution receipt.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-core/src/definition.rs](../../../../rust/tcl-lsp-core/src/definition.rs), `proc_visible_from_namespace`: Use genuine original operand and scope for unique declaration advice; retain positioned dispatch and explicit lexical compatibility as separate purposes.
- [rust/tcl-lsp-core/src/definition.rs](../../../../rust/tcl-lsp-core/src/definition.rs), `definition::tests::original_declaration_advice_cannot_choose_between_redefinitions` (linked): Retain two original candidates and decline advice with a counterfactual reporting label rather than selecting the last definition.

A named test is a coverage binding, not a claim that it executed.

## Replay

The named Rust selectors bind fixed source assertions. Their execution results are separate receipts; no native interpreter executes this editor contract.
