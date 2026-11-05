# naming.minifier.lexical-authoring-passes

Kind: `implementation-contract`

## Problem statement

An unsupported Native naming pass must not disable explicitly selected lexical authoring features or gain authority from a provider label. Compatibility passes must retain the actual complete editing configuration and availability context.

## Question

Does the minifier retain explicit lexical authoring compaction as a separately selected purpose?

## Conclusion

Compatibility naming, alias and semantic passes run only when the retained analysis independently permits lexical declaration advice and the complete configuration and availability context are supported. The profile, command store and actual immutable context are retained as editing inputs. Unsupported custom configuration or context returns the whole original source with a typed refusal and no compatibility or Native fallback. Lexical authoring transformations supply no Native cell, command, observer, insertion, transfer or edit authority.

## Scope

Core explicitly selected Logical compatibility transformations only. Current Native and hosted original-input planners retain their separate purposes. Profile-default configuration and structurally matching full availability context bound the remaining legacy passes; unsupported axes remain terminal.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This Rust consumer contract has no native execution receipt.

## Consumer bindings

- [rust/tcl-lsp-core/src/minify/original.rs](../../../../rust/tcl-lsp-core/src/minify/original.rs), `minify_with_analysis`: Require independently selected Logical advice, current source, supported complete configuration and actual full context before any compatibility transformation.
- [rust/tcl-lsp-core/src/minify.rs](../../../../rust/tcl-lsp-core/src/minify.rs), `lexical_compact`: Keep reporting-map authoring behaviour inside the supported Logical branch and retain the same actual input when analysing rewritten source.
- [rust/tcl-lsp-core/src/minify/original.rs](../../../../rust/tcl-lsp-core/src/minify/original.rs), `analysis_for_profile`: Retain the supplied profile/store as explicit driver inputs.
- [rust/tcl-lsp-core/src/minify/original.rs](../../../../rust/tcl-lsp-core/src/minify/original.rs), `minify::original::tests::lexical_minifier_keeps_explicit_authoring_compaction_separate` (linked): An explicitly selected lexical-only profile preserves supported authoring local/procedure compaction without enabling the hosted or Native path.

## Evidence

- [rust/tcl-lsp-core/src/minify/original.rs](../../../../rust/tcl-lsp-core/src/minify/original.rs)
- [rust/tcl-lsp-core/src/minify.rs](../../../../rust/tcl-lsp-core/src/minify.rs)
