# naming.grammar.original-error-term-geometry

Kind: `implementation-contract`

## Problem statement

A malformed nested command can report an inner quote, brace or variable error while the enclosing component still begins at its outer bracket or variable marker. Using that raw component start as the parser term truncates or refuses the runtime command context.

## Question

How does the shared scanner retain the actual failed delimiter without changing the original component extent or borrowing a native compilation result?

## Conclusion

The scanner keeps an independent error term and the executable arena rebases it into the original image. ParseCut uses that term for the diagnostic extent and keeps its existing outer reporting anchor. This is a Rust geometry contract, not compilation admission, guest completion or a new native observation.

## Scope

Original Document and NativeValue images, complete LexerConfig and retained scanner components. Tcl 8.4 capture rows 6 and 9 independently support the inner quote and brace terms. Other fixed Rust controls verify byte/rebasing invariants; their presence does not establish provider execution. Jim template grammar and compiler diagnostic rendering remain independently selected.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `c84-terms` (observation): [rust/tcl-registry/tests/data/native_c84_parse_context/8.4.20.tsv](../../../../rust/tcl-registry/tests/data/native_c84_parse_context/8.4.20.tsv). SHA-256 `9b985563f9617716b3657543d3b5a60b02024d4660f3c18f63e91da7701c5342`. Retained native row 6 records the inner quote term 13; row 9 records the inner brace term 11. Provider observation belongs to naming.grammar.c84-parser-context-extents.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lexer/src/word_parts.rs](../../../../rust/tcl-lexer/src/word_parts.rs), `scan_var_ref_with_term`: One variable scanner retains the actual failed delimiter while the public message-only adapter preserves its contract.
- [rust/tcl-lexer/src/executable_parts.rs](../../../../rust/tcl-lexer/src/executable_parts.rs), `SpannedExecutablePart::parse_error_term`: Retain image-coordinate parser error geometry separately from the raw component span.
- [rust/tcl-lexer/src/parse_cut.rs](../../../../rust/tcl-lexer/src/parse_cut.rs), `first_parse_cut_image_checked`: Inspect the same original channel/config and publish separate reporting anchor and parser term.
- [rust/tcl-lexer/src/parse_cut.rs](../../../../rust/tcl-lexer/src/parse_cut.rs), `parse_cut::tests::original_error_term_rebasing_preserves_native_bytes_and_inner_reference` (linked): Preserve outer reporting anchors and exact inner delimiter terms under complete selected C grammars, including opaque bytes, zero, braced variables, array indices and nested brackets.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "cargo",
  "test",
  "-p",
  "tcl-lexer",
  "--lib",
  "original_error_term_"
]
```

Run from rust/. The implementation controls are not native guest executions; the original C8.4 evidence is replayed through its separate retained probe protocol.
