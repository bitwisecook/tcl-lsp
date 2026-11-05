# naming.core.original-format-hint-producer

Kind: `implementation-contract`

## Problem statement

A format template can be captured by an alias or produced by expansion rather than appearing as the first written argument. Rendering offsets from the call head or logical display can attach percent labels to another word, and decoding escapes can shift the original extent. Applicability depends on the exact template producer and current complete source, separately from selected format semantics.

## Question

Do format hints map template specifiers through the genuine original template producer and native escape extents?

## Conclusion

Format roles consume the same complete effective origin vector as parameter hints. Template content and native escape offsets are projected from their retained producer; captured values without consumer geometry do not receive invented anchors. Static expansion children retain readonly source extents. The hints grant no evaluated template value, runtime format success or source edit authority.

## Scope

Core format hints for original written templates, static list expansion, captured prefix values and escape-producing source. Rust selectors are linked obligations; native encoding measurements remain separate questions.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This Rust consumer contract has no native execution receipt.

## Consumer bindings

- [rust/tcl-lsp-core/src/inlay_hints.rs](../../../../rust/tcl-lsp-core/src/inlay_hints.rs), `format_args`: Select the genuine template operand through complete effective argument ownership.
- [rust/tcl-lsp-core/src/inlay_hints.rs](../../../../rust/tcl-lsp-core/src/inlay_hints.rs), `collect_format_string_hints`: Map specifier offsets through the original template content and escape extent.
- [rust/tcl-lsp-core/src/inlay_hints.rs](../../../../rust/tcl-lsp-core/src/inlay_hints.rs), `inlay_hints::original_hint_tests::original_format_alias_expansion_and_escape_hints_use_the_template_producer` (linked): Template origins, captured prefix refusal and escape geometry remain distinct.

## Evidence

- [rust/tcl-lsp-core/src/inlay_hints.rs](../../../../rust/tcl-lsp-core/src/inlay_hints.rs)
