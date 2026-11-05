# naming.core.original-command-head-colours

Kind: `implementation-contract`

## Problem statement

Searching a displayed command spelling for a namespace separator can split an escaped or counted original head at the wrong source offset, or treat separators outside the selected command-purpose extent as a qualifier.

## Question

How do readonly command-head colours retain the actual original qualifier and tail source geometry independently of the canonical Registry label?

## Conclusion

The shared source-schema carrier retains a separate original head operand with its genuine name input and whole NativeWord. Semantic colouring requires that input's original key to retain the same word and complete document image. The independently selected command recipe chooses the native qualifier/tail extent; the existing original decoded-value-to-source mapper projects each extent through its actual source spelling. A split is emitted only when the two source ranges meet. The canonical Registry schema classifies the head for presentation and supplies no source spelling. Unavailable input/key, unsupported projection or no selected qualifier retains a whole-head presentation rather than deriving a split from display text.

## Scope

Readonly command-head colour geometry under the actual full source/configuration and source-schema owner. Fixed coverage includes literal and escaped root qualifiers and a counted zero before a later separator. It supplies no command lookup, namespace existence, handler execution, native admission, writable name, runtime binding or edit permission.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

No observation for this implementation question.

## Consumer bindings

- [rust/tcl-compiler/src/registry_invocation/source_structure.rs](../../../../rust/tcl-compiler/src/registry_invocation/source_structure.rs), `OriginalRegistryWords::head_source`: Expose independently retained original written head input/word geometry separate from the effective Registry command label.
- [rust/tcl-lsp-core/src/original_invocation.rs](../../../../rust/tcl-lsp-core/src/original_invocation.rs), `OriginalRegistryWords::head_source`: Delegate head ownership directly to the shared sealed Compiler carrier without reconstructing source or a name input.
- [rust/tcl-lsp-core/src/semantic_tokens/original.rs](../../../../rust/tcl-lsp-core/src/semantic_tokens/original.rs), `registry_head`: Classify the canonical source schema independently and split only actual original qualifier/tail ranges whose decoded source projections meet.
- [rust/tcl-lsp-core/src/original_name_edit.rs](../../../../rust/tcl-lsp-core/src/original_name_edit.rs), `original_static_name_value_span`: Project an independently selected original value extent to its genuine source spelling; geometry alone provides no edit grant.
- [rust/tcl-lsp-core/src/semantic_tokens/original.rs](../../../../rust/tcl-lsp-core/src/semantic_tokens/original.rs), `semantic_tokens::original::original_source_schema_tests::original_semantic_command_qualifiers_use_decoded_source_geometry` (linked): Cleared reporting projections preserve literal and escaped original root qualifier/tail spans; a zero before a later separator cannot borrow a namespace split from the complete display spelling.
