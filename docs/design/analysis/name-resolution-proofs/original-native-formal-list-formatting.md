# naming.editor.original-native-formal-list-formatting

Kind: `implementation-contract`

## Problem statement

String list decoding can replace native formal units or decode the wrong lambda list level before a formatter reconstructs source.

## Question

How does parameter and lambda formatting retain native formal values when a String list decoder cannot represent the selected units?

## Conclusion

Exact retained native children and full-source word ownership feed a spacing proposal. The shared channel ingress must retain the original literal units; a Unicode view of changed native units cannot replace their source spelling. Shared source/value/list owners then check byte-identical formal elements and optional lambda namespace after complete reconstruction. Unavailable, transformed or opaque units preserve the original whole word.

## Scope

Parameter/lambda source spelling under the actual full lexer configuration and independently retained native string/list recipe. No original proposal key, formal installation, native compiler/admission, body frame, Normal or runtime value is manufactured.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this implementation question.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this implementation question.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this implementation question.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this implementation question.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this implementation question.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this implementation question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No observation for this implementation question.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-core/src/formatting/engine.rs](../../../../rust/tcl-lsp-core/src/formatting/engine.rs), `native_param_list_source`: Read the actual native formal children and require exact representable views without a Unicode repair.
- [rust/tcl-lsp-core/src/formatting/engine.rs](../../../../rust/tcl-lsp-core/src/formatting/engine.rs), `original_static_spelling_value`: Parse a source spelling proposal with the same full grammar and native codec; no original key is issued.
- [rust/tcl-lsp-core/src/formatting/engine.rs](../../../../rust/tcl-lsp-core/src/formatting/engine.rs), `original_list_values_match`: Compare complete proposed native list elements to retained original child bytes.
- [rust/tcl-lsp-core/src/formatting/engine.rs](../../../../rust/tcl-lsp-core/src/formatting/engine.rs), `normalise_original_param_list`: Compose exact source and native list roundtrips before changing formal spacing.
- [rust/tcl-lsp-core/src/formatting/engine.rs](../../../../rust/tcl-lsp-core/src/formatting/engine.rs), `render_param_list_arg`: Keep explicit Logical compatibility separate and preserve the whole original word on missing native proof.
- [rust/tcl-lsp-core/src/formatting/engine.rs](../../../../rust/tcl-lsp-core/src/formatting/engine.rs), `original_lambda_parameters_match`: Validate native lambda cardinality, nested formal values and namespace after full reassembly.
- [rust/tcl-lsp-core/src/formatting/engine.rs](../../../../rust/tcl-lsp-core/src/formatting/engine.rs), `render_lambda_literal_arg`: Consume the exact nested formal proof or preserve the original whole lambda word.
- [rust/tcl-lsp-core/src/formatting/engine.rs](../../../../rust/tcl-lsp-core/src/formatting/engine.rs), `formatting::engine::tests::original_param_spacing_retains_native_children_and_opaque_units` (linked): Ordinary spacing and escaped spaces normalize only with exact native children; opaque surrogate/C8 supplementary units retain their complete original word; literal nested escapes remain intact.
- [rust/tcl-lsp-core/src/formatting/engine.rs](../../../../rust/tcl-lsp-core/src/formatting/engine.rs), `formatting::engine::tests::original_param_spacing_declines_foreign_word_and_full_config` (linked): A foreign original word, changed full config or absent native input cannot supply a normalization proof.
- [rust/tcl-lsp-core/src/formatting/engine.rs](../../../../rust/tcl-lsp-core/src/formatting/engine.rs), `formatting::engine::tests::original_lambda_spacing_checks_nested_formals_before_emission` (linked): Native nested formal decoding refuses unrepresentable children before source emission, preserves the whole lambda, and retains an exact ordinary parameters/namespace positive.
- [rust/tcl-lsp-core/src/formatting/engine.rs](../../../../rust/tcl-lsp-core/src/formatting/engine.rs), `formatting::engine::tests::original_source_formatting_keeps_opaque_formal_word_verbatim` (linked): The actual formatting entry preserves an opaque formal word while continuing to normalize ordinary parameter spacing.

A named test is a coverage binding, not a claim that it executed.

## Replay

Exact Rust coverage selectors are linked without an execution receipt. Neither source inspection nor native guest execution establishes this formatter invariant.
