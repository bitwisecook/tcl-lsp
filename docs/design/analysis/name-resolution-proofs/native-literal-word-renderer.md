# Can the shared source literal renderer retain exact selected native bytes under the requested original channel, full escape grammar and independent string protocol?

Proof ID: `naming.source.native-literal-word-renderer`

## Problem statement

A new source spelling can change the native value of a name: a counted raw zero differs from modified zero, a lone surrogate has no Rust Unicode scalar, and a Document character channel differs from a counted NativeValue input. Using a displayed String or a list formatter as a source-name identity can therefore rename another slot. This question concerns exact source spelling, without command lookup or native object authority.

## Question

Can the shared source literal renderer retain exact selected native bytes under the requested original channel, full escape grammar and independent string protocol?

## Scope

Rust source rendering for C8.4–9.1 and current modeled Jim protocol, both Document and NativeValue channels. The decoder/protocol guards retain the requested grammar. This question does not observe a real interpreter executing the rendered words.

## Provider answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 | not-tested | No observation for this question. |
| tcl8.5 | not-tested | No observation for this question. |
| tcl8.6 | not-tested | No observation for this question. |
| tcl9.0 | not-tested | No observation for this question. |
| tcl9.1 | not-tested | No observation for this question. |
| jim | not-tested | No observation for this question. |
| bigip | not-tested | No appliance observation for this question. |

## Conclusion

native_literal_source_word returns a quoted literal only when the shared source decoder produces the requested bytes exactly. It first checks a literal UTF-8 spelling, then independently selected C native units with exact byte re-encoding and numeric escapes. Raw counted zero remains raw in NativeValue, modified zero uses its own escape spelling, and Document raw-zero bytes or invalid opaque bytes that have no exact representable spelling remain unavailable. No naming, object, cache, completion or compiler capability follows from rendering.

## Shared owners and tests

- `rust/tcl-syntax/src/backslash.rs`: `native_literal_source_word`. Central original-channel native-byte source spelling.
- `rust/tcl-syntax/src/backslash.rs`: `native_source_string_bytes_channel_in`. Independent exact native value comparison.
- `rust/tcl-syntax/src/backslash.rs`: `backslash::tests::literal_source_renderer_preserves_selected_native_bytes_and_channel`. Check exact decoded values for syntax metacharacters, newline controls, modified zero and distinct native surrogate units; distinguish raw source channels and unavailable opaque bytes.

## Reconfirmation

```text
cargo test -p tcl-syntax backslash::tests::literal_source_renderer_preserves_selected_native_bytes_and_channel -- --exact
```

These are Rust implementation-contract checks. Native provider answers remain not-tested for this precise Rust question; neighboring native observations establish separate byte, evaluator and object rules. A captured interpreter outcome is not a Rust test pass.
