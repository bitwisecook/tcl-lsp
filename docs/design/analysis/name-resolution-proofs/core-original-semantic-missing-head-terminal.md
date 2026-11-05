# naming.core.original-semantic-missing-head-terminal

Kind: `implementation-contract`

## Problem statement

A Native-policy invocation can lose its original head input or method receipt while a reporting class/object map still contains a plausible String name. Gating only heads that have an Input permits this missing-owner case to fall through to a legacy OO overlay. Missing source authority must remain terminal for semantic selection.

## Question

Do Native semantic token OO roles decline reporting overlays when the genuine original head is absent?

## Conclusion

OriginalTokenRoles blocks reporting OO overlays for all Native-policy head positions independently of whether an Input was retained. Current exact declaration and dispatch receipts still supply their own roles. Stale images retain the terminal guard; explicit lexical-only analysis keeps its separate reporting advice. No original Input is reconstructed to make an unavailable role appear.

## Scope

Core semantic token Native OO role terminal guard, with absent head/lookup, stale source and explicit F5 lexical compatibility controls. Token kinds are presentation and grant no dispatch, edit or execution authority.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This Rust consumer contract has no native execution receipt.

## Consumer bindings

- [rust/tcl-lsp-core/src/semantic_tokens/original.rs](../../../../rust/tcl-lsp-core/src/semantic_tokens/original.rs), `OriginalTokenRoles::has_head`: Prevent missing Native head ownership from falling through to String OO maps.
- [rust/tcl-lsp-core/src/semantic_tokens/original.rs](../../../../rust/tcl-lsp-core/src/semantic_tokens/original.rs), `semantic_tokens::original::original_missing_head_tests::original_native_missing_head_never_authorises_reporting_oo_overlays` (linked): Missing genuine Native Input/lookup and stale image remain terminal while lexical F5 advice stays separate.

## Evidence

- [rust/tcl-lsp-core/src/semantic_tokens/original.rs](../../../../rust/tcl-lsp-core/src/semantic_tokens/original.rs)
