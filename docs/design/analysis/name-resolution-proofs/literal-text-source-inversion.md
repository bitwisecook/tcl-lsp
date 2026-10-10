# naming.source.literal-text-source-inversion

Kind: `implementation-contract`

## Problem statement

An executable lexical variable root does not evaluate numeric escapes. A quoted-word renderer therefore cannot supply its name. C8.4/8.5 Document ingress also represents a supplementary Unicode character as four Latin-1 byte units, so decoding all retained native units as UTF-16 can lose an otherwise representable source spelling. The insertion needs exact literal text under the original source channel and independent string protocol.

## Question

Does the literal-text renderer invert retained Document ingress units without borrowing numeric escape evaluation or changing native units?

## Conclusion

native_literal_source_text returns a proposed literal String only after exact selected-channel retranslation reproduces every requested byte. The C8.4/8.5 Document candidate recognises four decoded Latin-1 units only when standard UTF-8 decoding yields one supplementary scalar, then verifies the whole roundtrip. C8.6 UTF-16 and C9 scalar candidates retain their separate paths. Invalid units, unpaired surrogates and incompatible NativeValue zero spellings remain unavailable. The fixed assertion uses retained actual C5 ingress rows as inputs to a separate Rust renderer contract; it does not execute those proposed spellings or issue naming/edit authority.

## Scope

Literal text serialization under modeled C8.4–9.1 string protocols and an explicitly supplied Document or NativeValue channel. The retained plain native ingress row contains NUL, a supplementary scalar and normalized line endings. This contract does not admit source words, lexical variable grammar, namespace lookup, a current cell or native allocation. Jim and BIG-IP have no native observation for this precise inverse-renderer question.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This is a Rust implementation assertion; no native provider observation answers this precise source-consumer question.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This is a Rust implementation assertion; no native provider observation answers this precise source-consumer question.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This is a Rust implementation assertion; no native provider observation answers this precise source-consumer question.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This is a Rust implementation assertion; no native provider observation answers this precise source-consumer question.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This is a Rust implementation assertion; no native provider observation answers this precise source-consumer question.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

This is a Rust implementation assertion; no native provider observation answers this precise source-consumer question.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

This is a Rust implementation assertion; no native provider observation answers this precise source-consumer question.

## Exact evidence

- `source-backslash` (implementation): [rust/tcl-syntax/src/backslash.rs](../../../../rust/tcl-syntax/src/backslash.rs). SHA-256 `b5e0c4b86c8e54dd4df5405c29ffa1c509183b00075e82e0b260dcf9129560df`. Reviewed current implementation and linked fixed assertions; no execution receipt.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-syntax/src/backslash.rs](../../../../rust/tcl-syntax/src/backslash.rs), `native_literal_source_text`: Require exact literal-channel retranslation, independently of source-word escapes and quoting.
- [rust/tcl-syntax/src/backslash.rs](../../../../rust/tcl-syntax/src/backslash.rs), `native_source_literal_bytes`: Own the selected original input-channel translation used to verify proposed literal text.
- [rust/tcl-lsp-core/src/completion/original_variables.rs](../../../../rust/tcl-lsp-core/src/completion/original_variables.rs), `reference_spelling`: Validate literal text again as one authentic executable variable reference with an independent index.
- [rust/tcl-syntax/src/backslash.rs](../../../../rust/tcl-syntax/src/backslash.rs), `backslash::tests::literal_text_inverse_roundtrips_actual_document_ingress_units` (linked): Invert retained C5 plain ingress units and refuse invalid bytes, unpaired surrogates and NativeValue modified-zero donation.

A named test is a coverage binding, not a claim that it executed.

## Replay

The listed fixed Rust selectors are replay bindings, not execution receipts. Native source ingress observations are separately scoped and do not certify these consumer assertions.
