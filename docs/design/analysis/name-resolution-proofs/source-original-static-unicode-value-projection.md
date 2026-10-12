# naming.source.original-static-unicode-value-projection

Kind: `implementation-contract`

## Problem statement

Source presentation may decode an unrepresentable numeric escape to a replacement character. A Logical name or literal fact then needs a checked value facet before treating that presentation as its original value; equal presentation bytes alone cannot close original source identity.

## Question

How does a static original Document word supply a String-compatible Unicode value while keeping unrepresentable interpreted escape units separate from replacement presentation and Native values?

## Conclusion

original_static_word_unicode_value requires the authentic complete Document word and its retained lexer configuration. It starts from the shared static source presentation, then examines original interpreted escape lexemes through the shared backslash scanner. Each numeric codepoint must be a Unicode scalar and its selected decoded bytes must match the scalar encoding; otherwise the String-compatible projection is unavailable. Final bytes must be valid UTF8. Braced escapes and escaped backslashes remain literal under the retained grammar. Valid Unicode, literal U+FFFD and the explicit replacement escape can remain admitted, while unrepresentable interpreted units cannot become replacement-character Logical names or facts. Dynamic substitutions, expansion and NativeValue channel words do not supply this source facet. Presentation remains a separate API; refusal here says only that this checked Logical value is unavailable.

The independent [original source escape comparison](source-original-surrogate-versus-replacement-escape-values.md) answers public equality and length only. The [static ASCII metadata](static-ascii-metadata.md) owner retains its separate admission scope.

## Scope

One marked Syntax source/API control checks valid Unicode and replacement values, braced and escaped literal spellings, unrepresentable high/low units, dynamic/command/expanded words and Native channel refusal using its explicitly constructed configurations. A profile label or default configuration does not establish an original provider or native grammar. Two separately bound Compiler source controls join the checked facet to retained Logical procedure identity and authored transition operands; full original source/input/configuration/unknown guards and Native unit policies remain independent. All seven provider rows are not tested for this API contract, and no Rust assertion outcome is attached. The independent original public comparison proof reports exact source escape equality/length observations; it does not issue this API, certify internal units or turn a Logical projection refusal into Native failure. No Normal, object/header/cache, slot, runtime lookup, frame, handler entry, publication or edit permission follows.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: No original external process for the checked static Document value API. Dialect: tcl8.4.

The software definition retains original word/configuration and checks a String-compatible Logical projection. Original Native source comparisons, representations and naming purposes remain independent; this contract has no external provider observation.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: No original external process for the checked static Document value API. Dialect: tcl8.5.

The software definition retains original word/configuration and checks a String-compatible Logical projection. Original Native source comparisons, representations and naming purposes remain independent; this contract has no external provider observation.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: No original external process for the checked static Document value API. Dialect: tcl8.6.

The software definition retains original word/configuration and checks a String-compatible Logical projection. Original Native source comparisons, representations and naming purposes remain independent; this contract has no external provider observation.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: No original external process for the checked static Document value API. Dialect: tcl9.0.

The software definition retains original word/configuration and checks a String-compatible Logical projection. Original Native source comparisons, representations and naming purposes remain independent; this contract has no external provider observation.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: No original external process for the checked static Document value API. Dialect: tcl9.1.

The software definition retains original word/configuration and checks a String-compatible Logical projection. Original Native source comparisons, representations and naming purposes remain independent; this contract has no external provider observation.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: No original external process for the checked static Document value API. Dialect: jim.

The software definition retains original word/configuration and checks a String-compatible Logical projection. Original Native source comparisons, representations and naming purposes remain independent; this contract has no external provider observation.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: No original external process for the checked static Document value API. Dialect: bigip.

The software definition retains original word/configuration and checks a String-compatible Logical projection. Original Native source comparisons, representations and naming purposes remain independent; this contract has no external provider observation.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-syntax/src/word_rules.rs](../../../../rust/tcl-syntax/src/word_rules.rs), `original_static_word_unicode_value`: Require authentic Document/static word/configuration, validate original numeric escape lexemes and final UTF8, and withhold a String-compatible value when interpreted units are unrepresentable.
- [rust/tcl-syntax/src/word_rules.rs](../../../../rust/tcl-syntax/src/word_rules.rs), `original_static_word_source_bytes`: Retain independent static source presentation; its bytes alone do not establish the checked Unicode value or original Native units.
- [rust/tcl-syntax/src/word_rules.rs](../../../../rust/tcl-syntax/src/word_rules.rs), `original_static_word_ascii_presentation`: Keep the separate static ASCII presentation facet and its own lexical scope, without filling a refused Unicode-value projection.
- [rust/tcl-syntax/src/word_rules.rs](../../../../rust/tcl-syntax/src/word_rules.rs), `word_rules::original_metadata_tests::static_unicode_values_keep_original_units_separate_from_replacement_presentation` (linked): Exact Document words retain valid Unicode/replacement and braced or escaped literal spellings; interpreted unrepresentable numeric units remain unavailable even when presentation contains U+FFFD. Dynamic/expanded and Native words refuse the checked facet. Configuration inputs are software premises, without inferred native provider selection or outcome.

A named test is a coverage binding, not a claim that it executed.

## Replay

Current source/API definitions only. No Rust assertion or external provider run is attached. The independent native original-surrogate-versus-replacement-escape-values proof retains its exact public comparison/length source, releases and complete streams; it grants no source API, internal-unit or Native naming authority.
