# Static ascii metadata

Proof ID: `naming.grammar.static-ascii-metadata`

## Problem statement

A BIG-IP document may legitimately select an ASCII Registry subcommand while having no C/Jim native naming policy. Requiring such a policy loses metadata; inventing one would grant native naming semantics that the appliance has not supplied.

## Question

Which static metadata presentation can be obtained from an authentic Document word under its complete LexerConfig without a native name policy?

## Scope

Implementation lexical metadata contract, including custom brace rules. Actual C/Jim and BIG-IP execution is not observed here; these bytes do not supply cells, command lookup identity, compiler eligibility or Normal completion.

## Provider answers

| Provider | Status | Answer |
| --- | --- | --- |
| tcl8.4 | not-tested | This is an implementation contract; no interpreter or appliance observation is attached. |
| tcl8.5 | not-tested | This is an implementation contract; no interpreter or appliance observation is attached. |
| tcl8.6 | not-tested | This is an implementation contract; no interpreter or appliance observation is attached. |
| tcl9.0 | not-tested | This is an implementation contract; no interpreter or appliance observation is attached. |
| tcl9.1 | not-tested | This is an implementation contract; no interpreter or appliance observation is attached. |
| jim | not-tested | This is an implementation contract; no interpreter or appliance observation is attached. |
| bigip | not-tested | This is an implementation contract; no interpreter or appliance observation is attached. |

## Conclusion

The shared word owner returns ASCII presentation only for a complete non-expanded Document word whose executable arena contains Text alone. Braced continuation and escaped text use the retained lexical configuration. Dynamic components, non-ASCII output and NativeValue input decline. The result grants metadata presentation only.

## Shared owners and tests

- `rust/tcl-syntax/src/word_rules.rs`: `original_static_word_ascii_presentation`.

- `rust/tcl-syntax/src/word_rules.rs`: `word_rules::original_metadata_tests::lexical_metadata_word_uses_full_document_grammar_without_native_policy`.

The tests assert implementation correspondence and refusal, not a native observation. No actual run or timing result is encoded in this record. Source producer currency is independent of cell-read currency and Normal completion.

## Reconfirmation

Run the exact Rust selectors listed above in the relevant crate with the workspace's maintained test setup. No C/Jim/BIG-IP replay is attached to this implementation question.
