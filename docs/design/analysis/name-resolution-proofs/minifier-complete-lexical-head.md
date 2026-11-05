# naming.minifier.complete-lexical-head

Kind: `implementation-contract`

## Problem statement

A Logical minifier that selects a command from the first displayed token fragment can treat data following a computed or compound head as a builtin script body.

## Question

Does Logical role selection require the static metadata value of the exact complete original head word?

## Conclusion

Logical minification retains the actual complete script-word plan under the full body configuration. A head value is available only when the authentic Document word matches the complete current image, configuration and token vector, and the shared static ASCII metadata projection accepts the whole word. Substitutions, expansions, opaque units and non-ASCII values remain unavailable; braced, quoted and escaped static ASCII heads keep their exact decoded metadata value. The retained Realm and full ContextRegistry select authored roles and case layout from that value. Unavailable heads do not provide builtin Body, LambdaLiteral, Expr or case-list roles.

## Scope

The explicitly selected Logical compatibility branch only. The static ASCII projection supplies lexical metadata, not a Native name policy, Key, value object, command-table fact, compiler admission, frame, effects, Normal result or edit-equivalence permission. Native and hosted original-input planners remain separately selected. Existing complete-context and configuration refusals remain mandatory; ordinary lexical reconstruction keeps its separate authoring scope.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This complete-word Rust metadata contract has no native interpreter observation.

## Consumer bindings

- [rust/tcl-lsp-core/src/minify.rs](../../../../rust/tcl-lsp-core/src/minify.rs), `minify_body`: Capture original complete command words once under the actual full body configuration and pass the authentic head word into role rendering.
- [rust/tcl-lsp-core/src/minify.rs](../../../../rust/tcl-lsp-core/src/minify.rs), `lexical_script_words`: Use the shared complete original script-word plan without constructing a Word from fragments or granting Native execution authority.
- [rust/tcl-lsp-core/src/minify.rs](../../../../rust/tcl-lsp-core/src/minify.rs), `lexical_head_value`: Require exact Document image, full configuration and token-vector correspondence before the shared complete-word static ASCII metadata projection.
- [rust/tcl-lsp-core/src/minify.rs](../../../../rust/tcl-lsp-core/src/minify.rs), `render_command`: Select Logical authored roles and case layout only from the complete static head value through the retained Realm and full ContextRegistry.
- [rust/tcl-lsp-core/src/minify/original.rs](../../../../rust/tcl-lsp-core/src/minify/original.rs), `minify::original::tests::lexical_complete_heads_preserve_computed_body_data` (linked): Computed variable, compound and command-substitution heads preserve body-like braced data without borrowing a builtin Body role.
- [rust/tcl-lsp-core/src/minify/original.rs](../../../../rust/tcl-lsp-core/src/minify/original.rs), `minify::original::tests::lexical_complete_static_heads_keep_selected_body_roles` (linked): Bare, braced, quoted and escaped static ASCII heads retain available authored Body roles under the actual Logical context.
- [rust/tcl-lsp-core/src/minify/original.rs](../../../../rust/tcl-lsp-core/src/minify/original.rs), `minify::original::tests::lexical_head_value_checks_whole_image_config_and_tokens` (linked): Equal head spelling in a different complete source image, a different full configuration or a different original token vector cannot supply the head value.
- [rust/tcl-lsp-core/src/minify/original.rs](../../../../rust/tcl-lsp-core/src/minify/original.rs), `minify::original::tests::lexical_head_value_keeps_expansion_and_opaque_units_unavailable` (linked): A genuine expanded head and opaque or non-ASCII static units have no ASCII metadata value; a non-expanding braced star keeps its literal value under its actual grammar.
